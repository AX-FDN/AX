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
- local `let` / assignment
- `return`
- MIR 级 `goto` / `branch`
- 一元 `-` / `!`
- `+ - * / %`
- `== != < <= > >=`
- `&& ||`
- 同文件直接函数调用

代表样例：

```powershell
axc build examples/aot_return.ax
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

下面这些内容仍然由 `aot_readiness` 或 LLVM AOT v0 的 unsupported notes 暴露，不会被假装成已支持：

- `println` 和宿主 IO runtime ABI
- `string`
- `f32`
- arrays / slices
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

LLVM AOT v0 把 `build-manifest.json` 升级到 schema version `6`。

新增或变化的字段重点：

- `backend.kind = "llvm-aot"`：当前输入进入 LLVM AOT v0 子集
- `backend.status = "ir_generated"`：已生成 LLVM IR，但未生成 exe
- `backend.status = "built"`：已生成 LLVM IR，并且 clang 链接成功
- `artifacts.llvm_ir = "generated/main.ll"`：文本 LLVM IR artifact
- `artifacts.executable`：只有链接成功时才出现
- `aot_readiness.stage = "Build-1 LLVM IR prototype"`：当前进入 Build-1 原型阶段
- `aot_readiness.status = "ir_generated"` 或 `"built"`

工具链 blocker：

| Code | Meaning |
| --- | --- |
| `AOT1000` | LLVM IR 已生成，但链接未开启 |
| `AOT1001` | 请求链接，但找不到 clang |
| `AOT1002` | 请求链接，clang 执行失败 |

## 验证入口

当前回归覆盖：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --lib
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

关键测试：

- `backend::llvm::ir::tests::renders_minimal_main_return`
- `backend::llvm::ir::tests::renders_i32_function_calls_and_arithmetic`
- `llvm_aot_return_build_emits_ir_artifact_without_linking_by_default`

手动检查：

```powershell
axc build examples/aot_return.ax
Get-Content build/aot_return/generated/main.ll
Get-Content build/aot_return/build-manifest.json
```

## 下一步顺序

LLVM AOT 后续不能靠“照着解释器抄”推进，而要按契约补齐：

1. 继续把单文件 `i32/bool` 子集跑稳，并对比 `axc run` 语义。
2. 增加最小 host stdio ABI，决定 `println` 如何进入 native runtime。
3. 增加 string runtime representation，否则真实工具样例无法 AOT。
4. 增加 struct/enum layout contract，再谈 `match`、`Result`、`Option`。
5. 增加 multi-file/project linking contract，再谈本地包和标准库 AOT。
6. 等 Build-2 代表项目可 AOT 后，才考虑发布级 Build-3。

JIT 不应早于发布级 AOT 评估。当前最重要的是把 AOT 输入契约、IR artifact、toolchain blocker 和解释器语义对照链先做稳。
