use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinaryOp, Block, Expr, ExprKind, ItemKind, Program, Stmt, StmtKind, TypeRef, UnaryOp,
};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

pub fn check_program(source: &SourceFile, program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let program_info = ProgramInfo::collect(source, program, &mut diagnostics);

    if !program_info.has_main {
        diagnostics.push(
            Diagnostic::new(
                "S0004",
                "program is missing `fn main() -> i32`",
                source,
                Span::new(0, 0),
            )
            .with_suggestion("add `fn main() -> i32 { return 0; }`"),
        );
    }

    for item in &program.items {
        if let ItemKind::Function {
            name,
            params,
            return_type,
            body,
            ..
        } = &item.kind
        {
            let resolved_return_type =
                program_info.resolve_type_ref(return_type, &mut diagnostics);
            let mut checker =
                TypeChecker::new(&program_info, resolved_return_type, &mut diagnostics);

            for param in params {
                let resolved_param_type =
                    program_info.resolve_type_ref(&param.ty, checker.diagnostics);
                checker.declare(&param.name, resolved_param_type, false, param.span.start);
            }

            checker.check_block(body);
            let missing_return_type = if !block_guarantees_return(body) {
                Some(checker.return_type.describe())
            } else {
                None
            };
            drop(checker);

            if let Some(return_type_name) = missing_return_type {
                diagnostics.push(
                    Diagnostic::new(
                        "S0023",
                        format!(
                            "function `{name}` may complete without returning `{}`",
                            return_type_name
                        ),
                        source,
                        body.span,
                    )
                    .with_suggestion("ensure every control-flow path ends with `return ...;`"),
                );
            }
        }
    }

    diagnostics
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Type {
    Bool,
    I32,
    F32,
    String,
    Struct(String),
    Enum(String),
    Void,
    Error,
}

impl Type {
    fn describe(&self) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::I32 => "i32".to_string(),
            Self::F32 => "f32".to_string(),
            Self::String => "string".to_string(),
            Self::Struct(name) | Self::Enum(name) => name.clone(),
            Self::Void => "<void>".to_string(),
            Self::Error => "<error>".to_string(),
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Self::I32 | Self::F32)
    }

    fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    fn is_comparable_primitive(&self) -> bool {
        matches!(self, Self::Bool | Self::I32 | Self::F32 | Self::String)
    }
}

#[derive(Debug, Clone)]
struct FunctionSignature {
    params: Vec<ParamInfo>,
    return_type: Type,
}

#[derive(Debug, Clone)]
struct ParamInfo {
    name: String,
    ty: Type,
}

#[derive(Debug, Clone)]
struct StructInfo {
    fields: HashMap<String, StructFieldInfo>,
}

#[derive(Debug, Clone)]
struct StructFieldInfo {
    ty: Type,
    start: usize,
}

struct ProgramInfo<'a> {
    source: &'a SourceFile,
    named_types: HashMap<String, Type>,
    functions: HashMap<String, FunctionSignature>,
    structs: HashMap<String, StructInfo>,
    has_main: bool,
}

impl<'a> ProgramInfo<'a> {
    fn collect(
        source: &'a SourceFile,
        program: &Program,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut named_types = builtin_types();
        let mut all_item_names: HashMap<String, usize> = HashMap::new();
        let mut has_main = false;

        for item in &program.items {
            let name = item_name(&item.kind);
            if let Some(previous_start) = all_item_names.insert(name.to_string(), item.span.start) {
                let (line, column) = source.line_col(previous_start);
                diagnostics.push(
                    Diagnostic::new(
                        "S0001",
                        format!("duplicate definition of `{name}`"),
                        source,
                        item.span,
                    )
                    .with_note(format!("previous definition was at {line}:{column}")),
                );
            }

            match &item.kind {
                ItemKind::Struct { name, .. } => {
                    named_types.insert(name.clone(), Type::Struct(name.clone()));
                }
                ItemKind::Enum { name, .. } => {
                    named_types.insert(name.clone(), Type::Enum(name.clone()));
                }
                ItemKind::Function {
                    name,
                    params,
                    return_type,
                    ..
                } if name == "main" => {
                    has_main = true;
                    if !params.is_empty() || return_type.name != "i32" {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0005",
                                "`main` must have the signature `fn main() -> i32`",
                                source,
                                return_type.span,
                            )
                            .with_suggestion("change main to `fn main() -> i32 { ... }`"),
                        );
                    }
                }
                _ => {}
            }
        }

        let mut info = Self {
            source,
            named_types,
            functions: HashMap::new(),
            structs: HashMap::new(),
            has_main,
        };

        for item in &program.items {
            match &item.kind {
                ItemKind::Struct { name, fields } => {
                    let mut field_map = HashMap::new();
                    for field in fields {
                        let resolved_type = info.resolve_type_ref(&field.ty, diagnostics);
                        if let Some(previous_field) = field_map.insert(
                            field.name.clone(),
                            StructFieldInfo {
                                ty: resolved_type,
                                start: field.span.start,
                            },
                        ) {
                            let (line, column) = source.line_col(previous_field.start);
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0001",
                                    format!(
                                        "duplicate field `{}` in struct `{name}`",
                                        field.name
                                    ),
                                    source,
                                    field.span,
                                )
                                .with_note(format!("previous field was declared at {line}:{column}")),
                            );
                        }
                    }
                    info.structs.insert(name.clone(), StructInfo { fields: field_map });
                }
                ItemKind::Enum { name, variants } => {
                    let mut variant_names = HashSet::new();
                    for variant in variants {
                        if !variant_names.insert(variant.name.clone()) {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0001",
                                    format!(
                                        "duplicate variant `{}` in enum `{name}`",
                                        variant.name
                                    ),
                                    source,
                                    variant.span,
                                )
                                .with_suggestion("remove or rename the duplicate variant"),
                            );
                        }
                    }
                }
                ItemKind::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let resolved_params = params
                        .iter()
                        .map(|param| ParamInfo {
                            name: param.name.clone(),
                            ty: info.resolve_type_ref(&param.ty, diagnostics),
                        })
                        .collect::<Vec<_>>();
                    let resolved_return_type = info.resolve_type_ref(return_type, diagnostics);
                    info.functions.insert(
                        name.clone(),
                        FunctionSignature {
                            params: resolved_params,
                            return_type: resolved_return_type,
                        },
                    );
                }
            }
        }

        info
    }

    fn resolve_type_ref(
        &self,
        ty: &TypeRef,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Type {
        match self.named_types.get(&ty.name) {
            Some(found) => found.clone(),
            None => {
                diagnostics.push(
                    Diagnostic::new(
                        "S0006",
                        format!("unknown type `{}`", ty.name),
                        self.source,
                        ty.span,
                    )
                    .with_suggestion(
                        "use a builtin type or declare the type before referencing it",
                    ),
                );
                Type::Error
            }
        }
    }
}

struct TypeChecker<'a, 'b> {
    info: &'a ProgramInfo<'a>,
    return_type: Type,
    scopes: Vec<HashMap<String, Binding>>,
    diagnostics: &'b mut Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
struct Binding {
    mutable: bool,
    ty: Type,
    start: usize,
}

impl<'a, 'b> TypeChecker<'a, 'b> {
    fn new(
        info: &'a ProgramInfo<'a>,
        return_type: Type,
        diagnostics: &'b mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            info,
            return_type,
            scopes: vec![HashMap::new()],
            diagnostics,
        }
    }

    fn check_block(&mut self, block: &Block) {
        self.scopes.push(HashMap::new());
        for statement in &block.statements {
            self.check_statement(statement);
        }
        self.scopes.pop();
    }

    fn check_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                initializer,
            } => {
                let declared_type = self.info.resolve_type_ref(ty, self.diagnostics);
                let initializer_type = self.check_expr(initializer);
                self.expect_type_match(
                    &declared_type,
                    &initializer_type,
                    initializer.span,
                    format!(
                        "cannot initialize `{name}` of type `{}` with `{}`",
                        declared_type.describe(),
                        initializer_type.describe()
                    ),
                );
                self.declare(name, declared_type, *mutable, statement.span.start);
            }
            StmtKind::Assign { target, value } => {
                let value_type = self.check_expr(value);
                match &target.kind {
                    ExprKind::Name { value: name } => match self.lookup(name) {
                        Some(binding) if binding.mutable => {
                            self.expect_type_match(
                                &binding.ty,
                                &value_type,
                                value.span,
                                format!(
                                    "cannot assign `{}` to `{name}` of type `{}`",
                                    value_type.describe(),
                                    binding.ty.describe()
                                ),
                            );
                        }
                        Some(binding) => {
                            let (line, column) = self.info.source.line_col(binding.start);
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0003",
                                    format!("cannot assign to immutable variable `{name}`"),
                                    self.info.source,
                                    target.span,
                                )
                                .with_note(format!(
                                    "`{name}` was declared immutable at {line}:{column}"
                                ))
                                .with_suggestion(format!("declare `{name}` with `let mut`")),
                            );
                        }
                        None => {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0002",
                                    format!("use of undefined variable `{name}`"),
                                    self.info.source,
                                    target.span,
                                )
                                .with_suggestion(format!(
                                    "declare `{name}` before assigning to it"
                                )),
                            );
                        }
                    },
                    _ => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0008",
                                "assignment target must be a previously declared mutable variable",
                                self.info.source,
                                target.span,
                            )
                            .with_suggestion(
                                "assign directly to a variable name like `value = expr;`",
                            ),
                        );
                        self.check_expr(target);
                    }
                }
            }
            StmtKind::Expr { expr } => {
                self.check_expr(expr);
            }
            StmtKind::Return { value } => {
                let actual_type = match value {
                    Some(expr) => self.check_expr(expr),
                    None => Type::Void,
                };
                self.expect_type_match(
                    &self.return_type.clone(),
                    &actual_type,
                    statement.span,
                    return_type_message(&self.return_type, &actual_type),
                );
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_type = self.check_expr(condition);
                self.expect_type_match(
                    &Type::Bool,
                    &condition_type,
                    condition.span,
                    format!(
                        "`if` condition must be `bool`, found `{}`",
                        condition_type.describe()
                    ),
                );
                self.check_block(then_branch);
                if let Some(block) = else_branch {
                    self.check_block(block);
                }
            }
            StmtKind::While { condition, body } => {
                let condition_type = self.check_expr(condition);
                self.expect_type_match(
                    &Type::Bool,
                    &condition_type,
                    condition.span,
                    format!(
                        "`while` condition must be `bool`, found `{}`",
                        condition_type.describe()
                    ),
                );
                self.check_block(body);
            }
            StmtKind::Block { block } => self.check_block(block),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::Int { value } => {
                if i32::try_from(*value).is_err() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0009",
                            "integer literal is out of range for `i32`",
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion(
                            "use a value that fits in the AX `i32` range",
                        ),
                    );
                    Type::Error
                } else {
                    Type::I32
                }
            }
            ExprKind::Float { value } => {
                let narrowed = *value as f32;
                if value.is_finite() && !narrowed.is_finite() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0010",
                            "float literal is out of range for `f32`",
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion(
                            "use a smaller floating-point value that fits in `f32`",
                        ),
                    );
                    Type::Error
                } else {
                    Type::F32
                }
            }
            ExprKind::Bool { .. } => Type::Bool,
            ExprKind::String { .. } => Type::String,
            ExprKind::Name { value } => match self.lookup(value) {
                Some(binding) => binding.ty,
                None if self.info.functions.contains_key(value) => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0011",
                            format!("function `{value}` cannot be used as a value"),
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion(format!(
                            "call `{value}` with parentheses, for example `{value}(...)`",
                        )),
                    );
                    Type::Error
                }
                None => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0002",
                            format!("use of undefined variable `{value}`"),
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion(format!("declare `{value}` before using it")),
                    );
                    Type::Error
                }
            },
            ExprKind::Unary { op, expr: inner } => {
                let inner_type = self.check_expr(inner);
                if inner_type.is_error() {
                    return Type::Error;
                }

                match op {
                    UnaryOp::Negate if inner_type.is_numeric() => inner_type,
                    UnaryOp::Negate => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0012",
                                format!(
                                    "unary `-` expects `i32` or `f32`, found `{}`",
                                    inner_type.describe()
                                ),
                                self.info.source,
                                expr.span,
                            ),
                        );
                        Type::Error
                    }
                    UnaryOp::Not if inner_type == Type::Bool => Type::Bool,
                    UnaryOp::Not => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0013",
                                format!(
                                    "unary `!` expects `bool`, found `{}`",
                                    inner_type.describe()
                                ),
                                self.info.source,
                                expr.span,
                            ),
                        );
                        Type::Error
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                let left_type = self.check_expr(left);
                let right_type = self.check_expr(right);
                if left_type.is_error() || right_type.is_error() {
                    return Type::Error;
                }

                match op {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide => {
                        if left_type.is_numeric() && left_type == right_type {
                            left_type
                        } else {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0014",
                                    format!(
                                        "operator `{}` expects matching numeric operands, found `{}` and `{}`",
                                        binary_op_name(*op),
                                        left_type.describe(),
                                        right_type.describe()
                                    ),
                                    self.info.source,
                                    expr.span,
                                ),
                            );
                            Type::Error
                        }
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        if left_type == right_type && left_type.is_comparable_primitive() {
                            Type::Bool
                        } else {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0015",
                                    format!(
                                        "operator `{}` expects matching primitive operands, found `{}` and `{}`",
                                        binary_op_name(*op),
                                        left_type.describe(),
                                        right_type.describe()
                                    ),
                                    self.info.source,
                                    expr.span,
                                ),
                            );
                            Type::Error
                        }
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if left_type.is_numeric() && left_type == right_type {
                            Type::Bool
                        } else {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0016",
                                    format!(
                                        "operator `{}` expects matching numeric operands, found `{}` and `{}`",
                                        binary_op_name(*op),
                                        left_type.describe(),
                                        right_type.describe()
                                    ),
                                    self.info.source,
                                    expr.span,
                                ),
                            );
                            Type::Error
                        }
                    }
                }
            }
            ExprKind::Call { callee, arguments } => match &callee.kind {
                ExprKind::Name { value } if value == "println" => {
                    for argument in arguments {
                        self.check_expr(argument);
                    }
                    Type::Void
                }
                ExprKind::Name { value } => {
                    let signature = self.info.functions.get(value).cloned();
                    let argument_types = arguments
                        .iter()
                        .map(|argument| self.check_expr(argument))
                        .collect::<Vec<_>>();

                    match signature {
                        Some(signature) => {
                            if signature.params.len() != argument_types.len() {
                                self.diagnostics.push(
                                    Diagnostic::new(
                                        "S0017",
                                        format!(
                                            "function `{value}` expects {} argument(s), found {}",
                                            signature.params.len(),
                                            argument_types.len()
                                        ),
                                        self.info.source,
                                        expr.span,
                                    ),
                                );
                            }

                            for (argument, parameter) in argument_types
                                .iter()
                                .zip(signature.params.iter())
                            {
                                self.expect_type_match(
                                    &parameter.ty,
                                    argument,
                                    expr.span,
                                    format!(
                                        "function `{value}` expects argument `{}` to be `{}`, found `{}`",
                                        parameter.name,
                                        parameter.ty.describe(),
                                        argument.describe()
                                    ),
                                );
                            }

                            signature.return_type
                        }
                        None if self.lookup(value).is_some() => {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0018",
                                    format!("variable `{value}` is not callable"),
                                    self.info.source,
                                    callee.span,
                                )
                                .with_suggestion(
                                    "only function names and builtin functions can be called",
                                ),
                            );
                            Type::Error
                        }
                        None => {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0007",
                                    format!("call to undefined function `{value}`"),
                                    self.info.source,
                                    callee.span,
                                )
                                .with_suggestion(format!(
                                    "declare `{value}` or fix the call target"
                                )),
                            );
                            Type::Error
                        }
                    }
                }
                _ => {
                    self.check_expr(callee);
                    for argument in arguments {
                        self.check_expr(argument);
                    }
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0019",
                            "call target must be a function name",
                            self.info.source,
                            callee.span,
                        )
                        .with_suggestion(
                            "use a direct function call like `name(arg1, arg2)`",
                        ),
                    );
                    Type::Error
                }
            },
            ExprKind::StructLiteral { name, fields } => {
                let struct_info = match self.info.named_types.get(name).cloned() {
                    Some(Type::Struct(struct_name)) => {
                        self.info.structs.get(&struct_name).cloned().map(|info| (struct_name, info))
                    }
                    Some(other) => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0024",
                                format!(
                                    "`{name}` cannot be used as a struct literal because it is `{}`",
                                    other.describe()
                                ),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion(
                                "use the name of a declared `struct` for struct literal construction",
                            ),
                        );
                        None
                    }
                    None => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0006",
                                format!("unknown type `{name}`"),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion(
                                "declare the struct before constructing it",
                            ),
                        );
                        None
                    }
                };

                let Some((struct_name, struct_info)) = struct_info else {
                    for field in fields {
                        self.check_expr(&field.value);
                    }
                    return Type::Error;
                };

                let mut seen_fields = HashSet::new();
                for field in fields {
                    let value_type = self.check_expr(&field.value);
                    if !seen_fields.insert(field.name.clone()) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0025",
                                format!(
                                    "duplicate field `{}` in struct literal `{struct_name}`",
                                    field.name
                                ),
                                self.info.source,
                                field.span,
                            )
                            .with_suggestion("keep only one initializer for each field"),
                        );
                        continue;
                    }

                    match struct_info.fields.get(&field.name) {
                        Some(expected_field) => {
                            self.expect_type_match(
                                &expected_field.ty,
                                &value_type,
                                field.value.span,
                                format!(
                                    "field `{}` of `{struct_name}` expects `{}`, found `{}`",
                                    field.name,
                                    expected_field.ty.describe(),
                                    value_type.describe()
                                ),
                            );
                        }
                        None => {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0027",
                                    format!(
                                        "struct `{struct_name}` does not have a field `{}`",
                                        field.name
                                    ),
                                    self.info.source,
                                    field.span,
                                )
                                .with_suggestion(
                                    "use an existing field name from the struct declaration",
                                ),
                            );
                        }
                    }
                }

                for field_name in struct_info.fields.keys() {
                    if !seen_fields.contains(field_name) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0026",
                                format!(
                                    "struct literal `{struct_name}` is missing field `{field_name}`",
                                ),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion(format!(
                                "provide `{field_name}: ...` in the struct literal",
                            )),
                        );
                    }
                }

                Type::Struct(struct_name)
            }
            ExprKind::Field { base, field } => {
                let base_type = self.check_expr(base);
                match base_type {
                    Type::Struct(struct_name) => {
                        let struct_info = self.info.structs.get(&struct_name).cloned();
                        match struct_info {
                            Some(struct_info) => match struct_info.fields.get(field) {
                                Some(field_info) => field_info.ty.clone(),
                                None => {
                                    self.diagnostics.push(
                                        Diagnostic::new(
                                            "S0020",
                                            format!(
                                                "struct `{struct_name}` does not have a field `{field}`",
                                            ),
                                            self.info.source,
                                            expr.span,
                                        )
                                        .with_suggestion(
                                            "use an existing field name from the struct declaration",
                                        ),
                                    );
                                    Type::Error
                                }
                            },
                            None => Type::Error,
                        }
                    }
                    Type::Error => Type::Error,
                    other => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0021",
                                format!(
                                    "field access expects a struct value, found `{}`",
                                    other.describe()
                                ),
                                self.info.source,
                                expr.span,
                            ),
                        );
                        Type::Error
                    }
                }
            }
            ExprKind::Error => Type::Error,
        }
    }

    fn declare(&mut self, name: &str, ty: Type, mutable: bool, start: usize) {
        let current_scope = self.scopes.last_mut().expect("scope must exist");
        if let Some(previous) = current_scope.insert(
            name.to_string(),
            Binding {
                mutable,
                ty,
                start,
            },
        ) {
            let (line, column) = self.info.source.line_col(previous.start);
            self.diagnostics.push(
                Diagnostic::new(
                    "S0001",
                    format!("duplicate definition of `{name}`"),
                    self.info.source,
                    Span::new(start, start + name.len()),
                )
                .with_note(format!("previous definition was at {line}:{column}")),
            );
        }
    }

    fn lookup(&self, name: &str) -> Option<Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn expect_type_match(
        &mut self,
        expected: &Type,
        actual: &Type,
        span: Span,
        message: String,
    ) {
        if expected.is_error() || actual.is_error() || expected == actual {
            return;
        }

        self.diagnostics
            .push(Diagnostic::new("S0022", message, self.info.source, span));
    }
}

fn builtin_types() -> HashMap<String, Type> {
    [
        ("bool", Type::Bool),
        ("i32", Type::I32),
        ("f32", Type::F32),
        ("string", Type::String),
    ]
    .into_iter()
    .map(|(name, ty)| (name.to_string(), ty))
    .collect()
}

fn return_type_message(expected: &Type, actual: &Type) -> String {
    if *actual == Type::Void {
        format!(
            "return statement must produce `{}`, but no value was returned",
            expected.describe()
        )
    } else {
        format!(
            "return statement must produce `{}`, found `{}`",
            expected.describe(),
            actual.describe()
        )
    }
}

fn binary_op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
    }
}

fn item_name(kind: &ItemKind) -> &str {
    match kind {
        ItemKind::Function { name, .. }
        | ItemKind::Struct { name, .. }
        | ItemKind::Enum { name, .. } => name.as_str(),
    }
}

fn block_guarantees_return(block: &Block) -> bool {
    block
        .statements
        .iter()
        .any(statement_guarantees_return)
}

fn statement_guarantees_return(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Return { .. } => true,
        StmtKind::Block { block } => block_guarantees_return(block),
        StmtKind::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        } => block_guarantees_return(then_branch) && block_guarantees_return(else_branch),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::check_program;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::source::SourceFile;

    fn check(source_text: &str) -> Vec<String> {
        let source = SourceFile::anonymous(source_text);
        let tokens = tokenize(&source).tokens;
        let parsed = parse(&source, tokens);
        check_program(&source, &parsed.program)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    #[test]
    fn reports_immutable_assignment() {
        let codes = check("fn main() -> i32 { let value: i32 = 1; value = 2; return value; }");
        assert!(codes.iter().any(|code| code == "S0003"));
    }

    #[test]
    fn reports_missing_main() {
        let codes = check("fn helper() -> i32 { return 0; }");
        assert!(codes.iter().any(|code| code == "S0004"));
    }

    #[test]
    fn reports_duplicate_function_definitions() {
        let codes = check(
            "fn helper() -> i32 { return 0; } fn helper() -> i32 { return 1; } fn main() -> i32 { return helper(); }",
        );
        assert!(codes.iter().any(|code| code == "S0001"));
    }

    #[test]
    fn reports_type_mismatch_in_variable_declaration() {
        let codes = check("fn main() -> i32 { let value: bool = 1; return 0; }");
        assert!(codes.iter().any(|code| code == "S0022"));
    }

    #[test]
    fn reports_bad_function_argument_type() {
        let codes = check(
            "fn add(value: i32) -> i32 { return value; } fn main() -> i32 { return add(true); }",
        );
        assert!(codes.iter().any(|code| code == "S0022"));
    }

    #[test]
    fn reports_function_that_can_fall_through() {
        let codes =
            check("fn helper(flag: bool) -> i32 { if (flag) { return 1; } } fn main() -> i32 { return helper(true); }");
        assert!(codes.iter().any(|code| code == "S0023"));
    }

    #[test]
    fn checks_struct_literal_fields() {
        let codes = check(
            "struct Point { x: i32, y: i32 } fn main() -> i32 { let point: Point = Point { x: 1, y: 2 }; return point.x; }",
        );
        assert!(!codes.iter().any(|code| code == "S0022"));
        assert!(!codes.iter().any(|code| code == "S0026"));
    }

    #[test]
    fn reports_missing_struct_literal_field() {
        let codes = check(
            "struct Point { x: i32, y: i32 } fn main() -> i32 { let point: Point = Point { x: 1 }; return 0; }",
        );
        assert!(codes.iter().any(|code| code == "S0026"));
    }
}
