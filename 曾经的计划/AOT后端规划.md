# AX AOT 后端规划

最后更新：2026-05-01

本文只规划 AX 的 AOT 后端路线，不替代 [`PLAN.md`](./PLAN.md)、[`WORKLIST.md`](./WORKLIST.md) 和 [`docs/llvm-aot.md`](./docs/llvm-aot.md)。

它回答一个问题：

> AX 接下来如何从当前 LLVM IR v0，逐步走到可发布、可验证、能承载后端工具和未来服务端能力的 native build 路径。

## 1. 当前基线

当前 `axc build` 已经不是纯 skeleton，但也不是成熟 native compiler。

当前事实：

| 维度 | 状态 |
| --- | --- |
| 稳定执行路径 | `axc run` 仍然是解释器 |
| build 基础产物 | `source.ax`、`program.hir.json`、`program.mir.json`、`build-manifest.json` |
| AOT 原型 | LLVM AOT v0 已能为极小单文件 MIR 子集生成 `generated/main.ll` |
| manifest | `build-manifest.json` schema version `7`，`aot_readiness.schema_version = 2` |
| exe 链接 | 默认关闭，只有 `AX_LLVM_AOT_LINK=1` 时才尝试 clang |
| 当前样例 | [`examples/aot_return.ax`](./examples/aot_return.ax)、[`examples/aot_math.ax`](./examples/aot_math.ax)、[`examples/aot_control_flow.ax`](./examples/aot_control_flow.ax) |
| 当前后端代码 | [`src/backend/llvm/`](./src/backend/llvm/) |
| 当前边界文档 | [`docs/llvm-aot.md`](./docs/llvm-aot.md) |

当前 LLVM IR v0 支持：

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

当前不支持：

- `println`
- `string`
- `f32`
- arrays / slices
- struct / enum / payload enum
- `match`
- `Result` / `Option`
- `?`
- methods / impl / traits / generics 的 native lowering
- 多文件项目链接
- 本地 path package 链接
- registry package
- host extension ABI

## 2. AOT 的项目定位

AX 的 AOT 后端不是为了替换解释器，也不是为了证明 benchmark 概念，而是为了让 AX 从“能解释执行的 AI-first 工具语言”走向“能发布产物的后端可用语言”。

AOT 的职责：

- 把 `axc build` 从稳定 artifact 导出升级为真实交付路径。
- 让 AX 程序可以生成 native executable。
- 为后续 backend worker、CLI 工具发布、包生态和部分自举提供产物基础。
- 让语言能力不只停留在解释器里，而是进入可发布 runtime。

AOT 不承担的职责：

- 不负责重新定义 AX 语法。
- 不负责绕过 semantic check。
- 不负责直接桥接 Rust crate。
- 不负责提前启动 JIT。
- 不负责把不稳定语法硬编译成 native code。

## 3. 不可破坏的边界

### 3.1 解释器继续作为语义参考

`src/interpreter.rs` 仍然是当前语义参考路径。

AOT 后端必须遵守：

- 不能为了 AOT 改坏 `axc run`。
- 不能复制一整套解释器语义后各自漂移。
- 每个 AOT 支持的新能力都要有 interpreter / AOT 对照测试。
- 如果解释器与 AOT 行为冲突，默认先认为 AOT 后端不完整。

### 3.2 MIR 是 AOT 的主要输入

AOT 后端应尽量从 MIR 读取程序结构。

允许读取 AST/HIR 的场景：

- manifest/readiness 统计
- debug metadata
- future source map
- diagnostic/context 补充信息

不允许：

- AOT 后端绕过 MIR 重新解释 AST。
- AOT 后端私自实现与 semantic 不一致的类型规则。
- AOT 后端为了方便而接收未通过 check 的程序。

### 3.3 每个 AOT 能力都必须进入契约

新增 AOT lowering 不算“完成”，除非同时补齐：

- LLVM IR 输出
- manifest 字段或 blocker 更新
- `aot_readiness` 更新
- 至少一个正向样例
- 至少一个不支持时的稳定 blocker 或 unsupported note
- interpreter / AOT 对照验证入口
- 文档更新

## 4. 总路线

AOT 路线分成七段：

| 阶段 | 名称 | 目标 |
| --- | --- | --- |
| `AOT-0` | Readiness Contract | 后端阻塞项可见 |
| `AOT-1` | LLVM IR Core | 极小单文件核心能生成 LLVM IR |
| `AOT-2` | Executable Core | 极小单文件核心能链接并运行 exe |
| `AOT-3` | Runtime ABI | `println`、string、基础 runtime 边界成立 |
| `AOT-4` | Data Layout | struct / enum / match / Result 进入 native layout |
| `AOT-5` | Project Linking | 多文件项目、std、local path package 可 AOT |
| `AOT-6` | Release Backend | AOT 成为公开 v1 交付路径 |

JIT 只能在 `AOT-6` 之后评估，不能抢 AOT 主线。

## 5. AOT-0：Readiness Contract

状态：已完成第一版。

已成立内容：

- `build-manifest.json` 输出 `aot_readiness`
- `context evidence` 输出 build/AOT readiness
- blocker 能说明当前程序为什么还不能 native build
- schema version 已升级到 `6`
- AOT v0 生成 IR 后会移除 `AOT0001` 并改用 toolchain blocker

关键 blocker：

| Code | 含义 |
| --- | --- |
| `AOT0001` | native executable emission 尚未实现 |
| `AOT0101` | project source graph native linking 未实现 |
| `AOT0102` | local path package native linking 未实现 |
| `AOT0103` | local package graph 需要 current `AX.lock` |
| `AOT0201` | 泛型 monomorphization / 类型参数 lowering 未冻结 |
| `AOT0202` | trait/interface lowering 未冻结 |
| `AOT0203` | method ABI 未冻结 |
| `AOT0204` | enum / pattern / match native layout 未冻结 |
| `AOT0205` | `?` 早返回 native lowering 未实现 |
| `AOT0301` | host boundary builtin 需要 native runtime ABI |
| `AOT0302` | string / string_list 需要 native runtime representation |
| `AOT1000` | LLVM IR 已生成，但链接被关闭 |
| `AOT1001` | 请求链接，但找不到 clang |
| `AOT1002` | 请求链接，clang 失败 |

每个 blocker 还必须带 `resolution`，至少说明：

- `agent_action`：AI 或外部工具下一步应该解释 unsupported、开启链接、配置 clang、检查 clang 失败，还是验证 lockfile。
- `source_edit_safe`：当前 blocker 是否适合让 AI 自动改 AX 源码。后端未支持和工具链问题默认都是 `false`。
- `recommended_command`：只有存在明确命令时才输出，例如 `AOT1000` 建议开启 `AX_LLVM_AOT_LINK`，`AOT0103` 建议 `axc lock <project> --check`。

退出标准：

- readiness 输出能覆盖语言、runtime、package、project 四类阻塞。
- 所有 blocker 都有明确 required stage。
- 外部文档不再把 build 误写成 mature native compiler。

## 6. AOT-1：LLVM IR Core

状态：已启动。

目标：

- 让单文件、无 runtime 依赖、无复杂数据布局的核心 MIR 能稳定生成 LLVM IR。
- 把 `generated/main.ll` 变成稳定 artifact。
- 不要求默认生成 exe。

当前已完成：

- `src/backend/llvm/` 后端目录
- `ir.rs` 文本 LLVM IR generator
- `toolchain.rs` 可选 clang linking
- `examples/aot_return.ax`
- `artifacts.llvm_ir`
- `backend.kind = "llvm-aot"`
- `backend.status = "ir_generated"`

下一步补强顺序：

1. 增加更多 core arithmetic/control-flow 样例。
2. 增加 interpreter / LLVM IR 语义对照脚本或测试 helper。
3. 明确 `main` 退出码契约：`fn main() -> i32` 的返回值就是进程退出码。
4. 增加 Linux 上的 LLVM IR artifact smoke。
5. 保持项目输入不进入 LLVM IR，直到 project linking contract 成立。

退出标准：

- 单文件 `i32/bool` 核心子集稳定生成 LLVM IR。
- 该子集有至少 `5` 个正向样例。
- 每个样例能通过 `check / run / build`。
- build manifest 快照覆盖 `ir_generated` 状态。
- unsupported case 不崩溃，只输出 readiness/blocker。

禁止事项：

- 不提前支持项目链接。
- 不为了 demo 把 string/println 写死进 IR。
- 不绕过 MIR 直接从 AST 生成 IR。

## 7. AOT-2：Executable Core

目标：

- 在安装 clang/LLVM toolchain 的机器上，让极小单文件核心生成真实 exe。
- 让 `AX_LLVM_AOT_LINK=1` 从实验开关变成可验证路径。

必须完成：

- Windows GNU 本地链接路径说明。
- Linux clang 链接路径说明。
- `bin/<target>` 或 `bin/<target>.exe` 真实存在。
- manifest 中 `artifacts.executable` 只在 exe 真实生成时出现。
- `backend.status = "built"` 只在链接成功时出现。
- exe 运行结果与 `axc run` 结果一致。

测试矩阵：

| 平台 | 必须验证 |
| --- | --- |
| Windows | `AX_LLVM_AOT_LINK=1` 能找到 clang 并生成 `.exe` |
| Linux | `AX_LLVM_AOT_LINK=1` 能生成无后缀 executable |
| CI | Ubuntu job 安装 `clang` 后运行 `scripts/smoke-aot-link.ps1`，验证 `examples/aot_return.ax` 能生成并运行 executable；Windows 链接验证等目标三元组和 MSVC/MinGW 策略稳定后再常驻 |

退出标准：

- `examples/aot_return.ax` 能生成并运行 exe。
- 至少 `3` 个单文件核心样例能生成并运行 exe。
- `axc run` 与 AOT exe 结果一致。
- toolchain missing/failure 都有稳定 `AOT1001/AOT1002` 输出。

禁止事项：

- 不把 clang 作为默认强依赖。
- 不让缺 clang 破坏普通 `axc build`。
- 不在没有 runtime ABI 的情况下承诺真实工具项目 AOT。

## 8. AOT-3：Runtime ABI

目标：

- 让最基础的真实工具能力进入 native runtime。
- 先补 `println` 和 `string`，再考虑更宽 host boundary。

必须先定义：

- AX `string` 在 native runtime 中如何表示。
- 字符串所有权和释放责任。
- `println` 的 ABI。
- runtime helper 的链接方式。
- Windows/Linux 下 runtime object 或 runtime library 的放置方式。

建议实现顺序：

1. `println(i32)` / `println(bool)` 最小 stdio ABI。
2. string literal 的只读全局数据表示。
3. `println(string)`。
4. string concat。
5. `len(string)`。
6. `to_string(i32/bool)`。

退出标准：

- `examples/hello.ax` 能 AOT。
- 至少一个字符串处理单文件样例能 AOT。
- runtime helper 链接失败有稳定 blocker。
- `AOT0301/AOT0302` 能随能力完成逐步消失。

禁止事项：

- 不先做完整 GC。
- 不先做复杂 string_list。
- 不先做 fs/process/env host boundary。

## 9. AOT-4：Data Layout

目标：

- 让 AX 的数据结构有明确 native layout。
- 先 struct，再 enum，再 match，再 Result/Option。

执行顺序：

1. struct layout
2. struct literal / field read
3. enum unit variant representation
4. payload enum representation
5. match lowering
6. `Option<T>` native shape
7. `Result<T,E>` native shape
8. `?` early-return lowering

每一步必须同步：

- MIR lowering 对应检查
- LLVM IR lowering
- interpreter / AOT 对照
- `aot_readiness` blocker 更新
- docs / feature matrix 更新
- 至少一个样例

退出标准：

- payload enum + match 能 AOT。
- `Result.Ok/Err` 能 AOT。
- `?` 能 AOT。
- 至少一个当前 project-backed 逻辑可被压缩为单文件 AOT smoke。

禁止事项：

- 不先做泛型 monomorphization。
- 不先做 trait dispatch。
- 不先做 async。

## 10. AOT-5：Project Linking

目标：

- 让 AOT 从单文件进入多文件项目和标准库。
- 让 `AX.toml + sources` 不只是解释器路径，也能成为 build 路径。

必须定义：

- module symbol 命名规则。
- source unit 到 LLVM module 的映射。
- 单 module IR 还是多 module IR。
- cross-module function call ABI。
- `std.*` 源码如何被纳入 AOT 输入。
- local path package 如何参与 symbol naming 和链接。
- `AX.lock` 对 AOT package graph 的约束。

执行顺序：

1. 多 source 单项目 AOT，不含 dependency。
2. `std.*` 源码模块 AOT。
3. local path package AOT。
4. `AX.lock --check` 成为 package AOT 前置校验。
5. build manifest 输出 project/package linking metadata。

退出标准：

- 至少 `3` 个代表项目样例可 AOT。
- 至少 `1` 个使用 `std.*` 的项目可 AOT。
- 至少 `1` 个 local path package 项目可 AOT。
- `package_graph_readiness.aot_ready` 可以在满足条件时变成 `true`。

优先候选项目：

- `examples/project_text_normalize/`
- `examples/project_directory_index/`
- `examples/project_config_validate/`
- `examples/project_job_runner/`

禁止事项：

- 不支持 `AX import -> Cargo crate` 直通。
- 不跳过 `AX.lock` 直接做包链接。
- 不先做 registry。

## 11. AOT-6：Release Backend

目标：

- AOT 成为 AX v1 的正式发布路径。
- `axc build` 不再被描述为 skeleton 或 prototype。

必须完成：

- Windows executable output。
- Linux executable output。
- macOS 至少在 core support 启动后评估。
- release artifact 目录结构冻结。
- build manifest executable contract 冻结。
- interpreter / AOT parity suite。
- runtime ABI 文档。
- package AOT 文档。
- AOT troubleshooting 文档。

退出标准：

- `axc build <project>` 生成可运行 executable。
- 代表项目能在 Windows/Linux 上 AOT。
- CI 至少覆盖 IR generation；toolchain 稳定后覆盖 executable smoke。
- README 可以明确写“AX 支持 AOT build”，但仍需说明支持范围。

禁止事项：

- 不在 Build-2 前宣传发布级 AOT。
- 不在没有 parity suite 前做性能宣传。
- 不把 AOT 说成比成熟语言更快，除非有公开 benchmark。

## 12. JIT 启动门槛

JIT 当前不启动。

只有同时满足下面条件，才允许进入 JIT-Eval：

- `AOT-6` 完成。
- AOT 已成为稳定 public path。
- 已有 interpreter / AOT 语义一致性回归。
- 已证明 compile latency 是真实瓶颈。
- 已有明确场景说明 JIT 比 AOT 更有价值。

JIT 即使启动，也只作为实验或开发路径，不抢主发布路径。

## 13. 与语法扩张的关系

后续新增语法必须考虑 AOT，但不要求所有语法一开始就 AOT。

规则：

- 解释器可以先支持新语法。
- AOT 可以先输出 blocker。
- 但不能让 AOT silently miscompile。
- 任何语法进入“后端可发布能力”前，都必须有 native lowering contract。

语法进入 AOT 的建议顺序：

| 语法/能力 | AOT 阶段 |
| --- | --- |
| `i32/bool` 基础表达式 | `AOT-1` |
| 函数调用 / return / branch | `AOT-1` |
| exe 链接与退出码 | `AOT-2` |
| `println` / string | `AOT-3` |
| struct | `AOT-4` |
| enum / payload enum | `AOT-4` |
| match | `AOT-4` |
| Result / Option | `AOT-4` |
| `?` | `AOT-4` |
| module/import | `AOT-5` |
| `std.*` | `AOT-5` |
| local path package | `AOT-5` |
| generics monomorphization | `AOT-5` 或之后 |
| methods / impl ABI | `AOT-5` 或之后 |
| trait dispatch | `AOT-6` 或之后 |
| closures | AOT release path 稳定后 |
| async | AOT、runtime、package、错误模型稳定后 |

## 14. 与标准库和包系统的关系

标准库和包系统不能等 AOT 全完成才设计，但它们进入 AOT 必须分层。

顺序：

1. `std.*` 先作为 AX 源码接口在解释器路径稳定。
2. AOT 先支持不依赖 host boundary 的 std helper。
3. 再支持 string/report/path 这类 runtime 需求较小的 std helper。
4. 再支持 fs/env/process 这类 host boundary helper。
5. local path package 先要求 `AX.lock --check` current。
6. registry package 等 AOT project linking 稳定后再进入。

原则：

- 用户依赖 AX package，不直接依赖 Rust crate。
- Rust/native runtime 只能藏在 AX runtime ABI 或 host extension ABI 后。
- 包接口进入 AOT 前必须有 package graph readiness。

## 15. 与 AI-first 护城河的关系

AOT 不能削弱 AX 的 AI-first 特征。

每个 AOT 阶段都必须保留：

- structured diagnostics
- `aot_readiness`
- context evidence
- stable blocker codes
- repair-friendly messages
- reproducible build artifacts

AOT 的 AI-facing 价值：

- agent 可以知道哪些语法阻塞 native build。
- agent 可以根据 blocker 精准改代码，而不是猜后端失败原因。
- context evidence 可以告诉 agent 应该跑 `check / run / build / lock` 哪些命令。
- build manifest 可以成为后续 repair benchmark 的证据输入。

## 16. 验证矩阵

每个 AOT 阶段至少需要下面验证：

| 验证 | 说明 |
| --- | --- |
| `cargo test --lib` | 后端单元测试、IR 生成测试 |
| `interface_snapshots` | manifest、CLI、代表样例契约 |
| manual build smoke | `axc build examples/aot_return.ax` |
| artifact check | `generated/main.ll` 存在且内容稳定 |
| linker-off check | 没有 clang 也不失败 |
| linker-on check | 有 clang 时生成 executable |
| run parity | `axc run` 与 AOT exe 结果一致 |
| unsupported check | 不支持能力输出 blocker，不 silent fail |

当前推荐本地命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 fmt --check
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --lib
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots
git diff --check
```

## 17. 下一批实际施工项

下一批不应该直接冲泛型、trait dispatch 或 async，而应该把 AOT v0 做成可靠工程路径。

优先级：

1. `AOT-1.1` 增加 `examples/aot_control_flow.ax`
   - 覆盖 `if / else`、比较、bool branch。

2. `AOT-1.2` 增加 `examples/aot_math.ax`
   - 覆盖 `+ - * / %`、一元 `-`、函数调用。

3. `AOT-1.3` 增加 AOT parity helper
   - 对支持子集比较 `axc run` 和 AOT exe/IR 预期。

4. `AOT-2.1` 安装和记录 clang 工具链路径
   - Ubuntu CI 已安装 `clang` 并运行 `scripts/smoke-aot-link.ps1`；Windows 不强制，但要补 MSVC/MinGW 目标策略说明。

5. `AOT-2.2` 链接成功时写入 executable snapshot
   - 先可选，等 CI toolchain 稳定后常驻。

6. `AOT-3.1` 设计 runtime ABI 文档
   - 先写清 `println`、string literal、string ownership。

7. `AOT-3.2` 实现最小 stdio ABI
   - 目标是让 `examples/hello.ax` 能 AOT。

## 18. 当前不做

当前不做：

- JIT
- async lowering
- closure lowering
- 泛型 trait AOT
- trait object / dynamic dispatch
- registry package AOT
- host extension ABI
- Rust crate 直通导入
- 完整 GC
- 性能 benchmark 宣传

这些都不是永远不做，而是必须等 AOT 的单文件、runtime、layout、project linking 四个基础层站稳后再做。

## 19. 一句话收口

AX 的 AOT 路线应该是：

> 先让最小 MIR 子集稳定生成 LLVM IR，再让 exe 链接可复现；随后补 runtime ABI、数据布局、多文件项目和包链接；最后才把 AOT 宣传成正式发布路径，并在那之后评估 JIT。

当前最重要的不是“马上编译所有语法”，而是避免 silent miscompile，保持解释器语义参考、manifest/readiness 透明、unsupported blocker 稳定，让 AOT 每前进一步都能被 agent、CI 和用户看懂。
