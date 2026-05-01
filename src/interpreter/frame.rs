use std::collections::HashMap;

use super::value::Value;

pub(super) struct Frame {
    pub(super) scopes: Vec<HashMap<String, Slot>>,
}

#[derive(Clone)]
pub(super) struct Slot {
    pub(super) mutable: bool,
    pub(super) value: Value,
}

pub(super) fn lookup_slot<'a>(frame: &'a Frame, name: &str) -> Option<&'a Slot> {
    frame.scopes.iter().rev().find_map(|scope| scope.get(name))
}

pub(super) fn lookup_slot_mut<'a>(frame: &'a mut Frame, name: &str) -> Option<&'a mut Slot> {
    frame
        .scopes
        .iter_mut()
        .rev()
        .find_map(|scope| scope.get_mut(name))
}
