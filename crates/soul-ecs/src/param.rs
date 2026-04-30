use std::marker::PhantomData;
use std::mem;

use soul_ecs_sys as sys;

use crate::borrow::ComponentBorrowGuard;
use crate::world::World;

#[derive(Clone, Copy)]
pub(crate) struct Term {
    pub(crate) id: sys::ecs_id_t,
    pub(crate) inout: i16,
    pub(crate) access: TermAccess,
}

#[derive(Clone, Copy)]
pub(crate) enum TermAccess {
    Shared,
    Mutable,
}

impl Term {
    fn shared<T: Copy + 'static>(world: &World) -> Self {
        let info = world.component_info::<T>();
        Self {
            id: info.id,
            inout: sys::ECS_IN,
            access: TermAccess::Shared,
        }
    }

    fn mutable<T: Copy + 'static>(world: &World) -> Self {
        let info = world.component_info::<T>();
        Self {
            id: info.id,
            inout: sys::ECS_INOUT,
            access: TermAccess::Mutable,
        }
    }
}

pub(crate) struct QueryBorrowGuards<'world> {
    guards: Vec<ComponentBorrowGuard<'world>>,
}

impl<'world> QueryBorrowGuards<'world> {
    fn new() -> Self {
        Self { guards: Vec::new() }
    }

    fn push(&mut self, world: &'world World, entity: sys::ecs_entity_t, term: Term) {
        match term.access {
            TermAccess::Shared => {
                self.guards.push(world.borrow_component(entity, term.id));
            }
            TermAccess::Mutable => {
                self.guards
                    .push(world.borrow_component_mut(entity, term.id, true));
            }
        }
    }
}

pub trait QueryParam {
    type Item<'row>;
}

pub(crate) trait QueryParamInternal: QueryParam {
    fn terms(world: &World) -> Vec<Term>;

    fn borrow_row<'world>(
        world: &'world World,
        entity: sys::ecs_entity_t,
        terms: &[Term],
    ) -> QueryBorrowGuards<'world>;

    unsafe fn fetch_row<'row>(
        iter: *const sys::soul_ecs_query_iter_t,
        row: i32,
    ) -> <Self as QueryParam>::Item<'row>;
}

pub(crate) struct Field<'row, T> {
    ptr: *mut T,
    _marker: PhantomData<&'row mut T>,
}

impl<'row, T> Field<'row, T> {
    unsafe fn from_iter(iter: *const sys::soul_ecs_query_iter_t, row: i32, index: i8) -> Self {
        // SAFETY: the caller ensures iter is a live query iterator currently positioned
        // on a result with row in bounds and index naming a T field in the query.
        let ptr = unsafe { sys::soul_ecs_query_iter_field(iter, mem::size_of::<T>(), index) };
        assert!(!ptr.is_null(), "query field is not set");
        Self {
            // SAFETY: flecs returns a contiguous field array for regular components,
            // and row was checked against the iterator result count by the caller.
            ptr: unsafe { ptr.cast::<T>().add(row as usize) },
            _marker: PhantomData,
        }
    }

    unsafe fn shared(self) -> &'row T {
        // SAFETY: row borrow guards reject active mutable borrows before this reference is created.
        unsafe { &*self.ptr.cast_const() }
    }

    unsafe fn mutable(self) -> &'row mut T {
        // SAFETY: row borrow guards reject active shared or mutable borrows before this reference is created.
        unsafe { &mut *self.ptr }
    }
}

impl<T: Copy + 'static> QueryParam for (&T,) {
    type Item<'row> = (&'row T,);
}

impl<T: Copy + 'static> QueryParamInternal for (&T,) {
    fn terms(world: &World) -> Vec<Term> {
        vec![Term::shared::<T>(world)]
    }

    fn borrow_row<'world>(
        world: &'world World,
        entity: sys::ecs_entity_t,
        terms: &[Term],
    ) -> QueryBorrowGuards<'world> {
        let mut guards = QueryBorrowGuards::new();
        guards.push(world, entity, terms[0]);
        guards
    }

    unsafe fn fetch_row<'row>(
        iter: *const sys::soul_ecs_query_iter_t,
        row: i32,
    ) -> <Self as QueryParam>::Item<'row> {
        // SAFETY: Query::each only calls this for matching rows after acquiring shared guards.
        (unsafe { Field::<T>::from_iter(iter, row, 0).shared() },)
    }
}

impl<T: Copy + 'static> QueryParam for (&mut T,) {
    type Item<'row> = (&'row mut T,);
}

impl<T: Copy + 'static> QueryParamInternal for (&mut T,) {
    fn terms(world: &World) -> Vec<Term> {
        vec![Term::mutable::<T>(world)]
    }

    fn borrow_row<'world>(
        world: &'world World,
        entity: sys::ecs_entity_t,
        terms: &[Term],
    ) -> QueryBorrowGuards<'world> {
        let mut guards = QueryBorrowGuards::new();
        guards.push(world, entity, terms[0]);
        guards
    }

    unsafe fn fetch_row<'row>(
        iter: *const sys::soul_ecs_query_iter_t,
        row: i32,
    ) -> <Self as QueryParam>::Item<'row> {
        // SAFETY: Query::each only calls this for matching rows after acquiring mutable guards.
        (unsafe { Field::<T>::from_iter(iter, row, 0).mutable() },)
    }
}

impl<T: Copy + 'static, U: Copy + 'static> QueryParam for (&mut T, &U) {
    type Item<'row> = (&'row mut T, &'row U);
}

impl<T: Copy + 'static, U: Copy + 'static> QueryParamInternal for (&mut T, &U) {
    fn terms(world: &World) -> Vec<Term> {
        vec![Term::mutable::<T>(world), Term::shared::<U>(world)]
    }

    fn borrow_row<'world>(
        world: &'world World,
        entity: sys::ecs_entity_t,
        terms: &[Term],
    ) -> QueryBorrowGuards<'world> {
        let mut guards = QueryBorrowGuards::new();
        guards.push(world, entity, terms[0]);
        guards.push(world, entity, terms[1]);
        guards
    }

    unsafe fn fetch_row<'row>(
        iter: *const sys::soul_ecs_query_iter_t,
        row: i32,
    ) -> <Self as QueryParam>::Item<'row> {
        // SAFETY: Query::each only calls this for matching rows after acquiring row guards.
        (
            unsafe { Field::<T>::from_iter(iter, row, 0).mutable() },
            unsafe { Field::<U>::from_iter(iter, row, 1).shared() },
        )
    }
}
