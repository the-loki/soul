use std::cell::RefCell;
use std::hint::black_box;
use std::rc::Rc;
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use soul_ecs::World;

const ENTITY_COUNTS: [usize; 23] = [
    0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1_024, 2_048, 4_096, 8_192, 16_384, 32_768, 65_536,
    131_072, 262_144, 524_288, 1_048_576, 2_097_152,
];
const FAKE_TIME_DELTA: f32 = 1.0 / 60.0;

#[derive(Clone, Copy, Default)]
struct PositionComponent {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct VelocityComponent {
    x: f32,
    y: f32,
}

impl Default for VelocityComponent {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}

#[derive(Clone, Copy)]
struct RandomXoshiro128 {
    state: [u32; 4],
}

impl RandomXoshiro128 {
    fn new(seed: u32) -> Self {
        Self {
            state: [seed + 3, seed + 5, seed + 7, seed + 11],
        }
    }

    fn next(&mut self) -> u32 {
        let result = self.state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 9;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(11);
        result
    }

    fn range(&mut self, low: u32, high: u32) -> u32 {
        let range = high - low + 1;
        (self.next() % range) + low
    }
}

impl Default for RandomXoshiro128 {
    fn default() -> Self {
        Self::new(340_383)
    }
}

#[derive(Clone, Copy)]
struct DataComponent {
    thingy: i32,
    dingy: f64,
    mingy: bool,
    rng: RandomXoshiro128,
    numgy: u32,
}

impl Default for DataComponent {
    fn default() -> Self {
        let mut rng = RandomXoshiro128::default();
        let numgy = rng.next();
        Self {
            thingy: 0,
            dingy: 0.0,
            mingy: false,
            rng,
            numgy,
        }
    }
}

#[derive(Clone, Copy)]
enum PlayerType {
    Npc,
    Monster,
    Hero,
}

#[derive(Clone, Copy)]
struct PlayerComponent {
    rng: RandomXoshiro128,
    player_type: PlayerType,
}

impl Default for PlayerComponent {
    fn default() -> Self {
        Self {
            rng: RandomXoshiro128::default(),
            player_type: PlayerType::Npc,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusEffect {
    Spawn,
    Dead,
    Alive,
}

#[derive(Clone, Copy)]
struct HealthComponent {
    hp: i32,
    maxhp: i32,
    status: StatusEffect,
}

impl Default for HealthComponent {
    fn default() -> Self {
        Self {
            hp: 0,
            maxhp: 0,
            status: StatusEffect::Spawn,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct DamageComponent {
    atk: i32,
    def: i32,
}

#[derive(Clone, Copy)]
struct SpriteComponent {
    character: char,
}

impl Default for SpriteComponent {
    fn default() -> Self {
        Self { character: ' ' }
    }
}

#[derive(Clone, Copy)]
struct EmptyEvent;

#[derive(Default)]
struct ComponentsCounter {
    component_one_count: usize,
    component_two_count: usize,
    component_three_count: usize,
    hero_count: usize,
    monster_count: usize,
}

fn black_box_counter(counter: &ComponentsCounter) {
    black_box(counter.component_one_count);
    black_box(counter.component_two_count);
    black_box(counter.component_three_count);
    black_box(counter.hero_count);
    black_box(counter.monster_count);
}

struct FrameBuffer {
    width: usize,
    height: usize,
    buffer: Vec<char>,
}

impl FrameBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![' '; width * height],
        }
    }

    fn draw(&mut self, x: i32, y: i32, c: char) {
        if x >= 0 && y >= 0 {
            let x = x as usize;
            let y = y as usize;
            if x < self.width && y < self.height {
                self.buffer[x + y * self.width] = c;
            }
        }
    }
}

fn default_position() -> PositionComponent {
    PositionComponent::default()
}

fn default_velocity() -> VelocityComponent {
    VelocityComponent::default()
}

fn create_empty(world: &World) -> u64 {
    world.entity().id()
}

fn create_single(world: &World) -> u64 {
    world.entity().set(default_position()).id()
}

fn create_minimal(world: &World) -> u64 {
    world
        .entity()
        .set(default_position())
        .set(default_velocity())
        .id()
}

fn create_full(world: &World) -> u64 {
    world
        .entity()
        .set(default_position())
        .set(default_velocity())
        .set(DataComponent::default())
        .id()
}

fn create_entities_with_single_component(
    world: &World,
    count: usize,
) -> (Vec<u64>, ComponentsCounter) {
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(create_single(world));
    }
    (
        ids,
        ComponentsCounter {
            component_one_count: count,
            ..ComponentsCounter::default()
        },
    )
}

fn create_entities_with_minimal_components(
    world: &World,
    count: usize,
) -> (Vec<u64>, ComponentsCounter) {
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(create_minimal(world));
    }
    (
        ids,
        ComponentsCounter {
            component_one_count: count,
            component_two_count: count,
            ..ComponentsCounter::default()
        },
    )
}

fn create_entities_with_half_components(
    world: &World,
    count: usize,
) -> (Vec<u64>, ComponentsCounter) {
    let mut ids = Vec::with_capacity(count);
    let mut counter = ComponentsCounter::default();
    for index in 0..count {
        if index % 2 == 0 {
            ids.push(create_full(world));
            counter.component_one_count += 1;
            counter.component_two_count += 1;
            counter.component_three_count += 1;
        } else {
            let _ = create_minimal(world);
            counter.component_one_count += 1;
            counter.component_two_count += 1;
        }
    }
    (ids, counter)
}

fn create_entities_with_mixed_components(
    world: &World,
    count: usize,
) -> (Vec<u64>, ComponentsCounter) {
    let mut ids = Vec::with_capacity(count);
    let mut counter = ComponentsCounter::default();
    let mut mixed_index = 0;
    for index in 0..count {
        let id = create_full(world);
        ids.push(id);
        counter.component_one_count += 1;
        counter.component_two_count += 1;
        counter.component_three_count += 1;

        if count < 100 || (index >= 2 * count / 4 && index <= 3 * count / 4) {
            if count < 100 || mixed_index % 10 == 0 {
                let entity = world.entity_from_id(id);
                if index % 7 == 0 {
                    let entity = entity.remove::<PositionComponent>();
                    counter.component_one_count -= 1;
                    if index % 11 == 0 {
                        let entity = entity.remove::<VelocityComponent>();
                        counter.component_two_count -= 1;
                        if index % 13 == 0 {
                            entity.remove::<DataComponent>();
                            counter.component_three_count -= 1;
                        }
                    } else if index % 13 == 0 {
                        entity.remove::<DataComponent>();
                        counter.component_three_count -= 1;
                    }
                } else if index % 11 == 0 {
                    let entity = entity.remove::<VelocityComponent>();
                    counter.component_two_count -= 1;
                    if index % 13 == 0 {
                        entity.remove::<DataComponent>();
                        counter.component_three_count -= 1;
                    }
                } else if index % 13 == 0 {
                    entity.remove::<DataComponent>();
                    counter.component_three_count -= 1;
                }
            }
            mixed_index += 1;
        }
    }
    (ids, counter)
}

fn add_hero_monster_components(world: &World, id: u64) {
    world
        .entity_from_id(id)
        .set(PlayerComponent::default())
        .set(HealthComponent::default())
        .set(DamageComponent::default())
        .set(SpriteComponent::default());
    if !world.entity_from_id(id).has::<PositionComponent>() {
        world.entity_from_id(id).set(default_position());
    }
}

fn init_hero_monster_components(
    world: &World,
    id: u64,
    forced_type: Option<PlayerType>,
) -> PlayerType {
    let entity = world.entity_from_id(id);
    let mut player = PlayerComponent::default();
    entity.get::<PlayerComponent>(|component| {
        player = *component;
    });
    let player_type = forced_type.unwrap_or_else(|| {
        let rate = player.rng.range(1, 100);
        if rate <= 3 {
            PlayerType::Npc
        } else if rate <= 30 {
            PlayerType::Hero
        } else {
            PlayerType::Monster
        }
    });
    player.player_type = player_type;

    let mut health = HealthComponent::default();
    let mut damage = DamageComponent::default();
    match player_type {
        PlayerType::Hero => {
            health.maxhp = player.rng.range(5, 15) as i32;
            damage.def = player.rng.range(2, 6) as i32;
            damage.atk = player.rng.range(4, 10) as i32;
        }
        PlayerType::Monster => {
            health.maxhp = player.rng.range(4, 12) as i32;
            damage.def = player.rng.range(2, 8) as i32;
            damage.atk = player.rng.range(3, 9) as i32;
        }
        PlayerType::Npc => {
            health.maxhp = player.rng.range(6, 12) as i32;
            damage.def = player.rng.range(3, 8) as i32;
            damage.atk = 0;
        }
    }

    let position = PositionComponent {
        x: player.rng.range(0, 420) as f32 - 100.0,
        y: player.rng.range(0, 340) as f32 - 100.0,
    };

    world
        .entity_from_id(id)
        .set(position)
        .set(player)
        .set(health)
        .set(damage)
        .set(SpriteComponent { character: '_' });

    player_type
}

fn setup_systems_update(
    count: usize,
    complex: bool,
    mixed: bool,
) -> (World, Vec<u64>, ComponentsCounter) {
    let world = World::new();
    let (ids, mut counter) = if mixed {
        create_entities_with_mixed_components(&world, count)
    } else {
        let mut ids = Vec::with_capacity(count);
        for _ in 0..count {
            ids.push(create_full(&world));
        }
        (
            ids,
            ComponentsCounter {
                component_one_count: count,
                component_two_count: count,
                component_three_count: count,
                ..ComponentsCounter::default()
            },
        )
    };

    if complex {
        let mut mixed_index = 0;
        for (index, id) in ids.iter().copied().enumerate() {
            if !mixed || count >= 100 || index == 0 || index >= count / 8 {
                let should_add = !mixed || count >= 100 || index == 0 || mixed_index % 2 == 0;
                if should_add {
                    add_hero_monster_components(&world, id);
                    let player_type = if mixed && index == 0 {
                        init_hero_monster_components(&world, id, Some(PlayerType::Hero))
                    } else if mixed && index % 6 == 0 {
                        init_hero_monster_components(&world, id, None)
                    } else if mixed && index % 4 == 0 {
                        init_hero_monster_components(&world, id, Some(PlayerType::Hero))
                    } else if mixed && index % 2 == 0 {
                        init_hero_monster_components(&world, id, Some(PlayerType::Monster))
                    } else if !mixed {
                        init_hero_monster_components(&world, id, None)
                    } else {
                        continue;
                    };
                    match player_type {
                        PlayerType::Hero => counter.hero_count += 1,
                        PlayerType::Monster => counter.monster_count += 1,
                        PlayerType::Npc => {}
                    }
                }
                mixed_index += 1;
            }
        }
    }

    (world, ids, counter)
}

fn update_data(data: &mut DataComponent, dt: f32) {
    data.thingy = (data.thingy + 1) % 1_000_000;
    data.dingy += 0.0001 * f64::from(dt);
    data.mingy = !data.mingy;
    data.numgy = data.rng.next();
}

fn update_more_complex(
    position: &PositionComponent,
    velocity: &mut VelocityComponent,
    data: &mut DataComponent,
) {
    if data.thingy % 10 == 0 {
        if position.x > position.y {
            velocity.x = data.rng.range(3, 19) as f32 - 10.0;
            velocity.y = data.rng.range(0, 5) as f32;
        } else {
            velocity.x = data.rng.range(0, 5) as f32;
            velocity.y = data.rng.range(3, 19) as f32 - 10.0;
        }
    }
}

fn update_health(health: &mut HealthComponent) {
    if health.hp <= 0 && health.status != StatusEffect::Dead {
        health.hp = 0;
        health.status = StatusEffect::Dead;
    } else if health.status == StatusEffect::Dead && health.hp == 0 {
        health.hp = health.maxhp;
        health.status = StatusEffect::Spawn;
    } else if health.hp >= health.maxhp && health.status != StatusEffect::Alive {
        health.hp = health.maxhp;
        health.status = StatusEffect::Alive;
    } else {
        health.status = StatusEffect::Alive;
    }
}

fn update_damage(health: &mut HealthComponent, damage: &DamageComponent) {
    let total_damage = damage.atk - damage.def;
    if health.hp > 0 && total_damage > 0 {
        health.hp = (health.hp - total_damage).max(0);
    }
}

fn update_sprite(sprite: &mut SpriteComponent, player: &PlayerComponent, health: &HealthComponent) {
    sprite.character = match health.status {
        StatusEffect::Alive => match player.player_type {
            PlayerType::Hero => '@',
            PlayerType::Monster => 'k',
            PlayerType::Npc => 'h',
        },
        StatusEffect::Dead => '|',
        StatusEffect::Spawn => '_',
    };
}

fn install_update_systems(world: &World, complex: bool) {
    let movement = world
        .system::<(&mut PositionComponent, &VelocityComponent)>()
        .each(|(position, velocity)| {
            position.x += velocity.x * FAKE_TIME_DELTA;
            position.y += velocity.y * FAKE_TIME_DELTA;
        });
    let data = world.system::<(&mut DataComponent,)>().each(|(data,)| {
        update_data(data, FAKE_TIME_DELTA);
    });
    black_box((&movement, &data));

    if complex {
        let more = world
            .system::<(
                &PositionComponent,
                &mut VelocityComponent,
                &mut DataComponent,
            )>()
            .each(|(position, velocity, data)| {
                update_more_complex(position, velocity, data);
            });
        let health = world.system::<(&mut HealthComponent,)>().each(|(health,)| {
            update_health(health);
        });
        let damage = world
            .system::<(&mut HealthComponent, &DamageComponent)>()
            .each(|(health, damage)| {
                update_damage(health, damage);
            });
        let sprite = world
            .system::<(&mut SpriteComponent, &PlayerComponent, &HealthComponent)>()
            .each(|(sprite, player, health)| {
                update_sprite(sprite, player, health);
            });
        let frame_buffer = Rc::new(RefCell::new(FrameBuffer::new(320, 240)));
        let render_buffer = Rc::clone(&frame_buffer);
        let render = world
            .system::<(&PositionComponent, &SpriteComponent)>()
            .each(move |(position, sprite)| {
                render_buffer.borrow_mut().draw(
                    position.x as i32,
                    position.y as i32,
                    sprite.character,
                );
            });
        black_box((&more, &health, &damage, &sprite, &render, frame_buffer));
    }
}

fn bench_with_default_inputs(
    c: &mut Criterion,
    group_name: &str,
    mut bench: impl FnMut(&mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>, usize),
) {
    let mut group = c.benchmark_group(group_name);
    for count in ENTITY_COUNTS {
        bench(&mut group, count);
    }
    group.finish();
}

fn bench_entity_suite(c: &mut Criterion) {
    c.bench_function("BM_CreateNoEntities", |b| {
        b.iter_batched(
            World::new,
            |world| {
                black_box(world);
            },
            BatchSize::SmallInput,
        );
    });

    bench_with_default_inputs(c, "BM_CreateEmptyEntities", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                World::new,
                |world| {
                    for _ in 0..count {
                        black_box(create_empty(&world));
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });

    bench_with_default_inputs(c, "BM_UnpackNoComponent", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let world = World::new();
            let (ids, _) = create_entities_with_minimal_components(&world, count);
            b.iter(|| {
                for id in &ids {
                    black_box(id);
                }
            });
            black_box(world);
        });
    });

    bench_with_default_inputs(c, "BM_CreateEntities", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                World::new,
                |world| {
                    for _ in 0..count {
                        black_box(create_minimal(&world));
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });

    bench_with_default_inputs(c, "BM_UnpackOneComponent", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || {
                    let world = World::new();
                    let (ids, _) = create_entities_with_minimal_components(&world, count);
                    (world, ids)
                },
                |(world, ids)| {
                    for id in ids {
                        world
                            .entity_from_id(id)
                            .get_mut::<PositionComponent>(|component| {
                                black_box(component);
                            });
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });

    bench_with_default_inputs(c, "BM_UnpackTwoComponents", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || {
                    let world = World::new();
                    let (ids, _) = create_entities_with_minimal_components(&world, count);
                    (world, ids)
                },
                |(world, ids)| {
                    for id in ids {
                        let entity = world.entity_from_id(id);
                        entity.get_mut::<PositionComponent>(|component| {
                            black_box(component);
                        });
                        entity.get_mut::<VelocityComponent>(|component| {
                            black_box(component);
                        });
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });

    bench_with_default_inputs(c, "BM_UnpackThreeComponents", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || {
                    let world = World::new();
                    let (ids, _) = create_entities_with_half_components(&world, count);
                    (world, ids)
                },
                |(world, ids)| {
                    for id in ids {
                        let entity = world.entity_from_id(id);
                        entity.get_mut::<PositionComponent>(|component| {
                            black_box(component);
                        });
                        entity.get_mut::<VelocityComponent>(|component| {
                            black_box(component);
                        });
                        entity.get_mut::<DataComponent>(|component| {
                            black_box(component);
                        });
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });

    bench_with_default_inputs(c, "BM_AddComponent", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_custom(|iterations| {
                let mut total = Duration::ZERO;
                for _ in 0..iterations {
                    let world = World::new();
                    let (ids, _) = create_entities_with_minimal_components(&world, count);
                    for id in &ids {
                        world.entity_from_id(*id).remove::<PositionComponent>();
                    }
                    let start = Instant::now();
                    for id in &ids {
                        world.entity_from_id(*id).set(default_position());
                    }
                    total += start.elapsed();
                    black_box(world);
                }
                total
            });
        });
    });

    bench_with_default_inputs(c, "BM_RemoveAddComponent", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || {
                    let world = World::new();
                    let (ids, _) = create_entities_with_minimal_components(&world, count);
                    (world, ids)
                },
                |(world, ids)| {
                    for id in ids {
                        world
                            .entity_from_id(id)
                            .remove::<PositionComponent>()
                            .set(default_position());
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });

    bench_with_default_inputs(c, "BM_DestroyEntities", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                || {
                    let world = World::new();
                    let (ids, _) = create_entities_with_minimal_components(&world, count);
                    (world, ids)
                },
                |(world, ids)| {
                    for id in ids {
                        world.entity_from_id(id).destruct();
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });

    bench_with_default_inputs(c, "BM_CreateEntitiesInBulk", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                World::new,
                |world| {
                    let entities = world.bulk_with2::<PositionComponent, VelocityComponent>(count);
                    black_box(entities);
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });

    bench_with_default_inputs(c, "BM_CreateEmptyEntitiesInBulk", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_batched(
                World::new,
                |world| {
                    let entities = world.bulk_empty(count);
                    black_box(entities);
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    });
}

fn bench_update_suite(c: &mut Criterion) {
    bench_with_default_inputs(c, "BM_SystemsUpdate", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let (world, ids, counter) = setup_systems_update(count, false, false);
            install_update_systems(&world, false);
            b.iter(|| black_box(world.progress()));
            black_box_counter(&counter);
            black_box((world, ids, counter));
        });
    });

    bench_with_default_inputs(c, "BM_SystemsUpdateMixedEntities", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let (world, ids, counter) = setup_systems_update(count, false, true);
            install_update_systems(&world, false);
            b.iter(|| black_box(world.progress()));
            black_box_counter(&counter);
            black_box((world, ids, counter));
        });
    });
}

fn bench_extended_suite(c: &mut Criterion) {
    bench_with_default_inputs(c, "BM_ComplexSystemsUpdate", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let (world, ids, counter) = setup_systems_update(count, true, false);
            install_update_systems(&world, true);
            b.iter(|| black_box(world.progress()));
            black_box_counter(&counter);
            black_box((world, ids, counter));
        });
    });

    bench_with_default_inputs(c, "BM_ComplexSystemsUpdateMixedEntities", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let (world, ids, counter) = setup_systems_update(count, true, true);
            install_update_systems(&world, true);
            b.iter(|| black_box(world.progress()));
            black_box_counter(&counter);
            black_box((world, ids, counter));
        });
    });

    bench_with_default_inputs(c, "BM_IterateSingleComponent", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let world = World::new();
            let (ids, counter) = create_entities_with_single_component(&world, count);
            let query = world.query::<(&PositionComponent,)>().build();
            b.iter(|| {
                query.each(|(component,)| {
                    black_box(component);
                });
            });
            black_box_counter(&counter);
            drop(query);
            black_box((world, ids, counter));
        });
    });

    bench_with_default_inputs(c, "BM_IterateTwoComponents", |group, count| {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let world = World::new();
            let (ids, counter) = create_entities_with_minimal_components(&world, count);
            let query = world
                .query::<(&PositionComponent, &VelocityComponent)>()
                .build();
            b.iter(|| {
                query.each(|(component_one, component_two)| {
                    black_box((component_one, component_two));
                });
            });
            black_box_counter(&counter);
            drop(query);
            black_box((world, ids, counter));
        });
    });

    bench_with_default_inputs(
        c,
        "BM_IterateThreeComponentsWithMixedEntities",
        |group, count| {
            group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
                let world = World::new();
                let (ids, counter) = create_entities_with_mixed_components(&world, count);
                let query = world
                    .query::<(&PositionComponent, &VelocityComponent, &DataComponent)>()
                    .build();
                b.iter(|| {
                    query.each(|(component_one, component_two, component_three)| {
                        black_box((component_one, component_two, component_three));
                    });
                });
                black_box_counter(&counter);
                drop(query);
                black_box((world, ids, counter));
            });
        },
    );
}

fn bench_event_suite(c: &mut Criterion) {
    bench_with_default_inputs(
        c,
        "BM_EnqueueAndUpdateEventsViaObserverWithMixedEntities",
        |group, count| {
            group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
                let world = World::new();
                let (ids, counter) = create_entities_with_mixed_components(&world, count);
                let observer = world
                    .observer::<(&PositionComponent, &VelocityComponent)>()
                    .event::<EmptyEvent>()
                    .each(|(position, velocity)| {
                        black_box((position, velocity));
                    });
                let query = world
                    .query::<(&PositionComponent, &VelocityComponent)>()
                    .build();

                b.iter(|| {
                    black_box(world.defer_begin());
                    query.each_entity(|entity, (position, velocity)| {
                        black_box((position, velocity));
                        entity.enqueue2::<EmptyEvent, PositionComponent, VelocityComponent>();
                    });
                    black_box(world.defer_end());
                });

                black_box_counter(&counter);
                drop(query);
                black_box((&world, &ids, &observer));
                black_box(counter);
            });
        },
    );

    let mut group = c.benchmark_group("BM_EmitAndUpdateEventsViaObserverWithMixedEntities");
    for count in [
        0usize, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1_024, 2_048, 4_096, 8_192, 16_384, 32_768,
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let world = World::new();
            let (ids, counter) = create_entities_with_mixed_components(&world, count);
            let query = world
                .query::<(&PositionComponent, &VelocityComponent)>()
                .build();
            for id in &ids {
                let entity = world.entity_from_id(*id);
                if entity.has::<PositionComponent>() && entity.has::<VelocityComponent>() {
                    entity.observe::<EmptyEvent>(|source| {
                        source.get::<PositionComponent>(|position| {
                            source.get::<VelocityComponent>(|velocity| {
                                black_box((position, velocity));
                            });
                        });
                    });
                }
            }

            b.iter(|| {
                query.each_entity(|entity, _| {
                    entity.emit::<EmptyEvent>();
                });
            });

            black_box_counter(&counter);
            drop(query);
            black_box((&world, &ids));
            black_box(counter);
        });
    }
    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_entity_suite(c);
    bench_update_suite(c);
    bench_extended_suite(c);
    bench_event_suite(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
