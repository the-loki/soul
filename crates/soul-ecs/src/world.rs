use std::any::Any;
use std::cell::RefCell;
use std::panic::resume_unwind;
use std::rc::Rc;

use soul_ecs_sys as sys;

use crate::borrow::{BorrowContext, BorrowTracker, ComponentBorrowGuard};
use crate::entity::Entity;
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

    pub fn query<P: QueryParam>(&self) -> QueryBuilder<'_, P> {
        QueryBuilder::new(self)
    }

    pub fn system<P: QueryParam>(&self) -> SystemBuilder<'_, P> {
        SystemBuilder::new(self)
    }

    pub fn progress(&self) -> bool {
        // SAFETY: self.raw is a valid, live flecs world owned by World.
        let progressed = unsafe { sys::ecs_progress(self.raw, 0.0) };
        let pending_panic = self.pending_panic.borrow_mut().take();
        if let Some(payload) = pending_panic {
            resume_unwind(payload);
        }
        progressed
    }

    pub(crate) fn component_info<T: Copy + 'static>(&self) -> ComponentInfo {
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

    pub(crate) fn borrow_context(&self) -> BorrowContext {
        BorrowContext::new(Rc::clone(&self.borrows), self.raw)
    }

    pub(crate) fn panic_slot(&self) -> PanicSlot {
        Rc::clone(&self.pending_panic)
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
