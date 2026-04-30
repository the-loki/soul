// Covers raw world initialization and finalization through the sys crate.
#[test]
fn creates_and_frees_raw_world() {
    let world = unsafe { soul_ecs_sys::ecs_init() };
    assert!(!world.is_null());

    let result = unsafe { soul_ecs_sys::ecs_fini(world) };
    assert_eq!(result, 0);
}
