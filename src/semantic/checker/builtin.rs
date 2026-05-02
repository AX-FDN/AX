use crate::ast::Expr;
use crate::diagnostics::{Diagnostic, DiagnosticKind};

use super::{Type, TypeChecker};

#[path = "builtin/args_env.rs"]
mod args_env;
#[path = "builtin/conversion.rs"]
mod conversion;
#[path = "builtin/fs.rs"]
mod fs;
#[path = "builtin/helpers.rs"]
mod helpers;
#[path = "builtin/output.rs"]
mod output;
#[path = "builtin/path.rs"]
mod path;
#[path = "builtin/process.rs"]
mod process;
#[path = "builtin/sequences.rs"]
mod sequences;
#[path = "builtin/string_list.rs"]
mod string_list;
#[path = "builtin/strings.rs"]
mod strings;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_builtin_call(
        &mut self,
        expr: &Expr,
        name: &str,
        arguments: &[Expr],
    ) -> Option<Type> {
        let ty = match name {
            "println" => self.check_println_builtin(arguments),
            "string_len" => self.check_string_len_builtin(expr, arguments),
            "string_contains" => self.check_string_contains_builtin(expr, arguments),
            "string_starts_with" => self.check_string_starts_with_builtin(expr, arguments),
            "string_ends_with" => self.check_string_ends_with_builtin(expr, arguments),
            "string_replace" => self.check_string_replace_builtin(expr, arguments),
            "string_trim" => self.check_string_trim_builtin(expr, arguments),
            "string_split_lines" => self.check_string_split_lines_builtin(expr, arguments),
            "string_list_new" => self.check_string_list_new_builtin(expr, arguments),
            "string_list_push" => self.check_string_list_push_builtin(expr, arguments),
            "string_list_join" => self.check_string_list_join_builtin(expr, arguments),
            "string_list_get" => self.check_string_list_get_builtin(expr, arguments),
            "len" => self.check_len_builtin(expr, arguments),
            "argv_len" => self.check_argv_len_builtin(expr, arguments),
            "argv_get" => self.check_argv_get_builtin(expr, arguments),
            "env_has" => self.check_env_has_builtin(expr, arguments),
            "env_get" => self.check_env_get_builtin(expr, arguments),
            "process_cwd" => self.check_process_cwd_builtin(expr, arguments),
            "process_run" => self.check_process_run_builtin(expr, arguments),
            "process_capture" => self.check_process_capture_builtin(expr, arguments),
            "process_run_in" => self.check_process_run_in_builtin(expr, arguments),
            "process_capture_in" => self.check_process_capture_in_builtin(expr, arguments),
            "path_join" => self.check_path_join_builtin(expr, arguments),
            "path_parent" => self.check_path_parent_builtin(expr, arguments),
            "path_resolve" => self.check_path_resolve_builtin(expr, arguments),
            "path_file_name" => self.check_path_file_name_builtin(expr, arguments),
            "path_stem" => self.check_path_stem_builtin(expr, arguments),
            "path_extension" => self.check_path_extension_builtin(expr, arguments),
            "path_is_absolute" => self.check_path_is_absolute_builtin(expr, arguments),
            "fs_is_file" => self.check_fs_is_file_builtin(expr, arguments),
            "fs_is_dir" => self.check_fs_is_dir_builtin(expr, arguments),
            "fs_exists" => self.check_fs_exists_builtin(expr, arguments),
            "fs_file_size" => self.check_fs_file_size_builtin(expr, arguments),
            "fs_copy_file" => self.check_fs_copy_file_builtin(expr, arguments),
            "fs_rename" => self.check_fs_rename_builtin(expr, arguments),
            "fs_create_dir_all" => self.check_fs_create_dir_all_builtin(expr, arguments),
            "fs_remove_file" => self.check_fs_remove_file_builtin(expr, arguments),
            "fs_remove_dir_all" => self.check_fs_remove_dir_all_builtin(expr, arguments),
            "fs_read_dir" => self.check_fs_read_dir_builtin(expr, arguments),
            "fs_read_to_string" => self.check_fs_read_to_string_builtin(expr, arguments),
            "fs_write_string" => self.check_fs_write_string_builtin(expr, arguments),
            "to_string" => self.check_to_string_builtin(expr, arguments),
            _ => return None,
        };

        Some(ty)
    }
}
