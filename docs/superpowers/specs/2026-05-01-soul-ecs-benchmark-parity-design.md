# soul-ecs benchmark 复刻设计

## 背景

上一版 benchmark 只按 `abeimler/ecs_benchmark` 的大类做了松散映射，没有严格复刻原项目的 flecs benchmark。用户要求“完全复刻”，因此本设计以原仓库 `benchmark/benchmarks/flecs*` 实际注册的 benchmark 名称和数据构造逻辑为准。

## 复刻范围

本轮复刻以下四组 benchmark：

- `flecs`：`BM_SystemsUpdate`、`BM_SystemsUpdateMixedEntities`；
- `flecs-entities`：`BM_CreateNoEntities`、`BM_CreateEmptyEntities`、`BM_UnpackNoComponent`、`BM_CreateEntities`、`BM_UnpackOneComponent`、`BM_UnpackTwoComponents`、`BM_UnpackThreeComponents`、`BM_AddComponent`、`BM_RemoveAddComponent`、`BM_DestroyEntities`、`BM_CreateEntitiesInBulk`、`BM_CreateEmptyEntitiesInBulk`；
- `flecs-extended`：`BM_ComplexSystemsUpdate`、`BM_ComplexSystemsUpdateMixedEntities`、`BM_IterateSingleComponent`、`BM_IterateTwoComponents`、`BM_IterateThreeComponentsWithMixedEntities`；
- `flecs-extended` event：`BM_EnqueueAndUpdateEventsViaObserverWithMixedEntities`、`BM_EmitAndUpdateEventsViaObserverWithMixedEntities`。

除 emit 事件场景外，输入规模按原项目 `BEDefaultArguments` 生成：从 `0` 到 `2_097_152`，倍数为 `2`。emit 事件场景按原项目 `BESmallArguments` 生成：从 `0` 到 `32_768`，倍数为 `2`。为了避免完整 benchmark 在日常验证中耗时过长，文档同时提供按名称过滤的小样本 smoke 命令。

## API 补齐

为了避免 benchmark 绕过 `soul-ecs` 安全门面直接使用 raw flecs 指针，本轮同步补齐以下绑定能力：

- bulk API：`World::bulk_empty`、`World::bulk_with1`、`World::bulk_with2`、`World::bulk_with3`，底层封装 flecs `ecs_bulk_init`；
- world observer API：`World::observer().event().each(...)`，用于 enqueue 事件场景；
- entity observer API：`Entity::observe`，按 flecs C++ `e.observe<Event>` 的结构创建固定 source 的 observer；
- event API：`Entity::emit`、`Entity::emit2`、`Entity::enqueue`、`Entity::enqueue2`；
- query entity row API：`Query::each_entity`，用于 benchmark 按 query 命中实体触发事件。

## 实现策略

benchmark 文件保留 Criterion 作为 Rust 端 runner，但名称、输入规模、组件模型、实体构造模式和系统更新逻辑对齐原项目。组件和系统逻辑在 benchmark 内部实现，代码使用英文命名。

为了支持销毁 benchmark，`soul-ecs` 新增 `Entity::destruct(self)`，封装 flecs entity delete/destruct 行为。bulk benchmark 通过 `soul-ecs` typed safe bulk API 调用 flecs `ecs_bulk_init`。observer/event benchmark 通过新增的 safe observer/event API 表达，不在 benchmark 中直接绕过门面调用 raw flecs 指针。
