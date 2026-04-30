# soul-ecs benchmark 设计

## 目标

为 `soul-ecs` 增加一组可重复运行的 Criterion benchmark，场景参考 `abeimler/ecs_benchmark` 的常见 ECS 操作分类，但只覆盖当前 `soul-ecs` 已公开且稳定的 API。

## 范围

第一版 benchmark 聚焦绑定层开销，不做跨 ECS 框架排名，也不引入其他 ECS crate 作为对照。覆盖场景包括：

- entity 创建；
- component set、get、get_mut、remove；
- query 读写迭代；
- system progress 更新。

## 架构

benchmark 放在 `crates/soul-ecs/benches/ecs_benchmark.rs`，作为 `soul-ecs` crate 的 Criterion bench target。Cargo 配置使用 `[[bench]] harness = false`，运行入口由 `criterion_group!` 和 `criterion_main!` 提供。

benchmark 内部定义专用的 `Position`、`Velocity` 和 `Tag` 测试组件，并使用固定实体数量构建世界。每个场景都通过 `std::hint::black_box` 降低优化器消除工作负载的概率。

## 运行方式

使用以下命令构建并运行：

```bash
cargo bench -p soul-ecs --bench ecs_benchmark
```

快速验证 benchmark 是否可编译：

```bash
cargo bench -p soul-ecs --bench ecs_benchmark --no-run
```

## 非目标

本次不复制 flecs upstream 的完整测试套件，不实现 `ecs_benchmark` 的跨语言 runner，也不加入第三方 ECS 框架对比。后续如果需要横向对比，可以在此基础上新增独立 benchmark crate。
