use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use soul_ecs::World;

const ENTITY_COUNTS: [usize; 3] = [1_000, 10_000, 100_000];

#[derive(Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct Tag;

fn populate_position_velocity(count: usize) -> World {
    let world = World::new();
    for index in 0..count {
        let value = index as f32;
        world
            .entity()
            .set(Position { x: value, y: value })
            .set(Velocity { x: 1.0, y: 2.0 });
    }
    world
}

fn bench_entity_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_creation");

    for count in ENTITY_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let world = World::new();
                for _ in 0..count {
                    black_box(world.entity());
                }
                black_box(world);
            });
        });
    }

    group.finish();
}

fn bench_component_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_set");

    for count in ENTITY_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let world = World::new();
                for index in 0..count {
                    let value = index as f32;
                    black_box(world.entity().set(Position { x: value, y: value }));
                }
                black_box(world);
            });
        });
    }

    group.finish();
}

fn bench_component_add_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_add_remove");

    for count in ENTITY_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let world = World::new();
                for _ in 0..count {
                    black_box(world.entity().add::<Tag>().remove::<Tag>());
                }
                black_box(world);
            });
        });
    }

    group.finish();
}

fn bench_component_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_get");

    for count in ENTITY_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let world = World::new();
                let mut sum = 0.0;
                for index in 0..count {
                    let value = index as f32;
                    let entity = world.entity().set(Position { x: value, y: value });
                    entity.get::<Position>(|position| {
                        sum += position.x + position.y;
                    });
                }
                black_box(sum);
                black_box(world);
            });
        });
    }

    group.finish();
}

fn bench_component_get_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_get_mut");

    for count in ENTITY_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let world = World::new();
                for index in 0..count {
                    let value = index as f32;
                    let entity = world.entity().set(Position { x: value, y: value });
                    entity.get_mut::<Position>(|position| {
                        position.x += 1.0;
                        position.y += 1.0;
                    });
                }
                black_box(world);
            });
        });
    }

    group.finish();
}

fn bench_query_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_iteration");

    for count in ENTITY_COUNTS {
        group.bench_with_input(
            BenchmarkId::new("read_position_velocity", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || populate_position_velocity(count),
                    |world| {
                        let mut sum = 0.0;
                        {
                            let query = world.query::<(&Position, &Velocity)>().build();
                            query.each(|(position, velocity)| {
                                sum += position.x + position.y + velocity.x + velocity.y;
                            });
                        }
                        black_box(sum);
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("update_position_velocity", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || populate_position_velocity(count),
                    |world| {
                        {
                            let query = world.query::<(&mut Position, &Velocity)>().build();
                            query.each(|(position, velocity)| {
                                position.x += velocity.x;
                                position.y += velocity.y;
                            });
                        }
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_system_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("system_progress");

    for count in ENTITY_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || populate_position_velocity(count),
                |world| {
                    {
                        let system = world.system::<(&mut Position, &Velocity)>().each(
                            |(position, velocity)| {
                                position.x += velocity.x;
                                position.y += velocity.y;
                            },
                        );
                        black_box(&system);
                        black_box(world.progress());
                    }
                    black_box(world);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_entity_creation(c);
    bench_component_set(c);
    bench_component_add_remove(c);
    bench_component_get(c);
    bench_component_get_mut(c);
    bench_query_iteration(c);
    bench_system_progress(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
