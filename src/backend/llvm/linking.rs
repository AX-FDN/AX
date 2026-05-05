use std::path::{Path, PathBuf};

use super::{LlvmAotLinkMode, abi, toolchain};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeRuntimeStrategy {
    ProcessLifetimeRuntimeV0,
}

impl NativeRuntimeStrategy {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            NativeRuntimeStrategy::ProcessLifetimeRuntimeV0 => abi::native_memory_policy(),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct NativeLinkPlan {
    target: String,
    ir_artifact: PathBuf,
    executable: PathBuf,
    link_mode: LlvmAotLinkMode,
    runtime_strategy: NativeRuntimeStrategy,
}

impl NativeLinkPlan {
    pub(super) fn single_ir_executable(
        target: impl Into<String>,
        ir_artifact: impl AsRef<Path>,
        executable: impl AsRef<Path>,
        link_mode: LlvmAotLinkMode,
    ) -> Self {
        Self {
            target: target.into(),
            ir_artifact: ir_artifact.as_ref().to_path_buf(),
            executable: executable.as_ref().to_path_buf(),
            link_mode,
            runtime_strategy: NativeRuntimeStrategy::ProcessLifetimeRuntimeV0,
        }
    }

    pub(super) fn target(&self) -> &str {
        &self.target
    }

    pub(super) fn ir_artifact(&self) -> &Path {
        &self.ir_artifact
    }

    pub(super) fn executable(&self) -> &Path {
        &self.executable
    }

    pub(super) fn link_mode(&self) -> LlvmAotLinkMode {
        self.link_mode
    }

    pub(super) fn runtime_strategy(&self) -> NativeRuntimeStrategy {
        self.runtime_strategy
    }
}

pub(super) fn execute(plan: &NativeLinkPlan) -> toolchain::LinkOutcome {
    let _ = (plan.target(), plan.runtime_strategy().as_str());
    match plan.link_mode() {
        LlvmAotLinkMode::Environment => {
            toolchain::link_if_enabled(plan.ir_artifact(), plan.executable())
        }
        LlvmAotLinkMode::Force => toolchain::link_executable(plan.ir_artifact(), plan.executable()),
        LlvmAotLinkMode::Skip => toolchain::LinkOutcome::Skipped,
    }
}
