#[derive(Debug, Clone, Copy)]
pub(super) struct AotBlockerRule {
    pub(super) rule_id: &'static str,
    pub(super) layer: &'static str,
    pub(super) summary: &'static str,
    pub(super) repair_goal: &'static str,
    pub(super) validation: &'static [&'static str],
}

pub(super) fn rule_for_blocker(code: &str, category: &str) -> AotBlockerRule {
    match code {
        "AOT0001" => RULE_NATIVE_EMISSION_PENDING,
        "AOT0101" => RULE_PROJECT_LINKING_PENDING,
        "AOT0102" => RULE_PACKAGE_LINKING_PENDING,
        "AOT0103" => RULE_PACKAGE_LOCK_REQUIRED,
        "AOT0104" => RULE_REGISTRY_HOST_BOUNDARY_PACKAGE,
        "AOT0105" => RULE_REGISTRY_FUTURE_NATIVE_PACKAGE,
        "AOT0201" => RULE_GENERIC_LOWERING_PENDING,
        "AOT0202" => RULE_TRAIT_LOWERING_PENDING,
        "AOT0203" => RULE_METHOD_ABI_PENDING,
        "AOT0204" => RULE_ENUM_MATCH_LOWERING_PENDING,
        "AOT0206" => RULE_ARRAY_WRITE_LOWERING_PENDING,
        "AOT0207" => RULE_STRUCT_WRITE_LOWERING_PENDING,
        "AOT0301" => RULE_HOST_RUNTIME_ABI_PENDING,
        "AOT0302" => RULE_STRING_RUNTIME_ABI_PENDING,
        "AOT0303" => RULE_BYTES_RUNTIME_ABI_PENDING,
        "AOT1000" => RULE_LINKING_DISABLED,
        "AOT1001" => RULE_CLANG_MISSING,
        "AOT1002" => RULE_CLANG_LINK_FAILED,
        "AOT2001" => RULE_LLVM_LOWERING_UNSUPPORTED,
        _ => fallback_rule_for_category(category),
    }
}

fn fallback_rule_for_category(category: &str) -> AotBlockerRule {
    match category {
        "toolchain" => RULE_TOOLCHAIN_BLOCKER,
        "llvm_lowering" => RULE_LLVM_LOWERING_UNSUPPORTED,
        "runtime" => RULE_RUNTIME_ABI_BLOCKER,
        "project" | "package" => RULE_PACKAGING_BLOCKER,
        _ => RULE_AOT_UNSUPPORTED,
    }
}

const VALIDATE_RUN_BUILD: &[&str] = &["axc run <target>", "axc build <target> --json"];
const VALIDATE_LOCK_BUILD: &[&str] = &["axc lock <project> --check", "axc build <project> --json"];
const VALIDATE_LINKING: &[&str] = &[
    "$env:AX_LLVM_CLANG = \"<path-to-clang>\"",
    "axc build <target> --emit exe --json",
];

const RULE_NATIVE_EMISSION_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_native_emission_pending",
    layer: "aot_readiness",
    summary: "native executable emission is still behind the AOT readiness gate",
    repair_goal: "Do not rewrite valid AX source; keep axc run as the semantic reference until native emission is enabled.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_PROJECT_LINKING_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_project_linking_pending",
    layer: "aot_readiness",
    summary: "project source graph packaging is not yet native-linkable",
    repair_goal: "Preserve the project source layout and add explicit native project linking semantics before treating build as a source repair failure.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_PACKAGE_LINKING_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_package_linking_pending",
    layer: "aot_readiness",
    summary: "local path package linking is not yet native-linkable",
    repair_goal: "Keep package code unchanged unless package metadata is invalid; implement native package linking before expecting executable output.",
    validation: VALIDATE_LOCK_BUILD,
};

const RULE_PACKAGE_LOCK_REQUIRED: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_package_lock_must_be_current",
    layer: "aot_readiness",
    summary: "AOT package input needs a current lockfile",
    repair_goal: "Refresh or verify AX.lock before treating the package graph as reproducible AOT input.",
    validation: VALIDATE_LOCK_BUILD,
};

const RULE_REGISTRY_HOST_BOUNDARY_PACKAGE: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_registry_package_host_boundary_preview",
    layer: "aot_readiness",
    summary: "host-boundary registry packages need native runtime ABI coverage",
    repair_goal: "Treat check/run as valid when they pass; do not rewrite package users just to satisfy AOT until the host runtime ABI is available.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_REGISTRY_FUTURE_NATIVE_PACKAGE: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_registry_package_future_native_preview",
    layer: "aot_readiness",
    summary: "future-native registry packages are interpreter-first previews",
    repair_goal: "Keep package usage explicit and explain the native ABI gap; do not claim executable parity until the package graduates from future-native preview.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_GENERIC_LOWERING_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_generic_lowering_pending",
    layer: "aot_readiness",
    summary: "generic functions, structs, impls, and aliases are outside the current AOT subset",
    repair_goal: "Do not erase generics from user code; add monomorphization for function, struct, impl, and alias lowering before native validation.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_TRAIT_LOWERING_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_trait_lowering_pending",
    layer: "aot_readiness",
    summary: "trait lowering and dispatch are outside the current AOT subset",
    repair_goal: "Keep trait-bearing source valid for check/run and add explicit native trait lowering before build parity.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_METHOD_ABI_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_method_abi_pending",
    layer: "aot_readiness",
    summary: "impl method lowering and method ABI are not frozen for AOT",
    repair_goal: "Do not rewrite methods away; define the native method ABI before expecting executable output.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_ENUM_MATCH_LOWERING_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_enum_match_lowering_pending",
    layer: "aot_readiness",
    summary: "advanced pattern lowering needs a native backend contract",
    repair_goal: "Keep semantic behavior under axc run and add struct destructuring plus complex pattern lowering before AOT validation.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_ARRAY_WRITE_LOWERING_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_array_write_lowering_pending",
    layer: "aot_readiness",
    summary: "slice mutation still needs native write semantics",
    repair_goal: "Keep slice-mutating source valid for axc run and add explicit native slice layout write lowering before AOT parity.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_STRUCT_WRITE_LOWERING_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_struct_write_lowering_pending",
    layer: "aot_readiness",
    summary: "struct field mutation needs native write semantics",
    repair_goal: "Keep struct-mutating source valid for axc run and add explicit native field write lowering before AOT parity.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_HOST_RUNTIME_ABI_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_host_runtime_abi_pending",
    layer: "runtime_abi",
    summary: "host boundary builtins need a native runtime ABI",
    repair_goal: "Treat the source as valid when check/run pass; add runtime ABI support before native build can preserve host behavior.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_STRING_RUNTIME_ABI_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_string_runtime_abi_pending",
    layer: "runtime_abi",
    summary: "string values need a native representation and ABI",
    repair_goal: "Keep string-using source valid for the interpreter and add a native string ABI before AOT parity.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_BYTES_RUNTIME_ABI_PENDING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_bytes_runtime_abi_pending",
    layer: "runtime_abi",
    summary: "bytes values need a native byte-buffer ABI",
    repair_goal: "Keep byte-buffer source valid for the interpreter and add a native bytes ABI before AOT parity.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_LINKING_DISABLED: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_linking_must_be_enabled",
    layer: "toolchain_link",
    summary: "LLVM IR was generated but executable linking is disabled",
    repair_goal: "Run `axc build <target> --emit exe` when executable validation is intended; do not edit user source.",
    validation: VALIDATE_LINKING,
};

const RULE_CLANG_MISSING: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_clang_toolchain_required",
    layer: "toolchain_link",
    summary: "clang is required to link the generated LLVM IR",
    repair_goal: "Install clang or point AX_LLVM_CLANG at a working clang executable before rerunning build.",
    validation: VALIDATE_LINKING,
};

const RULE_CLANG_LINK_FAILED: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_clang_link_failed",
    layer: "toolchain_link",
    summary: "clang failed while linking the executable artifact",
    repair_goal: "Inspect the linker stderr and toolchain target before editing AX source; this is a toolchain/link failure by default.",
    validation: VALIDATE_LINKING,
};

const RULE_LLVM_LOWERING_UNSUPPORTED: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_llvm_lowering_unsupported",
    layer: "llvm_lowering",
    summary: "MIR reached LLVM AOT but uses features outside the current lowering subset",
    repair_goal: "Keep axc run as the semantic reference and add explicit MIR-to-LLVM lowering support before native validation.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_TOOLCHAIN_BLOCKER: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_toolchain_blocker",
    layer: "toolchain_link",
    summary: "AOT executable emission is blocked by the native toolchain",
    repair_goal: "Inspect compiler/linker configuration before editing AX source; toolchain blockers are not source repairs by default.",
    validation: VALIDATE_LINKING,
};

const RULE_RUNTIME_ABI_BLOCKER: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_runtime_abi_blocker",
    layer: "aot_readiness",
    summary: "AOT needs a native runtime ABI for this feature",
    repair_goal: "Keep interpreter behavior as the reference and add runtime ABI support before expecting executable parity.",
    validation: VALIDATE_RUN_BUILD,
};

const RULE_PACKAGING_BLOCKER: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_packaging_blocker",
    layer: "aot_readiness",
    summary: "AOT package or project packaging needs a native linking contract",
    repair_goal: "Validate package metadata separately and add native package/project linking before executable parity.",
    validation: VALIDATE_LOCK_BUILD,
};

const RULE_AOT_UNSUPPORTED: AotBlockerRule = AotBlockerRule {
    rule_id: "aot_feature_unsupported",
    layer: "aot_readiness",
    summary: "the current AOT backend does not support this feature yet",
    repair_goal: "Do not rewrite valid source just to satisfy build; explain the backend gap and keep check/run as the semantic reference.",
    validation: VALIDATE_RUN_BUILD,
};
