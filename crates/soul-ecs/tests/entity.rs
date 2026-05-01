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

#[derive(Clone, Copy, Debug, PartialEq)]
struct UnregisteredTag;

#[derive(Clone, Copy, Debug, PartialEq)]
struct UnregisteredComponent {
    value: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EmptyEvent;

fn next_entity_id_after_position_entity() -> u64 {
    let world = World::new();
    let _entity = world.entity().set(Position { x: 1.0, y: 2.0 });
    world.entity().id()
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
    let expected_next_entity_id = next_entity_id_after_position_entity();

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.set(Velocity { x: 3.0, y: 4.0 });
        });
    }));

    assert!(result.is_err());
    assert_eq!(world.entity().id(), expected_next_entity_id);
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

// Covers structural add rejection before registering an unregistered tag.
#[test]
fn entity_rejects_add_during_component_get_without_registering_tag() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });
    let expected_next_entity_id = next_entity_id_after_position_entity();

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.add::<Tag>();
        });
    }));

    assert!(result.is_err());
    assert_eq!(world.entity().id(), expected_next_entity_id);
}

// Covers zero-sized set rejection before delegating to tag registration.
#[test]
fn entity_rejects_tag_set_during_component_get_without_registering_tag() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });
    let expected_next_entity_id = next_entity_id_after_position_entity();

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.set(Tag);
        });
    }));

    assert!(result.is_err());
    assert_eq!(world.entity().id(), expected_next_entity_id);
}

// Covers structural remove rejection before registering an unregistered component.
#[test]
fn entity_rejects_remove_during_component_get_without_registering_component() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });
    let expected_next_entity_id = next_entity_id_after_position_entity();

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.remove::<Velocity>();
        });
    }));

    assert!(result.is_err());
    assert_eq!(world.entity().id(), expected_next_entity_id);
}

// Covers has rejection before lazily registering an unregistered tag.
#[test]
fn entity_rejects_has_during_component_get_without_registering_tag() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });
    let expected_next_entity_id = next_entity_id_after_position_entity();

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.has::<UnregisteredTag>();
        });
    }));

    assert!(result.is_err());
    assert_eq!(world.entity().id(), expected_next_entity_id);
}

// Covers get rejection before lazily registering an unregistered component.
#[test]
fn entity_rejects_get_during_component_get_without_registering_component() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });
    let expected_next_entity_id = next_entity_id_after_position_entity();

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.get::<UnregisteredComponent>(|_| {});
        });
    }));

    assert!(result.is_err());
    assert_eq!(world.entity().id(), expected_next_entity_id);
}

// Covers get_mut rejection before lazily registering an unregistered component.
#[test]
fn entity_rejects_get_mut_during_component_get_without_registering_component() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });
    let expected_next_entity_id = next_entity_id_after_position_entity();

    let result = catch_unwind(AssertUnwindSafe(|| {
        entity.get::<Position>(|_| {
            entity.get_mut::<UnregisteredComponent>(|_| {});
        });
    }));

    assert!(result.is_err());
    assert_eq!(world.entity().id(), expected_next_entity_id);
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

// Covers entity destruction removing the entity from typed queries.
#[test]
fn entity_destruct_removes_entity_from_queries() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 1.0, y: 2.0 })
        .set(Velocity { x: 3.0, y: 4.0 });

    entity.destruct();

    let query = world.query::<(&Position, &Velocity)>().build();
    let mut count = 0;
    query.each(|_| {
        count += 1;
    });

    assert_eq!(count, 0);
}

// Covers typed observer callbacks for immediate custom event emission.
#[test]
fn entity_emit_invokes_typed_observer() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 1.0, y: 2.0 })
        .set(Velocity { x: 3.0, y: 4.0 });
    let hits = std::rc::Rc::new(std::cell::RefCell::new(0));
    let observer_hits = std::rc::Rc::clone(&hits);

    let _observer = world
        .observer::<(&Position, &Velocity)>()
        .event::<EmptyEvent>()
        .each(move |(position, velocity)| {
            assert_eq!(*position, Position { x: 1.0, y: 2.0 });
            assert_eq!(*velocity, Velocity { x: 3.0, y: 4.0 });
            *observer_hits.borrow_mut() += 1;
        });

    entity.emit2::<EmptyEvent, Position, Velocity>();

    assert_eq!(*hits.borrow(), 1);
}

// Covers entity-scoped observers for empty event emission.
#[test]
fn entity_observe_invokes_entity_scoped_observer() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 1.0, y: 2.0 })
        .set(Velocity { x: 3.0, y: 4.0 });
    let other = world
        .entity()
        .set(Position { x: 5.0, y: 6.0 })
        .set(Velocity { x: 7.0, y: 8.0 });
    let hits = std::rc::Rc::new(std::cell::RefCell::new(0));
    let observer_hits = std::rc::Rc::clone(&hits);

    let _observer = entity.observe::<EmptyEvent>(move |source| {
        source.get::<Position>(|position| {
            assert_eq!(*position, Position { x: 1.0, y: 2.0 });
        });
        *observer_hits.borrow_mut() += 1;
    });

    other.emit::<EmptyEvent>();
    entity.emit::<EmptyEvent>();

    assert_eq!(*hits.borrow(), 1);
}

// Covers deferred custom event enqueue flushed by defer_end.
#[test]
fn entity_enqueue_invokes_typed_observer_on_defer_end() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 1.0, y: 2.0 })
        .set(Velocity { x: 3.0, y: 4.0 });
    let hits = std::rc::Rc::new(std::cell::RefCell::new(0));
    let observer_hits = std::rc::Rc::clone(&hits);

    let _observer = world
        .observer::<(&Position, &Velocity)>()
        .event::<EmptyEvent>()
        .each(move |(position, velocity)| {
            assert_eq!(*position, Position { x: 1.0, y: 2.0 });
            assert_eq!(*velocity, Velocity { x: 3.0, y: 4.0 });
            *observer_hits.borrow_mut() += 1;
        });

    assert!(world.defer_begin());
    entity.enqueue2::<EmptyEvent, Position, Velocity>();
    assert_eq!(*hits.borrow(), 0);
    assert!(world.defer_end());

    assert_eq!(*hits.borrow(), 1);
}
