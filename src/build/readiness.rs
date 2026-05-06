use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{self, Program as AstProgram};
use crate::source::SourceFile;

use super::*;

pub fn assess_aot_readiness(program: &AstProgram, input: AotReadinessInput<'_>) -> AotReadiness {
    assess_aot_readiness_inner(None, program, input)
}

pub fn assess_aot_readiness_with_source(
    source: &SourceFile,
    program: &AstProgram,
    input: AotReadinessInput<'_>,
) -> AotReadiness {
    assess_aot_readiness_inner(Some(source), program, input)
}

pub fn apply_registry_package_maturity_readiness(
    readiness: &mut AotReadiness,
    packages: &[RegistryPackageArtifact],
) {
    if packages.is_empty() {
        return;
    }

    insert_feature(readiness, "registry_packages");
    for package in packages {
        match package.maturity.as_str() {
            "stable_pure_ax" => insert_feature(readiness, "registry_package_stable_pure_ax"),
            "host_boundary_preview" => {
                insert_feature(readiness, "registry_package_host_boundary_preview")
            }
            "future_native_preview" => {
                insert_feature(readiness, "registry_package_future_native_preview")
            }
            _ => insert_feature(readiness, "registry_package_unknown_maturity"),
        }
    }

    let host_boundary_packages = package_aliases_by_maturity(packages, "host_boundary_preview");
    if !host_boundary_packages.is_empty() {
        push_blocker_if_missing(
            readiness,
            AotReadinessBlocker::new(
                "AOT0104",
                "package",
                format!(
                    "registry packages with host_boundary_preview maturity need native runtime ABI coverage before AOT parity: {}",
                    host_boundary_packages.join(", ")
                ),
                "Package-2",
            ),
        );
    }

    let future_native_packages = package_aliases_by_maturity(packages, "future_native_preview");
    if !future_native_packages.is_empty() {
        push_blocker_if_missing(
            readiness,
            AotReadinessBlocker::new(
                "AOT0105",
                "package",
                format!(
                    "registry packages with future_native_preview maturity are interpreter-first package previews until their native ABI is designed: {}",
                    future_native_packages.join(", ")
                ),
                "Package-2",
            ),
        );
    }

    readiness.single_file_core_candidate = false;
    readiness.recommended_next_steps.push(
        "use package maturity to distinguish pure AX package AOT candidates from host/native ABI blockers"
            .to_string(),
    );
}

fn assess_aot_readiness_inner(
    source: Option<&SourceFile>,
    program: &AstProgram,
    input: AotReadinessInput<'_>,
) -> AotReadiness {
    let mut features = BTreeSet::new();
    collect_aot_features(source, program, &mut features);

    if input.is_project {
        features.insert("project_sources".to_string());
    }
    if input.has_local_path_packages {
        features.insert("local_path_packages".to_string());
    }
    if input.package_lock_status.is_some() {
        features.insert("package_lock".to_string());
    }

    let mut blockers = vec![AotReadinessBlocker::new(
        "AOT0001",
        "backend",
        "native executable emission is not implemented yet",
        "Build-1",
    )];

    if input.has_local_path_packages && input.package_lock_status != Some("current") {
        blockers.push(AotReadinessBlocker::new(
            "AOT0103",
            "package",
            "local package graph must have a current AX.lock before it can be treated as reproducible AOT input",
            "P5",
        ));
    }
    if features.iter().any(|feature| {
        feature.starts_with("host_")
            && !matches!(
                feature.as_str(),
                "host_stdio"
                    | "host_argv"
                    | "host_env"
                    | "host_fs_read"
                    | "host_fs_write"
                    | "host_process"
            )
    }) {
        blockers.push(AotReadinessBlocker::new(
            "AOT0301",
            "runtime",
            "host boundary builtins need runtime-owned handle ABI coverage before AOT can preserve check/run behavior",
            "Build-2/Build-3",
        ));
    }
    if features.contains("string_runtime") || features.contains("string_list_runtime") {
        blockers.push(AotReadinessBlocker::new(
            "AOT0302",
            "runtime",
            "full string runtime operations and string_list values need a native runtime representation and ABI",
            "Build-1/Build-2",
        ));
    }
    let single_file_core_candidate = !input.has_local_path_packages
        && !features.iter().any(|feature| {
            matches!(
                feature.as_str(),
                "bytes_runtime" | "string_runtime" | "string_list_runtime"
            ) || (feature.starts_with("host_")
                && !matches!(
                    feature.as_str(),
                    "host_stdio"
                        | "host_argv"
                        | "host_env"
                        | "host_fs_read"
                        | "host_fs_write"
                        | "host_process"
                ))
        });

    AotReadiness {
        schema_version: 3,
        stage: "Build-0 skeleton".to_string(),
        status: "blocked".to_string(),
        executable_emission: false,
        planned_executable_artifact: true,
        single_file_core_candidate,
        required_backend_features: features.into_iter().collect(),
        blockers,
        recommended_next_steps: vec![
            "freeze the MIR subset for a single-file i32 main AOT prototype".to_string(),
            "keep axc run as the semantic reference while native output is pending".to_string(),
            "when packages are present, require axc lock <project> --check before AOT planning"
                .to_string(),
        ],
    }
}

fn insert_feature(readiness: &mut AotReadiness, feature: &str) {
    if readiness
        .required_backend_features
        .iter()
        .any(|candidate| candidate == feature)
    {
        return;
    }
    readiness
        .required_backend_features
        .push(feature.to_string());
    readiness.required_backend_features.sort();
}

fn push_blocker_if_missing(readiness: &mut AotReadiness, blocker: AotReadinessBlocker) {
    if readiness
        .blockers
        .iter()
        .any(|candidate| candidate.code == blocker.code)
    {
        return;
    }
    readiness.blockers.push(blocker);
}

fn package_aliases_by_maturity(
    packages: &[RegistryPackageArtifact],
    maturity: &str,
) -> Vec<String> {
    packages
        .iter()
        .filter(|package| package.maturity == maturity)
        .map(|package| package.alias.clone())
        .collect()
}

fn collect_aot_features(
    source: Option<&SourceFile>,
    program: &AstProgram,
    features: &mut BTreeSet<String>,
) {
    if program.source_units.len() > 1 {
        features.insert("multi_source_program".to_string());
    }
    if program
        .source_units
        .iter()
        .any(|unit| unit.module.is_some() || !unit.imports.is_empty())
    {
        features.insert("module_imports".to_string());
    }

    if let Some(source) = source {
        collect_reachable_item_aot_features(source, program, features);
    } else {
        for item in &program.items {
            collect_item_aot_features(&item.kind, features);
        }
    }
}

fn collect_reachable_item_aot_features(
    source: &SourceFile,
    program: &AstProgram,
    features: &mut BTreeSet<String>,
) {
    let mut functions = BTreeMap::<String, &ast::Item>::new();
    let mut simple_functions = BTreeMap::<String, Vec<String>>::new();
    let mut entry_functions = Vec::<String>::new();

    for item in &program.items {
        let ast::ItemKind::Function { name, .. } = &item.kind else {
            collect_item_aot_features(&item.kind, features);
            continue;
        };

        let qualified_name = qualified_item_name(source, program, item, name);
        if is_entry_function(source, program, item, name) {
            entry_functions.push(qualified_name.clone());
        }
        simple_functions
            .entry(name.clone())
            .or_default()
            .push(qualified_name.clone());
        functions.insert(qualified_name, item);
    }

    if entry_functions.is_empty() {
        entry_functions.extend(
            functions
                .keys()
                .filter(|name| name.as_str() == "main" || name.ends_with(".main"))
                .cloned(),
        );
    }

    let mut pending = entry_functions;
    let mut visited = BTreeSet::<String>::new();
    while let Some(function_name) = pending.pop() {
        if !visited.insert(function_name.clone()) {
            continue;
        }
        let Some(item) = functions.get(&function_name) else {
            continue;
        };
        collect_item_aot_features(&item.kind, features);

        let ast::ItemKind::Function { body, .. } = &item.kind else {
            continue;
        };
        let mut raw_calls = BTreeSet::<String>::new();
        collect_called_names_in_block(body, &mut raw_calls);
        for call in raw_calls {
            if functions.contains_key(&call) {
                pending.push(call);
                continue;
            }
            if let Some(candidates) = simple_functions.get(&call) {
                if candidates.len() == 1 {
                    pending.push(candidates[0].clone());
                }
            }
        }
    }
}

fn collect_item_aot_features(kind: &ast::ItemKind, features: &mut BTreeSet<String>) {
    match kind {
        ast::ItemKind::Function {
            type_params,
            type_param_bounds,
            params,
            return_type,
            body,
            ..
        } => {
            features.insert("functions".to_string());
            if !type_params.is_empty() {
                features.insert("generic_functions".to_string());
            }
            if !type_param_bounds.is_empty() {
                features.insert("trait_bounds".to_string());
            }
            for bound in type_param_bounds {
                collect_type_ref_aot_features(&bound.trait_ref, features);
            }
            for param in params {
                collect_type_ref_aot_features(&param.ty, features);
            }
            collect_type_ref_aot_features(return_type, features);
            collect_block_aot_features(body, features);
        }
        ast::ItemKind::Const { ty, value, .. } => {
            features.insert("consts".to_string());
            collect_type_ref_aot_features(ty, features);
            collect_expr_aot_features(value, features);
        }
        ast::ItemKind::TypeAlias {
            type_params,
            target,
            ..
        } => {
            features.insert("type_aliases".to_string());
            if !type_params.is_empty() {
                features.insert("generic_type_aliases".to_string());
            }
            collect_type_ref_aot_features(target, features);
        }
        ast::ItemKind::Struct {
            type_params,
            fields,
            ..
        } => {
            features.insert("structs".to_string());
            if !type_params.is_empty() {
                features.insert("generic_structs".to_string());
            }
            for field in fields {
                collect_type_ref_aot_features(&field.ty, features);
            }
        }
        ast::ItemKind::Enum {
            type_params,
            variants,
            ..
        } => {
            features.insert("enums".to_string());
            if !type_params.is_empty() {
                features.insert("generic_enums".to_string());
            }
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    features.insert("payload_enums".to_string());
                    collect_type_ref_aot_features(payload, features);
                }
            }
        }
        ast::ItemKind::Trait { methods, .. } => {
            features.insert("traits".to_string());
            for method in methods {
                for param in &method.params {
                    collect_type_ref_aot_features(&param.ty, features);
                }
                collect_type_ref_aot_features(&method.return_type, features);
            }
        }
        ast::ItemKind::Impl {
            type_params,
            trait_ref,
            target,
            methods,
        } => {
            features.insert("impl_methods".to_string());
            if !type_params.is_empty() {
                features.insert("generic_impls".to_string());
            }
            if let Some(trait_ref) = trait_ref {
                features.insert("trait_impls".to_string());
                collect_type_ref_aot_features(trait_ref, features);
            }
            collect_type_ref_aot_features(target, features);
            for method in methods {
                if !method.type_params.is_empty() {
                    features.insert("generic_methods".to_string());
                }
                for param in &method.params {
                    collect_type_ref_aot_features(&param.ty, features);
                }
                collect_type_ref_aot_features(&method.return_type, features);
                collect_block_aot_features(&method.body, features);
            }
        }
    }
}

fn collect_block_aot_features(block: &ast::Block, features: &mut BTreeSet<String>) {
    for statement in &block.statements {
        collect_stmt_aot_features(statement, features);
    }
}

fn collect_stmt_aot_features(statement: &ast::Stmt, features: &mut BTreeSet<String>) {
    match &statement.kind {
        ast::StmtKind::Let {
            ty, initializer, ..
        } => {
            collect_type_ref_aot_features(ty, features);
            collect_expr_aot_features(initializer, features);
        }
        ast::StmtKind::Assign { target, value } => {
            collect_assignment_target_aot_features(target, features);
            collect_expr_aot_features(target, features);
            collect_expr_aot_features(value, features);
        }
        ast::StmtKind::Expr { expr } => collect_expr_aot_features(expr, features),
        ast::StmtKind::Return { value } => {
            if let Some(value) = value {
                collect_expr_aot_features(value, features);
            }
        }
        ast::StmtKind::Break | ast::StmtKind::Continue => {
            features.insert("loop_control".to_string());
        }
        ast::StmtKind::Match { scrutinee, arms } => {
            features.insert("match_statements".to_string());
            collect_expr_aot_features(scrutinee, features);
            for arm in arms {
                collect_match_pattern_aot_features(&arm.pattern, features);
                if arm.guard.is_some() {
                    features.insert("match_guards".to_string());
                }
                if let Some(guard) = &arm.guard {
                    collect_expr_aot_features(guard, features);
                }
                collect_block_aot_features(&arm.body, features);
            }
        }
        ast::StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            features.insert("control_flow".to_string());
            collect_expr_aot_features(condition, features);
            collect_block_aot_features(then_branch, features);
            if let Some(else_branch) = else_branch {
                collect_block_aot_features(else_branch, features);
            }
        }
        ast::StmtKind::While { condition, body } => {
            features.insert("loops".to_string());
            collect_expr_aot_features(condition, features);
            collect_block_aot_features(body, features);
        }
        ast::StmtKind::For {
            initializer,
            condition,
            step,
            body,
        } => {
            features.insert("loops".to_string());
            if let Some(initializer) = initializer {
                collect_stmt_aot_features(initializer, features);
            }
            if let Some(condition) = condition {
                collect_expr_aot_features(condition, features);
            }
            if let Some(step) = step {
                collect_stmt_aot_features(step, features);
            }
            collect_block_aot_features(body, features);
        }
        ast::StmtKind::ForIn {
            binding,
            iterable,
            body,
        } => {
            features.insert("for_in".to_string());
            collect_type_ref_aot_features(&binding.ty, features);
            collect_expr_aot_features(iterable, features);
            collect_block_aot_features(body, features);
        }
        ast::StmtKind::Block { block } => collect_block_aot_features(block, features),
    }
}

fn qualified_item_name(
    source: &SourceFile,
    program: &AstProgram,
    item: &ast::Item,
    name: &str,
) -> String {
    let source_path = source.display_path_for_offset(item.span.start);
    let module_path = program
        .source_unit_for_path(source_path)
        .and_then(|unit| unit.module.as_ref())
        .map(|module| module.path.as_str());
    match module_path {
        Some(module_path) => format!("{module_path}.{name}"),
        None => name.to_string(),
    }
}

fn is_entry_function(
    source: &SourceFile,
    program: &AstProgram,
    item: &ast::Item,
    name: &str,
) -> bool {
    if name != "main" {
        return false;
    }
    let source_path = source.display_path_for_offset(item.span.start);
    program
        .source_unit_for_path(source_path)
        .is_none_or(|unit| unit.is_entry)
}

fn collect_called_names_in_block(block: &ast::Block, calls: &mut BTreeSet<String>) {
    for statement in &block.statements {
        collect_called_names_in_stmt(statement, calls);
    }
}

fn collect_called_names_in_stmt(statement: &ast::Stmt, calls: &mut BTreeSet<String>) {
    match &statement.kind {
        ast::StmtKind::Let { initializer, .. } => collect_called_names_in_expr(initializer, calls),
        ast::StmtKind::Assign { target, value } => {
            collect_called_names_in_expr(target, calls);
            collect_called_names_in_expr(value, calls);
        }
        ast::StmtKind::Expr { expr } => collect_called_names_in_expr(expr, calls),
        ast::StmtKind::Return { value } => {
            if let Some(value) = value {
                collect_called_names_in_expr(value, calls);
            }
        }
        ast::StmtKind::Break | ast::StmtKind::Continue => {}
        ast::StmtKind::Match { scrutinee, arms } => {
            collect_called_names_in_expr(scrutinee, calls);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_called_names_in_expr(guard, calls);
                }
                collect_called_names_in_block(&arm.body, calls);
            }
        }
        ast::StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_called_names_in_expr(condition, calls);
            collect_called_names_in_block(then_branch, calls);
            if let Some(else_branch) = else_branch {
                collect_called_names_in_block(else_branch, calls);
            }
        }
        ast::StmtKind::While { condition, body } => {
            collect_called_names_in_expr(condition, calls);
            collect_called_names_in_block(body, calls);
        }
        ast::StmtKind::For {
            initializer,
            condition,
            step,
            body,
        } => {
            if let Some(initializer) = initializer {
                collect_called_names_in_stmt(initializer, calls);
            }
            if let Some(condition) = condition {
                collect_called_names_in_expr(condition, calls);
            }
            if let Some(step) = step {
                collect_called_names_in_stmt(step, calls);
            }
            collect_called_names_in_block(body, calls);
        }
        ast::StmtKind::ForIn { iterable, body, .. } => {
            collect_called_names_in_expr(iterable, calls);
            collect_called_names_in_block(body, calls);
        }
        ast::StmtKind::Block { block } => collect_called_names_in_block(block, calls),
    }
}

fn collect_called_names_in_expr(expression: &ast::Expr, calls: &mut BTreeSet<String>) {
    match &expression.kind {
        ast::ExprKind::Int { .. }
        | ast::ExprKind::Float { .. }
        | ast::ExprKind::Bool { .. }
        | ast::ExprKind::String { .. }
        | ast::ExprKind::Name { .. }
        | ast::ExprKind::Error => {}
        ast::ExprKind::Unary { expr, .. } | ast::ExprKind::Try { expr } => {
            collect_called_names_in_expr(expr, calls);
        }
        ast::ExprKind::Binary { left, right, .. } => {
            collect_called_names_in_expr(left, calls);
            collect_called_names_in_expr(right, calls);
        }
        ast::ExprKind::Call { callee, arguments } => {
            if let Some(name) = callee.qualified_name() {
                calls.insert(name);
            }
            collect_called_names_in_expr(callee, calls);
            for argument in arguments {
                collect_called_names_in_expr(argument, calls);
            }
        }
        ast::ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_called_names_in_expr(&field.value, calls);
            }
        }
        ast::ExprKind::ArrayLiteral { elements } => {
            for element in elements {
                collect_called_names_in_expr(element, calls);
            }
        }
        ast::ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_called_names_in_stmt(statement, calls);
            }
            collect_called_names_in_expr(value, calls);
        }
        ast::ExprKind::Match { scrutinee, arms } => {
            collect_called_names_in_expr(scrutinee, calls);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_called_names_in_expr(guard, calls);
                }
                collect_called_names_in_expr(&arm.value, calls);
            }
        }
        ast::ExprKind::Field { base, .. } => collect_called_names_in_expr(base, calls),
        ast::ExprKind::Index { base, index } => {
            collect_called_names_in_expr(base, calls);
            collect_called_names_in_expr(index, calls);
        }
        ast::ExprKind::Slice { base, start, end } => {
            collect_called_names_in_expr(base, calls);
            collect_called_names_in_expr(start, calls);
            collect_called_names_in_expr(end, calls);
        }
    }
}

fn collect_expr_aot_features(expression: &ast::Expr, features: &mut BTreeSet<String>) {
    match &expression.kind {
        ast::ExprKind::Int { .. } => {
            features.insert("i32_values".to_string());
        }
        ast::ExprKind::Float { .. } => {
            features.insert("f32_values".to_string());
        }
        ast::ExprKind::Bool { .. } => {
            features.insert("bool_values".to_string());
        }
        ast::ExprKind::String { .. } => {
            features.insert("string_literals".to_string());
        }
        ast::ExprKind::Name { .. } | ast::ExprKind::Error => {}
        ast::ExprKind::Unary { expr, .. } => collect_expr_aot_features(expr, features),
        ast::ExprKind::Try { expr } => {
            features.insert("result_propagation".to_string());
            collect_expr_aot_features(expr, features);
        }
        ast::ExprKind::Binary { op, left, right } => {
            if matches!(op, ast::BinaryOp::Add)
                && (expr_may_produce_string(left) || expr_may_produce_string(right))
            {
                features.insert("string_concat".to_string());
            }
            collect_expr_aot_features(left, features);
            collect_expr_aot_features(right, features);
        }
        ast::ExprKind::Call { callee, arguments } => {
            if let Some(name) = callee.qualified_name() {
                collect_call_aot_features(&name, features);
            }
            collect_expr_aot_features(callee, features);
            for argument in arguments {
                collect_expr_aot_features(argument, features);
            }
        }
        ast::ExprKind::StructLiteral { fields, .. } => {
            features.insert("structs".to_string());
            for field in fields {
                collect_expr_aot_features(&field.value, features);
            }
        }
        ast::ExprKind::ArrayLiteral { elements } => {
            features.insert("arrays".to_string());
            for element in elements {
                collect_expr_aot_features(element, features);
            }
        }
        ast::ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_stmt_aot_features(statement, features);
            }
            collect_expr_aot_features(value, features);
        }
        ast::ExprKind::Match { scrutinee, arms } => {
            features.insert("match_expressions".to_string());
            collect_expr_aot_features(scrutinee, features);
            for arm in arms {
                collect_match_pattern_aot_features(&arm.pattern, features);
                if arm.guard.is_some() {
                    features.insert("match_guards".to_string());
                }
                if let Some(guard) = &arm.guard {
                    collect_expr_aot_features(guard, features);
                }
                collect_expr_aot_features(&arm.value, features);
            }
        }
        ast::ExprKind::Field { base, .. } => collect_expr_aot_features(base, features),
        ast::ExprKind::Index { base, index } => {
            features.insert("arrays".to_string());
            collect_expr_aot_features(base, features);
            collect_expr_aot_features(index, features);
        }
        ast::ExprKind::Slice { base, start, end } => {
            features.insert("slices".to_string());
            collect_expr_aot_features(base, features);
            collect_expr_aot_features(start, features);
            collect_expr_aot_features(end, features);
        }
    }
}

fn collect_assignment_target_aot_features(target: &ast::Expr, features: &mut BTreeSet<String>) {
    match &target.kind {
        ast::ExprKind::Index { .. } => {
            features.insert("array_writes".to_string());
        }
        ast::ExprKind::Slice { .. } => {
            features.insert("slice_writes".to_string());
        }
        ast::ExprKind::Field { base, .. } => {
            features.insert("struct_writes".to_string());
            collect_assignment_target_aot_features(base, features);
        }
        _ => {}
    }
}

fn collect_match_pattern_aot_features(
    pattern: &ast::MatchPattern,
    features: &mut BTreeSet<String>,
) {
    match &pattern.kind {
        ast::MatchPatternKind::Wildcard
        | ast::MatchPatternKind::Bool { .. }
        | ast::MatchPatternKind::Int { .. }
        | ast::MatchPatternKind::String { .. }
        | ast::MatchPatternKind::Error => {}
        ast::MatchPatternKind::Binding { .. } => {
            features.insert("pattern_bindings".to_string());
        }
        ast::MatchPatternKind::IntRange { .. } => {
            features.insert("range_patterns".to_string());
        }
        ast::MatchPatternKind::EnumVariant { payload, .. } => {
            features.insert("enum_patterns".to_string());
            if let Some(payload) = payload {
                features.insert("payload_enum_patterns".to_string());
                if matches!(payload, ast::EnumVariantPayloadPattern::Binding { .. }) {
                    features.insert("pattern_bindings".to_string());
                }
            }
        }
        ast::MatchPatternKind::Struct { fields, .. } => {
            features.insert("struct_patterns".to_string());
            if !fields.is_empty() {
                features.insert("pattern_bindings".to_string());
            }
        }
        ast::MatchPatternKind::Or { alternatives } => {
            features.insert("or_patterns".to_string());
            for alternative in alternatives {
                collect_match_pattern_aot_features(alternative, features);
            }
        }
    }
}

fn collect_type_ref_aot_features(ty: &ast::TypeRef, features: &mut BTreeSet<String>) {
    if let Some(name) = &ty.name {
        match name.as_str() {
            "bool" => {
                features.insert("bool_values".to_string());
            }
            "i32" => {
                features.insert("i32_values".to_string());
            }
            "f32" => {
                features.insert("f32_values".to_string());
            }
            "string" => {
                features.insert("string_values".to_string());
            }
            "bytes" => {
                features.insert("bytes_runtime".to_string());
            }
            "string_list" => {
                features.insert("string_list_runtime".to_string());
            }
            _ => {
                if name.ends_with("Result") || name.ends_with(".Result") {
                    features.insert("result_values".to_string());
                }
                if name.ends_with("Option") || name.ends_with(".Option") {
                    features.insert("option_values".to_string());
                }
            }
        }
    }
    if !ty.type_args.is_empty() {
        features.insert("generic_type_instances".to_string());
        for arg in &ty.type_args {
            collect_type_ref_aot_features(arg, features);
        }
    }
    if let Some(element) = &ty.element {
        if ty.length.is_some() {
            features.insert("arrays".to_string());
        } else {
            features.insert("slices".to_string());
        }
        collect_type_ref_aot_features(element, features);
    }
}

fn collect_call_aot_features(name: &str, features: &mut BTreeSet<String>) {
    if name == "println" {
        features.insert("host_stdio".to_string());
    }
    if name == "argv_len" || name == "argv_get" || name.starts_with("std.cli.") {
        features.insert("host_argv".to_string());
    }
    if name.starts_with("env_") || name.starts_with("std.env.") {
        features.insert("host_env".to_string());
    }
    if is_aot_supported_fs_read_call(name) {
        features.insert("host_fs_read".to_string());
    } else if is_aot_supported_fs_write_call(name) {
        features.insert("host_fs_write".to_string());
    } else if name.starts_with("fs_") || name.starts_with("std.fs.") {
        features.insert("host_fs".to_string());
    }
    if name.starts_with("process_") || name.starts_with("std.process.") {
        features.insert("host_process".to_string());
    }
    if name.starts_with("http_") || matches!(name, "std.http.get" | "std.http.try_get") {
        features.insert("host_http".to_string());
    }
    if name.starts_with("net_") || name.starts_with("std.net.") {
        features.insert("host_net".to_string());
    }
    if name.starts_with("db_") || name.starts_with("std.db.") || name.starts_with("std.database.") {
        features.insert("host_db".to_string());
    }
    if name.starts_with("bytes_")
        || name.starts_with("std.bytes.")
        || name.starts_with("std.encoding.")
        || name.starts_with("std.hash.")
    {
        features.insert("bytes_runtime".to_string());
    }
    if name.starts_with("path_") || name.starts_with("std.path.") {
        features.insert("path_runtime".to_string());
    }
    if name.starts_with("string_list_") || name.starts_with("std.collections.") {
        features.insert("string_list_runtime".to_string());
    }
    if name == "string_len" {
        features.insert("string_len".to_string());
    } else if matches!(
        name,
        "string_contains" | "string_starts_with" | "string_ends_with"
    ) {
        features.insert("string_predicates".to_string());
    } else if name == "string_replace" {
        features.insert("string_replace".to_string());
    } else if name == "string_split_lines" {
        features.insert("string_split_lines".to_string());
    } else if name == "string_trim" {
        features.insert("string_trim".to_string());
    } else if name == "to_string" {
        features.insert("to_string_values".to_string());
    } else if name.starts_with("string_")
        || is_std_http_string_helper(name)
        || name.starts_with("std.json.")
        || name.starts_with("std.hash.")
        || name.starts_with("std.text.")
        || name.starts_with("std.report.")
    {
        features.insert("string_runtime".to_string());
    }
}

fn is_std_http_string_helper(name: &str) -> bool {
    matches!(
        name,
        "std.http.status_text"
            | "std.http.status_class"
            | "std.http.query_pair"
            | "std.http.append_query"
            | "std.http.request_key"
            | "std.http.header_line"
            | "std.http.accept_json_header"
    )
}

fn is_aot_supported_fs_read_call(name: &str) -> bool {
    matches!(
        name,
        "fs_exists"
            | "fs_is_file"
            | "fs_is_dir"
            | "fs_file_size"
            | "fs_read_to_string"
            | "fs_read_dir"
            | "std.fs.exists"
            | "std.fs.is_file"
            | "std.fs.is_dir"
            | "std.fs.file_size"
            | "std.fs.try_file_size"
            | "std.fs.read_to_string"
            | "std.fs.try_read_to_string"
            | "std.fs.read_dir"
            | "std.fs.try_read_dir"
    )
}

fn is_aot_supported_fs_write_call(name: &str) -> bool {
    matches!(
        name,
        "fs_write_string"
            | "fs_remove_file"
            | "fs_rename"
            | "fs_copy_file"
            | "fs_create_dir_all"
            | "fs_remove_dir_all"
            | "std.fs.write_string"
            | "std.fs.remove_file"
            | "std.fs.remove_dir_all"
            | "std.fs.rename"
            | "std.fs.copy_file"
            | "std.fs.create_dir_all"
    )
}

fn expr_may_produce_string(expression: &ast::Expr) -> bool {
    match &expression.kind {
        ast::ExprKind::String { .. } => true,
        ast::ExprKind::Call { callee, .. } => callee.qualified_name().is_some_and(|name| {
            matches!(
                name.as_str(),
                "to_string" | "string_replace" | "string_trim"
            )
        }),
        ast::ExprKind::Binary { left, right, .. } => {
            expr_may_produce_string(left) || expr_may_produce_string(right)
        }
        ast::ExprKind::Unary { expr, .. }
        | ast::ExprKind::Try { expr }
        | ast::ExprKind::Field { base: expr, .. } => expr_may_produce_string(expr),
        ast::ExprKind::Block { value, .. } => expr_may_produce_string(value),
        ast::ExprKind::Match { arms, .. } => {
            arms.iter().any(|arm| expr_may_produce_string(&arm.value))
        }
        ast::ExprKind::Int { .. }
        | ast::ExprKind::Float { .. }
        | ast::ExprKind::Bool { .. }
        | ast::ExprKind::Name { .. }
        | ast::ExprKind::StructLiteral { .. }
        | ast::ExprKind::ArrayLiteral { .. }
        | ast::ExprKind::Index { .. }
        | ast::ExprKind::Slice { .. }
        | ast::ExprKind::Error => false,
    }
}
