# soul ECS 基准测试

`soul-ecs` 的 benchmark 对照 `abeimler/ecs_benchmark` 中 flecs 相关 benchmark 的命名、输入规模和主要实体构造逻辑。当前 runner 使用 Rust 侧 Criterion，而不是原项目的 google/benchmark。

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

输入规模对齐原项目 `BEDefaultArguments`：从 `0` 到 `2_097_152`，按 2 倍递增。

benchmark 内部实现了原项目 base 组件和系统逻辑的 Rust 版本，包括 position、velocity、data、player、health、damage、sprite、frame buffer 和 xoshiro128 随机数。

## 差异与缺口

当前仍有两个差异：

- `BM_CreateEntitiesInBulk` 和 `BM_CreateEmptyEntitiesInBulk` 使用 `soul-ecs` safe API 循环创建实体，没有绕过门面调用 flecs raw `ecs_bulk_init`。如果需要原项目 bulk 快路径语义，需要先在 `soul-ecs` 增加安全 bulk API。
- 原项目 flecs observer/event 扩展中的 `BM_EnqueueAndUpdateEventsViaObserverWithMixedEntities` 和 `BM_EmitAndUpdateEventsViaObserverWithMixedEntities` 尚未复刻。原因是 `soul-ecs` 当前没有 observer/event API；不应在 `soul-ecs` benchmark 中直接绕过门面使用 raw flecs 指针伪造结果。

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
