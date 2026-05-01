use crate::diagnostics::DiagnosticKind;

use super::RuleTemplate;

pub(super) fn match_code(code: &str) -> Option<RuleTemplate> {
    match code {
        "R0012" | "R0018" | "R0019" | "R0020" | "R0022" | "R0024" => {
            Some(RULE_INTEGER_ARITHMETIC_IN_RANGE)
        }
        "R0021" => Some(RULE_DIVISION_BY_ZERO),
        "R0030" => Some(RULE_ARRAY_INDEX_NON_NEGATIVE),
        "R0031" => Some(RULE_ARRAY_INDEX_IN_BOUNDS),
        _ => None,
    }
}

pub(super) fn match_kind(kind: DiagnosticKind) -> Option<RuleTemplate> {
    match kind {
        DiagnosticKind::ArgvIndexNegative => Some(RULE_ARGV_INDEX_NON_NEGATIVE),
        DiagnosticKind::ArgvIndexOutOfBounds => Some(RULE_ARGV_INDEX_IN_BOUNDS),
        DiagnosticKind::StringListIndexNegative => Some(RULE_STRING_LIST_INDEX_NON_NEGATIVE),
        DiagnosticKind::StringListIndexOutOfBounds => Some(RULE_STRING_LIST_INDEX_IN_BOUNDS),
        DiagnosticKind::EnvironmentVariableUnavailable => {
            Some(RULE_ENVIRONMENT_VARIABLE_MUST_BE_AVAILABLE)
        }
        DiagnosticKind::ReadableFilePathRequired => Some(RULE_READABLE_FILE_PATH_REQUIRED),
        DiagnosticKind::ReadableDirectoryPathRequired => {
            Some(RULE_READABLE_DIRECTORY_PATH_REQUIRED)
        }
        DiagnosticKind::ProcessCommandNotLaunchable => {
            Some(RULE_PROCESS_COMMAND_MUST_BE_LAUNCHABLE)
        }
        DiagnosticKind::ProcessCaptureNonZeroExit => {
            Some(RULE_PROCESS_CAPTURE_REQUIRES_SUCCESSFUL_EXIT)
        }
        _ => None,
    }
}

const RULE_ARGV_INDEX_NON_NEGATIVE: RuleTemplate = RuleTemplate {
    rule_id: "argv_index_must_be_non_negative",
    normalized_pattern: "argv_index_must_be_non_negative",
    repair_goal: "Call `argv_get(index)` only with a zero-based index that stays at `0` or above.",
    summary: "AX command-line arguments use zero-based `i32` indexing, so negative values are always invalid at runtime.",
    pattern: "if (argv_len() > 0) { let first: string = argv_get(0); }",
    minimal_example: "let output: string = argv_get(1);",
    anti_pattern: Some("let flag: string = argv_get(-1);"),
    default_fixit: "change the index so it is zero or greater before calling `argv_get(...)`",
};

const RULE_ARGV_INDEX_IN_BOUNDS: RuleTemplate = RuleTemplate {
    rule_id: "argv_index_must_stay_in_bounds",
    normalized_pattern: "argv_index_must_stay_in_bounds",
    repair_goal: "Check `argv_len()` first and only read positions that exist in the current runtime invocation.",
    summary: "`argv_get(index)` fails at runtime when the selected index is outside the argument list provided by the host.",
    pattern: "if (argv_len() >= 2) { let output: string = argv_get(1); }",
    minimal_example: "let target: string = argv_get(0);",
    anti_pattern: Some("let missing: string = argv_get(3);"),
    default_fixit: "guard with `argv_len()` or reduce the requested argument index",
};

const RULE_STRING_LIST_INDEX_NON_NEGATIVE: RuleTemplate = RuleTemplate {
    rule_id: "string_list_index_must_be_non_negative",
    normalized_pattern: "string_list_index_must_be_non_negative",
    repair_goal: "Call `string_list_get(list, index)` only with a zero-based index that stays at `0` or above.",
    summary: "AX string lists use zero-based `i32` indexing, so negative values are always invalid at runtime.",
    pattern: "if (index >= 0) { let value: string = string_list_get(items, index); }",
    minimal_example: "let value: string = string_list_get(items, 0);",
    anti_pattern: Some("let value: string = string_list_get(items, -1);"),
    default_fixit: "guard the read with `index >= 0`, or use a known non-negative index",
};

const RULE_STRING_LIST_INDEX_IN_BOUNDS: RuleTemplate = RuleTemplate {
    rule_id: "string_list_index_must_stay_in_bounds",
    normalized_pattern: "string_list_index_must_stay_in_bounds",
    repair_goal: "Check `len(list)` first and only read positions that exist in the current string list.",
    summary: "`string_list_get(list, index)` fails at runtime when the selected index is outside the current list length.",
    pattern: "if (index < len(items)) { let value: string = string_list_get(items, index); }",
    minimal_example: "let value: string = string_list_get(items, 0);",
    anti_pattern: Some("let value: string = string_list_get(items, 99);"),
    default_fixit: "guard the read with `index < len(items)`, or use `std.collections.string_list_index_of` before reading",
};

const RULE_ENVIRONMENT_VARIABLE_MUST_BE_AVAILABLE: RuleTemplate = RuleTemplate {
    rule_id: "environment_variable_must_be_available",
    normalized_pattern: "environment_variable_must_be_available",
    repair_goal: "Only call `env_get(name)` when that variable is present in the host environment, or guard first with `env_has(name)`.",
    summary: "AX exposes host environment variables directly, so missing keys still fail at runtime even though the program type-checks.",
    pattern: "if (env_has(\"PATH\")) { let path: string = env_get(\"PATH\"); }",
    minimal_example: "let home: string = env_get(\"HOME\");",
    anti_pattern: Some("let token: string = env_get(\"MISSING_KEY\");"),
    default_fixit: "guard with `env_has(name)` or ensure the host sets the variable before running the program",
};

const RULE_READABLE_FILE_PATH_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "readable_file_path_required",
    normalized_pattern: "readable_file_path_required",
    repair_goal: "Pass an existing readable file path before reading file contents or file metadata.",
    summary: "Host file-reading builtins such as `fs_read_to_string` and `fs_file_size` fail at runtime when the target path is missing, unreadable, or not a regular readable file.",
    pattern: "if (fs_is_file(path)) { let text: string = fs_read_to_string(path); }",
    minimal_example: "let size: i32 = fs_file_size(path);",
    anti_pattern: Some("let text: string = fs_read_to_string(\"missing.txt\");"),
    default_fixit: "guard with `fs_is_file(path)` or pass an existing readable file path",
};

const RULE_READABLE_DIRECTORY_PATH_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "readable_directory_path_required",
    normalized_pattern: "readable_directory_path_required",
    repair_goal: "Pass an existing readable directory path before calling `fs_read_dir(path)`.",
    summary: "`fs_read_dir` only succeeds on readable directory paths; missing paths, files, and unreadable directories fail at runtime.",
    pattern: "if (fs_is_dir(path)) { let children: [string] = fs_read_dir(path); }",
    minimal_example: "let children: [string] = fs_read_dir(root_dir);",
    anti_pattern: Some("let children: [string] = fs_read_dir(\"missing-dir\");"),
    default_fixit: "guard with `fs_is_dir(path)` or pass an existing readable directory path",
};

const RULE_PROCESS_COMMAND_MUST_BE_LAUNCHABLE: RuleTemplate = RuleTemplate {
    rule_id: "process_command_must_be_launchable",
    normalized_pattern: "process_command_must_be_launchable",
    repair_goal: "Use a shell command the host can start, and when using `*_in`, pass an existing working directory.",
    summary: "AX process builtins delegate to the host shell, so runtime launch fails if the command cannot start or the selected working directory does not exist.",
    pattern: "let status: i32 = process_run(command);",
    minimal_example: "let output: string = process_capture_in(work_dir, command);",
    anti_pattern: Some("let status: i32 = process_run_in(\"missing-dir\", \"echo ready\");"),
    default_fixit: "fix the command text or working directory so the host shell can start the process",
};

const RULE_PROCESS_CAPTURE_REQUIRES_SUCCESSFUL_EXIT: RuleTemplate = RuleTemplate {
    rule_id: "process_capture_requires_successful_exit",
    normalized_pattern: "process_capture_requires_successful_exit",
    repair_goal: "Use `process_capture` only when the command is expected to exit with status 0, or switch to `process_run` when non-zero exit codes are part of the workflow.",
    summary: "`process_capture` treats non-zero exit status as a runtime error instead of returning stdout, so failing commands must be rewritten or run with the status-oriented builtin.",
    pattern: "let output: string = process_capture(command);",
    minimal_example: "let output: string = process_capture(\"echo ready\");",
    anti_pattern: Some("let output: string = process_capture(\"exit 7\");"),
    default_fixit: "fix the command so it exits 0 or switch to `process_run(...)` / `process_run_in(...)`",
};

const RULE_ARRAY_INDEX_IN_BOUNDS: RuleTemplate = RuleTemplate {
    rule_id: "array_index_must_stay_in_bounds",
    normalized_pattern: "array_index_must_stay_in_bounds",
    repair_goal: "Keep the index within `0..len-1` for the current fixed-size array.",
    summary: "AX array indexing is bounds-checked at runtime, so the accessed index must stay within the declared array length.",
    pattern: "let values: [i32; 2] = [1, 2]; return values[1];",
    minimal_example: "let values: [i32; 3] = [1, 2, 3]; println(values[2]);",
    anti_pattern: Some("let values: [i32; 2] = [1, 2]; return values[2];"),
    default_fixit: "change the index or array length so the access stays within bounds",
};

const RULE_ARRAY_INDEX_NON_NEGATIVE: RuleTemplate = RuleTemplate {
    rule_id: "array_index_must_be_non_negative",
    normalized_pattern: "array_index_must_be_non_negative",
    repair_goal: "Use an index expression that never evaluates to a negative `i32` value.",
    summary: "AX array indexing accepts `i32`, but runtime indexing still requires the resolved value to be zero or greater.",
    pattern: "let values: [i32; 2] = [1, 2]; return values[0];",
    minimal_example: "let values: [i32; 3] = [1, 2, 3]; println(values[index]);",
    anti_pattern: Some("let values: [i32; 2] = [1, 2]; return values[-1];"),
    default_fixit: "change the index expression so it stays at 0 or above",
};

const RULE_DIVISION_BY_ZERO: RuleTemplate = RuleTemplate {
    rule_id: "division_by_zero_must_be_avoided",
    normalized_pattern: "division_by_zero_must_be_avoided",
    repair_goal: "Prove that the divisor is never zero before dividing.",
    summary: "AX rejects division by zero at runtime for both `i32` and `f32` division.",
    pattern: "if (divisor == 0) { return 0; } return value / divisor;",
    minimal_example: "let safe: i32 = total / count;",
    anti_pattern: Some("return value / 0;"),
    default_fixit: "guard the divisor or rewrite the calculation so the right-hand side cannot be zero",
};

const RULE_INTEGER_ARITHMETIC_IN_RANGE: RuleTemplate = RuleTemplate {
    rule_id: "integer_arithmetic_must_stay_in_range",
    normalized_pattern: "integer_arithmetic_must_stay_in_range",
    repair_goal: "Rewrite the arithmetic so every intermediate `i32` result stays within the valid range.",
    summary: "AX checks `i32` arithmetic at runtime, so negation, addition, subtraction, multiplication, and division must stay within range.",
    pattern: "let value: i32 = left + right;",
    minimal_example: "let value: i32 = count - 1;",
    anti_pattern: Some("let value: i32 = 2147483647 + 1;"),
    default_fixit: "use smaller operands or rewrite the arithmetic so the `i32` result stays in range",
};
