use soul_ecs::World;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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

struct DropCounter {
    drops: Arc<AtomicUsize>,
}

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

// Covers C++-style typed system registration and execution through progress.
#[test]
fn system_each_runs_during_world_progress() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    world
        .system::<(&mut Position, &Velocity)>()
        .each(|(position, velocity)| {
            position.x += velocity.x;
            position.y += velocity.y;
        });

    world.progress();

    entity.get::<Position>(|position| {
        assert_eq!(*position, Position { x: 11.0, y: 22.0 });
    });
}

// Covers rejecting duplicate component fields before system registration succeeds.
#[test]
fn system_each_rejects_duplicate_components() {
    let world = World::new();

    let result = catch_unwind(AssertUnwindSafe(|| {
        world.system::<(&mut Position, &Position)>().each(|_| {});
    }));

    assert!(result.is_err());
}

// Covers keeping Rust panics inside the system callback from unwinding through C.
#[test]
fn system_each_panic_resumes_after_world_progress() {
    let world = World::new();
    world.entity().set(Position { x: 1.0, y: 2.0 });

    world.system::<(&mut Position,)>().each(|_| {
        panic!("system callback panic");
    });

    let result = catch_unwind(AssertUnwindSafe(|| {
        world.progress();
    }));

    assert!(result.is_err());
}

// Covers releasing the boxed callback context when the owning world is finalized.
#[test]
fn system_each_drops_callback_context_with_world() {
    let drops = Arc::new(AtomicUsize::new(0));

    {
        let world = World::new();
        let counter = DropCounter {
            drops: Arc::clone(&drops),
        };
        world.system::<(&Position,)>().each(move |_| {
            let _counter = &counter;
        });

        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
