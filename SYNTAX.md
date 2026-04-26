# AX Current Prototype Syntax

本文件描述的是**当前仓库已经实现并建议实践的 AX 原型语法**。

- 它服务于人类学习、AI 代码生成、示例编写与 `axc check / ast / run` 的当前行为理解。
- 它不是第二套设计基线；项目路线与最终决策仍以 [`PLAN.md`](./PLAN.md) 为准。

## 1. 快速规则

- 块语法固定为大括号：`{ ... }`
- 注释当前只支持 `//` 单行注释
- 顶层声明当前支持 `module`、`import`、`fn`、`struct`、`enum`
- 所有函数参数、返回类型、局部变量都必须显式写出类型
- `main` 必须是 `fn main() -> i32`
- `let`、赋值、表达式语句、`return` 必须带分号
- `if`、`while`、`for` 必须写成带括号的头部：`if (cond) { ... }`、`while (cond) { ... }`、`for (init; cond; step) { ... }`、`for (let value: T in values) { ... }`
- `break;` 当前已支持，可用于提前退出最近一层 `while` 或 `for`
- `continue;` 当前已支持，可用于跳过最近一层 `while` 或 `for` 的本次迭代并进入下一轮
- `match (...) { ... }` 当前已支持最小语句形态：模式只支持 `true` / `false`、整数、枚举值与 `_`
- 逻辑运算当前已支持 `&&` 与 `||`，并按短路语义执行
- 余数运算 `%` 当前已支持，且当前只接受 `i32` 操作数
- 枚举值必须写成 `EnumName.Variant`
- 可写目标当前支持嵌套路径：`point.x = expr;`、`outer.inner.value = expr;`、`tokens[index].value = expr;`

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

模块头（support source）：

```ax
module lib.report;
```

导入（entry 或 support source）：

```ax
import lib.report;
```

## 3. 类型

当前支持的类型只有下面这些：

- `bool`
- `i32`
- `f32`
- `string`
- `string_list`
- 只读切片：`[Type]`
- 固定长度数组：`[Type; N]`
- 用户声明的 `struct` 名
- 用户声明的 `enum` 名

当前没有：

- 泛型
- `Option` / `Result` 的完整表面语法

补充说明：

- AX 当前已经支持最小 `module / import` 模式：support source 可声明 `module ...;`，entry 与 support source 都可显式写 `import ...;`。
- 当前最小的代码组织方式仍然是项目清单：可以在 `AX.toml` 里用 `[package].sources = ["src/lib.ax", "lib", ...]` 列出额外源文件或源目录，在 `check / run / build` 时与 `entry` 一起装载；目录项会递归展开为稳定路径顺序的 `.ax` 文件列表。
- `module` 当前只允许出现在 support source，manifest `entry` 文件仍必须保持根入口身份并提供 `fn main() -> i32`。
- 当前仍不做 alias / wildcard import / `pub` / 远程依赖；设计说明见 [`docs/import-module-minimal-design.md`](C:/Users/xiaoy/Desktop/A语言/AX/docs/import-module-minimal-design.md)。

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

```ax
let mut outer: Outer = Outer { inner: Inner { value: 1 } };
outer.inner.value = outer.inner.value + 1;
```

数组元素写入：

```ax
let mut values: [i32; 3] = [1, 2, 3];
values[1] = values[0] + values[2];
```

```ax
let mut tokens: [Token; 2] = [Token { value: 1 }, Token { value: 2 }];
tokens[1].value = tokens[0].value + 4;
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

第一版 `for in`：

```ax
for (let value: i32 in values) {
    println(value);
}
```

- 第一版 `for in` 当前只支持数组 `[T; N]` 与 slice `[T]`
- loop variable 仍要求显式类型：`let value: T`
- 如果需要更底层控制，仍可退回 `for (init; cond; step)`

```ax
while (true) {
    if (ready) {
        break;
    }
}
```

```ax
for (let mut i: i32 = 0; i < 4; i = i + 1) {
    if (i == 2) {
        continue;
    }
    println(i);
}
```

`match`：

```ax
match (flag) {
    true => {
        println(1);
    }
    false => {
        println(0);
    }
}
```

```ax
match (value) {
    0 => {
        return 7;
    }
    _ => {
        return value;
    }
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
left % right
left && right
left || right
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

数组字面量：

```ax
[1, 2, 3]
```

索引读取：

```ax
values[1]
```

切片表达式：

```ax
values[1:3]
```

字符串拼接：

```ax
"AX " + "report"
string_len("AX report")
let mut items: string_list = string_list_new();
items = string_list_push(items, "alpha");
items = string_list_push(items, "beta");
string_list_join(items, ", ")
len("AX report")
len(items)
to_string(42)
```

枚举值：

```ax
Flag.On
Flag.Off
```

## 6. 当前 EBNF

```text
program           := source_unit+
source_unit       := module_decl? import_decl* item*
module_decl       := "module" qualified_name ";"
import_decl       := "import" qualified_name ";"
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
                  | break_stmt
                  | continue_stmt
                  | match_stmt
                  | if_stmt
                  | while_stmt
                  | for_stmt
                  | assign_stmt
                  | expr_stmt
                  | block

let_stmt          := "let" "mut"? IDENT ":" type_ref "=" expr ";"
return_stmt       := "return" expr? ";"
break_stmt        := "break" ";"
continue_stmt     := "continue" ";"
match_stmt        := "match" "(" expr ")" "{" match_arm+ "}"
match_arm         := match_pattern "=>" block
match_pattern     := "_" | "true" | "false" | INT | qualified_name
if_stmt           := "if" "(" expr ")" block ("else" (block | if_stmt))?
while_stmt        := "while" "(" expr ")" block
for_stmt          := "for" "(" for_init? ";" expr? ";" for_step? ")" block
for_init          := for_let_init | for_expr_stmt
for_let_init      := "let" "mut"? IDENT ":" type_ref "=" expr
for_step          := for_expr_stmt
for_expr_stmt     := expr ("=" expr)?
assign_stmt       := expr "=" expr ";"
expr_stmt         := expr ";"
qualified_name    := IDENT ("." IDENT)*

expr              := binary_expr
binary_expr       := unary_expr (BINARY_OP unary_expr)*
unary_expr        := ("-" | "!") unary_expr | postfix_expr
postfix_expr      := primary_expr (call_suffix | field_suffix | index_suffix | slice_suffix)*
call_suffix       := "(" arg_list? ")"
field_suffix      := "." IDENT
index_suffix      := "[" expr "]"
slice_suffix      := "[" expr ":" expr "]"
arg_list          := expr ("," expr)*

primary_expr      := INT
                  | FLOAT
                  | BOOL
                  | STRING
                  | IDENT
                  | struct_literal
                  | array_literal
                  | "(" expr ")"

struct_literal    := IDENT "{" struct_init_list? "}"
struct_init_list  := struct_init ("," struct_init)* ","?
struct_init       := IDENT ":" expr

array_literal     := "[" array_item_list? "]"
array_item_list   := expr ("," expr)* ","?

type_ref          := named_type | slice_type | array_type
named_type        := "bool" | "i32" | "f32" | "string" | IDENT
slice_type        := "[" type_ref "]"
array_type        := "[" type_ref ";" INT "]"
```

补充说明：

- `assign_stmt` 在语法层允许 `expr = expr;`，但语义层当前只接受从可变根绑定出发的可写路径：
- 变量赋值：`name = expr;`
- 结构体字段路径赋值：`name.field = expr;`、`name.field.other = expr;`
- 数组元素路径赋值：`name[index] = expr;`、`name[index].field = expr;`
- 只读切片仍然不能写入，因此 `view[index] = expr;` 和 `view[index].field = expr;` 都会被拒绝
- `break;` 只能出现在 `while` 或 `for` 的循环体内
- `continue;` 只能出现在 `while` 或 `for` 的循环体内
- `match` 当前只支持语句形态，不支持表达式形态
- `match` 模式当前只支持 `bool`、`i32`、枚举值与 `_`
- `match` 要求穷尽：`bool` 必须覆盖 `true/false`，枚举必须覆盖全部 variant，`i32` 当前必须以 `_` 兜底
- `&&` 与 `||` 当前要求两边都为 `bool`
- `%` 当前要求两边都为 `i32`
- `for` 当前支持的表头子句是：
- 初始化：空、`let`、赋值、表达式
- 条件：空或任意会检查为 `bool` 的表达式
- 迭代：空、赋值、表达式
- `for in` 当前只支持 `for (let value: T in values) { ... }`
- `for in` 中的 `values` 当前必须是数组或 slice，且 `T` 必须与元素类型一致

## 7. 当前解释器可执行范围

`axc run` 当前已经可以执行：

- `main`
- `string_list` 类型与 `string_list_new / string_list_push / string_list_join`
- 只读切片类型、切片表达式与切片索引读取
- 局部变量
- 变量赋值
- 结构体字段赋值
- 数组元素赋值
- 固定长度数组字面量与索引读取
- 算术与比较
- `if / else`
- `while`
- `for`
- `for in`
- `break`
- `continue`
- `match`
- 逻辑短路 `&&` / `||`
- 余数运算 `%`
- 用户函数调用
- 递归
- 内置 `string_len`
- 内置 `len`
- 内置 `to_string`
- 结构体字面量与字段读取
- 枚举值比较
- 内置 `println`

## 8. 当前不支持

下面这些请不要在当前原型里使用：

- 异常
- async / await
- 泛型
- 宏
- 原生后端

补充说明：

- 空数组字面量 `[]` 不是“完全不支持”。
- 当前只支持带显式零长度数组上下文的写法：`let values: [i32; 0] = [];`
- 如果上下文不是零长度数组，例如 `let values: [i32; 1] = [];`，会报 `S0032`。
- `match` 当前是最小第一版：不支持绑定模式、解构、guard、表达式返回值或多模式合并。
- `module / import` 当前是最小第一版：不支持 alias、wildcard import、`pub`、包管理与远程依赖。

## 9. 给 AI 的直接提示词

```text
Generate code in the current AX prototype syntax only.
Rules:
- Use braces for all blocks.
- Use only module, import, fn, struct, enum, let, let mut, return, if/else, while, for, and statement-form match.
- `break;` may be used to exit the nearest `while` or `for` loop early.
- `continue;` may be used to skip to the next iteration of the nearest `while` or `for` loop.
- `match` is statement-only and uses `match (value) { pattern => { ... } ... }`.
- `match` patterns currently support only `true`, `false`, integer literals, enum variants, and final `_`.
- `&&` and `||` are supported and both sides must produce `bool`.
- `%` is supported and currently requires `i32` operands.
- Every function parameter, return type, and local variable must have an explicit type.
- main must be exactly: fn main() -> i32 { ... }.
- End let/assignment/expression/return/`break`/`continue` statements with semicolons.
- Supported builtin types are bool, i32, f32, string, and string_list.
- Builtin helpers are println(...), string_len(text), string_list_new(), string_list_push(list, value), string_list_join(list, separator), len(value), and to_string(value).
- Enum values must use EnumName.Variant.
- Construct structs with TypeName { field: expr, ... }.
- Use for loops only as `for (init; condition; step) { ... }` or `for (let value: T in values) { ... }`.
- Read-only slices are allowed as [Type] and values[start:end].
- Fixed-size arrays are allowed as [Type; N], [a, b, c], and values[index].
- Empty array literals are allowed only in explicit zero-length array context, for example: let values: [i32; 0] = [];.
- Mutable write paths may target variables, nested struct fields, and fields selected from mutable array elements.
- Slice values remain read-only, so assignments through values[start:end] are not allowed.
- In project mode, support sources may declare `module ...;` and files may use explicit `import module.path;`.
- Do not use exceptions, async, generics, binding-pattern match arms, or expression-form match.
- Use [] only when the target type is explicitly a zero-length array like [i32; 0].
- Return 0 from main on success unless a different exit code is explicitly needed.
```
