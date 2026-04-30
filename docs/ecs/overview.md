# soul ECS 概览

`soul-ecs` 是基于 flecs C API 的 Rust ECS 门面。公开 API 参考 flecs C++ 的 typed builder 风格，底层由 `soul-ecs-sys` 编译 vendored flecs v4.1.5。

## crate 边界

- `soul-ecs-sys`：低层 FFI 和 C shim，不提供安全抽象。
- `soul-ecs`：safe Rust API，隐藏 flecs 指针和 `unsafe`。

## component 限制

第一版支持 `Copy + 'static` 数据 component。数据 component 应通过 `set` 初始化和更新；`Entity::add<T>()` 只用于零大小 tag component。

组件读取和写入会经过运行时借用守卫。同一实体上的同一 component 可以被重复共享读取，但共享读取和可变访问、多个可变访问会被拒绝。

## 基础用法

```rust
use soul_ecs::World;

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
    assert_eq!(position.x, 11.0);
});
```
