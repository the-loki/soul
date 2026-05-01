# soul-ecs Benchmark Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `soul-ecs` benchmark 从松散映射改为对齐 `abeimler/ecs_benchmark` 的 flecs benchmark 名称、输入规模和核心场景。

**Architecture:** 继续使用 `crates/soul-ecs/benches/ecs_benchmark.rs` 作为 Criterion bench target。新增 `Entity::destruct(self)` 支持销毁场景，benchmark 内部定义原项目等价组件、实体构造 helper、系统 update helper 和 benchmark group。

**Tech Stack:** Rust 2021、Criterion.rs 0.7、soul-ecs safe API、vendored flecs C API。

---

### Task 1: Entity destruction API

**Files:**
- Modify: `crates/soul-ecs/src/entity.rs`
- Modify: `crates/soul-ecs/tests/entity.rs`

- [x] **Step 1: Add failing test**

Add a test that creates an entity with two components, calls `destruct`, and verifies a query no longer visits it.

- [x] **Step 2: Run test and verify failure**

Run: `cargo test -p soul-ecs --test entity entity_destruct_removes_entity_from_queries`

Expected: FAIL before implementation because `destruct` does not exist.

- [x] **Step 3: Implement `Entity::destruct(self)`**

Call flecs delete/destruct through `soul-ecs-sys`, after checking structural mutation guards.

- [x] **Step 4: Verify test passes**

Run: `cargo test -p soul-ecs --test entity entity_destruct_removes_entity_from_queries`

Expected: PASS.

### Task 2: Benchmark parity rewrite

**Files:**
- Modify: `crates/soul-ecs/benches/ecs_benchmark.rs`
- Modify: `docs/ecs/benchmark.md`

- [x] **Step 1: Replace loose benchmark names**

Rewrite benchmark groups to expose names matching the original flecs benchmark names.

- [x] **Step 2: Add benchmark components and helpers**

Implement `PositionComponent`, `VelocityComponent`, `DataComponent`, `PlayerComponent`, `HealthComponent`, `DamageComponent`, `SpriteComponent`, `EmptyComponent`, frame buffer, RNG, and helper constructors matching the original base logic.

- [x] **Step 3: Add entity benchmark scenarios**

Implement create, bulk-like create, unpack, add/remove, and destroy scenarios.

- [x] **Step 4: Add system and iteration benchmark scenarios**

Implement basic update, mixed update, complex update, mixed complex update, and one/two/three component iteration scenarios.

- [x] **Step 5: Document unsupported observer/event parity**

Update `docs/ecs/benchmark.md` to state exactly which original flecs benchmark groups are implemented and which observer/event scenarios require future API work.

### Task 3: Verification

**Files:**
- All changed files

- [x] **Step 1: Run formatting**

Run: `cargo fmt --all --check`

Expected: PASS.

- [x] **Step 2: Run tests**

Run: `cargo test --workspace`

Expected: PASS.

- [x] **Step 3: Run benchmark compile**

Run: `cargo bench -p soul-ecs --bench ecs_benchmark --no-run`

Expected: PASS.

- [x] **Step 4: Run clippy**

Run: `cargo clippy -p soul-ecs --all-targets -- -D warnings`

Expected: PASS.
