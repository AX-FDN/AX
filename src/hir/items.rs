use super::*;

impl<'a> LoweringContext<'a> {
    pub(in crate::hir) fn lower_program(
        &self,
        program: &ast::Program,
    ) -> Result<Program, Diagnostic> {
        let mut items = Vec::new();
        for item in &program.items {
            items.extend(self.lower_items(item)?);
        }
        Ok(Program { items })
    }

    pub(in crate::hir) fn lower_items(&self, item: &ast::Item) -> Result<Vec<Item>, Diagnostic> {
        let kind = match &item.kind {
            ast::ItemKind::Function {
                name,
                type_params,
                type_param_bounds,
                params,
                return_type,
                body,
            } => ItemKind::Function {
                name: self.canonical_name(name, item.span),
                type_params: type_params.clone(),
                type_param_bounds: type_param_bounds
                    .iter()
                    .map(|bound| self.lower_type_param_bound(bound))
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
                params: params
                    .iter()
                    .map(|param| {
                        Ok(Param {
                            name: param.name.clone(),
                            ty: self.lower_type_ref(&param.ty)?,
                            span: param.span,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
                return_type: self.lower_type_ref(return_type)?,
                body: self.lower_block(body)?,
            },
            ast::ItemKind::Const { name, ty, value } => ItemKind::Const {
                name: self.canonical_name(name, item.span),
                ty: self.lower_type_ref(ty)?,
                value: self.lower_expr(value)?,
            },
            ast::ItemKind::TypeAlias { .. } => return Ok(Vec::new()),
            ast::ItemKind::Struct {
                name,
                type_params,
                fields,
            } => ItemKind::Struct {
                name: self.canonical_name(name, item.span),
                type_params: type_params.clone(),
                fields: fields
                    .iter()
                    .map(|field| {
                        Ok(StructField {
                            name: field.name.clone(),
                            ty: self.lower_type_ref(&field.ty)?,
                            span: field.span,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
            },
            ast::ItemKind::Enum {
                name,
                type_params,
                variants,
            } => ItemKind::Enum {
                name: self.canonical_name(name, item.span),
                type_params: type_params.clone(),
                variants: variants
                    .iter()
                    .map(|variant| {
                        Ok(EnumVariant {
                            name: variant.name.clone(),
                            payload: variant
                                .payload
                                .as_ref()
                                .map(|payload| self.lower_type_ref(payload))
                                .transpose()?,
                            span: variant.span,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
            },
            ast::ItemKind::Trait { .. } => return Ok(Vec::new()),
            ast::ItemKind::Impl {
                type_params,
                target,
                methods,
                ..
            } => {
                let method_prefix = self.impl_method_prefix(target, item.span)?;
                return methods
                    .iter()
                    .map(|method| {
                        let all_type_params = type_params
                            .iter()
                            .cloned()
                            .chain(method.type_params.iter().cloned())
                            .collect::<Vec<_>>();
                        Ok(Item {
                            kind: ItemKind::Function {
                                name: format!("{method_prefix}.{}", method.name),
                                type_params: all_type_params,
                                type_param_bounds: Vec::new(),
                                params: method
                                    .params
                                    .iter()
                                    .map(|param| {
                                        Ok(Param {
                                            name: param.name.clone(),
                                            ty: self.lower_type_ref(&param.ty)?,
                                            span: param.span,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, Diagnostic>>()?,
                                return_type: self.lower_type_ref(&method.return_type)?,
                                body: self.lower_block(&method.body)?,
                            },
                            visibility: crate::ast::Visibility::Private,
                            span: method.span,
                        })
                    })
                    .collect();
            }
        };

        Ok(vec![Item {
            kind,
            visibility: item.visibility,
            span: item.span,
        }])
    }

    pub(in crate::hir) fn lower_type_param_bound(
        &self,
        bound: &ast::TypeParamBound,
    ) -> Result<TypeParamBound, Diagnostic> {
        let Some(trait_name) = bound.trait_ref.direct_name() else {
            return Err(self.lowering_error(
                "H0001",
                "cannot lower invalid trait bound into HIR",
                bound.span,
            ));
        };
        let trait_name = self
            .resolve_canonical_name(trait_name, bound.trait_ref.span, &self.trait_names)
            .unwrap_or_else(|| trait_name.to_string());
        Ok(TypeParamBound {
            type_param: bound.type_param.clone(),
            trait_name,
            span: bound.span,
        })
    }
}
