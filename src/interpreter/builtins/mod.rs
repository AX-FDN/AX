mod argv;
mod bytes;
mod core;
mod env;
mod fs;
mod http;
mod net;
mod path;
mod process;
mod string;
mod string_list;

use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::Interpreter;
use super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_function(
        &mut self,
        name: &str,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match name {
            "println" => self.call_println_builtin(arguments, span),
            "len" => self.call_len_builtin(arguments, span),
            "to_string" => self.call_to_string_builtin(arguments, span),
            "string_len" => self.call_string_len_builtin(arguments, span),
            "string_contains" => self.call_string_contains_builtin(arguments, span),
            "string_starts_with" => self.call_string_starts_with_builtin(arguments, span),
            "string_ends_with" => self.call_string_ends_with_builtin(arguments, span),
            "string_replace" => self.call_string_replace_builtin(arguments, span),
            "string_trim" => self.call_string_trim_builtin(arguments, span),
            "string_split_lines" => self.call_string_split_lines_builtin(arguments, span),
            "bytes_empty" => self.call_bytes_empty_builtin(arguments, span),
            "bytes_from_string" => self.call_bytes_from_string_builtin(arguments, span),
            "bytes_to_string_lossy" => self.call_bytes_to_string_lossy_builtin(arguments, span),
            "bytes_to_hex" => self.call_bytes_to_hex_builtin(arguments, span),
            "bytes_push" => self.call_bytes_push_builtin(arguments, span),
            "bytes_get" => self.call_bytes_get_builtin(arguments, span),
            "string_list_new" => self.call_string_list_new_builtin(arguments, span),
            "string_list_push" => self.call_string_list_push_builtin(arguments, span),
            "string_list_join" => self.call_string_list_join_builtin(arguments, span),
            "string_list_get" => self.call_string_list_get_builtin(arguments, span),
            "argv_len" => self.call_argv_len_builtin(arguments, span),
            "argv_get" => self.call_argv_get_builtin(arguments, span),
            "env_has" => self.call_env_has_builtin(arguments, span),
            "env_get" => self.call_env_get_builtin(arguments, span),
            "process_cwd" => self.call_process_cwd_builtin(arguments, span),
            "process_run" => self.call_process_run_builtin(arguments, span),
            "process_capture" => self.call_process_capture_builtin(arguments, span),
            "process_run_in" => self.call_process_run_in_builtin(arguments, span),
            "process_capture_in" => self.call_process_capture_in_builtin(arguments, span),
            "path_join" => self.call_path_join_builtin(arguments, span),
            "path_resolve" => self.call_path_resolve_builtin(arguments, span),
            "path_parent" => self.call_path_parent_builtin(arguments, span),
            "path_file_name" => self.call_path_file_name_builtin(arguments, span),
            "path_stem" => self.call_path_stem_builtin(arguments, span),
            "path_extension" => self.call_path_extension_builtin(arguments, span),
            "path_is_absolute" => self.call_path_is_absolute_builtin(arguments, span),
            "fs_is_file" => self.call_fs_is_file_builtin(arguments, span),
            "fs_is_dir" => self.call_fs_is_dir_builtin(arguments, span),
            "fs_exists" => self.call_fs_exists_builtin(arguments, span),
            "fs_file_size" => self.call_fs_file_size_builtin(arguments, span),
            "fs_copy_file" => self.call_fs_copy_file_builtin(arguments, span),
            "fs_rename" => self.call_fs_rename_builtin(arguments, span),
            "fs_create_dir_all" => self.call_fs_create_dir_all_builtin(arguments, span),
            "fs_remove_file" => self.call_fs_remove_file_builtin(arguments, span),
            "fs_remove_dir_all" => self.call_fs_remove_dir_all_builtin(arguments, span),
            "fs_read_dir" => self.call_fs_read_dir_builtin(arguments, span),
            "fs_read_to_string" => self.call_fs_read_to_string_builtin(arguments, span),
            "fs_write_string" => self.call_fs_write_string_builtin(arguments, span),
            "http_get" => self.call_http_get_builtin(arguments, span),
            "net_tcp_exchange" => self.call_net_tcp_exchange_builtin(arguments, span),
            _ => self.call_declared_function(name, arguments, span),
        }
    }
}
