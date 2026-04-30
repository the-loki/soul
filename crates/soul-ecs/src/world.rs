use std::cell::RefCell;

use soul_ecs_sys as sys;

use crate::borrow::{BorrowTracker, ComponentBorrowGuard};
use crate::entity::Entity;
use crate::registry::{ComponentInfo, Registry};

pub struct World {
    raw: *mut sys::ecs_world_t,
    registry: RefCell<Registry>,
    borrows: RefCell<BorrowTracker>,
}

impl World {
    pub fn new() -> Self {
        // SAFETY: ecs_init creates a new flecs world and returns ownership to the caller.
        let raw = unsafe { sys::ecs_init() };
        assert!(!raw.is_null(), "failed to initialize flecs world");
        Self {
            raw,
            registry: RefCell::new(Registry::default()),
            borrows: RefCell::new(BorrowTracker::default()),
        }
    }

    pub fn as_ptr(&self) -> *mut sys::ecs_world_t {
        self.raw
    }

    pub fn entity(&self) -> Entity<'_> {
        // SAFETY: self.raw is a valid, live flecs world owned by World.
        let raw = unsafe { sys::ecs_new(self.raw) };
        assert_ne!(raw, 0, "failed to create entity");
        Entity::new(self, raw)
    }

    pub(crate) fn component_info<T: Copy + 'static>(&self) -> ComponentInfo {
        self.registry.borrow_mut().component::<T>(self.raw)
    }

    pub(crate) fn borrow_component(
        &self,
        entity: sys::ecs_entity_t,
        component: sys::ecs_id_t,
    ) -> ComponentBorrowGuard<'_> {
        ComponentBorrowGuard::shared(&self.borrows, entity, component)
    }

    pub(crate) fn borrow_component_mut(
        &self,
        entity: sys::ecs_entity_t,
        component: sys::ecs_id_t,
        notify_modified: bool,
    ) -> ComponentBorrowGuard<'_> {
        ComponentBorrowGuard::mutable(&self.borrows, self.raw, entity, component, notify_modified)
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
