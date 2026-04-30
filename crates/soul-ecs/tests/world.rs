use soul_ecs::World;

// Covers safe world creation and drop through the public facade.
#[test]
fn creates_and_drops_world() {
    let world = World::new();
    let entity = world.entity();
    assert_ne!(entity.id(), 0);

    let default_world = World::default();
    let default_entity = default_world.entity();
    assert_ne!(default_entity.id(), 0);
}
