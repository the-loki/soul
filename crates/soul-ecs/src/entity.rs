use std::mem;

use soul_ecs_sys as sys;

use crate::world::World;

#[derive(Clone, Copy)]
pub struct Entity<'world> {
    world: &'world World,
    raw: sys::ecs_entity_t,
}

impl<'world> Entity<'world> {
    pub(crate) fn new(world: &'world World, raw: sys::ecs_entity_t) -> Self {
        Self { world, raw }
    }

    pub fn id(&self) -> sys::ecs_entity_t {
        self.raw
    }

    pub fn add<T: Copy + 'static>(self) -> Self {
        assert!(
            mem::size_of::<T>() == 0,
            "add can only be used with zero-sized tag components; use set for data components"
        );
        let info = self.world.component_info::<T>();
        let _guard = self.world.borrow_component_mut(self.raw, info.id, false);
        // SAFETY: self.raw is an entity created in this world, and info.id is registered for the same world.
        unsafe { sys::ecs_add_id(self.world.as_ptr(), self.raw, info.id) };
        self
    }

    pub fn set<T: Copy + 'static>(self, value: T) -> Self {
        if mem::size_of::<T>() == 0 {
            return self.add::<T>();
        }

        let info = self.world.component_info::<T>();
        let _guard = self.world.borrow_component_mut(self.raw, info.id, false);
        // SAFETY: self.raw is an entity created in this world, info.id is registered for T,
        // and value points to a valid T with the size passed to flecs for the duration of the call.
        unsafe {
            sys::ecs_set_id(
                self.world.as_ptr(),
                self.raw,
                info.id,
                info.size,
                (&value as *const T).cast(),
            );
        }
        self
    }

    pub fn has<T: Copy + 'static>(&self) -> bool {
        let info = self.world.component_info::<T>();
        // SAFETY: self.raw is an entity created in this world, and info.id is registered for the same world.
        unsafe { sys::ecs_has_id(self.world.as_ptr(), self.raw, info.id) }
    }

    pub fn get<T: Copy + 'static>(&self, f: impl FnOnce(&T)) {
        let info = self.world.component_info::<T>();
        // SAFETY: self.raw is an entity created in this world, and info.id is registered for T.
        let ptr = unsafe { sys::ecs_get_id(self.world.as_ptr(), self.raw, info.id) };
        assert!(!ptr.is_null(), "component does not exist on entity");
        let _guard = self.world.borrow_component(self.raw, info.id);
        unsafe {
            // SAFETY: flecs returned a non-null pointer for component T on this entity.
            f(&*(ptr.cast::<T>()));
        }
    }

    pub fn get_mut<T: Copy + 'static>(&self, f: impl FnOnce(&mut T)) {
        let info = self.world.component_info::<T>();
        // SAFETY: self.raw is an entity created in this world, and info.id is registered for T.
        let ptr = unsafe { sys::ecs_get_mut_id(self.world.as_ptr(), self.raw, info.id) };
        assert!(!ptr.is_null(), "component does not exist on entity");
        let _guard = self.world.borrow_component_mut(self.raw, info.id, true);
        unsafe {
            // SAFETY: flecs returned a unique mutable pointer for component T on this entity.
            f(&mut *(ptr.cast::<T>()));
        }
    }

    pub fn remove<T: Copy + 'static>(self) -> Self {
        let info = self.world.component_info::<T>();
        let _guard = self.world.borrow_component_mut(self.raw, info.id, false);
        // SAFETY: self.raw is an entity created in this world, and info.id is registered for the same world.
        unsafe { sys::ecs_remove_id(self.world.as_ptr(), self.raw, info.id) };
        self
    }
}
