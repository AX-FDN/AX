# AX

`PLAN.md` 是当前唯一主文档与唯一设计基线。

- 代码实现放在 `AX/` 目录下推进。
- 如需补充路线、规范或决策，请先更新当前目录下的 `PLAN.md`。
- 不再在 `docs/` 维护并行 Markdown 设计文档。

## Prototype Status

当前仓库已经可以实践第一版 AX 原型语法。

- `axc check <file>`: 词法、语法、基础语义与类型检查
- `axc ast <file>`: 输出稳定 AST JSON
- `axc run <file>`: 通过最小解释器执行 AX 程序

当前最小可运行子集包含:

- 顶层: `fn`、`struct`、`enum`
- 语句: `let` / `let mut`、赋值、表达式语句、`return`、`if / else`、`while`
- 表达式: 整数、浮点、布尔、字符串、变量引用、一元运算、二元运算、函数调用、字段访问
- 解释器已支持: `main`、局部变量、算术/比较、条件、循环、函数调用、内置 `println`

注意:

- `main` 必须是 `fn main() -> i32`
- `main` 的返回值就是进程退出码；日常练习建议成功时返回 `0`
- `struct` / `enum` 当前已经能被解析和检查，但最小解释器还没有实现构造与字段执行

## Quick Start

先跑测试:

```powershell
cargo test
```

检查语义:

```powershell
cargo run -- check examples\hello.ax
```

查看 AST:

```powershell
cargo run -- ast examples\hello.ax
```

执行程序:

```powershell
cargo run -- run examples\hello.ax
cargo run -- run examples\factorial.ax
```

## Sample

```ax
fn fact(n: i32) -> i32 {
    if (n == 0) {
        return 1;
    } else {
        return n * fact(n - 1);
    }
}

fn main() -> i32 {
    println(fact(5));
    return 0;
}
```
