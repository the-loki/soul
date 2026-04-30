use soul_ecs::World;

// Covers safe world creation, raw pointer ownership, and drop.
#[test]
fn creates_and_drops_world() {
    let world = World::new();
    assert!(!world.as_ptr().is_null());
}
