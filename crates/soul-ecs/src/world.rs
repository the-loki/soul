use std::any::Any;
use std::cell::RefCell;
use std::ffi::c_void;
use std::panic::resume_unwind;
use std::rc::Rc;

use soul_ecs_sys as sys;

use crate::borrow::{BorrowContext, BorrowTracker, ComponentBorrowGuard};
use crate::entity::Entity;
use crate::observer::ObserverBuilder;
use crate::param::QueryParam;
use crate::query::QueryBuilder;
use crate::registry::{ComponentInfo, Registry};
use crate::system::SystemBuilder;

pub(crate) type PanicSlot = Rc<RefCell<Option<Box<dyn Any + Send>>>>;

pub struct World {
    raw: *mut sys::ecs_world_t,
    registry: RefCell<Registry>,
    borrows: Rc<RefCell<BorrowTracker>>,
    pending_panic: PanicSlot,
}

impl World {
    pub fn new() -> Self {
        // SAFETY: ecs_init creates a new flecs world and returns ownership to the caller.
        let raw = unsafe { sys::ecs_init() };
        assert!(!raw.is_null(), "failed to initialize flecs world");
        Self {
            raw,
            registry: RefCell::new(Registry::default()),
            borrows: Rc::new(RefCell::new(BorrowTracker::default())),
            pending_panic: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut sys::ecs_world_t {
        self.raw
    }

    pub fn entity(&self) -> Entity<'_> {
        // SAFETY: self.raw is a valid, live flecs world owned by World.
        let raw = unsafe { sys::ecs_new(self.raw) };
        assert_ne!(raw, 0, "failed to create entity");
        Entity::new(self, raw)
    }

    pub fn entity_from_id(&self, id: sys::ecs_entity_t) -> Entity<'_> {
        assert_ne!(id, 0, "entity id must not be zero");
        // SAFETY: self.raw is a valid, live flecs world owned by World.
        assert!(
            unsafe { sys::ecs_is_alive(self.raw, id) },
            "entity id is not alive in this world"
        );
        Entity::new(self, id)
    }

    pub fn query<P: QueryParam>(&self) -> QueryBuilder<'_, P> {
        QueryBuilder::new(self)
    }

    pub fn system<P: QueryParam>(&self) -> SystemBuilder<'_, P> {
        SystemBuilder::new(self)
    }

    pub fn observer<P: QueryParam>(&self) -> ObserverBuilder<'_, P> {
        ObserverBuilder::new(self)
    }

    pub fn bulk_empty(&self, count: usize) -> Vec<Entity<'_>> {
        self.assert_no_active_component_borrows();
        self.bulk_init(&[], std::ptr::null_mut(), count)
    }

    pub fn bulk_with1<T: Copy + Default + 'static>(&self, count: usize) -> Vec<Entity<'_>> {
        self.assert_no_active_component_borrows();
        let info = self.component_info::<T>();
        let mut values = vec![T::default(); count];
        let mut data = [values.as_mut_ptr().cast::<c_void>()];
        self.bulk_init(&[info.id], data.as_mut_ptr(), count)
    }

    pub fn bulk_with2<T: Copy + Default + 'static, U: Copy + Default + 'static>(
        &self,
        count: usize,
    ) -> Vec<Entity<'_>> {
        self.assert_no_active_component_borrows();
        let first = self.component_info::<T>();
        let second = self.component_info::<U>();
        let mut first_values = vec![T::default(); count];
        let mut second_values = vec![U::default(); count];
        let mut data = [
            first_values.as_mut_ptr().cast::<c_void>(),
            second_values.as_mut_ptr().cast::<c_void>(),
        ];
        self.bulk_init(&[first.id, second.id], data.as_mut_ptr(), count)
    }

    pub fn bulk_with3<
        T: Copy + Default + 'static,
        U: Copy + Default + 'static,
        V: Copy + Default + 'static,
    >(
        &self,
        count: usize,
    ) -> Vec<Entity<'_>> {
        self.assert_no_active_component_borrows();
        let first = self.component_info::<T>();
        let second = self.component_info::<U>();
        let third = self.component_info::<V>();
        let mut first_values = vec![T::default(); count];
        let mut second_values = vec![U::default(); count];
        let mut third_values = vec![V::default(); count];
        let mut data = [
            first_values.as_mut_ptr().cast::<c_void>(),
            second_values.as_mut_ptr().cast::<c_void>(),
            third_values.as_mut_ptr().cast::<c_void>(),
        ];
        self.bulk_init(&[first.id, second.id, third.id], data.as_mut_ptr(), count)
    }

    pub fn progress(&self) -> bool {
        // SAFETY: self.raw is a valid, live flecs world owned by World.
        let progressed = unsafe { sys::ecs_progress(self.raw, 0.0) };
        self.resume_pending_panic();
        progressed
    }

    pub fn defer_begin(&self) -> bool {
        // SAFETY: self.raw is a valid, live flecs world owned by World.
        unsafe { sys::ecs_defer_begin(self.raw) }
    }

    pub fn defer_end(&self) -> bool {
        // SAFETY: self.raw is a valid, live flecs world owned by World.
        let result = unsafe { sys::ecs_defer_end(self.raw) };
        self.resume_pending_panic();
        result
    }

    pub(crate) fn component_info<T: Copy + 'static>(&self) -> ComponentInfo {
        if let Some(info) = self.registry.borrow().get::<T>() {
            return info;
        }

        self.assert_no_active_component_borrows();
        self.registry.borrow_mut().component::<T>(self.raw)
    }

    pub(crate) fn borrow_component(
        &self,
        entity: sys::ecs_entity_t,
        component: sys::ecs_id_t,
    ) -> ComponentBorrowGuard {
        ComponentBorrowGuard::shared(Rc::clone(&self.borrows), entity, component)
    }

    pub(crate) fn borrow_component_mut(
        &self,
        entity: sys::ecs_entity_t,
        component: sys::ecs_id_t,
        notify_modified: bool,
    ) -> ComponentBorrowGuard {
        ComponentBorrowGuard::mutable(
            Rc::clone(&self.borrows),
            self.raw,
            entity,
            component,
            notify_modified,
        )
    }

    pub(crate) fn assert_can_mutate_structure(&self, entity: sys::ecs_entity_t) {
        self.borrows.borrow().assert_can_mutate_structure(entity);
    }

    pub(crate) fn assert_no_active_component_borrows(&self) {
        self.borrows.borrow().assert_no_active_component_borrows();
    }

    pub(crate) fn borrow_context(&self) -> BorrowContext {
        BorrowContext::new(Rc::clone(&self.borrows), self.raw)
    }

    pub(crate) fn panic_slot(&self) -> PanicSlot {
        Rc::clone(&self.pending_panic)
    }

    pub(crate) fn resume_pending_panic(&self) {
        let pending_panic = self.pending_panic.borrow_mut().take();
        if let Some(payload) = pending_panic {
            resume_unwind(payload);
        }
    }

    fn bulk_init(
        &self,
        ids: &[sys::ecs_id_t],
        data: *mut *mut c_void,
        count: usize,
    ) -> Vec<Entity<'_>> {
        let entity_count: i32 = count.try_into().expect("bulk entity count exceeds i32");
        let id_count: i32 = ids
            .len()
            .try_into()
            .expect("bulk component count exceeds i32");
        let mut out = vec![0; count];
        // SAFETY: self.raw is live, ids/data describe id_count entries for this call,
        // and out has room for entity_count returned ids.
        let ok = unsafe {
            sys::soul_ecs_bulk_init(
                self.raw,
                ids.as_ptr(),
                data,
                id_count,
                entity_count,
                out.as_mut_ptr(),
            )
        };
        assert!(ok, "failed to bulk create entities");
        out.into_iter().map(|id| Entity::new(self, id)).collect()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for World {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: self.raw is a valid, live flecs world uniquely owned by World and has not been finalized before.
            let result = unsafe { sys::ecs_fini(self.raw) };
            debug_assert_eq!(result, 0);
            self.raw = std::ptr::null_mut();
        }
    }
}
