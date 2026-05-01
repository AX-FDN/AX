use crate::diagnostics::DiagnosticKind;

use super::RuleTemplate;

pub(super) fn match_code(code: &str) -> Option<RuleTemplate> {
    match code {
        "S0037" => Some(RULE_ENTRY_FILE_MUST_NOT_DECLARE_MODULE),
        "S0039" => Some(RULE_MODULE_PATH_MUST_MATCH_SOURCE_PATH),
        "S0040" => Some(RULE_MODULE_PATH_MUST_BE_UNIQUE),
        "S0041" => Some(RULE_MODULE_IMPORT_MUST_BE_UNIQUE),
        "S0042" => Some(RULE_IMPORTED_MODULE_MUST_EXIST),
        "S0043" => Some(RULE_CROSS_MODULE_REFERENCE_REQUIRES_IMPORT),
        _ => None,
    }
}

pub(super) fn match_kind(kind: DiagnosticKind) -> Option<RuleTemplate> {
    match kind {
        DiagnosticKind::EntryFileDeclaresModule => Some(RULE_ENTRY_FILE_MUST_NOT_DECLARE_MODULE),
        DiagnosticKind::SupportSourceMissingModuleDeclaration => {
            Some(RULE_SUPPORT_SOURCE_MUST_DECLARE_MODULE)
        }
        DiagnosticKind::SupportSourceMissingManifestListing => {
            Some(RULE_SUPPORT_SOURCE_MUST_BE_LISTED_IN_MANIFEST)
        }
        DiagnosticKind::ModulePathMismatch => Some(RULE_MODULE_PATH_MUST_MATCH_SOURCE_PATH),
        DiagnosticKind::DuplicateModulePath => Some(RULE_MODULE_PATH_MUST_BE_UNIQUE),
        DiagnosticKind::DuplicateModuleImport => Some(RULE_MODULE_IMPORT_MUST_BE_UNIQUE),
        DiagnosticKind::ImportedModuleMissing => Some(RULE_IMPORTED_MODULE_MUST_EXIST),
        DiagnosticKind::CrossModuleReferenceMissingImport => {
            Some(RULE_CROSS_MODULE_REFERENCE_REQUIRES_IMPORT)
        }
        _ => None,
    }
}

const RULE_ENTRY_FILE_MUST_NOT_DECLARE_MODULE: RuleTemplate = RuleTemplate {
    rule_id: "entry_file_must_not_declare_module",
    normalized_pattern: "entry_file_must_not_declare_module",
    repair_goal: "Keep the manifest entry file as the root unit and remove its `module` declaration.",
    summary: "In AX minimal module mode, only support sources declare `module`; the entry file stays manifest-owned and provides `fn main() -> i32`.",
    pattern: "src/main.ax: import lib.report;\nfn main() -> i32 { return lib.report.helper(); }",
    minimal_example: "lib/report.ax: module lib.report;\nfn helper() -> i32 { return 1; }",
    anti_pattern: Some("src/main.ax: module app.main;"),
    default_fixit: "remove the `module ...;` line from the entry file",
};

const RULE_SUPPORT_SOURCE_MUST_DECLARE_MODULE: RuleTemplate = RuleTemplate {
    rule_id: "support_source_must_declare_module",
    normalized_pattern: "support_source_must_declare_module",
    repair_goal: "Add a top-of-file `module ...;` declaration to each support source in module mode.",
    summary: "Every support source discovered from `[package].sources` must declare its module path before top-level items.",
    pattern: "lib/report.ax: module lib.report;\nfn helper() -> i32 { return 1; }",
    minimal_example: "src/main.ax: import lib.report;\nfn main() -> i32 { return lib.report.helper(); }",
    anti_pattern: Some("lib/report.ax: fn helper() -> i32 { return 1; }"),
    default_fixit: "add a top-of-file `module ...;` declaration",
};

const RULE_SUPPORT_SOURCE_MUST_BE_LISTED_IN_MANIFEST: RuleTemplate = RuleTemplate {
    rule_id: "support_source_must_be_listed_in_manifest",
    normalized_pattern: "support_source_must_be_listed_in_manifest",
    repair_goal: "List each support source file or directory under `[package].sources` so module discovery can see it.",
    summary: "AX only loads support modules from paths declared in `[package].sources` inside `AX.toml`.",
    pattern: "sources = [\"foundation\", \"lib\"]",
    minimal_example: "sources = [\"lib/report.ax\"]",
    anti_pattern: Some("sources = []"),
    default_fixit: "add this file or its parent directory to `[package].sources`",
};

const RULE_MODULE_PATH_MUST_MATCH_SOURCE_PATH: RuleTemplate = RuleTemplate {
    rule_id: "module_path_must_match_source_path",
    normalized_pattern: "module_path_must_match_source_path",
    repair_goal: "Change the `module` declaration so it matches the support-source root alias and relative file path.",
    summary: "AX derives the minimal module path from `[package].sources` and the support file path, so the declaration must match that derived path exactly.",
    pattern: "lib/report.ax: module lib.report;",
    minimal_example: "lib/search/index.ax: module lib.search.index;",
    anti_pattern: Some("lib/report.ax: module report;"),
    default_fixit: "change the declaration to the expected module path",
};

const RULE_MODULE_PATH_MUST_BE_UNIQUE: RuleTemplate = RuleTemplate {
    rule_id: "module_path_must_be_unique",
    normalized_pattern: "module_path_must_be_unique",
    repair_goal: "Rename, move, or merge support files so each module path is owned by exactly one file.",
    summary: "Minimal module mode requires a one-to-one mapping between support files and module paths.",
    pattern: "lib/report.ax: module lib.report;\nlib/summary.ax: module lib.summary;",
    minimal_example: "foundation/text.ax: module foundation.text;",
    anti_pattern: Some("lib/report.ax: module lib.report;\nlib/report_copy.ax: module lib.report;"),
    default_fixit: "rename or move one support file so the module paths are unique",
};

const RULE_MODULE_IMPORT_MUST_BE_UNIQUE: RuleTemplate = RuleTemplate {
    rule_id: "module_import_must_be_unique",
    normalized_pattern: "module_import_must_be_unique",
    repair_goal: "Keep only one `import` line for each module path in the current file.",
    summary: "Repeated `import` lines for the same module do not add behavior and should be collapsed to a single import.",
    pattern: "import lib.report;\nfn main() -> i32 { return lib.report.helper(); }",
    minimal_example: "import foundation.text;\nimport lib.report;",
    anti_pattern: Some("import lib.report;\nimport lib.report;"),
    default_fixit: "remove the duplicate import",
};

const RULE_IMPORTED_MODULE_MUST_EXIST: RuleTemplate = RuleTemplate {
    rule_id: "imported_module_must_exist",
    normalized_pattern: "imported_module_must_exist",
    repair_goal: "Import a support module that actually exists in the current project.",
    summary: "An `import` path must match a support source module declared somewhere under the current project's `[package].sources` roots.",
    pattern: "import lib.report;",
    minimal_example: "lib/report.ax: module lib.report;",
    anti_pattern: Some("import lib.missing;"),
    default_fixit: "import an existing support module declared in this project",
};

const RULE_CROSS_MODULE_REFERENCE_REQUIRES_IMPORT: RuleTemplate = RuleTemplate {
    rule_id: "cross_module_reference_requires_import",
    normalized_pattern: "cross_module_reference_requires_import",
    repair_goal: "Add the missing `import module.path;` line before using a cross-module qualified name.",
    summary: "AX minimal module mode requires explicit imports for cross-module references, even when the code already uses a fully qualified name.",
    pattern: "import lib.report;\nfn main() -> i32 { return lib.report.helper(); }",
    minimal_example: "import lib.flag;\nlet value: lib.flag.Flag = lib.flag.Flag.On;",
    anti_pattern: Some("fn main() -> i32 { return lib.report.helper(); }"),
    default_fixit: "add the required `import module.path;` line near the top of this file",
};
