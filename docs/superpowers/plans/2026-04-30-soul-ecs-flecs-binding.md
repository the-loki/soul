# soul ECS Flecs Binding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建 `soul-ecs-sys` 与 `soul-ecs` 两个 crate，实现基于 flecs C API 的 safe Rust ECS 门面，并让公开 API 参考 flecs C++ typed builder 风格。

**Architecture:** `soul-ecs-sys` vendored flecs v4.1.5，并用一个小型 C shim 屏蔽 flecs descriptor 与 iterator 结构体布局；`soul-ecs` 只暴露 safe Rust API，内部通过 type registry 把 `Copy + 'static` Rust component 映射到 flecs component id。query 与 system 共享 tuple 参数 trait，把 `(&mut T, &U)` 映射为 flecs term 和 iterator 字段。

**Tech Stack:** Rust 2021、Cargo workspace、`cc` build dependency、vendored flecs v4.1.5 C source、Rust integration tests。

---

## 文件结构

- 修改：`Cargo.toml`，加入 workspace members 与 workspace dependencies。
- 创建：`crates/soul-ecs-sys/Cargo.toml`，低层 FFI crate 配置。
- 创建：`crates/soul-ecs-sys/build.rs`，用 `cc` 编译 vendored `flecs.c` 与 `src/shim.c`。
- 创建：`crates/soul-ecs-sys/src/lib.rs`，手写最小 FFI 声明，只暴露 opaque pointer、整数类型和 shim 函数。
- 创建：`crates/soul-ecs-sys/src/shim.c`，封装 flecs component/query/system descriptor 和 iterator 字段访问。
- 创建：`crates/soul-ecs-sys/vendor/flecs/flecs.c`，来自 flecs `v4.1.5/distr/flecs.c`。
- 创建：`crates/soul-ecs-sys/vendor/flecs/flecs.h`，来自 flecs `v4.1.5/distr/flecs.h`。
- 创建：`crates/soul-ecs/Cargo.toml`，safe wrapper crate 配置。
- 创建：`crates/soul-ecs/src/lib.rs`，公开 `World`、`Entity`、`QueryBuilder`、`Query`、`SystemBuilder`。
- 创建：`crates/soul-ecs/src/world.rs`，world 生命周期、registry、builder 入口。
- 创建：`crates/soul-ecs/src/entity.rs`，entity set/get/has/remove。
- 创建：`crates/soul-ecs/src/registry.rs`，component type registry。
- 创建：`crates/soul-ecs/src/param.rs`，tuple 参数注册、字段访问和 query/system 共享逻辑。
- 创建：`crates/soul-ecs/src/query.rs`，query builder、query 迭代。
- 创建：`crates/soul-ecs/src/system.rs`，system builder、callback trampoline。
- 创建：`crates/soul-ecs/tests/world.rs`，world 生命周期测试。
- 创建：`crates/soul-ecs/tests/entity.rs`，entity/component 测试。
- 创建：`crates/soul-ecs/tests/query.rs`，query 测试。
- 创建：`crates/soul-ecs/tests/system.rs`，system/progress 测试。
- 创建：`docs/ecs/overview.md`，中文架构与用法文档。
- 创建：`docs/ecs/safety.md`，中文 unsafe 边界说明。

## Task 1: Workspace 与 vendored flecs sys crate

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/soul-ecs-sys/Cargo.toml`
- Create: `crates/soul-ecs-sys/build.rs`
- Create: `crates/soul-ecs-sys/src/lib.rs`
- Create: `crates/soul-ecs-sys/src/shim.c`
- Create: `crates/soul-ecs-sys/vendor/flecs/flecs.c`
- Create: `crates/soul-ecs-sys/vendor/flecs/flecs.h`

- [ ] **Step 1: Write the failing sys smoke test**

Create `crates/soul-ecs-sys/tests/world.rs`:

```rust
// Covers raw world initialization and finalization through the sys crate.
#[test]
fn creates_and_frees_raw_world() {
    let world = unsafe { soul_ecs_sys::ecs_init() };
    assert!(!world.is_null());

    let result = unsafe { soul_ecs_sys::ecs_fini(world) };
    assert_eq!(result, 0);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p soul-ecs-sys creates_and_frees_raw_world
```

Expected: FAIL because package `soul-ecs-sys` does not exist.

- [ ] **Step 3: Fetch vendored flecs v4.1.5**

Run:

```bash
mkdir -p crates/soul-ecs-sys/vendor/flecs
curl -L --fail -o crates/soul-ecs-sys/vendor/flecs/flecs.c https://raw.githubusercontent.com/SanderMertens/flecs/v4.1.5/distr/flecs.c
curl -L --fail -o crates/soul-ecs-sys/vendor/flecs/flecs.h https://raw.githubusercontent.com/SanderMertens/flecs/v4.1.5/distr/flecs.h
```

Expected: both files are written under `crates/soul-ecs-sys/vendor/flecs/`.

- [ ] **Step 4: Add workspace members and sys crate files**

Update root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/soul-ecs-sys",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT"

[workspace.dependencies]
cc = "1"
soul-ecs-sys = { path = "crates/soul-ecs-sys" }
```

Create `crates/soul-ecs-sys/Cargo.toml`:

```toml
[package]
name = "soul-ecs-sys"
version = "0.1.0"
edition.workspace = true
license.workspace = true
build = "build.rs"

[build-dependencies]
cc.workspace = true
```

Create `crates/soul-ecs-sys/build.rs`:

```rust
fn main() {
    println!("cargo:rerun-if-changed=vendor/flecs/flecs.c");
    println!("cargo:rerun-if-changed=vendor/flecs/flecs.h");
    println!("cargo:rerun-if-changed=src/shim.c");

    cc::Build::new()
        .file("vendor/flecs/flecs.c")
        .file("src/shim.c")
        .include("vendor/flecs")
        .define("FLECS_NO_CPP", None)
        .compile("soul_ecs_flecs");
}
```

Create `crates/soul-ecs-sys/src/shim.c`:

```c
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include "flecs.h"

typedef struct soul_ecs_query_iter_t {
    ecs_iter_t iter;
    bool finished;
} soul_ecs_query_iter_t;

ecs_entity_t soul_ecs_component_init(
    ecs_world_t *world,
    const char *name,
    size_t size,
    size_t alignment
) {
    ecs_entity_desc_t entity_desc = {0};
    entity_desc.name = name;

    ecs_component_desc_t desc = {0};
    desc.entity = ecs_entity_init(world, &entity_desc);
    desc.type.size = size;
    desc.type.alignment = alignment;

    return ecs_component_init(world, &desc);
}

ecs_query_t *soul_ecs_query_init(
    ecs_world_t *world,
    const ecs_id_t *ids,
    const int16_t *inouts,
    int32_t count
) {
    if (count < 0 || count > FLECS_TERM_COUNT_MAX) {
        return NULL;
    }

    ecs_query_desc_t desc = {0};
    for (int32_t i = 0; i < count; i++) {
        desc.terms[i].id = ids[i];
        desc.terms[i].inout = inouts[i];
    }

    return ecs_query_init(world, &desc);
}

soul_ecs_query_iter_t *soul_ecs_query_iter(
    const ecs_world_t *world,
    const ecs_query_t *query
) {
    soul_ecs_query_iter_t *wrapper = calloc(1, sizeof(soul_ecs_query_iter_t));
    if (!wrapper) {
        return NULL;
    }

    wrapper->iter = ecs_query_iter(world, query);
    return wrapper;
}

bool soul_ecs_query_next(soul_ecs_query_iter_t *wrapper) {
    bool has_next = ecs_query_next(&wrapper->iter);
    wrapper->finished = !has_next;
    return has_next;
}

int32_t soul_ecs_query_iter_count(const soul_ecs_query_iter_t *wrapper) {
    return wrapper->iter.count;
}

void *soul_ecs_query_iter_field(
    const soul_ecs_query_iter_t *wrapper,
    size_t size,
    int8_t index
) {
    return ecs_field_w_size(&wrapper->iter, size, index);
}

void soul_ecs_query_iter_fini(soul_ecs_query_iter_t *wrapper) {
    if (!wrapper) {
        return;
    }
    if (!wrapper->finished) {
        ecs_iter_fini(&wrapper->iter);
    }
    free(wrapper);
}

ecs_entity_t soul_ecs_system_init(
    ecs_world_t *world,
    const ecs_id_t *ids,
    const int16_t *inouts,
    int32_t count,
    ecs_iter_action_t callback,
    void *ctx,
    ecs_ctx_free_t ctx_free
) {
    if (count < 0 || count > FLECS_TERM_COUNT_MAX) {
        return 0;
    }

    ecs_system_desc_t desc = {0};
    for (int32_t i = 0; i < count; i++) {
        desc.query.terms[i].id = ids[i];
        desc.query.terms[i].inout = inouts[i];
    }
    desc.callback = callback;
    desc.ctx = ctx;
    desc.ctx_free = ctx_free;
    desc.phase = EcsOnUpdate;

    return ecs_system_init(world, &desc);
}

int32_t soul_ecs_iter_count(const ecs_iter_t *iter) {
    return iter->count;
}

void *soul_ecs_iter_field(const ecs_iter_t *iter, size_t size, int8_t index) {
    return ecs_field_w_size(iter, size, index);
}

void *soul_ecs_iter_ctx(const ecs_iter_t *iter) {
    return iter->ctx;
}
```

Create `crates/soul-ecs-sys/src/lib.rs`:

```rust
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_void};

pub enum ecs_world_t {}
pub enum ecs_query_t {}
pub enum ecs_iter_t {}
pub enum soul_ecs_query_iter_t {}

pub type ecs_entity_t = u64;
pub type ecs_id_t = u64;
pub type ecs_ftime_t = f32;
pub type ecs_iter_action_t = Option<unsafe extern "C" fn(*mut ecs_iter_t)>;
pub type ecs_ctx_free_t = Option<unsafe extern "C" fn(*mut c_void)>;

pub const ECS_INOUT_DEFAULT: i16 = 0;
pub const ECS_INOUT_NONE: i16 = 1;
pub const ECS_INOUT_FILTER: i16 = 2;
pub const ECS_INOUT: i16 = 3;
pub const ECS_IN: i16 = 4;
pub const ECS_OUT: i16 = 5;

extern "C" {
    pub fn ecs_init() -> *mut ecs_world_t;
    pub fn ecs_fini(world: *mut ecs_world_t) -> i32;
    pub fn ecs_progress(world: *mut ecs_world_t, delta_time: ecs_ftime_t) -> bool;
    pub fn ecs_new(world: *mut ecs_world_t) -> ecs_entity_t;
    pub fn ecs_add_id(world: *mut ecs_world_t, entity: ecs_entity_t, id: ecs_id_t);
    pub fn ecs_remove_id(world: *mut ecs_world_t, entity: ecs_entity_t, id: ecs_id_t);
    pub fn ecs_has_id(world: *const ecs_world_t, entity: ecs_entity_t, id: ecs_id_t) -> bool;
    pub fn ecs_set_id(
        world: *mut ecs_world_t,
        entity: ecs_entity_t,
        id: ecs_id_t,
        size: usize,
        ptr: *const c_void,
    );
    pub fn ecs_get_id(
        world: *const ecs_world_t,
        entity: ecs_entity_t,
        id: ecs_id_t,
    ) -> *const c_void;
    pub fn ecs_get_mut_id(
        world: *const ecs_world_t,
        entity: ecs_entity_t,
        id: ecs_id_t,
    ) -> *mut c_void;
    pub fn ecs_modified_id(world: *mut ecs_world_t, entity: ecs_entity_t, id: ecs_id_t);
    pub fn ecs_query_fini(query: *mut ecs_query_t);

    pub fn soul_ecs_component_init(
        world: *mut ecs_world_t,
        name: *const c_char,
        size: usize,
        alignment: usize,
    ) -> ecs_entity_t;
    pub fn soul_ecs_query_init(
        world: *mut ecs_world_t,
        ids: *const ecs_id_t,
        inouts: *const i16,
        count: i32,
    ) -> *mut ecs_query_t;
    pub fn soul_ecs_query_iter(
        world: *const ecs_world_t,
        query: *const ecs_query_t,
    ) -> *mut soul_ecs_query_iter_t;
    pub fn soul_ecs_query_next(iter: *mut soul_ecs_query_iter_t) -> bool;
    pub fn soul_ecs_query_iter_count(iter: *const soul_ecs_query_iter_t) -> i32;
    pub fn soul_ecs_query_iter_field(
        iter: *const soul_ecs_query_iter_t,
        size: usize,
        index: i8,
    ) -> *mut c_void;
    pub fn soul_ecs_query_iter_fini(iter: *mut soul_ecs_query_iter_t);
    pub fn soul_ecs_system_init(
        world: *mut ecs_world_t,
        ids: *const ecs_id_t,
        inouts: *const i16,
        count: i32,
        callback: ecs_iter_action_t,
        ctx: *mut c_void,
        ctx_free: ecs_ctx_free_t,
    ) -> ecs_entity_t;
    pub fn soul_ecs_iter_count(iter: *const ecs_iter_t) -> i32;
    pub fn soul_ecs_iter_field(iter: *const ecs_iter_t, size: usize, index: i8) -> *mut c_void;
    pub fn soul_ecs_iter_ctx(iter: *const ecs_iter_t) -> *mut c_void;
}
```

- [ ] **Step 5: Run the sys smoke test**

Run:

```bash
cargo test -p soul-ecs-sys creates_and_frees_raw_world
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add Cargo.toml crates/soul-ecs-sys
git commit -m "feat: add flecs sys crate"
```

## Task 2: Safe World 生命周期

**Files:**
- Create: `crates/soul-ecs/Cargo.toml`
- Create: `crates/soul-ecs/src/lib.rs`
- Create: `crates/soul-ecs/src/world.rs`
- Create: `crates/soul-ecs/tests/world.rs`

- [ ] **Step 1: Write the failing world test**

Create `crates/soul-ecs/tests/world.rs`:

```rust
use soul_ecs::World;

// Covers safe world creation, raw pointer ownership, and drop.
#[test]
fn creates_and_drops_world() {
    let world = World::new();
    assert!(!world.as_ptr().is_null());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p soul-ecs creates_and_drops_world
```

Expected: FAIL because package `soul-ecs` does not exist.

- [ ] **Step 3: Implement minimal `World`**

Update root `Cargo.toml` workspace members:

```toml
[workspace]
members = [
    "crates/soul-ecs-sys",
    "crates/soul-ecs",
]
resolver = "2"
```

Create `crates/soul-ecs/Cargo.toml`:

```toml
[package]
name = "soul-ecs"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
soul-ecs-sys.workspace = true
```

Create `crates/soul-ecs/src/lib.rs`:

```rust
mod world;

pub use world::World;
```

Create `crates/soul-ecs/src/world.rs`:

```rust
use soul_ecs_sys as sys;

pub struct World {
    raw: *mut sys::ecs_world_t,
}

impl World {
    pub fn new() -> Self {
        let raw = unsafe { sys::ecs_init() };
        assert!(!raw.is_null(), "failed to initialize flecs world");
        Self { raw }
    }

    pub fn as_ptr(&self) -> *mut sys::ecs_world_t {
        self.raw
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for World {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            let result = unsafe { sys::ecs_fini(self.raw) };
            debug_assert_eq!(result, 0);
            self.raw = std::ptr::null_mut();
        }
    }
}
```

- [ ] **Step 4: Run the world test**

Run:

```bash
cargo test -p soul-ecs creates_and_drops_world
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add Cargo.toml crates/soul-ecs
git commit -m "feat: add safe ecs world"
```

## Task 3: Entity 与 Copy component 操作

**Files:**
- Modify: `crates/soul-ecs/src/lib.rs`
- Modify: `crates/soul-ecs/src/world.rs`
- Create: `crates/soul-ecs/src/entity.rs`
- Create: `crates/soul-ecs/src/registry.rs`
- Create: `crates/soul-ecs/tests/entity.rs`

- [ ] **Step 1: Write failing entity/component tests**

Create `crates/soul-ecs/tests/entity.rs`:

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

// Covers set, has, get, get_mut, and remove for one component.
#[test]
fn entity_manages_single_component() {
    let world = World::new();
    let entity = world.entity().set(Position { x: 1.0, y: 2.0 });

    assert!(entity.has::<Position>());

    entity.get::<Position>(|position| {
        assert_eq!(*position, Position { x: 1.0, y: 2.0 });
    });

    entity.get_mut::<Position>(|position| {
        position.x += 3.0;
    });

    entity.get::<Position>(|position| {
        assert_eq!(*position, Position { x: 4.0, y: 2.0 });
    });

    let entity = entity.remove::<Position>();
    assert!(!entity.has::<Position>());
}

// Covers C++-style chained entity construction with multiple components.
#[test]
fn entity_supports_chained_component_set() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    assert!(entity.has::<Position>());
    assert!(entity.has::<Velocity>());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p soul-ecs --test entity
```

Expected: FAIL because `World::entity` and `Entity` component methods do not exist.

- [ ] **Step 3: Implement registry and entity**

Update `crates/soul-ecs/src/lib.rs`:

```rust
mod entity;
mod registry;
mod world;

pub use entity::Entity;
pub use world::World;
```

Create `crates/soul-ecs/src/registry.rs`:

```rust
use std::any::{type_name, TypeId};
use std::collections::HashMap;
use std::ffi::CString;
use std::mem;

use soul_ecs_sys as sys;

#[derive(Clone, Copy)]
pub(crate) struct ComponentInfo {
    pub(crate) id: sys::ecs_id_t,
    pub(crate) size: usize,
}

#[derive(Default)]
pub(crate) struct Registry {
    components: HashMap<TypeId, ComponentInfo>,
}

impl Registry {
    pub(crate) fn component<T: Copy + 'static>(
        &mut self,
        world: *mut sys::ecs_world_t,
    ) -> ComponentInfo {
        let type_id = TypeId::of::<T>();
        if let Some(info) = self.components.get(&type_id) {
            return *info;
        }

        let name = CString::new(type_name::<T>()).expect("component type name contains nul byte");
        let id = unsafe {
            sys::soul_ecs_component_init(
                world,
                name.as_ptr(),
                mem::size_of::<T>(),
                mem::align_of::<T>(),
            )
        };
        assert_ne!(id, 0, "failed to register component");

        let info = ComponentInfo {
            id,
            size: mem::size_of::<T>(),
        };
        self.components.insert(type_id, info);
        info
    }
}
```

Create `crates/soul-ecs/src/entity.rs`:

```rust
use soul_ecs_sys as sys;

use crate::world::World;

#[derive(Clone, Copy)]
pub struct Entity<'world> {
    world: &'world World,
    raw: sys::ecs_entity_t,
}

impl<'world> Entity<'world> {
    pub(crate) fn new(world: &'world World, raw: sys::ecs_entity_t) -> Self {
        Self { world, raw }
    }

    pub fn id(&self) -> sys::ecs_entity_t {
        self.raw
    }

    pub fn add<T: Copy + 'static>(self) -> Self {
        let info = self.world.component_info::<T>();
        unsafe { sys::ecs_add_id(self.world.as_ptr(), self.raw, info.id) };
        self
    }

    pub fn set<T: Copy + 'static>(self, value: T) -> Self {
        let info = self.world.component_info::<T>();
        unsafe {
            sys::ecs_set_id(
                self.world.as_ptr(),
                self.raw,
                info.id,
                info.size,
                (&value as *const T).cast(),
            );
        }
        self
    }

    pub fn has<T: Copy + 'static>(&self) -> bool {
        let info = self.world.component_info::<T>();
        unsafe { sys::ecs_has_id(self.world.as_ptr(), self.raw, info.id) }
    }

    pub fn get<T: Copy + 'static>(&self, f: impl FnOnce(&T)) {
        let info = self.world.component_info::<T>();
        let ptr = unsafe { sys::ecs_get_id(self.world.as_ptr(), self.raw, info.id) };
        assert!(!ptr.is_null(), "component does not exist on entity");
        unsafe {
            // SAFETY: flecs returned a non-null pointer for component T on this entity.
            f(&*(ptr.cast::<T>()));
        }
    }

    pub fn get_mut<T: Copy + 'static>(&self, f: impl FnOnce(&mut T)) {
        let info = self.world.component_info::<T>();
        let ptr = unsafe { sys::ecs_get_mut_id(self.world.as_ptr(), self.raw, info.id) };
        assert!(!ptr.is_null(), "component does not exist on entity");
        unsafe {
            // SAFETY: flecs returned a unique mutable pointer for component T on this entity.
            f(&mut *(ptr.cast::<T>()));
        }
        unsafe { sys::ecs_modified_id(self.world.as_ptr(), self.raw, info.id) };
    }

    pub fn remove<T: Copy + 'static>(self) -> Self {
        let info = self.world.component_info::<T>();
        unsafe { sys::ecs_remove_id(self.world.as_ptr(), self.raw, info.id) };
        self
    }
}
```

Update `crates/soul-ecs/src/world.rs`:

```rust
use std::cell::RefCell;

use soul_ecs_sys as sys;

use crate::entity::Entity;
use crate::registry::{ComponentInfo, Registry};

pub struct World {
    raw: *mut sys::ecs_world_t,
    registry: RefCell<Registry>,
}

impl World {
    pub fn new() -> Self {
        let raw = unsafe { sys::ecs_init() };
        assert!(!raw.is_null(), "failed to initialize flecs world");
        Self {
            raw,
            registry: RefCell::new(Registry::default()),
        }
    }

    pub fn as_ptr(&self) -> *mut sys::ecs_world_t {
        self.raw
    }

    pub fn entity(&self) -> Entity<'_> {
        let raw = unsafe { sys::ecs_new(self.raw) };
        assert_ne!(raw, 0, "failed to create entity");
        Entity::new(self, raw)
    }

    pub(crate) fn component_info<T: Copy + 'static>(&self) -> ComponentInfo {
        self.registry.borrow_mut().component::<T>(self.raw)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for World {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            let result = unsafe { sys::ecs_fini(self.raw) };
            debug_assert_eq!(result, 0);
            self.raw = std::ptr::null_mut();
        }
    }
}
```

- [ ] **Step 4: Run entity tests**

Run:

```bash
cargo test -p soul-ecs --test entity
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add crates/soul-ecs
git commit -m "feat: add entity component operations"
```

## Task 4: Query builder 与 `each`

**Files:**
- Modify: `crates/soul-ecs/src/lib.rs`
- Modify: `crates/soul-ecs/src/world.rs`
- Create: `crates/soul-ecs/src/param.rs`
- Create: `crates/soul-ecs/src/query.rs`
- Create: `crates/soul-ecs/tests/query.rs`

- [ ] **Step 1: Write failing query test**

Create `crates/soul-ecs/tests/query.rs`:

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

// Covers typed query iteration over mutable and readonly fields.
#[test]
fn query_each_updates_matching_entities() {
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
        assert_eq!(*position, Position { x: 11.0, y: 22.0 });
    });
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p soul-ecs --test query
```

Expected: FAIL because `World::query` does not exist.

- [ ] **Step 3: Implement query parameter trait and query wrapper**

Create `crates/soul-ecs/src/param.rs`:

```rust
use std::marker::PhantomData;
use std::mem;

use soul_ecs_sys as sys;

use crate::world::World;

pub(crate) struct Term {
    pub(crate) id: sys::ecs_id_t,
    pub(crate) inout: i16,
}

pub(crate) trait QueryParam {
    type Item<'a>;

    fn terms(world: &World) -> Vec<Term>;

    unsafe fn fetch_query<'a>(
        iter: *const sys::soul_ecs_query_iter_t,
        row: i32,
    ) -> Self::Item<'a>;

    unsafe fn fetch_system<'a>(iter: *const sys::ecs_iter_t, row: i32) -> Self::Item<'a>;
}

impl<T: Copy + 'static> QueryParam for (&T,) {
    type Item<'a> = (&'a T,);

    fn terms(world: &World) -> Vec<Term> {
        let info = world.component_info::<T>();
        vec![Term {
            id: info.id,
            inout: sys::ECS_IN,
        }]
    }

    unsafe fn fetch_query<'a>(
        iter: *const sys::soul_ecs_query_iter_t,
        row: i32,
    ) -> Self::Item<'a> {
        let ptr = sys::soul_ecs_query_iter_field(iter, mem::size_of::<T>(), 0).cast::<T>();
        (&*ptr.add(row as usize),)
    }

    unsafe fn fetch_system<'a>(iter: *const sys::ecs_iter_t, row: i32) -> Self::Item<'a> {
        let ptr = sys::soul_ecs_iter_field(iter, mem::size_of::<T>(), 0).cast::<T>();
        (&*ptr.add(row as usize),)
    }
}

impl<T: Copy + 'static> QueryParam for (&mut T,) {
    type Item<'a> = (&'a mut T,);

    fn terms(world: &World) -> Vec<Term> {
        let info = world.component_info::<T>();
        vec![Term {
            id: info.id,
            inout: sys::ECS_INOUT,
        }]
    }

    unsafe fn fetch_query<'a>(
        iter: *const sys::soul_ecs_query_iter_t,
        row: i32,
    ) -> Self::Item<'a> {
        let ptr = sys::soul_ecs_query_iter_field(iter, mem::size_of::<T>(), 0).cast::<T>();
        (&mut *ptr.add(row as usize),)
    }

    unsafe fn fetch_system<'a>(iter: *const sys::ecs_iter_t, row: i32) -> Self::Item<'a> {
        let ptr = sys::soul_ecs_iter_field(iter, mem::size_of::<T>(), 0).cast::<T>();
        (&mut *ptr.add(row as usize),)
    }
}

impl<T: Copy + 'static, U: Copy + 'static> QueryParam for (&mut T, &U) {
    type Item<'a> = (&'a mut T, &'a U);

    fn terms(world: &World) -> Vec<Term> {
        let first = world.component_info::<T>();
        let second = world.component_info::<U>();
        assert_ne!(first.id, second.id, "duplicate component in query parameter");
        vec![
            Term {
                id: first.id,
                inout: sys::ECS_INOUT,
            },
            Term {
                id: second.id,
                inout: sys::ECS_IN,
            },
        ]
    }

    unsafe fn fetch_query<'a>(
        iter: *const sys::soul_ecs_query_iter_t,
        row: i32,
    ) -> Self::Item<'a> {
        let first = sys::soul_ecs_query_iter_field(iter, mem::size_of::<T>(), 0).cast::<T>();
        let second = sys::soul_ecs_query_iter_field(iter, mem::size_of::<U>(), 1).cast::<U>();
        (&mut *first.add(row as usize), &*second.add(row as usize))
    }

    unsafe fn fetch_system<'a>(iter: *const sys::ecs_iter_t, row: i32) -> Self::Item<'a> {
        let first = sys::soul_ecs_iter_field(iter, mem::size_of::<T>(), 0).cast::<T>();
        let second = sys::soul_ecs_iter_field(iter, mem::size_of::<U>(), 1).cast::<U>();
        (&mut *first.add(row as usize), &*second.add(row as usize))
    }
}

pub(crate) struct ParamMarker<P>(pub(crate) PhantomData<P>);
```

Create `crates/soul-ecs/src/query.rs`:

```rust
use std::marker::PhantomData;

use soul_ecs_sys as sys;

use crate::param::QueryParam;
use crate::world::World;

pub struct QueryBuilder<'world, P> {
    world: &'world World,
    _marker: PhantomData<P>,
}

impl<'world, P: QueryParam> QueryBuilder<'world, P> {
    pub(crate) fn new(world: &'world World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn build(self) -> Query<'world, P> {
        let terms = P::terms(self.world);
        let ids: Vec<_> = terms.iter().map(|term| term.id).collect();
        let inouts: Vec<_> = terms.iter().map(|term| term.inout).collect();
        let raw = unsafe {
            sys::soul_ecs_query_init(
                self.world.as_ptr(),
                ids.as_ptr(),
                inouts.as_ptr(),
                ids.len() as i32,
            )
        };
        assert!(!raw.is_null(), "failed to create query");
        Query {
            world: self.world,
            raw,
            _marker: PhantomData,
        }
    }
}

pub struct Query<'world, P> {
    world: &'world World,
    raw: *mut sys::ecs_query_t,
    _marker: PhantomData<P>,
}

impl<P: QueryParam> Query<'_, P> {
    pub fn each<F>(&self, mut f: F)
    where
        F: for<'a> FnMut(P::Item<'a>),
    {
        let iter = unsafe { sys::soul_ecs_query_iter(self.world.as_ptr(), self.raw) };
        assert!(!iter.is_null(), "failed to create query iterator");

        while unsafe { sys::soul_ecs_query_next(iter) } {
            let count = unsafe { sys::soul_ecs_query_iter_count(iter) };
            for row in 0..count {
                let item = unsafe {
                    // SAFETY: P created the query terms, and row is within the current iterator count.
                    P::fetch_query(iter, row)
                };
                f(item);
            }
        }

        unsafe { sys::soul_ecs_query_iter_fini(iter) };
    }
}

impl<P> Drop for Query<'_, P> {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { sys::ecs_query_fini(self.raw) };
            self.raw = std::ptr::null_mut();
        }
    }
}
```

Update `crates/soul-ecs/src/lib.rs`:

```rust
mod entity;
mod param;
mod query;
mod registry;
mod world;

pub use entity::Entity;
pub use query::{Query, QueryBuilder};
pub use world::World;
```

Update `crates/soul-ecs/src/world.rs` by adding:

```rust
use crate::param::QueryParam;
use crate::query::QueryBuilder;

impl World {
    pub fn query<P: QueryParam>(&self) -> QueryBuilder<'_, P> {
        QueryBuilder::new(self)
    }
}
```

- [ ] **Step 4: Run query tests**

Run:

```bash
cargo test -p soul-ecs --test query
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add crates/soul-ecs
git commit -m "feat: add typed query iteration"
```

## Task 5: System builder 与 `World::progress`

**Files:**
- Modify: `crates/soul-ecs/src/lib.rs`
- Modify: `crates/soul-ecs/src/world.rs`
- Create: `crates/soul-ecs/src/system.rs`
- Create: `crates/soul-ecs/tests/system.rs`

- [ ] **Step 1: Write failing system test**

Create `crates/soul-ecs/tests/system.rs`:

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

// Covers C++-style typed system registration and execution through progress.
#[test]
fn system_each_runs_during_world_progress() {
    let world = World::new();
    let entity = world
        .entity()
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 2.0 });

    world
        .system::<(&mut Position, &Velocity)>()
        .each(|(position, velocity)| {
            position.x += velocity.x;
            position.y += velocity.y;
        });

    world.progress();

    entity.get::<Position>(|position| {
        assert_eq!(*position, Position { x: 11.0, y: 22.0 });
    });
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p soul-ecs --test system
```

Expected: FAIL because `World::system` and `World::progress` do not exist.

- [ ] **Step 3: Implement system builder and callback trampoline**

Create `crates/soul-ecs/src/system.rs`:

```rust
use std::ffi::c_void;
use std::marker::PhantomData;

use soul_ecs_sys as sys;

use crate::param::QueryParam;
use crate::world::World;

pub struct SystemBuilder<'world, P> {
    world: &'world World,
    _marker: PhantomData<P>,
}

impl<'world, P: QueryParam + 'static> SystemBuilder<'world, P> {
    pub(crate) fn new(world: &'world World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn each<F>(self, f: F) -> System
    where
        F: for<'a> FnMut(P::Item<'a>) + 'static,
    {
        let terms = P::terms(self.world);
        let ids: Vec<_> = terms.iter().map(|term| term.id).collect();
        let inouts: Vec<_> = terms.iter().map(|term| term.inout).collect();

        let callback: Box<SystemCallback<P>> = Box::new(SystemCallback {
            f: Box::new(f),
            _marker: PhantomData,
        });
        let ctx = Box::into_raw(callback).cast::<c_void>();

        let raw = unsafe {
            sys::soul_ecs_system_init(
                self.world.as_ptr(),
                ids.as_ptr(),
                inouts.as_ptr(),
                ids.len() as i32,
                Some(system_trampoline::<P>),
                ctx,
                Some(drop_system_callback::<P>),
            )
        };
        assert_ne!(raw, 0, "failed to create system");

        System { raw }
    }
}

type CallbackFn<P> = dyn for<'a> FnMut(<P as QueryParam>::Item<'a>);

struct SystemCallback<P: QueryParam> {
    f: Box<CallbackFn<P>>,
    _marker: PhantomData<P>,
}

unsafe extern "C" fn system_trampoline<P: QueryParam>(iter: *mut sys::ecs_iter_t) {
    let ctx = sys::soul_ecs_iter_ctx(iter).cast::<SystemCallback<P>>();
    assert!(!ctx.is_null(), "system callback context is null");
    let callback = &mut *ctx;
    let count = sys::soul_ecs_iter_count(iter);

    for row in 0..count {
        let item = unsafe {
            // SAFETY: flecs invokes this callback for a system created from P terms.
            P::fetch_system(iter, row)
        };
        (callback.f)(item);
    }
}

unsafe extern "C" fn drop_system_callback<P: QueryParam>(ctx: *mut c_void) {
    if !ctx.is_null() {
        unsafe {
            // SAFETY: ctx was allocated with Box::into_raw in SystemBuilder::each.
            drop(Box::from_raw(ctx.cast::<SystemCallback<P>>()));
        }
    }
}

#[derive(Clone, Copy)]
pub struct System {
    raw: sys::ecs_entity_t,
}

impl System {
    pub fn id(&self) -> sys::ecs_entity_t {
        self.raw
    }
}
```

Update `crates/soul-ecs/src/lib.rs`:

```rust
mod entity;
mod param;
mod query;
mod registry;
mod system;
mod world;

pub use entity::Entity;
pub use query::{Query, QueryBuilder};
pub use system::{System, SystemBuilder};
pub use world::World;
```

Update `crates/soul-ecs/src/world.rs` by adding:

```rust
use crate::system::SystemBuilder;

impl World {
    pub fn system<P: QueryParam + 'static>(&self) -> SystemBuilder<'_, P> {
        SystemBuilder::new(self)
    }

    pub fn progress(&self) -> bool {
        unsafe { sys::ecs_progress(self.raw, 0.0) }
    }
}
```

- [ ] **Step 4: Run system tests**

Run:

```bash
cargo test -p soul-ecs --test system
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add crates/soul-ecs
git commit -m "feat: add typed systems"
```

## Task 6: 文档、格式化和全量验证

**Files:**
- Create: `docs/ecs/overview.md`
- Create: `docs/ecs/safety.md`
- Modify: `README.md`

- [ ] **Step 1: Write docs**

Create `docs/ecs/overview.md`:

```markdown
# soul ECS 概览

`soul-ecs` 是基于 flecs C API 的 Rust ECS 门面。公开 API 参考 flecs C++ 的 typed builder 风格，底层由 `soul-ecs-sys` 编译 vendored flecs v4.1.5。

## crate 边界

- `soul-ecs-sys`：低层 FFI 和 C shim，不提供安全抽象。
- `soul-ecs`：safe Rust API，隐藏 flecs 指针和 `unsafe`。

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
```

Create `docs/ecs/safety.md`:

```markdown
# soul ECS 安全边界

第一版只支持 `Copy + 'static` component。原因是当前实现通过 flecs C API 的按字节 set/get 路径存储组件，没有为 Rust 析构、移动和复制语义注册 flecs lifecycle hooks。

`unsafe` 被限制在以下边界：

- `soul-ecs-sys` 的 FFI 声明。
- `soul-ecs-sys/src/shim.c` 对 flecs descriptor 和 iterator 字段的访问。
- `soul-ecs` 内部把 flecs 字段指针转换为 Rust 引用的代码。
- system callback trampoline 对 boxed closure 指针的恢复与释放。

公开 API 不要求调用方写 `unsafe`。后续支持非 `Copy` component 前，必须先设计 lifecycle hooks，并增加析构、复制和移动行为测试。
```

Update `README.md`:

```markdown
# soul

`soul` 当前包含一个基于 flecs C API 的 Rust ECS 绑定设计。

文档：

- [ECS 概览](docs/ecs/overview.md)
- [ECS 安全边界](docs/ecs/safety.md)
```

- [ ] **Step 2: Run formatting**

Run:

```bash
cargo fmt --all
```

Expected: command exits 0.

- [ ] **Step 3: Run all tests**

Run:

```bash
cargo test --workspace
```

Expected: all tests pass.

- [ ] **Step 4: Run docs build**

Run:

```bash
cargo doc --workspace --no-deps
```

Expected: docs build succeeds.

- [ ] **Step 5: Commit**

Run:

```bash
git add README.md docs/ecs crates Cargo.toml
git commit -m "docs: document ecs binding"
```

## 自查

- 规格覆盖：计划覆盖 `soul-ecs-sys`、`soul-ecs`、world、entity、component、query、system、vendored flecs、中文文档和验收命令。
- 安全修正：计划把第一版 component bound 收紧为 `Copy + 'static`，避免对带析构 Rust 类型做 C 侧按字节复制。
- Context7 依据：flecs C API 文档确认了 `ecs_component_init`、`ecs_set_id`、`ecs_get_id`、`ecs_get_mut_id`、`ecs_query_init`、`ecs_query_iter`、`ecs_query_next`、`ecs_system_init` 和 `ecs_progress` 的使用路径。
- 无占位项：每个任务都给出目标文件、测试、命令和预期结果。
