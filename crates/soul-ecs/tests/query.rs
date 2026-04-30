use soul_ecs::World;
use std::panic::{catch_unwind, AssertUnwindSafe};

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

// Covers typed query iteration over mutable and readonly fields.
#[test]
fn query_each_updates_matching_entities() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    let query = world.query::<(&mut Position, &Velocity)>().build();
    query.each(|(position, velocity)| {
        position.x += velocity.x;
        position.y += velocity.y;
    });

    entity.get::<Position>(|position| {
        assert_eq!(*position, Position { x: 11.0, y: 22.0 });
    });
}

// Covers query mutable borrows rejecting reentrant shared entity access.
#[test]
fn query_each_rejects_shared_get_during_mutable_field() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    let query = world.query::<(&mut Position,)>().build();
    let result = catch_unwind(AssertUnwindSafe(|| {
        query.each(|(position,)| {
            position.x = 5.0;
            entity.get::<Position>(|position| {
                assert_eq!(*position, Position { x: 5.0, y: 2.0 });
            });
        });
    }));

    assert!(result.is_err());

    entity.get_mut::<Position>(|position| {
        assert_eq!(*position, Position { x: 5.0, y: 2.0 });
        position.y = 8.0;
    });
}

// Covers query shared borrows rejecting reentrant mutable entity access.
#[test]
fn query_each_rejects_mutable_get_during_shared_field() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    let query = world.query::<(&Position,)>().build();
    let result = catch_unwind(AssertUnwindSafe(|| {
        query.each(|(position,)| {
            assert_eq!(*position, Position { x: 1.0, y: 2.0 });
            entity.get_mut::<Position>(|position| {
                position.x = 5.0;
            });
        });
    }));

    assert!(result.is_err());

    entity.get_mut::<Position>(|position| {
        assert_eq!(*position, Position { x: 1.0, y: 2.0 });
        position.y = 8.0;
    });
}

// Covers rejecting duplicate component fields before query iteration starts.
#[test]
fn query_build_rejects_duplicate_components() {
    let world = World::new();

    let result = catch_unwind(AssertUnwindSafe(|| {
        world.query::<(&mut Position, &Position)>().build();
    }));

    assert!(result.is_err());
}
