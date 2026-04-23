# AX

`PLAN.md` 是 AX 项目的唯一设计基线。

- `README.md` 负责说明当前仓库已经实现并可直接实践的 AX 原型。
- `SYNTAX.md` 负责给人和 AI 提供更完整的当前语法参考与 EBNF。
- 如需改语言方向、路线或边界，请先更新 `PLAN.md`，再改代码与文档。

## 当前状态

当前仓库已经可以实践第一版 AX 原型语法。

- `axc check <path>`：执行词法、语法、基础语义与类型检查，`<path>` 可以是 `.ax` 文件、项目目录或 `AX.toml`
- `axc check <path> --json --ai`：输出带规则卡、修复目标与上下文切片的 AI 增强诊断
- `axc ast <file>`：输出稳定 AST JSON
- `axc hir <file>`：输出稳定 HIR JSON
- `axc run <path>`：通过最小解释器执行 AX 程序
- `axc run <path> --json`：在运行失败时输出结构化 JSON diagnostics
- `axc run <path> --json --ai`：在命中已注册运行时规则时附加 AI 增强诊断
- `axc fmt <path>`：按唯一官方风格原地格式化当前 AX 原型代码

当前最小可运行子集已经支持：

- 顶层：`fn`、`struct`、`enum`
- 语句：`let`、`let mut`、变量赋值、结构体字段赋值、表达式语句、`return`、`if / else`、`while`、`for`
- 表达式：整数、浮点、布尔、字符串、变量引用、一元运算、二元运算、函数调用、结构体字面量、数组字面量、字段访问、索引读取、枚举值引用
- 解释器：`main`、局部变量、函数调用、递归、算术/比较、条件、循环、内置 `println`、结构体、固定长度数组、枚举值

使用时请记住这几个硬约束：

- `main` 必须是 `fn main() -> i32`
- `main` 的返回值就是进程退出码；成功时建议返回 `0`
- 枚举值写法固定为 `EnumName.Variant`
- 固定长度数组类型写作 `[Type; N]`，字面量写作 `[expr, ...]`，当前支持读取 `values[index]`
- 固定长度数组当前也支持直接元素写入：`values[index] = expr;`，前提是数组变量用 `let mut` 声明
- 结构体字段写入当前只支持直接形式：`point.x = expr;`
- `for` 当前使用 C 风格表头：`for (init; condition; step) { ... }`
- `let`、赋值、表达式语句、`return` 都必须带分号

完整语法说明请看 [`SYNTAX.md`](./SYNTAX.md)。

## 快速开始

先跑检查：

```powershell
cargo run -- check examples\hello.ax
cargo run -- check examples\arrays.ax
cargo run -- check examples\syntax_overview.ax
cargo run -- check examples\missing_semicolon.ax --json --ai
cargo run -- check examples\project_hello
```

查看 AST：

```powershell
cargo run -- ast examples\syntax_overview.ax
```

查看 HIR：
```powershell
cargo run -- hir examples\syntax_overview.ax
```

格式化文件：

```powershell
cargo run -- fmt examples\syntax_overview.ax
```

执行示例：

```powershell
cargo run -- run examples\hello.ax
cargo run -- run examples\arrays.ax
cargo run -- run examples\factorial.ax
cargo run -- run examples\for_loop.ax
cargo run -- run examples\syntax_overview.ax
cargo run -- run examples\project_hello
cargo run -- run examples\index_out_of_bounds.ax --json
cargo run -- run examples\index_out_of_bounds.ax --json --ai
```

最小项目 manifest 也已经接入了，当前采用单包、单入口形态：

```toml
manifest_version = 1

[package]
name = "project_hello"
entry = "src/main.ax"
```

项目目录示例见 [`examples/project_hello`](./examples/project_hello)。对这种目录可以直接运行：

```powershell
cargo run -- check examples\project_hello
cargo run -- run examples\project_hello
cargo run -- build examples\project_hello
```

如果当前 Windows 环境没有可用的 MSVC `link.exe`，请直接使用仓库内的 GNU 启动脚本：

```powershell
.\scripts\cargo-gnu.ps1 test
.\scripts\cargo-gnu.ps1 run -- check examples\syntax_overview.ax
.\scripts\cargo-gnu.ps1 run -- check examples\missing_semicolon.ax --json --ai
.\scripts\cargo-gnu.ps1 run -- hir examples\syntax_overview.ax
.\scripts\cargo-gnu.ps1 run -- run examples\arrays.ax
.\scripts\cargo-gnu.ps1 run -- run examples\for_loop.ax
.\scripts\cargo-gnu.ps1 run -- run examples\syntax_overview.ax
.\scripts\cargo-gnu.ps1 run -- run examples\index_out_of_bounds.ax --json
.\scripts\cargo-gnu.ps1 run -- run examples\index_out_of_bounds.ax --json --ai
```

这个脚本会自动切到 `stable-x86_64-pc-windows-gnu`，并接好 Rust 自带的 GNU linker。若本机还没装该工具链，先执行：

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal -c rustfmt
```

如果你想把 Cargo 构建产物放到别的盘位，比如 `D:`，可以把 [`.cargo/config.example.toml`](./.cargo/config.example.toml) 复制为本机自己的 `.cargo/config.toml`。这个本地文件已经加入 `.gitignore`，不会被提交到 GitHub。

如果你希望在一次 AI 修复会话里逐步升级教学层级，可以配合 `--ai-session <path>` 使用，例如：

```powershell
cargo run -- check examples\missing_semicolon.ax --json --ai --ai-session .ax-ai-session.demo.json
```

默认的 `axc check` 仍然走基础快路径，不会自动做 AI 上下文拼装；只有显式传入 `--json --ai` 时才会启用增强诊断。

如果你想快速比较基础诊断和 AI 增强诊断的开销，可以直接运行：

```powershell
.\scripts\benchmark-diagnostics.ps1 -Iterations 10
```

这个脚本会先构建 `axc`，然后按 [`benchmarks/repair-cases.json`](./benchmarks/repair-cases.json) 里的稳定坏例子，分别测量 `check`、`check --json`、`check --json --ai` 三种模式。

如果你想把这些坏例子一次性导出成“源码 + 基础 JSON diagnostics + AI 增强 JSON diagnostics”的成对工件，直接运行：

```powershell
.\scripts\export-repair-benchmark.ps1
```

默认输出会写到 `.ax-ai\repair-benchmark\<timestamp>\`，里面会包含每个 case 的源码副本、两份诊断结果、两份 provider-neutral repair prompt、两份结构化 bundle，以及一个总索引 `index.json`，方便后续喂给 Codex、Claude Code 或你自己的 benchmark 自动化。

如果你已经拿到一批修复结果，想批量验证它们是否真正通过 AX 检查，可以运行：

```powershell
.\scripts\score-repair-benchmark.ps1 -CandidatesDir .ax-ai\repair-candidates\demo
```

评分脚本默认会读取最近一次导出的 repair benchmark；候选修复文件可以放成两种形式之一：

- `.ax-ai\repair-candidates\demo\<case-id>.ax`
- `.ax-ai\repair-candidates\demo\<case-id>\repaired.ax`

评分结果会写到 `.ax-ai\repair-results\<timestamp>\`，并生成总汇总 `summary.json`。

如果你想把“导出 prompt -> 调用修复器 -> 评分”串成一次运行，可以直接用：

```powershell
.\scripts\run-repair-benchmark.ps1 -RunnerScript .\scripts\replay-repair-adapter.ps1 -RunnerExtraArgs @('-SourceDir', '.ax-ai\repair-candidates\smoke')
```

这个命令会：

- 读取最近一次 repair benchmark 导出，或在没有导出时自动先导出一份
- 逐个 case 调用你提供的 runner script
- 把候选修复写到新的 run 目录下
- 自动调用 `score-repair-benchmark.ps1` 生成评分结果

当前 runner script 的最小契约是接收这些参数：

- `-PromptPath`
- `-BundlePath`
- `-OutputPath`
- `-CaseId`
- `-FeedbackMode`

仓库里自带的 [`replay-repair-adapter.ps1`](./scripts/replay-repair-adapter.ps1) 只是一个回放适配器，适合 smoke test 或重放已有候选结果；后面如果你要接 `Codex`、`Claude Code` 或别的模型 CLI，直接按同样参数签名写一个新的 adapter 就行。

仓库现在也自带了一个真实可调用的 [`codex-repair-adapter.ps1`](./scripts/codex-repair-adapter.ps1)。它会用 `codex exec` 的非交互模式读取 benchmark prompt，并通过 JSON schema 把最终输出约束成稳定的 `repaired_source` 字段。

如果本机已经能直接运行 `codex exec`，可以这样跑一轮真实修复：

```powershell
.\scripts\run-repair-benchmark.ps1 `
  -RunnerScript .\scripts\codex-repair-adapter.ps1 `
  -RunnerExtraArgs @('-Model', 'gpt-5.4') `
  -FeedbackMode ai
```

如果你想沿用本机 `codex` 默认模型，也可以不传 `-Model`：

```powershell
.\scripts\run-repair-benchmark.ps1 `
  -RunnerScript .\scripts\codex-repair-adapter.ps1 `
  -FeedbackMode ai
```

如果你想正式比较 `base diagnostics` 和 `ai diagnostics` 的修复提升，可以直接运行：

```powershell
.\scripts\compare-repair-feedback.ps1 `
  -RunnerScript .\scripts\codex-repair-adapter.ps1 `
  -RunnerExtraArgs @('-Model', 'gpt-5.4')
```

这个脚本会对同一份 repair benchmark 连续跑两轮：

- `base`：只给基础 diagnostics
- `ai`：给 AI 增强 diagnostics

输出会写到 `.ax-ai\repair-comparisons\<timestamp>\`，其中至少包含：

- `comparison.json`：机器可读的 lift 汇总、分类统计、逐 case 对照结果
- `comparison.md`：便于人直接阅读的实验报告
- `base\` / `ai\`：两轮独立 run 和 score 产物，方便回溯

这个 adapter 额外支持这些可选参数：

- `-CodexCommand <name>`：指定 `codex` 可执行文件名或路径
- `-Model <name>`：显式指定模型，例如 `gpt-5.4`
- `-Profile <name>`：复用 `~/.codex/config.toml` 里的 profile
- `-ConfigOverride @('key=value', ...)`：透传额外 `codex -c` 配置覆盖

仓库现在也带了一套稳定接口快照和 CI 烟雾验证：

- [`tests/interface_snapshots.rs`](./tests/interface_snapshots.rs) 会覆盖 `fmt` 幂等、`check --json`、`run --json`、`ast`、`hir`
- [`tests/snapshots`](./tests/snapshots) 保存当前 CLI 外部接口快照
- [`scripts/smoke-repair-benchmark.ps1`](./scripts/smoke-repair-benchmark.ps1) 会用仓库内的 replay candidates 跑最小 repair benchmark 闭环
- [`.github/workflows/ci.yml`](./.github/workflows/ci.yml) 会在 GitHub Actions 上执行 `cargo fmt --check`、Rust tests 和 repair benchmark smoke run

## 推荐阅读顺序

- 想了解项目边界与阶段路线：看 [`PLAN.md`](./PLAN.md)
- 想按当前仓库真实语法写代码：看 [`SYNTAX.md`](./SYNTAX.md)
- 想直接照着例子练：看 [`examples/hello.ax`](./examples/hello.ax)、[`examples/arrays.ax`](./examples/arrays.ax)、[`examples/factorial.ax`](./examples/factorial.ax)、[`examples/for_loop.ax`](./examples/for_loop.ax)、[`examples/syntax_overview.ax`](./examples/syntax_overview.ax)

## 给 AI 的最小规则

如果你把 README 直接喂给模型，请让它严格遵守下面这些规则：

1. 只使用当前仓库已实现的原型语法，不要发明 `match`、切片、空数组字面量、模块、泛型、异常、async。
2. 所有函数参数、返回类型、局部变量都必须显式标注类型。
3. `main` 必须写成 `fn main() -> i32 { ... }`。
4. 枚举值必须写成 `EnumName.Variant`。
5. 构造结构体必须写成 `TypeName { field: expr, ... }`。
6. 结构体字段写入只生成直接形式：`point.x = expr;`，其中 `point` 必须是 `let mut` 声明的结构体变量。
7. `for` 使用 `for (init; condition; step) { ... }`；推荐 `for (let mut i: i32 = 0; i < n; i = i + 1) { ... }`。
8. `println` 是当前唯一内置函数。
9. 若不确定某个语法是否支持，就不要使用；优先生成更朴素、更显式的代码。

## 最小示例

```ax
struct Point {
    x: i32,
    y: i32,
}

enum Flag {
    On,
    Off,
}

fn total(point: Point) -> i32 {
    return point.x + point.y;
}

fn main() -> i32 {
    let mut point: Point = Point { x: 2, y: 3 };
    point.x = point.x + 1;

    let flag: Flag = Flag.On;
    if (flag == Flag.On) {
        println("enabled");
    } else {
        println("disabled");
    }

    println(flag);
    println(total(point));
    return 0;
}
```
