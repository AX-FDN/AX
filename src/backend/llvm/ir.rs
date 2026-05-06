use std::collections::BTreeMap;
use std::fmt::Write;

use crate::hir::{EnumVariant, EnumVariantPayloadPattern, StructField};
use crate::mir::{
    BasicBlock, BinaryOp, Expr, ExprKind, ItemKind, Local, MatchExprArm, MatchPattern,
    MatchPatternKind, Param, Place, PlaceKind, Program, Statement, StatementKind,
    StructLiteralField, TerminatorKind, Type, UnaryOp,
};

use super::{abi, diagnostic, monomorph, runtime, symbols};

mod emitter;
mod layout;
pub(in crate::backend::llvm) mod specialization;
mod support;

use self::layout::*;
use self::specialization::*;
use self::support::*;

#[derive(Clone)]
struct FunctionSignature {
    symbol: String,
    params: Vec<String>,
    param_ax_types: Vec<Type>,
    return_type: String,
    return_ax_type: Type,
}

struct ResolvedCallSignature<'a> {
    name: String,
    signature: &'a FunctionSignature,
}

#[derive(Clone)]
struct LocalSlot {
    ptr: String,
    ty: String,
    ax_ty: Type,
}

#[derive(Clone)]
struct LlvmValue {
    ty: String,
    repr: String,
    ax_ty: Option<Type>,
}

#[derive(Clone)]
struct ConstBinding {
    ty: Type,
    value: Expr,
}

#[derive(Clone)]
struct StructLayout {
    name: String,
    ax_ty: Type,
    ty: String,
    fields: Vec<StructFieldLayout>,
}

#[derive(Clone)]
struct StructFieldLayout {
    name: String,
    index: usize,
    ty: String,
    ax_ty: Type,
}

#[derive(Clone)]
struct EnumLayout {
    name: String,
    ax_ty: Type,
    ty: String,
    variants: Vec<EnumVariantLayout>,
}

#[derive(Clone)]
struct EnumVariantLayout {
    name: String,
    tag: i32,
    payload_ax_ty: Option<Type>,
}

#[derive(Clone)]
struct StringLiteral {
    symbol: String,
    len: usize,
    encoded: String,
}

struct FunctionEmitter<'a> {
    signatures: &'a BTreeMap<String, FunctionSignature>,
    layouts: &'a BTreeMap<String, StructLayout>,
    enum_layouts: &'a BTreeMap<String, EnumLayout>,
    consts: &'a BTreeMap<String, ConstBinding>,
    strings: &'a BTreeMap<String, StringLiteral>,
    locals: BTreeMap<u32, LocalSlot>,
    return_ax_ty: Type,
    const_stack: Vec<String>,
    temp_index: u32,
}

pub fn render_program(program: &Program) -> Result<String, Vec<String>> {
    render_program_with_diagnostics(program).map_err(diagnostic::user_messages)
}

fn render_program_with_diagnostics(
    program: &Program,
) -> Result<String, Vec<diagnostic::AotLoweringDiagnostic>> {
    let mut unsupported = Vec::new();
    let mut signatures = BTreeMap::new();
    let enum_layouts = match collect_enum_layouts(program) {
        Ok(layouts) => layouts,
        Err(reasons) => {
            unsupported.extend(
                reasons
                    .into_iter()
                    .map(diagnostic::AotLoweringDiagnostic::llvm_lowering),
            );
            BTreeMap::new()
        }
    };
    let layouts = match collect_struct_layouts(program, &enum_layouts) {
        Ok(layouts) => layouts,
        Err(reasons) => {
            unsupported.extend(
                reasons
                    .into_iter()
                    .map(diagnostic::AotLoweringDiagnostic::llvm_lowering),
            );
            BTreeMap::new()
        }
    };
    let monomorph_plan = match monomorph::plan_program(program) {
        Ok(plan) => plan,
        Err(reasons) => {
            unsupported.extend(
                reasons
                    .into_iter()
                    .map(diagnostic::AotLoweringDiagnostic::monomorphization),
            );
            monomorph::MonomorphizationPlan::empty()
        }
    };

    let mut consts = BTreeMap::new();
    for item in &program.items {
        match &item.kind {
            ItemKind::Function {
                name,
                type_params,
                type_param_bounds,
                params,
                return_type,
                ..
            } => {
                if !type_params.is_empty() {
                    continue;
                }
                if !monomorph_plan.reachable_functions().contains(name) {
                    continue;
                }
                if !type_param_bounds.is_empty() {
                    unsupported.push(diagnostic::AotLoweringDiagnostic::aot_readiness(
                        "trait_bounds",
                        format!(
                            "function `{name}` uses trait bounds, which LLVM AOT v0 does not lower"
                        ),
                    ));
                    continue;
                }

                let mut lowered_params = Vec::new();
                for param in params {
                    match llvm_type(&param.ty, &layouts, &enum_layouts) {
                        Some(ty) => lowered_params.push(ty),
                        None => unsupported.push(diagnostic::AotLoweringDiagnostic::runtime_abi(
                            ax_type_name(&param.ty),
                            format!(
                                "function `{name}` parameter `{}` uses unsupported type {}",
                                param.name,
                                ax_type_name(&param.ty)
                            ),
                        )),
                    }
                }

                let return_ax_type = return_type.clone();
                let Some(lowered_return_type) = llvm_type(return_type, &layouts, &enum_layouts)
                else {
                    unsupported.push(diagnostic::AotLoweringDiagnostic::runtime_abi(
                        ax_type_name(return_type),
                        format!(
                            "function `{name}` returns unsupported type {}",
                            ax_type_name(return_type)
                        ),
                    ));
                    continue;
                };

                signatures.insert(
                    name.clone(),
                    FunctionSignature {
                        symbol: llvm_symbol(name),
                        params: lowered_params,
                        param_ax_types: params.iter().map(|param| param.ty.clone()).collect(),
                        return_type: lowered_return_type,
                        return_ax_type,
                    },
                );
            }
            ItemKind::Const { name, ty, value } => {
                if llvm_type(ty, &layouts, &enum_layouts).is_none() {
                    unsupported.push(diagnostic::AotLoweringDiagnostic::runtime_abi(
                        ax_type_name(ty),
                        format!(
                            "top-level const `{name}` uses unsupported type {}",
                            ax_type_name(ty)
                        ),
                    ));
                    continue;
                }
                consts.insert(
                    name.clone(),
                    ConstBinding {
                        ty: ty.clone(),
                        value: value.clone(),
                    },
                );
            }
            ItemKind::Struct { .. } => {}
            ItemKind::Enum { .. } => {}
        }
    }

    for specialization in monomorph_plan.used_concrete_instances() {
        let Some(FunctionSource {
            type_params,
            params,
            return_type,
            ..
        }) = find_function_source(program, &specialization.source_name)
        else {
            unsupported.push(diagnostic::AotLoweringDiagnostic::monomorphization(
                format!(
                    "generic method specialization `{}` has no source function `{}`",
                    specialization.key, specialization.source_name
                ),
            ));
            continue;
        };
        if type_params
            .iter()
            .any(|param| !specialization.substitutions.contains_key(param))
        {
            unsupported.push(diagnostic::AotLoweringDiagnostic::monomorphization(
                format!(
                    "generic method specialization `{}` does not bind every type parameter",
                    specialization.source_name
                ),
            ));
            continue;
        }

        let specialized_params = substitute_params(params, &specialization.substitutions);
        let specialized_return_type =
            substitute_type_params(return_type, &specialization.substitutions);
        let mut lowered_params = Vec::new();
        for param in &specialized_params {
            match llvm_type(&param.ty, &layouts, &enum_layouts) {
                Some(ty) => lowered_params.push(ty),
                None => unsupported.push(diagnostic::AotLoweringDiagnostic::runtime_abi(
                    ax_type_name(&param.ty),
                    format!(
                        "function `{}` parameter `{}` uses unsupported type {}",
                        specialization.key,
                        param.name,
                        ax_type_name(&param.ty)
                    ),
                )),
            }
        }

        let Some(lowered_return_type) =
            llvm_type(&specialized_return_type, &layouts, &enum_layouts)
        else {
            unsupported.push(diagnostic::AotLoweringDiagnostic::runtime_abi(
                ax_type_name(&specialized_return_type),
                format!(
                    "function `{}` returns unsupported type {}",
                    specialization.key,
                    ax_type_name(&specialized_return_type)
                ),
            ));
            continue;
        };

        signatures.insert(
            specialization.key.clone(),
            FunctionSignature {
                symbol: llvm_symbol(&specialization.key),
                params: lowered_params,
                param_ax_types: specialized_params
                    .iter()
                    .map(|param| param.ty.clone())
                    .collect(),
                return_type: lowered_return_type,
                return_ax_type: specialized_return_type,
            },
        );
    }

    if !unsupported.is_empty() {
        return Err(unsupported);
    }

    if !signatures.contains_key("main") {
        return Err(vec![diagnostic::AotLoweringDiagnostic::aot_readiness(
            "entrypoint",
            "LLVM AOT v0 requires an explicit `fn main() -> i32` entrypoint",
        )]);
    }

    let strings = collect_string_literals(program, &layouts, &enum_layouts);
    let mut module = String::new();
    writeln!(module, "; generated by axc LLVM AOT v0").expect("writing to string cannot fail");
    writeln!(module, "source_filename = \"axc\"").expect("writing to string cannot fail");
    writeln!(module, "@.ax_argc = private global i32 0").expect("writing to string cannot fail");
    writeln!(module, "@.ax_argv = private global ptr null").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    for layout in enum_layouts.values() {
        if layout.ty != "i32" {
            writeln!(module, "{} = type {{ i32, ptr }}", layout.ty)
                .expect("writing to string cannot fail");
        }
    }
    for layout in layouts.values() {
        let fields = layout
            .fields
            .iter()
            .map(|field| field.ty.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let body = if fields.is_empty() {
            "{}".to_string()
        } else {
            format!("{{ {fields} }}")
        };
        writeln!(module, "{} = type {}", layout.ty, body).expect("writing to string cannot fail");
    }
    if enum_layouts.values().any(|layout| layout.ty != "i32") || !layouts.is_empty() {
        writeln!(module).expect("writing to string cannot fail");
    }
    runtime::write_builtin_globals(&mut module);
    for literal in strings.values() {
        writeln!(
            module,
            "{} = private unnamed_addr constant [{} x i8] c\"{}\"",
            literal.symbol, literal.len, literal.encoded
        )
        .expect("writing to string cannot fail");
    }
    runtime::write_external_declarations(&mut module);
    writeln!(module).expect("writing to string cannot fail");
    runtime::write_runtime_error_helper(&mut module);
    runtime::write_string_helpers(&mut module);
    runtime::write_bytes_helpers(&mut module);
    runtime::write_host_helpers(&mut module);

    for item in &program.items {
        let ItemKind::Function {
            name,
            type_params,
            type_param_bounds,
            params,
            return_type,
            locals,
            entry_block,
            blocks,
            ..
        } = &item.kind
        else {
            continue;
        };

        if !type_params.is_empty() || !type_param_bounds.is_empty() {
            continue;
        }
        if !monomorph_plan.reachable_functions().contains(name) {
            continue;
        }

        match render_function(
            name,
            name,
            params,
            return_type,
            locals,
            *entry_block,
            blocks,
            None,
            &signatures,
            &layouts,
            &enum_layouts,
            &consts,
            &strings,
        ) {
            Ok(function_text) => module.push_str(&function_text),
            Err(reason) => {
                unsupported.push(diagnostic::AotLoweringDiagnostic::llvm_lowering(reason))
            }
        }
    }

    for specialization in monomorph_plan.used_concrete_instances() {
        let Some(FunctionSource {
            params,
            return_type,
            locals,
            entry_block,
            blocks,
            ..
        }) = find_function_source(program, &specialization.source_name)
        else {
            continue;
        };
        match render_function(
            &specialization.key,
            &specialization.source_name,
            params,
            return_type,
            locals,
            *entry_block,
            blocks,
            Some(&specialization.substitutions),
            &signatures,
            &layouts,
            &enum_layouts,
            &consts,
            &strings,
        ) {
            Ok(function_text) => module.push_str(&function_text),
            Err(reason) => {
                unsupported.push(diagnostic::AotLoweringDiagnostic::llvm_lowering(reason))
            }
        }
    }

    if unsupported.is_empty() {
        Ok(module)
    } else {
        Err(unsupported)
    }
}

fn render_function(
    signature_name: &str,
    source_name: &str,
    params: &[Param],
    return_type: &Type,
    locals: &[Local],
    entry_block: u32,
    blocks: &[BasicBlock],
    substitutions: Option<&BTreeMap<String, Type>>,
    signatures: &BTreeMap<String, FunctionSignature>,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    consts: &BTreeMap<String, ConstBinding>,
    strings: &BTreeMap<String, StringLiteral>,
) -> Result<String, String> {
    let signature = signatures.get(signature_name).ok_or_else(|| {
        format!("internal LLVM AOT error: missing signature for `{signature_name}`")
    })?;
    let params = substitutions
        .map(|substitutions| substitute_params(params, substitutions))
        .unwrap_or_else(|| params.to_vec());
    let locals = substitutions
        .map(|substitutions| substitute_locals(locals, substitutions))
        .unwrap_or_else(|| locals.to_vec());
    let return_type = substitutions
        .map(|substitutions| substitute_type_params(return_type, substitutions))
        .unwrap_or_else(|| return_type.clone());
    let declared_return_type = llvm_type(&return_type, layouts, enum_layouts)
        .ok_or_else(|| format!("function `{source_name}` returns an unsupported type"))?;
    if signature.return_type != declared_return_type {
        return Err(format!(
            "internal LLVM AOT error: function `{signature_name}` signature return type drifted"
        ));
    }

    let mut emitter = FunctionEmitter {
        signatures,
        layouts,
        enum_layouts,
        consts,
        strings,
        locals: BTreeMap::new(),
        return_ax_ty: return_type.clone(),
        const_stack: Vec::new(),
        temp_index: 0,
    };
    let local_type_overrides =
        infer_concrete_local_types(&locals, blocks, signatures, enum_layouts);

    for local in &locals {
        let ax_ty = local_type_overrides
            .get(&local.id)
            .cloned()
            .unwrap_or_else(|| local.ty.clone());
        let Some(ty) = llvm_type(&ax_ty, layouts, enum_layouts) else {
            return Err(format!(
                "function `{source_name}` local `{}` uses unsupported type {}",
                local.name,
                ax_type_name(&local.ty)
            ));
        };
        emitter.locals.insert(
            local.id,
            LocalSlot {
                ptr: format!("%local{}", local.id),
                ty,
                ax_ty,
            },
        );
    }

    let mut function = String::new();
    write!(
        function,
        "define {} @{}(",
        signature.return_type, signature.symbol
    )
    .expect("writing to string cannot fail");
    let native_main_with_argv = signature_name == "main" && params.is_empty();
    if native_main_with_argv {
        write!(function, "i32 %argc, ptr %argv").expect("writing to string cannot fail");
    } else {
        for (index, param) in params.iter().enumerate() {
            if index > 0 {
                write!(function, ", ").expect("writing to string cannot fail");
            }
            let Some(param_ty) = llvm_type(&param.ty, layouts, enum_layouts) else {
                return Err(format!(
                    "function `{source_name}` parameter `{}` uses unsupported type {}",
                    param.name,
                    ax_type_name(&param.ty)
                ));
            };
            write!(function, "{param_ty} %arg{}", param.local)
                .expect("writing to string cannot fail");
        }
    }
    writeln!(function, ") {{").expect("writing to string cannot fail");
    writeln!(function, "entry:").expect("writing to string cannot fail");
    if native_main_with_argv {
        writeln!(function, "  store i32 %argc, ptr @.ax_argc")
            .expect("writing to string cannot fail");
        writeln!(function, "  store ptr %argv, ptr @.ax_argv")
            .expect("writing to string cannot fail");
    }

    for local in &locals {
        let slot = emitter.local_slot(local.id)?;
        writeln!(function, "  {} = alloca {}", slot.ptr, slot.ty)
            .expect("writing to string cannot fail");
    }
    for param in &params {
        let slot = emitter.local_slot(param.local)?;
        writeln!(
            function,
            "  store {} %arg{}, ptr {}",
            slot.ty, param.local, slot.ptr
        )
        .expect("writing to string cannot fail");
    }
    writeln!(function, "  br label %bb{entry_block}").expect("writing to string cannot fail");

    for block in blocks {
        writeln!(function, "bb{}:", block.id).expect("writing to string cannot fail");
        for statement in &block.statements {
            emitter.emit_statement(statement, &mut function)?;
        }
        emitter.emit_terminator(block, &mut function)?;
    }

    writeln!(function, "}}\n").expect("writing to string cannot fail");
    Ok(function)
}

fn infer_concrete_local_types(
    locals: &[Local],
    blocks: &[BasicBlock],
    signatures: &BTreeMap<String, FunctionSignature>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> BTreeMap<u32, Type> {
    let local_types = locals
        .iter()
        .map(|local| (local.id, local.ty.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut overrides = BTreeMap::new();
    for block in blocks {
        for statement in &block.statements {
            infer_statement_concrete_local_types(
                statement,
                &local_types,
                signatures,
                enum_layouts,
                &mut overrides,
            );
        }
        infer_terminator_concrete_local_types(
            &block.terminator.kind,
            &local_types,
            signatures,
            enum_layouts,
            &mut overrides,
        );
        infer_match_test_branch_payload_types(
            &block.terminator.kind,
            blocks,
            &local_types,
            signatures,
            enum_layouts,
            &mut overrides,
        );
    }
    overrides
}

fn infer_statement_concrete_local_types(
    statement: &Statement,
    local_types: &BTreeMap<u32, Type>,
    signatures: &BTreeMap<String, FunctionSignature>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    overrides: &mut BTreeMap<u32, Type>,
) {
    match &statement.kind {
        StatementKind::Let {
            local, initializer, ..
        } => {
            infer_expr_concrete_local_types(
                initializer,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
            if let Some(inferred) =
                static_expr_type_with_signatures(initializer, local_types, overrides, signatures)
                && let Some(declared) = local_types.get(local)
                && should_override_local_type(declared, &inferred)
            {
                overrides.insert(*local, inferred);
            }
        }
        StatementKind::Eval { expr: initializer } => {
            infer_expr_concrete_local_types(
                initializer,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
        }
        StatementKind::Assign { value, .. } => {
            infer_expr_concrete_local_types(
                value,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
        }
    }
}

fn should_override_local_type(declared: &Type, inferred: &Type) -> bool {
    if declared == inferred {
        return false;
    }

    match (declared, inferred) {
        (Type::TypeParam { .. }, _) => true,
        (Type::Struct { name }, Type::StructInstance { name: inferred, .. })
        | (Type::Enum { name }, Type::EnumInstance { name: inferred, .. }) => name == inferred,
        (Type::StructInstance { name, args }, Type::StructInstance { name: inferred, .. })
        | (Type::EnumInstance { name, args }, Type::EnumInstance { name: inferred, .. }) => {
            name == inferred && args.iter().any(type_contains_type_param)
        }
        _ => false,
    }
}

fn type_contains_type_param(ty: &Type) -> bool {
    match ty {
        Type::TypeParam { .. } => true,
        Type::Slice { element } | Type::Array { element, .. } => type_contains_type_param(element),
        Type::StructInstance { args, .. } | Type::EnumInstance { args, .. } => {
            args.iter().any(type_contains_type_param)
        }
        Type::Bool
        | Type::I32
        | Type::F32
        | Type::String
        | Type::Bytes
        | Type::StringList
        | Type::Struct { .. }
        | Type::Enum { .. } => false,
    }
}

fn infer_terminator_concrete_local_types(
    terminator: &TerminatorKind,
    local_types: &BTreeMap<u32, Type>,
    signatures: &BTreeMap<String, FunctionSignature>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    overrides: &mut BTreeMap<u32, Type>,
) {
    match terminator {
        TerminatorKind::Branch { condition, .. } | TerminatorKind::Return { value: condition } => {
            infer_expr_concrete_local_types(
                condition,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
        }
        TerminatorKind::Goto { .. } | TerminatorKind::Unreachable => {}
    }
}

fn infer_match_test_branch_payload_types(
    terminator: &TerminatorKind,
    blocks: &[BasicBlock],
    local_types: &BTreeMap<u32, Type>,
    signatures: &BTreeMap<String, FunctionSignature>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    overrides: &mut BTreeMap<u32, Type>,
) {
    let TerminatorKind::Branch {
        condition,
        then_block,
        ..
    } = terminator
    else {
        return;
    };
    let ExprKind::MatchTest { scrutinee, pattern } = &condition.kind else {
        return;
    };
    let MatchPatternKind::EnumVariant {
        variant,
        payload: Some(EnumVariantPayloadPattern::Binding { name }),
        ..
    } = &pattern.kind
    else {
        return;
    };
    let Some(scrutinee_ty) =
        static_expr_type_with_signatures(scrutinee, local_types, overrides, signatures)
    else {
        return;
    };
    let Some(layout) = enum_layout_for_static_type(&scrutinee_ty, enum_layouts) else {
        return;
    };
    let Some(payload_ty) = layout
        .variants
        .iter()
        .find(|candidate| candidate.name == *variant)
        .and_then(|candidate| candidate.payload_ax_ty.clone())
    else {
        return;
    };
    let Some(target_block) = blocks.iter().find(|block| block.id == *then_block) else {
        return;
    };
    let Some(local) = find_local_declaration_by_name(target_block, name) else {
        return;
    };
    if let Some(declared) = local_types.get(&local)
        && should_override_local_type(declared, &payload_ty)
    {
        overrides.insert(local, payload_ty);
    }
}

fn find_local_declaration_by_name(block: &BasicBlock, name: &str) -> Option<u32> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Let {
                local,
                name: local_name,
                ..
            } if local_name == name => Some(*local),
            StatementKind::Let { .. }
            | StatementKind::Eval { .. }
            | StatementKind::Assign { .. } => None,
        })
}

fn infer_expr_concrete_local_types(
    expr: &Expr,
    local_types: &BTreeMap<u32, Type>,
    signatures: &BTreeMap<String, FunctionSignature>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    overrides: &mut BTreeMap<u32, Type>,
) {
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            infer_expr_concrete_local_types(
                scrutinee,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
            let Some(scrutinee_ty) =
                static_expr_type_with_signatures(scrutinee, local_types, overrides, signatures)
            else {
                for arm in arms {
                    infer_expr_concrete_local_types(
                        &arm.value,
                        local_types,
                        signatures,
                        enum_layouts,
                        overrides,
                    );
                }
                return;
            };
            let Some(layout) = enum_layout_for_static_type(&scrutinee_ty, enum_layouts) else {
                for arm in arms {
                    infer_expr_concrete_local_types(
                        &arm.value,
                        local_types,
                        signatures,
                        enum_layouts,
                        overrides,
                    );
                }
                return;
            };
            for arm in arms {
                if let MatchPatternKind::EnumVariant {
                    variant,
                    payload: Some(EnumVariantPayloadPattern::Binding { name }),
                    ..
                } = &arm.pattern.kind
                    && let Some(payload_ty) = layout
                        .variants
                        .iter()
                        .find(|candidate| candidate.name == *variant)
                        .and_then(|candidate| candidate.payload_ax_ty.clone())
                    && let Some(local) = find_local_use_by_name_in_match_arm(arm, name)
                    && matches!(
                        local_types.get(&local),
                        Some(Type::TypeParam { .. }) | Some(Type::EnumInstance { .. })
                    )
                {
                    overrides.insert(local, payload_ty);
                }
                if let Some(guard) = &arm.guard {
                    infer_expr_concrete_local_types(
                        guard,
                        local_types,
                        signatures,
                        enum_layouts,
                        overrides,
                    );
                }
                infer_expr_concrete_local_types(
                    &arm.value,
                    local_types,
                    signatures,
                    enum_layouts,
                    overrides,
                );
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Try { expr }
        | ExprKind::EnumPayload { value: expr }
        | ExprKind::Field { base: expr, .. } => {
            infer_expr_concrete_local_types(expr, local_types, signatures, enum_layouts, overrides);
        }
        ExprKind::Binary { left, right, .. } => {
            infer_expr_concrete_local_types(left, local_types, signatures, enum_layouts, overrides);
            infer_expr_concrete_local_types(
                right,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
        }
        ExprKind::Call { arguments, .. }
        | ExprKind::ArrayLiteral {
            elements: arguments,
        } => {
            for argument in arguments {
                infer_expr_concrete_local_types(
                    argument,
                    local_types,
                    signatures,
                    enum_layouts,
                    overrides,
                );
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                infer_expr_concrete_local_types(
                    &field.value,
                    local_types,
                    signatures,
                    enum_layouts,
                    overrides,
                );
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                infer_statement_concrete_local_types(
                    statement,
                    local_types,
                    signatures,
                    enum_layouts,
                    overrides,
                );
            }
            infer_expr_concrete_local_types(
                value,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
        }
        ExprKind::EnumVariant { payload, .. } => {
            if let Some(payload) = payload {
                infer_expr_concrete_local_types(
                    payload,
                    local_types,
                    signatures,
                    enum_layouts,
                    overrides,
                );
            }
        }
        ExprKind::MatchTest { scrutinee, .. } => {
            infer_expr_concrete_local_types(
                scrutinee,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
        }
        ExprKind::Index { base, index } => {
            infer_expr_concrete_local_types(base, local_types, signatures, enum_layouts, overrides);
            infer_expr_concrete_local_types(
                index,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
        }
        ExprKind::Slice { base, start, end } => {
            infer_expr_concrete_local_types(base, local_types, signatures, enum_layouts, overrides);
            infer_expr_concrete_local_types(
                start,
                local_types,
                signatures,
                enum_layouts,
                overrides,
            );
            infer_expr_concrete_local_types(end, local_types, signatures, enum_layouts, overrides);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Local { .. }
        | ExprKind::Const { .. } => {}
    }
}

fn static_expr_type(
    expr: &Expr,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
) -> Option<Type> {
    match &expr.kind {
        ExprKind::Local { local, .. } => overrides
            .get(local)
            .or_else(|| local_types.get(local))
            .cloned(),
        ExprKind::Int { .. } => Some(Type::I32),
        ExprKind::Float { .. } => Some(Type::F32),
        ExprKind::Bool { .. } => Some(Type::Bool),
        ExprKind::String { .. } => Some(Type::String),
        ExprKind::Block { value, .. } => static_expr_type(value, local_types, overrides),
        ExprKind::EnumVariant { enum_name, .. } => Some(Type::Enum {
            name: enum_name.clone(),
        }),
        _ => None,
    }
}

fn static_expr_type_with_signatures(
    expr: &Expr,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
    signatures: &BTreeMap<String, FunctionSignature>,
) -> Option<Type> {
    if let Some(ty) = static_expr_type(expr, local_types, overrides) {
        return Some(ty);
    }

    match &expr.kind {
        ExprKind::Call {
            function,
            arguments,
        } => {
            if function == "string_len" || function == "len" {
                return Some(Type::I32);
            }
            if matches!(
                function.as_str(),
                "string_contains" | "string_starts_with" | "string_ends_with"
            ) {
                return Some(Type::Bool);
            }
            if matches!(
                function.as_str(),
                "string_replace" | "string_trim" | "to_string"
            ) {
                return Some(Type::String);
            }
            if function == "string_split_lines" {
                return Some(Type::Slice {
                    element: Box::new(Type::String),
                });
            }
            if function == "fs_read_dir" {
                return Some(Type::Slice {
                    element: Box::new(Type::String),
                });
            }
            if matches!(
                function.as_str(),
                "path_join"
                    | "path_parent"
                    | "path_resolve"
                    | "path_file_name"
                    | "path_stem"
                    | "path_extension"
            ) {
                return Some(Type::String);
            }
            if function == "path_is_absolute" {
                return Some(Type::Bool);
            }
            if let Some(signature) = signatures.get(function) {
                return Some(signature.return_ax_type.clone());
            }

            let mut argument_ax_types = Vec::new();
            for argument in arguments {
                let argument_ty =
                    static_expr_type_with_signatures(argument, local_types, overrides, signatures)?;
                argument_ax_types.push(argument_ty);
            }
            let prefix = format!("{function}<");
            let mut candidates = signatures.iter().filter(|(name, signature)| {
                name.starts_with(&prefix) && signature.param_ax_types == argument_ax_types
            });
            let (_, signature) = candidates.next()?;
            if candidates.next().is_some() {
                return None;
            }
            Some(signature.return_ax_type.clone())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
