# AX LLVM AOT v0

> 本页记录 `axc build` 当前新增的 LLVM AOT 原型边界。它是后端启动点，不是发布级 native compiler 承诺。

## 当前定位

AX 的稳定执行路径仍然是解释器：

```powershell
axc run <file-or-project>
```

`axc build` 现在有两层职责：

- 始终导出稳定构建资产：`source.ax`、`program.hir.json`、`program.mir.json`、`build-manifest.json`
- 对持续扩展的单文件 MIR 子集尝试生成文本 LLVM IR：`generated/main.ll`

这意味着 AOT 已经从“只有 readiness 规划”进入“可观察、可链接、可和解释器对照的 IR artifact v0”，但还没有变成成熟 native executable 输出链。

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
- `src/backend/llvm/ir.rs`
- `src/backend/llvm/runtime.rs`
- `src/backend/llvm/toolchain.rs`

解释器位置仍然是 `src/interpreter.rs`，LLVM AOT v0 不替换解释器，也不把解释器逻辑复制成第二套语义实现。解释器继续作为 `axc run` 的语义参考路径。

`runtime.rs` 是 AOT runtime ABI 的收口点：当前负责内建 format/text globals、libc/clang 可见的外部声明，以及 `ax_string_len` / `ax_string_concat` / `ax_i32_to_string` 这类文本 IR helper。后续补 `string_trim`、`string_replace`、host runtime、slice/string_list runtime 时，都优先在这里扩 ABI，不把 runtime 细节散落回 lowering 主流程。

## 当前支持的最小子集

LLVM IR v0 先支持单文件核心子集，并按 parity 样例逐包扩展：

- `fn main() -> i32`
- 同文件普通函数
- `i32`
- `bool`
- `string` 局部变量 / 参数 / 返回值，当前表示为只读 C 字符串指针
- `string_len(text)` / `len(text)`，当前按 UTF-8 codepoint 数量返回 `i32`
- `string == string` / `string != string` 内容比较，当前通过 C ABI `strcmp` 完成
- `to_string(i32)` / `to_string(bool)` / `to_string(string)`
- `string_contains(text, needle)` / `string_starts_with(text, prefix)` / `string_ends_with(text, suffix)` 字符串谓词 v0
- `string + string`，当前通过 process-lifetime `malloc` 分配拼接结果，暂不回收
- 固定长度数组 v0：非空 array literal、局部变量、函数参数 by value、索引读取、元素写入、`len(array)`；当前主要验证 `[i32; N]`
- 只读 Slice v0：固定数组可借出 `{ ptr, i32 len }` slice，支持 `values[start:end]` 半开区间、`len(slice)`、`slice[index]` 读取、同文件 slice 参数调用，并支撑 `for in` over fixed array；slice 写入、host/runtime slice 来源和跨项目 slice ABI 仍未进入完整 native contract
- Struct v0：非泛型 struct 定义、struct literal、局部变量、函数参数 by value、返回值、字段读取和字段写入
- Unit Enum v0：非泛型无 payload enum、variant 常量、局部变量、函数参数 by value、返回值、`==` / `!=` tag 比较和语句形态 unit enum `match` 判断
- Payload Enum v0：非泛型 payload enum 以 `{ i32 tag, ptr payload }` lower，支持 payload constructor、payload read、函数参数 / 返回值和语句形态 payload enum `match`
- Match Expression v0：支持表达式形态 `match`、简单 binding pattern、payload binding 和 block-valued arm
- Range Pattern v0：支持 `i32` inclusive range pattern，例如 `200..=299`，当前 lower 成 `icmp sge` / `icmp sle` / `and`
- Concrete Generic Enum Instance v0：支持同文件非泛型函数内的 `Option<i32>` 与 `Result<i32,string>` 具体 enum 实例，包含 constructor、参数、返回值和 `match` payload 读取；这不是 full generics / generic impl / std project linking 的完整 monomorphization
- Result Try v0：支持同文件非泛型函数内 `Result<T,E>` 形状的 `expr?`，Ok 分支解包 payload 继续执行，Err 分支重新构造当前函数返回类型的 `Err(E)` 并 early return；当前重点验证具体 `Result<i32,string>` / `Result<string,string>` 实例
- local `let` / assignment
- top-level `const` v0：支持当前 AOT 类型子集内的 `i32/bool/string` 常量引用
- `return`
- MIR 级 `goto` / `branch`
- `for`、固定数组 `for in`、`break`、`continue`
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
axc build examples/aot_math.ax
axc build examples/aot_control_flow.ax
axc build examples/aot_loop.ax
axc build examples/for_in.ax
axc build examples/aot_bool_logic.ax
axc build examples/aot_comparisons.ax
axc build examples/aot_nested_calls.ax
axc build examples/aot_print.ax
axc build examples/aot_print_string.ax
axc build examples/aot_string_values.ax
axc build examples/aot_string_len_compare.ax
axc build examples/aot_string_runtime.ax
axc build examples/aot_array_read.ax
axc build examples/aot_array_write.ax
axc build examples/aot_struct_read.ax
axc build examples/aot_struct_write.ax
axc build examples/aot_enum_unit.ax
axc build examples/aot_enum_match.ax
axc build examples/aot_payload_enum.ax
axc build examples/aot_match_expression.ax
axc build examples/match_range.ax
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

- `to_string(...)` 作用于 array / slice / struct / enum / f32 等复杂值
- `string_replace(...)` / `string_trim(...)` / `string_split_lines(...)` 等更完整的 string runtime helper
- `len(...)` 作用于 string_list，或作用于尚未由 AOT 只读 slice v0 表达的复杂 slice 来源
- 更完整的通用 `string` ownership / allocation / free 规则
- 更完整的宿主 IO runtime ABI
- `f32`
- slice 写入、host/runtime slice 来源、跨项目 slice ABI 和完整 slice ownership / lifetime contract
- enum formatter / complex payload equality / binding-bearing or pattern lowering
- methods / impl / traits / full generics 的 native lowering
- `std.option` / `std.result` project import、静态 helper 方法和跨模块 native linking
- multi-file project linking
- local path package native linking
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

LLVM AOT v0 把 `build-manifest.json` 升级到 schema version `9`，并把 `aot_readiness.schema_version` 升级到 `3`。

新增或变化的字段重点：

- `backend.kind = "llvm-aot"`：当前输入进入 LLVM AOT v0 子集
- `backend.status = "ir_generated"`：已生成 LLVM IR，但未生成 exe
- `backend.status = "built"`：已生成 LLVM IR，并且 clang 链接成功
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

这条 smoke 是 G3 的核心验证入口。它会对同一批 AX core 样例依次执行 `axc check`、`axc run`、`axc build --json`、运行生成的 executable，并比较解释器与 executable 的 `exit code / stdout / stderr`。它默认覆盖：

- `examples/aot_return.ax`
- `examples/aot_math.ax`
- `examples/aot_control_flow.ax`
- `examples/aot_loop.ax`
- `examples/consts.ax`
- `examples/modulo.ax`
- `examples/for_loop.ax`
- `examples/break_loop.ax`
- `examples/continue.ax`
- `examples/for_in.ax`
- `examples/aot_bool_logic.ax`
- `examples/aot_comparisons.ax`
- `examples/aot_nested_calls.ax`
- `examples/aot_print.ax`
- `examples/aot_print_string.ax`
- `examples/aot_string_values.ax`
- `examples/aot_string_len_compare.ax`
- `examples/aot_string_runtime.ax`
- `examples/aot_array_read.ax`
- `examples/aot_array_write.ax`
- `examples/aot_struct_read.ax`
- `examples/aot_struct_write.ax`
- `examples/aot_enum_unit.ax`
- `examples/aot_enum_match.ax`
- `examples/aot_payload_enum.ax`
- `examples/aot_match_expression.ax`
- `examples/match_range.ax`
- `examples/aot_result_option.ax`
- `examples/aot_result_try.ax`

有 clang 的 CI 应优先跑 parity smoke，因为它不只证明“能链接 exe”，还证明当前 AOT executable 没有偏离解释器语义。Ubuntu CI 会安装 `clang` 后跑这条验证。没有 clang 的本机不要求这条通过；必须保证默认 IR-only 路径和缺 clang 的 `AOT1001` blocker 路径稳定。

关键测试：

- `backend::llvm::ir::tests::renders_minimal_main_return`
- `backend::llvm::ir::tests::renders_i32_function_calls_and_arithmetic`
- `backend::llvm::ir::tests::renders_i32_and_bool_println_calls`
- `backend::llvm::ir::tests::renders_string_literal_println_calls`
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
6. 已完成 String Runtime v0：`to_string(i32/bool/string)`、`string + string`、`string_contains`、`string_starts_with` 和 `string_ends_with` 可进入 AOT parity；当前拼接和 `to_string(i32)` 使用 process-lifetime `malloc`，暂不回收。
7. 已完成 Array + Read-only Slice v0：固定长度数组 literal / 局部变量 / 参数 by value / 索引读取 / 元素写入 / `len(array)` 可进入 AOT parity；固定数组可借出只读 `{ ptr, len }` slice，支持 `values[start:end]` / `len(slice)` / `slice[index]` / 同文件 slice 参数调用，并让 `for in` over fixed array 进入 parity；slice 写入、host/runtime slice 来源和跨项目 slice ABI 仍未进入完整 native layout。
8. 已完成 Struct v0：非泛型 struct 定义 / literal / 局部变量 / 参数 by value / 返回值 / 字段读取 / 字段写入可进入 AOT parity。
9. 已完成 Unit Enum v0：非泛型无 payload enum 以 `i32 tag` lower，支持 variant 值、参数、返回值、`== !=` 和语句形态 unit enum `match`。
10. 已完成 Payload Enum v0：非泛型 payload enum 以 `{ i32 tag, ptr payload }` lower，支持 constructor、payload read、参数 / 返回值和语句形态 payload enum `match`。
11. 已完成 Match Expression v0：表达式形态 `match`、简单 binding pattern、payload binding 和 block-valued arm 已进入 AOT parity。
12. 已完成 Concrete Generic Enum Instance v0：同文件非泛型函数内的 `Option<i32>` / `Result<i32,string>` constructor、match、参数和返回值可进入 AOT parity。
13. 已把 `%`、C-style `for`、固定数组 `for in`、`break`、`continue` 纳入默认 executable parity，不再只是“能生成 IR”的隐性能力。
14. 已完成 top-level Const v0：MIR 保留 const initializer，LLVM AOT 可在函数中内联 lower 当前 AOT 类型子集内的 `i32/bool/string` const 引用。
15. 已完成 Range Pattern v0：`i32` inclusive range pattern 已 lower 成 native 比较链，并验证 `examples/match_range.ax`。
16. 已完成 Result Try v0：同文件非泛型函数内的 `expr?` 可 lower 成 Ok 解包与 Err early return，并验证 `Result<i32,string>` 传播到 `Result<string,string>` 的错误重包路径。
17. 已完成 Or Pattern v0：无绑定 alternative 的 `A | B` pattern 已 lower 成 native boolean 合并，并验证 `examples/match_or.ax`；带绑定的 or pattern 仍需要 binding merge semantics，当前保持 blocker。
18. 已完成 Match Guard v0：guarded arm 的 bool guard 已进入 native branch lowering，pattern binding / payload binding 会先绑定再计算 guard，guard 为 false 时继续尝试后续 arm，并验证 `examples/match_guard.ax`。
19. 已把 String Pattern v0 纳入默认 parity：`match` 中的 string literal pattern 使用 native `strcmp` 比较，并验证 `examples/string_match.ax`。
20. 已完成 Payload Enum Equality v0：payload enum 的 `== !=` 先比较 tag，再比较当前 AOT 可比较的 `i32/bool/string` payload，并验证 `examples/aot_payload_enum_equality.ax`。
21. 已完成 Struct Pattern v0：非泛型 struct 的全字段 shorthand 解构 pattern 可 lower 成 native field extract，并验证 `examples/match_struct_pattern.ax`。
22. 下一步补 enum formatter、复杂 payload equality、partial/nested struct destructuring、带绑定 or pattern、完整 slice layout 和 runtime ABI。
23. 增加 multi-file/project linking contract，再谈本地包和标准库 AOT。
24. 等 Build-2 代表项目可 AOT 后，才考虑发布级 Build-3。

JIT 不应早于发布级 AOT 评估。当前最重要的是把 AOT 输入契约、IR artifact、toolchain blocker resolution 和解释器语义对照链先做稳。
