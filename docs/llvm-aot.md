# AX LLVM AOT v0

> 本页记录 `axc build` 当前新增的 LLVM AOT 原型边界。它是后端启动点，不是发布级 native compiler 承诺。

## 当前定位

AX 的稳定执行路径仍然是解释器：

```powershell
axc run <file-or-project>
```

`axc build` 现在有两层职责：

- 始终导出稳定构建资产：`source.ax`、`program.hir.json`、`program.mir.json`、`build-manifest.json`
- 对持续扩展的 MIR 子集尝试生成文本 LLVM IR：`generated/main.ll`，当前已包含单入口、多文件、`module/import`、多组 `std.*` project、host runtime v0 和 local path package `AX.toml` project-backed AOT

这意味着 AOT 已经从“只有 readiness 规划”进入“可观察、可链接、可和解释器对照的 IR artifact v0”。它已经能在当前子集内生成 native executable，但仍不是成熟发布级 native backend。

## 生成路径

当前路径是：

```text
AX source
  -> parser / semantic
  -> HIR
  -> MIR
  -> src/backend/llvm/
  -> generated/main.ll
  -> clang linking, only when explicitly enabled
```

后端代码位置：

- `src/backend/mod.rs`
- `src/backend/llvm/mod.rs`
- `src/backend/llvm/abi.rs`
- `src/backend/llvm/ir.rs`
- `src/backend/llvm/symbols.rs`
- `src/backend/llvm/monomorph.rs`
- `src/backend/llvm/linking.rs`
- `src/backend/llvm/runtime/mod.rs`
- `src/backend/llvm/runtime/core.rs`
- `src/backend/llvm/runtime/string.rs`
- `src/backend/llvm/runtime/list.rs`
- `src/backend/llvm/runtime/fs.rs`
- `src/backend/llvm/runtime/path.rs`
- `src/backend/llvm/runtime/process.rs`
- `src/backend/llvm/toolchain.rs`

解释器位置仍然是 `src/interpreter.rs`，LLVM AOT v0 不替换解释器，也不把解释器逻辑复制成第二套语义实现。解释器继续作为 `axc run` 的语义参考路径。

`src/backend/llvm/abi.rs` 和 [`aot-native-abi.md`](./aot-native-abi.md) 是 AX Native ABI v1 的收口点：当前固定 `string = ptr`、`slice = { ptr, i32 }`、`string_list = opaque ptr`，并把 process-lifetime allocation v0 明确成当前内存策略。`symbols.rs` 收口用户函数、方法、静态方法、泛型实例和 runtime helper 的 symbol 生成入口，第一轮保持 `main` / `ax_<sanitized_name>` 兼容输出。

`runtime/` 是 AOT runtime ABI 的实现分区：`core.rs` 负责内建 format/text globals、libc/clang 可见的外部声明和 `ax_runtime_error` 最小 stderr 错误出口；`string.rs`、`list.rs`、`fs.rs`、`path.rs`、`process.rs` 分别承载 string、string_list、filesystem、path、process helper。`runtime/mod.rs` 对外仍保留原来的 `write_*` 入口，调用方不需要感知拆分。后续补 host runtime、slice runtime 与更完整 std/native package runtime 时，都优先在这里扩 ABI，不把 runtime 细节散落回 lowering 主流程。

`monomorph.rs` 现在提供 `MonomorphizationPlan`，集中记录当前 AOT 使用到的 reachable functions 和 concrete generic instances；`linking.rs` 现在提供 `NativeLinkPlan`，把当前单个 `generated/main.ll` 链接 executable 的行为计划化。当前 CLI、manifest schema 和 IR 符号兼容性保持不变。

## 当前支持的最小子集

LLVM IR v0 先支持核心子集，并按 parity 样例逐包扩展：

- `fn main() -> i32`
- Project-backed AOT：当前默认 parity 清单包含 26 个 `AX.toml` project 样例，覆盖单入口、多文件、`module/import`、`std.option/std.result`、`std.collections`、`std.env`、`std.fs`、`std.path`、`std.process`、真实工具型 project 和 local path package project；完整清单由 [`scripts/smoke-aot-parity.ps1`](../scripts/smoke-aot-parity.ps1) 维护
- 同文件普通函数
- `i32`，包含 negation/add/sub/mul/div/rem overflow、除零和取余零的最小 runtime error guard
- `bool`
- `f32` core v0：字面量、局部变量、函数参数 / 返回值、`+ - * /`、一元 `-`、`== != < <= > >=`、`println(f32)` 和 `to_string(f32)`
- `string` 局部变量 / 参数 / 返回值，当前表示为只读 C 字符串指针
- `string_len(text)` / `len(text)`，当前按 UTF-8 codepoint 数量返回 `i32`
- `string == string` / `string != string` 内容比较，当前通过 C ABI `strcmp` 完成
- `to_string(i32)` / `to_string(bool)` / `to_string(f32)` / `to_string(string)` / `to_string(array)` / `to_string(slice)` / `to_string(struct)` / `to_string(enum)`
- `string_contains(text, needle)` / `string_starts_with(text, prefix)` / `string_ends_with(text, suffix)` 字符串谓词 v0
- `string_replace(text, from, to)` 全量替换 v0，包含 `from == ""` 时按 UTF-8 字符边界插入的解释器一致语义
- `string_split_lines(text)` LF / CRLF 行切分 v0，返回只读 `[string]` slice，可配合 `len(lines)`、`lines[i]` 和 `for in` 遍历
- `string_trim(text)` ASCII whitespace v0，当前分配返回新字符串，使用 process-lifetime `malloc`，暂不回收
- `string_list_new()` / `string_list_push(items, value)` / `string_list_get(items, index)` / `string_list_join(items, separator)` / `len(items)`，当前以 opaque native list pointer 表达，使用 process-lifetime `malloc`，暂不回收
- `string + string`，当前通过 process-lifetime `malloc` 分配拼接结果，暂不回收
- `argv_len()` / `argv_get(index)` v0：native `main(argc, argv)` 会把 CLI 参数暴露给 AX，`argv_get(0)` 对应用户传入的第一个参数
- `env_has(name)` / `env_get(name)` v0：当前通过 C ABI `getenv` 读取宿主环境变量；`env_get` 缺失时输出 `R0053` runtime error，`std.env.try_get` 可通过 `env_has` 避免失败路径
- `process_cwd()` / `process_run(command)` / `process_run_in(dir, command)` / `process_capture(command)` / `process_capture_in(dir, command)` host process ABI v0：当前通过 C ABI `system`、`popen/_popen`、`pclose/_pclose` 和 cwd 切换 helper 实现；`capture` 非零退出会进入 native runtime error，适合先覆盖短 stdout 命令和 `std.process` run/status 封装
- 固定长度数组 v0：非空 array literal、显式零长度 array literal（例如 `let values: [i32; 0] = []`）、局部变量、函数参数 by value、索引读取、元素写入、`len(array)`、`to_string(array)`、直接 `println(array)` 与 element-wise `==` / `!=`；当前主要验证 `[i32; N]` / `[bool; N]` / `[string; N]`
- Slice v0：固定数组可形成 `{ ptr, i32 len }` slice，支持 `values[start:end]` 半开区间、`len(slice)`、`slice[index]` 读取、mutable slice element assignment、`to_string(slice)`、直接 `println(slice)`、同文件 slice 参数调用和 element-wise `==` / `!=`，并支撑 `for in` over fixed array 与 `values[start:end]` slice range 直接遍历；`string_split_lines(text)` 返回的 `[string]` slice 也已支持 `len(lines)`、`lines[i]` 和 `for in`；当前 range slice 是 copy-backed value，除 `string_split_lines` 外的 host/runtime slice 来源和跨项目 slice ABI 仍未进入完整 native contract
- Struct v0：非泛型 struct 定义、struct literal、局部变量、函数参数 by value、返回值、字段读取、字段写入、字段级 `==` / `!=`、`to_string(struct)` 与直接 `println(struct)`
- Unit Enum v0：非泛型无 payload enum、variant 常量、局部变量、函数参数 by value、返回值、`==` / `!=` tag 比较和语句形态 unit enum `match` 判断
- Payload Enum v0：非泛型 payload enum 以 `{ i32 tag, ptr payload }` lower，支持 payload constructor、payload read、函数参数 / 返回值和语句形态 payload enum `match`
- Payload Enum Equality v0：payload enum 的 `==` / `!=` 会先比较 tag，再对 `i32/bool/f32/string`、固定数组、struct 和 slice payload 做 native equality；不同 variant 直接按不相等处理
- Enum Formatter v0：`to_string(enum)` 和直接 `println(enum)` 支持 unit variant、`i32/bool/string` payload variant 以及固定数组 / struct / slice payload variant，格式与解释器的 `Enum.Variant(...)` display 保持一致
- Match Expression v0：支持表达式形态 `match`、简单 binding pattern、payload binding 和 block-valued arm
- Range Pattern v0：支持 `i32` inclusive range pattern，例如 `200..=299`，当前 lower 成 `icmp sge` / `icmp sle` / `and`
- Concrete Generic Enum Instance v0：支持同文件非泛型函数内的 `Option<i32>` 与 `Result<i32,string>` 具体 enum 实例，包含 constructor、参数、返回值、`match` payload 读取、`to_string(...)` 和直接 `println(...)`；这不是 full generics / generic impl / std project linking 的完整 monomorphization
- Result Static Constructor / Try v0：支持同文件 `Result<T,E>` 形状的 `Result.ok(...)` / `Result.err(...)` 静态构造器从 `let` / `return` / `match` 上下文推断缺失类型参数；`expr?` 会 lower 成 Ok payload 继续执行和 Err(E) early return；当前重点验证具体 `Result<i32,string>` / `Result<string,string>` 实例
- local `let` / assignment
- top-level `const` v0：支持当前 AOT 类型子集内的 `i32/bool/string` 常量引用
- `return`
- MIR 级 `goto` / `branch`
- `for`、固定数组 / slice range / runtime `[string]` slice `for in`、`break`、`continue`
- 一元 `-` / `!`
- `+ - * / %`
- `== != < <= > >=`
- `&& ||`
- 同文件直接函数调用
- `println(i32)` / `println(bool)`，当前通过 libc `printf` 完成最小 stdout ABI
- 只读 string literal 直接传给 `println`，例如 `println("hello")`

代表样例：

```powershell
axc build examples/aot_return.ax
axc build examples/project_hello
axc build examples/project_split
axc build examples/project_collections_core
axc build examples/project_env_result
axc build examples/aot_math.ax
axc build examples/aot_control_flow.ax
axc build examples/aot_loop.ax
axc build examples/consts.ax
axc build examples/modulo.ax
axc build examples/for_loop.ax
axc build examples/break_loop.ax
axc build examples/continue.ax
axc build examples/for_in.ax
axc build examples/aot_slice_range.ax
axc build examples/aot_slice_for_in.ax
axc build examples/aot_slice_to_string.ax
axc build examples/aot_slice_equality.ax
axc build examples/aot_bool_logic.ax
axc build examples/aot_comparisons.ax
axc build examples/aot_f32_core.ax
axc build examples/aot_nested_calls.ax
axc build examples/aot_print.ax
axc build examples/aot_print_string.ax
axc build examples/aot_string_values.ax
axc build examples/aot_string_len_compare.ax
axc build examples/aot_string_runtime.ax
axc build examples/aot_string_predicates.ax
axc build examples/aot_string_replace.ax
axc build examples/aot_string_split_lines.ax
axc build examples/aot_string_split_lines_for_in.ax
axc build examples/aot_string_trim.ax
axc build examples/string_list.ax
axc build examples/aot_process_runtime.ax
axc build examples/aot_argv.ax
axc build examples/string_match.ax
axc build examples/aot_array_read.ax
axc build examples/aot_array_write.ax
axc build examples/aot_array_to_string.ax
axc build examples/aot_array_equality.ax
axc build examples/empty_array.ax
axc build examples/aot_struct_read.ax
axc build examples/aot_struct_write.ax
axc build examples/aot_struct_to_string.ax
axc build examples/aot_struct_equality.ax
axc build examples/match_struct_pattern.ax
axc build examples/aot_enum_unit.ax
axc build examples/aot_enum_match.ax
axc build examples/aot_payload_enum.ax
axc build examples/aot_payload_enum_equality.ax
axc build examples/aot_enum_to_string.ax
axc build examples/aot_enum_print.ax
axc build examples/aot_enum_array_payload.ax
axc build examples/aot_enum_array_payload_equality.ax
axc build examples/aot_enum_struct_slice_payload.ax
axc build examples/aot_enum_slice_payload_equality.ax
axc build examples/aot_generic_enum_print.ax
axc build examples/aot_match_expression.ax
axc build examples/match_range.ax
axc build examples/match_or.ax
axc build examples/match_guard.ax
axc build examples/aot_result_option.ax
axc build examples/aot_result_try.ax
```

生成：

```text
build/aot_return/
  source.ax
  program.hir.json
  program.mir.json
  generated/main.ll
  build-manifest.json
```

## 当前不支持的内容

下面这些内容仍然由 `aot_readiness.blockers` 和 LLVM AOT v0 的 unsupported notes 暴露，不会被假装成已支持：

- `to_string(string_list)` 或直接 `println(string_list)` 这类尚未具备 native formatter 的值
- 除 `string_split_lines` 外的 host/runtime slice 来源、跨项目 slice ABI 和更完整的 slice ownership / lifetime contract
- `len(...)` 作用于尚未由 AOT slice v0 表达的复杂 slice 来源
- 更完整的通用 `string` ownership / allocation / free 规则
- 更完整的宿主 IO runtime ABI；当前 fs/path/process/env 已有 v0，但不是完整 host extension ABI
- 更深层组合 enum payload formatter / 更深层组合 payload equality / binding-bearing or pattern lowering
- 跨项目 methods / impl / traits / generics native linking，以及完整 project/std monomorphization
- 更完整 `std`/native package monomorphization 与 helper native linking
- 更完整 local path package native linking（当前已有 project-backed 第一批 parity，不等于完整跨包 ABI 冻结）
- registry package
- host extension ABI

这些能力不是不能做，而是必须先给出 native layout、runtime ABI、monomorphization、package linking 或 host boundary contract，不能直接照抄解释器行为。

## 可执行文件链接

默认情况下，`axc build` 只生成 LLVM IR，不尝试链接 exe。这样做是为了保证没有 LLVM/clang 的机器也能稳定运行 `build`、快照和 CI。

显式开启链接：

```powershell
$env:AX_LLVM_AOT_LINK = "1"
axc build examples/aot_return.ax
```

指定 clang：

```powershell
$env:AX_LLVM_AOT_LINK = "1"
$env:AX_LLVM_CLANG = "C:\path\to\clang.exe"
axc build examples/aot_return.ax
```

有效的 `AX_LLVM_AOT_LINK` 值：

- `1`
- `true`
- `yes`
- `on`

如果没有开启链接，manifest 会出现：

```json
{
  "backend": {
    "kind": "llvm-aot",
    "status": "ir_generated"
  },
  "artifacts": {
    "llvm_ir": "generated/main.ll"
  }
}
```

同时 `aot_readiness.blockers` 会包含 `AOT1000`，说明 IR 已生成，但 executable linking 被显式关闭。

## Build Manifest 契约

LLVM AOT v0 把 `build-manifest.json` 升级到 schema version `10`，并把 `aot_readiness.schema_version` 升级到 `3`。

新增或变化的字段重点：

- `backend.kind = "llvm-aot"`：当前输入进入 LLVM AOT v0 子集
- `backend.status = "ir_generated"`：已生成 LLVM IR，但未生成 exe
- `backend.status = "built"`：已生成 LLVM IR，并且 clang 链接成功
- `requested_emit`：记录本次 `axc build` 请求的产物模式，当前为 `default`、`ir`、`exe` 或 `all`
- `user_code_valid = true`：`axc build` 已经通过 frontend check；如果前端失败，build 不会写 manifest
- `interpreter_supported = true`：当前输入仍以解释器语义作为参考路径
- `aot_supported`：只有 native executable 已生成、没有 AOT blocker 时才为 `true`
- `artifacts.llvm_ir = "generated/main.ll"`：文本 LLVM IR artifact
- `artifacts.executable`：只有链接成功时才出现
- `aot_readiness.stage = "Build-1 LLVM IR prototype"`：当前进入 Build-1 原型阶段
- `aot_readiness.status = "ir_generated"` 或 `"built"`
- `aot_readiness.blockers[].resolution`：给工具链说明下一步是解释 unsupported、开启链接、配置 clang，还是检查 toolchain failure
- `aot_readiness.blockers[].ai`：给 AI 明确 `rule_id / layer / ai_action / safe_to_edit / repair_goal / validation`，避免把合法源码的 AOT 能力缺口误修成业务代码变化

LLVM lowering blocker：

| Code | Meaning | Layer | `ai.rule_id` | `ai.ai_action` |
| --- | --- | --- | --- | --- |
| `AOT2001` | 当前 MIR 进入 LLVM AOT v0，但 IR lowering 子集不支持这些 feature | `llvm_lowering` | `aot_llvm_lowering_unsupported` | `explain_unsupported` |

工具链 blocker：

| Code | Meaning | Layer | `ai.rule_id` | `ai.ai_action` |
| --- | --- | --- | --- | --- |
| `AOT1000` | LLVM IR 已生成，但链接未开启 | `toolchain_link` | `aot_linking_must_be_enabled` | `enable_linking` |
| `AOT1001` | 请求链接，但找不到 clang | `toolchain_link` | `aot_clang_toolchain_required` | `configure_toolchain` |
| `AOT1002` | 请求链接，clang 执行失败 | `toolchain_link` | `aot_clang_link_failed` | `inspect_toolchain_failure` |

## 验证入口

当前回归覆盖：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --lib
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

LLVM executable 链接 smoke：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-aot-link.ps1
```

这条 smoke 要求本机能找到 `clang`，会显式设置 `AX_LLVM_AOT_LINK=1`，构建 `examples/aot_return.ax`，要求 manifest 进入 `built` 状态，并运行生成的 executable，验证退出码是 `42`。它适合作为单样例链接检查；Ubuntu CI 的常驻 AOT 链路使用下面的 parity smoke。没有 `clang` 的本地机器仍然可以只跑默认 IR artifact 验证。

LLVM AOT parity smoke：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-aot-parity.ps1
```

这条 smoke 是 G3 的核心验证入口。它会对同一批 AX core/project 样例依次执行 `axc check`、`axc run`、`axc build --json`、运行生成的 executable，并比较解释器与 executable 的 `exit code / stdout / stderr`。它默认覆盖 `123` 个样例，其中 `97` 个是单文件/直接样例，`26` 个是 `AX.toml` project 样例；仓库内全部 project 示例都已列入默认清单。

默认清单由 [`scripts/smoke-aot-parity.ps1`](../scripts/smoke-aot-parity.ps1) 维护，不在本文复制长列表，避免文档和脚本漂移。覆盖范围包括 core/control-flow/consts/f32-core/stdout/string/string-runtime/string-list/std-collections/std-env/std-fs/std-path/std-process/argv/array/slice/struct/enum/match/concrete generic enum/Result/project-backed/local path package。部分 side-effect-heavy project 仍可能通过受控失败路径做 parity，后续会通过 per-backend fixture isolation 继续把它们收口到成功路径。

有 clang 的 CI 应优先跑 parity smoke，因为它不只证明“能链接 exe”，还证明当前 AOT executable 没有偏离解释器语义。Ubuntu CI 会安装 `clang` 后跑这条验证。没有 clang 的本机不要求这条通过；必须保证默认 IR-only 路径和缺 clang 的 `AOT1001` blocker 路径稳定。

关键测试：

- `backend::llvm::ir::tests::renders_minimal_main_return`
- `backend::llvm::ir::tests::renders_i32_function_calls_and_arithmetic`
- `backend::llvm::ir::tests::renders_i32_and_bool_println_calls`
- `backend::llvm::ir::tests::renders_string_literal_println_calls`
- `backend::llvm::ir::tests::renders_enum_to_string_formatter_v0`
- `backend::llvm::ir::tests::renders_enum_array_payload_formatter_v0`
- `backend::llvm::ir::tests::renders_enum_struct_and_slice_payload_formatter_v0`
- `backend::llvm::ir::tests::renders_fixed_array_formatter_and_direct_print_v0`
- `backend::llvm::ir::tests::renders_slice_formatter_and_direct_print_v0`
- `backend::llvm::ir::tests::renders_struct_formatter_and_direct_print_v0`
- `llvm_aot_return_build_emits_ir_artifact_without_linking_by_default`
- `llvm_aot_core_examples_check_run_and_emit_ir_without_linking_by_default`
- `llvm_aot_link_reports_missing_clang_as_readiness_blocker`

手动检查：

```powershell
axc build examples/aot_return.ax
Get-Content build/aot_return/generated/main.ll
Get-Content build/aot_return/build-manifest.json
axc build examples/aot_return.ax --json
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-aot-parity.ps1
```

`axc build --json` 打印的对象必须和落盘的 `build-manifest.json` 是同一个 manifest 契约，不另起 summary 协议。

## 下一步顺序

LLVM AOT 后续不能靠“照着解释器抄”推进，而要按契约补齐：

1. 已完成第一批单文件 `i32/bool` 子集 parity：返回值、算术、控制流、循环、比较、逻辑、同文件调用都能对比 `axc run`。
2. 已完成最小 host stdio ABI 第一刀：`println(i32)` / `println(bool)` 可进入 native stdout，并由 parity smoke 比较 stdout。
3. 已完成 string literal 的只读全局表示，并支持直接 `println("...")`。
4. 已完成最小 string value representation：局部变量 / 参数 / 返回值都可按只读 C 字符串指针进入 AOT。
5. 已完成无 allocator 的 string helper 第一刀：`string_len` / `len(string)` 与字符串内容 `==` / `!=` 可进入 AOT parity。
6. 已完成 String Runtime v0：`to_string(i32/bool/string/array/slice/struct/enum)`、直接 `println(array/slice/struct/enum)`、`string + string`、`string_contains`、`string_starts_with`、`string_ends_with`、`string_replace` 全量替换 v0、`string_split_lines` LF/CRLF 行切分 v0、`string_trim` ASCII whitespace v0，以及 `string_list_new/push/get/join` 与 `len(string_list)` 可进入 AOT parity；当前拼接、formatter、`string_replace`、`string_split_lines`、`string_trim`、`string_list` 和 `to_string(i32)` 使用 process-lifetime `malloc`，暂不回收。
7. 已完成 CLI argv v0：native `main(argc, argv)` 会记录宿主参数，`argv_len()` 与 `argv_get(index)` 可进入 AOT；`examples/aot_argv.ax` 已验证无参数 parity，并手工验证带参数时解释器与 native exe 的输出 / 退出码一致。
8. 已完成 Host Env v0：`env_has/env_get` 通过 `getenv` 进入 native AOT，`std.env.try_get` 已在 `examples/project_env_result` 中通过 project-backed parity；缺失变量的裸 `env_get` 会走 `R0053` native runtime error。
9. 已完成 Array + Slice v0：固定长度数组 literal（含显式零长度 `[]`）/ 局部变量 / 参数 by value / 索引读取 / 元素写入 / `len(array)` / `to_string(array)` / 直接 `println(array)` 可进入 AOT parity；固定数组可形成 `{ ptr, len }` slice，支持 `values[start:end]` / `len(slice)` / `slice[index]` / mutable slice element assignment / `to_string(slice)` / 直接 `println(slice)` / 同文件 slice 参数调用，并让 `for in` over fixed array 与 slice range 直接遍历进入 parity；`string_split_lines(text)` 返回的 runtime `[string]` slice 也已支持 `len(lines)` / `lines[i]` / `for in`；除 `string_split_lines` 外的 host/runtime slice 来源和跨项目 slice ABI 仍未进入完整 native layout。
10. 已完成 Struct v0：非泛型 struct 定义 / literal / 局部变量 / 参数 by value / 返回值 / 字段读取 / 字段写入 / `to_string(struct)` / 直接 `println(struct)` 可进入 AOT parity。
11. 已完成 Unit Enum v0：非泛型无 payload enum 以 `i32 tag` lower，支持 variant 值、参数、返回值、`== !=` 和语句形态 unit enum `match`。
12. 已完成 Payload Enum v0：非泛型 payload enum 以 `{ i32 tag, ptr payload }` lower，支持 constructor、payload read、参数 / 返回值和语句形态 payload enum `match`。
13. 已完成 Match Expression v0：表达式形态 `match`、简单 binding pattern、payload binding 和 block-valued arm 已进入 AOT parity。
14. 已完成 Concrete Generic Enum Instance v0：同文件非泛型函数内的 `Option<i32>` / `Result<i32,string>` constructor、match、参数、返回值、formatter 和 direct print 可进入 AOT parity。
15. 已把 `%`、C-style `for`、固定数组 / slice range / runtime `[string]` slice `for in`、`break`、`continue` 纳入默认 executable parity，不再只是“能生成 IR”的隐性能力。
16. 已完成 top-level Const v0：MIR 保留 const initializer，LLVM AOT 可在函数中内联 lower 当前 AOT 类型子集内的 `i32/bool/string` const 引用。
17. 已完成 Range Pattern v0：`i32` inclusive range pattern 已 lower 成 native 比较链，并验证 `examples/match_range.ax`。
18. 已完成 Result Static Constructor / Try v0：同文件 `Result.ok(...)` / `Result.err(...)` 可从上下文推断缺失类型参数，`expr?` 可 lower 成 Ok 解包与 Err early return，并验证 `Result<i32,string>` 传播到 `Result<string,string>` 的错误重包路径。
19. 已完成 Or Pattern v0：无绑定 alternative 的 `A | B` pattern 已 lower 成 native boolean 合并，并验证 `examples/match_or.ax`；带绑定的 or pattern 仍需要 binding merge semantics，当前保持 blocker。
20. 已完成 Match Guard v0：guarded arm 的 bool guard 已进入 native branch lowering，pattern binding / payload binding 会先绑定再计算 guard，guard 为 false 时继续尝试后续 arm，并验证 `examples/match_guard.ax`。
21. 已把 String Pattern v0 纳入默认 parity：`match` 中的 string literal pattern 使用 native `strcmp` 比较，并验证 `examples/string_match.ax`。
22. 已完成 Payload Enum Equality v0：payload enum 的 `== !=` 先比较 tag，再比较当前 AOT 可比较的 `i32/bool/string` payload，并验证 `examples/aot_payload_enum_equality.ax`。
23. 已完成 Struct Pattern v0：非泛型 struct 的全字段 shorthand 解构 pattern 可 lower 成 native field extract，并验证 `examples/match_struct_pattern.ax`。
24. 已完成 Composite Formatter v0：固定数组和非泛型 struct 的 `to_string(...)` 与直接 `println(...)` 会按解释器显示格式生成文本，并验证 `examples/aot_array_to_string.ax` 与 `examples/aot_struct_to_string.ax`。
25. 已完成 Enum Formatter v0：`to_string(enum)` 与直接 `println(enum)` 可格式化 unit variant、`i32/bool/string` payload variant、固定数组 payload、struct payload、slice payload 和当前 AOT concrete generic enum 实例，并验证 `examples/aot_enum_to_string.ax`、`examples/aot_enum_print.ax`、`examples/aot_enum_array_payload.ax`、`examples/aot_enum_struct_slice_payload.ax` 与 `examples/aot_generic_enum_print.ax`。
26. 下一步补更深层组合 payload formatter、复杂 payload equality、partial/nested struct destructuring、带绑定 or pattern、完整 slice layout 和 runtime ABI。
27. 增加 multi-file/project linking contract，再谈本地包和标准库 AOT。
28. 等 Build-2 代表项目可 AOT 后，才考虑发布级 Build-3。

JIT 不应早于发布级 AOT 评估。当前最重要的是把 AOT 输入契约、IR artifact、toolchain blocker resolution 和解释器语义对照链先做稳。
