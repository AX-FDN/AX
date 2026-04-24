use crate::ast::Expr;
use crate::diagnostics::Diagnostic;

use super::{Type, TypeChecker};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_builtin_call(
        &mut self,
        expr: &Expr,
        name: &str,
        arguments: &[Expr],
    ) -> Option<Type> {
        match name {
            "println" => {
                for argument in arguments {
                    self.check_expr(argument);
                }
                Some(Type::Void)
            }
            "string_len" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `string_len` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `string_len` expects argument `text` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::I32)
            }
            "string_contains" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `string_contains` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `string_contains` expects argument `text` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `string_contains` expects argument `needle` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::Bool)
            }
            "string_starts_with" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `string_starts_with` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `string_starts_with` expects argument `text` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `string_starts_with` expects argument `prefix` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::Bool)
            }
            "string_ends_with" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `string_ends_with` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `string_ends_with` expects argument `text` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `string_ends_with` expects argument `suffix` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::Bool)
            }
            "string_replace" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 3 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `string_replace` expects 3 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `string_replace` expects argument `text` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `string_replace` expects argument `from` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[2],
                    expr.span,
                    format!(
                        "function `string_replace` expects argument `to` to be `string`, found `{}`",
                        argument_types[2].describe()
                    ),
                );
                Some(Type::String)
            }
            "len" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `len` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                match &argument_types[0] {
                    Type::String | Type::Array { .. } | Type::Slice { .. } => Some(Type::I32),
                    actual => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0022",
                                format!(
                                    "function `len` expects argument `value` to be `string`, array, or slice, found `{}`",
                                    actual.describe()
                                ),
                                self.info.source,
                                expr.span,
                            )
                            .with_note(
                                "`len` is the general traversal-length builtin for strings, fixed-size arrays, and slices",
                            )
                            .with_suggestion(
                                "call `len` with a string, array, or slice value like `len(values)`",
                            ),
                        );
                        Some(Type::Error)
                    }
                }
            }
            "argv_len" => {
                for argument in arguments {
                    self.check_expr(argument);
                }

                if !arguments.is_empty() {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `argv_len` expects 0 argument(s), found {}",
                            arguments.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                Some(Type::I32)
            }
            "argv_get" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `argv_get` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::I32,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `argv_get` expects argument `index` to be `i32`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "env_has" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `env_has` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `env_has` expects argument `name` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::Bool)
            }
            "env_get" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `env_get` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `env_get` expects argument `name` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "process_cwd" => {
                for argument in arguments {
                    self.check_expr(argument);
                }

                if !arguments.is_empty() {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `process_cwd` expects 0 argument(s), found {}",
                            arguments.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                Some(Type::String)
            }
            "process_run" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `process_run` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `process_run` expects argument `command` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::I32)
            }
            "process_capture" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `process_capture` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `process_capture` expects argument `command` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "process_run_in" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `process_run_in` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `process_run_in` expects argument `working_dir` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `process_run_in` expects argument `command` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::I32)
            }
            "process_capture_in" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `process_capture_in` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `process_capture_in` expects argument `working_dir` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `process_capture_in` expects argument `command` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::String)
            }
            "path_join" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `path_join` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `path_join` expects argument `left` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `path_join` expects argument `right` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::String)
            }
            "path_parent" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `path_parent` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `path_parent` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "path_resolve" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `path_resolve` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `path_resolve` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "path_file_name" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `path_file_name` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `path_file_name` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "path_stem" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `path_stem` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `path_stem` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "path_extension" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `path_extension` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `path_extension` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "path_is_absolute" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `path_is_absolute` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `path_is_absolute` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::Bool)
            }
            "fs_is_file" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_is_file` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_is_file` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::Bool)
            }
            "fs_is_dir" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_is_dir` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_is_dir` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::Bool)
            }
            "fs_exists" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_exists` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_exists` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::Bool)
            }
            "fs_file_size" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_file_size` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_file_size` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::I32)
            }
            "fs_copy_file" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_copy_file` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_copy_file` expects argument `source_path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `fs_copy_file` expects argument `destination_path` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::I32)
            }
            "fs_rename" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_rename` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_rename` expects argument `source_path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `fs_rename` expects argument `destination_path` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::Void)
            }
            "fs_create_dir_all" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_create_dir_all` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_create_dir_all` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::Void)
            }
            "fs_remove_file" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_remove_file` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_remove_file` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::Void)
            }
            "fs_remove_dir_all" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_remove_dir_all` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_remove_dir_all` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::Void)
            }
            "fs_read_to_string" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_read_to_string` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_read_to_string` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::String)
            }
            "fs_write_string" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 2 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `fs_write_string` expects 2 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `fs_write_string` expects argument `path` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                self.expect_type_match(
                    &Type::String,
                    &argument_types[1],
                    expr.span,
                    format!(
                        "function `fs_write_string` expects argument `text` to be `string`, found `{}`",
                        argument_types[1].describe()
                    ),
                );
                Some(Type::Void)
            }
            "to_string" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `to_string` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                match &argument_types[0] {
                    Type::Error => Some(Type::Error),
                    Type::Void => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0022",
                                "function `to_string` expects argument `value` to be a concrete runtime value, found `<void>`",
                                self.info.source,
                                expr.span,
                            )
                            .with_note(
                                "`to_string` formats an existing runtime value; `println(...)` does not produce one",
                            )
                            .with_suggestion(
                                "call `to_string` on a string, number, bool, enum, struct, array, or slice value",
                            ),
                        );
                        Some(Type::Error)
                    }
                    Type::EmptyArrayLiteral => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0022",
                                "function `to_string` expects argument `value` to have a concrete runtime type, found `[]`",
                                self.info.source,
                                expr.span,
                            )
                            .with_note(
                                "an empty array literal must first be placed in an explicit zero-length array context",
                            )
                            .with_suggestion(
                                "bind `[]` as something like `[i32; 0]` before converting it with `to_string`",
                            ),
                        );
                        Some(Type::Error)
                    }
                    _ => Some(Type::String),
                }
            }
            _ => None,
        }
    }
}
