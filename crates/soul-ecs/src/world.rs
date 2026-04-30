use soul_ecs_sys as sys;

pub struct World {
    raw: *mut sys::ecs_world_t,
}

impl World {
    pub fn new() -> Self {
        let raw = unsafe { sys::ecs_init() };
        assert!(!raw.is_null(), "failed to initialize flecs world");
        Self { raw }
    }

    pub fn as_ptr(&self) -> *mut sys::ecs_world_t {
        self.raw
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
            let result = unsafe { sys::ecs_fini(self.raw) };
            debug_assert_eq!(result, 0);
            self.raw = std::ptr::null_mut();
        }
    }
}
