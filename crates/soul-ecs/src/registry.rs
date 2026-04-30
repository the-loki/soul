use std::any::{type_name, TypeId};
use std::collections::HashMap;
use std::ffi::CString;
use std::mem;

use soul_ecs_sys as sys;

#[derive(Clone, Copy)]
pub(crate) struct ComponentInfo {
    pub(crate) id: sys::ecs_id_t,
    pub(crate) size: usize,
}

#[derive(Default)]
pub(crate) struct Registry {
    components: HashMap<TypeId, ComponentInfo>,
}

impl Registry {
    pub(crate) fn component<T: Copy + 'static>(
        &mut self,
        world: *mut sys::ecs_world_t,
    ) -> ComponentInfo {
        let type_id = TypeId::of::<T>();
        if let Some(info) = self.components.get(&type_id) {
            return *info;
        }

        let name = CString::new(type_name::<T>()).expect("component type name contains nul byte");
        // SAFETY: world is the live flecs world owned by World, and name is a valid C string for this call.
        let id = unsafe {
            sys::soul_ecs_component_init(
                world,
                name.as_ptr(),
                mem::size_of::<T>(),
                mem::align_of::<T>(),
            )
        };
        assert_ne!(id, 0, "failed to register component");

        let info = ComponentInfo {
            id,
            size: mem::size_of::<T>(),
        };
        self.components.insert(type_id, info);
        info
    }
}
