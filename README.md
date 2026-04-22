# AX

`PLAN.md` 是 AX 项目的唯一设计基线。

- `README.md` 负责说明当前仓库已经实现并可直接实践的 AX 原型。
- `SYNTAX.md` 负责给人和 AI 提供更完整的当前语法参考与 EBNF。
- 如需改语言方向、路线或边界，请先更新 `PLAN.md`，再改代码与文档。

## 当前状态

当前仓库已经可以实践第一版 AX 原型语法。

- `axc check <file>`：执行词法、语法、基础语义与类型检查
- `axc ast <file>`：输出稳定 AST JSON
- `axc run <file>`：通过最小解释器执行 AX 程序
- `axc fmt <file>`：命令入口已保留，格式化器尚未实现

当前最小可运行子集已经支持：

- 顶层：`fn`、`struct`、`enum`
- 语句：`let`、`let mut`、变量赋值、结构体字段赋值、表达式语句、`return`、`if / else`、`while`
- 表达式：整数、浮点、布尔、字符串、变量引用、一元运算、二元运算、函数调用、结构体字面量、字段访问、枚举值引用
- 解释器：`main`、局部变量、函数调用、递归、算术/比较、条件、循环、内置 `println`、结构体、枚举值

使用时请记住这几个硬约束：

- `main` 必须是 `fn main() -> i32`
- `main` 的返回值就是进程退出码；成功时建议返回 `0`
- 枚举值写法固定为 `EnumName.Variant`
- 结构体字段写入当前只支持直接形式：`point.x = expr;`
- `let`、赋值、表达式语句、`return` 都必须带分号

完整语法说明请看 [`SYNTAX.md`](/C:/Users/xiaoy/Desktop/A语言/AX/SYNTAX.md)。

## 快速开始

先跑检查：

```powershell
cargo run -- check examples\hello.ax
cargo run -- check examples\syntax_overview.ax
```

查看 AST：

```powershell
cargo run -- ast examples\syntax_overview.ax
```

执行示例：

```powershell
cargo run -- run examples\hello.ax
cargo run -- run examples\factorial.ax
cargo run -- run examples\syntax_overview.ax
```

## 推荐阅读顺序

- 想了解项目边界与阶段路线：看 [`PLAN.md`](/C:/Users/xiaoy/Desktop/A语言/AX/PLAN.md)
- 想按当前仓库真实语法写代码：看 [`SYNTAX.md`](/C:/Users/xiaoy/Desktop/A语言/AX/SYNTAX.md)
- 想直接照着例子练：看 [`examples/hello.ax`](/C:/Users/xiaoy/Desktop/A语言/AX/examples/hello.ax)、[`examples/factorial.ax`](/C:/Users/xiaoy/Desktop/A语言/AX/examples/factorial.ax)、[`examples/syntax_overview.ax`](/C:/Users/xiaoy/Desktop/A语言/AX/examples/syntax_overview.ax)

## 给 AI 的最小规则

如果你把 README 直接喂给模型，请让它严格遵守下面这些规则：

1. 只使用当前仓库已实现的原型语法，不要发明 `for`、`match`、数组、模块、泛型、异常、async。
2. 所有函数参数、返回类型、局部变量都必须显式标注类型。
3. `main` 必须写成 `fn main() -> i32 { ... }`。
4. 枚举值必须写成 `EnumName.Variant`。
5. 构造结构体必须写成 `TypeName { field: expr, ... }`。
6. 结构体字段写入只生成直接形式：`point.x = expr;`，其中 `point` 必须是 `let mut` 声明的结构体变量。
7. `println` 是当前唯一内置函数。
8. 若不确定某个语法是否支持，就不要使用；优先生成更朴素、更显式的代码。

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
