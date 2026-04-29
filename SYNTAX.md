# AX Current Prototype Syntax

本文件描述的是**当前仓库已经实现并建议实践的 AX 原型语法**。

- 它服务于人类学习、AI 代码生成、示例编写与 `axc check / ast / run` 的当前行为理解。
- 它不是第二套设计基线；项目路线与最终决策仍以 [`PLAN.md`](./PLAN.md) 为准。

## 1. 快速规则

- 块语法固定为大括号：`{ ... }`
- 注释当前只支持 `//` 单行注释
- 顶层声明当前支持 `module`、`import`、`pub`、`fn`、`const`、`struct`、`enum`、`trait`、`impl`
- 所有函数参数、返回类型、局部变量都必须显式写出类型
- `main` 必须是 `fn main() -> i32`
- `let`、赋值、表达式语句、`return` 必须带分号
- `if`、`while`、`for` 必须写成带括号的头部：`if (cond) { ... }`、`while (cond) { ... }`、`for (init; cond; step) { ... }`、`for (let value: T in values) { ... }`
- `break;` 当前已支持，可用于提前退出最近一层 `while` 或 `for`
- `continue;` 当前已支持，可用于跳过最近一层 `while` 或 `for` 的本次迭代并进入下一轮
- `match (...) { ... }` 当前已支持语句形态、表达式形态、最终裸标识符绑定模式、字符串字面量 pattern、payload enum pattern、`A | B` 多 pattern arm、`i32` range pattern 与 bool guard；模式当前支持 `true` / `false`、整数、`400..=499` 这类闭区间、字符串、枚举值、最终 `_`、最终裸标识符（如 `other`），以及 `Enum.Variant(name)` / `Enum.Variant(_)`
- 逻辑运算当前已支持 `&&` 与 `||`，并按短路语义执行
- 余数运算 `%` 当前已支持，且当前只接受 `i32` 操作数
- 枚举值必须写成 `EnumName.Variant`；如果该 variant 声明了 payload，则当前写成 `EnumName.Variant(value)`
- 方法当前写成 `impl Type { fn method(self: Type, ...) -> Ret { ... } }`，调用写成 `value.method(...)`；不带 `self` 的 inherent impl 函数是静态方法，调用写成 `Type.method(...)`；泛型类型可写成 `impl<T> Box<T> { ... }` 或 `impl<T> Trait for Box<T> { ... }`
- trait 当前写成 `trait Name { fn method(self: Self) -> Ret; }`，实现写成 `impl Name for Type { ... }`
- 泛型当前支持泛型结构体：`struct Box<T> { value: T }`，使用时写成 `Box<i32>`；结构体字面量仍写成 `Box { value: 1 }`；泛型 impl 可用 `impl<T> Box<T>` 与 `impl<T> Trait for Box<T>`
- 泛型函数当前支持由实参推断类型参数：`fn identity<T>(value: T) -> T { return value; }`
- 泛型函数支持一个或多个 trait bounds：`fn render<T: Label + ExitCode>(value: T) -> string { return value.label(); }`；也接受 `where` 写法并由 formatter 收敛到 canonical 泛型参数约束
- 泛型 enum 当前支持 Result-like 类型：`enum Result<T, E> { Ok(T), Err(E) }`，使用时写成 `Result<i32, string>`
- 类型别名当前支持非泛型与泛型别名：`type UserId = i32;`、`type Scores = [i32; 3];`、`type Boxed<T> = Box<T>;`
- 可写目标当前支持嵌套路径：`point.x = expr;`、`outer.inner.value = expr;`、`tokens[index].value = expr;`

## 2. 顶层声明

函数：

```ax
fn add(left: i32, right: i32) -> i32 {
    return left + right;
}
```

顶层常量：

```ax
const EXIT_OK: i32 = 0;
const TOOL_NAME: string = "ax";
```

类型别名：

```ax
type UserId = i32;
type Scores = [i32; 3];
type Boxed<T> = Box<T>;

fn sum_scores(scores: Scores) -> UserId {
    let mut total: UserId = 0;
    for (let score: i32 in scores) {
        total = total + score;
    }
    return total;
}
```

当前类型别名支持非泛型与泛型形式；递归别名、包级别名导出和更复杂的别名 diagnostics 保留给后续标准库与包接口阶段继续补强。

公开顶层声明：

```ax
pub const STATUS_OK: i32 = 0;

pub fn render_status(value: i32) -> string {
    return "status=" + to_string(value);
}
```

泛型函数：

```ax
fn identity<T>(value: T) -> T {
    return value;
}
```

带 trait bounds 的泛型函数：

```ax
fn render<T: Label + ExitCode>(value: T) -> string {
    return value.label() + ":" + to_string(value.exit_code());
}
```

结构体：

```ax
struct Point {
    x: i32,
    y: i32,
}
```

泛型结构体：

```ax
struct Box<T> {
    value: T,
}

fn main() -> i32 {
    let boxed: Box<i32> = Box { value: 7 };
    return boxed.value;
}
```

枚举：

```ax
enum Flag {
    On,
    Off,
}
```

```ax
enum Result {
    Ok(i32),
    Err(string),
    Empty,
}
```

泛型 enum / Result-like 类型：

```ax
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn value_or_zero(result: Result<i32, string>) -> i32 {
    return match (result) {
        Result.Ok(value) => value,
        Result.Err(_) => 0,
    };
}
```

方法：

```ax
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn origin() -> Point {
        return Point { x: 0, y: 0 };
    }

    fn sum(self: Point) -> i32 {
        return self.x + self.y;
    }
}

let origin: Point = Point.origin();
```

trait / interface：

```ax
trait Label {
    fn label(self: Self) -> string;
}

struct Command {
    name: string,
}

impl Label for Command {
    fn label(self: Command) -> string {
        return self.name;
    }
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
- 用户声明的泛型 `struct` 实例，例如 `Box<i32>`、`Pair<string>`
- 用户声明的 `enum` 名
- 用户声明的泛型 `enum` 实例，例如 `Result<i32, string>`
- 标准库约定类型：`std.option.Option<T>`、`std.result.Result<T, E>`

当前没有：

- 泛型 trait
- 错误传播语法，例如 `?`

补充说明：

- AX 当前已经支持最小 `module / import` 模式：support source 可声明 `module ...;`，entry 与 support source 都可显式写 `import ...;`。
- 当前最小的代码组织方式仍然是项目清单：可以在 `AX.toml` 里用 `[package].sources = ["src/lib.ax", "lib", ...]` 列出额外源文件或源目录，在 `check / run / build` 时与 `entry` 一起装载；目录项会递归展开为稳定路径顺序的 `.ax` 文件列表。
- `module` 当前只允许出现在 support source，manifest `entry` 文件仍必须保持根入口身份并提供 `fn main() -> i32`。
- 当前仍不做 alias / wildcard import / `pub` / 远程依赖；设计说明见 [`docs/import-module-minimal-design.md`](C:/Users/xiaoy/Desktop/A语言/AX/docs/import-module-minimal-design.md)。

标准 `Option` / `Result` 约定：

```ax
import std.option;
import std.result;

let maybe_score: std.option.Option<i32> = std.option.Option.Some(7);
let missing_score: std.option.Option<i32> = std.option.Option.None;

let parsed: std.result.Result<i32, string> = std.result.Result.Ok(7);
let failed: std.result.Result<i32, string> = std.result.Result.Err("bad");
```

这两个类型目前是 `std/` 中的 AX 源码模块，不是宿主语言直通接口。它们的目标是给 AI 和人类都提供稳定、显式、低歧义的“可能缺失/可能失败”返回值形态；后续错误传播语法必须建立在这套约定之上。

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

`for in`：

```ax
for (let value: i32 in values) {
    println(value);
}
```

- `for in` 当前只支持数组 `[T; N]` 与 slice `[T]`
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

```ax
let code: i32 = match (flag) {
    true => 1,
    false => 0,
};
```

```ax
let code: i32 = match (result) {
    Result.Ok(value) => value,
    Result.Err(_) => 0,
    Result.Empty => -1,
};
```

```ax
match (flag) {
    true => {
        println("true");
    }
    current => {
        println(to_string(current));
    }
}
```

```ax
let code: i32 = match (value) {
    0 => 10,
    other => other + 2,
};
```

- 语句形态使用 `pattern => { ... }`
- 表达式形态当前使用 `pattern => expr`
- 裸标识符 pattern 是最终 catch-all，并在当前 arm 内引入一个不可变局部名
- 多 pattern arm 写作 `A | B => ...`，当前用于字面量或 unit enum variant；不要在多 pattern arm 中引入绑定
- `i32` 闭区间 pattern 写作 `start..=end => ...`，常用于状态码、退出码、token 范围分类；区间 arm 仍需要最终 `_` 或绑定兜底来满足穷尽性
- guard 写作 `pattern if bool_expr => ...`；guard 必须是 `bool`，带 guard 的 arm 不参与穷尽性证明，可以读取当前 arm 引入的 pattern binding
- payload enum pattern 当前只支持单名字绑定或 `_`：`Result.Ok(value)`、`Result.Err(_)`
- payload enum 当前只支持 unit variant 与单 payload variant，不支持多 payload、命名字段或更深解构
- 表达式形态的所有 arm 必须返回同类型

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
Result.Ok(7)
Result.Err("bad")
Result.Empty
```

## 6. 当前 EBNF

```text
program           := source_unit+
source_unit       := module_decl? import_decl* item*
module_decl       := "module" qualified_name ";"
import_decl       := "import" qualified_name ";"
item              := visibility? (function | const_decl | type_alias | struct_decl | enum_decl | trait_decl | impl_decl)
visibility        := "pub"

function          := "fn" IDENT function_generic_params? "(" param_list? ")" "->" type_ref where_clause? block
param_list        := param ("," param)*
param             := IDENT ":" type_ref

const_decl        := "const" IDENT ":" type_ref "=" expr ";"
type_alias        := "type" IDENT plain_generic_params? "=" type_ref ";"

struct_decl       := "struct" IDENT plain_generic_params? "{" struct_field_list? "}"
function_generic_params := "<" function_generic_param ("," function_generic_param)* ">"
function_generic_param  := IDENT (":" type_ref ("+" type_ref)*)?
plain_generic_params    := "<" IDENT ("," IDENT)* ">"
where_clause      := "where" where_bound ("," where_bound)*
where_bound       := IDENT ":" type_ref ("+" type_ref)*
struct_field_list := struct_field ("," struct_field)* ","?
struct_field      := IDENT ":" type_ref

enum_decl         := "enum" IDENT plain_generic_params? "{" enum_variant_list? "}"
enum_variant_list := enum_variant ("," enum_variant)* ","?
enum_variant      := IDENT ("(" type_ref ")")?

trait_decl        := "trait" IDENT "{" trait_method* "}"
trait_method      := "fn" IDENT "(" param_list? ")" "->" type_ref ";"

impl_decl         := "impl" plain_generic_params? (type_ref | type_ref "for" type_ref) "{" impl_method* "}"
impl_method       := "fn" IDENT plain_generic_params? "(" param_list? ")" "->" type_ref block

type_ref          := qualified_name generic_args?
                  | "[" type_ref "]"
                  | "[" type_ref ";" INT "]"
generic_args      := "<" type_ref ("," type_ref)* ">"

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
match_arm         := match_pattern match_guard? "=>" block
match_guard       := "if" expr
match_pattern     := single_pattern ("|" single_pattern)*
single_pattern    := "_"
                  | "true"
                  | "false"
                  | INT
                  | INT "..=" INT
                  | STRING
                  | enum_pattern
                  | qualified_name
                  | binding_name
enum_pattern      := qualified_name ("(" ("_" | IDENT) ")")?
binding_name      := IDENT  // bare identifier without `.`; catch-all and must be final
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
                  | match_expr
                  | struct_literal
                  | array_literal
                  | "(" expr ")"

match_expr        := "match" "(" expr ")" "{" match_expr_arm ("," match_expr_arm)* ","? "}"
match_expr_arm    := match_pattern match_guard? "=>" expr

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
- `match` 当前支持语句形态、表达式形态、最终绑定 catch-all、payload enum pattern、`A | B` 多 pattern arm、`i32` range pattern 与 bool guard
- `match` 模式当前只支持 `bool`、`i32`、`i32` 闭区间、`string`、枚举值、最终 `_`、最终裸标识符绑定、`A | B` 多 pattern arm，以及 `Enum.Variant(name)` / `Enum.Variant(_)`
- `_` 与裸标识符绑定都属于 catch-all，必须出现在最后一个 arm
- 带 guard 的 arm 不参与穷尽性证明；guard 必须返回 `bool`，可以读取当前 arm 引入的 pattern binding
- `match` 要求穷尽：`bool` 必须覆盖 `true/false` 或最终 catch-all，枚举必须覆盖全部 variant 或最终 catch-all，`i32` 当前必须以 `_` 或最终绑定兜底
- 表达式 `match` 当前要求所有 arm 返回同类型，且 arm body 仍然必须是单个表达式
- payload enum 当前只支持单 payload variant：声明 `Ok(i32)`，构造 `Result.Ok(7)`，pattern 写成 `Result.Ok(value)` 或 `Result.Ok(_)`
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
- 泛型 trait
- 宏
- 原生后端

补充说明：

- 空数组字面量 `[]` 不是“完全不支持”。
- 当前只支持带显式零长度数组上下文的写法：`let values: [i32; 0] = [];`
- 如果上下文不是零长度数组，例如 `let values: [i32; 1] = [];`，会报 `S0032`。
- `match` 当前已支持表达式形态、最终绑定模式、字符串字面量 pattern、payload enum pattern、`A | B` 多 pattern arm、`i32` range pattern 与 binding-aware bool guard，但仍不支持结构体/数组/tuple 解构，表达式形态也还不支持 block-valued arm。
- `module / import` 当前支持显式模块声明与显式导入；`pub` 当前已作为顶层导出标记进入语法、formatter、AST/HIR/MIR、context 与 AI focus 元数据；暂不支持 alias、wildcard import、包管理与远程依赖。
- `impl / methods` 当前支持值方法、显式 `self: Type` 参数、不带 `self` 的静态方法、`impl<T> Box<T>` / `impl<T> Trait for Box<T>` 这类泛型 impl，以及方法自带类型参数的泛型方法；暂不支持可变接收者、方法重载或 trait 静态方法。
- `trait / interface` 当前支持 trait 方法签名、`impl Trait for Type`、缺失方法检查、签名匹配检查、trait impl 方法作为普通方法调用，以及泛型函数上的一个或多个 trait bounds；暂不支持动态派发、关联类型、默认方法或泛型 trait。
- `generic struct` 当前支持 `struct Box<T>`、`Box<i32>` 类型引用、字段推断、字段读取与可变字段写入；暂不支持 struct 级 trait bounds。
- `generic function` 当前支持 `fn identity<T>(value: T) -> T` 并由调用实参推断 `T`；也支持 `fn render<T: Label + ExitCode>(value: T) -> string` 与 `fn render<T>(value: T) -> string where T: Label + ExitCode` 这类 trait bounds；暂不支持显式 turbofish。
- `generic enum` 当前支持 `enum Result<T, E> { Ok(T), Err(E) }`、`Result<i32, string>`、payload 构造、unit variant 上下文归入与 `match` payload 绑定；`std.option.Option<T>` 与 `std.result.Result<T, E>` 已作为官方约定进入 `std/`；暂不支持 enum 级 trait bounds、多 payload tuple variant、命名 payload 字段或错误传播语法。
- `type alias` 当前支持非泛型与泛型类型别名，例如 `type UserId = i32;`、`type Scores = [i32; 3];` 与 `type Boxed<T> = Box<T>;`；暂不支持递归别名。

## 9. 给 AI 的直接提示词

```text
Generate code in the current AX prototype syntax only.
Rules:
- Use braces for all blocks.
- Use only module, import, pub, fn, const, type, struct, enum, trait, impl, let, let mut, return, if/else, while, for, and the current match forms.
- `break;` may be used to exit the nearest `while` or `for` loop early.
- `continue;` may be used to skip to the next iteration of the nearest `while` or `for` loop.
- `match` supports statement form `match (value) { pattern => { ... } ... }` and expression form `match (value) { pattern => expr, ... }`.
- `match` patterns currently support `true`, `false`, integer literals, inclusive `i32` ranges like `400..=499`, string literals, enum variants, `A | B` alternatives, payload enum patterns like `Result.Ok(value)` / `Result.Err(_)`, final `_`, and final bare binding names like `other`.
- `match` guards use `pattern if bool_expr => ...`; guards must be bool, do not count as exhaustive coverage, and may read pattern bindings introduced by the same arm.
- Top-level constants may be declared as `const NAME: Type = expr;` and used as read-only values.
- Public top-level declarations may use `pub`, such as `pub fn helper() -> i32 { ... }` or `pub const STATUS_OK: i32 = 0;`.
- Type aliases may be declared as `type UserId = i32;` or `type Boxed<T> = Box<T>;`; do not use recursive type aliases yet.
- A bare binding pattern is a final catch-all and introduces an immutable arm-local name.
- Expression-form `match` arms must stay single expressions and all arms must produce the same type.
- `&&` and `||` are supported and both sides must produce `bool`.
- `%` is supported and currently requires `i32` operands.
- Every function parameter, return type, and local variable must have an explicit type.
- main must be exactly: fn main() -> i32 { ... }.
- End let/assignment/expression/return/`break`/`continue` statements with semicolons.
- Supported builtin types are bool, i32, f32, string, and string_list.
- Builtin helpers are println(...), string_len(text), string_list_new(), string_list_push(list, value), string_list_join(list, separator), len(value), and to_string(value).
- Enum values must use `EnumName.Variant` or `EnumName.Variant(value)` when the variant declares a payload.
- Methods are declared in `impl Type { fn name(self: Type, ...) -> Ret { ... } }`, `impl<T> Box<T> { ... }`, or `impl<T> Trait for Box<T> { ... }` blocks and called as `value.name(...)`; static inherent methods omit `self` and are called as `Type.name(...)`.
- Traits are declared as `trait Name { fn method(self: Self) -> Ret; }` and implemented as `impl Name for Type { ... }`.
- Generic functions may use trait bounds such as `fn render<T: Label + ExitCode>(value: T) -> string { return value.label(); }`; `where` bounds are accepted and formatted back to canonical generic parameter bounds.
- Construct structs with TypeName { field: expr, ... }.
- Use for loops only as `for (init; condition; step) { ... }` or `for (let value: T in values) { ... }`.
- Read-only slices are allowed as [Type] and values[start:end].
- Fixed-size arrays are allowed as [Type; N], [a, b, c], and values[index].
- Empty array literals are allowed only in explicit zero-length array context, for example: let values: [i32; 0] = [];.
- Mutable write paths may target variables, nested struct fields, and fields selected from mutable array elements.
- Slice values remain read-only, so assignments through values[start:end] are not allowed.
- In project mode, support sources may declare `module ...;` and files may use explicit `import module.path;`.
- Generic structs may be declared as `struct Box<T> { value: T }` and used in type positions like `Box<i32>`; construct them with normal struct literals like `Box { value: 1 }`.
- Generic functions may be declared as `fn identity<T>(value: T) -> T { return value; }`; type parameters are inferred from arguments.
- Do not use exceptions, async, explicit turbofish calls, dynamic dispatch, associated types, default trait methods, generic traits, recursive type aliases, destructuring match patterns, named payload fields, or multi-payload enum variants.
- Use [] only when the target type is explicitly a zero-length array like [i32; 0].
- Return 0 from main on success unless a different exit code is explicitly needed.
```
