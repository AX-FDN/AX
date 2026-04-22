# AX Current Prototype Syntax

本文件描述的是**当前仓库已经实现并建议实践的 AX 原型语法**。

- 它服务于人类学习、AI 代码生成、示例编写与 `axc check / ast / run` 的当前行为理解。
- 它不是第二套设计基线；项目路线与最终决策仍以 [`PLAN.md`](./PLAN.md) 为准。

## 1. 快速规则

- 块语法固定为大括号：`{ ... }`
- 注释当前只支持 `//` 单行注释
- 顶层声明只支持 `fn`、`struct`、`enum`
- 所有函数参数、返回类型、局部变量都必须显式写出类型
- `main` 必须是 `fn main() -> i32`
- `let`、赋值、表达式语句、`return` 必须带分号
- `if`、`while`、`for` 必须写成 `if (cond) { ... }`、`while (cond) { ... }`、`for (init; cond; step) { ... }`
- 枚举值必须写成 `EnumName.Variant`
- 结构体字段写入当前只支持直接形式：`point.x = expr;`

## 2. 顶层声明

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
enum Flag {
    On,
    Off,
}
```

## 3. 类型

当前支持的类型只有下面这些：

- `bool`
- `i32`
- `f32`
- `string`
- 用户声明的 `struct` 名
- 用户声明的 `enum` 名

当前没有：

- 数组
- 切片
- 泛型
- `Option` / `Result` 的完整表面语法
- 模块与 import

## 4. 语句

不可变变量：

```ax
let value: i32 = 1;
```

可变变量：

```ax
let mut count: i32 = 0;
count = count + 1;
```

结构体字段写入：

```ax
let mut point: Point = Point { x: 1, y: 2 };
point.x = point.x + 1;
```

返回：

```ax
return count;
return;
```

条件：

```ax
if (flag == Flag.On) {
    println("enabled");
} else {
    println("disabled");
}
```

循环：

```ax
while (count < 3) {
    count = count + 1;
}
```

`for` 循环：

```ax
for (let mut i: i32 = 0; i < 3; i = i + 1) {
    println(i);
}
```

## 5. 表达式

字面量：

```ax
1
3.14
true
"hello"
```

一元运算：

```ax
-value
!flag
```

二元运算：

```ax
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

结构体字面量：

```ax
Point { x: 2, y: 3 }
```

字段访问：

```ax
point.x
```

枚举值：

```ax
Flag.On
Flag.Off
```

## 6. 当前 EBNF

```text
program           := item*
item              := function | struct_decl | enum_decl

function          := "fn" IDENT "(" param_list? ")" "->" type_ref block
param_list        := param ("," param)*
param             := IDENT ":" type_ref

struct_decl       := "struct" IDENT "{" struct_field_list? "}"
struct_field_list := struct_field ("," struct_field)* ","?
struct_field      := IDENT ":" type_ref

enum_decl         := "enum" IDENT "{" enum_variant_list? "}"
enum_variant_list := IDENT ("," IDENT)* ","?

block             := "{" stmt* "}"
stmt              := let_stmt
                  | return_stmt
                  | if_stmt
                  | while_stmt
                  | for_stmt
                  | assign_stmt
                  | expr_stmt
                  | block

let_stmt          := "let" "mut"? IDENT ":" type_ref "=" expr ";"
return_stmt       := "return" expr? ";"
if_stmt           := "if" "(" expr ")" block ("else" (block | if_stmt))?
while_stmt        := "while" "(" expr ")" block
for_stmt          := "for" "(" for_init? ";" expr? ";" for_step? ")" block
for_init          := for_let_init | for_expr_stmt
for_let_init      := "let" "mut"? IDENT ":" type_ref "=" expr
for_step          := for_expr_stmt
for_expr_stmt     := expr ("=" expr)?
assign_stmt       := expr "=" expr ";"
expr_stmt         := expr ";"

expr              := binary_expr
binary_expr       := unary_expr (BINARY_OP unary_expr)*
unary_expr        := ("-" | "!") unary_expr | postfix_expr
postfix_expr      := primary_expr (call_suffix | field_suffix)*
call_suffix       := "(" arg_list? ")"
field_suffix      := "." IDENT
arg_list          := expr ("," expr)*

primary_expr      := INT
                  | FLOAT
                  | BOOL
                  | STRING
                  | IDENT
                  | struct_literal
                  | "(" expr ")"

struct_literal    := IDENT "{" struct_init_list? "}"
struct_init_list  := struct_init ("," struct_init)* ","?
struct_init       := IDENT ":" expr

type_ref          := "bool" | "i32" | "f32" | "string" | IDENT
```

补充说明：

- `assign_stmt` 在语法层允许 `expr = expr;`，但语义层当前只接受两种目标：
- 变量赋值：`name = expr;`
- 结构体字段赋值：`name.field = expr;`
- `for` 当前支持的表头子句是：
- 初始化：空、`let`、赋值、表达式
- 条件：空或任意会检查为 `bool` 的表达式
- 迭代：空、赋值、表达式

## 7. 当前解释器可执行范围

`axc run` 当前已经可以执行：

- `main`
- 局部变量
- 变量赋值
- 结构体字段赋值
- 算术与比较
- `if / else`
- `while`
- `for`
- 用户函数调用
- 递归
- 结构体字面量与字段读取
- 枚举值比较
- 内置 `println`

## 8. 当前不支持

下面这些请不要在当前原型里使用：

- `match`
- 数组 / 切片
- import / module
- manifest
- 异常
- async / await
- 泛型
- 宏
- 原生后端
- `axc build`

## 9. 给 AI 的直接提示词

```text
Generate code in the current AX prototype syntax only.
Rules:
- Use braces for all blocks.
- Use only fn, struct, enum, let, let mut, return, if/else, while, for.
- Every function parameter, return type, and local variable must have an explicit type.
- main must be exactly: fn main() -> i32 { ... }.
- End let/assignment/expression/return statements with semicolons.
- Supported primitive types are bool, i32, f32, string.
- Enum values must use EnumName.Variant.
- Construct structs with TypeName { field: expr, ... }.
- Use for loops only as for (init; condition; step) { ... }.
- Direct field assignment is allowed only as name.field = expr; and only when name is a mutable struct variable.
- Do not use match, arrays, modules, imports, exceptions, async, or generics.
- Return 0 from main on success unless a different exit code is explicitly needed.
```
