use std::collections::{HashMap, HashSet};

use crate::ast::{Block, Expr, ExprKind, ItemKind, Program, Stmt, StmtKind, TypeRef};
use crate::diagnostics::Diagnostic;
use crate::source::SourceFile;

pub fn check_program(source: &SourceFile, program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut all_item_names: HashMap<String, usize> = HashMap::new();
    let mut type_names: HashSet<String> = builtin_types();
    let mut function_names: HashSet<String> = HashSet::new();
    let mut main_seen = false;

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
            ItemKind::Struct { name, fields } => {
                type_names.insert(name.clone());
                for field in fields {
                    validate_type(source, &type_names, &field.ty, &mut diagnostics);
                }
            }
            ItemKind::Enum { name, .. } => {
                type_names.insert(name.clone());
            }
            ItemKind::Function {
                name,
                params,
                return_type,
                ..
            } => {
                function_names.insert(name.clone());
                if name == "main" {
                    main_seen = true;
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
            }
        }
    }

    if !main_seen {
        diagnostics.push(
            Diagnostic::new(
                "S0004",
                "program is missing `fn main() -> i32`",
                source,
                crate::source::Span::new(0, 0),
            )
            .with_suggestion("add `fn main() -> i32 { return 0; }`"),
        );
    }

    let checker = ProgramChecker {
        source,
        type_names,
        function_names,
    };

    for item in &program.items {
        if let ItemKind::Function {
            params,
            return_type,
            body,
            ..
        } = &item.kind
        {
            validate_type(source, &checker.type_names, return_type, &mut diagnostics);
            let mut scope_checker = ScopeChecker::new(&checker, &mut diagnostics);
            for param in params {
                validate_type(source, &checker.type_names, &param.ty, scope_checker.diagnostics);
                scope_checker.declare(&param.name, false, param.span.start);
            }
            scope_checker.check_block(body);
        }
    }

    diagnostics
}

struct ProgramChecker<'a> {
    source: &'a SourceFile,
    type_names: HashSet<String>,
    function_names: HashSet<String>,
}

struct ScopeChecker<'a, 'b> {
    checker: &'a ProgramChecker<'a>,
    scopes: Vec<HashMap<String, Binding>>,
    diagnostics: &'b mut Vec<Diagnostic>,
}

#[derive(Clone, Copy)]
struct Binding {
    mutable: bool,
    start: usize,
}

impl<'a, 'b> ScopeChecker<'a, 'b> {
    fn new(checker: &'a ProgramChecker<'a>, diagnostics: &'b mut Vec<Diagnostic>) -> Self {
        Self {
            checker,
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
                validate_type(self.checker.source, &self.checker.type_names, ty, self.diagnostics);
                self.check_expr(initializer);
                self.declare(name, *mutable, statement.span.start);
            }
            StmtKind::Assign { target, value } => {
                self.check_assignment_target(target);
                self.check_expr(value);
            }
            StmtKind::Expr { expr } => self.check_expr(expr),
            StmtKind::Return { value } => {
                if let Some(expr) = value {
                    self.check_expr(expr);
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_expr(condition);
                self.check_block(then_branch);
                if let Some(block) = else_branch {
                    self.check_block(block);
                }
            }
            StmtKind::While { condition, body } => {
                self.check_expr(condition);
                self.check_block(body);
            }
            StmtKind::Block { block } => self.check_block(block),
        }
    }

    fn check_assignment_target(&mut self, target: &Expr) {
        match &target.kind {
            ExprKind::Name { value } => match self.lookup(value) {
                Some(binding) if binding.mutable => {}
                Some(binding) => {
                    let (line, column) = self.checker.source.line_col(binding.start);
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0003",
                            format!("cannot assign to immutable variable `{value}`"),
                            self.checker.source,
                            target.span,
                        )
                        .with_note(format!("`{value}` was declared immutable at {line}:{column}"))
                        .with_suggestion(format!("declare `{value}` with `let mut`")),
                    );
                }
                None => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0002",
                            format!("use of undefined variable `{value}`"),
                            self.checker.source,
                            target.span,
                        )
                        .with_suggestion(format!("declare `{value}` before assigning to it")),
                    );
                }
            },
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0008",
                        "assignment target must be a previously declared mutable variable",
                        self.checker.source,
                        target.span,
                    )
                    .with_suggestion("assign directly to a variable name like `value = expr;`"),
                );
                self.check_expr(target);
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Int { .. }
            | ExprKind::Float { .. }
            | ExprKind::Bool { .. }
            | ExprKind::String { .. }
            | ExprKind::Error => {}
            ExprKind::Name { value } => {
                if self.lookup(value).is_none() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0002",
                            format!("use of undefined variable `{value}`"),
                            self.checker.source,
                            expr.span,
                        )
                        .with_suggestion(format!("declare `{value}` before using it")),
                    );
                }
            }
            ExprKind::Unary { expr, .. } => self.check_expr(expr),
            ExprKind::Binary { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            ExprKind::Call { callee, arguments } => {
                if let ExprKind::Name { value } = &callee.kind {
                    if self.lookup(value).is_none()
                        && !self.checker.function_names.contains(value)
                        && value != "println"
                    {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0007",
                                format!("call to undefined function `{value}`"),
                                self.checker.source,
                                callee.span,
                            )
                            .with_suggestion(format!("declare `{value}` or fix the call target")),
                        );
                    }
                } else {
                    self.check_expr(callee);
                }

                for argument in arguments {
                    self.check_expr(argument);
                }
            }
            ExprKind::Field { base, .. } => self.check_expr(base),
        }
    }

    fn declare(&mut self, name: &str, mutable: bool, start: usize) {
        let current_scope = self.scopes.last_mut().expect("scope must exist");
        if let Some(previous) = current_scope.insert(name.to_string(), Binding { mutable, start }) {
            let (line, column) = self.checker.source.line_col(previous.start);
            self.diagnostics.push(
                Diagnostic::new(
                    "S0001",
                    format!("duplicate definition of `{name}`"),
                    self.checker.source,
                    crate::source::Span::new(start, start + name.len()),
                )
                .with_note(format!("previous definition was at {line}:{column}")),
            );
        }
    }

    fn lookup(&self, name: &str) -> Option<Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }
}

fn builtin_types() -> HashSet<String> {
    ["bool", "i32", "f32", "string"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn validate_type(
    source: &SourceFile,
    types: &HashSet<String>,
    ty: &TypeRef,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !types.contains(&ty.name) {
        diagnostics.push(
            Diagnostic::new(
                "S0006",
                format!("unknown type `{}`", ty.name),
                source,
                ty.span,
            )
            .with_suggestion("use a builtin type or declare the type before referencing it"),
        );
    }
}

fn item_name(kind: &ItemKind) -> &str {
    match kind {
        ItemKind::Function { name, .. } | ItemKind::Struct { name, .. } | ItemKind::Enum { name, .. } => {
            name.as_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::check_program;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::source::SourceFile;

    #[test]
    fn reports_immutable_assignment() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let value: i32 = 1; value = 2; return value; }",
        );
        let tokens = tokenize(&source).tokens;
        let parsed = parse(&source, tokens);
        let diagnostics = check_program(&source, &parsed.program);
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code == "S0003"));
    }

    #[test]
    fn reports_missing_main() {
        let source = SourceFile::anonymous("fn helper() -> i32 { return 0; }");
        let tokens = tokenize(&source).tokens;
        let parsed = parse(&source, tokens);
        let diagnostics = check_program(&source, &parsed.program);
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code == "S0004"));
    }
}
