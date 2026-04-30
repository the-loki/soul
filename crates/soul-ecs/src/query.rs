use std::marker::PhantomData;

use soul_ecs_sys as sys;

use crate::param::{validate_unique_terms, QueryParam, QueryParamInternal, Term};
use crate::world::World;

pub struct QueryBuilder<'world, P> {
    world: &'world World,
    _marker: PhantomData<P>,
}

impl<'world, P> QueryBuilder<'world, P> {
    pub(crate) fn new(world: &'world World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }
}

impl<'world, 'param, T: Copy + 'static> QueryBuilder<'world, (&'param T,)> {
    pub fn build(self) -> Query<'world, (&'param T,)> {
        build_query(self.world)
    }
}

impl<'world, 'param, T: Copy + 'static> QueryBuilder<'world, (&'param mut T,)> {
    pub fn build(self) -> Query<'world, (&'param mut T,)> {
        build_query(self.world)
    }
}

impl<'world, 'param, T: Copy + 'static, U: Copy + 'static>
    QueryBuilder<'world, (&'param T, &'param U)>
{
    pub fn build(self) -> Query<'world, (&'param T, &'param U)> {
        build_query(self.world)
    }
}

impl<'world, 'param, T: Copy + 'static, U: Copy + 'static>
    QueryBuilder<'world, (&'param mut T, &'param U)>
{
    pub fn build(self) -> Query<'world, (&'param mut T, &'param U)> {
        build_query(self.world)
    }
}

fn build_query<P>(world: &World) -> Query<'_, P>
where
    P: QueryParamInternal,
{
    let terms = P::terms(world);
    validate_unique_terms(&terms, "query");
    let ids = terms.iter().map(|term| term.id).collect::<Vec<_>>();
    let inouts = terms.iter().map(|term| term.inout).collect::<Vec<_>>();
    // SAFETY: world is live, and ids/inouts point to count initialized terms for this call.
    let raw = unsafe {
        sys::soul_ecs_query_init(
            world.as_ptr(),
            ids.as_ptr(),
            inouts.as_ptr(),
            ids.len()
                .try_into()
                .expect("query term count exceeds flecs limit"),
        )
    };
    assert!(!raw.is_null(), "failed to create query");

    Query {
        world,
        raw,
        terms,
        _marker: PhantomData,
    }
}

pub struct Query<'world, P> {
    world: &'world World,
    raw: *mut sys::ecs_query_t,
    terms: Vec<Term>,
    _marker: PhantomData<P>,
}

impl<'param, T: Copy + 'static> Query<'_, (&'param T,)> {
    pub fn each(&self, f: impl for<'row> FnMut(<(&'param T,) as QueryParam>::Item<'row>)) {
        each_query(self, f);
    }
}

impl<'param, T: Copy + 'static> Query<'_, (&'param mut T,)> {
    pub fn each(&self, f: impl for<'row> FnMut(<(&'param mut T,) as QueryParam>::Item<'row>)) {
        each_query(self, f);
    }
}

impl<'param, T: Copy + 'static, U: Copy + 'static> Query<'_, (&'param T, &'param U)> {
    pub fn each(
        &self,
        f: impl for<'row> FnMut(<(&'param T, &'param U) as QueryParam>::Item<'row>),
    ) {
        each_query(self, f);
    }
}

impl<'param, T: Copy + 'static, U: Copy + 'static> Query<'_, (&'param mut T, &'param U)> {
    pub fn each(
        &self,
        f: impl for<'row> FnMut(<(&'param mut T, &'param U) as QueryParam>::Item<'row>),
    ) {
        each_query(self, f);
    }
}

fn each_query<P>(query: &Query<'_, P>, mut f: impl for<'row> FnMut(P::Item<'row>))
where
    P: QueryParamInternal,
{
    // SAFETY: query.raw is a live query created for query.world.
    let iter = unsafe { sys::soul_ecs_query_iter(query.world.as_ptr(), query.raw) };
    assert!(!iter.is_null(), "failed to create query iterator");
    let _iter_guard = QueryIterGuard { raw: iter };

    // SAFETY: iter remains live until _iter_guard is dropped.
    while unsafe { sys::soul_ecs_query_next(iter) } {
        // SAFETY: iter is currently positioned on a query result.
        let count = unsafe { sys::soul_ecs_query_iter_count(iter) };
        assert!(count >= 0, "query iterator returned a negative row count");

        for row in 0..count {
            // SAFETY: row is in bounds for the current query result.
            let entity = unsafe { sys::soul_ecs_query_iter_entity(iter, row) };
            assert_ne!(entity, 0, "query iterator returned an invalid entity");

            let _guards = P::borrow_row(&query.world.borrow_context(), entity, &query.terms);
            // SAFETY: row is in bounds, field terms match P, and borrow guards are active.
            let item = unsafe { P::fetch_row(iter, row) };
            f(item);
        }
    }
}

impl<P> Drop for Query<'_, P> {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: self.raw is owned by this Query and has not been finalized before.
            unsafe { sys::ecs_query_fini(self.raw) };
            self.raw = std::ptr::null_mut();
        }
    }
}

struct QueryIterGuard {
    raw: *mut sys::soul_ecs_query_iter_t,
}

impl Drop for QueryIterGuard {
    fn drop(&mut self) {
        // SAFETY: raw is either null or a query iterator wrapper returned by soul_ecs_query_iter.
        unsafe { sys::soul_ecs_query_iter_fini(self.raw) };
    }
}
