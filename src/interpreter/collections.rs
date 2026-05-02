use super::*;

impl<'a> Interpreter<'a> {
    pub(in crate::interpreter) fn indexable_elements(
        &self,
        value: Value,
        span: Span,
    ) -> Result<Vec<Value>, Diagnostic> {
        match value {
            Value::Array(elements) | Value::Slice(elements) => Ok(elements),
            other => Err(self.runtime_error(
                "R0028",
                format!(
                    "index access requires an array or slice value, got `{}`",
                    other.display()
                ),
                span,
            )),
        }
    }

    pub(in crate::interpreter) fn resolve_array_index(
        &self,
        index_value: Value,
        index_span: Span,
        array_len: usize,
        overall_span: Span,
    ) -> Result<usize, Diagnostic> {
        let Value::I32(index) = index_value else {
            return Err(self
                .runtime_error(
                    "R0029",
                    format!(
                        "array index must evaluate to `i32`, got `{}`",
                        index_value.display()
                    ),
                    index_span,
                )
                .with_note("AX array indices use `i32` values in the current prototype")
                .with_suggestion("compute or convert an `i32` index before indexing the array"));
        };

        if index < 0 {
            return Err(self
                .runtime_error(
                    "R0030",
                    format!("array index cannot be negative, got `{index}`"),
                    index_span,
                )
                .with_note("AX arrays use zero-based indexing")
                .with_suggestion("use an index in the range `0..len-1`"));
        }

        let index = usize::try_from(index).expect("non-negative i32 should fit in usize");
        if index >= array_len {
            return Err(self
                .runtime_error(
                    "R0031",
                    format!("array index `{index}` is out of bounds for length {array_len}"),
                    overall_span,
                )
                .with_note(format!(
                    "this access targets a fixed-size array with length {array_len}"
                ))
                .with_suggestion(
                    "change the index or array length so the access stays within bounds",
                ));
        }

        Ok(index)
    }

    pub(in crate::interpreter) fn resolve_slice_bound(
        &self,
        bound_value: Value,
        bound_span: Span,
        array_len: usize,
        label: &str,
    ) -> Result<usize, Diagnostic> {
        let Value::I32(bound) = bound_value else {
            return Err(self
                .runtime_error(
                    "R0032",
                    format!(
                        "slice {label} bound must evaluate to `i32`, got `{}`",
                        bound_value.display()
                    ),
                    bound_span,
                )
                .with_note("AX slice bounds currently use `i32` values")
                .with_suggestion("compute or convert an `i32` bound before slicing"));
        };

        if bound < 0 {
            return Err(self
                .runtime_error(
                    "R0033",
                    format!("slice {label} bound cannot be negative, got `{bound}`"),
                    bound_span,
                )
                .with_note("AX slice bounds use zero-based positions")
                .with_suggestion("use a bound in the range `0..len`"));
        }

        let bound = usize::try_from(bound).expect("non-negative i32 should fit in usize");
        if bound > array_len {
            return Err(self
                .runtime_error(
                    "R0034",
                    format!(
                        "slice {label} bound `{bound}` is out of bounds for length {array_len}"
                    ),
                    bound_span,
                )
                .with_note(format!(
                    "slice bounds may range from 0 up to the collection length {array_len}"
                ))
                .with_suggestion("change the slice bounds so they stay within `0..len`"));
        }

        Ok(bound)
    }
}
