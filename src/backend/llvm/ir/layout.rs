use super::*;

pub(super) fn collect_struct_layouts(
    program: &Program,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Result<BTreeMap<String, StructLayout>, Vec<String>> {
    let mut layouts = BTreeMap::new();
    let mut unsupported = Vec::new();
    let mut definitions = BTreeMap::new();
    let mut layout_sources: BTreeMap<String, Vec<StructField>> = BTreeMap::new();

    for item in &program.items {
        let ItemKind::Struct {
            name,
            type_params,
            fields,
        } = &item.kind
        else {
            continue;
        };

        definitions.insert(name.clone(), (type_params.clone(), fields.clone()));

        if type_params.is_empty() {
            let ty = Type::Struct { name: name.clone() };
            insert_struct_layout(&mut layouts, ty.clone());
            layout_sources.insert(struct_layout_key(&ty), fields.clone());
        }
    }

    let mut struct_instances = BTreeMap::new();
    collect_struct_instance_types(program, &mut struct_instances);
    for struct_ty in struct_instances.values() {
        let Type::StructInstance { name, args } = struct_ty else {
            continue;
        };
        let Some((type_params, fields)) = definitions.get(name) else {
            unsupported.push(format!(
                "struct instance {} has no struct definition in LLVM AOT v0",
                ax_type_name(struct_ty)
            ));
            continue;
        };
        if type_params.len() != args.len() {
            unsupported.push(format!(
                "struct instance {} has {} type argument(s), but struct `{name}` declares {}",
                ax_type_name(struct_ty),
                args.len(),
                type_params.len()
            ));
            continue;
        };
        let substitutions = type_params
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect::<BTreeMap<_, _>>();
        let concrete_fields = fields
            .iter()
            .map(|field| StructField {
                name: field.name.clone(),
                ty: substitute_type_params(&field.ty, &substitutions),
                span: field.span,
            })
            .collect::<Vec<_>>();
        insert_struct_layout(&mut layouts, struct_ty.clone());
        layout_sources.insert(struct_layout_key(struct_ty), concrete_fields);
    }

    for (key, fields) in layout_sources {
        let Some(layout) = layouts.get(&key) else {
            continue;
        };
        let layout_name = layout.name.clone();
        match lower_struct_fields(&layout_name, &fields, &layouts, enum_layouts) {
            Ok(field_layouts) => {
                if let Some(layout) = layouts.get_mut(&key) {
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

fn insert_struct_layout(layouts: &mut BTreeMap<String, StructLayout>, struct_ty: Type) {
    let (Type::Struct { .. } | Type::StructInstance { .. }) = &struct_ty else {
        return;
    };
    let key = struct_layout_key(&struct_ty);
    let name = ax_type_name(&struct_ty);
    let ty = llvm_struct_type_name_for_type(&struct_ty);
    layouts.insert(
        key,
        StructLayout {
            name,
            ax_ty: struct_ty,
            ty,
            fields: Vec::new(),
        },
    );
}

pub(super) fn collect_enum_layouts(
    program: &Program,
) -> Result<BTreeMap<String, EnumLayout>, Vec<String>> {
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

fn collect_struct_instance_types(program: &Program, instances: &mut BTreeMap<String, Type>) {
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
                    collect_struct_instance_type(&param.ty, instances);
                }
                collect_struct_instance_type(return_type, instances);
                for local in locals {
                    collect_struct_instance_type(&local.ty, instances);
                }
                for block in blocks {
                    for statement in &block.statements {
                        collect_statement_struct_instance_types(statement, instances);
                    }
                    collect_terminator_struct_instance_types(&block.terminator.kind, instances);
                }
            }
            ItemKind::Struct { fields, .. } => {
                for field in fields {
                    collect_struct_instance_type(&field.ty, instances);
                }
            }
            ItemKind::Enum { variants, .. } => {
                for variant in variants {
                    if let Some(payload) = &variant.payload {
                        collect_struct_instance_type(payload, instances);
                    }
                }
            }
            ItemKind::Const { ty, .. } => collect_struct_instance_type(ty, instances),
        }
    }
}

fn collect_statement_struct_instance_types(
    statement: &Statement,
    instances: &mut BTreeMap<String, Type>,
) {
    match &statement.kind {
        StatementKind::Let {
            ty, initializer, ..
        } => {
            collect_struct_instance_type(ty, instances);
            collect_expr_struct_instance_types(initializer, instances);
        }
        StatementKind::Assign { target, value } => {
            collect_place_struct_instance_types(target, instances);
            collect_expr_struct_instance_types(value, instances);
        }
        StatementKind::Eval { expr } => collect_expr_struct_instance_types(expr, instances),
    }
}

fn collect_terminator_struct_instance_types(
    terminator: &TerminatorKind,
    instances: &mut BTreeMap<String, Type>,
) {
    match terminator {
        TerminatorKind::Branch { condition, .. } | TerminatorKind::Return { value: condition } => {
            collect_expr_struct_instance_types(condition, instances);
        }
        TerminatorKind::Goto { .. } | TerminatorKind::Unreachable => {}
    }
}

fn collect_place_struct_instance_types(place: &Place, instances: &mut BTreeMap<String, Type>) {
    match &place.kind {
        PlaceKind::Local { .. } => {}
        PlaceKind::Field { base, .. } => collect_place_struct_instance_types(base, instances),
        PlaceKind::Index { base, index } => {
            collect_place_struct_instance_types(base, instances);
            collect_expr_struct_instance_types(index, instances);
        }
    }
}

fn collect_expr_struct_instance_types(expr: &Expr, instances: &mut BTreeMap<String, Type>) {
    match &expr.kind {
        ExprKind::Unary { expr, .. }
        | ExprKind::Try { expr }
        | ExprKind::EnumPayload { value: expr }
        | ExprKind::Field { base: expr, .. } => collect_expr_struct_instance_types(expr, instances),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_struct_instance_types(left, instances);
            collect_expr_struct_instance_types(right, instances);
        }
        ExprKind::Call { arguments, .. }
        | ExprKind::ArrayLiteral {
            elements: arguments,
        } => {
            for argument in arguments {
                collect_expr_struct_instance_types(argument, instances);
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_expr_struct_instance_types(&field.value, instances);
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_statement_struct_instance_types(statement, instances);
            }
            collect_expr_struct_instance_types(value, instances);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_struct_instance_types(scrutinee, instances);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_expr_struct_instance_types(guard, instances);
                }
                collect_match_pattern_struct_instance_types(&arm.pattern, instances);
                collect_expr_struct_instance_types(&arm.value, instances);
            }
        }
        ExprKind::EnumVariant { payload, .. } => {
            if let Some(payload) = payload {
                collect_expr_struct_instance_types(payload, instances);
            }
        }
        ExprKind::MatchTest { scrutinee, pattern } => {
            collect_expr_struct_instance_types(scrutinee, instances);
            collect_match_pattern_struct_instance_types(pattern, instances);
        }
        ExprKind::Index { base, index } => {
            collect_expr_struct_instance_types(base, instances);
            collect_expr_struct_instance_types(index, instances);
        }
        ExprKind::Slice { base, start, end } => {
            collect_expr_struct_instance_types(base, instances);
            collect_expr_struct_instance_types(start, instances);
            collect_expr_struct_instance_types(end, instances);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Local { .. }
        | ExprKind::Const { .. } => {}
    }
}

fn collect_match_pattern_struct_instance_types(
    pattern: &MatchPattern,
    instances: &mut BTreeMap<String, Type>,
) {
    match &pattern.kind {
        MatchPatternKind::EnumVariant { payload_type, .. } => {
            if let Some(payload_type) = payload_type {
                collect_struct_instance_type(payload_type, instances);
            }
        }
        MatchPatternKind::Struct { fields, .. } => {
            for field in fields {
                collect_struct_instance_type(&field.ty, instances);
            }
        }
        MatchPatternKind::Or { alternatives } => {
            for alternative in alternatives {
                collect_match_pattern_struct_instance_types(alternative, instances);
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

fn collect_struct_instance_type(ty: &Type, instances: &mut BTreeMap<String, Type>) {
    match ty {
        Type::StructInstance { args, .. } => {
            if !type_contains_type_param(ty) {
                instances.insert(struct_layout_key(ty), ty.clone());
            }
            for arg in args {
                collect_struct_instance_type(arg, instances);
            }
        }
        Type::EnumInstance { args, .. } => {
            for arg in args {
                collect_struct_instance_type(arg, instances);
            }
        }
        Type::Slice { element } | Type::Array { element, .. } => {
            collect_struct_instance_type(element, instances);
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
        | Type::StringList
        | Type::Struct { .. }
        | Type::Enum { .. } => false,
    }
}

pub(super) fn substitute_type_params(ty: &Type, substitutions: &BTreeMap<String, Type>) -> Type {
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
