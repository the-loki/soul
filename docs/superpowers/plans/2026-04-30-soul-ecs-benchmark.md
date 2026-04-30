# soul-ecs Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 `soul-ecs` 增加参考 `ecs_benchmark` 场景的 Criterion benchmark。

**Architecture:** benchmark 作为 `soul-ecs` crate 的 bench target 存放在 `crates/soul-ecs/benches/ecs_benchmark.rs`。Cargo 使用 Criterion 的 `harness = false` 入口，文档放在 `docs/ecs/benchmark.md`。

**Tech Stack:** Rust 2021、Criterion.rs 0.7、Cargo bench、soul-ecs typed entity/query/system API。

---

### Task 1: Benchmark target 配置

**Files:**
- Modify: `crates/soul-ecs/Cargo.toml`
- Create: `crates/soul-ecs/benches/ecs_benchmark.rs`

- [x] **Step 1: Verify missing benchmark target**

Run: `cargo bench -p soul-ecs --bench ecs_benchmark --no-run`

Expected: FAIL because the benchmark target does not exist.

- [x] **Step 2: Add Criterion dev dependency and bench target**

Add `criterion = { version = "0.7", features = ["html_reports"] }` under `[dev-dependencies]`.

Add:

```toml
[[bench]]
name = "ecs_benchmark"
harness = false
```

- [x] **Step 3: Add a minimal compiling benchmark file**

Create `crates/soul-ecs/benches/ecs_benchmark.rs` with Criterion imports, a minimal benchmark function, and `criterion_group!` / `criterion_main!`.

- [x] **Step 4: Verify benchmark target compiles**

Run: `cargo bench -p soul-ecs --bench ecs_benchmark --no-run`

Expected: PASS.

### Task 2: ECS benchmark scenarios

**Files:**
- Modify: `crates/soul-ecs/benches/ecs_benchmark.rs`

- [x] **Step 1: Add benchmark-only components and world builders**

Define `Position`, `Velocity`, and `Tag`. Add helper functions that create worlds with fixed entity counts and component layouts.

- [x] **Step 2: Add entity/component benchmarks**

Add benchmarks for entity creation, component set, component get, component get_mut, and component remove.

- [x] **Step 3: Add query/system benchmarks**

Add benchmarks for read-only query iteration, mutable query update, and system progress update.

- [x] **Step 4: Verify benchmark target compiles**

Run: `cargo bench -p soul-ecs --bench ecs_benchmark --no-run`

Expected: PASS.

### Task 3: Documentation and final verification

**Files:**
- Create: `docs/ecs/benchmark.md`
- Modify: `README.md`

- [x] **Step 1: Document benchmark usage**

Write Chinese documentation explaining covered scenarios and the commands for compile-only and full benchmark runs.

- [x] **Step 2: Link benchmark documentation**

Add `docs/ecs/benchmark.md` to the README document list.

- [x] **Step 3: Run verification**

Run:

```bash
cargo fmt --all --check
cargo test --workspace
cargo bench -p soul-ecs --bench ecs_benchmark --no-run
```

Expected: all commands pass.
