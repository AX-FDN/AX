# AX LLVM AOT v0

> 本页记录 `axc build` 当前新增的 LLVM AOT 原型边界。它是后端启动点，不是发布级 native compiler 承诺。

## 当前定位

AX 的稳定执行路径仍然是解释器：

```powershell
axc run <file-or-project>
```

`axc build` 现在有两层职责：

- 始终导出稳定构建资产：`source.ax`、`program.hir.json`、`program.mir.json`、`build-manifest.json`
- 对一个非常小的 MIR 子集尝试生成文本 LLVM IR：`generated/main.ll`

这意味着 AOT 已经从“只有 readiness 规划”进入“可观察的 IR artifact v0”，但还没有变成成熟 native executable 输出链。

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
- `src/backend/llvm/toolchain.rs`

解释器位置仍然是 `src/interpreter.rs`，LLVM AOT v0 不替换解释器，也不把解释器逻辑复制成第二套语义实现。解释器继续作为 `axc run` 的语义参考路径。

## 当前支持的最小子集

LLVM IR v0 只支持足够小的单文件核心：

- `fn main() -> i32`
- 同文件普通函数
- `i32`
- `bool`
- `string` 局部变量 / 参数 / 返回值，当前表示为只读 C 字符串指针
- `string_len(text)` / `len(text)`，当前按 UTF-8 codepoint 数量返回 `i32`
- `string == string` / `string != string` 内容比较，当前通过 C ABI `strcmp` 完成
- `to_string(i32)` / `to_string(bool)` / `to_string(string)`
- `string + string`，当前通过 process-lifetime `malloc` 分配拼接结果，暂不回收
- 固定长度数组 v0：非空 array literal、局部变量、函数参数 by value、索引读取、`len(array)`；当前主要验证 `[i32; N]`
- local `let` / assignment
- `return`
- MIR 级 `goto` / `branch`
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
axc build examples/aot_bool_logic.ax
axc build examples/aot_comparisons.ax
axc build examples/aot_nested_calls.ax
axc build examples/aot_print.ax
axc build examples/aot_print_string.ax
axc build examples/aot_string_values.ax
axc build examples/aot_string_len_compare.ax
axc build examples/aot_string_runtime.ax
axc build examples/aot_array_read.ax
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
- `string_contains(...)` / `string_replace(...)` / `string_trim(...)` 等更完整的 string runtime helper
- `len(...)` 作用于 slice / string_list
- 更完整的通用 `string` ownership / allocation / free 规则
- 更完整的宿主 IO runtime ABI
- `f32`
- array element assignment / slices
- struct / enum / payload enum
- `match`
- `Result` / `Option`
- `?` 错误传播
- methods / impl / traits / generics 的 native lowering
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
- `examples/aot_bool_logic.ax`
- `examples/aot_comparisons.ax`
- `examples/aot_nested_calls.ax`
- `examples/aot_print.ax`
- `examples/aot_print_string.ax`
- `examples/aot_string_values.ax`
- `examples/aot_string_len_compare.ax`
- `examples/aot_string_runtime.ax`

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
6. 已完成 String Runtime v0：`to_string(i32/bool/string)` 与 `string + string` 可进入 AOT parity；当前拼接和 `to_string(i32)` 使用 process-lifetime `malloc`，暂不回收。
7. 已完成 Array Read v0：固定长度数组 literal / 局部变量 / 参数 by value / 索引读取 / `len(array)` 可进入 AOT parity；当前主要验证 `[i32; N]`，数组写入和 slice 仍未进入 native layout。
8. 下一步进入 struct layout / field read contract，再谈 enum、`match`、`Result`、`Option`。
9. 增加 multi-file/project linking contract，再谈本地包和标准库 AOT。
10. 等 Build-2 代表项目可 AOT 后，才考虑发布级 Build-3。

JIT 不应早于发布级 AOT 评估。当前最重要的是把 AOT 输入契约、IR artifact、toolchain blocker resolution 和解释器语义对照链先做稳。
