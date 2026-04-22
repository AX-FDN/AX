# AX

`PLAN.md` 是当前唯一主文档与唯一设计基线。

- 代码实现放在 `AX/` 目录下推进。
- 如需调整路线、规范或决策，请先更新当前目录下的 `PLAN.md`。
- README 的职责是教会人和 AI 如何使用**当前已经实现的原型语法**，而不是提供第二套设计基线。

## 当前状态

当前仓库已经可以实践第一版 AX 原型语法。

- `axc check <file>`: 词法、语法、基础语义与类型检查
- `axc ast <file>`: 输出稳定 AST JSON
- `axc run <file>`: 通过最小解释器执行 AX 程序

当前最小可运行子集包含：

- 顶层：`fn`、`struct`、`enum`
- 语句：`let` / `let mut`、赋值、表达式语句、`return`、`if / else`、`while`
- 表达式：整数、浮点、布尔、字符串、变量引用、一元运算、二元运算、函数调用、结构体字面量、字段访问
- 解释器已支持：`main`、局部变量、算术/比较、条件、循环、函数调用、内置 `println`、结构体字段读取

注意：

- `main` 必须是 `fn main() -> i32`
- `main` 的返回值就是进程退出码；日常练习建议成功时返回 `0`
- `enum` 当前已经能被解析和检查，但最小解释器还没有实现值构造与执行
- 结构体当前支持字面量构造和字段读取，但还不支持字段写入

## 快速开始

先跑测试：

```powershell
cargo test
```

检查语义：

```powershell
cargo run -- check examples\hello.ax
cargo run -- check examples\syntax_overview.ax
```

查看 AST：

```powershell
cargo run -- ast examples\syntax_overview.ax
```

执行程序：

```powershell
cargo run -- run examples\hello.ax
cargo run -- run examples\factorial.ax
cargo run -- run examples\syntax_overview.ax
```

## 原型语法速查

### 顶层声明

函数：

```ax
fn add(left: i32, right: i32) -> i32 {
    return left + right;
}
```

结构体：

```ax
struct Point {
    x: i32,
    y: i32,
}
```

枚举：

```ax
enum Color {
    Red,
    Green,
    Blue,
}
```

### 变量与语句

不可变变量：

```ax
let value: i32 = 1;
```

可变变量：

```ax
let mut count: i32 = 0;
count = count + 1;
```

返回：

```ax
return count;
```

条件：

```ax
if (count > 0) {
    println("positive");
} else {
    println("zero-or-negative");
}
```

循环：

```ax
while (count < 3) {
    count = count + 1;
}
```

### 表达式

支持的字面量：

```ax
1
3.14
true
"hello"
```

支持的运算：

```ax
-value
!flag
left + right
left - right
left * right
left / right
left == right
left != right
left < right
left <= right
left > right
left >= right
```

函数调用：

```ax
println(value);
total(point);
```

结构体字面量与字段访问：

```ax
let point: Point = Point { x: 2, y: 3 };
println(point.x);
```

## 近似 EBNF

下面这份语法描述的是**当前仓库实现**，可以直接给人看，也可以作为 AI 生成 AX 原型代码时的约束参考。

```text
program        := item*
item           := function | struct_decl | enum_decl

function       := "fn" IDENT "(" param_list? ")" "->" type_ref block
param_list     := param ("," param)*
param          := IDENT ":" type_ref

struct_decl    := "struct" IDENT "{" struct_field_list? "}"
struct_field_list := struct_field ("," struct_field)* ","?
struct_field   := IDENT ":" type_ref

enum_decl      := "enum" IDENT "{" enum_variant_list? "}"
enum_variant_list := IDENT ("," IDENT)* ","?

block          := "{" stmt* "}"
stmt           := let_stmt
               | return_stmt
               | if_stmt
               | while_stmt
               | assign_stmt
               | expr_stmt
               | block

let_stmt       := "let" "mut"? IDENT ":" type_ref "=" expr ";"
return_stmt    := "return" expr? ";"
if_stmt        := "if" "(" expr ")" block ("else" (block | if_stmt))?
while_stmt     := "while" "(" expr ")" block
assign_stmt    := expr "=" expr ";"
expr_stmt      := expr ";"

expr           := binary_expr
binary_expr    := unary_expr (BINARY_OP unary_expr)*
unary_expr     := ("-" | "!") unary_expr | postfix_expr
postfix_expr   := primary_expr (call_suffix | field_suffix)*
call_suffix    := "(" arg_list? ")"
field_suffix   := "." IDENT
arg_list       := expr ("," expr)*

primary_expr   := INT
               | FLOAT
               | BOOL
               | STRING
               | IDENT
               | struct_literal
               | "(" expr ")"

struct_literal := IDENT "{" struct_init_list? "}"
struct_init_list := struct_init ("," struct_init)* ","?
struct_init    := IDENT ":" expr

type_ref       := "bool" | "i32" | "f32" | "string" | IDENT
```

## 给 AI 的生成规则

如果你要把 README 直接喂给 AI，让它生成当前原型可通过 `axc check` / `axc run` 的代码，请让它严格遵守下面这些规则：

1. 只使用本 README 列出的原型语法，不要发明 `for`、`match`、数组、模块、异常、泛型等能力。
2. 所有函数参数、返回类型、局部变量都必须显式标注类型。
3. `main` 必须写成 `fn main() -> i32 { ... }`。
4. `let`、赋值、调用、`return` 都必须带分号。
5. 条件和循环必须写成 `if (cond) { ... }` 与 `while (cond) { ... }`。
6. 构造结构体时使用 `TypeName { field: expr, ... }`。
7. 当前只允许读取字段，如 `point.x`，不要生成 `point.x = ...`。
8. `println` 是唯一内置函数，可以打印 `i32`、`f32`、`bool`、`string` 和结构体值。
9. 若程序执行成功，建议 `main` 返回 `0`。
10. 若你不确定某个语法是否支持，就不要使用；优先生成更朴素、更显式的代码。

## 推荐提示词

下面这段提示词可以直接给没有 AX 训练数据的模型使用：

```text
Generate code in the current AX prototype syntax only.
Rules:
- Use braces for all blocks.
- Use only fn, struct, enum, let, let mut, return, if/else, while.
- Every function parameter, return type, and local variable must have an explicit type.
- main must be exactly: fn main() -> i32 { ... }.
- End let/assignment/expression/return statements with semicolons.
- Supported primitive types are bool, i32, f32, string.
- Supported expressions are literals, variable references, unary ops, binary ops, function calls, struct literals, and field access.
- Construct structs with TypeName { field: expr, ... }.
- Do not use for, match, arrays, modules, imports, exceptions, async, or generics.
- Return 0 from main on success unless a different exit code is explicitly needed.
```

## 示例

最小运行示例：

```ax
fn main() -> i32 {
    let mut value: i32 = 1;
    value = value + 2;
    println(value);
    return 0;
}
```

递归示例：

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

结构体示例：

```ax
struct Point {
    x: i32,
    y: i32,
}

fn total(point: Point) -> i32 {
    return point.x + point.y;
}

fn main() -> i32 {
    let point: Point = Point { x: 2, y: 3 };
    println(total(point));
    return 0;
}
```
