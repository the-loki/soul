use std::ffi::c_void;
use std::marker::PhantomData;
use std::panic::{catch_unwind, AssertUnwindSafe};

use soul_ecs_sys as sys;

use crate::borrow::BorrowContext;
use crate::entity::Entity;
use crate::param::{validate_unique_terms, QueryParam, QueryParamInternal, Term};
use crate::world::{PanicSlot, World};

type ObserverEach<P> = dyn for<'row> FnMut(<P as QueryParam>::Item<'row>);
type EntityObserverEach = dyn for<'entity> FnMut(Entity<'entity>);

pub struct ObserverBuilder<'world, P> {
    world: &'world World,
    _marker: PhantomData<P>,
}

impl<'world, P> ObserverBuilder<'world, P> {
    pub(crate) fn new(world: &'world World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }
}

impl<'world, 'param, T: Copy + 'static, U: Copy + 'static>
    ObserverBuilder<'world, (&'param T, &'param U)>
{
    pub fn event<E: Copy + 'static>(
        self,
    ) -> ObserverEventBuilder<'world, (&'param T, &'param U), E> {
        ObserverEventBuilder {
            world: self.world,
            _marker: PhantomData,
        }
    }
}

pub struct ObserverEventBuilder<'world, P, E> {
    world: &'world World,
    _marker: PhantomData<(P, E)>,
}

impl<'world, 'param, T: Copy + 'static, U: Copy + 'static, E: Copy + 'static>
    ObserverEventBuilder<'world, (&'param T, &'param U), E>
{
    pub fn each(
        self,
        f: impl for<'row> FnMut(<(&'param T, &'param U) as QueryParam>::Item<'row>) + 'static,
    ) -> Observer<'world, (&'param T, &'param U)> {
        build_observer::<(&'param T, &'param U), E, _>(self.world, f)
    }
}

fn build_observer<'world, P, E, F>(world: &'world World, f: F) -> Observer<'world, P>
where
    P: QueryParamInternal,
    E: Copy + 'static,
    F: for<'row> FnMut(P::Item<'row>) + 'static,
{
    world.assert_no_active_component_borrows();
    let event = world.component_info::<E>().id;
    let terms = P::terms(world);
    validate_unique_terms(&terms, "observer");
    let ids = terms.iter().map(|term| term.id).collect::<Vec<_>>();
    let inouts = terms.iter().map(|term| term.inout).collect::<Vec<_>>();
    let context = Box::new(ObserverContext::<P> {
        callback: Box::new(f),
        terms,
        borrows: world.borrow_context(),
        pending_panic: world.panic_slot(),
    });
    let context = Box::into_raw(context).cast::<c_void>();

    // SAFETY: world is live, ids/inouts point to count initialized terms for this call,
    // and context remains owned by flecs until the registered free callback runs.
    let id = unsafe {
        sys::soul_ecs_observer_init(
            world.as_ptr(),
            ids.as_ptr(),
            inouts.as_ptr(),
            ids.len()
                .try_into()
                .expect("observer term count exceeds flecs limit"),
            event,
            Some(observer_callback::<P>),
            context,
            Some(drop_observer_context::<P>),
        )
    };

    if id == 0 {
        // SAFETY: flecs did not take ownership when observer creation failed.
        unsafe { drop(Box::from_raw(context.cast::<ObserverContext<P>>())) };
        panic!("failed to create observer");
    }

    Observer {
        id,
        _world: world,
        _marker: PhantomData,
    }
}

pub struct Observer<'world, P> {
    id: sys::ecs_entity_t,
    _world: &'world World,
    _marker: PhantomData<P>,
}

impl<P> Observer<'_, P> {
    pub fn id(&self) -> sys::ecs_entity_t {
        self.id
    }
}

pub struct EntityObserver<'world> {
    id: sys::ecs_entity_t,
    _world: &'world World,
}

impl EntityObserver<'_> {
    pub fn id(&self) -> sys::ecs_entity_t {
        self.id
    }
}

struct ObserverContext<P: QueryParamInternal> {
    callback: Box<ObserverEach<P>>,
    terms: Vec<Term>,
    borrows: BorrowContext,
    pending_panic: PanicSlot,
}

unsafe extern "C" fn observer_callback<P>(iter: *mut sys::ecs_iter_t)
where
    P: QueryParamInternal,
{
    // SAFETY: flecs invokes this callback with the context pointer registered by build_observer.
    let context = unsafe {
        sys::soul_ecs_iter_ctx(iter)
            .cast::<ObserverContext<P>>()
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
            .replace(Box::new("observer iterator returned a negative row count"));
        return;
    }

    for row in 0..count {
        // SAFETY: row is in bounds for the current observer iterator result.
        let entity = unsafe { sys::soul_ecs_iter_entity(iter, row) };
        if entity == 0 {
            context
                .pending_panic
                .borrow_mut()
                .replace(Box::new("observer iterator returned an invalid entity"));
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

unsafe extern "C" fn drop_observer_context<P>(context: *mut c_void)
where
    P: QueryParamInternal,
{
    if !context.is_null() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: context was allocated with Box::into_raw in build_observer for this P.
            unsafe { drop(Box::from_raw(context.cast::<ObserverContext<P>>())) };
        }));

        if result.is_err() {
            // Panics from ctx_free would otherwise unwind through a C ABI cleanup callback.
            std::process::abort();
        }
    }
}

pub(crate) fn build_entity_observer<'world, E, F>(
    world: &'world World,
    entity: sys::ecs_entity_t,
    f: F,
) -> EntityObserver<'world>
where
    E: Copy + 'static,
    F: for<'entity> FnMut(Entity<'entity>) + 'static,
{
    world.assert_no_active_component_borrows();
    let event = world.component_info::<E>().id;
    let context = Box::new(EntityObserverContext {
        callback: Box::new(f),
        world: world as *const World,
        pending_panic: world.panic_slot(),
    });
    let context = Box::into_raw(context).cast::<c_void>();

    // SAFETY: world and entity are live, and context remains owned by flecs
    // until the registered free callback runs.
    let id = unsafe {
        sys::soul_ecs_entity_observer_init(
            world.as_ptr(),
            event,
            entity,
            Some(entity_observer_callback),
            context,
            Some(drop_entity_observer_context),
        )
    };

    if id == 0 {
        // SAFETY: flecs did not take ownership when observer creation failed.
        unsafe { drop(Box::from_raw(context.cast::<EntityObserverContext>())) };
        panic!("failed to create entity observer");
    }

    EntityObserver { id, _world: world }
}

struct EntityObserverContext {
    callback: Box<EntityObserverEach>,
    world: *const World,
    pending_panic: PanicSlot,
}

unsafe extern "C" fn entity_observer_callback(iter: *mut sys::ecs_iter_t) {
    // SAFETY: flecs invokes this callback with the context pointer registered by build_entity_observer.
    let context = unsafe {
        sys::soul_ecs_iter_ctx(iter)
            .cast::<EntityObserverContext>()
            .as_mut()
    };
    let Some(context) = context else {
        return;
    };

    if context.pending_panic.borrow().is_some() {
        return;
    }

    // SAFETY: iter is the live iterator supplied by flecs for this callback invocation.
    let src = unsafe { sys::soul_ecs_iter_field_src(iter, 0) };
    if src == 0 {
        context.pending_panic.borrow_mut().replace(Box::new(
            "entity observer iterator returned an invalid source",
        ));
        return;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: world points to the World that registered this observer and lives
        // at least as long as the returned EntityObserver handle.
        let world = unsafe { &*context.world };
        (context.callback)(Entity::new(world, src));
    }));

    if let Err(payload) = result {
        context.pending_panic.borrow_mut().replace(payload);
    }
}

unsafe extern "C" fn drop_entity_observer_context(context: *mut c_void) {
    if !context.is_null() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: context was allocated with Box::into_raw in build_entity_observer.
            unsafe { drop(Box::from_raw(context.cast::<EntityObserverContext>())) };
        }));

        if result.is_err() {
            // Panics from ctx_free would otherwise unwind through a C ABI cleanup callback.
            std::process::abort();
        }
    }
}
