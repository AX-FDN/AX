use super::*;

impl<'a> Interpreter<'a> {
    pub(in crate::interpreter) fn assign_target(
        &mut self,
        frame: &mut Frame,
        target: &Place,
        next_value: Value,
    ) -> Result<(), Diagnostic> {
        let root_name = place_root_name(target);
        let root_slot = lookup_slot(frame, root_name).ok_or_else(|| {
            self.runtime_error(
                "R0006",
                format!("assignment to unknown variable `{root_name}`"),
                target.span,
            )
        })?;

        if !root_slot.mutable {
            return match &target.kind {
                PlaceKind::Local { .. } => Err(self.runtime_error(
                    "R0007",
                    format!("cannot assign to immutable variable `{root_name}`"),
                    target.span,
                )),
                PlaceKind::Field { field, .. } => Err(self.runtime_error(
                    "R0025",
                    format!("cannot assign to field `{field}` on immutable variable `{root_name}`"),
                    target.span,
                )),
                PlaceKind::Index { .. } => Err(self.runtime_error(
                    "R0007",
                    format!("cannot assign through immutable array variable `{root_name}`"),
                    target.span,
                )),
            };
        }

        let target_value = self.resolve_place_value_mut(frame, target)?;
        *target_value = next_value;
        Ok(())
    }

    pub(in crate::interpreter) fn resolve_place_value_mut<'f>(
        &mut self,
        frame: &'f mut Frame,
        place: &Place,
    ) -> Result<&'f mut Value, Diagnostic> {
        match &place.kind {
            PlaceKind::Local { name } => {
                let slot = lookup_slot_mut(frame, name).ok_or_else(|| {
                    self.runtime_error(
                        "R0006",
                        format!("assignment to unknown variable `{name}`"),
                        place.span,
                    )
                })?;
                Ok(&mut slot.value)
            }
            PlaceKind::Field { base, field } => {
                let base_value = self.resolve_place_value_mut(frame, base)?;
                match base_value {
                    Value::Struct { fields, .. } => fields.get_mut(field).ok_or_else(|| {
                        self.runtime_error(
                            "R0026",
                            format!("struct value does not contain field `{field}`"),
                            place.span,
                        )
                    }),
                    other => Err(self.runtime_error(
                        "R0027",
                        format!(
                            "field assignment requires a struct value, got `{}`",
                            other.display()
                        ),
                        place.span,
                    )),
                }
            }
            PlaceKind::Index { base, index } => {
                let index_value = match self.eval_expr(index, frame)? {
                    EvalFlow::Value(value) => value,
                    EvalFlow::Return(_) => {
                        return Err(self.runtime_error(
                            "R0135",
                            "`?` cannot propagate while resolving an assignment target",
                            index.span,
                        ));
                    }
                };
                let base_value = self.resolve_place_value_mut(frame, base)?;
                match base_value {
                    Value::Array(elements) => {
                        let resolved_index = self.resolve_array_index(
                            index_value,
                            index.span,
                            elements.len(),
                            place.span,
                        )?;
                        Ok(&mut elements[resolved_index])
                    }
                    Value::Slice(_) => Err(self
                        .runtime_error(
                            "R0036",
                            format!(
                                "cannot assign through slice variable `{}` because slices are read-only",
                                place_root_name(base)
                            ),
                            place.span,
                        )
                        .with_suggestion(
                            "assign through the original mutable array instead of a slice view",
                        )),
                    other => Err(self.runtime_error(
                        "R0028",
                        format!(
                            "array element assignment requires an array value, got `{}`",
                            other.display()
                        ),
                        place.span,
                    )),
                }
            }
        }
    }
}
