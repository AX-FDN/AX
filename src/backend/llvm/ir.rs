use std::collections::BTreeMap;
use std::fmt::Write;

use crate::hir::{EnumVariant, EnumVariantPayloadPattern, StructField};
use crate::mir::{
    BasicBlock, BinaryOp, Expr, ExprKind, ItemKind, Local, MatchExprArm, MatchPattern,
    MatchPatternKind, Place, PlaceKind, Program, Statement, StatementKind, StructLiteralField,
    TerminatorKind, Type, UnaryOp,
};

#[derive(Clone)]
struct FunctionSignature {
    symbol: String,
    params: Vec<String>,
    param_ax_types: Vec<Type>,
    return_type: String,
    return_ax_type: Type,
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
    let mut unsupported = Vec::new();
    let mut signatures = BTreeMap::new();
    let enum_layouts = match collect_enum_layouts(program) {
        Ok(layouts) => layouts,
        Err(reasons) => {
            unsupported.extend(reasons);
            BTreeMap::new()
        }
    };
    let layouts = match collect_struct_layouts(program, &enum_layouts) {
        Ok(layouts) => layouts,
        Err(reasons) => {
            unsupported.extend(reasons);
            BTreeMap::new()
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
                if !type_params.is_empty() || !type_param_bounds.is_empty() {
                    unsupported.push(format!(
                        "function `{name}` uses generics or trait bounds, which LLVM AOT v0 does not lower"
                    ));
                    continue;
                }

                let mut lowered_params = Vec::new();
                for param in params {
                    match llvm_type(&param.ty, &layouts, &enum_layouts) {
                        Some(ty) => lowered_params.push(ty),
                        None => unsupported.push(format!(
                            "function `{name}` parameter `{}` uses unsupported type {}",
                            param.name,
                            ax_type_name(&param.ty)
                        )),
                    }
                }

                let return_ax_type = return_type.clone();
                let Some(lowered_return_type) = llvm_type(return_type, &layouts, &enum_layouts)
                else {
                    unsupported.push(format!(
                        "function `{name}` returns unsupported type {}",
                        ax_type_name(return_type)
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
                    unsupported.push(format!(
                        "top-level const `{name}` uses unsupported type {}",
                        ax_type_name(ty)
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

    if !unsupported.is_empty() {
        return Err(unsupported);
    }

    if !signatures.contains_key("main") {
        return Err(vec![
            "LLVM AOT v0 requires an explicit `fn main() -> i32` entrypoint".to_string(),
        ]);
    }

    let strings = collect_string_literals(program);
    let mut module = String::new();
    writeln!(module, "; generated by axc LLVM AOT v0").expect("writing to string cannot fail");
    writeln!(module, "source_filename = \"axc\"").expect("writing to string cannot fail");
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
    writeln!(
        module,
        "@.ax_fmt_i32 = private unnamed_addr constant [3 x i8] c\"%d\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_fmt_str = private unnamed_addr constant [3 x i8] c\"%s\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_text_true = private unnamed_addr constant [5 x i8] c\"true\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_text_false = private unnamed_addr constant [6 x i8] c\"false\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_text_space = private unnamed_addr constant [2 x i8] c\" \\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_text_newline = private unnamed_addr constant [2 x i8] c\"\\0A\\00\""
    )
    .expect("writing to string cannot fail");
    for literal in strings.values() {
        writeln!(
            module,
            "{} = private unnamed_addr constant [{} x i8] c\"{}\"",
            literal.symbol, literal.len, literal.encoded
        )
        .expect("writing to string cannot fail");
    }
    writeln!(module, "declare i32 @printf(ptr, ...)").expect("writing to string cannot fail");
    writeln!(module, "declare void @exit(i32)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @strcmp(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @strstr(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @strncmp(ptr, ptr, i64)").expect("writing to string cannot fail");
    writeln!(module, "declare i64 @strlen(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @malloc(i64)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @memcpy(ptr, ptr, i64)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @snprintf(ptr, i64, ptr, ...)")
        .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    write_string_len_helper(&mut module);
    write_string_runtime_helpers(&mut module);

    for item in &program.items {
        let ItemKind::Function {
            name,
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

        match render_function(
            name,
            params,
            return_type,
            locals,
            *entry_block,
            blocks,
            &signatures,
            &layouts,
            &enum_layouts,
            &consts,
            &strings,
        ) {
            Ok(function_text) => module.push_str(&function_text),
            Err(reason) => unsupported.push(reason),
        }
    }

    if unsupported.is_empty() {
        Ok(module)
    } else {
        Err(unsupported)
    }
}

fn collect_struct_layouts(
    program: &Program,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Result<BTreeMap<String, StructLayout>, Vec<String>> {
    let mut layouts = BTreeMap::new();
    let mut unsupported = Vec::new();

    for item in &program.items {
        let ItemKind::Struct {
            name, type_params, ..
        } = &item.kind
        else {
            continue;
        };

        if !type_params.is_empty() {
            unsupported.push(format!(
                "generic struct `{name}` needs monomorphized native layout before LLVM AOT can lower it"
            ));
            continue;
        }

        layouts.insert(
            name.clone(),
            StructLayout {
                name: name.clone(),
                ty: llvm_struct_type_name(name),
                fields: Vec::new(),
            },
        );
    }

    for item in &program.items {
        let ItemKind::Struct {
            name,
            type_params,
            fields,
        } = &item.kind
        else {
            continue;
        };

        if !type_params.is_empty() {
            continue;
        }

        match lower_struct_fields(name, fields, &layouts, enum_layouts) {
            Ok(field_layouts) => {
                if let Some(layout) = layouts.get_mut(name) {
                    layout.fields = field_layouts;
                }
            }
            Err(reason) => unsupported.push(reason),
        }
    }

    if unsupported.is_empty() {
        Ok(layouts)
    } else {
        Err(unsupported)
    }
}

fn collect_enum_layouts(program: &Program) -> Result<BTreeMap<String, EnumLayout>, Vec<String>> {
    let mut layouts = BTreeMap::new();
    let mut unsupported = Vec::new();
    let mut definitions = BTreeMap::new();

    for item in &program.items {
        let ItemKind::Enum {
            name,
            type_params,
            variants,
        } = &item.kind
        else {
            continue;
        };

        definitions.insert(name.clone(), (type_params.clone(), variants.clone()));

        if type_params.is_empty() {
            match lower_enum_variants(name, variants) {
                Ok(variants) => {
                    insert_enum_layout(&mut layouts, Type::Enum { name: name.clone() }, variants);
                }
                Err(reason) => unsupported.push(reason),
            }
        }
    }

    let mut enum_instances = BTreeMap::new();
    collect_enum_instance_types(program, &mut enum_instances);
    for enum_ty in enum_instances.values() {
        let Type::EnumInstance { name, args } = enum_ty else {
            continue;
        };
        let Some((type_params, variants)) = definitions.get(name) else {
            unsupported.push(format!(
                "enum instance {} has no enum definition in LLVM AOT v0",
                ax_type_name(enum_ty)
            ));
            continue;
        };
        if type_params.len() != args.len() {
            unsupported.push(format!(
                "enum instance {} has {} type argument(s), but enum `{name}` declares {}",
                ax_type_name(enum_ty),
                args.len(),
                type_params.len()
            ));
            continue;
        }
        let substitutions = type_params
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect::<BTreeMap<_, _>>();
        let concrete_variants = variants
            .iter()
            .map(|variant| EnumVariant {
                name: variant.name.clone(),
                payload: variant
                    .payload
                    .as_ref()
                    .map(|payload| substitute_type_params(payload, &substitutions)),
                span: variant.span,
            })
            .collect::<Vec<_>>();
        match lower_enum_variants(name, &concrete_variants) {
            Ok(variants) => insert_enum_layout(&mut layouts, enum_ty.clone(), variants),
            Err(reason) => unsupported.push(reason),
        }
    }

    if unsupported.is_empty() {
        Ok(layouts)
    } else {
        Err(unsupported)
    }
}

fn insert_enum_layout(
    layouts: &mut BTreeMap<String, EnumLayout>,
    enum_ty: Type,
    variants: Vec<EnumVariantLayout>,
) {
    let has_payload = variants
        .iter()
        .any(|variant| variant.payload_ax_ty.is_some());
    let (Type::Enum { name } | Type::EnumInstance { name, .. }) = &enum_ty else {
        return;
    };
    layouts.insert(
        enum_layout_key(&enum_ty),
        EnumLayout {
            name: name.clone(),
            ax_ty: enum_ty.clone(),
            ty: if has_payload {
                llvm_enum_type_name_for_type(&enum_ty)
            } else {
                "i32".to_string()
            },
            variants,
        },
    );
}

fn lower_enum_variants(
    enum_name: &str,
    variants: &[EnumVariant],
) -> Result<Vec<EnumVariantLayout>, String> {
    let mut lowered = Vec::new();
    for (index, variant) in variants.iter().enumerate() {
        let tag = i32::try_from(index).map_err(|_| {
            format!("enum `{enum_name}` has too many variants for LLVM AOT v0 i32 tags")
        })?;
        lowered.push(EnumVariantLayout {
            name: variant.name.clone(),
            tag,
            payload_ax_ty: variant.payload.clone(),
        });
    }
    Ok(lowered)
}

fn collect_enum_instance_types(program: &Program, instances: &mut BTreeMap<String, Type>) {
    for item in &program.items {
        match &item.kind {
            ItemKind::Function {
                params,
                return_type,
                locals,
                blocks,
                ..
            } => {
                for param in params {
                    collect_enum_instance_type(&param.ty, instances);
                }
                collect_enum_instance_type(return_type, instances);
                for local in locals {
                    collect_enum_instance_type(&local.ty, instances);
                }
                for block in blocks {
                    for statement in &block.statements {
                        collect_statement_enum_instance_types(statement, instances);
                    }
                    collect_terminator_enum_instance_types(&block.terminator.kind, instances);
                }
            }
            ItemKind::Struct { fields, .. } => {
                for field in fields {
                    collect_enum_instance_type(&field.ty, instances);
                }
            }
            ItemKind::Enum { variants, .. } => {
                for variant in variants {
                    if let Some(payload) = &variant.payload {
                        collect_enum_instance_type(payload, instances);
                    }
                }
            }
            ItemKind::Const { ty, .. } => collect_enum_instance_type(ty, instances),
        }
    }
}

fn collect_statement_enum_instance_types(
    statement: &Statement,
    instances: &mut BTreeMap<String, Type>,
) {
    match &statement.kind {
        StatementKind::Let {
            ty, initializer, ..
        } => {
            collect_enum_instance_type(ty, instances);
            collect_expr_enum_instance_types(initializer, instances);
        }
        StatementKind::Assign { target, value } => {
            collect_place_enum_instance_types(target, instances);
            collect_expr_enum_instance_types(value, instances);
        }
        StatementKind::Eval { expr } => collect_expr_enum_instance_types(expr, instances),
    }
}

fn collect_terminator_enum_instance_types(
    terminator: &TerminatorKind,
    instances: &mut BTreeMap<String, Type>,
) {
    match terminator {
        TerminatorKind::Branch { condition, .. } | TerminatorKind::Return { value: condition } => {
            collect_expr_enum_instance_types(condition, instances);
        }
        TerminatorKind::Goto { .. } | TerminatorKind::Unreachable => {}
    }
}

fn collect_place_enum_instance_types(place: &Place, instances: &mut BTreeMap<String, Type>) {
    match &place.kind {
        PlaceKind::Local { .. } => {}
        PlaceKind::Field { base, .. } => collect_place_enum_instance_types(base, instances),
        PlaceKind::Index { base, index } => {
            collect_place_enum_instance_types(base, instances);
            collect_expr_enum_instance_types(index, instances);
        }
    }
}

fn collect_expr_enum_instance_types(expr: &Expr, instances: &mut BTreeMap<String, Type>) {
    match &expr.kind {
        ExprKind::Unary { expr, .. }
        | ExprKind::Try { expr }
        | ExprKind::EnumPayload { value: expr }
        | ExprKind::Field { base: expr, .. } => collect_expr_enum_instance_types(expr, instances),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_enum_instance_types(left, instances);
            collect_expr_enum_instance_types(right, instances);
        }
        ExprKind::Call { arguments, .. }
        | ExprKind::ArrayLiteral {
            elements: arguments,
        } => {
            for argument in arguments {
                collect_expr_enum_instance_types(argument, instances);
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_expr_enum_instance_types(&field.value, instances);
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_statement_enum_instance_types(statement, instances);
            }
            collect_expr_enum_instance_types(value, instances);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_enum_instance_types(scrutinee, instances);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_expr_enum_instance_types(guard, instances);
                }
                collect_match_pattern_enum_instance_types(&arm.pattern, instances);
                collect_expr_enum_instance_types(&arm.value, instances);
            }
        }
        ExprKind::EnumVariant { payload, .. } => {
            if let Some(payload) = payload {
                collect_expr_enum_instance_types(payload, instances);
            }
        }
        ExprKind::MatchTest { scrutinee, pattern } => {
            collect_expr_enum_instance_types(scrutinee, instances);
            collect_match_pattern_enum_instance_types(pattern, instances);
        }
        ExprKind::Index { base, index } => {
            collect_expr_enum_instance_types(base, instances);
            collect_expr_enum_instance_types(index, instances);
        }
        ExprKind::Slice { base, start, end } => {
            collect_expr_enum_instance_types(base, instances);
            collect_expr_enum_instance_types(start, instances);
            collect_expr_enum_instance_types(end, instances);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Local { .. }
        | ExprKind::Const { .. } => {}
    }
}

fn collect_match_pattern_enum_instance_types(
    pattern: &MatchPattern,
    instances: &mut BTreeMap<String, Type>,
) {
    match &pattern.kind {
        MatchPatternKind::EnumVariant { payload_type, .. } => {
            if let Some(payload_type) = payload_type {
                collect_enum_instance_type(payload_type, instances);
            }
        }
        MatchPatternKind::Struct { fields, .. } => {
            for field in fields {
                collect_enum_instance_type(&field.ty, instances);
            }
        }
        MatchPatternKind::Or { alternatives } => {
            for alternative in alternatives {
                collect_match_pattern_enum_instance_types(alternative, instances);
            }
        }
        MatchPatternKind::Wildcard
        | MatchPatternKind::Binding { .. }
        | MatchPatternKind::Bool { .. }
        | MatchPatternKind::Int { .. }
        | MatchPatternKind::IntRange { .. }
        | MatchPatternKind::String { .. }
        | MatchPatternKind::Error => {}
    }
}

fn collect_enum_instance_type(ty: &Type, instances: &mut BTreeMap<String, Type>) {
    match ty {
        Type::EnumInstance { args, .. } => {
            instances.insert(enum_layout_key(ty), ty.clone());
            for arg in args {
                collect_enum_instance_type(arg, instances);
            }
        }
        Type::StructInstance { args, .. } => {
            for arg in args {
                collect_enum_instance_type(arg, instances);
            }
        }
        Type::Slice { element } | Type::Array { element, .. } => {
            collect_enum_instance_type(element, instances);
        }
        Type::Bool
        | Type::I32
        | Type::F32
        | Type::String
        | Type::StringList
        | Type::Struct { .. }
        | Type::Enum { .. }
        | Type::TypeParam { .. } => {}
    }
}

fn substitute_type_params(ty: &Type, substitutions: &BTreeMap<String, Type>) -> Type {
    match ty {
        Type::TypeParam { name } => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Slice { element } => Type::Slice {
            element: Box::new(substitute_type_params(element, substitutions)),
        },
        Type::Array { element, length } => Type::Array {
            element: Box::new(substitute_type_params(element, substitutions)),
            length: *length,
        },
        Type::StructInstance { name, args } => Type::StructInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_params(arg, substitutions))
                .collect(),
        },
        Type::EnumInstance { name, args } => Type::EnumInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_params(arg, substitutions))
                .collect(),
        },
        _ => ty.clone(),
    }
}

fn lower_struct_fields(
    struct_name: &str,
    fields: &[StructField],
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Result<Vec<StructFieldLayout>, String> {
    let mut lowered = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let Some(ty) = llvm_type(&field.ty, layouts, enum_layouts) else {
            return Err(format!(
                "struct `{struct_name}` field `{}` uses unsupported type {}",
                field.name,
                ax_type_name(&field.ty)
            ));
        };
        lowered.push(StructFieldLayout {
            name: field.name.clone(),
            index,
            ty,
            ax_ty: field.ty.clone(),
        });
    }
    Ok(lowered)
}

fn write_string_len_helper(module: &mut String) {
    writeln!(module, "define private i32 @ax_string_len(ptr %text) {{")
        .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  br label %loop").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "loop:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %index = phi i32 [0, %entry], [%next_index, %body]"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %count = phi i32 [0, %entry], [%next_count, %body]"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %char_ptr = getelementptr i8, ptr %text, i32 %index"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %byte = load i8, ptr %char_ptr").expect("writing to string cannot fail");
    writeln!(module, "  %is_end = icmp eq i8 %byte, 0").expect("writing to string cannot fail");
    writeln!(module, "  br i1 %is_end, label %done, label %body")
        .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "body:").expect("writing to string cannot fail");
    writeln!(module, "  %prefix = and i8 %byte, -64").expect("writing to string cannot fail");
    writeln!(module, "  %is_continuation = icmp eq i8 %prefix, -128")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %starts_codepoint = xor i1 %is_continuation, true"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %delta = zext i1 %starts_codepoint to i32")
        .expect("writing to string cannot fail");
    writeln!(module, "  %next_count = add i32 %count, %delta")
        .expect("writing to string cannot fail");
    writeln!(module, "  %next_index = add i32 %index, 1").expect("writing to string cannot fail");
    writeln!(module, "  br label %loop").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "done:").expect("writing to string cannot fail");
    writeln!(module, "  ret i32 %count").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}

fn write_string_runtime_helpers(module: &mut String) {
    writeln!(
        module,
        "define private ptr @ax_string_concat(ptr %left, ptr %right) {{"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  %left_len = call i64 @strlen(ptr %left)")
        .expect("writing to string cannot fail");
    writeln!(module, "  %right_len = call i64 @strlen(ptr %right)")
        .expect("writing to string cannot fail");
    writeln!(module, "  %combined_len = add i64 %left_len, %right_len")
        .expect("writing to string cannot fail");
    writeln!(module, "  %total_len = add i64 %combined_len, 1")
        .expect("writing to string cannot fail");
    writeln!(module, "  %buffer = call ptr @malloc(i64 %total_len)")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %copy_left = call ptr @memcpy(ptr %buffer, ptr %left, i64 %left_len)"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %right_dest = getelementptr i8, ptr %buffer, i64 %left_len"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %copy_right = call ptr @memcpy(ptr %right_dest, ptr %right, i64 %right_len)"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %end = getelementptr i8, ptr %buffer, i64 %combined_len"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  store i8 0, ptr %end").expect("writing to string cannot fail");
    writeln!(module, "  ret ptr %buffer").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");

    writeln!(
        module,
        "define private ptr @ax_i32_to_string(i32 %value) {{"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  %buffer = call ptr @malloc(i64 12)")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %written = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %buffer, i64 12, ptr @.ax_fmt_i32, i32 %value)"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  ret ptr %buffer").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}

fn render_function(
    name: &str,
    params: &[crate::mir::Param],
    return_type: &Type,
    locals: &[Local],
    entry_block: u32,
    blocks: &[BasicBlock],
    signatures: &BTreeMap<String, FunctionSignature>,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    consts: &BTreeMap<String, ConstBinding>,
    strings: &BTreeMap<String, StringLiteral>,
) -> Result<String, String> {
    let signature = signatures
        .get(name)
        .ok_or_else(|| format!("internal LLVM AOT error: missing signature for `{name}`"))?;
    let declared_return_type = llvm_type(return_type, layouts, enum_layouts)
        .ok_or_else(|| format!("function `{name}` returns an unsupported type"))?;
    if signature.return_type != declared_return_type {
        return Err(format!(
            "internal LLVM AOT error: function `{name}` signature return type drifted"
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
    let local_type_overrides = infer_concrete_local_types(locals, blocks, enum_layouts);

    for local in locals {
        let ax_ty = local_type_overrides
            .get(&local.id)
            .cloned()
            .unwrap_or_else(|| local.ty.clone());
        let Some(ty) = llvm_type(&ax_ty, layouts, enum_layouts) else {
            return Err(format!(
                "function `{name}` local `{}` uses unsupported type {}",
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
    for (index, param) in params.iter().enumerate() {
        if index > 0 {
            write!(function, ", ").expect("writing to string cannot fail");
        }
        let Some(param_ty) = llvm_type(&param.ty, layouts, enum_layouts) else {
            return Err(format!(
                "function `{name}` parameter `{}` uses unsupported type {}",
                param.name,
                ax_type_name(&param.ty)
            ));
        };
        write!(function, "{param_ty} %arg{}", param.local).expect("writing to string cannot fail");
    }
    writeln!(function, ") {{").expect("writing to string cannot fail");
    writeln!(function, "entry:").expect("writing to string cannot fail");

    for local in locals {
        let slot = emitter.local_slot(local.id)?;
        writeln!(function, "  {} = alloca {}", slot.ptr, slot.ty)
            .expect("writing to string cannot fail");
    }
    for param in params {
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
                enum_layouts,
                &mut overrides,
            );
        }
        infer_terminator_concrete_local_types(
            &block.terminator.kind,
            &local_types,
            enum_layouts,
            &mut overrides,
        );
    }
    overrides
}

fn infer_statement_concrete_local_types(
    statement: &Statement,
    local_types: &BTreeMap<u32, Type>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    overrides: &mut BTreeMap<u32, Type>,
) {
    match &statement.kind {
        StatementKind::Let { initializer, .. } | StatementKind::Eval { expr: initializer } => {
            infer_expr_concrete_local_types(initializer, local_types, enum_layouts, overrides);
        }
        StatementKind::Assign { value, .. } => {
            infer_expr_concrete_local_types(value, local_types, enum_layouts, overrides);
        }
    }
}

fn infer_terminator_concrete_local_types(
    terminator: &TerminatorKind,
    local_types: &BTreeMap<u32, Type>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    overrides: &mut BTreeMap<u32, Type>,
) {
    match terminator {
        TerminatorKind::Branch { condition, .. } | TerminatorKind::Return { value: condition } => {
            infer_expr_concrete_local_types(condition, local_types, enum_layouts, overrides);
        }
        TerminatorKind::Goto { .. } | TerminatorKind::Unreachable => {}
    }
}

fn infer_expr_concrete_local_types(
    expr: &Expr,
    local_types: &BTreeMap<u32, Type>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
    overrides: &mut BTreeMap<u32, Type>,
) {
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            infer_expr_concrete_local_types(scrutinee, local_types, enum_layouts, overrides);
            let Some(scrutinee_ty) = static_expr_type(scrutinee, local_types, overrides) else {
                for arm in arms {
                    infer_expr_concrete_local_types(
                        &arm.value,
                        local_types,
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
                    infer_expr_concrete_local_types(guard, local_types, enum_layouts, overrides);
                }
                infer_expr_concrete_local_types(&arm.value, local_types, enum_layouts, overrides);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Try { expr }
        | ExprKind::EnumPayload { value: expr }
        | ExprKind::Field { base: expr, .. } => {
            infer_expr_concrete_local_types(expr, local_types, enum_layouts, overrides);
        }
        ExprKind::Binary { left, right, .. } => {
            infer_expr_concrete_local_types(left, local_types, enum_layouts, overrides);
            infer_expr_concrete_local_types(right, local_types, enum_layouts, overrides);
        }
        ExprKind::Call { arguments, .. }
        | ExprKind::ArrayLiteral {
            elements: arguments,
        } => {
            for argument in arguments {
                infer_expr_concrete_local_types(argument, local_types, enum_layouts, overrides);
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                infer_expr_concrete_local_types(&field.value, local_types, enum_layouts, overrides);
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                infer_statement_concrete_local_types(
                    statement,
                    local_types,
                    enum_layouts,
                    overrides,
                );
            }
            infer_expr_concrete_local_types(value, local_types, enum_layouts, overrides);
        }
        ExprKind::EnumVariant { payload, .. } => {
            if let Some(payload) = payload {
                infer_expr_concrete_local_types(payload, local_types, enum_layouts, overrides);
            }
        }
        ExprKind::MatchTest { scrutinee, .. } => {
            infer_expr_concrete_local_types(scrutinee, local_types, enum_layouts, overrides);
        }
        ExprKind::Index { base, index } => {
            infer_expr_concrete_local_types(base, local_types, enum_layouts, overrides);
            infer_expr_concrete_local_types(index, local_types, enum_layouts, overrides);
        }
        ExprKind::Slice { base, start, end } => {
            infer_expr_concrete_local_types(base, local_types, enum_layouts, overrides);
            infer_expr_concrete_local_types(start, local_types, enum_layouts, overrides);
            infer_expr_concrete_local_types(end, local_types, enum_layouts, overrides);
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
        ExprKind::Bool { .. } => Some(Type::Bool),
        ExprKind::String { .. } => Some(Type::String),
        ExprKind::Block { value, .. } => static_expr_type(value, local_types, overrides),
        ExprKind::EnumVariant { enum_name, .. } => Some(Type::Enum {
            name: enum_name.clone(),
        }),
        _ => None,
    }
}

impl<'a> FunctionEmitter<'a> {
    fn emit_statement(&mut self, statement: &Statement, out: &mut String) -> Result<(), String> {
        match &statement.kind {
            StatementKind::Let {
                local, initializer, ..
            } => {
                let slot = self.local_slot(*local)?.clone();
                let value = match &initializer.kind {
                    _ if matches!(slot.ax_ty, Type::Slice { .. }) => {
                        self.emit_slice_from_expr(initializer, out)?
                    }
                    ExprKind::EnumPayload { value } => {
                        self.emit_enum_payload(value, Some(&slot.ax_ty), out)?
                    }
                    ExprKind::EnumVariant {
                        enum_name,
                        variant,
                        payload,
                    } => self.emit_enum_variant(
                        enum_name,
                        variant,
                        payload.as_deref(),
                        Some(&slot.ax_ty),
                        out,
                    )?,
                    _ => self.emit_expr(initializer, out)?,
                };
                ensure_same_type(&slot.ty, &value.ty)?;
                writeln!(out, "  store {} {}, ptr {}", value.ty, value.repr, slot.ptr)
                    .expect("writing to string cannot fail");
                Ok(())
            }
            StatementKind::Assign { target, value } => {
                let slot = self.emit_place_ptr(target, out)?;
                let value = self.emit_expr(value, out)?;
                ensure_same_type(&slot.ty, &value.ty)?;
                writeln!(out, "  store {} {}, ptr {}", value.ty, value.repr, slot.ptr)
                    .expect("writing to string cannot fail");
                Ok(())
            }
            StatementKind::Eval { expr } => {
                self.emit_expr(expr, out)?;
                Ok(())
            }
        }
    }

    fn emit_terminator(&mut self, block: &BasicBlock, out: &mut String) -> Result<(), String> {
        match &block.terminator.kind {
            TerminatorKind::Goto { target } => {
                writeln!(out, "  br label %bb{target}").expect("writing to string cannot fail");
                Ok(())
            }
            TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let condition = self.emit_expr(condition, out)?;
                ensure_same_type("i1", &condition.ty)?;
                writeln!(
                    out,
                    "  br i1 {}, label %bb{}, label %bb{}",
                    condition.repr, then_block, else_block
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            TerminatorKind::Return { value } => {
                let value = match &value.kind {
                    ExprKind::EnumVariant {
                        enum_name,
                        variant,
                        payload,
                    } => {
                        let return_ax_ty = self.return_ax_ty.clone();
                        self.emit_enum_variant(
                            enum_name,
                            variant,
                            payload.as_deref(),
                            Some(&return_ax_ty),
                            out,
                        )?
                    }
                    _ => self.emit_expr(value, out)?,
                };
                writeln!(out, "  ret {} {}", value.ty, value.repr)
                    .expect("writing to string cannot fail");
                Ok(())
            }
            TerminatorKind::Unreachable => {
                writeln!(out, "  unreachable").expect("writing to string cannot fail");
                Ok(())
            }
        }
    }

    fn emit_expr(&mut self, expr: &Expr, out: &mut String) -> Result<LlvmValue, String> {
        match &expr.kind {
            ExprKind::Int { value } => Ok(LlvmValue {
                ty: "i32".to_string(),
                repr: value.to_string(),
                ax_ty: Some(Type::I32),
            }),
            ExprKind::Bool { value } => Ok(LlvmValue {
                ty: "i1".to_string(),
                repr: if *value { "1" } else { "0" }.to_string(),
                ax_ty: Some(Type::Bool),
            }),
            ExprKind::Local { local, .. } => {
                let slot = self.local_slot(*local)?.clone();
                let temp = self.next_temp();
                writeln!(out, "  {temp} = load {}, ptr {}", slot.ty, slot.ptr)
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: slot.ty,
                    repr: temp,
                    ax_ty: Some(slot.ax_ty),
                })
            }
            ExprKind::Unary { op, expr } => {
                let value = self.emit_expr(expr, out)?;
                match op {
                    UnaryOp::Negate => {
                        ensure_same_type("i32", &value.ty)?;
                        let temp = self.next_temp();
                        writeln!(out, "  {temp} = sub i32 0, {}", value.repr)
                            .expect("writing to string cannot fail");
                        Ok(LlvmValue {
                            ty: "i32".to_string(),
                            repr: temp,
                            ax_ty: Some(Type::I32),
                        })
                    }
                    UnaryOp::Not => {
                        ensure_same_type("i1", &value.ty)?;
                        let temp = self.next_temp();
                        writeln!(out, "  {temp} = xor i1 {}, 1", value.repr)
                            .expect("writing to string cannot fail");
                        Ok(LlvmValue {
                            ty: "i1".to_string(),
                            repr: temp,
                            ax_ty: Some(Type::Bool),
                        })
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                let left = self.emit_expr(left, out)?;
                let right = self.emit_expr(right, out)?;
                self.emit_binary(*op, left, right, out)
            }
            ExprKind::Call {
                function,
                arguments,
            } => self.emit_call(function, arguments, out),
            ExprKind::Float { .. } => Err("f32 values are not in the LLVM AOT v0 subset".into()),
            ExprKind::String { value } => {
                let literal = self
                    .strings
                    .get(value)
                    .ok_or_else(|| "internal LLVM AOT error: missing string literal".to_string())?;
                Ok(LlvmValue {
                    ty: "ptr".to_string(),
                    repr: literal.symbol.clone(),
                    ax_ty: Some(Type::String),
                })
            }
            ExprKind::Const { name } => self.emit_const(name, out),
            ExprKind::Try { expr } => self.emit_try(expr, out),
            ExprKind::ArrayLiteral { elements } => self.emit_array_literal(elements, out),
            ExprKind::Index { base, index } => self.emit_index(base, index, out),
            ExprKind::Slice { base, start, end } => self.emit_slice_range(base, start, end, out),
            ExprKind::StructLiteral { name, fields } => self.emit_struct_literal(name, fields, out),
            ExprKind::Field { base, field } => self.emit_field(base, field, out),
            ExprKind::EnumVariant {
                enum_name,
                variant,
                payload,
            } => self.emit_enum_variant(enum_name, variant, payload.as_deref(), None, out),
            ExprKind::MatchTest { scrutinee, pattern } => {
                self.emit_match_test(scrutinee, pattern, out)
            }
            ExprKind::EnumPayload { value } => self.emit_enum_payload(value, None, out),
            ExprKind::Block { statements, value } => self.emit_block_expr(statements, value, out),
            ExprKind::Match { scrutinee, arms } => self.emit_match_expr(scrutinee, arms, out),
        }
    }

    fn emit_block_expr(
        &mut self,
        statements: &[Statement],
        value: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        for statement in statements {
            self.emit_statement(statement, out)?;
        }
        self.emit_expr(value, out)
    }

    fn emit_const(&mut self, name: &str, out: &mut String) -> Result<LlvmValue, String> {
        let binding = self
            .consts
            .get(name)
            .cloned()
            .ok_or_else(|| format!("const `{name}` was not found for LLVM AOT lowering"))?;
        if self.const_stack.iter().any(|active| active == name) {
            return Err(format!(
                "const `{name}` recursively references itself during LLVM AOT lowering"
            ));
        }

        self.const_stack.push(name.to_string());
        let value = self.emit_expr(&binding.value, out);
        self.const_stack.pop();

        let mut value = value?;
        let Some(expected_ty) = llvm_type(&binding.ty, self.layouts, self.enum_layouts) else {
            return Err(format!(
                "const `{name}` uses unsupported type {}",
                ax_type_name(&binding.ty)
            ));
        };
        ensure_same_type(&expected_ty, &value.ty)?;
        value.ax_ty = Some(binding.ty);
        Ok(value)
    }

    fn emit_match_expr(
        &mut self,
        scrutinee: &Expr,
        arms: &[MatchExprArm],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arms.is_empty() {
            return Err("match expression has no arms in LLVM AOT v0".to_string());
        }
        let (result_ty, result_ax_ty) = self.infer_expr_value_type(&arms[0].value)?;
        for arm in &arms[1..] {
            let (arm_ty, _) = self.infer_expr_value_type(&arm.value)?;
            ensure_same_type(&result_ty, &arm_ty)?;
        }

        let scrutinee_value = self.emit_expr(scrutinee, out)?;
        let result_slot = self.next_temp();
        let done_label = self.next_label("match_done");
        let no_match_label = self.next_label("match_no_match");
        writeln!(out, "  {result_slot} = alloca {result_ty}")
            .expect("writing to string cannot fail");

        let arm_labels = (0..arms.len())
            .map(|index| self.next_label(&format!("match_arm_{index}")))
            .collect::<Vec<_>>();
        let next_labels = (0..arms.len())
            .map(|index| {
                if index + 1 == arms.len() {
                    no_match_label.clone()
                } else {
                    self.next_label(&format!("match_next_{index}"))
                }
            })
            .collect::<Vec<_>>();

        for (index, arm) in arms.iter().enumerate() {
            let matched = self.emit_match_test_value(&scrutinee_value, &arm.pattern, out)?;
            ensure_same_type("i1", &matched.ty)?;
            if let Some(guard) = &arm.guard {
                let guard_label = self.next_label(&format!("match_guard_{index}"));
                writeln!(
                    out,
                    "  br i1 {}, label %{}, label %{}",
                    matched.repr, guard_label, next_labels[index]
                )
                .expect("writing to string cannot fail");
                writeln!(out, "{guard_label}:").expect("writing to string cannot fail");
                self.emit_match_bindings(&scrutinee_value, arm, out)?;
                let guard_value = self.emit_expr(guard, out)?;
                ensure_same_type("i1", &guard_value.ty)?;
                writeln!(
                    out,
                    "  br i1 {}, label %{}, label %{}",
                    guard_value.repr, arm_labels[index], next_labels[index]
                )
                .expect("writing to string cannot fail");
            } else {
                writeln!(
                    out,
                    "  br i1 {}, label %{}, label %{}",
                    matched.repr, arm_labels[index], next_labels[index]
                )
                .expect("writing to string cannot fail");
            }
            writeln!(out, "{}:", arm_labels[index]).expect("writing to string cannot fail");
            if arm.guard.is_none() {
                self.emit_match_bindings(&scrutinee_value, arm, out)?;
            }
            let value = self.emit_expr(&arm.value, out)?;
            ensure_same_type(&result_ty, &value.ty)?;
            writeln!(
                out,
                "  store {} {}, ptr {result_slot}",
                value.ty, value.repr
            )
            .expect("writing to string cannot fail");
            writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
            if index + 1 < arms.len() {
                writeln!(out, "{}:", next_labels[index]).expect("writing to string cannot fail");
            }
        }

        writeln!(out, "{no_match_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @exit(i32 1)").expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load {result_ty}, ptr {result_slot}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: result_ty,
            repr: loaded,
            ax_ty: Some(result_ax_ty),
        })
    }

    fn emit_array_literal(
        &mut self,
        elements: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if elements.is_empty() {
            return Err(
                "empty array literals need explicit native array type propagation before LLVM AOT can lower them"
                    .into(),
            );
        }

        let mut rendered_elements = Vec::new();
        let mut element_ty: Option<String> = None;
        let mut element_ax_ty: Option<Type> = None;
        for element in elements {
            let value = self.emit_expr(element, out)?;
            if let Some(expected_ty) = &element_ty {
                ensure_same_type(expected_ty, &value.ty)?;
            } else {
                element_ty = Some(value.ty.clone());
            }
            if let Some(expected_ax_ty) = &element_ax_ty {
                if value.ax_ty.as_ref() != Some(expected_ax_ty) {
                    return Err("array literal elements must have one LLVM AOT type".to_string());
                }
            } else {
                element_ax_ty = value.ax_ty.clone();
            }
            rendered_elements.push((value.ty, value.repr));
        }

        let element_ty = element_ty.expect("non-empty array should have an element type");
        let element_ax_ty = element_ax_ty.ok_or_else(|| {
            "array literal element type is not representable in LLVM AOT v0".to_string()
        })?;
        let length = elements.len();
        let array_ty = format!("[{length} x {element_ty}]");
        let repr = self.emit_aggregate_value(&array_ty, &rendered_elements, out);
        Ok(LlvmValue {
            ty: array_ty,
            repr,
            ax_ty: Some(Type::Array {
                element: Box::new(element_ax_ty),
                length,
            }),
        })
    }

    fn emit_index(
        &mut self,
        base: &Expr,
        index: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let (_, base_ax_ty) = self.infer_expr_value_type(base)?;
        if matches!(base_ax_ty, Type::Slice { .. }) {
            return self.emit_slice_index(base, index, out);
        }

        let (array_ptr, array_ty, element_ty, length, element_ax_ty) =
            self.emit_array_base_ptr(base, out)?;
        let index = self.emit_expr(index, out)?;
        ensure_same_type("i32", &index.ty)?;
        self.emit_fixed_bounds_check(&index.repr, length, out);
        let element_ptr = self.next_temp();
        writeln!(
            out,
            "  {element_ptr} = getelementptr {array_ty}, ptr {array_ptr}, i32 0, i32 {}",
            index.repr
        )
        .expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load {element_ty}, ptr {element_ptr}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: element_ty,
            repr: loaded,
            ax_ty: Some(element_ax_ty),
        })
    }

    fn emit_slice_from_expr(&mut self, expr: &Expr, out: &mut String) -> Result<LlvmValue, String> {
        if let ExprKind::Local { local, .. } = &expr.kind {
            let slot = self.local_slot(*local)?.clone();
            match &slot.ax_ty {
                Type::Array { .. } => {
                    return self.emit_slice_from_array_ptr(&slot.ptr, &slot.ax_ty, out);
                }
                Type::Slice { .. } => return self.emit_expr(expr, out),
                _ => {}
            }
        }

        let value = self.emit_expr(expr, out)?;
        match value.ax_ty.as_ref() {
            Some(Type::Array { .. }) => self.emit_slice_from_array_value(value, out),
            Some(Type::Slice { .. }) => Ok(value),
            Some(other) => Err(format!(
                "cannot coerce {} to a read-only slice in LLVM AOT v0",
                ax_type_name(other)
            )),
            None => Err("slice source value is not representable in LLVM AOT v0".to_string()),
        }
    }

    fn emit_slice_from_array_value(
        &mut self,
        value: LlvmValue,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let Some(ax_ty) = value.ax_ty.clone() else {
            return Err(
                "array-to-slice coercion needs an AX array type in LLVM AOT v0".to_string(),
            );
        };
        let (array_ty, _, _, _) = array_type_parts(&ax_ty, self.layouts, self.enum_layouts)?;
        ensure_same_type(&array_ty, &value.ty)?;
        let temp_array = self.next_temp();
        writeln!(out, "  {temp_array} = alloca {array_ty}").expect("writing to string cannot fail");
        writeln!(out, "  store {array_ty} {}, ptr {temp_array}", value.repr)
            .expect("writing to string cannot fail");
        self.emit_slice_from_array_ptr(&temp_array, &ax_ty, out)
    }

    fn emit_slice_from_array_ptr(
        &mut self,
        array_ptr: &str,
        ax_ty: &Type,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let (array_ty, element_ty, length, element_ax_ty) =
            array_type_parts(ax_ty, self.layouts, self.enum_layouts)?;
        let data_ptr = self.next_temp();
        writeln!(
            out,
            "  {data_ptr} = getelementptr {array_ty}, ptr {array_ptr}, i32 0, i32 0"
        )
        .expect("writing to string cannot fail");
        let with_ptr = self.next_temp();
        writeln!(
            out,
            "  {with_ptr} = insertvalue {} undef, ptr {data_ptr}, 0",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let repr = self.next_temp();
        writeln!(
            out,
            "  {repr} = insertvalue {} {with_ptr}, i32 {length}, 1",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let slice_ax_ty = Type::Slice {
            element: Box::new(element_ax_ty),
        };
        let expected_slice_ty = llvm_type(&slice_ax_ty, self.layouts, self.enum_layouts)
            .ok_or_else(|| format!("slice element type {element_ty} is outside LLVM AOT v0"))?;
        Ok(LlvmValue {
            ty: expected_slice_ty,
            repr,
            ax_ty: Some(slice_ax_ty),
        })
    }

    fn emit_slice_index(
        &mut self,
        base: &Expr,
        index: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let slice = self.emit_expr(base, out)?;
        let Some(Type::Slice { element }) = slice.ax_ty.clone() else {
            return Err("slice index base is not a slice in LLVM AOT v0".to_string());
        };
        ensure_same_type(&slice_llvm_type(), &slice.ty)?;
        let element_ty = llvm_type(&element, self.layouts, self.enum_layouts).ok_or_else(|| {
            format!(
                "slice element type {} is outside LLVM AOT v0",
                ax_type_name(&element)
            )
        })?;
        let index = self.emit_expr(index, out)?;
        ensure_same_type("i32", &index.ty)?;
        let data_ptr = self.next_temp();
        writeln!(
            out,
            "  {data_ptr} = extractvalue {} {}, 0",
            slice.ty, slice.repr
        )
        .expect("writing to string cannot fail");
        let len = self.next_temp();
        writeln!(out, "  {len} = extractvalue {} {}, 1", slice.ty, slice.repr)
            .expect("writing to string cannot fail");
        self.emit_dynamic_bounds_check(&index.repr, &len, out);
        let element_ptr = self.next_temp();
        writeln!(
            out,
            "  {element_ptr} = getelementptr {element_ty}, ptr {data_ptr}, i32 {}",
            index.repr
        )
        .expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load {element_ty}, ptr {element_ptr}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: element_ty,
            repr: loaded,
            ax_ty: Some(*element),
        })
    }

    fn emit_slice_range(
        &mut self,
        base: &Expr,
        start: &Expr,
        end: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let slice = self.emit_slice_from_expr(base, out)?;
        let Some(Type::Slice { element }) = slice.ax_ty.clone() else {
            return Err("slice expression base is not slice-compatible in LLVM AOT v0".to_string());
        };
        ensure_same_type(&slice_llvm_type(), &slice.ty)?;
        let element_ax_ty = *element;
        let element_ty =
            llvm_type(&element_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "slice element type {} is outside LLVM AOT v0",
                    ax_type_name(&element_ax_ty)
                )
            })?;

        let start = self.emit_expr(start, out)?;
        ensure_same_type("i32", &start.ty)?;
        let end = self.emit_expr(end, out)?;
        ensure_same_type("i32", &end.ty)?;

        let base_ptr = self.next_temp();
        writeln!(
            out,
            "  {base_ptr} = extractvalue {} {}, 0",
            slice.ty, slice.repr
        )
        .expect("writing to string cannot fail");
        let base_len = self.next_temp();
        writeln!(
            out,
            "  {base_len} = extractvalue {} {}, 1",
            slice.ty, slice.repr
        )
        .expect("writing to string cannot fail");

        self.emit_slice_bound_check(&start.repr, &base_len, out);
        self.emit_slice_bound_check(&end.repr, &base_len, out);
        self.emit_slice_order_check(&start.repr, &end.repr, out);

        let data_ptr = self.next_temp();
        writeln!(
            out,
            "  {data_ptr} = getelementptr {element_ty}, ptr {base_ptr}, i32 {}",
            start.repr
        )
        .expect("writing to string cannot fail");
        let len = self.next_temp();
        writeln!(out, "  {len} = sub i32 {}, {}", end.repr, start.repr)
            .expect("writing to string cannot fail");
        let with_ptr = self.next_temp();
        writeln!(
            out,
            "  {with_ptr} = insertvalue {} undef, ptr {data_ptr}, 0",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let repr = self.next_temp();
        writeln!(
            out,
            "  {repr} = insertvalue {} {with_ptr}, i32 {len}, 1",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");

        Ok(LlvmValue {
            ty: slice_llvm_type(),
            repr,
            ax_ty: Some(Type::Slice {
                element: Box::new(element_ax_ty),
            }),
        })
    }

    fn emit_array_base_ptr(
        &mut self,
        base: &Expr,
        out: &mut String,
    ) -> Result<(String, String, String, usize, Type), String> {
        if let ExprKind::Local { local, .. } = &base.kind {
            let slot = self.local_slot(*local)?.clone();
            let (array_ty, element_ty, length, element_ax_ty) =
                array_type_parts(&slot.ax_ty, self.layouts, self.enum_layouts)?;
            ensure_same_type(&array_ty, &slot.ty)?;
            return Ok((slot.ptr, array_ty, element_ty, length, element_ax_ty));
        }

        let value = self.emit_expr(base, out)?;
        let Some(ax_ty) = value.ax_ty.clone() else {
            return Err("array index base is not a fixed-size array in LLVM AOT v0".to_string());
        };
        let (array_ty, element_ty, length, element_ax_ty) =
            array_type_parts(&ax_ty, self.layouts, self.enum_layouts)?;
        ensure_same_type(&array_ty, &value.ty)?;
        let temp_array = self.next_temp();
        writeln!(out, "  {temp_array} = alloca {array_ty}").expect("writing to string cannot fail");
        writeln!(out, "  store {array_ty} {}, ptr {temp_array}", value.repr)
            .expect("writing to string cannot fail");
        Ok((temp_array, array_ty, element_ty, length, element_ax_ty))
    }

    fn emit_fixed_bounds_check(&mut self, index: &str, length: usize, out: &mut String) {
        self.emit_dynamic_bounds_check(index, &length.to_string(), out);
    }

    fn emit_dynamic_bounds_check(&mut self, index: &str, length: &str, out: &mut String) {
        let negative = self.next_temp();
        let too_high = self.next_temp();
        let out_of_bounds = self.next_temp();
        let fail_label = self.next_label("array_oob");
        let ok_label = self.next_label("array_ok");
        writeln!(out, "  {negative} = icmp slt i32 {index}, 0")
            .expect("writing to string cannot fail");
        writeln!(out, "  {too_high} = icmp sge i32 {index}, {length}")
            .expect("writing to string cannot fail");
        writeln!(out, "  {out_of_bounds} = or i1 {negative}, {too_high}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {out_of_bounds}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @exit(i32 1)").expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
    }

    fn emit_slice_bound_check(&mut self, bound: &str, length: &str, out: &mut String) {
        let negative = self.next_temp();
        let too_high = self.next_temp();
        let out_of_bounds = self.next_temp();
        let fail_label = self.next_label("slice_bound_oob");
        let ok_label = self.next_label("slice_bound_ok");
        writeln!(out, "  {negative} = icmp slt i32 {bound}, 0")
            .expect("writing to string cannot fail");
        writeln!(out, "  {too_high} = icmp sgt i32 {bound}, {length}")
            .expect("writing to string cannot fail");
        writeln!(out, "  {out_of_bounds} = or i1 {negative}, {too_high}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {out_of_bounds}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @exit(i32 1)").expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
    }

    fn emit_slice_order_check(&mut self, start: &str, end: &str, out: &mut String) {
        let invalid = self.next_temp();
        let fail_label = self.next_label("slice_order_invalid");
        let ok_label = self.next_label("slice_order_ok");
        writeln!(out, "  {invalid} = icmp sgt i32 {start}, {end}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {invalid}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @exit(i32 1)").expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
    }

    fn emit_struct_literal(
        &mut self,
        name: &str,
        fields: &[StructLiteralField],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let layout = self.struct_layout_by_name(name)?.clone();
        let mut values = BTreeMap::new();
        for field in fields {
            let value = self.emit_expr(&field.value, out)?;
            values.insert(field.name.clone(), value);
        }

        let mut rendered_fields = Vec::new();
        for field in &layout.fields {
            let value = values.remove(&field.name).ok_or_else(|| {
                format!(
                    "internal LLVM AOT error: struct `{}` literal is missing field `{}`",
                    layout.name, field.name
                )
            })?;
            ensure_same_type(&field.ty, &value.ty)?;
            rendered_fields.push((field.ty.clone(), value.repr));
        }
        if let Some(extra) = values.keys().next() {
            return Err(format!(
                "internal LLVM AOT error: struct `{}` literal has unknown field `{extra}`",
                layout.name
            ));
        }

        Ok(LlvmValue {
            ty: layout.ty.clone(),
            repr: self.emit_aggregate_value(&layout.ty, &rendered_fields, out),
            ax_ty: Some(Type::Struct {
                name: name.to_string(),
            }),
        })
    }

    fn emit_aggregate_value(
        &mut self,
        aggregate_ty: &str,
        fields: &[(String, String)],
        out: &mut String,
    ) -> String {
        let mut current = "undef".to_string();
        for (index, (field_ty, field_repr)) in fields.iter().enumerate() {
            let temp = self.next_temp();
            writeln!(
                out,
                "  {temp} = insertvalue {aggregate_ty} {current}, {field_ty} {field_repr}, {index}"
            )
            .expect("writing to string cannot fail");
            current = temp;
        }
        current
    }

    fn emit_field(
        &mut self,
        base: &Expr,
        field: &str,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let (base_ptr, layout) = self.emit_struct_base_ptr(base, out)?;
        let field_layout = layout
            .fields
            .iter()
            .find(|candidate| candidate.name == field)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "internal LLVM AOT error: struct `{}` has no field `{field}`",
                    layout.name
                )
            })?;
        let field_ptr = self.next_temp();
        writeln!(
            out,
            "  {field_ptr} = getelementptr {}, ptr {base_ptr}, i32 0, i32 {}",
            layout.ty, field_layout.index
        )
        .expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(
            out,
            "  {loaded} = load {}, ptr {field_ptr}",
            field_layout.ty
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: field_layout.ty,
            repr: loaded,
            ax_ty: Some(field_layout.ax_ty),
        })
    }

    fn emit_struct_base_ptr(
        &mut self,
        base: &Expr,
        out: &mut String,
    ) -> Result<(String, StructLayout), String> {
        if let ExprKind::Local { local, .. } = &base.kind {
            let slot = self.local_slot(*local)?.clone();
            let layout = self.struct_layout_for_type(&slot.ax_ty)?.clone();
            ensure_same_type(&layout.ty, &slot.ty)?;
            return Ok((slot.ptr, layout));
        }

        let value = self.emit_expr(base, out)?;
        let Some(ax_ty) = value.ax_ty.clone() else {
            return Err("field access base is not a struct in LLVM AOT v0".to_string());
        };
        let layout = self.struct_layout_for_type(&ax_ty)?.clone();
        ensure_same_type(&layout.ty, &value.ty)?;
        let temp_struct = self.next_temp();
        writeln!(out, "  {temp_struct} = alloca {}", layout.ty)
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {}, ptr {temp_struct}",
            layout.ty, value.repr
        )
        .expect("writing to string cannot fail");
        Ok((temp_struct, layout))
    }

    fn emit_enum_variant(
        &mut self,
        enum_name: &str,
        variant: &str,
        payload: Option<&Expr>,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let payload_value = payload
            .map(|payload_expr| self.emit_expr(payload_expr, out))
            .transpose()?;
        self.emit_enum_variant_value(enum_name, variant, payload_value, expected_ax_ty, out)
    }

    fn emit_enum_variant_value(
        &mut self,
        enum_name: &str,
        variant: &str,
        payload_value: Option<LlvmValue>,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let layout = self
            .enum_layout_for_constructor(enum_name, expected_ax_ty)?
            .clone();
        let variant_layout = layout
            .variants
            .iter()
            .find(|candidate| candidate.name == variant)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "internal LLVM AOT error: enum `{}` has no variant `{variant}`",
                    layout.name
                )
            })?;

        if layout.ty == "i32" {
            if payload_value.is_some() {
                return Err(format!(
                    "enum `{enum_name}.{variant}` does not carry a payload in LLVM AOT v0"
                ));
            }
            return Ok(LlvmValue {
                ty: layout.ty.clone(),
                repr: variant_layout.tag.to_string(),
                ax_ty: Some(self.enum_value_type_for_layout(&layout, expected_ax_ty)?),
            });
        }

        let payload_ptr = match (&variant_layout.payload_ax_ty, payload_value) {
            (Some(payload_ax_ty), Some(payload_value)) => {
                let payload_ty = llvm_type(payload_ax_ty, self.layouts, self.enum_layouts)
                    .ok_or_else(|| {
                        format!(
                            "enum `{enum_name}.{variant}` payload type {} is outside LLVM AOT v0",
                            ax_type_name(payload_ax_ty)
                        )
                    })?;
                ensure_same_type(&payload_ty, &payload_value.ty)?;
                let payload_size = llvm_alloc_size(payload_ax_ty, self.layouts, self.enum_layouts)
                    .ok_or_else(|| {
                        format!(
                            "enum `{enum_name}.{variant}` payload type {} needs a native size contract before LLVM AOT can lower it",
                            ax_type_name(payload_ax_ty)
                        )
                    })?;
                let payload_ptr = self.next_temp();
                writeln!(
                    out,
                    "  {payload_ptr} = call ptr @malloc(i64 {payload_size})"
                )
                .expect("writing to string cannot fail");
                writeln!(
                    out,
                    "  store {payload_ty} {}, ptr {payload_ptr}",
                    payload_value.repr
                )
                .expect("writing to string cannot fail");
                payload_ptr
            }
            (Some(_), None) => {
                return Err(format!(
                    "enum `{enum_name}.{variant}` requires a payload in LLVM AOT v0"
                ));
            }
            (None, Some(_)) => {
                return Err(format!(
                    "enum `{enum_name}.{variant}` does not carry a payload in LLVM AOT v0"
                ));
            }
            (None, None) => "null".to_string(),
        };

        let repr = self.emit_aggregate_value(
            &layout.ty,
            &[
                ("i32".to_string(), variant_layout.tag.to_string()),
                ("ptr".to_string(), payload_ptr),
            ],
            out,
        );
        Ok(LlvmValue {
            ty: layout.ty.clone(),
            repr,
            ax_ty: Some(self.enum_value_type_for_layout(&layout, expected_ax_ty)?),
        })
    }

    fn emit_try(&mut self, inner: &Expr, out: &mut String) -> Result<LlvmValue, String> {
        let result_value = self.emit_expr(inner, out)?;
        let Some(result_ax_ty @ Type::EnumInstance { .. }) = result_value.ax_ty.clone() else {
            return Err(
                "`?` result propagation requires a concrete Result<T, E> value in LLVM AOT v0"
                    .to_string(),
            );
        };
        let (success_ax_ty, error_ax_ty) = self.result_success_error_types(&result_ax_ty)?;
        let (_return_success_ax_ty, return_error_ax_ty) =
            self.result_success_error_types(&self.return_ax_ty)?;
        if error_ax_ty != return_error_ax_ty {
            return Err(format!(
                "`?` result propagation error type {} does not match current function error type {} in LLVM AOT v0",
                ax_type_name(&error_ax_ty),
                ax_type_name(&return_error_ax_ty)
            ));
        }

        let layout = self.enum_layout_for_type(&result_ax_ty)?.clone();
        ensure_same_type(&layout.ty, &result_value.ty)?;
        if layout.ty == "i32" {
            return Err(
                "`?` result propagation requires payload enum layout in LLVM AOT v0".to_string(),
            );
        }

        let ok_tag = self.enum_variant_tag(&layout, "Ok")?;
        let tag_value = self.next_temp();
        writeln!(
            out,
            "  {tag_value} = extractvalue {} {}, 0",
            layout.ty, result_value.repr
        )
        .expect("writing to string cannot fail");
        let is_ok = self.next_temp();
        writeln!(out, "  {is_ok} = icmp eq i32 {tag_value}, {ok_tag}")
            .expect("writing to string cannot fail");

        let ok_label = self.next_label("try_ok");
        let err_label = self.next_label("try_err");
        writeln!(
            out,
            "  br i1 {is_ok}, label %{ok_label}, label %{err_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{err_label}:").expect("writing to string cannot fail");
        let error_value = self.emit_enum_payload_value(&result_value, Some(&error_ax_ty), out)?;
        let return_ax_ty = self.return_ax_ty.clone();
        let return_enum_name = enum_base_name(&return_ax_ty).to_string();
        let return_value = self.emit_enum_variant_value(
            &return_enum_name,
            "Err",
            Some(error_value),
            Some(&return_ax_ty),
            out,
        )?;
        let return_ty =
            llvm_type(&return_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "function return type {} is outside LLVM AOT v0",
                    ax_type_name(&return_ax_ty)
                )
            })?;
        ensure_same_type(&return_ty, &return_value.ty)?;
        writeln!(out, "  ret {} {}", return_value.ty, return_value.repr)
            .expect("writing to string cannot fail");

        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        self.emit_enum_payload_value(&result_value, Some(&success_ax_ty), out)
    }

    fn emit_enum_payload(
        &mut self,
        value: &Expr,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let value = self.emit_expr(value, out)?;
        self.emit_enum_payload_value(&value, expected_ax_ty, out)
    }

    fn emit_enum_payload_value(
        &mut self,
        value: &LlvmValue,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let Some(enum_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. })) =
            value.ax_ty.clone()
        else {
            return Err(
                "enum payload extraction requires an enum value in LLVM AOT v0".to_string(),
            );
        };
        let enum_name = enum_base_name(&enum_ax_ty).to_string();
        let layout = self.enum_layout_for_type(&enum_ax_ty)?.clone();
        if layout.ty == "i32" {
            return Err(format!(
                "enum `{enum_name}` has no native payload storage in LLVM AOT v0"
            ));
        }

        let payload_ax_ty = match expected_ax_ty {
            Some(expected) => {
                if !layout
                    .variants
                    .iter()
                    .any(|variant| variant.payload_ax_ty.as_ref() == Some(expected))
                {
                    return Err(format!(
                        "enum `{enum_name}` has no payload variant with type {} in LLVM AOT v0",
                        ax_type_name(expected)
                    ));
                }
                expected.clone()
            }
            None => {
                let payload_types = layout
                    .variants
                    .iter()
                    .filter_map(|variant| variant.payload_ax_ty.clone())
                    .collect::<Vec<_>>();
                let first = payload_types.first().cloned().ok_or_else(|| {
                    format!("enum `{enum_name}` has no payload variant in LLVM AOT v0")
                })?;
                if payload_types.iter().any(|payload| payload != &first) {
                    return Err(format!(
                        "enum `{enum_name}` payload extraction needs an expected type in LLVM AOT v0"
                    ));
                }
                first
            }
        };

        let payload_ty =
            llvm_type(&payload_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "enum `{enum_name}` payload type {} is outside LLVM AOT v0",
                    ax_type_name(&payload_ax_ty)
                )
            })?;
        let payload_ptr = self.next_temp();
        writeln!(
            out,
            "  {payload_ptr} = extractvalue {} {}, 1",
            layout.ty, value.repr
        )
        .expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load {payload_ty}, ptr {payload_ptr}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: payload_ty,
            repr: loaded,
            ax_ty: Some(payload_ax_ty),
        })
    }

    fn emit_match_test(
        &mut self,
        scrutinee: &Expr,
        pattern: &MatchPattern,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let value = self.emit_expr(scrutinee, out)?;
        self.emit_match_test_value(&value, pattern, out)
    }

    fn emit_match_test_value(
        &mut self,
        value: &LlvmValue,
        pattern: &MatchPattern,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        match &pattern.kind {
            MatchPatternKind::Wildcard | MatchPatternKind::Binding { .. } => Ok(LlvmValue {
                ty: "i1".to_string(),
                repr: "1".to_string(),
                ax_ty: Some(Type::Bool),
            }),
            MatchPatternKind::Bool { value: expected } => {
                ensure_same_type("i1", &value.ty)?;
                let result = self.next_temp();
                let expected = if *expected { "1" } else { "0" };
                writeln!(out, "  {result} = icmp eq i1 {}, {expected}", value.repr)
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::Int { value: expected } => {
                ensure_same_type("i32", &value.ty)?;
                let result = self.next_temp();
                writeln!(out, "  {result} = icmp eq i32 {}, {expected}", value.repr)
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::IntRange { start, end } => {
                ensure_same_type("i32", &value.ty)?;
                let lower = self.next_temp();
                let upper = self.next_temp();
                let result = self.next_temp();
                writeln!(out, "  {lower} = icmp sge i32 {}, {start}", value.repr)
                    .expect("writing to string cannot fail");
                writeln!(out, "  {upper} = icmp sle i32 {}, {end}", value.repr)
                    .expect("writing to string cannot fail");
                writeln!(out, "  {result} = and i1 {lower}, {upper}")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::String { value: expected } => {
                ensure_same_type("ptr", &value.ty)?;
                let literal = self.strings.get(expected).ok_or_else(|| {
                    "internal LLVM AOT error: missing string pattern literal".to_string()
                })?;
                let compare = self.next_temp();
                let result = self.next_temp();
                writeln!(
                    out,
                    "  {compare} = call i32 @strcmp(ptr {}, ptr {})",
                    value.repr, literal.symbol
                )
                .expect("writing to string cannot fail");
                writeln!(out, "  {result} = icmp eq i32 {compare}, 0")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload,
                ..
            } => {
                let Some(value_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. })) =
                    value.ax_ty.as_ref()
                else {
                    return Err(format!(
                        "match pattern `{enum_name}.{variant}` cannot be tested against {} in LLVM AOT v0",
                        value
                            .ax_ty
                            .as_ref()
                            .map(ax_type_name)
                            .unwrap_or_else(|| value.ty.clone())
                    ));
                };
                if enum_base_name(value_ax_ty) != enum_name {
                    return Err(format!(
                        "match pattern `{enum_name}.{variant}` cannot be tested against {} in LLVM AOT v0",
                        value
                            .ax_ty
                            .as_ref()
                            .map(ax_type_name)
                            .unwrap_or_else(|| value.ty.clone())
                    ));
                }
                let layout = self.enum_layout_for_type(value_ax_ty)?;
                let layout_name = layout.name.clone();
                let layout_ty = layout.ty.clone();
                let tag = layout
                    .variants
                    .iter()
                    .find(|candidate| candidate.name == *variant)
                    .map(|candidate| candidate.tag)
                    .ok_or_else(|| {
                        format!(
                            "internal LLVM AOT error: enum `{}` has no variant `{variant}`",
                            layout_name
                        )
                    })?;
                if payload.is_some()
                    && !layout.variants.iter().any(|candidate| {
                        candidate.name == *variant && candidate.payload_ax_ty.is_some()
                    })
                {
                    return Err(format!(
                        "match pattern `{enum_name}.{variant}(...)` targets a unit variant in LLVM AOT v0"
                    ));
                }
                let tag_value = if layout_ty == "i32" {
                    value.repr.clone()
                } else {
                    let tag_value = self.next_temp();
                    writeln!(
                        out,
                        "  {tag_value} = extractvalue {layout_ty} {}, 0",
                        value.repr
                    )
                    .expect("writing to string cannot fail");
                    tag_value
                };
                let result = self.next_temp();
                writeln!(out, "  {result} = icmp eq i32 {tag_value}, {tag}")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::Struct {
                struct_name,
                fields,
            } => {
                let Some(value_ax_ty @ Type::Struct { .. }) = value.ax_ty.as_ref() else {
                    return Err(format!(
                        "struct pattern `{struct_name} {{ ... }}` cannot be tested against {} in LLVM AOT v0",
                        value
                            .ax_ty
                            .as_ref()
                            .map(ax_type_name)
                            .unwrap_or_else(|| value.ty.clone())
                    ));
                };
                let layout = self.struct_layout_for_type(value_ax_ty)?;
                if layout.name != *struct_name {
                    return Err(format!(
                        "struct pattern `{struct_name} {{ ... }}` cannot be tested against {} in LLVM AOT v0",
                        ax_type_name(value_ax_ty)
                    ));
                }
                ensure_same_type(&layout.ty, &value.ty)?;
                for field in fields {
                    if !layout
                        .fields
                        .iter()
                        .any(|candidate| candidate.name == field.name)
                    {
                        return Err(format!(
                            "internal LLVM AOT error: struct `{struct_name}` has no field `{}`",
                            field.name
                        ));
                    }
                }
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: "1".to_string(),
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::Error => {
                Err("match pattern requires native lowering outside LLVM AOT v0".to_string())
            }
            MatchPatternKind::Or { alternatives } => {
                if alternatives.iter().any(match_pattern_contains_binding) {
                    return Err(
                        "or patterns with bindings need binding merge semantics before LLVM AOT can lower them"
                            .to_string(),
                    );
                }
                let Some(first) = alternatives.first() else {
                    return Ok(LlvmValue {
                        ty: "i1".to_string(),
                        repr: "0".to_string(),
                        ax_ty: Some(Type::Bool),
                    });
                };
                let mut result = self.emit_match_test_value(value, first, out)?;
                ensure_same_type("i1", &result.ty)?;
                for alternative in &alternatives[1..] {
                    let next = self.emit_match_test_value(value, alternative, out)?;
                    ensure_same_type("i1", &next.ty)?;
                    let combined = self.next_temp();
                    writeln!(out, "  {combined} = or i1 {}, {}", result.repr, next.repr)
                        .expect("writing to string cannot fail");
                    result = LlvmValue {
                        ty: "i1".to_string(),
                        repr: combined,
                        ax_ty: Some(Type::Bool),
                    };
                }
                Ok(result)
            }
        }
    }

    fn emit_match_bindings(
        &mut self,
        scrutinee: &LlvmValue,
        arm: &MatchExprArm,
        out: &mut String,
    ) -> Result<(), String> {
        match &arm.pattern.kind {
            MatchPatternKind::Binding { name } => {
                if let Some(local) = find_local_use_by_name_in_match_arm(arm, name) {
                    let slot = self.local_slot(local)?.clone();
                    if slot.ty == scrutinee.ty {
                        writeln!(
                            out,
                            "  store {} {}, ptr {}",
                            slot.ty, scrutinee.repr, slot.ptr
                        )
                        .expect("writing to string cannot fail");
                    }
                }
                Ok(())
            }
            MatchPatternKind::EnumVariant {
                variant,
                payload: Some(EnumVariantPayloadPattern::Binding { name: payload_name }),
                ..
            } => {
                if let Some(local) = find_local_use_by_name_in_match_arm(arm, payload_name) {
                    let slot = self.local_slot(local)?.clone();
                    let payload_ax_ty = self
                        .enum_layout_for_value(scrutinee)?
                        .variants
                        .iter()
                        .find(|candidate| candidate.name == *variant)
                        .and_then(|candidate| candidate.payload_ax_ty.clone())
                        .ok_or_else(|| {
                            format!(
                                "match pattern payload `{variant}` needs a native payload type in LLVM AOT v0"
                            )
                        })?;
                    let expected_ty = llvm_type(&payload_ax_ty, self.layouts, self.enum_layouts)
                        .ok_or_else(|| {
                            format!(
                                "enum payload type {} is outside LLVM AOT v0",
                                ax_type_name(&payload_ax_ty)
                            )
                        })?;
                    if slot.ty == expected_ty {
                        let value =
                            self.emit_enum_payload_value(scrutinee, Some(&payload_ax_ty), out)?;
                        ensure_same_type(&slot.ty, &value.ty)?;
                        writeln!(out, "  store {} {}, ptr {}", slot.ty, value.repr, slot.ptr)
                            .expect("writing to string cannot fail");
                    }
                }
                Ok(())
            }
            MatchPatternKind::Struct {
                struct_name,
                fields,
                ..
            } => {
                let Some(scrutinee_ax_ty @ Type::Struct { .. }) = scrutinee.ax_ty.as_ref() else {
                    return Err(format!(
                        "struct pattern `{struct_name} {{ ... }}` requires a struct value in LLVM AOT v0"
                    ));
                };
                let layout = self.struct_layout_for_type(scrutinee_ax_ty)?.clone();
                if layout.name != *struct_name {
                    return Err(format!(
                        "struct pattern `{struct_name} {{ ... }}` cannot bind fields from {} in LLVM AOT v0",
                        ax_type_name(scrutinee_ax_ty)
                    ));
                }
                ensure_same_type(&layout.ty, &scrutinee.ty)?;
                for field in fields {
                    if let Some(local) = find_local_use_by_name_in_match_arm(arm, &field.binding) {
                        let slot = self.local_slot(local)?.clone();
                        let field_layout = layout
                            .fields
                            .iter()
                            .find(|candidate| candidate.name == field.name)
                            .cloned()
                            .ok_or_else(|| {
                                format!(
                                    "internal LLVM AOT error: struct `{}` has no field `{}`",
                                    layout.name, field.name
                                )
                            })?;
                        ensure_same_type(&field_layout.ty, &slot.ty)?;
                        let value = self.next_temp();
                        writeln!(
                            out,
                            "  {value} = extractvalue {} {}, {}",
                            layout.ty, scrutinee.repr, field_layout.index
                        )
                        .expect("writing to string cannot fail");
                        writeln!(out, "  store {} {value}, ptr {}", slot.ty, slot.ptr)
                            .expect("writing to string cannot fail");
                    }
                }
                Ok(())
            }
            MatchPatternKind::Wildcard
            | MatchPatternKind::Bool { .. }
            | MatchPatternKind::Int { .. }
            | MatchPatternKind::IntRange { .. }
            | MatchPatternKind::String { .. }
            | MatchPatternKind::EnumVariant { .. }
            | MatchPatternKind::Or { .. }
            | MatchPatternKind::Error => Ok(()),
        }
    }

    fn emit_binary(
        &mut self,
        op: BinaryOp,
        left: LlvmValue,
        right: LlvmValue,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        ensure_same_type(&left.ty, &right.ty)?;
        if is_enum_value(&left) || is_enum_value(&right) {
            if left.ax_ty != right.ax_ty {
                return Err(
                    "enum comparison in LLVM AOT v0 requires both operands to have the same enum type"
                        .to_string(),
                );
            }
            if !matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
                return Err(
                    "enum values only support equality comparisons in LLVM AOT v0".to_string(),
                );
            }
            return self.emit_enum_equality(op, left, right, out);
        }
        let temp = self.next_temp();
        match op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder => {
                if left.ty == "ptr" {
                    if matches!(op, BinaryOp::Add) {
                        writeln!(
                            out,
                            "  {temp} = call ptr @ax_string_concat(ptr {}, ptr {})",
                            left.repr, right.repr
                        )
                        .expect("writing to string cannot fail");
                        return Ok(LlvmValue {
                            ty: "ptr".to_string(),
                            repr: temp,
                            ax_ty: Some(Type::String),
                        });
                    }
                    return Err(format!(
                        "string values do not support `{}` in LLVM AOT v0",
                        llvm_binary_op_name(op)
                    ));
                }
                ensure_same_type("i32", &left.ty)?;
                let instruction = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Subtract => "sub",
                    BinaryOp::Multiply => "mul",
                    BinaryOp::Divide => "sdiv",
                    BinaryOp::Remainder => "srem",
                    _ => unreachable!(),
                };
                writeln!(
                    out,
                    "  {temp} = {instruction} i32 {}, {}",
                    left.repr, right.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i32".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::I32),
                })
            }
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                ensure_same_type("i1", &left.ty)?;
                let instruction = if matches!(op, BinaryOp::LogicalAnd) {
                    "and"
                } else {
                    "or"
                };
                writeln!(
                    out,
                    "  {temp} = {instruction} i1 {}, {}",
                    left.repr, right.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::Bool),
                })
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                if left.ty == "ptr" {
                    let compare = temp;
                    let result = self.next_temp();
                    writeln!(
                        out,
                        "  {compare} = call i32 @strcmp(ptr {}, ptr {})",
                        left.repr, right.repr
                    )
                    .expect("writing to string cannot fail");
                    let predicate = if matches!(op, BinaryOp::Equal) {
                        "eq"
                    } else {
                        "ne"
                    };
                    writeln!(out, "  {result} = icmp {predicate} i32 {compare}, 0")
                        .expect("writing to string cannot fail");
                    return Ok(LlvmValue {
                        ty: "i1".to_string(),
                        repr: result,
                        ax_ty: Some(Type::Bool),
                    });
                }
                let predicate = if matches!(op, BinaryOp::Equal) {
                    "eq"
                } else {
                    "ne"
                };
                writeln!(
                    out,
                    "  {temp} = icmp {predicate} {} {}, {}",
                    left.ty, left.repr, right.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::Bool),
                })
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                ensure_same_type("i32", &left.ty)?;
                let predicate = match op {
                    BinaryOp::Less => "slt",
                    BinaryOp::LessEqual => "sle",
                    BinaryOp::Greater => "sgt",
                    BinaryOp::GreaterEqual => "sge",
                    _ => unreachable!(),
                };
                writeln!(
                    out,
                    "  {temp} = icmp {predicate} i32 {}, {}",
                    left.repr, right.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::Bool),
                })
            }
        }
    }

    fn emit_enum_equality(
        &mut self,
        op: BinaryOp,
        left: LlvmValue,
        right: LlvmValue,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let enum_ax_ty = left.ax_ty.as_ref().cloned().ok_or_else(|| {
            "enum equality requires enum type metadata in LLVM AOT v0".to_string()
        })?;
        let layout = self.enum_layout_for_type(&enum_ax_ty)?.clone();
        ensure_same_type(&layout.ty, &left.ty)?;
        ensure_same_type(&layout.ty, &right.ty)?;

        if layout.ty == "i32" {
            let result = self.next_temp();
            let predicate = if matches!(op, BinaryOp::Equal) {
                "eq"
            } else {
                "ne"
            };
            writeln!(
                out,
                "  {result} = icmp {predicate} i32 {}, {}",
                left.repr, right.repr
            )
            .expect("writing to string cannot fail");
            return Ok(LlvmValue {
                ty: "i1".to_string(),
                repr: result,
                ax_ty: Some(Type::Bool),
            });
        }

        let left_tag = self.next_temp();
        let right_tag = self.next_temp();
        let tags_equal = self.next_temp();
        writeln!(
            out,
            "  {left_tag} = extractvalue {} {}, 0",
            left.ty, left.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_tag} = extractvalue {} {}, 0",
            right.ty, right.repr
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {tags_equal} = icmp eq i32 {left_tag}, {right_tag}")
            .expect("writing to string cannot fail");

        let result_slot = self.next_temp();
        let tags_match_label = self.next_label("enum_eq_tags_match");
        let tags_differ_label = self.next_label("enum_eq_tags_differ");
        let done_label = self.next_label("enum_eq_done");
        let invalid_label = self.next_label("enum_eq_invalid_tag");
        writeln!(out, "  {result_slot} = alloca i1").expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {tags_equal}, label %{tags_match_label}, label %{tags_differ_label}"
        )
        .expect("writing to string cannot fail");

        let differ_value = if matches!(op, BinaryOp::Equal) {
            "0"
        } else {
            "1"
        };
        writeln!(out, "{tags_differ_label}:").expect("writing to string cannot fail");
        writeln!(out, "  store i1 {differ_value}, ptr {result_slot}")
            .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{tags_match_label}:").expect("writing to string cannot fail");
        let variant_blocks = layout
            .variants
            .iter()
            .map(|variant| {
                (
                    variant.clone(),
                    self.next_label(&format!("enum_eq_test_{}", variant.name)),
                    self.next_label(&format!("enum_eq_{}", variant.name)),
                )
            })
            .collect::<Vec<_>>();
        let first_test_label = variant_blocks
            .first()
            .map(|(_, test_label, _)| test_label.as_str())
            .unwrap_or(invalid_label.as_str());
        writeln!(out, "  br label %{first_test_label}").expect("writing to string cannot fail");

        for (index, (variant, test_label, variant_label)) in variant_blocks.iter().enumerate() {
            let next_label = variant_blocks
                .get(index + 1)
                .map(|(_, label, _)| label.as_str())
                .unwrap_or(invalid_label.as_str());
            writeln!(out, "{test_label}:").expect("writing to string cannot fail");
            let is_variant = self.next_temp();
            writeln!(
                out,
                "  {is_variant} = icmp eq i32 {left_tag}, {}",
                variant.tag
            )
            .expect("writing to string cannot fail");
            writeln!(
                out,
                "  br i1 {is_variant}, label %{variant_label}, label %{next_label}"
            )
            .expect("writing to string cannot fail");
            writeln!(out, "{variant_label}:").expect("writing to string cannot fail");
            let result = if let Some(payload_ax_ty) = &variant.payload_ax_ty {
                self.emit_enum_payload_equality(
                    op,
                    &left,
                    &right,
                    payload_ax_ty,
                    &layout.name,
                    &variant.name,
                    out,
                )?
            } else if matches!(op, BinaryOp::Equal) {
                "1".to_string()
            } else {
                "0".to_string()
            };
            writeln!(out, "  store i1 {result}, ptr {result_slot}")
                .expect("writing to string cannot fail");
            writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
        }

        writeln!(out, "{invalid_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @exit(i32 1)").expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load i1, ptr {result_slot}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i1".to_string(),
            repr: loaded,
            ax_ty: Some(Type::Bool),
        })
    }

    fn emit_enum_payload_equality(
        &mut self,
        op: BinaryOp,
        left: &LlvmValue,
        right: &LlvmValue,
        payload_ax_ty: &Type,
        enum_name: &str,
        variant_name: &str,
        out: &mut String,
    ) -> Result<String, String> {
        let payload_ty =
            llvm_type(payload_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "enum `{enum_name}.{variant_name}` payload type {} is outside LLVM AOT v0",
                    ax_type_name(payload_ax_ty)
                )
            })?;
        if !payload_equality_supported(payload_ax_ty, self.enum_layouts) {
            return Err(format!(
                "enum `{enum_name}.{variant_name}` payload type {} needs a native equality comparator before LLVM AOT can lower it",
                ax_type_name(payload_ax_ty)
            ));
        }

        let left_payload_ptr = self.next_temp();
        let right_payload_ptr = self.next_temp();
        let left_payload = self.next_temp();
        let right_payload = self.next_temp();
        writeln!(
            out,
            "  {left_payload_ptr} = extractvalue {} {}, 1",
            left.ty, left.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_payload_ptr} = extractvalue {} {}, 1",
            right.ty, right.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {left_payload} = load {payload_ty}, ptr {left_payload_ptr}"
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_payload} = load {payload_ty}, ptr {right_payload_ptr}"
        )
        .expect("writing to string cannot fail");

        let predicate = if matches!(op, BinaryOp::Equal) {
            "eq"
        } else {
            "ne"
        };
        if payload_ty == "ptr" {
            let compare = self.next_temp();
            let result = self.next_temp();
            writeln!(
                out,
                "  {compare} = call i32 @strcmp(ptr {left_payload}, ptr {right_payload})"
            )
            .expect("writing to string cannot fail");
            writeln!(out, "  {result} = icmp {predicate} i32 {compare}, 0")
                .expect("writing to string cannot fail");
            Ok(result)
        } else {
            let result = self.next_temp();
            writeln!(
                out,
                "  {result} = icmp {predicate} {payload_ty} {left_payload}, {right_payload}"
            )
            .expect("writing to string cannot fail");
            Ok(result)
        }
    }

    fn emit_call(
        &mut self,
        function: &str,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if function == "println" {
            return self.emit_println(arguments, out);
        }
        if function == "string_len" || function == "len" {
            return self.emit_string_len(function, arguments, out);
        }
        if matches!(
            function,
            "string_contains" | "string_starts_with" | "string_ends_with"
        ) {
            return self.emit_string_predicate(function, arguments, out);
        }
        if function == "to_string" {
            return self.emit_to_string(arguments, out);
        }

        let signature = self.signatures.get(function).ok_or_else(|| {
            format!(
                "call to `{function}` is outside LLVM AOT v0; only direct calls to AX functions in the same file are currently lowered"
            )
        })?;
        if signature.params.len() != arguments.len() {
            return Err(format!(
                "call to `{function}` has {} argument(s), but LLVM AOT expected {}",
                arguments.len(),
                signature.params.len()
            ));
        }

        let mut rendered_args = Vec::new();
        for ((argument, expected_ty), expected_ax_ty) in arguments
            .iter()
            .zip(&signature.params)
            .zip(&signature.param_ax_types)
        {
            let value = if matches!(expected_ax_ty, Type::Slice { .. }) {
                self.emit_slice_from_expr(argument, out)?
            } else {
                self.emit_expr(argument, out)?
            };
            ensure_same_type(expected_ty, &value.ty)?;
            rendered_args.push(format!("{} {}", value.ty, value.repr));
        }

        let temp = self.next_temp();
        writeln!(
            out,
            "  {temp} = call {} @{}({})",
            signature.return_type,
            signature.symbol,
            rendered_args.join(", ")
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: signature.return_type.clone(),
            repr: temp,
            ax_ty: Some(signature.return_ax_type.clone()),
        })
    }

    fn emit_string_len(
        &mut self,
        function: &str,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `{function}` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let value = self.emit_expr(&arguments[0], out)?;
        if function == "len" {
            if let Some(Type::Array { length, .. }) = value.ax_ty.as_ref() {
                return Ok(LlvmValue {
                    ty: "i32".to_string(),
                    repr: length.to_string(),
                    ax_ty: Some(Type::I32),
                });
            }
            if matches!(value.ax_ty.as_ref(), Some(Type::Slice { .. })) {
                ensure_same_type(&slice_llvm_type(), &value.ty)?;
                let len = self.next_temp();
                writeln!(out, "  {len} = extractvalue {} {}, 1", value.ty, value.repr)
                    .expect("writing to string cannot fail");
                return Ok(LlvmValue {
                    ty: "i32".to_string(),
                    repr: len,
                    ax_ty: Some(Type::I32),
                });
            }
        }
        if value.ty != "ptr" {
            return Err(format!(
                "{function}({}) needs a native layout or runtime ABI before LLVM AOT can lower it",
                value.ty
            ));
        }
        let temp = self.next_temp();
        writeln!(
            out,
            "  {temp} = call i32 @ax_string_len(ptr {})",
            value.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i32".to_string(),
            repr: temp,
            ax_ty: Some(Type::I32),
        })
    }

    fn emit_string_predicate(
        &mut self,
        function: &str,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `{function}` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }

        let text = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument(function, "text", &text)?;
        let pattern = self.emit_expr(&arguments[1], out)?;
        let pattern_name = match function {
            "string_contains" => "needle",
            "string_starts_with" => "prefix",
            "string_ends_with" => "suffix",
            _ => "pattern",
        };
        ensure_string_argument(function, pattern_name, &pattern)?;

        match function {
            "string_contains" => {
                let found = self.next_temp();
                writeln!(
                    out,
                    "  {found} = call ptr @strstr(ptr {}, ptr {})",
                    text.repr, pattern.repr
                )
                .expect("writing to string cannot fail");
                let result = self.next_temp();
                writeln!(out, "  {result} = icmp ne ptr {found}, null")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            "string_starts_with" => {
                let prefix_len = self.next_temp();
                writeln!(
                    out,
                    "  {prefix_len} = call i64 @strlen(ptr {})",
                    pattern.repr
                )
                .expect("writing to string cannot fail");
                let compare = self.next_temp();
                writeln!(
                    out,
                    "  {compare} = call i32 @strncmp(ptr {}, ptr {}, i64 {prefix_len})",
                    text.repr, pattern.repr
                )
                .expect("writing to string cannot fail");
                let result = self.next_temp();
                writeln!(out, "  {result} = icmp eq i32 {compare}, 0")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            "string_ends_with" => {
                let text_len = self.next_temp();
                writeln!(out, "  {text_len} = call i64 @strlen(ptr {})", text.repr)
                    .expect("writing to string cannot fail");
                let suffix_len = self.next_temp();
                writeln!(
                    out,
                    "  {suffix_len} = call i64 @strlen(ptr {})",
                    pattern.repr
                )
                .expect("writing to string cannot fail");
                let too_long = self.next_temp();
                writeln!(out, "  {too_long} = icmp ugt i64 {suffix_len}, {text_len}")
                    .expect("writing to string cannot fail");
                let compare_label = self.next_label("string_suffix_compare");
                let false_label = self.next_label("string_suffix_false");
                let done_label = self.next_label("string_suffix_done");
                writeln!(
                    out,
                    "  br i1 {too_long}, label %{false_label}, label %{compare_label}"
                )
                .expect("writing to string cannot fail");
                writeln!(out, "{compare_label}:").expect("writing to string cannot fail");
                let offset = self.next_temp();
                writeln!(out, "  {offset} = sub i64 {text_len}, {suffix_len}")
                    .expect("writing to string cannot fail");
                let suffix_start = self.next_temp();
                writeln!(
                    out,
                    "  {suffix_start} = getelementptr i8, ptr {}, i64 {offset}",
                    text.repr
                )
                .expect("writing to string cannot fail");
                let compare = self.next_temp();
                writeln!(
                    out,
                    "  {compare} = call i32 @strncmp(ptr {suffix_start}, ptr {}, i64 {suffix_len})",
                    pattern.repr
                )
                .expect("writing to string cannot fail");
                let matched = self.next_temp();
                writeln!(out, "  {matched} = icmp eq i32 {compare}, 0")
                    .expect("writing to string cannot fail");
                writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
                writeln!(out, "{false_label}:").expect("writing to string cannot fail");
                writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
                writeln!(out, "{done_label}:").expect("writing to string cannot fail");
                let result = self.next_temp();
                writeln!(
                    out,
                    "  {result} = phi i1 [{matched}, %{compare_label}], [0, %{false_label}]"
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            _ => Err(format!(
                "call to `{function}` is outside LLVM AOT v0 string predicate lowering"
            )),
        }
    }

    fn emit_to_string(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `to_string` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let value = self.emit_expr(&arguments[0], out)?;
        if is_enum_value(&value) {
            return Err(
                "to_string(enum) needs a native enum formatter before LLVM AOT can lower it"
                    .to_string(),
            );
        }
        match value.ty.as_str() {
            "i32" => {
                let temp = self.next_temp();
                writeln!(
                    out,
                    "  {temp} = call ptr @ax_i32_to_string(i32 {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "ptr".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::String),
                })
            }
            "i1" => {
                let temp = self.next_temp();
                writeln!(
                    out,
                    "  {temp} = select i1 {}, ptr @.ax_text_true, ptr @.ax_text_false",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "ptr".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::String),
                })
            }
            "ptr" => Ok(value),
            "void" => Err("to_string cannot format a `<void>` value in LLVM AOT v0".to_string()),
            other => Err(format!(
                "to_string({other}) needs a native runtime formatter before LLVM AOT can lower it"
            )),
        }
    }

    fn emit_println(&mut self, arguments: &[Expr], out: &mut String) -> Result<LlvmValue, String> {
        for (index, argument) in arguments.iter().enumerate() {
            if index > 0 {
                self.emit_printf_text("@.ax_text_space", out);
            }
            let value = self.emit_expr(argument, out)?;
            self.emit_print_value(value, out)?;
        }
        self.emit_printf_text("@.ax_text_newline", out);
        Ok(LlvmValue {
            ty: "void".to_string(),
            repr: String::new(),
            ax_ty: None,
        })
    }

    fn emit_print_value(&mut self, value: LlvmValue, out: &mut String) -> Result<(), String> {
        if is_enum_value(&value) {
            return Err(
                "println(enum) needs a native enum formatter before LLVM AOT can lower it"
                    .to_string(),
            );
        }
        match value.ty.as_str() {
            "i32" => {
                writeln!(
                    out,
                    "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_i32, i32 {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            "i1" => {
                let text = self.next_temp();
                writeln!(
                    out,
                    "  {text} = select i1 {}, ptr @.ax_text_true, ptr @.ax_text_false",
                    value.repr
                )
                .expect("writing to string cannot fail");
                writeln!(
                    out,
                    "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr {text})"
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            "ptr" => {
                writeln!(
                    out,
                    "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            "void" => Err("println cannot print a `<void>` value in LLVM AOT v0".to_string()),
            other => Err(format!(
                "println({other}) needs a native runtime ABI before LLVM AOT can lower it"
            )),
        }
    }

    fn emit_printf_text(&mut self, global: &str, out: &mut String) {
        writeln!(
            out,
            "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr {global})"
        )
        .expect("writing to string cannot fail");
    }

    fn infer_expr_value_type(&self, expr: &Expr) -> Result<(String, Type), String> {
        match &expr.kind {
            ExprKind::Int { .. } => Ok(("i32".to_string(), Type::I32)),
            ExprKind::Bool { .. } => Ok(("i1".to_string(), Type::Bool)),
            ExprKind::String { .. } => Ok(("ptr".to_string(), Type::String)),
            ExprKind::Local { local, .. } => {
                let slot = self.local_slot(*local)?;
                Ok((slot.ty.clone(), slot.ax_ty.clone()))
            }
            ExprKind::Unary { op, expr } => match op {
                UnaryOp::Negate => {
                    let (ty, _) = self.infer_expr_value_type(expr)?;
                    ensure_same_type("i32", &ty)?;
                    Ok(("i32".to_string(), Type::I32))
                }
                UnaryOp::Not => {
                    let (ty, _) = self.infer_expr_value_type(expr)?;
                    ensure_same_type("i1", &ty)?;
                    Ok(("i1".to_string(), Type::Bool))
                }
            },
            ExprKind::Binary { op, left, right } => {
                let (left_ty, left_ax_ty) = self.infer_expr_value_type(left)?;
                let (right_ty, _right_ax_ty) = self.infer_expr_value_type(right)?;
                ensure_same_type(&left_ty, &right_ty)?;
                match op {
                    BinaryOp::LogicalOr
                    | BinaryOp::LogicalAnd
                    | BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Ok(("i1".to_string(), Type::Bool)),
                    BinaryOp::Add if left_ax_ty == Type::String => {
                        Ok(("ptr".to_string(), Type::String))
                    }
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => Ok((left_ty, left_ax_ty)),
                }
            }
            ExprKind::Call {
                function,
                arguments,
            } => {
                if function == "println" {
                    return Err("println returns <void>, which cannot be a match value".to_string());
                }
                if function == "string_len" || function == "len" {
                    return Ok(("i32".to_string(), Type::I32));
                }
                if matches!(
                    function.as_str(),
                    "string_contains" | "string_starts_with" | "string_ends_with"
                ) {
                    return Ok(("i1".to_string(), Type::Bool));
                }
                if function == "to_string" {
                    return Ok(("ptr".to_string(), Type::String));
                }
                let signature = self.signatures.get(function).ok_or_else(|| {
                    format!(
                        "call to `{function}` is outside LLVM AOT v0; only direct calls to AX functions in the same file are currently lowered"
                    )
                })?;
                if signature.params.len() != arguments.len() {
                    return Err(format!(
                        "call to `{function}` has {} argument(s), but LLVM AOT expected {}",
                        arguments.len(),
                        signature.params.len()
                    ));
                }
                Ok((
                    signature.return_type.clone(),
                    signature.return_ax_type.clone(),
                ))
            }
            ExprKind::StructLiteral { name, .. } => {
                let ax_ty = Type::Struct { name: name.clone() };
                let ty = llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!("struct `{name}` is outside the LLVM AOT v0 layout subset")
                })?;
                Ok((ty, ax_ty))
            }
            ExprKind::ArrayLiteral { elements } => {
                let Some(first) = elements.first() else {
                    return Err(
                        "empty array literals need explicit native array type propagation before LLVM AOT can lower them"
                            .to_string(),
                    );
                };
                let (element_ty, element_ax_ty) = self.infer_expr_value_type(first)?;
                for element in &elements[1..] {
                    let (next_ty, _next_ax_ty) = self.infer_expr_value_type(element)?;
                    ensure_same_type(&element_ty, &next_ty)?;
                }
                let ax_ty = Type::Array {
                    element: Box::new(element_ax_ty),
                    length: elements.len(),
                };
                let ty = llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!("array type {} is outside LLVM AOT v0", ax_type_name(&ax_ty))
                })?;
                Ok((ty, ax_ty))
            }
            ExprKind::Block { value, .. } => self.infer_expr_value_type(value),
            ExprKind::Match { arms, .. } => {
                let Some(first) = arms.first() else {
                    return Err("match expression has no arms in LLVM AOT v0".to_string());
                };
                let (ty, ax_ty) = self.infer_expr_value_type(&first.value)?;
                for arm in &arms[1..] {
                    let (arm_ty, _) = self.infer_expr_value_type(&arm.value)?;
                    ensure_same_type(&ty, &arm_ty)?;
                }
                Ok((ty, ax_ty))
            }
            ExprKind::EnumVariant { enum_name, .. } => {
                let ax_ty = Type::Enum {
                    name: enum_name.clone(),
                };
                let ty = llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!("enum `{enum_name}` is outside the LLVM AOT v0 layout subset")
                })?;
                Ok((ty, ax_ty))
            }
            ExprKind::EnumPayload { value } => self.infer_enum_payload_type(value),
            ExprKind::Field { base, field } => {
                let (_, base_ax_ty) = self.infer_expr_value_type(base)?;
                let layout = self.struct_layout_for_type(&base_ax_ty)?;
                let field_layout = layout
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == *field)
                    .ok_or_else(|| {
                        format!(
                            "internal LLVM AOT error: struct `{}` has no field `{field}`",
                            layout.name
                        )
                    })?;
                Ok((field_layout.ty.clone(), field_layout.ax_ty.clone()))
            }
            ExprKind::Index { base, .. } => {
                let (_, base_ax_ty) = self.infer_expr_value_type(base)?;
                match base_ax_ty {
                    Type::Slice { element } => {
                        let element_ty = llvm_type(&element, self.layouts, self.enum_layouts)
                            .ok_or_else(|| {
                                format!(
                                    "slice element type {} is outside LLVM AOT v0",
                                    ax_type_name(&element)
                                )
                            })?;
                        Ok((element_ty, *element))
                    }
                    _ => {
                        let (_, element_ty, _, element_ax_ty) =
                            array_type_parts(&base_ax_ty, self.layouts, self.enum_layouts)?;
                        Ok((element_ty, element_ax_ty))
                    }
                }
            }
            ExprKind::Float { .. } => Err("f32 values are not in the LLVM AOT v0 subset".into()),
            ExprKind::Const { name } => {
                let binding = self
                    .consts
                    .get(name)
                    .ok_or_else(|| format!("const `{name}` was not found for LLVM AOT lowering"))?;
                let ty =
                    llvm_type(&binding.ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                        format!(
                            "const `{name}` type {} is outside LLVM AOT v0",
                            ax_type_name(&binding.ty)
                        )
                    })?;
                Ok((ty, binding.ty.clone()))
            }
            ExprKind::Try { expr } => {
                let (_, result_ax_ty) = self.infer_expr_value_type(expr)?;
                let (success_ax_ty, _) = self.result_success_error_types(&result_ax_ty)?;
                let success_ty = llvm_type(&success_ax_ty, self.layouts, self.enum_layouts)
                    .ok_or_else(|| {
                        format!(
                            "`?` success type {} is outside LLVM AOT v0",
                            ax_type_name(&success_ax_ty)
                        )
                    })?;
                Ok((success_ty, success_ax_ty))
            }
            ExprKind::MatchTest { .. } => Ok(("i1".to_string(), Type::Bool)),
            ExprKind::Slice { base, .. } => {
                let (_, base_ax_ty) = self.infer_expr_value_type(base)?;
                let element = match base_ax_ty {
                    Type::Array { element, .. } | Type::Slice { element } => element,
                    other => {
                        return Err(format!(
                            "slice expression base type {} is outside LLVM AOT v0",
                            ax_type_name(&other)
                        ));
                    }
                };
                let ax_ty = Type::Slice { element };
                let ty = llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!("slice type {} is outside LLVM AOT v0", ax_type_name(&ax_ty))
                })?;
                Ok((ty, ax_ty))
            }
        }
    }

    fn infer_enum_payload_type(&self, value: &Expr) -> Result<(String, Type), String> {
        let (_, value_ax_ty) = self.infer_expr_value_type(value)?;
        let enum_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. }) = value_ax_ty else {
            return Err(
                "enum payload extraction requires an enum value in LLVM AOT v0".to_string(),
            );
        };
        let enum_name = enum_base_name(&enum_ax_ty).to_string();
        let layout = self.enum_layout_for_type(&enum_ax_ty)?;
        let payload_types = layout
            .variants
            .iter()
            .filter_map(|variant| variant.payload_ax_ty.clone())
            .collect::<Vec<_>>();
        let payload_ax_ty = payload_types
            .first()
            .cloned()
            .ok_or_else(|| format!("enum `{enum_name}` has no payload variant in LLVM AOT v0"))?;
        if payload_types
            .iter()
            .any(|payload| payload != &payload_ax_ty)
        {
            return Err(format!(
                "enum `{enum_name}` payload extraction needs an expected type in LLVM AOT v0"
            ));
        }
        let payload_ty =
            llvm_type(&payload_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "enum `{enum_name}` payload type {} is outside LLVM AOT v0",
                    ax_type_name(&payload_ax_ty)
                )
            })?;
        Ok((payload_ty, payload_ax_ty))
    }

    fn result_success_error_types(&self, ty: &Type) -> Result<(Type, Type), String> {
        let Type::EnumInstance { .. } = ty else {
            return Err(format!(
                "`?` result propagation requires a concrete Result<T, E> enum instance, found {}",
                ax_type_name(ty)
            ));
        };
        let layout = self.enum_layout_for_type(ty)?;
        Ok((
            self.enum_variant_payload_type(layout, "Ok")?,
            self.enum_variant_payload_type(layout, "Err")?,
        ))
    }

    fn enum_variant_payload_type(
        &self,
        layout: &EnumLayout,
        variant: &str,
    ) -> Result<Type, String> {
        layout
            .variants
            .iter()
            .find(|candidate| candidate.name == variant)
            .and_then(|candidate| candidate.payload_ax_ty.clone())
            .ok_or_else(|| {
                format!(
                    "enum `{}` must have payload variant `{variant}` for `?` lowering in LLVM AOT v0",
                    layout.name
                )
            })
    }

    fn enum_variant_tag(&self, layout: &EnumLayout, variant: &str) -> Result<i32, String> {
        layout
            .variants
            .iter()
            .find(|candidate| candidate.name == variant)
            .map(|candidate| candidate.tag)
            .ok_or_else(|| {
                format!(
                    "enum `{}` must have variant `{variant}` for `?` lowering in LLVM AOT v0",
                    layout.name
                )
            })
    }

    fn local_slot(&self, local: u32) -> Result<&LocalSlot, String> {
        self.locals
            .get(&local)
            .ok_or_else(|| format!("internal LLVM AOT error: missing local slot {local}"))
    }

    fn emit_place_ptr(&mut self, place: &Place, out: &mut String) -> Result<LocalSlot, String> {
        match &place.kind {
            PlaceKind::Local { local, .. } => self.local_slot(*local).cloned(),
            PlaceKind::Field { base, field } => {
                let base_slot = self.emit_place_ptr(base, out)?;
                let layout = self.struct_layout_for_type(&base_slot.ax_ty)?.clone();
                ensure_same_type(&layout.ty, &base_slot.ty)?;
                let field_layout = layout
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == *field)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "internal LLVM AOT error: struct `{}` has no field `{field}`",
                            layout.name
                        )
                    })?;
                let field_ptr = self.next_temp();
                writeln!(
                    out,
                    "  {field_ptr} = getelementptr {}, ptr {}, i32 0, i32 {}",
                    layout.ty, base_slot.ptr, field_layout.index
                )
                .expect("writing to string cannot fail");
                Ok(LocalSlot {
                    ptr: field_ptr,
                    ty: field_layout.ty,
                    ax_ty: field_layout.ax_ty,
                })
            }
            PlaceKind::Index { base, index } => {
                let base_slot = self.emit_place_ptr(base, out)?;
                let (array_ty, element_ty, length, element_ax_ty) =
                    array_type_parts(&base_slot.ax_ty, self.layouts, self.enum_layouts)?;
                ensure_same_type(&array_ty, &base_slot.ty)?;
                let index = self.emit_expr(index, out)?;
                ensure_same_type("i32", &index.ty)?;
                self.emit_fixed_bounds_check(&index.repr, length, out);
                let element_ptr = self.next_temp();
                writeln!(
                    out,
                    "  {element_ptr} = getelementptr {array_ty}, ptr {}, i32 0, i32 {}",
                    base_slot.ptr, index.repr
                )
                .expect("writing to string cannot fail");
                Ok(LocalSlot {
                    ptr: element_ptr,
                    ty: element_ty,
                    ax_ty: element_ax_ty,
                })
            }
        }
    }

    fn struct_layout_by_name(&self, name: &str) -> Result<&StructLayout, String> {
        self.layouts
            .get(name)
            .ok_or_else(|| format!("struct `{name}` is outside the LLVM AOT v0 layout subset"))
    }

    fn struct_layout_for_type(&self, ty: &Type) -> Result<&StructLayout, String> {
        match ty {
            Type::Struct { name } => self.struct_layout_by_name(name),
            Type::StructInstance { name, .. } => Err(format!(
                "generic struct instance `{name}` needs monomorphized native layout before LLVM AOT can lower it"
            )),
            _ => Err(format!(
                "field access base type {} is not a struct in LLVM AOT v0",
                ax_type_name(ty)
            )),
        }
    }

    fn enum_layout_for_type(&self, ty: &Type) -> Result<&EnumLayout, String> {
        self.enum_layouts.get(&enum_layout_key(ty)).ok_or_else(|| {
            format!(
                "enum `{}` is outside the LLVM AOT v0 layout subset",
                ax_type_name(ty)
            )
        })
    }

    fn enum_layout_for_value(&self, value: &LlvmValue) -> Result<&EnumLayout, String> {
        let Some(enum_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. })) =
            value.ax_ty.as_ref()
        else {
            return Err("enum operation requires an enum value in LLVM AOT v0".to_string());
        };
        self.enum_layout_for_type(enum_ax_ty)
    }

    fn enum_layout_for_constructor(
        &self,
        enum_name: &str,
        expected_ax_ty: Option<&Type>,
    ) -> Result<&EnumLayout, String> {
        if let Some(expected @ (Type::Enum { .. } | Type::EnumInstance { .. })) = expected_ax_ty {
            if enum_base_name(expected) != enum_name {
                return Err(format!(
                    "enum constructor `{enum_name}` does not match expected type {} in LLVM AOT v0",
                    ax_type_name(expected)
                ));
            }
            return self.enum_layout_for_type(expected);
        }

        let mut candidates = self
            .enum_layouts
            .values()
            .filter(|layout| layout.name == enum_name);
        let Some(first) = candidates.next() else {
            return Err(format!(
                "enum `{enum_name}` is outside the LLVM AOT v0 layout subset"
            ));
        };
        if candidates.next().is_some() {
            return Err(format!(
                "generic enum constructor `{enum_name}` needs an expected concrete enum type before LLVM AOT can lower it"
            ));
        }
        Ok(first)
    }

    fn enum_value_type_for_layout(
        &self,
        layout: &EnumLayout,
        expected_ax_ty: Option<&Type>,
    ) -> Result<Type, String> {
        if let Some(expected @ (Type::Enum { .. } | Type::EnumInstance { .. })) = expected_ax_ty {
            return Ok(expected.clone());
        }

        self.enum_layouts
            .values()
            .find_map(|candidate| {
                if candidate.ty == layout.ty && candidate.name == layout.name {
                    Some(candidate.ax_ty.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                format!(
                    "internal LLVM AOT error: missing enum value type for layout `{}`",
                    layout.ty
                )
            })
    }

    fn next_temp(&mut self) -> String {
        let temp = format!("%t{}", self.temp_index);
        self.temp_index += 1;
        temp
    }

    fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.temp_index);
        self.temp_index += 1;
        label
    }
}

fn collect_string_literals(program: &Program) -> BTreeMap<String, StringLiteral> {
    let mut strings = BTreeMap::new();
    let mut next_id = 0;
    for item in &program.items {
        match &item.kind {
            ItemKind::Function { blocks, .. } => {
                for block in blocks {
                    for statement in &block.statements {
                        collect_statement_string_literals(statement, &mut strings, &mut next_id);
                    }
                    collect_terminator_string_literals(
                        &block.terminator.kind,
                        &mut strings,
                        &mut next_id,
                    );
                }
            }
            ItemKind::Const { value, .. } => {
                collect_expr_string_literals(value, &mut strings, &mut next_id);
            }
            ItemKind::Struct { .. } | ItemKind::Enum { .. } => {}
        }
    }
    strings
}

fn collect_statement_string_literals(
    statement: &Statement,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    match &statement.kind {
        StatementKind::Let { initializer, .. } | StatementKind::Eval { expr: initializer } => {
            collect_expr_string_literals(initializer, strings, next_id);
        }
        StatementKind::Assign { target, value } => {
            collect_place_string_literals(target, strings, next_id);
            collect_expr_string_literals(value, strings, next_id);
        }
    }
}

fn collect_terminator_string_literals(
    terminator: &TerminatorKind,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    match terminator {
        TerminatorKind::Branch { condition, .. } | TerminatorKind::Return { value: condition } => {
            collect_expr_string_literals(condition, strings, next_id);
        }
        TerminatorKind::Goto { .. } | TerminatorKind::Unreachable => {}
    }
}

fn collect_place_string_literals(
    place: &Place,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    match &place.kind {
        PlaceKind::Local { .. } => {}
        PlaceKind::Field { base, .. } => collect_place_string_literals(base, strings, next_id),
        PlaceKind::Index { base, index } => {
            collect_place_string_literals(base, strings, next_id);
            collect_expr_string_literals(index, strings, next_id);
        }
    }
}

fn collect_expr_string_literals(
    expr: &Expr,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    match &expr.kind {
        ExprKind::String { value } => {
            if !strings.contains_key(value) {
                let symbol = format!("@.ax_str_{}", *next_id);
                *next_id += 1;
                strings.insert(
                    value.clone(),
                    StringLiteral {
                        symbol,
                        len: value.len() + 1,
                        encoded: encode_llvm_c_string(value),
                    },
                );
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Try { expr }
        | ExprKind::EnumPayload { value: expr }
        | ExprKind::Field { base: expr, .. } => {
            collect_expr_string_literals(expr, strings, next_id);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_string_literals(left, strings, next_id);
            collect_expr_string_literals(right, strings, next_id);
        }
        ExprKind::Call { arguments, .. }
        | ExprKind::ArrayLiteral {
            elements: arguments,
        } => {
            for argument in arguments {
                collect_expr_string_literals(argument, strings, next_id);
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_expr_string_literals(&field.value, strings, next_id);
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_statement_string_literals(statement, strings, next_id);
            }
            collect_expr_string_literals(value, strings, next_id);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_string_literals(scrutinee, strings, next_id);
            for arm in arms {
                collect_match_pattern_string_literals(&arm.pattern, strings, next_id);
                if let Some(guard) = &arm.guard {
                    collect_expr_string_literals(guard, strings, next_id);
                }
                collect_expr_string_literals(&arm.value, strings, next_id);
            }
        }
        ExprKind::EnumVariant { payload, .. } => {
            if let Some(payload) = payload {
                collect_expr_string_literals(payload, strings, next_id);
            }
        }
        ExprKind::MatchTest { scrutinee, pattern } => {
            collect_expr_string_literals(scrutinee, strings, next_id);
            collect_match_pattern_string_literals(pattern, strings, next_id);
        }
        ExprKind::Index { base, index } => {
            collect_expr_string_literals(base, strings, next_id);
            collect_expr_string_literals(index, strings, next_id);
        }
        ExprKind::Slice { base, start, end } => {
            collect_expr_string_literals(base, strings, next_id);
            collect_expr_string_literals(start, strings, next_id);
            collect_expr_string_literals(end, strings, next_id);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::Local { .. }
        | ExprKind::Const { .. } => {}
    }
}

fn collect_match_pattern_string_literals(
    pattern: &MatchPattern,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    match &pattern.kind {
        MatchPatternKind::String { value } => {
            if !strings.contains_key(value) {
                let symbol = format!("@.ax_str_{}", *next_id);
                *next_id += 1;
                strings.insert(
                    value.clone(),
                    StringLiteral {
                        symbol,
                        len: value.len() + 1,
                        encoded: encode_llvm_c_string(value),
                    },
                );
            }
        }
        MatchPatternKind::Or { alternatives } => {
            for alternative in alternatives {
                collect_match_pattern_string_literals(alternative, strings, next_id);
            }
        }
        MatchPatternKind::Wildcard
        | MatchPatternKind::Binding { .. }
        | MatchPatternKind::Bool { .. }
        | MatchPatternKind::Int { .. }
        | MatchPatternKind::IntRange { .. }
        | MatchPatternKind::EnumVariant { .. }
        | MatchPatternKind::Struct { .. }
        | MatchPatternKind::Error => {}
    }
}

fn find_local_use_by_name_in_match_arm(arm: &MatchExprArm, name: &str) -> Option<u32> {
    arm.guard
        .as_ref()
        .and_then(|guard| find_local_use_by_name(guard, name))
        .or_else(|| find_local_use_by_name(&arm.value, name))
}

fn find_local_use_by_name(expr: &Expr, name: &str) -> Option<u32> {
    match &expr.kind {
        ExprKind::Local {
            local,
            name: local_name,
        } if local_name == name => Some(*local),
        ExprKind::Unary { expr, .. }
        | ExprKind::Try { expr }
        | ExprKind::EnumPayload { value: expr }
        | ExprKind::Field { base: expr, .. } => find_local_use_by_name(expr, name),
        ExprKind::Binary { left, right, .. } => {
            find_local_use_by_name(left, name).or_else(|| find_local_use_by_name(right, name))
        }
        ExprKind::Call { arguments, .. }
        | ExprKind::ArrayLiteral {
            elements: arguments,
        } => arguments
            .iter()
            .find_map(|argument| find_local_use_by_name(argument, name)),
        ExprKind::StructLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| find_local_use_by_name(&field.value, name)),
        ExprKind::Block { statements, value } => statements
            .iter()
            .find_map(|statement| find_local_use_by_name_in_statement(statement, name))
            .or_else(|| find_local_use_by_name(value, name)),
        ExprKind::Match { scrutinee, arms } => {
            find_local_use_by_name(scrutinee, name).or_else(|| {
                arms.iter()
                    .find_map(|arm| find_local_use_by_name_in_match_arm(arm, name))
            })
        }
        ExprKind::EnumVariant { payload, .. } => payload
            .as_ref()
            .and_then(|payload| find_local_use_by_name(payload, name)),
        ExprKind::MatchTest { scrutinee, .. } => find_local_use_by_name(scrutinee, name),
        ExprKind::Index { base, index } => {
            find_local_use_by_name(base, name).or_else(|| find_local_use_by_name(index, name))
        }
        ExprKind::Slice { base, start, end } => find_local_use_by_name(base, name)
            .or_else(|| find_local_use_by_name(start, name))
            .or_else(|| find_local_use_by_name(end, name)),
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Local { .. }
        | ExprKind::Const { .. } => None,
    }
}

fn find_local_use_by_name_in_statement(statement: &Statement, name: &str) -> Option<u32> {
    match &statement.kind {
        StatementKind::Let { initializer, .. } | StatementKind::Eval { expr: initializer } => {
            find_local_use_by_name(initializer, name)
        }
        StatementKind::Assign { target, value } => find_local_use_by_name_in_place(target, name)
            .or_else(|| find_local_use_by_name(value, name)),
    }
}

fn find_local_use_by_name_in_place(place: &Place, name: &str) -> Option<u32> {
    match &place.kind {
        PlaceKind::Local {
            local,
            name: local_name,
        } if local_name == name => Some(*local),
        PlaceKind::Field { base, .. } => find_local_use_by_name_in_place(base, name),
        PlaceKind::Index { base, index } => find_local_use_by_name_in_place(base, name)
            .or_else(|| find_local_use_by_name(index, name)),
        PlaceKind::Local { .. } => None,
    }
}

fn encode_llvm_c_string(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'\\' => encoded.push_str("\\5C"),
            b'"' => encoded.push_str("\\22"),
            0x20..=0x7e => encoded.push(*byte as char),
            other => write!(encoded, "\\{other:02X}").expect("writing to string cannot fail"),
        }
    }
    encoded.push_str("\\00");
    encoded
}

fn ensure_same_type(expected: &str, actual: &str) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "internal LLVM AOT type mismatch: expected {expected}, got {actual}"
        ))
    }
}

fn ensure_string_argument(function: &str, name: &str, value: &LlvmValue) -> Result<(), String> {
    if value.ty == "ptr" && matches!(value.ax_ty.as_ref(), Some(Type::String)) {
        Ok(())
    } else {
        Err(format!(
            "`{function}` argument `{name}` must be `string` in LLVM AOT v0"
        ))
    }
}

fn is_enum_value(value: &LlvmValue) -> bool {
    matches!(
        value.ax_ty.as_ref(),
        Some(Type::Enum { .. } | Type::EnumInstance { .. })
    )
}

fn payload_equality_supported(ty: &Type, enum_layouts: &BTreeMap<String, EnumLayout>) -> bool {
    match ty {
        Type::Bool | Type::I32 | Type::String => true,
        Type::Enum { .. } | Type::EnumInstance { .. } => enum_layouts
            .get(&enum_layout_key(ty))
            .is_some_and(|layout| layout.ty == "i32"),
        Type::F32
        | Type::StringList
        | Type::Slice { .. }
        | Type::Array { .. }
        | Type::Struct { .. }
        | Type::StructInstance { .. }
        | Type::TypeParam { .. } => false,
    }
}

fn match_pattern_contains_binding(pattern: &MatchPattern) -> bool {
    match &pattern.kind {
        MatchPatternKind::Binding { .. } => true,
        MatchPatternKind::EnumVariant {
            payload: Some(EnumVariantPayloadPattern::Binding { .. }),
            ..
        } => true,
        MatchPatternKind::Struct { fields, .. } => !fields.is_empty(),
        MatchPatternKind::Or { alternatives } => {
            alternatives.iter().any(match_pattern_contains_binding)
        }
        MatchPatternKind::Wildcard
        | MatchPatternKind::Bool { .. }
        | MatchPatternKind::Int { .. }
        | MatchPatternKind::IntRange { .. }
        | MatchPatternKind::String { .. }
        | MatchPatternKind::EnumVariant { .. }
        | MatchPatternKind::Error => false,
    }
}

fn llvm_binary_op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::LogicalOr => "||",
        BinaryOp::LogicalAnd => "&&",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Remainder => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
    }
}

fn llvm_type(
    ty: &Type,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Option<String> {
    match ty {
        Type::Bool => Some("i1".to_string()),
        Type::I32 => Some("i32".to_string()),
        Type::String => Some("ptr".to_string()),
        Type::Array { element, length } => {
            let element_ty = llvm_type(element, layouts, enum_layouts)?;
            Some(format!("[{length} x {element_ty}]"))
        }
        Type::Slice { element } => {
            llvm_type(element, layouts, enum_layouts)?;
            Some(slice_llvm_type())
        }
        Type::Struct { name } => layouts.get(name).map(|layout| layout.ty.clone()),
        Type::Enum { .. } | Type::EnumInstance { .. } => enum_layouts
            .get(&enum_layout_key(ty))
            .map(|layout| layout.ty.clone()),
        Type::F32 | Type::StringList | Type::StructInstance { .. } | Type::TypeParam { .. } => None,
    }
}

fn slice_llvm_type() -> String {
    "{ ptr, i32 }".to_string()
}

fn enum_layout_for_static_type<'a>(
    ty: &Type,
    enum_layouts: &'a BTreeMap<String, EnumLayout>,
) -> Option<&'a EnumLayout> {
    match ty {
        Type::Enum { .. } | Type::EnumInstance { .. } => enum_layouts.get(&enum_layout_key(ty)),
        _ => None,
    }
}

fn array_type_parts(
    ty: &Type,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Result<(String, String, usize, Type), String> {
    let Type::Array { element, length } = ty else {
        return Err("array index base is not a fixed-size array in LLVM AOT v0".to_string());
    };
    let array_ty = llvm_type(ty, layouts, enum_layouts)
        .ok_or_else(|| format!("array type {} is outside LLVM AOT v0", ax_type_name(ty)))?;
    let element_ty = llvm_type(element, layouts, enum_layouts).ok_or_else(|| {
        format!(
            "array element type {} is outside LLVM AOT v0",
            ax_type_name(element)
        )
    })?;
    Ok((array_ty, element_ty, *length, element.as_ref().clone()))
}

fn llvm_alloc_size(
    ty: &Type,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Option<usize> {
    match ty {
        Type::Bool => Some(1),
        Type::I32 => Some(4),
        Type::String => Some(8),
        Type::Enum { .. } | Type::EnumInstance { .. } => enum_layouts
            .get(&enum_layout_key(ty))
            .and_then(|layout| if layout.ty == "i32" { Some(4) } else { None }),
        Type::Array { element, length } => {
            llvm_alloc_size(element, layouts, enum_layouts).map(|size| size * length)
        }
        Type::Struct { .. } => None,
        Type::F32
        | Type::StringList
        | Type::Slice { .. }
        | Type::StructInstance { .. }
        | Type::TypeParam { .. } => None,
    }
}

fn llvm_struct_type_name(name: &str) -> String {
    let mut symbol = String::from("%ax_struct_");
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            symbol.push(ch);
        } else {
            symbol.push('_');
        }
    }
    symbol
}

fn llvm_enum_type_name(name: &str) -> String {
    let mut symbol = String::from("%ax_enum_");
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            symbol.push(ch);
        } else {
            symbol.push('_');
        }
    }
    symbol
}

fn llvm_enum_type_name_for_type(ty: &Type) -> String {
    llvm_enum_type_name(&ax_type_name(ty))
}

fn enum_layout_key(ty: &Type) -> String {
    ax_type_name(ty)
}

fn enum_base_name(ty: &Type) -> &str {
    match ty {
        Type::Enum { name } | Type::EnumInstance { name, .. } => name,
        _ => "<non-enum>",
    }
}

fn llvm_symbol(name: &str) -> String {
    if name == "main" {
        return "main".to_string();
    }

    let mut symbol = String::from("ax_");
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            symbol.push(ch);
        } else {
            symbol.push('_');
        }
    }
    symbol
}

fn ax_type_name(ty: &Type) -> String {
    match ty {
        Type::Bool => "bool".to_string(),
        Type::I32 => "i32".to_string(),
        Type::F32 => "f32".to_string(),
        Type::String => "string".to_string(),
        Type::StringList => "string_list".to_string(),
        Type::Slice { element } => format!("[]{}", ax_type_name(element)),
        Type::Array { element, length } => format!("[{}; {}]", ax_type_name(element), length),
        Type::Struct { name } => name.clone(),
        Type::StructInstance { name, args } | Type::EnumInstance { name, args } => format!(
            "{}<{}>",
            name,
            args.iter().map(ax_type_name).collect::<Vec<_>>().join(", ")
        ),
        Type::Enum { name } => name.clone(),
        Type::TypeParam { name } => name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::render_program;
    use crate::frontend::analyze;
    use crate::source::SourceFile;

    fn render(source_text: &str) -> String {
        let source = SourceFile::anonymous(source_text);
        let output = analyze(&source);
        assert!(
            output.diagnostics.is_empty(),
            "source should analyze before LLVM IR rendering: {:?}",
            output.diagnostics
        );
        render_program(output.mir.as_ref().expect("MIR should exist"))
            .expect("LLVM IR should render")
    }

    #[test]
    fn renders_minimal_main_return() {
        let rendered = render(
            "\
fn main() -> i32 {
    return 0;
}
",
        );

        assert!(rendered.contains("define i32 @main()"));
        assert!(rendered.contains("ret i32 0"));
    }

    #[test]
    fn renders_i32_function_calls_and_arithmetic() {
        let rendered = render(
            "\
fn add(left: i32, right: i32) -> i32 {
    return left + right;
}

fn main() -> i32 {
    return add(1, 2);
}
",
        );

        assert!(rendered.contains("define i32 @ax_add(i32 %arg0, i32 %arg1)"));
        assert!(rendered.contains("= add i32"));
        assert!(rendered.contains("= call i32 @ax_add(i32 1, i32 2)"));
    }

    #[test]
    fn renders_i32_and_bool_println_calls() {
        let rendered = render(
            "\
fn main() -> i32 {
    println(7);
    println(true);
    return 0;
}
",
        );

        assert!(rendered.contains("declare i32 @printf(ptr, ...)"));
        assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_i32, i32 7)"));
        assert!(rendered.contains("select i1 1, ptr @.ax_text_true, ptr @.ax_text_false"));
    }

    #[test]
    fn renders_string_literal_println_calls() {
        let rendered = render(
            "\
fn main() -> i32 {
    println(\"hello\");
    println(\"C:\\\\AX\");
    return 0;
}
",
        );

        assert!(
            rendered.contains("@.ax_str_0 = private unnamed_addr constant [6 x i8] c\"hello\\00\"")
        );
        assert!(
            rendered
                .contains("@.ax_str_1 = private unnamed_addr constant [6 x i8] c\"C:\\5CAX\\00\"")
        );
        assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr @.ax_str_0)"));
    }

    #[test]
    fn renders_top_level_i32_bool_and_string_consts() {
        let rendered = render(
            "\
const EXIT_OK: i32 = 7;
const ENABLED: bool = true;
const LABEL: string = \"const-ready\";

fn main() -> i32 {
    if (ENABLED) {
        println(LABEL);
    }
    return EXIT_OK;
}
",
        );

        assert!(
            rendered.contains(
                "@.ax_str_0 = private unnamed_addr constant [12 x i8] c\"const-ready\\00\""
            )
        );
        assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr @.ax_str_0)"));
        assert!(rendered.contains("br i1 1, label"));
        assert!(rendered.contains("ret i32 7"));
    }

    #[test]
    fn renders_string_locals_params_and_return_values() {
        let rendered = render(
            "\
fn choose(left: string, right: string) -> string {
    return right;
}

fn main() -> i32 {
    let text: string = choose(\"ignored\", \"kept\");
    println(text);
    return 0;
}
",
        );

        assert!(rendered.contains("define ptr @ax_choose(ptr %arg0, ptr %arg1)"));
        assert!(rendered.contains("store ptr %arg0, ptr %local"));
        assert!(rendered.contains("ret ptr %t"));
        assert!(rendered.contains("= call ptr @ax_choose(ptr @.ax_str_0, ptr @.ax_str_1)"));
        assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %t"));
    }

    #[test]
    fn renders_string_len_and_content_comparisons() {
        let rendered = render(
            "\
fn same(left: string, right: string) -> bool {
    return left == right;
}

fn main() -> i32 {
    let text: string = \"AX\";
    if (same(text, \"AX\") && text != \"BY\") {
        return string_len(text) + len(\"tool\");
    }
    return 1;
}
",
        );

        assert!(rendered.contains("declare i32 @strcmp(ptr, ptr)"));
        assert!(rendered.contains("define private i32 @ax_string_len(ptr %text)"));
        assert!(rendered.contains("call i32 @strcmp(ptr"));
        assert!(rendered.contains("icmp eq i32"));
        assert!(rendered.contains("icmp ne i32"));
        assert!(rendered.contains("call i32 @ax_string_len(ptr"));
    }

    #[test]
    fn renders_string_concat_and_to_string_values() {
        let rendered = render(
            "\
fn describe(value: i32, enabled: bool, label: string) -> string {
    return label + \"=\" + to_string(value) + \", enabled=\" + to_string(enabled);
}

fn main() -> i32 {
    let message: string = describe(7, true, \"count\") + \" done\";
    println(message);
    return string_len(message);
}
",
        );

        assert!(rendered.contains("declare ptr @malloc(i64)"));
        assert!(rendered.contains("declare ptr @memcpy(ptr, ptr, i64)"));
        assert!(rendered.contains("declare i32 @snprintf(ptr, i64, ptr, ...)"));
        assert!(rendered.contains("define private ptr @ax_string_concat(ptr %left, ptr %right)"));
        assert!(rendered.contains("define private ptr @ax_i32_to_string(i32 %value)"));
        assert!(rendered.contains("call ptr @ax_string_concat(ptr"));
        assert!(rendered.contains("call ptr @ax_i32_to_string(i32"));
        assert!(rendered.contains("select i1"));
    }

    #[test]
    fn renders_string_predicate_builtins() {
        let rendered = render(
            "\
fn main() -> i32 {
    let text: string = \"AX compiler\";
    if (string_contains(text, \"comp\") && string_starts_with(text, \"AX\") && string_ends_with(text, \"iler\")) {
        return 17;
    }
    return 1;
}
",
        );

        assert!(rendered.contains("declare ptr @strstr(ptr, ptr)"));
        assert!(rendered.contains("declare i32 @strncmp(ptr, ptr, i64)"));
        assert!(rendered.contains("call ptr @strstr(ptr"));
        assert!(rendered.contains("call i32 @strncmp(ptr"));
        assert!(rendered.contains("string_suffix_compare"));
        assert!(rendered.contains("phi i1"));
    }

    #[test]
    fn renders_fixed_array_literals_index_reads_and_len() {
        let rendered = render(
            "\
fn pick(values: [i32; 4], index: i32) -> i32 {
    return values[index];
}

fn main() -> i32 {
    let values: [i32; 4] = [3, 5, 8, 13];
    return values[0] + pick(values, len(values) - 1);
}
",
        );

        assert!(rendered.contains("define i32 @ax_pick([4 x i32] %arg0, i32 %arg1)"));
        assert!(rendered.contains("insertvalue [4 x i32] undef, i32 3, 0"));
        assert!(rendered.contains("store [4 x i32] %t"));
        assert!(rendered.contains("getelementptr [4 x i32], ptr %local"));
        assert!(rendered.contains("icmp slt i32"));
        assert!(rendered.contains("icmp sge i32"));
        assert!(rendered.contains("call i32 @ax_pick([4 x i32]"));
    }

    #[test]
    fn renders_fixed_array_element_assignments() {
        let rendered = render(
            "\
fn main() -> i32 {
    let mut values: [i32; 3] = [1, 2, 3];
    values[1] = values[0] + 8;
    return values[1];
}
",
        );

        assert!(rendered.contains("insertvalue [3 x i32] undef, i32 1, 0"));
        assert!(rendered.contains("getelementptr [3 x i32], ptr %local"));
        assert!(rendered.contains("store i32 %t"));
        assert!(rendered.contains("icmp slt i32 1, 0"));
        assert!(rendered.contains("icmp sge i32 1, 3"));
    }

    #[test]
    fn renders_for_in_over_fixed_arrays_with_read_only_slice_v0() {
        let rendered = render(
            "\
fn main() -> i32 {
    let values: [i32; 3] = [2, 4, 6];
    let mut total: i32 = 0;
    for (let value: i32 in values) {
        total = total + value;
    }
    return total;
}
",
        );

        assert!(rendered.contains("insertvalue { ptr, i32 } undef"));
        assert!(rendered.contains("extractvalue { ptr, i32 }"));
        assert!(rendered.contains("getelementptr i32, ptr"));
        assert!(rendered.contains("ret i32 %"));
    }

    #[test]
    fn renders_slice_range_reads_with_read_only_slice_v0() {
        let rendered = render(
            "\
fn sum_pair(values: [i32]) -> i32 {
    return values[0] + values[1];
}

fn main() -> i32 {
    let values: [i32; 5] = [1, 2, 3, 4, 5];
    let middle: [i32] = values[1:4];
    return len(middle) + middle[0] + middle[2] + sum_pair(values[2:4]) + sum_pair(values);
}
",
        );

        assert!(rendered.contains("define i32 @ax_sum_pair({ ptr, i32 } %arg0)"));
        assert!(rendered.contains("icmp sgt i32"));
        assert!(rendered.contains("slice_order_invalid"));
        assert!(rendered.contains("sub i32"));
        assert!(rendered.contains("call i32 @ax_sum_pair({ ptr, i32 }"));
        assert!(rendered.contains("ret i32 %"));
    }

    #[test]
    fn renders_struct_literals_params_returns_and_field_reads() {
        let rendered = render(
            "\
struct Point {
    x: i32,
    y: i32,
}

fn shift(point: Point, delta: i32) -> Point {
    return Point { x: point.x + delta, y: point.y + delta };
}

fn main() -> i32 {
    let point: Point = shift(Point { y: 5, x: 2 }, 3);
    return point.x + point.y;
}
",
        );

        assert!(rendered.contains("%ax_struct_Point = type { i32, i32 }"));
        assert!(
            rendered
                .contains("define %ax_struct_Point @ax_shift(%ax_struct_Point %arg0, i32 %arg1)")
        );
        assert!(rendered.contains("insertvalue %ax_struct_Point undef, i32 2, 0"));
        assert!(rendered.contains("insertvalue %ax_struct_Point %t"));
        assert!(rendered.contains("getelementptr %ax_struct_Point, ptr %local"));
        assert!(rendered.contains("ret %ax_struct_Point"));
        assert!(rendered.contains("= call %ax_struct_Point @ax_shift(%ax_struct_Point %t"));
    }

    #[test]
    fn renders_struct_field_assignments() {
        let rendered = render(
            "\
struct Point {
    x: i32,
    y: i32,
}

fn main() -> i32 {
    let mut point: Point = Point { x: 2, y: 5 };
    point.y = point.x + 10;
    return point.y;
}
",
        );

        assert!(rendered.contains("%ax_struct_Point = type { i32, i32 }"));
        assert!(rendered.contains("getelementptr %ax_struct_Point, ptr %local"));
        assert!(rendered.contains("store i32 %t"));
        assert!(rendered.contains("ret i32 %t"));
    }

    #[test]
    fn renders_unit_enum_values_params_returns_and_comparisons() {
        let rendered = render(
            "\
enum Flag {
    Off,
    On,
}

fn choose(flag: Flag) -> Flag {
    return flag;
}

fn score(flag: Flag) -> i32 {
    if (flag == Flag.On) {
        return 9;
    }
    return 2;
}

fn main() -> i32 {
    let flag: Flag = choose(Flag.On);
    return score(flag);
}
",
        );

        assert!(rendered.contains("define i32 @ax_choose(i32 %arg0)"));
        assert!(rendered.contains("ret i32 %t"));
        assert!(rendered.contains("call i32 @ax_choose(i32 1)"));
        assert!(rendered.contains("store i32 %t"));
        assert!(rendered.contains("icmp eq i32 %t"));
        assert!(rendered.contains("call i32 @ax_score(i32 %t"));
    }

    #[test]
    fn renders_unit_enum_match_statement_tests() {
        let rendered = render(
            "\
enum Flag {
    Off,
    On,
}

fn score(flag: Flag) -> i32 {
    match (flag) {
        Flag.On => {
            return 9;
        }
        Flag.Off => {
            return 2;
        }
    }
}

fn main() -> i32 {
    return score(Flag.On);
}
",
        );

        assert!(rendered.contains("define i32 @ax_score(i32 %arg0)"));
        assert!(rendered.contains("icmp eq i32 %t"));
        assert!(rendered.contains("br i1 %t"));
        assert!(rendered.contains("ret i32 9"));
        assert!(rendered.contains("ret i32 2"));
    }

    #[test]
    fn renders_payload_enum_constructors_payload_reads_and_match_tests() {
        let rendered = render(
            "\
enum Maybe {
    None,
    Some(i32),
}

fn score(value: Maybe) -> i32 {
    match (value) {
        Maybe.Some(number) => {
            return number;
        }
        Maybe.None => {
            return 0;
        }
    }
}

fn main() -> i32 {
    let value: Maybe = Maybe.Some(7);
    return score(value);
}
",
        );

        assert!(rendered.contains("%ax_enum_Maybe = type { i32, ptr }"));
        assert!(rendered.contains("define i32 @ax_score(%ax_enum_Maybe %arg0)"));
        assert!(rendered.contains("call ptr @malloc(i64 4)"));
        assert!(rendered.contains("store i32 7, ptr %t"));
        assert!(rendered.contains("insertvalue %ax_enum_Maybe undef, i32 1, 0"));
        assert!(rendered.contains("extractvalue %ax_enum_Maybe %t"));
        assert!(rendered.contains("load i32, ptr %t"));
        assert!(rendered.contains("call i32 @ax_score(%ax_enum_Maybe %t"));
    }

    #[test]
    fn renders_match_expression_with_payload_binding_and_block_arm() {
        let rendered = render(
            "\
enum Maybe {
    None,
    Some(i32),
}

fn score(value: Maybe) -> i32 {
    return match (value) {
        Maybe.Some(number) => {
            let bonus: i32 = 1;
            number + bonus
        },
        Maybe.None => 0,
    };
}

fn main() -> i32 {
    return score(Maybe.Some(7));
}
",
        );

        assert!(rendered.contains("match_arm_"));
        assert!(rendered.contains("match_done_"));
        assert!(rendered.contains("alloca i32"));
        assert!(rendered.contains("extractvalue %ax_enum_Maybe"));
        assert!(rendered.contains("store i32"));
        assert!(rendered.contains("load i32"));
    }

    #[test]
    fn renders_i32_range_match_patterns() {
        let rendered = render(
            "\
fn classify(status: i32) -> i32 {
    return match (status) {
        200..=299 => 2,
        400..=499 => 4,
        _ => 0,
    };
}

fn main() -> i32 {
    return classify(204);
}
",
        );

        assert!(rendered.contains("icmp sge i32"));
        assert!(rendered.contains("icmp sle i32"));
        assert!(rendered.contains("and i1"));
        assert!(rendered.contains("ret i32 %"));
    }

    #[test]
    fn renders_or_match_patterns() {
        let rendered = render(
            "\
enum Mode {
    Check,
    Run,
    Build,
}

fn score(mode: Mode) -> i32 {
    return match (mode) {
        Mode.Check | Mode.Run => 1,
        Mode.Build => 2,
    };
}

fn main() -> i32 {
    return score(Mode.Check);
}
",
        );

        assert!(rendered.contains("icmp eq i32"));
        assert!(rendered.contains("or i1"));
        assert!(rendered.contains("ret i32 %"));
    }

    #[test]
    fn renders_guarded_match_arms() {
        let rendered = render(
            "\
enum Token {
    Number(i32),
    End,
}

fn score(token: Token) -> i32 {
    return match (token) {
        Token.Number(value) if value > 9 => value,
        Token.Number(_) => 1,
        Token.End => 0,
    };
}

fn main() -> i32 {
    return score(Token.Number(12));
}
",
        );

        assert!(rendered.contains("match_guard_"));
        assert!(rendered.contains("icmp sgt i32"));
        assert!(rendered.contains("ret i32 %"));
    }

    #[test]
    fn renders_payload_enum_equality() {
        let rendered = render(
            "\
enum Status {
    Code(i32),
    Label(string),
    Done,
}

fn score() -> i32 {
    let code: Status = Status.Code(7);
    let same_code: Status = Status.Code(7);
    let other_code: Status = Status.Code(8);
    let label: Status = Status.Label(\"ok\");
    let same_label: Status = Status.Label(\"ok\");
    let other_label: Status = Status.Label(\"bad\");
    let mut total: i32 = 0;
    if (code == same_code) {
        total = total + 1;
    }
    if (code != other_code) {
        total = total + 2;
    }
    if (label == same_label) {
        total = total + 4;
    }
    if (label != other_label) {
        total = total + 8;
    }
    if (Status.Done == Status.Done) {
        total = total + 16;
    }
    if (code != Status.Done) {
        total = total + 32;
    }
    return total;
}

fn main() -> i32 {
    return score();
}
",
        );

        assert!(rendered.contains("enum_eq_tags_match"));
        assert!(rendered.contains("call i32 @strcmp(ptr"));
        assert!(rendered.contains("ret i32 %"));
    }

    #[test]
    fn renders_struct_match_patterns() {
        let rendered = render(
            "\
struct Point {
    x: i32,
    y: i32,
}

fn score(point: Point) -> i32 {
    return match (point) {
        Point { x, y } => x + y,
    };
}

fn main() -> i32 {
    return score(Point { x: 20, y: 22 });
}
",
        );

        assert!(rendered.contains("extractvalue %ax_struct_Point"));
        assert!(rendered.contains("store i32"));
        assert!(rendered.contains("ret i32 %"));
    }

    #[test]
    fn renders_concrete_generic_result_and_option_instances() {
        let rendered = render(
            "\
enum Option<T> {
    None,
    Some(T),
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn option_or(value: Option<i32>, fallback: i32) -> i32 {
    return match (value) { Option.Some(found) => found, Option.None => fallback };
}

fn value_or_zero(result: Result<i32, string>) -> i32 {
    return match (result) { Result.Ok(value) => value, Result.Err(_) => 0 };
}

fn main() -> i32 {
    let present: Option<i32> = Option.Some(5);
    let ok: Result<i32, string> = Result.Ok(7);
    return option_or(present, 0) + value_or_zero(ok);
}
",
        );

        assert!(rendered.contains("%ax_enum_Option_i32_ = type { i32, ptr }"));
        assert!(rendered.contains("%ax_enum_Result_i32__string_ = type { i32, ptr }"));
        assert!(rendered.contains("define i32 @ax_option_or(%ax_enum_Option_i32_ %arg0"));
        assert!(
            rendered.contains("define i32 @ax_value_or_zero(%ax_enum_Result_i32__string_ %arg0")
        );
        assert!(rendered.contains("call ptr @malloc(i64 4)"));
        assert!(rendered.contains("extractvalue %ax_enum_Result_i32__string_"));
    }

    #[test]
    fn renders_result_try_early_return_with_error_rewrap() {
        let rendered = render(
            "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn parse(text: string) -> Result<i32, string> {
    if (text == \"ok\") {
        return Result.Ok(7);
    }
    return Result.Err(\"bad\");
}

fn render_score(text: string) -> Result<string, string> {
    let score: i32 = parse(text)?;
    return Result.Ok(\"score=\" + to_string(score));
}

fn main() -> i32 {
    let result: Result<string, string> = render_score(\"ok\");
    return match (result) { Result.Ok(text) => string_len(text), Result.Err(message) => string_len(message) };
}
",
        );

        assert!(rendered.contains("%ax_enum_Result_i32__string_ = type { i32, ptr }"));
        assert!(rendered.contains("%ax_enum_Result_string__string_ = type { i32, ptr }"));
        assert!(rendered.contains("define %ax_enum_Result_string__string_ @ax_render_score"));
        assert!(rendered.contains("call %ax_enum_Result_i32__string_ @ax_parse"));
        assert!(rendered.contains("try_err_"));
        assert!(rendered.contains("try_ok_"));
        assert!(rendered.contains("ret %ax_enum_Result_string__string_"));
    }
}
