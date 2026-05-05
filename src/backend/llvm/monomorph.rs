use std::collections::{BTreeMap, BTreeSet};

use crate::mir::Program;

use super::ir::specialization::{
    FunctionSpecialization, ReachableFunctions, collect_reachable_functions,
};

#[derive(Clone, Default)]
pub(super) struct MonomorphizationPlan {
    reachable_functions: BTreeSet<String>,
    reachable_specializations: BTreeSet<String>,
    specializations: BTreeMap<String, FunctionSpecialization>,
}

impl MonomorphizationPlan {
    pub(super) fn empty() -> Self {
        Self::default()
    }

    pub(super) fn reachable_functions(&self) -> &BTreeSet<String> {
        &self.reachable_functions
    }

    pub(super) fn used_concrete_instances(&self) -> impl Iterator<Item = &FunctionSpecialization> {
        self.specializations
            .values()
            .filter(|specialization| self.reachable_specializations.contains(&specialization.key))
    }
}

pub(super) fn plan_program(program: &Program) -> Result<MonomorphizationPlan, Vec<String>> {
    let (reachable, specializations) = collect_reachable_functions(program)?;
    Ok(MonomorphizationPlan::from_parts(reachable, specializations))
}

impl MonomorphizationPlan {
    fn from_parts(
        reachable: ReachableFunctions,
        specializations: BTreeMap<String, FunctionSpecialization>,
    ) -> Self {
        Self {
            reachable_functions: reachable.functions,
            reachable_specializations: reachable.specializations,
            specializations,
        }
    }
}
