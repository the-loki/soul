# soul-ecs benchmark 复刻设计

## 背景

上一版 benchmark 只按 `abeimler/ecs_benchmark` 的大类做了松散映射，没有严格复刻原项目的 flecs benchmark。用户要求“完全复刻”，因此本设计以原仓库 `benchmark/benchmarks/flecs*` 实际注册的 benchmark 名称和数据构造逻辑为准。

## 复刻范围

本轮复刻以下三组 benchmark：

- `flecs`：`BM_SystemsUpdate`、`BM_SystemsUpdateMixedEntities`；
- `flecs-entities`：`BM_CreateNoEntities`、`BM_CreateEmptyEntities`、`BM_UnpackNoComponent`、`BM_CreateEntities`、`BM_UnpackOneComponent`、`BM_UnpackTwoComponents`、`BM_UnpackThreeComponents`、`BM_AddComponent`、`BM_RemoveAddComponent`、`BM_DestroyEntities`、`BM_CreateEntitiesInBulk`、`BM_CreateEmptyEntitiesInBulk`；
- `flecs-extended`：`BM_ComplexSystemsUpdate`、`BM_ComplexSystemsUpdateMixedEntities`、`BM_IterateSingleComponent`、`BM_IterateTwoComponents`、`BM_IterateThreeComponentsWithMixedEntities`。

输入规模按原项目 `BEDefaultArguments` 生成：从 `0` 到 `2_097_152`，倍数为 `2`。为了避免完整 benchmark 在日常验证中耗时过长，文档同时提供按名称过滤的小样本 smoke 命令。

## API 缺口

原项目还有 `flecs-extended` 下的 observer/event benchmark：

- `BM_EnqueueAndUpdateEventsViaObserverWithMixedEntities`；
- `BM_EmitAndUpdateEventsViaObserverWithMixedEntities`。

当前 `soul-ecs` 没有公开 observer/event API。为了避免 benchmark 绕过 `soul-ecs` 安全门面直接使用 raw flecs 指针，这两个场景本轮不伪造实现，而是在文档中列为未覆盖项。后续需要先设计并实现 observer/event 绑定，再加入对应 benchmark。

## 实现策略

benchmark 文件保留 Criterion 作为 Rust 端 runner，但名称、输入规模、组件模型、实体构造模式和系统更新逻辑对齐原项目。组件和系统逻辑在 benchmark 内部实现，代码使用英文命名。

为了支持销毁 benchmark，`soul-ecs` 新增 `Entity::destruct(self)`，封装 flecs entity delete/destruct 行为。bulk benchmark 使用 `soul-ecs` 当前安全 API 循环创建实体，保持调用面不绕过安全门面；文档会明确说明这不是 flecs C++ `ecs_bulk_init` 的 raw bulk 快路径。
