use std::collections::BTreeSet;

use crate::ast::{self, Program as AstProgram};

use super::*;

pub fn assess_aot_readiness(program: &AstProgram, input: AotReadinessInput<'_>) -> AotReadiness {
    let mut features = BTreeSet::new();
    collect_aot_features(program, &mut features);

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

    if input.is_project {
        blockers.push(AotReadinessBlocker::new(
            "AOT0101",
            "project",
            "project source graph packaging exists, but native project linking semantics are not implemented",
            "Build-2",
        ));
    }
    if input.has_local_path_packages {
        blockers.push(AotReadinessBlocker::new(
            "AOT0102",
            "package",
            "local path package sources are loadable, but native package linking semantics are not implemented",
            "Build-2/P5",
        ));
        if input.package_lock_status != Some("current") {
            blockers.push(AotReadinessBlocker::new(
                "AOT0103",
                "package",
                "local package graph must have a current AX.lock before it can be treated as reproducible AOT input",
                "P5",
            ));
        }
    }
    if feature_starts_with(&features, "generic_") || features.contains("generic_type_instances") {
        blockers.push(AotReadinessBlocker::new(
            "AOT0201",
            "language",
            "generic monomorphization and type-argument lowering are not frozen for native backend input",
            "Build-1/Build-2",
        ));
    }
    if features.contains("traits")
        || features.contains("trait_bounds")
        || features.contains("trait_impls")
    {
        blockers.push(AotReadinessBlocker::new(
            "AOT0202",
            "language",
            "trait/interface lowering, bound dispatch, and trait impl layout are not frozen for native backend input",
            "Build-2",
        ));
    }
    if features.contains("impl_methods") || features.contains("generic_methods") {
        blockers.push(AotReadinessBlocker::new(
            "AOT0203",
            "language",
            "impl method lowering and method ABI are not frozen for native backend input",
            "Build-2",
        ));
    }
    if features.contains("payload_enums")
        || features.contains("match_expressions")
        || features.contains("struct_patterns")
        || features.contains("or_patterns")
        || features.contains("range_patterns")
        || features.contains("match_guards")
        || features.contains("payload_enum_patterns")
        || features.contains("result_values")
        || features.contains("option_values")
    {
        blockers.push(AotReadinessBlocker::new(
            "AOT0204",
            "language",
            "payload enum layout, pattern tests, and match lowering need a native backend contract",
            "Build-2",
        ));
    }
    if features.contains("result_propagation") {
        blockers.push(AotReadinessBlocker::new(
            "AOT0205",
            "language",
            "`?` result propagation needs explicit native lowering for early-return control flow",
            "Build-2",
        ));
    }
    if features.contains("slice_writes") {
        blockers.push(AotReadinessBlocker::new(
            "AOT0206",
            "language",
            "slice mutation needs explicit native layout write semantics before AOT can preserve interpreter behavior",
            "Build-2",
        ));
    }
    if features
        .iter()
        .any(|feature| feature.starts_with("host_") && feature != "host_stdio")
    {
        blockers.push(AotReadinessBlocker::new(
            "AOT0301",
            "runtime",
            "host boundary builtins need a native runtime ABI before AOT can preserve check/run behavior",
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

    let single_file_core_candidate = !input.is_project
        && !input.has_local_path_packages
        && !features.iter().any(|feature| {
            matches!(
                feature.as_str(),
                "slices"
                    | "payload_enums"
                    | "match_expressions"
                    | "result_propagation"
                    | "slice_writes"
                    | "impl_methods"
                    | "traits"
                    | "trait_bounds"
                    | "trait_impls"
                    | "string_runtime"
                    | "string_list_runtime"
                    | "project_sources"
                    | "local_path_packages"
            ) || feature.starts_with("generic_")
                || (feature.starts_with("host_") && feature != "host_stdio")
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

fn feature_starts_with(features: &BTreeSet<String>, prefix: &str) -> bool {
    features.iter().any(|feature| feature.starts_with(prefix))
}

fn collect_aot_features(program: &AstProgram, features: &mut BTreeSet<String>) {
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

    for item in &program.items {
        collect_item_aot_features(&item.kind, features);
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
    if name.starts_with("fs_") || name.starts_with("std.fs.") {
        features.insert("host_fs".to_string());
    }
    if name.starts_with("process_") || name.starts_with("std.process.") {
        features.insert("host_process".to_string());
    }
    if name.starts_with("string_list_") || name.starts_with("std.collections.") {
        features.insert("string_list_runtime".to_string());
    }
    if name == "string_len" {
        features.insert("string_len".to_string());
    } else if name == "to_string" {
        features.insert("to_string_values".to_string());
    } else if name.starts_with("string_")
        || name.starts_with("std.text.")
        || name.starts_with("std.report.")
        || name.starts_with("std.path.")
    {
        features.insert("string_runtime".to_string());
    }
}

fn expr_may_produce_string(expression: &ast::Expr) -> bool {
    match &expression.kind {
        ast::ExprKind::String { .. } => true,
        ast::ExprKind::Call { callee, .. } => callee
            .qualified_name()
            .is_some_and(|name| name == "to_string" || name.starts_with("string_")),
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
