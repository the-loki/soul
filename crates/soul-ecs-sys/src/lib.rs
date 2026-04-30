#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_void};

pub enum ecs_world_t {}
pub enum ecs_query_t {}
pub enum ecs_iter_t {}
pub enum soul_ecs_query_iter_t {}

pub type ecs_entity_t = u64;
pub type ecs_id_t = u64;
pub type ecs_ftime_t = f32;
pub type ecs_iter_action_t = Option<unsafe extern "C" fn(*mut ecs_iter_t)>;
pub type ecs_ctx_free_t = Option<unsafe extern "C" fn(*mut c_void)>;

pub const ECS_INOUT_DEFAULT: i16 = 0;
pub const ECS_INOUT_NONE: i16 = 1;
pub const ECS_INOUT_FILTER: i16 = 2;
pub const ECS_INOUT: i16 = 3;
pub const ECS_IN: i16 = 4;
pub const ECS_OUT: i16 = 5;

extern "C" {
    pub fn ecs_init() -> *mut ecs_world_t;
    pub fn ecs_fini(world: *mut ecs_world_t) -> i32;
    pub fn ecs_progress(world: *mut ecs_world_t, delta_time: ecs_ftime_t) -> bool;
    pub fn ecs_new(world: *mut ecs_world_t) -> ecs_entity_t;
    pub fn ecs_add_id(world: *mut ecs_world_t, entity: ecs_entity_t, id: ecs_id_t);
    pub fn ecs_remove_id(world: *mut ecs_world_t, entity: ecs_entity_t, id: ecs_id_t);
    pub fn ecs_has_id(world: *const ecs_world_t, entity: ecs_entity_t, id: ecs_id_t) -> bool;
    pub fn ecs_set_id(
        world: *mut ecs_world_t,
        entity: ecs_entity_t,
        id: ecs_id_t,
        size: usize,
        ptr: *const c_void,
    );
    pub fn ecs_get_id(
        world: *const ecs_world_t,
        entity: ecs_entity_t,
        id: ecs_id_t,
    ) -> *const c_void;
    pub fn ecs_get_mut_id(
        world: *const ecs_world_t,
        entity: ecs_entity_t,
        id: ecs_id_t,
    ) -> *mut c_void;
    pub fn ecs_modified_id(world: *mut ecs_world_t, entity: ecs_entity_t, id: ecs_id_t);
    pub fn ecs_query_fini(query: *mut ecs_query_t);

    pub fn soul_ecs_component_init(
        world: *mut ecs_world_t,
        name: *const c_char,
        size: usize,
        alignment: usize,
    ) -> ecs_entity_t;
    pub fn soul_ecs_query_init(
        world: *mut ecs_world_t,
        ids: *const ecs_id_t,
        inouts: *const i16,
        count: i32,
    ) -> *mut ecs_query_t;
    pub fn soul_ecs_query_iter(
        world: *const ecs_world_t,
        query: *const ecs_query_t,
    ) -> *mut soul_ecs_query_iter_t;
    pub fn soul_ecs_query_next(iter: *mut soul_ecs_query_iter_t) -> bool;
    pub fn soul_ecs_query_iter_count(iter: *const soul_ecs_query_iter_t) -> i32;
    pub fn soul_ecs_query_iter_field(
        iter: *const soul_ecs_query_iter_t,
        size: usize,
        index: i8,
    ) -> *mut c_void;
    pub fn soul_ecs_query_iter_entity(iter: *const soul_ecs_query_iter_t, row: i32)
        -> ecs_entity_t;
    pub fn soul_ecs_query_iter_fini(iter: *mut soul_ecs_query_iter_t);
    pub fn soul_ecs_system_init(
        world: *mut ecs_world_t,
        ids: *const ecs_id_t,
        inouts: *const i16,
        count: i32,
        callback: ecs_iter_action_t,
        ctx: *mut c_void,
        ctx_free: ecs_ctx_free_t,
    ) -> ecs_entity_t;
    pub fn soul_ecs_iter_count(iter: *const ecs_iter_t) -> i32;
    pub fn soul_ecs_iter_field(iter: *const ecs_iter_t, size: usize, index: i8) -> *mut c_void;
    pub fn soul_ecs_iter_entity(iter: *const ecs_iter_t, row: i32) -> ecs_entity_t;
    pub fn soul_ecs_iter_ctx(iter: *const ecs_iter_t) -> *mut c_void;
}
