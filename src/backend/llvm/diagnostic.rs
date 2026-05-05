#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AotDiagnosticLayer {
    AotReadiness,
    Monomorphization,
    RuntimeAbi,
    LlvmLowering,
    Toolchain,
    Internal,
}

impl AotDiagnosticLayer {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            AotDiagnosticLayer::AotReadiness => "aot_readiness",
            AotDiagnosticLayer::Monomorphization => "monomorphization",
            AotDiagnosticLayer::RuntimeAbi => "runtime_abi",
            AotDiagnosticLayer::LlvmLowering => "llvm_lowering",
            AotDiagnosticLayer::Toolchain => "toolchain",
            AotDiagnosticLayer::Internal => "internal_compiler_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AotLoweringDiagnostic {
    pub(super) layer: AotDiagnosticLayer,
    pub(super) code: &'static str,
    pub(super) feature: Option<String>,
    pub(super) message: String,
}

impl AotLoweringDiagnostic {
    pub(super) fn new(
        layer: AotDiagnosticLayer,
        code: &'static str,
        feature: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            layer,
            code,
            feature: feature.into(),
            message: message.into(),
        }
    }

    pub(super) fn aot_readiness(feature: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            AotDiagnosticLayer::AotReadiness,
            "AOT2000",
            Some(feature.into()),
            message,
        )
    }

    pub(super) fn monomorphization(message: impl Into<String>) -> Self {
        Self::new(
            AotDiagnosticLayer::Monomorphization,
            "AOT2100",
            None,
            message,
        )
    }

    pub(super) fn runtime_abi(feature: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            AotDiagnosticLayer::RuntimeAbi,
            "AOT2200",
            Some(feature.into()),
            message,
        )
    }

    pub(super) fn llvm_lowering(message: impl Into<String>) -> Self {
        Self::new(AotDiagnosticLayer::LlvmLowering, "AOT2001", None, message)
    }

    pub(super) fn internal(message: impl Into<String>) -> Self {
        Self::new(AotDiagnosticLayer::Internal, "AOT9000", None, message)
    }

    pub(super) fn user_message(&self) -> String {
        self.message.clone()
    }
}

pub(super) fn user_messages(diagnostics: Vec<AotLoweringDiagnostic>) -> Vec<String> {
    diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.user_message())
        .collect()
}
