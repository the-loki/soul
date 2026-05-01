use soul_ecs::World;

#[derive(Clone, Copy, Default)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Default)]
struct Velocity {
    x: f32,
    y: f32,
}

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

// Covers bulk creation with typed default component data.
#[test]
fn bulk_create_entities_with_two_components() {
    let world = World::new();
    let entities = world.bulk_with2::<Position, Velocity>(4);
    assert_eq!(entities.len(), 4);

    let query = world.query::<(&Position, &Velocity)>().build();
    let mut count = 0;
    query.each(|(position, velocity)| {
        assert_eq!(position.x, 0.0);
        assert_eq!(position.y, 0.0);
        assert_eq!(velocity.x, 0.0);
        assert_eq!(velocity.y, 0.0);
        count += 1;
    });

    assert_eq!(count, 4);
}
