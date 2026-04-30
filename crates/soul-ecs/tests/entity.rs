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

#[derive(Clone, Copy, Debug, PartialEq)]
struct Tag;

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

// Covers shared reentrant reads for one entity/component pair.
#[test]
fn entity_allows_reentrant_shared_component_get() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    entity.get::<Position>(|outer| {
        entity.get::<Position>(|inner| {
            assert_eq!(*outer, *inner);
        });
    });
}

// Covers reentrant mutable access rejection while a shared borrow is active.
#[test]
fn entity_rejects_mutable_get_during_shared_get() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.get_mut::<Position>(|position| {
                position.x += 1.0;
            });
        });
    }));

    assert!(result.is_err());
}

// Covers reentrant shared access rejection while a mutable borrow is active.
#[test]
fn entity_rejects_shared_get_during_mutable_get() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get_mut::<Position>(|_| {
            entity.get::<Position>(|position| {
                assert_eq!(*position, Position { x: 1.0, y: 2.0 });
            });
        });
    }));

    assert!(result.is_err());
}

// Covers reentrant mutable access rejection while a mutable borrow is active.
#[test]
fn entity_rejects_mutable_get_during_mutable_get() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get_mut::<Position>(|_| {
            entity.get_mut::<Position>(|position| {
                position.x += 1.0;
            });
        });
    }));

    assert!(result.is_err());
}

// Covers structural mutation rejection while any component borrow is active.
#[test]
fn entity_rejects_set_during_component_get() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.set(Velocity { x: 3.0, y: 4.0 });
        });
    }));

    assert!(result.is_err());
    assert!(!entity.has::<Velocity>());
}

// Covers tag-only add while data components must be initialized through set.
#[test]
fn entity_add_only_accepts_zero_sized_tag_components() {
    let world = World::new();
    let entity = world.entity().add::<Tag>();

    assert!(entity.has::<Tag>());

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.add::<Position>();
    }));

    assert!(result.is_err());
}

// Covers borrow cleanup and modified notification when get_mut unwinds.
#[test]
fn entity_get_mut_releases_borrow_after_panic() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get_mut::<Position>(|position| {
            position.x = 5.0;
            panic!("forced panic");
        });
    }));

    assert!(result.is_err());

    entity.get_mut::<Position>(|position| {
        assert_eq!(*position, Position { x: 5.0, y: 2.0 });
        position.y = 8.0;
    });

    entity.get::<Position>(|position| {
        assert_eq!(*position, Position { x: 5.0, y: 8.0 });
    });
}
