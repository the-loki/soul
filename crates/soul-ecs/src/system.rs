use std::ffi::c_void;
use std::marker::PhantomData;
use std::panic::{catch_unwind, AssertUnwindSafe};

use soul_ecs_sys as sys;

use crate::borrow::BorrowContext;
use crate::param::{validate_unique_terms, QueryParam, QueryParamInternal, Term};
use crate::world::{PanicSlot, World};

type SystemEach<P> = dyn for<'row> FnMut(<P as QueryParam>::Item<'row>);

pub struct SystemBuilder<'world, P> {
    world: &'world World,
    _marker: PhantomData<P>,
}

impl<'world, P> SystemBuilder<'world, P> {
    pub(crate) fn new(world: &'world World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }
}

impl<'world, 'param, T: Copy + 'static> SystemBuilder<'world, (&'param T,)> {
    pub fn each(
        self,
        f: impl for<'row> FnMut(<(&'param T,) as QueryParam>::Item<'row>) + 'static,
    ) -> System<'world, (&'param T,)> {
        build_system::<(&'param T,), _>(self.world, f)
    }
}

impl<'world, 'param, T: Copy + 'static> SystemBuilder<'world, (&'param mut T,)> {
    pub fn each(
        self,
        f: impl for<'row> FnMut(<(&'param mut T,) as QueryParam>::Item<'row>) + 'static,
    ) -> System<'world, (&'param mut T,)> {
        build_system::<(&'param mut T,), _>(self.world, f)
    }
}

impl<'world, 'param, T: Copy + 'static, U: Copy + 'static>
    SystemBuilder<'world, (&'param T, &'param U)>
{
    pub fn each(
        self,
        f: impl for<'row> FnMut(<(&'param T, &'param U) as QueryParam>::Item<'row>) + 'static,
    ) -> System<'world, (&'param T, &'param U)> {
        build_system::<(&'param T, &'param U), _>(self.world, f)
    }
}

impl<'world, 'param, T: Copy + 'static, U: Copy + 'static>
    SystemBuilder<'world, (&'param mut T, &'param U)>
{
    pub fn each(
        self,
        f: impl for<'row> FnMut(<(&'param mut T, &'param U) as QueryParam>::Item<'row>) + 'static,
    ) -> System<'world, (&'param mut T, &'param U)> {
        build_system::<(&'param mut T, &'param U), _>(self.world, f)
    }
}

fn build_system<'world, P, F>(world: &'world World, f: F) -> System<'world, P>
where
    P: QueryParamInternal,
    F: for<'row> FnMut(P::Item<'row>) + 'static,
{
    world.assert_no_active_component_borrows();
    let terms = P::terms(world);
    validate_unique_terms(&terms, "system");
    let ids = terms.iter().map(|term| term.id).collect::<Vec<_>>();
    let inouts = terms.iter().map(|term| term.inout).collect::<Vec<_>>();
    let context = Box::new(SystemContext::<P> {
        callback: Box::new(f),
        terms,
        borrows: world.borrow_context(),
        pending_panic: world.panic_slot(),
    });
    let context = Box::into_raw(context).cast::<c_void>();

    // SAFETY: world is live, ids/inouts point to count initialized terms for this call,
    // and context remains owned by flecs until the registered free callback runs.
    let id = unsafe {
        sys::soul_ecs_system_init(
            world.as_ptr(),
            ids.as_ptr(),
            inouts.as_ptr(),
            ids.len()
                .try_into()
                .expect("system term count exceeds flecs limit"),
            Some(system_callback::<P>),
            context,
            Some(drop_system_context::<P>),
        )
    };

    if id == 0 {
        // SAFETY: flecs did not take ownership when system creation failed.
        unsafe { drop(Box::from_raw(context.cast::<SystemContext<P>>())) };
        panic!("failed to create system");
    }

    System {
        id,
        _world: world,
        _marker: PhantomData,
    }
}

pub struct System<'world, P> {
    id: sys::ecs_entity_t,
    _world: &'world World,
    _marker: PhantomData<P>,
}

impl<P> System<'_, P> {
    pub fn id(&self) -> sys::ecs_entity_t {
        self.id
    }
}

struct SystemContext<P: QueryParamInternal> {
    callback: Box<SystemEach<P>>,
    terms: Vec<Term>,
    borrows: BorrowContext,
    pending_panic: PanicSlot,
}

unsafe extern "C" fn system_callback<P>(iter: *mut sys::ecs_iter_t)
where
    P: QueryParamInternal,
{
    // SAFETY: flecs invokes this callback with the context pointer registered by build_system.
    let context = unsafe {
        sys::soul_ecs_iter_ctx(iter)
            .cast::<SystemContext<P>>()
            .as_mut()
    };
    let Some(context) = context else {
        return;
    };

    if context.pending_panic.borrow().is_some() {
        return;
    }

    // SAFETY: iter is the live iterator supplied by flecs for this callback invocation.
    let count = unsafe { sys::soul_ecs_iter_count(iter) };
    if count < 0 {
        context
            .pending_panic
            .borrow_mut()
            .replace(Box::new("system iterator returned a negative row count"));
        return;
    }

    for row in 0..count {
        // SAFETY: row is in bounds for the current system iterator result.
        let entity = unsafe { sys::soul_ecs_iter_entity(iter, row) };
        if entity == 0 {
            context
                .pending_panic
                .borrow_mut()
                .replace(Box::new("system iterator returned an invalid entity"));
            return;
        }

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guards = P::borrow_row(&context.borrows, entity, &context.terms);
            // SAFETY: row is in bounds, field terms match P, and borrow guards are active.
            let item = unsafe { P::fetch_system_row(iter, row) };
            (context.callback)(item);
        }));

        if let Err(payload) = result {
            context.pending_panic.borrow_mut().replace(payload);
            return;
        }
    }
}

unsafe extern "C" fn drop_system_context<P>(context: *mut c_void)
where
    P: QueryParamInternal,
{
    if !context.is_null() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: context was allocated with Box::into_raw in build_system for this P.
            unsafe { drop(Box::from_raw(context.cast::<SystemContext<P>>())) };
        }));

        if result.is_err() {
            // Panics from ctx_free would otherwise unwind through a C ABI cleanup callback.
            // There is no Rust caller that can resume the panic here, so abort to preserve
            // the no-unwind boundary contract.
            std::process::abort();
        }
    }
}
