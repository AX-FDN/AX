use super::*;

impl<'a> LoweringContext<'a> {
    pub(in crate::hir) fn new(source: &'a SourceFile, program: &ast::Program) -> Self {
        let unit_modules = program
            .source_units
            .iter()
            .filter_map(|unit| {
                unit.module
                    .as_ref()
                    .map(|module| (unit.path.clone(), module.path.clone()))
            })
            .collect::<HashMap<_, _>>();
        let mut function_names = HashSet::new();
        let mut struct_names = HashSet::new();
        let mut enum_names = HashSet::new();
        let mut trait_names = HashSet::new();
        let mut type_aliases = HashMap::new();

        for item in &program.items {
            let canonical_name = canonical_item_name(source, &unit_modules, item);
            match &item.kind {
                ast::ItemKind::Function { .. } => {
                    function_names.insert(canonical_name);
                }
                ast::ItemKind::Const { .. } => {}
                ast::ItemKind::TypeAlias {
                    type_params,
                    target,
                    ..
                } => {
                    type_aliases.insert(canonical_name, (type_params.clone(), target.clone()));
                }
                ast::ItemKind::Struct { .. } => {
                    struct_names.insert(canonical_name);
                }
                ast::ItemKind::Enum { .. } => {
                    enum_names.insert(canonical_name);
                }
                ast::ItemKind::Trait { .. } => {
                    trait_names.insert(canonical_name);
                }
                ast::ItemKind::Impl {
                    target, methods, ..
                } => {
                    if let Some(target_name) = target.direct_name() {
                        let method_prefix = if target_name.contains('.') {
                            target_name.to_string()
                        } else {
                            canonical_item_name(source, &unit_modules, item)
                        };
                        for method in methods {
                            function_names.insert(format!("{method_prefix}.{}", method.name));
                        }
                    }
                }
            }
        }

        let mut context = Self {
            source,
            unit_modules,
            function_names,
            struct_names,
            enum_names,
            trait_names,
            type_aliases,
            struct_fields: HashMap::new(),
            enum_variant_payloads: HashMap::new(),
            next_match_temp: Cell::new(0),
            next_for_in_temp: Cell::new(0),
        };
        context.struct_fields = context.collect_struct_fields(program);
        context.enum_variant_payloads = context.collect_enum_variant_payloads(program);
        context
    }

    pub(in crate::hir) fn collect_struct_fields(
        &self,
        program: &ast::Program,
    ) -> HashMap<String, (Vec<String>, HashMap<String, Type>)> {
        let mut structs = HashMap::new();

        for item in &program.items {
            let ast::ItemKind::Struct {
                type_params,
                fields,
                ..
            } = &item.kind
            else {
                continue;
            };

            let struct_name = canonical_item_name(self.source, &self.unit_modules, item);
            let mut lowered_fields = HashMap::new();
            for field in fields {
                if let Ok(field_type) = self.lower_type_ref(&field.ty) {
                    lowered_fields.insert(field.name.clone(), field_type);
                }
            }
            structs.insert(struct_name, (type_params.clone(), lowered_fields));
        }

        structs
    }

    pub(in crate::hir) fn collect_enum_variant_payloads(
        &self,
        program: &ast::Program,
    ) -> HashMap<String, HashMap<String, Type>> {
        let mut payloads = HashMap::new();

        for item in &program.items {
            let ast::ItemKind::Enum { variants, .. } = &item.kind else {
                continue;
            };

            let enum_name = canonical_item_name(self.source, &self.unit_modules, item);
            let mut variant_payloads = HashMap::new();
            for variant in variants {
                if let Some(payload) = &variant.payload
                    && let Ok(payload_type) = self.lower_type_ref(payload)
                {
                    variant_payloads.insert(variant.name.clone(), payload_type);
                }
            }
            payloads.insert(enum_name, variant_payloads);
        }

        payloads
    }
}
