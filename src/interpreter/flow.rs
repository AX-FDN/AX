use super::value::Value;

pub(super) enum ControlFlow {
    Continue,
    Break,
    LoopContinue,
    Return(Value),
}

pub(super) enum EvalFlow {
    Value(Value),
    Return(Value),
}

pub(super) enum ConditionFlow {
    Value(bool),
    Return(Value),
}
