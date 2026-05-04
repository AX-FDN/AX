use std::collections::BTreeSet;

use super::*;

#[derive(Clone)]
pub(super) struct FunctionSpecialization {
    pub(super) key: String,
    pub(super) source_name: String,
    pub(super) substitutions: BTreeMap<String, Type>,
}

pub(super) struct FunctionSource<'a> {
    pub(super) type_params: &'a [String],
    pub(super) params: &'a [Param],
    pub(super) return_type: &'a Type,
    pub(super) locals: &'a [Local],
    pub(super) entry_block: &'a u32,
    pub(super) blocks: &'a [BasicBlock],
}

pub(super) fn find_function_source<'a>(
    program: &'a Program,
    target: &str,
) -> Option<FunctionSource<'a>> {
    program.items.iter().find_map(|item| {
        let ItemKind::Function {
            name,
            type_params,
            type_param_bounds: _,
            params,
            return_type,
            locals,
            entry_block,
            blocks,
        } = &item.kind
        else {
            return None;
        };
        if name == target {
            Some(FunctionSource {
                type_params,
                params,
                return_type,
                locals,
                entry_block,
                blocks,
            })
        } else {
            None
        }
    })
}

pub(super) fn substitute_params(
    params: &[Param],
    substitutions: &BTreeMap<String, Type>,
) -> Vec<Param> {
    params
        .iter()
        .map(|param| Param {
            local: param.local,
            name: param.name.clone(),
            ty: substitute_type_params(&param.ty, substitutions),
            span: param.span,
        })
        .collect()
}

pub(super) fn substitute_locals(
    locals: &[Local],
    substitutions: &BTreeMap<String, Type>,
) -> Vec<Local> {
    locals
        .iter()
        .map(|local| Local {
            id: local.id,
            kind: local.kind.clone(),
            name: local.name.clone(),
            ty: substitute_type_params(&local.ty, substitutions),
            mutable: local.mutable,
            span: local.span,
        })
        .collect()
}

pub(super) fn collect_function_specializations(
    program: &Program,
) -> Result<BTreeMap<String, FunctionSpecialization>, Vec<String>> {
    let mut specializations = BTreeMap::new();
    let mut unsupported = Vec::new();

    for item in &program.items {
        let ItemKind::Function {
            type_params,
            type_param_bounds,
            params,
            return_type,
            locals,
            blocks,
            ..
        } = &item.kind
        else {
            continue;
        };
        if !type_params.is_empty() || !type_param_bounds.is_empty() {
            continue;
        }
        collect_function_body_specializations(
            program,
            params,
            return_type,
            locals,
            blocks,
            None,
            &mut specializations,
            &mut unsupported,
        );
    }

    let mut scanned_specializations = BTreeSet::new();
    loop {
        let pending = specializations
            .values()
            .filter(|specialization| !scanned_specializations.contains(&specialization.key))
            .cloned()
            .collect::<Vec<_>>();
        if pending.is_empty() {
            break;
        }

        for specialization in pending {
            scanned_specializations.insert(specialization.key.clone());
            let Some(FunctionSource {
                params,
                return_type,
                locals,
                blocks,
                ..
            }) = find_function_source(program, &specialization.source_name)
            else {
                unsupported.push(format!(
                    "generic function specialization `{}` has no source function `{}`",
                    specialization.key, specialization.source_name
                ));
                continue;
            };
            collect_function_body_specializations(
                program,
                params,
                return_type,
                locals,
                blocks,
                Some(&specialization.substitutions),
                &mut specializations,
                &mut unsupported,
            );
        }
    }

    if unsupported.is_empty() {
        Ok(specializations)
    } else {
        Err(unsupported)
    }
}

fn collect_function_body_specializations(
    program: &Program,
    params: &[Param],
    return_type: &Type,
    locals: &[Local],
    blocks: &[BasicBlock],
    substitutions: Option<&BTreeMap<String, Type>>,
    specializations: &mut BTreeMap<String, FunctionSpecialization>,
    unsupported: &mut Vec<String>,
) {
    let params = substitutions
        .map(|substitutions| substitute_params(params, substitutions))
        .unwrap_or_else(|| params.to_vec());
    let locals = substitutions
        .map(|substitutions| substitute_locals(locals, substitutions))
        .unwrap_or_else(|| locals.to_vec());
    let return_type = substitutions
        .map(|substitutions| substitute_type_params(return_type, substitutions))
        .unwrap_or_else(|| return_type.clone());
    let local_types = function_static_local_types(&params, &locals);
    let overrides = BTreeMap::new();
    for block in blocks {
        for statement in &block.statements {
            collect_statement_function_specializations(
                program,
                statement,
                &local_types,
                &overrides,
                specializations,
                unsupported,
            );
        }
        collect_terminator_function_specializations(
            program,
            &block.terminator.kind,
            &return_type,
            &local_types,
            &overrides,
            specializations,
            unsupported,
        );
    }
}

fn function_static_local_types(params: &[Param], locals: &[Local]) -> BTreeMap<u32, Type> {
    let mut local_types = BTreeMap::new();
    for param in params {
        local_types.insert(param.local, param.ty.clone());
    }
    for local in locals {
        local_types.insert(local.id, local.ty.clone());
    }
    local_types
}

fn collect_statement_function_specializations(
    program: &Program,
    statement: &Statement,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
    specializations: &mut BTreeMap<String, FunctionSpecialization>,
    unsupported: &mut Vec<String>,
) {
    match &statement.kind {
        StatementKind::Let {
            local, initializer, ..
        } => {
            let expected_type = local_types.get(local);
            collect_expr_function_specializations(
                program,
                initializer,
                expected_type,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
        StatementKind::Eval { expr } => {
            collect_expr_function_specializations(
                program,
                expr,
                None,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
        StatementKind::Assign { target, value } => {
            collect_place_function_specializations(
                program,
                target,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
            collect_expr_function_specializations(
                program,
                value,
                None,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
    }
}

fn collect_terminator_function_specializations(
    program: &Program,
    terminator: &TerminatorKind,
    return_type: &Type,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
    specializations: &mut BTreeMap<String, FunctionSpecialization>,
    unsupported: &mut Vec<String>,
) {
    match terminator {
        TerminatorKind::Branch { condition, .. } => {
            collect_expr_function_specializations(
                program,
                condition,
                Some(&Type::Bool),
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
        TerminatorKind::Return { value } => {
            collect_expr_function_specializations(
                program,
                value,
                Some(return_type),
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
        TerminatorKind::Goto { .. } | TerminatorKind::Unreachable => {}
    }
}

fn collect_place_function_specializations(
    program: &Program,
    place: &Place,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
    specializations: &mut BTreeMap<String, FunctionSpecialization>,
    unsupported: &mut Vec<String>,
) {
    match &place.kind {
        PlaceKind::Local { .. } => {}
        PlaceKind::Field { base, .. } => collect_place_function_specializations(
            program,
            base,
            local_types,
            overrides,
            specializations,
            unsupported,
        ),
        PlaceKind::Index { base, index } => {
            collect_place_function_specializations(
                program,
                base,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
            collect_expr_function_specializations(
                program,
                index,
                None,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
    }
}

fn collect_expr_function_specializations(
    program: &Program,
    expr: &Expr,
    expected_type: Option<&Type>,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
    specializations: &mut BTreeMap<String, FunctionSpecialization>,
    unsupported: &mut Vec<String>,
) {
    match &expr.kind {
        ExprKind::Call {
            function,
            arguments,
        } => {
            if let Some(method) = function.strip_prefix("<method>.") {
                collect_method_specialization(
                    program,
                    method,
                    arguments,
                    expected_type,
                    local_types,
                    overrides,
                    specializations,
                    unsupported,
                );
            } else {
                collect_direct_function_specialization(
                    program,
                    function,
                    arguments,
                    expected_type,
                    local_types,
                    overrides,
                    specializations,
                    unsupported,
                );
            }
            for argument in arguments {
                collect_expr_function_specializations(
                    program,
                    argument,
                    None,
                    local_types,
                    overrides,
                    specializations,
                    unsupported,
                );
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Try { expr }
        | ExprKind::EnumPayload { value: expr }
        | ExprKind::Field { base: expr, .. } => collect_expr_function_specializations(
            program,
            expr,
            None,
            local_types,
            overrides,
            specializations,
            unsupported,
        ),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_function_specializations(
                program,
                left,
                None,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
            collect_expr_function_specializations(
                program,
                right,
                None,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
        ExprKind::ArrayLiteral { elements } => {
            for element in elements {
                collect_expr_function_specializations(
                    program,
                    element,
                    None,
                    local_types,
                    overrides,
                    specializations,
                    unsupported,
                );
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_expr_function_specializations(
                    program,
                    &field.value,
                    None,
                    local_types,
                    overrides,
                    specializations,
                    unsupported,
                );
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_statement_function_specializations(
                    program,
                    statement,
                    local_types,
                    overrides,
                    specializations,
                    unsupported,
                );
            }
            collect_expr_function_specializations(
                program,
                value,
                expected_type,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_function_specializations(
                program,
                scrutinee,
                None,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_expr_function_specializations(
                        program,
                        guard,
                        Some(&Type::Bool),
                        local_types,
                        overrides,
                        specializations,
                        unsupported,
                    );
                }
                collect_expr_function_specializations(
                    program,
                    &arm.value,
                    expected_type,
                    local_types,
                    overrides,
                    specializations,
                    unsupported,
                );
            }
        }
        ExprKind::EnumVariant { payload, .. } => {
            if let Some(payload) = payload {
                collect_expr_function_specializations(
                    program,
                    payload,
                    None,
                    local_types,
                    overrides,
                    specializations,
                    unsupported,
                );
            }
        }
        ExprKind::MatchTest { scrutinee, .. } => collect_expr_function_specializations(
            program,
            scrutinee,
            None,
            local_types,
            overrides,
            specializations,
            unsupported,
        ),
        ExprKind::Index { base, index } => {
            collect_expr_function_specializations(
                program,
                base,
                None,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
            collect_expr_function_specializations(
                program,
                index,
                Some(&Type::I32),
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
        ExprKind::Slice { base, start, end } => {
            collect_expr_function_specializations(
                program,
                base,
                None,
                local_types,
                overrides,
                specializations,
                unsupported,
            );
            collect_expr_function_specializations(
                program,
                start,
                Some(&Type::I32),
                local_types,
                overrides,
                specializations,
                unsupported,
            );
            collect_expr_function_specializations(
                program,
                end,
                Some(&Type::I32),
                local_types,
                overrides,
                specializations,
                unsupported,
            );
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Local { .. }
        | ExprKind::Const { .. } => {}
    }
}

fn collect_direct_function_specialization(
    program: &Program,
    function: &str,
    arguments: &[Expr],
    expected_type: Option<&Type>,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
    specializations: &mut BTreeMap<String, FunctionSpecialization>,
    unsupported: &mut Vec<String>,
) {
    let Some(source) = find_function_source(program, function) else {
        return;
    };
    collect_generic_function_specialization(
        function,
        &source,
        arguments,
        expected_type,
        local_types,
        overrides,
        specializations,
        unsupported,
        "function",
    );
}

fn collect_method_specialization(
    program: &Program,
    method: &str,
    arguments: &[Expr],
    expected_type: Option<&Type>,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
    specializations: &mut BTreeMap<String, FunctionSpecialization>,
    unsupported: &mut Vec<String>,
) {
    let Some(receiver) = arguments.first() else {
        return;
    };
    let Some(receiver_ty) = static_expr_type(receiver, local_types, overrides) else {
        return;
    };
    let Some(method_function) = method_function_name(method, &receiver_ty) else {
        return;
    };
    let Some(source) = find_function_source(program, &method_function) else {
        return;
    };
    collect_generic_function_specialization(
        &method_function,
        &source,
        arguments,
        expected_type,
        local_types,
        overrides,
        specializations,
        unsupported,
        "method",
    );
}

fn collect_generic_function_specialization(
    function_name: &str,
    source: &FunctionSource<'_>,
    arguments: &[Expr],
    expected_type: Option<&Type>,
    local_types: &BTreeMap<u32, Type>,
    overrides: &BTreeMap<u32, Type>,
    specializations: &mut BTreeMap<String, FunctionSpecialization>,
    unsupported: &mut Vec<String>,
    label: &str,
) {
    if source.type_params.is_empty() {
        return;
    }
    if source.params.len() != arguments.len() {
        unsupported.push(format!(
            "{label} `{function_name}` has {} argument(s), but LLVM AOT saw {}",
            source.params.len(),
            arguments.len()
        ));
        return;
    }

    let mut substitutions = BTreeMap::new();
    for (param, argument) in source.params.iter().zip(arguments) {
        let Some(argument_ty) = static_expr_type(argument, local_types, overrides) else {
            continue;
        };
        if let Err(reason) = bind_type_params(&param.ty, &argument_ty, &mut substitutions) {
            unsupported.push(format!(
                "generic {label} `{function_name}` specialization failed: {reason}"
            ));
            return;
        }
    }
    if let Some(expected_type) = expected_type
        && let Err(reason) =
            bind_type_params(&source.return_type, expected_type, &mut substitutions)
    {
        unsupported.push(format!(
            "generic {label} `{function_name}` expected return type specialization failed: {reason}"
        ));
        return;
    }
    if source
        .type_params
        .iter()
        .any(|param| !substitutions.contains_key(param))
    {
        unsupported.push(format!(
            "generic {label} `{function_name}` needs type arguments that LLVM AOT v0 could not infer"
        ));
        return;
    }

    let key = function_specialization_key(function_name, source.type_params, &substitutions);
    specializations
        .entry(key.clone())
        .or_insert(FunctionSpecialization {
            key,
            source_name: function_name.to_string(),
            substitutions,
        });
}

pub(super) fn method_function_name(method: &str, receiver_ty: &Type) -> Option<String> {
    match receiver_ty {
        Type::Struct { name }
        | Type::StructInstance { name, .. }
        | Type::Enum { name }
        | Type::EnumInstance { name, .. } => Some(format!("{name}.{method}")),
        _ => None,
    }
}

fn bind_type_params(
    pattern: &Type,
    concrete: &Type,
    substitutions: &mut BTreeMap<String, Type>,
) -> Result<(), String> {
    match pattern {
        Type::TypeParam { name } => {
            if let Some(existing) = substitutions.get(name) {
                if existing == concrete {
                    Ok(())
                } else {
                    Err(format!(
                        "type parameter `{name}` was inferred as both {} and {}",
                        ax_type_name(existing),
                        ax_type_name(concrete)
                    ))
                }
            } else {
                substitutions.insert(name.clone(), concrete.clone());
                Ok(())
            }
        }
        Type::Array { element, length } => {
            let Type::Array {
                element: concrete_element,
                length: concrete_length,
            } = concrete
            else {
                return Err(format!(
                    "expected {}, found {}",
                    ax_type_name(pattern),
                    ax_type_name(concrete)
                ));
            };
            if length != concrete_length {
                return Err(format!(
                    "expected {}, found {}",
                    ax_type_name(pattern),
                    ax_type_name(concrete)
                ));
            }
            bind_type_params(element, concrete_element, substitutions)
        }
        Type::Slice { element } => {
            let Type::Slice {
                element: concrete_element,
            } = concrete
            else {
                return Err(format!(
                    "expected {}, found {}",
                    ax_type_name(pattern),
                    ax_type_name(concrete)
                ));
            };
            bind_type_params(element, concrete_element, substitutions)
        }
        Type::StructInstance { name, args } => {
            let Type::StructInstance {
                name: concrete_name,
                args: concrete_args,
            } = concrete
            else {
                return Err(format!(
                    "expected {}, found {}",
                    ax_type_name(pattern),
                    ax_type_name(concrete)
                ));
            };
            if name != concrete_name || args.len() != concrete_args.len() {
                return Err(format!(
                    "expected {}, found {}",
                    ax_type_name(pattern),
                    ax_type_name(concrete)
                ));
            }
            for (arg, concrete_arg) in args.iter().zip(concrete_args) {
                bind_type_params(arg, concrete_arg, substitutions)?;
            }
            Ok(())
        }
        Type::EnumInstance { name, args } => {
            let Type::EnumInstance {
                name: concrete_name,
                args: concrete_args,
            } = concrete
            else {
                return Err(format!(
                    "expected {}, found {}",
                    ax_type_name(pattern),
                    ax_type_name(concrete)
                ));
            };
            if name != concrete_name || args.len() != concrete_args.len() {
                return Err(format!(
                    "expected {}, found {}",
                    ax_type_name(pattern),
                    ax_type_name(concrete)
                ));
            }
            for (arg, concrete_arg) in args.iter().zip(concrete_args) {
                bind_type_params(arg, concrete_arg, substitutions)?;
            }
            Ok(())
        }
        _ if pattern == concrete => Ok(()),
        _ => Err(format!(
            "expected {}, found {}",
            ax_type_name(pattern),
            ax_type_name(concrete)
        )),
    }
}

fn function_specialization_key(
    function: &str,
    type_params: &[String],
    substitutions: &BTreeMap<String, Type>,
) -> String {
    let args = type_params
        .iter()
        .filter_map(|param| substitutions.get(param))
        .map(ax_type_name)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{function}<{args}>")
}
