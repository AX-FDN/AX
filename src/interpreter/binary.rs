use super::*;

impl<'a> Interpreter<'a> {
    pub(in crate::interpreter) fn eval_binary(
        &self,
        op: BinaryOp,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match (op, left, right) {
            (BinaryOp::LogicalAnd, Value::Bool(left), Value::Bool(right)) => {
                Ok(Value::Bool(left && right))
            }
            (BinaryOp::LogicalOr, Value::Bool(left), Value::Bool(right)) => {
                Ok(Value::Bool(left || right))
            }
            (BinaryOp::Add, Value::I32(left), Value::I32(right)) => left
                .checked_add(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0018", "integer addition overflowed", span)),
            (BinaryOp::Add, Value::String(left), Value::String(right)) => {
                Ok(Value::String(format!("{left}{right}")))
            }
            (BinaryOp::Subtract, Value::I32(left), Value::I32(right)) => left
                .checked_sub(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0019", "integer subtraction overflowed", span)),
            (BinaryOp::Multiply, Value::I32(left), Value::I32(right)) => {
                left.checked_mul(right).map(Value::I32).ok_or_else(|| {
                    self.runtime_error("R0020", "integer multiplication overflowed", span)
                })
            }
            (BinaryOp::Divide, Value::I32(_), Value::I32(0)) => Err(self
                .runtime_error("R0021", "division by zero", span)
                .with_note("AX checks integer division by zero at runtime")
                .with_suggestion(
                    "guard the divisor or rewrite the calculation so the right-hand side cannot be zero",
                )),
            (BinaryOp::Divide, Value::I32(left), Value::I32(right)) => left
                .checked_div(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0022", "integer division overflowed", span)),
            (BinaryOp::Remainder, Value::I32(_), Value::I32(0)) => Err(self
                .runtime_error("R0021", "modulo by zero", span)
                .with_note("AX checks integer remainder by zero at runtime")
                .with_suggestion(
                    "guard the divisor or rewrite the calculation so the right-hand side cannot be zero",
                )),
            (BinaryOp::Remainder, Value::I32(left), Value::I32(right)) => left
                .checked_rem(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0024", "integer remainder overflowed", span)),
            (BinaryOp::Add, Value::F32(left), Value::F32(right)) => Ok(Value::F32(left + right)),
            (BinaryOp::Subtract, Value::F32(left), Value::F32(right)) => {
                Ok(Value::F32(left - right))
            }
            (BinaryOp::Multiply, Value::F32(left), Value::F32(right)) => {
                Ok(Value::F32(left * right))
            }
            (BinaryOp::Divide, Value::F32(_), Value::F32(0.0)) => Err(self
                .runtime_error("R0021", "division by zero", span)
                .with_note("AX checks floating-point division by zero at runtime")
                .with_suggestion(
                    "guard the divisor or rewrite the calculation so the right-hand side cannot be zero",
                )),
            (BinaryOp::Divide, Value::F32(left), Value::F32(right)) => Ok(Value::F32(left / right)),
            (BinaryOp::Equal, left, right) => Ok(Value::Bool(left == right)),
            (BinaryOp::NotEqual, left, right) => Ok(Value::Bool(left != right)),
            (BinaryOp::Less, Value::I32(left), Value::I32(right)) => Ok(Value::Bool(left < right)),
            (BinaryOp::LessEqual, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left <= right))
            }
            (BinaryOp::Greater, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left > right))
            }
            (BinaryOp::GreaterEqual, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left >= right))
            }
            (BinaryOp::Less, Value::F32(left), Value::F32(right)) => Ok(Value::Bool(left < right)),
            (BinaryOp::LessEqual, Value::F32(left), Value::F32(right)) => {
                Ok(Value::Bool(left <= right))
            }
            (BinaryOp::Greater, Value::F32(left), Value::F32(right)) => {
                Ok(Value::Bool(left > right))
            }
            (BinaryOp::GreaterEqual, Value::F32(left), Value::F32(right)) => {
                Ok(Value::Bool(left >= right))
            }
            (_, left, right) => Err(self.runtime_error(
                "R0023",
                format!(
                    "invalid binary operation for runtime values `{}` and `{}`",
                    left.display(),
                    right.display()
                ),
                span,
            )),
        }
    }
}
