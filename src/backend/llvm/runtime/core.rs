use std::fmt::Write;

use super::super::abi;

pub(super) fn write_builtin_globals(module: &mut String) {
    writeln!(
        module,
        "@.ax_fmt_i32 = private unnamed_addr constant [3 x i8] c\"%d\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_fmt_f32 = private unnamed_addr constant [3 x i8] c\"%g\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_fmt_str = private unnamed_addr constant [3 x i8] c\"%s\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_text_true = private unnamed_addr constant [5 x i8] c\"true\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_text_false = private unnamed_addr constant [6 x i8] c\"false\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_text_space = private unnamed_addr constant [2 x i8] c\" \\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_text_newline = private unnamed_addr constant [2 x i8] c\"\\0A\\00\""
    )
    .expect("writing to string cannot fail");
    write_text_global(module, "@.ax_rt_div_zero", "R0021: division by zero\n");
    write_text_global(module, "@.ax_rt_mod_zero", "R0021: modulo by zero\n");
    write_text_global(
        module,
        "@.ax_rt_neg_overflow",
        "R0012: integer negation overflowed\n",
    );
    write_text_global(
        module,
        "@.ax_rt_add_overflow",
        "R0018: integer addition overflowed\n",
    );
    write_text_global(
        module,
        "@.ax_rt_sub_overflow",
        "R0019: integer subtraction overflowed\n",
    );
    write_text_global(
        module,
        "@.ax_rt_mul_overflow",
        "R0020: integer multiplication overflowed\n",
    );
    write_text_global(
        module,
        "@.ax_rt_div_overflow",
        "R0022: integer division overflowed\n",
    );
    write_text_global(
        module,
        "@.ax_rt_rem_overflow",
        "R0024: integer remainder overflowed\n",
    );
    write_text_global(
        module,
        "@.ax_rt_index_oob",
        "R0031: array index out of bounds\n",
    );
    write_text_global(
        module,
        "@.ax_rt_slice_bound_oob",
        "R0032: slice bound out of bounds\n",
    );
    write_text_global(
        module,
        "@.ax_rt_slice_order_invalid",
        "R0032: slice start bound is greater than end bound\n",
    );
    write_text_global(
        module,
        "@.ax_rt_argv_oob",
        "R0031: argv index out of bounds\n",
    );
    write_text_global(
        module,
        "@.ax_rt_env_missing",
        "R0053: environment variable is not available\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_read_failed",
        "R0061: failed to read file\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_metadata_failed",
        "R0103: failed to read file metadata\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_too_large",
        "R0104: file is too large to report as i32 bytes\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_write_failed",
        "R0075: failed to write file\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_remove_file_failed",
        "R0110: failed to remove file\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_rename_failed",
        "R0107: failed to rename file\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_copy_failed",
        "R0087: failed to copy file\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_copy_too_large",
        "R0086: copied file is too large to report as i32 bytes\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_create_dir_failed",
        "R0072: failed to create directory\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_remove_dir_failed",
        "R0113: failed to remove directory tree\n",
    );
    write_text_global(
        module,
        "@.ax_rt_fs_read_dir_failed",
        "R0123: failed to read directory\n",
    );
    write_text_global(
        module,
        "@.ax_rt_path_failed",
        "R0124: failed to resolve path\n",
    );
    write_text_global(
        module,
        "@.ax_rt_process_failed",
        "R0125: failed to run process command\n",
    );
    write_text_global(
        module,
        abi::HOST_ERROR_DEFAULT_MESSAGE,
        "host operation failed",
    );
    writeln!(
        module,
        "@.ax_path_dot = private unnamed_addr constant [2 x i8] c\".\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_path_dotdot = private unnamed_addr constant [3 x i8] c\"..\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_fs_mode_read_binary = private unnamed_addr constant [3 x i8] c\"rb\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_fs_mode_write_binary = private unnamed_addr constant [3 x i8] c\"wb\\00\""
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "@.ax_process_mode_read = private unnamed_addr constant [2 x i8] c\"r\\00\""
    )
    .expect("writing to string cannot fail");
}

pub(super) fn write_external_declarations(module: &mut String) {
    writeln!(module, "declare i32 @printf(ptr, ...)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @fputs(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @fopen(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @fgetc(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @fputc(i32, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @fclose(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare void @rewind(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @remove(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @rename(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare void @exit(i32)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @strcmp(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @strstr(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @strncmp(ptr, ptr, i64)").expect("writing to string cannot fail");
    writeln!(module, "declare i64 @strlen(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @getenv(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @malloc(i64)").expect("writing to string cannot fail");
    writeln!(module, "declare void @free(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @memcpy(ptr, ptr, i64)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @system(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @snprintf(ptr, i64, ptr, ...)")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "declare {{ i32, i1 }} @llvm.sadd.with.overflow.i32(i32, i32)"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "declare {{ i32, i1 }} @llvm.ssub.with.overflow.i32(i32, i32)"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "declare {{ i32, i1 }} @llvm.smul.with.overflow.i32(i32, i32)"
    )
    .expect("writing to string cannot fail");
    if cfg!(windows) {
        writeln!(module, "declare i32 @GetFileAttributesA(ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare i32 @CreateDirectoryA(ptr, ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare i32 @DeleteFileA(ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare i32 @RemoveDirectoryA(ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare i32 @GetCurrentDirectoryA(i32, ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare i32 @SetCurrentDirectoryA(ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare ptr @FindFirstFileA(ptr, ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare i32 @FindNextFileA(ptr, ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare i32 @FindClose(ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare ptr @_popen(ptr, ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare i32 @_pclose(ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare ptr @__acrt_iob_func(i32)")
            .expect("writing to string cannot fail");
    } else {
        writeln!(module, "declare i32 @access(ptr, i32)").expect("writing to string cannot fail");
        writeln!(module, "declare i32 @mkdir(ptr, i32)").expect("writing to string cannot fail");
        writeln!(module, "declare i32 @nftw(ptr, ptr, i32, i32)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare ptr @opendir(ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare ptr @readdir(ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare i32 @closedir(ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare ptr @getcwd(ptr, i64)").expect("writing to string cannot fail");
        writeln!(module, "declare i32 @chdir(ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare ptr @popen(ptr, ptr)").expect("writing to string cannot fail");
        writeln!(module, "declare i32 @pclose(ptr)").expect("writing to string cannot fail");
        writeln!(module, "@stderr = external global ptr").expect("writing to string cannot fail");
    }
}

pub(super) fn write_runtime_error_helper(module: &mut String) {
    writeln!(
        module,
        "define private void @{}(ptr %message) {{",
        abi::RUNTIME_ERROR_HELPER
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    if cfg!(windows) {
        writeln!(module, "  %stderr = call ptr @__acrt_iob_func(i32 2)")
            .expect("writing to string cannot fail");
    } else {
        writeln!(module, "  %stderr = load ptr, ptr @stderr")
            .expect("writing to string cannot fail");
    }
    writeln!(
        module,
        "  %written = call i32 @fputs(ptr %message, ptr %stderr)"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  call void @exit(i32 1)").expect("writing to string cannot fail");
    writeln!(module, "  unreachable").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}

fn write_text_global(module: &mut String, symbol: &str, value: &str) {
    writeln!(
        module,
        "{symbol} = private unnamed_addr constant [{} x i8] c\"{}\"",
        value.len() + 1,
        encode_llvm_c_string(value)
    )
    .expect("writing to string cannot fail");
}

fn encode_llvm_c_string(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'\\' => encoded.push_str("\\5C"),
            b'"' => encoded.push_str("\\22"),
            0x20..=0x7e => encoded.push(*byte as char),
            other => write!(encoded, "\\{other:02X}").expect("writing to string cannot fail"),
        }
    }
    encoded.push_str("\\00");
    encoded
}
