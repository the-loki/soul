use soul_ecs::World;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
}

// Covers set, has, get, get_mut, and remove for one component.
#[test]
fn entity_manages_single_component() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    assert!(entity.has::<Position>());

    entity.get::<Position>(|position| {
        assert_eq!(*position, Position { x: 1.0, y: 2.0 });
    });

    entity.get_mut::<Position>(|position| {
        position.x += 3.0;
    });

    entity.get::<Position>(|position| {
        assert_eq!(*position, Position { x: 4.0, y: 2.0 });
    });

    let entity = entity.remove::<Position>();
    assert!(!entity.has::<Position>());
}

// Covers C++-style chained entity construction with multiple components.
#[test]
fn entity_supports_chained_component_set() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    assert!(entity.has::<Position>());
    assert!(entity.has::<Velocity>());
}
