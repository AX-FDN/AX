use super::*;

impl<'a> LoweringContext<'a> {
    pub(in crate::hir) fn lower_type_ref(&self, ty: &ast::TypeRef) -> Result<Type, Diagnostic> {
        match (&ty.name, &ty.type_args[..], &ty.element, ty.length) {
            (Some(name), [], None, None) => match name.as_str() {
                "bool" => Ok(Type::Bool),
                "i32" => Ok(Type::I32),
                "f32" => Ok(Type::F32),
                "string" => Ok(Type::String),
                "bytes" => Ok(Type::Bytes),
                "string_list" => Ok(Type::StringList),
                _ => {
                    if let Some(name) =
                        self.resolve_canonical_name(name, ty.span, &self.struct_names)
                    {
                        return Ok(Type::Struct { name });
                    }
                    if let Some(name) = self.resolve_canonical_name(name, ty.span, &self.enum_names)
                    {
                        return Ok(Type::Enum { name });
                    }
                    if let Some(alias_name) =
                        self.resolve_canonical_name(name, ty.span, &self.type_alias_names())
                        && let Some((type_params, target)) = self.type_aliases.get(&alias_name)
                    {
                        if type_params.is_empty() {
                            return self.lower_type_ref(target);
                        }
                    }
                    if looks_like_type_param(name) {
                        return Ok(Type::TypeParam { name: name.clone() });
                    }
                    Err(self.lowering_error(
                        "H0001",
                        format!("cannot lower unknown type `{}` into HIR", name),
                        ty.span,
                    ))
                }
            },
            (Some(name), args, None, None) => {
                if let Some(alias_name) =
                    self.resolve_canonical_name(name, ty.span, &self.type_alias_names())
                    && let Some((type_params, target)) = self.type_aliases.get(&alias_name)
                {
                    let lowered_target = self.lower_type_ref(target)?;
                    let substitutions = type_params
                        .iter()
                        .cloned()
                        .zip(
                            args.iter()
                                .map(|arg| self.lower_type_ref(arg))
                                .collect::<Result<Vec<_>, _>>()?,
                        )
                        .collect::<HashMap<_, _>>();
                    return Ok(substitute_type_params(&lowered_target, &substitutions));
                }

                if let Some(name) = self.resolve_canonical_name(name, ty.span, &self.struct_names) {
                    return Ok(Type::StructInstance {
                        name,
                        args: args
                            .iter()
                            .map(|arg| self.lower_type_ref(arg))
                            .collect::<Result<Vec<_>, _>>()?,
                    });
                }
                if let Some(name) = self.resolve_canonical_name(name, ty.span, &self.enum_names) {
                    return Ok(Type::EnumInstance {
                        name,
                        args: args
                            .iter()
                            .map(|arg| self.lower_type_ref(arg))
                            .collect::<Result<Vec<_>, _>>()?,
                    });
                }
                Err(self.lowering_error(
                    "H0001",
                    format!("cannot lower unknown generic type `{}` into HIR", name),
                    ty.span,
                ))
            }
            (None, [], Some(element), None) => Ok(Type::Slice {
                element: Box::new(self.lower_type_ref(element)?),
            }),
            (None, [], Some(element), Some(length)) => Ok(Type::Array {
                element: Box::new(self.lower_type_ref(element)?),
                length,
            }),
            _ => Err(self.lowering_error(
                "H0001",
                "cannot lower invalid type syntax into HIR",
                ty.span,
            )),
        }
    }

    pub(in crate::hir) fn type_alias_names(&self) -> HashSet<String> {
        self.type_aliases.keys().cloned().collect()
    }
}
