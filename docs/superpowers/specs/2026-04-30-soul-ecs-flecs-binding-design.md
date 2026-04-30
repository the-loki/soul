# soul ECS flecs 绑定设计

## 背景

当前仓库是一个空的 Rust workspace，只有根 `Cargo.toml`、`README.md` 和协作规则。目标是在当前目录中创建一个 ECS 框架，其本质是 flecs 的绑定。

已通过 Context7 查询 flecs 官方资料，确认 flecs 提供 C API，并且 C++ 接口以 `world`、`entity`、`system<T...>()`、`query_builder<T...>()`、`each`、`run` 等链式和强类型 API 为核心。`soul-ecs` 的公开接口应参考 flecs C++ 设计，而不是暴露接近 C API 的函数集合。

## 目标

第一版交付两个 Rust crate：

- `soul-ecs-sys`：低层 FFI crate，负责编译或链接 flecs C 库，并暴露项目需要的最小 C API 绑定。
- `soul-ecs`：安全 Rust 门面，隐藏 flecs 原始指针和 `unsafe`，提供接近 flecs C++ 风格的 typed builder API。

第一版必须能完成以下基础流程：

```rust
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

let world = World::new();

let entity = world
    .entity()
    .set(Position { x: 10.0, y: 20.0 })
    .set(Velocity { x: 1.0, y: 2.0 });

assert!(entity.has::<Position>());

entity.get::<Position>(|position| {
    assert_eq!(position.x, 10.0);
});

let query = world.query::<(&mut Position, &Velocity)>().build();

query.each(|(position, velocity)| {
    position.x += velocity.x;
    position.y += velocity.y;
});
```

## 非目标

第一版不实现以下能力：

- prefab、relationship、pair、module、scope 等高级 flecs 建模功能。
- observer、trigger、event hook。
- pipeline phase、自定义 scheduler 和完整应用循环。
- 多线程 staging、defer 和 worker 调度。
- flecs meta、reflection、script、REST、monitor 等 addon。
- 完整覆盖 flecs C++ API。

这些能力应在基础绑定稳定后，按独立设计逐步增加。

## crate 边界

### `soul-ecs-sys`

`soul-ecs-sys` 只负责 FFI，不提供安全抽象。

职责：

- 通过 `build.rs` 构建 vendored flecs C 源码，或在后续 feature 中支持系统库链接。
- 暴露 `ecs_world_t`、`ecs_entity_t`、`ecs_id_t`、`ecs_iter_t` 等基础类型。
- 暴露第一版需要的 flecs C 函数，例如 world 生命周期、entity 创建、component 注册、set/get/has/remove、query/system 执行相关函数。
- 保持绑定层尽量薄，避免在 sys crate 中承载 Rust 语义。

安全边界：

- `soul-ecs-sys` 中的所有 FFI 调用默认视为 unsafe。
- C 类型布局、函数签名和生命周期不变量必须以英文注释说明。

### `soul-ecs`

`soul-ecs` 是用户主要依赖的 crate。

职责：

- 暴露 safe Rust API。
- 以 `World` 作为 flecs world 的拥有者，drop 时释放底层 world。
- 以 `Entity` 表示 world 内的 entity 句柄。
- 以 trait 和内部注册表管理 Rust component 类型到 flecs component id 的映射。
- 提供 `QueryBuilder`、`Query`、`SystemBuilder`、`System` 等接近 flecs C++ 的 typed builder API。
- 把所有 `unsafe` 限制在内部模块中，并通过测试覆盖关键行为。

## 公开 API 设计

### World

`World` 负责 world 生命周期和入口 builder。

```rust
let world = World::new();
let entity = world.entity();
let named = world.entity_named("Player");

let query = world.query::<(&mut Position, &Velocity)>().build();

world
    .system::<(&mut Position, &Velocity)>()
    .each(|(position, velocity)| {
        position.x += velocity.x;
        position.y += velocity.y;
    });

world.progress();
```

第一版 `World::new()` 可直接 panic 于 flecs 初始化失败；如果 flecs C API 给出明确失败状态，后续再补 `World::try_new()` 和错误类型。`World::progress()` 第一版可返回 `bool`，对应 flecs progress 是否继续运行。

### Entity

`Entity` 是轻量句柄，内部持有 world 引用和 flecs entity id，并实现 `Copy` 与 `Clone`。

```rust
let entity = world
    .entity()
    .set(Position { x: 1.0, y: 2.0 })
    .set(Velocity { x: 3.0, y: 4.0 });

entity.add::<Tag>();
entity.remove::<Velocity>();

assert!(entity.has::<Position>());

entity.get::<Position>(|position| {
    assert_eq!(position.x, 1.0);
});

entity.get_mut::<Position>(|position| {
    position.x += 1.0;
});
```

设计取舍：

- `get` 和 `get_mut` 使用闭包访问，避免把底层 flecs 组件指针泄漏到闭包外。
- `set` 消费并返回 `Self`，支持链式调用。
- `add` 和 `remove` 与 `set` 一样消费并返回 `Self`，依靠 `Entity` 的轻量可复制语义支持链式和重复使用。

### Component

Rust component 以普通 Rust 类型表示。第一版通过 flecs 的按字节 set/get 路径存储组件，因此要求 component 类型满足：

```rust
T: Copy + 'static
```

这个限制避免把带析构逻辑的 Rust 类型交给 flecs C 存储后产生未定义行为。后续支持非 `Copy` component 时，必须先设计并实现 component lifecycle hooks。

注册方式采用按需注册：

- 第一次 `set::<T>`、`add::<T>`、`has::<T>`、`query::<...>` 或 `system::<...>` 时注册 component。
- component 名称默认使用 `std::any::type_name::<T>()`。
- 同一 `World` 内同一 Rust 类型只注册一次。

后续可增加显式注册 API：

```rust
world.component::<Position>().named("Position");
```

第一版可以先不提供显式命名 builder，避免扩大范围。

### Query

Query API 参考 flecs C++ `world.query<T...>()` 和 `query_builder<T...>()`。

```rust
let query = world.query::<(&mut Position, &Velocity)>().build();

query.each(|(position, velocity)| {
    position.x += velocity.x;
    position.y += velocity.y;
});
```

字段类型约定：

- `&T` 表示只读组件。
- `&mut T` 表示可写组件。
- tuple 表示多个字段。

第一版只支持 `(&T,)`、`(&mut T,)`、`(&T, &U)`、`(&mut T, &U)` 这类小规模 tuple。后续可用宏扩展更多 arity。

Builder 第一版可保留最小形态：

```rust
world.query::<(&mut Position, &Velocity)>().build();
```

后续再扩展：

```rust
world
    .query::<(&mut Position, &Velocity)>()
    .with::<Tag>()
    .without::<Disabled>()
    .build();
```

### System

System API 参考 flecs C++ `world.system<T...>().each(...)`。

```rust
world
    .system::<(&mut Position, &Velocity)>()
    .each(|(position, velocity)| {
        position.x += velocity.x;
        position.y += velocity.y;
    });
```

第一版 system 目标是验证：

- 可以用 typed component 列表创建 system。
- system callback 能在 `world.progress()` 时被 flecs 调用。
- callback 内可以安全访问当前迭代的组件字段。

第一版不实现 phase builder。后续可增加：

```rust
world
    .system::<(&mut Position, &Velocity)>()
    .kind::<OnUpdate>()
    .each(|(position, velocity)| {
        position.x += velocity.x;
        position.y += velocity.y;
    });
```

## 内部设计

### 类型注册表

`World` 内部维护 component registry：

- key：`TypeId`
- value：flecs component id、layout 信息、名称

注册过程必须确保：

- 同一 Rust 类型在同一 world 内只注册一次。
- flecs component size 和 alignment 与 Rust 类型一致。
- component id 只在对应 world 中使用，不跨 world 复用。

### 字段访问

`soul-ecs` 内部定义 trait，把 Rust tuple 类型映射到 flecs query/system 字段：

```rust
trait QueryParam {
    fn register(world: &World);
    unsafe fn fetch(iter: *mut ecs_iter_t, row: i32) -> Self::Item<'_>;
}
```

该 trait 是内部实现边界，公开 API 不暴露它。实现可以拆分为多个内部 trait，但必须满足以下职责：

- 根据 tuple 中的 `&T` / `&mut T` 注册并构造 flecs term。
- 在 iteration 时把 flecs 字段指针转成 Rust 引用。
- 约束 mutable access，避免同一 component 在同一 tuple 中产生多个 `&mut`。

### 闭包存储

System callback 需要被 C API 回调调用。内部应：

- 把 Rust closure 装箱并存入稳定地址。
- 在 flecs system context 中保存指针。
- C trampoline 中取回 closure 并调用。
- 在 system 销毁或 world drop 时释放 closure。

这部分是第一版最重要的 unsafe 边界，必须有英文 safety 注释和测试。

## 错误处理

第一版公开 API 以简单、可测试为主：

- `World::new()` 初始化失败时 panic。
- component 注册、query build、system build 若底层 flecs 返回无效 id，则 panic 并给出英文错误字符串。
- 未来再引入 `Result` 风格 API，例如 `try_entity_named`、`try_query`、`try_system`。

源码中的错误字符串必须使用英文。

## 构建策略

第一版采用 vendored flecs：

- 在仓库中放置或生成 flecs C 源码副本。
- `soul-ecs-sys/build.rs` 使用 `cc` crate 编译 flecs。
- 根 workspace 包含 `crates/soul-ecs-sys` 和 `crates/soul-ecs`。

后续可增加 feature：

- `vendored`：默认启用，编译随仓库提供的 flecs 源码。
- `system`：通过 `pkg-config` 或系统链接器寻找已安装 flecs。

第一版只需要实现默认 `vendored`。

## 测试策略

采用测试先行。

第一批测试：

- `World::new()` 可以创建并 drop。
- entity 可以 set、has、get、remove 单个 component。
- entity 可以 set 多个 component 并链式返回。
- query 可以遍历 `(&mut Position, &Velocity)` 并修改 Position。
- system 可以在 `world.progress()` 后执行一次并修改 Position。

测试代码和测试名必须使用英文。新增测试函数前需要英文注释说明覆盖的行为或回归点。

## 文档策略

项目文档使用中文，放在 `docs/` 下。

第一版实现完成后应补充：

- `docs/ecs/overview.md`：说明架构、crate 边界和基础用法。
- `docs/ecs/safety.md`：说明 `unsafe` 边界、component 生命周期约束和 system callback 安全前提。

## 验收标准

完成第一版后应满足：

- `cargo test --workspace` 通过。
- `cargo doc --workspace --no-deps` 通过。
- 公开 API 能运行 entity/component/query/system 的基础示例。
- `unsafe` 不出现在公开 API 使用侧。
- `soul-ecs` 的 API 风格能明显对应 flecs C++ 的 typed builder 设计。
