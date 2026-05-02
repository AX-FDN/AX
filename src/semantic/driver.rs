use crate::ast::{Block, Expr, ItemKind, Param, Program, TypeRef};
use crate::diagnostics::Diagnostic;
use crate::project::Project;
use crate::source::{SourceFile, Span};

use super::checker::TypeChecker;
use super::program_info::ProgramInfo;
use super::return_analysis::missing_return_diagnostic;
pub fn check_program(source: &SourceFile, program: &Program) -> Vec<Diagnostic> {
    check_program_with_project(source, program, None)
}

pub fn check_program_with_project(
    source: &SourceFile,
    program: &Program,
    project: Option<&Project>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let program_info = ProgramInfo::collect(source, program, project, &mut diagnostics);

    if !program_info.has_main {
        diagnostics.push(
            Diagnostic::new(
                "S0004",
                "program is missing `fn main() -> i32`",
                source,
                Span::new(0, 0),
            )
            .with_note("runnable AX programs currently require a zero-argument `main` entrypoint")
            .with_suggestion("add `fn main() -> i32 { return 0; }`"),
        );
    }

    for item in &program.items {
        if let ItemKind::Const { name, ty, value } = &item.kind {
            check_const_item(
                source,
                name,
                ty,
                value,
                &program_info,
                item.span.start,
                &mut diagnostics,
            );
        }
    }

    for item in &program.items {
        if let ItemKind::Function {
            name,
            type_params,
            params,
            return_type,
            body,
            ..
        } = &item.kind
        {
            check_function_body(
                source,
                name,
                type_params,
                params,
                return_type,
                body,
                &program_info,
                item.span.start,
                &mut diagnostics,
            );
        } else if let ItemKind::Impl {
            type_params,
            methods,
            ..
        } = &item.kind
        {
            for method in methods {
                let all_type_params = type_params
                    .iter()
                    .cloned()
                    .chain(method.type_params.iter().cloned())
                    .collect::<Vec<_>>();
                check_function_body(
                    source,
                    &method.name,
                    &all_type_params,
                    &method.params,
                    &method.return_type,
                    &method.body,
                    &program_info,
                    method.span.start,
                    &mut diagnostics,
                );
            }
        }
    }

    diagnostics
}

fn check_const_item(
    source: &SourceFile,
    name: &str,
    ty: &TypeRef,
    value: &Expr,
    program_info: &ProgramInfo<'_>,
    span_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let current_unit_path = source.display_path_for_offset(span_start).to_string();
    let declared_type = program_info.resolve_type_ref(ty, &current_unit_path, diagnostics);
    let mut checker = TypeChecker::new(
        program_info,
        declared_type.clone(),
        current_unit_path,
        Vec::new(),
        diagnostics,
    );
    let actual_type = checker.check_expr(value);
    if !actual_type.is_error() && !actual_type.is_assignable_to(&declared_type) {
        checker.expect_type_match(
            &declared_type,
            &actual_type,
            value.span,
            format!(
                "constant `{name}` is declared as `{}`, but value is `{}`",
                declared_type.describe(),
                actual_type.describe()
            ),
        );
    }
}

fn check_function_body(
    source: &SourceFile,
    name: &str,
    type_params: &[String],
    params: &[Param],
    return_type: &TypeRef,
    body: &Block,
    program_info: &ProgramInfo<'_>,
    span_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let current_unit_path = source.display_path_for_offset(span_start).to_string();
    let active_type_param_bounds = program_info
        .function_signature_for_definition(name, &current_unit_path)
        .map(|signature| signature.type_param_bounds.clone())
        .unwrap_or_default();
    let resolved_return_type = program_info.resolve_type_ref_with_params(
        return_type,
        &current_unit_path,
        type_params,
        diagnostics,
    );
    let mut checker = TypeChecker::new(
        program_info,
        resolved_return_type,
        current_unit_path.clone(),
        active_type_param_bounds,
        diagnostics,
    );

    for param in params {
        let resolved_param_type = program_info.resolve_type_ref_with_params(
            &param.ty,
            &current_unit_path,
            type_params,
            checker.diagnostics_mut(),
        );
        checker.declare(&param.name, resolved_param_type, false, param.span.start);
    }

    checker.check_block(body);
    let missing_return = missing_return_diagnostic(
        source,
        name,
        checker.return_type(),
        body,
        program_info,
        &current_unit_path,
    );
    drop(checker);

    if let Some(diagnostic) = missing_return {
        diagnostics.push(diagnostic);
    }
}
