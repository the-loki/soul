# soul ECS 安全边界

第一版只支持 `Copy + 'static` 数据 component。原因是当前实现通过 flecs C API 的按字节 set/get 路径存储组件，没有为 Rust 析构、移动和复制语义注册 flecs lifecycle hooks。

`Entity::add<T>()` 只支持零大小 tag component。带数据的 component 必须使用 `set`，这样才能按已注册的 component 尺寸把值写入 flecs。

组件访问使用运行时借用守卫维护 Rust 侧别名规则：共享读取可以重入；共享读取期间的可变访问、可变访问期间的共享读取、以及重复可变访问都会 panic。

system callback 的 Rust panic 会在 C 回调边界被捕获并存入 `World`，随后从 `World::progress` 恢复 panic。清理回调中的 panic 无法交还给 Rust 调用方，因此会 abort，以保持 C ABI no-unwind 边界。

`unsafe` 被限制在以下边界：

- `soul-ecs-sys` 的 FFI 声明。
- `soul-ecs-sys/src/shim.c` 对 flecs descriptor 和 iterator 字段的访问。
- `soul-ecs` 内部把 flecs 字段指针转换为 Rust 引用的代码。
- system callback trampoline 对 boxed closure 指针的恢复与释放。

公开 API 不要求调用方写 `unsafe`。后续支持非 `Copy` component 前，必须先设计 lifecycle hooks，并增加析构、复制和移动行为测试。
