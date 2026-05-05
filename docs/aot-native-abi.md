# AX Native ABI v1

> 本文固定 AX LLVM AOT 当前 native 表示的第一版工程契约。它不是最终内存模型，也不是 GC/ownership 方案；它的目标是让 AOT runtime、IR lowering、package linking 和后续 std/native 支持沿同一套规则扩展。

## 定位

AX 当前仍以 `axc run` 解释执行作为语义参考，`axc build` 的 LLVM AOT 路径负责把已支持 MIR 子集 lower 成 textual LLVM IR，并在显式开启链接时通过 clang/lld 或系统链接器生成 native executable。

Native ABI v1 先服务三个目标：

- 让 runtime helper 的类型、命名和内存策略有固定入口。
- 让 AOT 不支持的能力能被归类为 `aot_readiness`、`monomorphization`、`runtime_abi`、`llvm_lowering`、`toolchain` 或 `internal_compiler_error`，避免误判成用户源码错误。
- 让后续补 `process`、local path package native linking、std native package、generics/impl/trait ABI 时不再把规则散落到 runtime 或 call lowering 里。

## Type Layout

当前 LLVM opaque pointer 模式下的 v1 规则：

| AX 类型 | Native 表示 | 说明 |
| --- | --- | --- |
| `bool` | `i1` | 与现有 LLVM IR 输出保持一致 |
| `i32` | `i32` | 当前整数核心类型 |
| `f32` | `float` | 当前浮点核心类型 |
| `string` | `ptr` | NUL-terminated UTF-8；当前指向 process-lifetime allocation 或静态 string literal |
| `[T]` slice | `{ ptr, i32 }` | `ptr` 指向连续元素区域，`i32` 是元素数量 |
| `string_list` | `ptr` | opaque native list pointer，内部布局只属于 runtime helper |
| fixed array | `[N x T]` | 由 LLVM value/layout 直接表示 |
| struct | LLVM struct type | 继续沿用现有 `%ax_struct_* = type { ... }` layout |
| enum without payload | `i32` | tag-only enum |
| enum with payload | `{ i32, ptr }` | tag + opaque payload pointer |
| `Option<T>` / `Result<T,E>` | enum layout | 继续通过 concrete enum instance layout lower |

## Memory Policy

当前统一写死为 `process_lifetime_malloc_v0`：

- string concat/replace/split/trim/to_string 等 runtime helper 会分配新字符串。
- `string_list` 会分配 list header 和 data buffer。
- payload enum 需要 boxed payload 时使用 process-lifetime allocation。
- v1 不引入 `free`、allocator 参数、引用计数、GC 或 ownership lowering。

这不是最终方案，但它是现在最稳定的 AOT parity 基线。后续如果引入 allocator/ownership，必须先扩展 ABI 文档和 runtime strategy，再迁移 lowering。

## Runtime Helper Boundary

AOT runtime helper 现在按目录分区：

| 文件 | 责任 |
| --- | --- |
| `src/backend/llvm/runtime/core.rs` | format globals、runtime error、C ABI declarations |
| `src/backend/llvm/runtime/string.rs` | string length、concat、replace、split lines、trim、to_string |
| `src/backend/llvm/runtime/list.rs` | `string_list` opaque runtime |
| `src/backend/llvm/runtime/fs.rs` | filesystem ABI helpers |
| `src/backend/llvm/runtime/path.rs` | path ABI helpers |
| `src/backend/llvm/runtime/process.rs` | process cwd/run/capture ABI helpers |
| `src/backend/llvm/runtime/mod.rs` | 对外保留 `write_*` 入口，调用方不感知拆分 |

`src/backend/llvm/abi.rs` 收口 LLVM ABI 常量和 helper；`src/backend/llvm/symbols.rs` 收口用户函数、方法、静态方法、泛型实例和 runtime helper 的 symbol 生成入口。

## Symbols

第一轮保持现有 IR 符号兼容：

- `main` 仍输出为 `@main`。
- 普通用户函数仍输出为 `@ax_<sanitized_name>`。
- 方法、静态方法和泛型实例先保留兼容 API，不强行改写历史 IR 符号。
- 新增 module/package-aware mangling API，但当前不主动重命名已有样例符号。

这保证了本轮是后端工程化升级，不是用户可见 ABI 符号迁移。

## Lowering Diagnostics

内部新增 `AotLoweringDiagnostic`，先把 unsupported/lowering reason 结构化为：

| 字段 | 含义 |
| --- | --- |
| `layer` | `aot_readiness`、`monomorphization`、`runtime_abi`、`llvm_lowering`、`toolchain`、`internal_compiler_error` |
| `code` | 内部 AOT 分层码，例如 `AOT2001` |
| `feature` | 相关 feature/type，可为空 |
| `message` | 保持当前用户可见文本 |

当前 `build-manifest.json` schema 不变；内部先结构化，再格式化成原 notes/blocker 文本，为后续 manifest 分层升级铺路。

## Monomorphization And Linking

`src/backend/llvm/monomorph.rs` 现在提供 `MonomorphizationPlan`，集中记录当前 AOT 用到的 reachable functions 和 concrete generic instances。第一版复用既有 specialization 逻辑，不宣称 full generics。

`src/backend/llvm/linking.rs` 现在提供 `NativeLinkPlan`，把 “单个 `generated/main.ll` -> executable” 包装成计划对象。当前 CLI 行为不变，仍由 `--emit ir/exe/all`、`--no-link` 和 `AX_LLVM_AOT_LINK` 控制是否调用 clang；后续 obj/package/std native linking 会沿这个入口扩展。
