use super::*;

pub(super) fn collect_string_literals(
    program: &Program,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> BTreeMap<String, StringLiteral> {
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
    collect_composite_formatter_string_literals(layouts, &mut strings, &mut next_id);
    collect_enum_formatter_string_literals(enum_layouts, &mut strings, &mut next_id);
    strings
}

pub(super) fn collect_composite_formatter_string_literals(
    layouts: &BTreeMap<String, StructLayout>,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    insert_string_literal("[", strings, next_id);
    insert_string_literal("]", strings, next_id);
    insert_string_literal(", ", strings, next_id);
    insert_string_literal(" }", strings, next_id);
    for layout in layouts.values() {
        insert_string_literal(&struct_formatter_prefix(&layout.name), strings, next_id);
        for field in &layout.fields {
            insert_string_literal(&struct_field_formatter_label(&field.name), strings, next_id);
        }
    }
}

pub(super) fn collect_enum_formatter_string_literals(
    enum_layouts: &BTreeMap<String, EnumLayout>,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    if enum_layouts.is_empty() {
        return;
    }
    insert_string_literal("(", strings, next_id);
    insert_string_literal(")", strings, next_id);
    for layout in enum_layouts.values() {
        for variant in &layout.variants {
            insert_string_literal(
                &enum_formatter_label(&layout.name, &variant.name),
                strings,
                next_id,
            );
        }
    }
}

pub(super) fn insert_string_literal(
    value: &str,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    if strings.contains_key(value) {
        return;
    }
    let symbol = format!("@.ax_str_{}", *next_id);
    *next_id += 1;
    strings.insert(
        value.to_string(),
        StringLiteral {
            symbol,
            len: value.len() + 1,
            encoded: encode_llvm_c_string(value),
        },
    );
}

pub(super) fn collect_statement_string_literals(
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

pub(super) fn collect_terminator_string_literals(
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

pub(super) fn collect_place_string_literals(
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

pub(super) fn collect_expr_string_literals(
    expr: &Expr,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    match &expr.kind {
        ExprKind::String { value } => {
            insert_string_literal(value, strings, next_id);
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

pub(super) fn collect_match_pattern_string_literals(
    pattern: &MatchPattern,
    strings: &mut BTreeMap<String, StringLiteral>,
    next_id: &mut usize,
) {
    match &pattern.kind {
        MatchPatternKind::String { value } => {
            insert_string_literal(value, strings, next_id);
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

pub(super) fn find_local_use_by_name_in_match_arm(arm: &MatchExprArm, name: &str) -> Option<u32> {
    arm.guard
        .as_ref()
        .and_then(|guard| find_local_use_by_name(guard, name))
        .or_else(|| find_local_use_by_name(&arm.value, name))
}

pub(super) fn find_local_use_by_name(expr: &Expr, name: &str) -> Option<u32> {
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

pub(super) fn find_local_use_by_name_in_statement(
    statement: &Statement,
    name: &str,
) -> Option<u32> {
    match &statement.kind {
        StatementKind::Let { initializer, .. } | StatementKind::Eval { expr: initializer } => {
            find_local_use_by_name(initializer, name)
        }
        StatementKind::Assign { target, value } => find_local_use_by_name_in_place(target, name)
            .or_else(|| find_local_use_by_name(value, name)),
    }
}

pub(super) fn find_local_use_by_name_in_place(place: &Place, name: &str) -> Option<u32> {
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

pub(super) fn encode_llvm_c_string(value: &str) -> String {
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

pub(super) fn ensure_same_type(expected: &str, actual: &str) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "internal LLVM AOT type mismatch: expected {expected}, got {actual}"
        ))
    }
}

pub(super) fn ensure_string_argument(
    function: &str,
    name: &str,
    value: &LlvmValue,
) -> Result<(), String> {
    if value.ty == abi::STRING_LLVM_TYPE && matches!(value.ax_ty.as_ref(), Some(Type::String)) {
        Ok(())
    } else {
        Err(format!(
            "`{function}` argument `{name}` must be `string` in LLVM AOT v0"
        ))
    }
}

pub(super) fn ensure_string_list_argument(
    function: &str,
    name: &str,
    value: &LlvmValue,
) -> Result<(), String> {
    if value.ty == abi::STRING_LIST_LLVM_TYPE
        && matches!(value.ax_ty.as_ref(), Some(Type::StringList))
    {
        Ok(())
    } else {
        Err(format!(
            "`{function}` argument `{name}` must be `string_list` in LLVM AOT v0"
        ))
    }
}

pub(super) fn is_enum_value(value: &LlvmValue) -> bool {
    matches!(
        value.ax_ty.as_ref(),
        Some(Type::Enum { .. } | Type::EnumInstance { .. })
    )
}

pub(super) fn payload_equality_supported(
    ty: &Type,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> bool {
    match ty {
        Type::Bool | Type::I32 | Type::F32 | Type::String => true,
        Type::Array { element, .. } | Type::Slice { element } => {
            payload_equality_supported(element, layouts, enum_layouts)
        }
        Type::Struct { name } => layouts.get(name).is_some_and(|layout| {
            layout
                .fields
                .iter()
                .all(|field| payload_equality_supported(&field.ax_ty, layouts, enum_layouts))
        }),
        Type::StructInstance { .. } => layouts.get(&struct_layout_key(ty)).is_some_and(|layout| {
            layout
                .fields
                .iter()
                .all(|field| payload_equality_supported(&field.ax_ty, layouts, enum_layouts))
        }),
        Type::Enum { .. } | Type::EnumInstance { .. } => enum_layouts
            .get(&enum_layout_key(ty))
            .is_some_and(|layout| {
                layout.variants.iter().all(|variant| {
                    variant.payload_ax_ty.as_ref().map_or(true, |payload| {
                        payload_equality_supported(payload, layouts, enum_layouts)
                    })
                })
            }),
        Type::StringList | Type::TypeParam { .. } => false,
    }
}

pub(super) fn match_pattern_contains_binding(pattern: &MatchPattern) -> bool {
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

pub(super) fn llvm_binary_op_name(op: BinaryOp) -> &'static str {
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

pub(super) fn llvm_type(
    ty: &Type,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Option<String> {
    match ty {
        Type::Bool | Type::I32 | Type::F32 | Type::String | Type::StringList => {
            abi::primitive_llvm_type(ty).map(str::to_string)
        }
        Type::Array { element, length } => {
            let element_ty = llvm_type(element, layouts, enum_layouts)?;
            Some(format!("[{length} x {element_ty}]"))
        }
        Type::Slice { element } => {
            llvm_type(element, layouts, enum_layouts)?;
            Some(slice_llvm_type())
        }
        Type::Struct { name } => layouts.get(name).map(|layout| layout.ty.clone()),
        Type::StructInstance { .. } => layouts
            .get(&struct_layout_key(ty))
            .map(|layout| layout.ty.clone()),
        Type::Enum { .. } | Type::EnumInstance { .. } => enum_layouts
            .get(&enum_layout_key(ty))
            .map(|layout| layout.ty.clone()),
        Type::TypeParam { .. } => None,
    }
}

pub(super) fn slice_llvm_type() -> String {
    abi::slice_llvm_type().to_string()
}

pub(super) fn enum_layout_for_static_type<'a>(
    ty: &Type,
    enum_layouts: &'a BTreeMap<String, EnumLayout>,
) -> Option<&'a EnumLayout> {
    match ty {
        Type::Enum { .. } | Type::EnumInstance { .. } => enum_layouts.get(&enum_layout_key(ty)),
        _ => None,
    }
}

pub(super) fn array_type_parts(
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

pub(super) fn llvm_alloc_size(
    ty: &Type,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Option<usize> {
    llvm_alloc_layout(ty, layouts, enum_layouts).map(|(size, _align)| size)
}

pub(super) fn llvm_alloc_layout(
    ty: &Type,
    layouts: &BTreeMap<String, StructLayout>,
    enum_layouts: &BTreeMap<String, EnumLayout>,
) -> Option<(usize, usize)> {
    match ty {
        Type::Bool => Some((1, 1)),
        Type::I32 => Some((4, 4)),
        Type::F32 => Some((4, 4)),
        Type::String | Type::StringList => Some((8, 8)),
        Type::Slice { element } => {
            llvm_alloc_layout(element, layouts, enum_layouts)?;
            Some((16, 8))
        }
        Type::Enum { .. } | Type::EnumInstance { .. } => enum_layouts
            .get(&enum_layout_key(ty))
            .map(|layout| if layout.ty == "i32" { (4, 4) } else { (16, 8) }),
        Type::Array { element, length } => {
            let (element_size, element_align) = llvm_alloc_layout(element, layouts, enum_layouts)?;
            let stride = align_to(element_size, element_align);
            Some((stride * length, element_align))
        }
        Type::Struct { name } => {
            let layout = layouts.get(name)?;
            let mut size = 0;
            let mut max_align = 1;
            for field in &layout.fields {
                let (field_size, field_align) =
                    llvm_alloc_layout(&field.ax_ty, layouts, enum_layouts)?;
                size = align_to(size, field_align);
                size += field_size;
                max_align = max_align.max(field_align);
            }
            Some((align_to(size, max_align), max_align))
        }
        Type::StructInstance { .. } => {
            let layout = layouts.get(&struct_layout_key(ty))?;
            let mut size = 0;
            let mut max_align = 1;
            for field in &layout.fields {
                let (field_size, field_align) =
                    llvm_alloc_layout(&field.ax_ty, layouts, enum_layouts)?;
                size = align_to(size, field_align);
                size += field_size;
                max_align = max_align.max(field_align);
            }
            Some((align_to(size, max_align), max_align))
        }
        Type::TypeParam { .. } => None,
    }
}

pub(super) fn align_to(size: usize, align: usize) -> usize {
    if align == 0 {
        return size;
    }
    size.div_ceil(align) * align
}

pub(super) fn llvm_struct_type_name(name: &str) -> String {
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

pub(super) fn llvm_struct_type_name_for_type(ty: &Type) -> String {
    llvm_struct_type_name(&ax_type_name(ty))
}

pub(super) fn llvm_enum_type_name(name: &str) -> String {
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

pub(super) fn llvm_enum_type_name_for_type(ty: &Type) -> String {
    llvm_enum_type_name(&ax_type_name(ty))
}

pub(super) fn enum_layout_key(ty: &Type) -> String {
    ax_type_name(ty)
}

pub(super) fn struct_layout_key(ty: &Type) -> String {
    ax_type_name(ty)
}

pub(super) fn enum_base_name(ty: &Type) -> &str {
    match ty {
        Type::Enum { name } | Type::EnumInstance { name, .. } => name,
        _ => "<non-enum>",
    }
}

pub(super) fn enum_formatter_label(enum_name: &str, variant: &str) -> String {
    format!("{enum_name}.{variant}")
}

pub(super) fn struct_formatter_prefix(struct_name: &str) -> String {
    format!("{struct_name} {{ ")
}

pub(super) fn struct_field_formatter_label(field_name: &str) -> String {
    format!("{field_name}: ")
}

pub(super) fn llvm_symbol(name: &str) -> String {
    symbols::user_function(name)
}

pub(super) fn ax_type_name(ty: &Type) -> String {
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

pub(super) fn llvm_float_literal(value: f32) -> String {
    format!("0x{:016X}", (value as f64).to_bits())
}
