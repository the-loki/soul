# soul ECS Benchmark

`soul-ecs` 的 benchmark 参考 `abeimler/ecs_benchmark` 的 ECS 操作分类，但第一版只衡量 `soul-ecs` 自身绑定层，不做跨框架排名。

## 覆盖场景

当前 benchmark 覆盖以下场景：

- entity 创建；
- component set；
- tag component add/remove；
- component get；
- component get_mut；
- query 只读迭代；
- query 可变更新；
- system progress 更新。

每个场景使用固定实体数量输入，当前包括 `1_000`、`10_000` 和 `100_000`。benchmark 组件只在 bench 文件内部定义，不属于公开 API。

## 运行

只验证 benchmark 是否能编译：

```bash
cargo bench -p soul-ecs --bench ecs_benchmark --no-run
```

运行完整 benchmark：

```bash
cargo bench -p soul-ecs --bench ecs_benchmark
```

Criterion 会在 `target/criterion/` 下生成统计结果和 HTML 报告。

## 解释边界

这些结果主要反映 `soul-ecs` safe wrapper、运行时借用检查、typed query/system 封装和 flecs C API 调用之间的组合成本。它不代表 flecs 本体的完整性能，也不能直接和 `ecs_benchmark` 中其他 ECS 框架的结果比较。
