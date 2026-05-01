# soul ECS 基准测试

`soul-ecs` 的 benchmark 对照 `abeimler/ecs_benchmark` 中 flecs 相关 benchmark 的命名、输入规模、实体构造逻辑和事件触发结构。当前 runner 使用 Rust 侧 Criterion，而不是原项目的 google/benchmark。

## 已复刻场景

当前 `crates/soul-ecs/benches/ecs_benchmark.rs` 覆盖以下原始 benchmark 名称：

- `BM_SystemsUpdate`
- `BM_SystemsUpdateMixedEntities`
- `BM_CreateNoEntities`
- `BM_CreateEmptyEntities`
- `BM_UnpackNoComponent`
- `BM_CreateEntities`
- `BM_UnpackOneComponent`
- `BM_UnpackTwoComponents`
- `BM_UnpackThreeComponents`
- `BM_AddComponent`
- `BM_RemoveAddComponent`
- `BM_DestroyEntities`
- `BM_CreateEntitiesInBulk`
- `BM_CreateEmptyEntitiesInBulk`
- `BM_ComplexSystemsUpdate`
- `BM_ComplexSystemsUpdateMixedEntities`
- `BM_IterateSingleComponent`
- `BM_IterateTwoComponents`
- `BM_IterateThreeComponentsWithMixedEntities`
- `BM_EnqueueAndUpdateEventsViaObserverWithMixedEntities`
- `BM_EmitAndUpdateEventsViaObserverWithMixedEntities`

除 emit 事件场景外，输入规模对齐原项目 `BEDefaultArguments`：从 `0` 到 `2_097_152`，按 2 倍递增。`BM_EmitAndUpdateEventsViaObserverWithMixedEntities` 对齐原项目 `BESmallArguments`：从 `0` 到 `32_768`，按 2 倍递增。

benchmark 内部实现了原项目 base 组件和系统逻辑的 Rust 版本，包括 position、velocity、data、player、health、damage、sprite、frame buffer 和 xoshiro128 随机数。

## 绑定能力

为避免 benchmark 绕过 `soul-ecs` 门面直接操作 raw flecs 指针，本轮补齐了以下 safe wrapper 能力：

- bulk 创建：`World::bulk_empty`、`World::bulk_with1`、`World::bulk_with2`、`World::bulk_with3`，底层绑定 flecs `ecs_bulk_init`。
- observer/event：`World::observer().event().each(...)` 对齐 flecs world observer；`Entity::observe` 对齐 flecs C++ `e.observe<Event>` 的实体级 observer；`Entity::emit`、`Entity::emit2`、`Entity::enqueue`、`Entity::enqueue2` 对齐空事件和带 id 列表事件。
- query entity row：`Query::each_entity` 用于事件 benchmark 中按 query 结果取得实体句柄。

## 差异边界

当前复刻的是 flecs 相关 benchmark 在 `soul-ecs` Rust 绑定层的等价实现，不是原 C++ 文件的逐行翻译。主要边界是：

- runner 使用 Criterion，因此输出格式、统计模型和命令行参数不同于 google/benchmark。
- 结果包含 `soul-ecs` safe wrapper、运行时借用检查、typed query/system/observer 封装和 FFI 调用成本。
- Rust benchmark 的组件初始化通过 typed safe API 表达；bulk 场景底层已经走 flecs `ecs_bulk_init`。

## 运行方式

只验证 benchmark 是否能编译：

```bash
cargo bench -p soul-ecs --bench ecs_benchmark --no-run
```

运行完整 benchmark：

```bash
cargo bench -p soul-ecs --bench ecs_benchmark
```

Criterion 会在 `target/criterion/` 下生成统计结果和 HTML 报告。

运行单个 smoke 场景：

```bash
cargo bench -p soul-ecs --bench ecs_benchmark -- BM_CreateNoEntities
```

## 解释边界

这些结果主要反映 `soul-ecs` safe wrapper、运行时借用检查、typed query/system 封装和 flecs C API 调用之间的组合成本。它不能直接和 `ecs_benchmark` 中其他 ECS 框架的 google/benchmark 结果比较。
