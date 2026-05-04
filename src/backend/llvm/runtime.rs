use std::fmt::Write;

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
    writeln!(module, "declare ptr @memcpy(ptr, ptr, i64)").expect("writing to string cannot fail");
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
        writeln!(module, "declare ptr @FindFirstFileA(ptr, ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare i32 @FindNextFileA(ptr, ptr)")
            .expect("writing to string cannot fail");
        writeln!(module, "declare i32 @FindClose(ptr)").expect("writing to string cannot fail");
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
        writeln!(module, "@stderr = external global ptr").expect("writing to string cannot fail");
    }
}

pub(super) fn write_runtime_error_helper(module: &mut String) {
    writeln!(
        module,
        "define private void @ax_runtime_error(ptr %message) {{"
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

pub(super) fn write_string_helpers(module: &mut String) {
    write_string_len_helper(module);
    write_ascii_trim_space_helper(module);
    write_string_runtime_helpers(module);
    write_string_list_helpers(module);
}

pub(super) fn write_host_helpers(module: &mut String) {
    write_fs_helpers(module);
}

fn write_fs_helpers(module: &mut String) {
    if cfg!(windows) {
        module.push_str(
            r#"define private i1 @ax_fs_exists(ptr %path) {
entry:
  %attrs = call i32 @GetFileAttributesA(ptr %path)
  %exists = icmp ne i32 %attrs, -1
  ret i1 %exists
}

define private i1 @ax_fs_is_dir(ptr %path) {
entry:
  %attrs = call i32 @GetFileAttributesA(ptr %path)
  %exists = icmp ne i32 %attrs, -1
  br i1 %exists, label %check, label %missing

check:
  %dir_bits = and i32 %attrs, 16
  %is_dir = icmp ne i32 %dir_bits, 0
  ret i1 %is_dir

missing:
  ret i1 false
}

define private i1 @ax_fs_is_file(ptr %path) {
entry:
  %attrs = call i32 @GetFileAttributesA(ptr %path)
  %exists = icmp ne i32 %attrs, -1
  br i1 %exists, label %check, label %missing

check:
  %dir_bits = and i32 %attrs, 16
  %is_file = icmp eq i32 %dir_bits, 0
  ret i1 %is_file

missing:
  ret i1 false
}

"#,
        );
    } else {
        module.push_str(
            r#"define private i1 @ax_fs_exists(ptr %path) {
entry:
  %status = call i32 @access(ptr %path, i32 0)
  %exists = icmp eq i32 %status, 0
  ret i1 %exists
}

define private i1 @ax_fs_is_dir(ptr %path) {
entry:
  %dir = call ptr @opendir(ptr %path)
  %is_dir = icmp ne ptr %dir, null
  br i1 %is_dir, label %close, label %done

close:
  %closed = call i32 @closedir(ptr %dir)
  br label %done

done:
  %result = phi i1 [false, %entry], [true, %close]
  ret i1 %result
}

define private i1 @ax_fs_is_file(ptr %path) {
entry:
  %exists = call i1 @ax_fs_exists(ptr %path)
  br i1 %exists, label %check_dir, label %missing

check_dir:
  %is_dir = call i1 @ax_fs_is_dir(ptr %path)
  %is_file = xor i1 %is_dir, true
  ret i1 %is_file

missing:
  ret i1 false
}

"#,
        );
    }

    if cfg!(windows) {
        module.push_str(
            r#"define private void @ax_fs_mkdir_if_missing(ptr %path) {
entry:
  %already_dir = call i1 @ax_fs_is_dir(ptr %path)
  br i1 %already_dir, label %done, label %create

create:
  %status = call i32 @CreateDirectoryA(ptr %path, ptr null)
  %created = icmp ne i32 %status, 0
  br i1 %created, label %done, label %verify

verify:
  %now_dir = call i1 @ax_fs_is_dir(ptr %path)
  br i1 %now_dir, label %done, label %fail

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_create_dir_failed)
  unreachable

done:
  ret void
}

"#,
        );
    } else {
        module.push_str(
            r#"define private void @ax_fs_mkdir_if_missing(ptr %path) {
entry:
  %already_dir = call i1 @ax_fs_is_dir(ptr %path)
  br i1 %already_dir, label %done, label %create

create:
  %status = call i32 @mkdir(ptr %path, i32 493)
  %created = icmp eq i32 %status, 0
  br i1 %created, label %done, label %verify

verify:
  %now_dir = call i1 @ax_fs_is_dir(ptr %path)
  br i1 %now_dir, label %done, label %fail

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_create_dir_failed)
  unreachable

done:
  ret void
}

"#,
        );
    }

    if cfg!(windows) {
        module.push_str(
            r#"define private ptr @ax_path_join(ptr %base, ptr %name) {
entry:
  %base_len = call i64 @strlen(ptr %base)
  %name_len = call i64 @strlen(ptr %name)
  %name_start = add i64 %base_len, 1
  %combined_len = add i64 %name_start, %name_len
  %alloc_len = add i64 %combined_len, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  %alloc_failed = icmp eq ptr %buffer, null
  br i1 %alloc_failed, label %fail, label %copy_base

copy_base:
  %copy_base_bytes = call ptr @memcpy(ptr %buffer, ptr %base, i64 %base_len)
  %separator = getelementptr i8, ptr %buffer, i64 %base_len
  store i8 47, ptr %separator
  %name_dest = getelementptr i8, ptr %buffer, i64 %name_start
  %copy_name_bytes = call ptr @memcpy(ptr %name_dest, ptr %name, i64 %name_len)
  %end = getelementptr i8, ptr %buffer, i64 %combined_len
  store i8 0, ptr %end
  ret ptr %buffer

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_remove_dir_failed)
  unreachable
}

define private ptr @ax_fs_child_glob(ptr %path) {
entry:
  %len = call i64 @strlen(ptr %path)
  %slash_index = add i64 %len, 0
  %star_index = add i64 %len, 1
  %nul_index = add i64 %len, 2
  %alloc_len = add i64 %len, 3
  %buffer = call ptr @malloc(i64 %alloc_len)
  %alloc_failed = icmp eq ptr %buffer, null
  br i1 %alloc_failed, label %fail, label %copy

copy:
  %copied = call ptr @memcpy(ptr %buffer, ptr %path, i64 %len)
  %slash = getelementptr i8, ptr %buffer, i64 %slash_index
  store i8 47, ptr %slash
  %star = getelementptr i8, ptr %buffer, i64 %star_index
  store i8 42, ptr %star
  %nul = getelementptr i8, ptr %buffer, i64 %nul_index
  store i8 0, ptr %nul
  ret ptr %buffer

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_remove_dir_failed)
  unreachable
}

define private void @ax_fs_remove_dir_all(ptr %path) {
entry:
  %is_dir = call i1 @ax_fs_is_dir(ptr %path)
  br i1 %is_dir, label %scan, label %fail

scan:
  %pattern = call ptr @ax_fs_child_glob(ptr %path)
  %find_data = call ptr @malloc(i64 592)
  %alloc_failed = icmp eq ptr %find_data, null
  br i1 %alloc_failed, label %fail, label %open

open:
  %handle = call ptr @FindFirstFileA(ptr %pattern, ptr %find_data)
  %handle_addr = ptrtoint ptr %handle to i64
  %missing = icmp eq i64 %handle_addr, -1
  br i1 %missing, label %remove_self, label %process

process:
  %name = getelementptr i8, ptr %find_data, i64 44
  %dot_cmp = call i32 @strcmp(ptr %name, ptr @.ax_path_dot)
  %is_dot = icmp eq i32 %dot_cmp, 0
  %dotdot_cmp = call i32 @strcmp(ptr %name, ptr @.ax_path_dotdot)
  %is_dotdot = icmp eq i32 %dotdot_cmp, 0
  %skip = or i1 %is_dot, %is_dotdot
  br i1 %skip, label %next, label %remove_child

remove_child:
  %child = call ptr @ax_path_join(ptr %path, ptr %name)
  %attrs = load i32, ptr %find_data
  %dir_bits = and i32 %attrs, 16
  %is_child_dir = icmp ne i32 %dir_bits, 0
  br i1 %is_child_dir, label %remove_child_dir, label %remove_child_file

remove_child_dir:
  call void @ax_fs_remove_dir_all(ptr %child)
  br label %next

remove_child_file:
  %delete_status = call i32 @DeleteFileA(ptr %child)
  %deleted = icmp ne i32 %delete_status, 0
  br i1 %deleted, label %next, label %fail_with_handle

next:
  %next_status = call i32 @FindNextFileA(ptr %handle, ptr %find_data)
  %has_next = icmp ne i32 %next_status, 0
  br i1 %has_next, label %process, label %close

close:
  %closed = call i32 @FindClose(ptr %handle)
  br label %remove_self

fail_with_handle:
  %closed_after_fail = call i32 @FindClose(ptr %handle)
  br label %fail

remove_self:
  %remove_status = call i32 @RemoveDirectoryA(ptr %path)
  %removed = icmp ne i32 %remove_status, 0
  br i1 %removed, label %done, label %fail

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_remove_dir_failed)
  unreachable

done:
  ret void
}

"#,
        );
    } else {
        module.push_str(
            r#"define private ptr @ax_path_join(ptr %base, ptr %name) {
entry:
  %base_len = call i64 @strlen(ptr %base)
  %name_len = call i64 @strlen(ptr %name)
  %name_start = add i64 %base_len, 1
  %combined_len = add i64 %name_start, %name_len
  %alloc_len = add i64 %combined_len, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  %alloc_failed = icmp eq ptr %buffer, null
  br i1 %alloc_failed, label %fail, label %copy_base

copy_base:
  %copy_base_bytes = call ptr @memcpy(ptr %buffer, ptr %base, i64 %base_len)
  %separator = getelementptr i8, ptr %buffer, i64 %base_len
  store i8 47, ptr %separator
  %name_dest = getelementptr i8, ptr %buffer, i64 %name_start
  %copy_name_bytes = call ptr @memcpy(ptr %name_dest, ptr %name, i64 %name_len)
  %end = getelementptr i8, ptr %buffer, i64 %combined_len
  store i8 0, ptr %end
  ret ptr %buffer

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_read_dir_failed)
  unreachable
}

define private i32 @ax_fs_remove_nftw_callback(ptr %path, ptr %stat, i32 %typeflag, ptr %ftwbuf) {
entry:
  %status = call i32 @remove(ptr %path)
  ret i32 %status
}

define private void @ax_fs_remove_dir_all(ptr %path) {
entry:
  %status = call i32 @nftw(ptr %path, ptr @ax_fs_remove_nftw_callback, i32 16, i32 9)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %fail, label %done

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_remove_dir_failed)
  unreachable

done:
  ret void
}

"#,
        );
    }

    module.push_str(
        r#"define private void @ax_sort_string_ptrs(ptr %data, i32 %len) {
entry:
  %too_short = icmp sle i32 %len, 1
  br i1 %too_short, label %done, label %outer

outer:
  %i = phi i32 [1, %entry], [%next_i, %place_key]
  %i64 = sext i32 %i to i64
  %key_slot = getelementptr ptr, ptr %data, i64 %i64
  %key = load ptr, ptr %key_slot
  %j_start = sub i32 %i, 1
  br label %inner

inner:
  %j = phi i32 [%j_start, %outer], [%prev_j, %shift]
  %slot_target = phi i32 [%i, %outer], [%j, %shift]
  %j_nonnegative = icmp sge i32 %j, 0
  br i1 %j_nonnegative, label %compare, label %place_key

compare:
  %j64 = sext i32 %j to i64
  %j_slot = getelementptr ptr, ptr %data, i64 %j64
  %j_value = load ptr, ptr %j_slot
  %cmp = call i32 @strcmp(ptr %j_value, ptr %key)
  %greater = icmp sgt i32 %cmp, 0
  br i1 %greater, label %shift, label %place_key

shift:
  %target64 = sext i32 %slot_target to i64
  %target_slot = getelementptr ptr, ptr %data, i64 %target64
  store ptr %j_value, ptr %target_slot
  %prev_j = sub i32 %j, 1
  br label %inner

place_key:
  %place_index = phi i32 [%slot_target, %inner], [%slot_target, %compare]
  %place64 = sext i32 %place_index to i64
  %place_slot = getelementptr ptr, ptr %data, i64 %place64
  store ptr %key, ptr %place_slot
  %next_i = add i32 %i, 1
  %more = icmp slt i32 %next_i, %len
  br i1 %more, label %outer, label %done

done:
  ret void
}

define private { ptr, i32 } @ax_string_list_to_sorted_slice(ptr %list) {
entry:
  %len = load i32, ptr %list
  %data_ptr = getelementptr i8, ptr %list, i64 8
  %data = load ptr, ptr %data_ptr
  call void @ax_sort_string_ptrs(ptr %data, i32 %len)
  %with_data = insertvalue { ptr, i32 } undef, ptr %data, 0
  %slice = insertvalue { ptr, i32 } %with_data, i32 %len, 1
  ret { ptr, i32 } %slice
}

"#,
    );

    if cfg!(windows) {
        module.push_str(
            r#"define private { ptr, i32 } @ax_fs_read_dir(ptr %path) {
entry:
  %is_dir = call i1 @ax_fs_is_dir(ptr %path)
  br i1 %is_dir, label %scan, label %fail

scan:
  %list = call ptr @ax_string_list_new()
  %pattern = call ptr @ax_fs_child_glob(ptr %path)
  %find_data = call ptr @malloc(i64 592)
  %alloc_failed = icmp eq ptr %find_data, null
  br i1 %alloc_failed, label %fail, label %open

open:
  %handle = call ptr @FindFirstFileA(ptr %pattern, ptr %find_data)
  %handle_addr = ptrtoint ptr %handle to i64
  %missing = icmp eq i64 %handle_addr, -1
  br i1 %missing, label %finish, label %process

process:
  %name = getelementptr i8, ptr %find_data, i64 44
  %dot_cmp = call i32 @strcmp(ptr %name, ptr @.ax_path_dot)
  %is_dot = icmp eq i32 %dot_cmp, 0
  %dotdot_cmp = call i32 @strcmp(ptr %name, ptr @.ax_path_dotdot)
  %is_dotdot = icmp eq i32 %dotdot_cmp, 0
  %skip = or i1 %is_dot, %is_dotdot
  br i1 %skip, label %next, label %push

push:
  %child = call ptr @ax_path_join(ptr %path, ptr %name)
  %pushed = call ptr @ax_string_list_push(ptr %list, ptr %child)
  br label %next

next:
  %next_status = call i32 @FindNextFileA(ptr %handle, ptr %find_data)
  %has_next = icmp ne i32 %next_status, 0
  br i1 %has_next, label %process, label %close

close:
  %closed = call i32 @FindClose(ptr %handle)
  br label %finish

finish:
  %result = call { ptr, i32 } @ax_string_list_to_sorted_slice(ptr %list)
  ret { ptr, i32 } %result

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_read_dir_failed)
  unreachable
}

"#,
        );
    } else {
        module.push_str(
            r#"define private { ptr, i32 } @ax_fs_read_dir(ptr %path) {
entry:
  %dir = call ptr @opendir(ptr %path)
  %missing = icmp eq ptr %dir, null
  br i1 %missing, label %fail, label %opened

opened:
  %list = call ptr @ax_string_list_new()
  br label %loop

loop:
  %entry_value = call ptr @readdir(ptr %dir)
  %done_reading = icmp eq ptr %entry_value, null
  br i1 %done_reading, label %close, label %process

process:
  %name = getelementptr i8, ptr %entry_value, i64 19
  %dot_cmp = call i32 @strcmp(ptr %name, ptr @.ax_path_dot)
  %is_dot = icmp eq i32 %dot_cmp, 0
  %dotdot_cmp = call i32 @strcmp(ptr %name, ptr @.ax_path_dotdot)
  %is_dotdot = icmp eq i32 %dotdot_cmp, 0
  %skip = or i1 %is_dot, %is_dotdot
  br i1 %skip, label %loop, label %push

push:
  %child = call ptr @ax_path_join(ptr %path, ptr %name)
  %pushed = call ptr @ax_string_list_push(ptr %list, ptr %child)
  br label %loop

close:
  %closed = call i32 @closedir(ptr %dir)
  %close_failed = icmp ne i32 %closed, 0
  br i1 %close_failed, label %fail, label %finish

finish:
  %result = call { ptr, i32 } @ax_string_list_to_sorted_slice(ptr %list)
  ret { ptr, i32 } %result

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_read_dir_failed)
  unreachable
}

"#,
        );
    }

    module.push_str(
        r#"define private i32 @ax_fs_file_size(ptr %path) {
entry:
  %file = call ptr @fopen(ptr %path, ptr @.ax_fs_mode_read_binary)
  %missing = icmp eq ptr %file, null
  br i1 %missing, label %fail_open, label %loop

fail_open:
  call void @ax_runtime_error(ptr @.ax_rt_fs_metadata_failed)
  unreachable

loop:
  %count = phi i64 [0, %entry], [%next_count, %count_continue]
  %byte = call i32 @fgetc(ptr %file)
  %eof = icmp eq i32 %byte, -1
  br i1 %eof, label %done, label %count_byte

count_byte:
  %next_count = add i64 %count, 1
  %too_large = icmp sgt i64 %next_count, 2147483647
  br i1 %too_large, label %too_large_fail, label %count_continue

count_continue:
  br label %loop

too_large_fail:
  %closed_large = call i32 @fclose(ptr %file)
  call void @ax_runtime_error(ptr @.ax_rt_fs_too_large)
  unreachable

done:
  %closed = call i32 @fclose(ptr %file)
  %size = trunc i64 %count to i32
  ret i32 %size
}

define private ptr @ax_fs_read_to_string(ptr %path) {
entry:
  %file = call ptr @fopen(ptr %path, ptr @.ax_fs_mode_read_binary)
  %missing = icmp eq ptr %file, null
  br i1 %missing, label %fail_open, label %count_loop

fail_open:
  call void @ax_runtime_error(ptr @.ax_rt_fs_read_failed)
  unreachable

count_loop:
  %count = phi i64 [0, %entry], [%next_count, %count_continue]
  %byte = call i32 @fgetc(ptr %file)
  %eof = icmp eq i32 %byte, -1
  br i1 %eof, label %counted, label %count_byte

count_byte:
  %next_count = add i64 %count, 1
  %too_large = icmp sgt i64 %next_count, 2147483647
  br i1 %too_large, label %too_large_fail, label %count_continue

count_continue:
  br label %count_loop

too_large_fail:
  %closed_large = call i32 @fclose(ptr %file)
  call void @ax_runtime_error(ptr @.ax_rt_fs_too_large)
  unreachable

counted:
  call void @rewind(ptr %file)
  %alloc_len = add i64 %count, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  %alloc_failed = icmp eq ptr %buffer, null
  br i1 %alloc_failed, label %fail_alloc, label %read_loop

fail_alloc:
  %closed_alloc = call i32 @fclose(ptr %file)
  call void @ax_runtime_error(ptr @.ax_rt_fs_read_failed)
  unreachable

read_loop:
  %index = phi i64 [0, %counted], [%next_index, %read_continue]
  %done_reading = icmp uge i64 %index, %count
  br i1 %done_reading, label %finish, label %read_byte

read_byte:
  %char = call i32 @fgetc(ptr %file)
  %read_eof = icmp eq i32 %char, -1
  br i1 %read_eof, label %finish, label %store_byte

store_byte:
  %byte_value = trunc i32 %char to i8
  %dest = getelementptr i8, ptr %buffer, i64 %index
  store i8 %byte_value, ptr %dest
  %next_index = add i64 %index, 1
  br label %read_continue

read_continue:
  br label %read_loop

finish:
  %final_index = phi i64 [%index, %read_loop], [%index, %read_byte]
  %nul = getelementptr i8, ptr %buffer, i64 %final_index
  store i8 0, ptr %nul
  %closed = call i32 @fclose(ptr %file)
  ret ptr %buffer
}

define private void @ax_fs_write_string(ptr %path, ptr %text) {
entry:
  %file = call ptr @fopen(ptr %path, ptr @.ax_fs_mode_write_binary)
  %missing = icmp eq ptr %file, null
  br i1 %missing, label %fail_open, label %write

fail_open:
  call void @ax_runtime_error(ptr @.ax_rt_fs_write_failed)
  unreachable

write:
  %written = call i32 @fputs(ptr %text, ptr %file)
  %write_failed = icmp slt i32 %written, 0
  br i1 %write_failed, label %fail_write, label %close

fail_write:
  %closed_after_write_fail = call i32 @fclose(ptr %file)
  call void @ax_runtime_error(ptr @.ax_rt_fs_write_failed)
  unreachable

close:
  %closed = call i32 @fclose(ptr %file)
  %close_failed = icmp ne i32 %closed, 0
  br i1 %close_failed, label %fail_close, label %done

fail_close:
  call void @ax_runtime_error(ptr @.ax_rt_fs_write_failed)
  unreachable

done:
  ret void
}

define private void @ax_fs_remove_file(ptr %path) {
entry:
  %status = call i32 @remove(ptr %path)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %fail, label %done

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_remove_file_failed)
  unreachable

done:
  ret void
}

define private void @ax_fs_rename(ptr %from, ptr %to) {
entry:
  %status = call i32 @rename(ptr %from, ptr %to)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %fail, label %done

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_rename_failed)
  unreachable

done:
  ret void
}

define private i32 @ax_fs_copy_file(ptr %source, ptr %destination) {
entry:
  %input = call ptr @fopen(ptr %source, ptr @.ax_fs_mode_read_binary)
  %input_missing = icmp eq ptr %input, null
  br i1 %input_missing, label %fail_open_input, label %open_output

fail_open_input:
  call void @ax_runtime_error(ptr @.ax_rt_fs_copy_failed)
  unreachable

open_output:
  %output = call ptr @fopen(ptr %destination, ptr @.ax_fs_mode_write_binary)
  %output_missing = icmp eq ptr %output, null
  br i1 %output_missing, label %fail_open_output, label %loop

fail_open_output:
  %closed_input_after_output_fail = call i32 @fclose(ptr %input)
  call void @ax_runtime_error(ptr @.ax_rt_fs_copy_failed)
  unreachable

loop:
  %count = phi i64 [0, %open_output], [%next_count, %copy_continue]
  %byte = call i32 @fgetc(ptr %input)
  %eof = icmp eq i32 %byte, -1
  br i1 %eof, label %close, label %write_byte

write_byte:
  %written = call i32 @fputc(i32 %byte, ptr %output)
  %write_failed = icmp eq i32 %written, -1
  br i1 %write_failed, label %fail_write, label %count_byte

count_byte:
  %next_count = add i64 %count, 1
  %too_large = icmp sgt i64 %next_count, 2147483647
  br i1 %too_large, label %too_large_fail, label %copy_continue

copy_continue:
  br label %loop

fail_write:
  %closed_input_after_write_fail = call i32 @fclose(ptr %input)
  %closed_output_after_write_fail = call i32 @fclose(ptr %output)
  call void @ax_runtime_error(ptr @.ax_rt_fs_copy_failed)
  unreachable

too_large_fail:
  %closed_input_after_large_fail = call i32 @fclose(ptr %input)
  %closed_output_after_large_fail = call i32 @fclose(ptr %output)
  call void @ax_runtime_error(ptr @.ax_rt_fs_copy_too_large)
  unreachable

close:
  %closed_input = call i32 @fclose(ptr %input)
  %closed_output = call i32 @fclose(ptr %output)
  %input_close_failed = icmp ne i32 %closed_input, 0
  %output_close_failed = icmp ne i32 %closed_output, 0
  %close_failed = or i1 %input_close_failed, %output_close_failed
  br i1 %close_failed, label %fail_close, label %done_copy

fail_close:
  call void @ax_runtime_error(ptr @.ax_rt_fs_copy_failed)
  unreachable

done_copy:
  %copied = trunc i64 %count to i32
  ret i32 %copied
}

define private void @ax_fs_create_dir_all(ptr %path) {
entry:
  %len = call i64 @strlen(ptr %path)
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %done, label %alloc

alloc:
  %alloc_len = add i64 %len, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  %alloc_failed = icmp eq ptr %buffer, null
  br i1 %alloc_failed, label %fail, label %copy

copy:
  %copied_path = call ptr @memcpy(ptr %buffer, ptr %path, i64 %alloc_len)
  br label %loop

loop:
  %index = phi i64 [0, %copy], [%next_index, %step]
  %scan_done = icmp uge i64 %index, %len
  br i1 %scan_done, label %final, label %check_byte

check_byte:
  %char_ptr = getelementptr i8, ptr %buffer, i64 %index
  %byte = load i8, ptr %char_ptr
  %is_slash = icmp eq i8 %byte, 47
  %is_backslash = icmp eq i8 %byte, 92
  %is_separator = or i1 %is_slash, %is_backslash
  br i1 %is_separator, label %maybe_prefix, label %step

maybe_prefix:
  %is_first_separator = icmp eq i64 %index, 0
  br i1 %is_first_separator, label %step, label %check_drive

check_drive:
  %is_drive_separator = icmp eq i64 %index, 2
  br i1 %is_drive_separator, label %check_drive_colon, label %create_prefix

check_drive_colon:
  %colon_ptr = getelementptr i8, ptr %buffer, i64 1
  %colon = load i8, ptr %colon_ptr
  %has_drive_colon = icmp eq i8 %colon, 58
  br i1 %has_drive_colon, label %step, label %create_prefix

create_prefix:
  store i8 0, ptr %char_ptr
  call void @ax_fs_mkdir_if_missing(ptr %buffer)
  store i8 %byte, ptr %char_ptr
  br label %step

step:
  %next_index = add i64 %index, 1
  br label %loop

final:
  call void @ax_fs_mkdir_if_missing(ptr %buffer)
  br label %done

fail:
  call void @ax_runtime_error(ptr @.ax_rt_fs_create_dir_failed)
  unreachable

done:
  ret void
}

"#,
    );
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

fn write_string_len_helper(module: &mut String) {
    writeln!(module, "define private i32 @ax_string_len(ptr %text) {{")
        .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  br label %loop").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "loop:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %index = phi i32 [0, %entry], [%next_index, %body]"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %count = phi i32 [0, %entry], [%next_count, %body]"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %char_ptr = getelementptr i8, ptr %text, i32 %index"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %byte = load i8, ptr %char_ptr").expect("writing to string cannot fail");
    writeln!(module, "  %is_end = icmp eq i8 %byte, 0").expect("writing to string cannot fail");
    writeln!(module, "  br i1 %is_end, label %done, label %body")
        .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "body:").expect("writing to string cannot fail");
    writeln!(module, "  %prefix = and i8 %byte, -64").expect("writing to string cannot fail");
    writeln!(module, "  %is_continuation = icmp eq i8 %prefix, -128")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %starts_codepoint = xor i1 %is_continuation, true"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %delta = zext i1 %starts_codepoint to i32")
        .expect("writing to string cannot fail");
    writeln!(module, "  %next_count = add i32 %count, %delta")
        .expect("writing to string cannot fail");
    writeln!(module, "  %next_index = add i32 %index, 1").expect("writing to string cannot fail");
    writeln!(module, "  br label %loop").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "done:").expect("writing to string cannot fail");
    writeln!(module, "  ret i32 %count").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}

fn write_string_runtime_helpers(module: &mut String) {
    writeln!(
        module,
        "define private ptr @ax_string_concat(ptr %left, ptr %right) {{"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  %left_len = call i64 @strlen(ptr %left)")
        .expect("writing to string cannot fail");
    writeln!(module, "  %right_len = call i64 @strlen(ptr %right)")
        .expect("writing to string cannot fail");
    writeln!(module, "  %combined_len = add i64 %left_len, %right_len")
        .expect("writing to string cannot fail");
    writeln!(module, "  %total_len = add i64 %combined_len, 1")
        .expect("writing to string cannot fail");
    writeln!(module, "  %buffer = call ptr @malloc(i64 %total_len)")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %copy_left = call ptr @memcpy(ptr %buffer, ptr %left, i64 %left_len)"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %right_dest = getelementptr i8, ptr %buffer, i64 %left_len"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %copy_right = call ptr @memcpy(ptr %right_dest, ptr %right, i64 %right_len)"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %end = getelementptr i8, ptr %buffer, i64 %combined_len"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  store i8 0, ptr %end").expect("writing to string cannot fail");
    writeln!(module, "  ret ptr %buffer").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");

    write_string_replace_helpers(module);
    write_string_split_lines_helpers(module);

    writeln!(module, "define private ptr @ax_string_trim(ptr %text) {{")
        .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  %len = call i64 @strlen(ptr %text)")
        .expect("writing to string cannot fail");
    writeln!(module, "  br label %lead_loop").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "lead_loop:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %start = phi i64 [0, %entry], [%next_start, %lead_step]"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %lead_done_len = icmp uge i64 %start, %len")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  br i1 %lead_done_len, label %tail_init, label %lead_check"
    )
    .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "lead_check:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %lead_ptr = getelementptr i8, ptr %text, i64 %start"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %lead_byte = load i8, ptr %lead_ptr")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %lead_space = call i1 @ax_is_ascii_trim_space(i8 %lead_byte)"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  br i1 %lead_space, label %lead_step, label %tail_init"
    )
    .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "lead_step:").expect("writing to string cannot fail");
    writeln!(module, "  %next_start = add i64 %start, 1").expect("writing to string cannot fail");
    writeln!(module, "  br label %lead_loop").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "tail_init:").expect("writing to string cannot fail");
    writeln!(module, "  br label %tail_loop").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "tail_loop:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %end = phi i64 [%len, %tail_init], [%next_end, %tail_step]"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %tail_done = icmp ule i64 %end, %start")
        .expect("writing to string cannot fail");
    writeln!(module, "  br i1 %tail_done, label %copy, label %tail_check")
        .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "tail_check:").expect("writing to string cannot fail");
    writeln!(module, "  %tail_index = sub i64 %end, 1").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %tail_ptr = getelementptr i8, ptr %text, i64 %tail_index"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %tail_byte = load i8, ptr %tail_ptr")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %tail_space = call i1 @ax_is_ascii_trim_space(i8 %tail_byte)"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  br i1 %tail_space, label %tail_step, label %copy")
        .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "tail_step:").expect("writing to string cannot fail");
    writeln!(module, "  %next_end = sub i64 %end, 1").expect("writing to string cannot fail");
    writeln!(module, "  br label %tail_loop").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "copy:").expect("writing to string cannot fail");
    writeln!(module, "  %trim_len = sub i64 %end, %start").expect("writing to string cannot fail");
    writeln!(module, "  %alloc_len = add i64 %trim_len, 1").expect("writing to string cannot fail");
    writeln!(module, "  %buffer = call ptr @malloc(i64 %alloc_len)")
        .expect("writing to string cannot fail");
    writeln!(module, "  %src = getelementptr i8, ptr %text, i64 %start")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %copy_trimmed = call ptr @memcpy(ptr %buffer, ptr %src, i64 %trim_len)"
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %nul = getelementptr i8, ptr %buffer, i64 %trim_len"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  store i8 0, ptr %nul").expect("writing to string cannot fail");
    writeln!(module, "  ret ptr %buffer").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");

    writeln!(
        module,
        "define private ptr @ax_i32_to_string(i32 %value) {{"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  %buffer = call ptr @malloc(i64 12)")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %written = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %buffer, i64 12, ptr @.ax_fmt_i32, i32 %value)"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  ret ptr %buffer").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");

    writeln!(
        module,
        "define private ptr @ax_f32_to_string(float %value) {{"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  %buffer = call ptr @malloc(i64 40)")
        .expect("writing to string cannot fail");
    writeln!(module, "  %wide = fpext float %value to double")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %written = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %buffer, i64 40, ptr @.ax_fmt_f32, double %wide)"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  br label %scan").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "scan:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %index = phi i32 [0, %entry], [%next_index, %scan_continue]"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %scan_done = icmp sge i32 %index, %written")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  br i1 %scan_done, label %append_decimal, label %scan_byte"
    )
    .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "scan_byte:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %byte_ptr = getelementptr i8, ptr %buffer, i32 %index"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %byte = load i8, ptr %byte_ptr").expect("writing to string cannot fail");
    writeln!(module, "  %is_dot = icmp eq i8 %byte, 46").expect("writing to string cannot fail");
    writeln!(module, "  %is_lower_exp = icmp eq i8 %byte, 101")
        .expect("writing to string cannot fail");
    writeln!(module, "  %is_upper_exp = icmp eq i8 %byte, 69")
        .expect("writing to string cannot fail");
    writeln!(module, "  %has_exp = or i1 %is_lower_exp, %is_upper_exp")
        .expect("writing to string cannot fail");
    writeln!(module, "  %has_marker = or i1 %is_dot, %has_exp")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  br i1 %has_marker, label %done, label %scan_continue"
    )
    .expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "scan_continue:").expect("writing to string cannot fail");
    writeln!(module, "  %next_index = add i32 %index, 1").expect("writing to string cannot fail");
    writeln!(module, "  br label %scan").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "append_decimal:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %dot_ptr = getelementptr i8, ptr %buffer, i32 %written"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  store i8 46, ptr %dot_ptr").expect("writing to string cannot fail");
    writeln!(module, "  %zero_index = add i32 %written, 1").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %zero_ptr = getelementptr i8, ptr %buffer, i32 %zero_index"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  store i8 48, ptr %zero_ptr").expect("writing to string cannot fail");
    writeln!(module, "  %nul_index = add i32 %written, 2").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %nul_ptr = getelementptr i8, ptr %buffer, i32 %nul_index"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  store i8 0, ptr %nul_ptr").expect("writing to string cannot fail");
    writeln!(module, "  br label %done").expect("writing to string cannot fail");
    writeln!(module).expect("writing to string cannot fail");
    writeln!(module, "done:").expect("writing to string cannot fail");
    writeln!(module, "  ret ptr %buffer").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}

fn write_string_replace_helpers(module: &mut String) {
    module.push_str(
        r#"define private ptr @ax_string_replace(ptr %text, ptr %from, ptr %to) {
entry:
  %text_len = call i64 @strlen(ptr %text)
  %from_len = call i64 @strlen(ptr %from)
  %to_len = call i64 @strlen(ptr %to)
  %empty_from = icmp eq i64 %from_len, 0
  br i1 %empty_from, label %replace_empty, label %count_loop

replace_empty:
  %empty_result = call ptr @ax_string_replace_empty_from(ptr %text, ptr %to)
  ret ptr %empty_result

count_loop:
  %count_cursor = phi ptr [%text, %entry], [%count_next_cursor, %count_step]
  %match_count = phi i64 [0, %entry], [%next_match_count, %count_step]
  %count_match = call ptr @strstr(ptr %count_cursor, ptr %from)
  %count_done = icmp eq ptr %count_match, null
  br i1 %count_done, label %allocate, label %count_step

count_step:
  %next_match_count = add i64 %match_count, 1
  %count_next_cursor = getelementptr i8, ptr %count_match, i64 %from_len
  br label %count_loop

allocate:
  %delta = sub i64 %to_len, %from_len
  %delta_total = mul i64 %match_count, %delta
  %new_len = add i64 %text_len, %delta_total
  %alloc_len = add i64 %new_len, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  br label %copy_loop

copy_loop:
  %copy_cursor = phi ptr [%text, %allocate], [%copy_next_cursor, %copy_step]
  %dest_cursor = phi ptr [%buffer, %allocate], [%dest_next, %copy_step]
  %copy_match = call ptr @strstr(ptr %copy_cursor, ptr %from)
  %copy_done = icmp eq ptr %copy_match, null
  br i1 %copy_done, label %copy_tail, label %copy_step

copy_step:
  %match_int = ptrtoint ptr %copy_match to i64
  %cursor_int = ptrtoint ptr %copy_cursor to i64
  %prefix_len = sub i64 %match_int, %cursor_int
  %copy_prefix = call ptr @memcpy(ptr %dest_cursor, ptr %copy_cursor, i64 %prefix_len)
  %dest_after_prefix = getelementptr i8, ptr %dest_cursor, i64 %prefix_len
  %copy_replacement = call ptr @memcpy(ptr %dest_after_prefix, ptr %to, i64 %to_len)
  %dest_next = getelementptr i8, ptr %dest_after_prefix, i64 %to_len
  %copy_next_cursor = getelementptr i8, ptr %copy_match, i64 %from_len
  br label %copy_loop

copy_tail:
  %tail_len = call i64 @strlen(ptr %copy_cursor)
  %copy_rest = call ptr @memcpy(ptr %dest_cursor, ptr %copy_cursor, i64 %tail_len)
  %nul = getelementptr i8, ptr %dest_cursor, i64 %tail_len
  store i8 0, ptr %nul
  ret ptr %buffer
}

define private ptr @ax_string_replace_empty_from(ptr %text, ptr %to) {
entry:
  %text_len = call i64 @strlen(ptr %text)
  %to_len = call i64 @strlen(ptr %to)
  %codepoints_i32 = call i32 @ax_string_len(ptr %text)
  %codepoints = zext i32 %codepoints_i32 to i64
  %insertions = add i64 %codepoints, 1
  %insert_bytes = mul i64 %insertions, %to_len
  %new_len = add i64 %text_len, %insert_bytes
  %alloc_len = add i64 %new_len, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  %copy_initial = call ptr @memcpy(ptr %buffer, ptr %to, i64 %to_len)
  %initial_dest = getelementptr i8, ptr %buffer, i64 %to_len
  %empty_text = icmp eq i64 %text_len, 0
  br i1 %empty_text, label %finish_empty, label %loop

finish_empty:
  store i8 0, ptr %initial_dest
  ret ptr %buffer

loop:
  %index = phi i64 [0, %entry], [%next_index, %continue]
  %dest_cursor = phi ptr [%initial_dest, %entry], [%next_dest, %continue]
  %src_ptr = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %src_ptr
  store i8 %byte, ptr %dest_cursor
  %dest_after_byte = getelementptr i8, ptr %dest_cursor, i64 1
  %next_index = add i64 %index, 1
  %at_end = icmp eq i64 %next_index, %text_len
  br i1 %at_end, label %insert_after_char, label %check_next_byte

check_next_byte:
  %next_src = getelementptr i8, ptr %text, i64 %next_index
  %next_byte = load i8, ptr %next_src
  %prefix = and i8 %next_byte, -64
  %is_continuation = icmp eq i8 %prefix, -128
  br i1 %is_continuation, label %continue_no_insert, label %insert_after_char

insert_after_char:
  %copy_insert = call ptr @memcpy(ptr %dest_after_byte, ptr %to, i64 %to_len)
  %dest_after_insert = getelementptr i8, ptr %dest_after_byte, i64 %to_len
  br label %continue

continue_no_insert:
  br label %continue

continue:
  %next_dest = phi ptr [%dest_after_insert, %insert_after_char], [%dest_after_byte, %continue_no_insert]
  %done = icmp eq i64 %next_index, %text_len
  br i1 %done, label %finish, label %loop

finish:
  store i8 0, ptr %next_dest
  ret ptr %buffer
}

"#,
    );
}

fn write_string_list_helpers(module: &mut String) {
    module.push_str(
        r#"define private ptr @ax_string_list_new() {
entry:
  %list = call ptr @malloc(i64 16)
  %data = call ptr @malloc(i64 32)
  store i32 0, ptr %list
  %cap_ptr = getelementptr i8, ptr %list, i64 4
  store i32 4, ptr %cap_ptr
  %data_ptr = getelementptr i8, ptr %list, i64 8
  store ptr %data, ptr %data_ptr
  ret ptr %list
}

define private i32 @ax_string_list_len(ptr %list) {
entry:
  %len = load i32, ptr %list
  ret i32 %len
}

define private ptr @ax_string_list_push(ptr %list, ptr %value) {
entry:
  %len = load i32, ptr %list
  %cap_ptr = getelementptr i8, ptr %list, i64 4
  %cap = load i32, ptr %cap_ptr
  %needs_grow = icmp sge i32 %len, %cap
  br i1 %needs_grow, label %grow, label %store

grow:
  %new_cap = mul i32 %cap, 2
  %new_cap64 = sext i32 %new_cap to i64
  %new_bytes = mul i64 %new_cap64, 8
  %new_data = call ptr @malloc(i64 %new_bytes)
  %data_ptr_grow = getelementptr i8, ptr %list, i64 8
  %old_data = load ptr, ptr %data_ptr_grow
  %len64_grow = sext i32 %len to i64
  %copy_bytes = mul i64 %len64_grow, 8
  %copy = call ptr @memcpy(ptr %new_data, ptr %old_data, i64 %copy_bytes)
  store ptr %new_data, ptr %data_ptr_grow
  store i32 %new_cap, ptr %cap_ptr
  br label %store

store:
  %data_ptr = getelementptr i8, ptr %list, i64 8
  %data = load ptr, ptr %data_ptr
  %len64 = sext i32 %len to i64
  %slot = getelementptr ptr, ptr %data, i64 %len64
  store ptr %value, ptr %slot
  %next_len = add i32 %len, 1
  store i32 %next_len, ptr %list
  ret ptr %list
}

define private ptr @ax_string_list_get(ptr %list, i32 %index) {
entry:
  %len = load i32, ptr %list
  %below_zero = icmp slt i32 %index, 0
  %past_end = icmp sge i32 %index, %len
  %out_of_bounds = or i1 %below_zero, %past_end
  br i1 %out_of_bounds, label %fail, label %ok

fail:
  call void @ax_runtime_error(ptr @.ax_rt_index_oob)
  unreachable

ok:
  %data_ptr = getelementptr i8, ptr %list, i64 8
  %data = load ptr, ptr %data_ptr
  %index64 = sext i32 %index to i64
  %slot = getelementptr ptr, ptr %data, i64 %index64
  %value = load ptr, ptr %slot
  ret ptr %value
}

define private ptr @ax_string_list_join(ptr %list, ptr %separator) {
entry:
  %len = load i32, ptr %list
  %separator_len = call i64 @strlen(ptr %separator)
  %data_ptr = getelementptr i8, ptr %list, i64 8
  %data = load ptr, ptr %data_ptr
  br label %count_loop

count_loop:
  %count_index = phi i32 [0, %entry], [%next_count_index, %count_body]
  %total = phi i64 [0, %entry], [%next_total, %count_body]
  %count_done = icmp sge i32 %count_index, %len
  br i1 %count_done, label %allocate, label %count_body

count_body:
  %count_index64 = sext i32 %count_index to i64
  %count_slot = getelementptr ptr, ptr %data, i64 %count_index64
  %count_item = load ptr, ptr %count_slot
  %item_len = call i64 @strlen(ptr %count_item)
  %needs_separator = icmp ne i32 %count_index, 0
  %separator_extra = select i1 %needs_separator, i64 %separator_len, i64 0
  %with_separator = add i64 %total, %separator_extra
  %next_total = add i64 %with_separator, %item_len
  %next_count_index = add i32 %count_index, 1
  br label %count_loop

allocate:
  %alloc_len = add i64 %total, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  br label %copy_loop

copy_loop:
  %copy_index = phi i32 [0, %allocate], [%next_copy_index, %copy_item]
  %cursor = phi ptr [%buffer, %allocate], [%next_cursor, %copy_item]
  %copy_done = icmp sge i32 %copy_index, %len
  br i1 %copy_done, label %finish, label %copy_separator_check

copy_separator_check:
  %copy_needs_separator = icmp ne i32 %copy_index, 0
  br i1 %copy_needs_separator, label %copy_separator, label %copy_item

copy_separator:
  %copy_separator_bytes = call ptr @memcpy(ptr %cursor, ptr %separator, i64 %separator_len)
  %after_separator = getelementptr i8, ptr %cursor, i64 %separator_len
  br label %copy_item

copy_item:
  %item_dest = phi ptr [%cursor, %copy_separator_check], [%after_separator, %copy_separator]
  %copy_index64 = sext i32 %copy_index to i64
  %copy_slot = getelementptr ptr, ptr %data, i64 %copy_index64
  %copy_item_value = load ptr, ptr %copy_slot
  %copy_item_len = call i64 @strlen(ptr %copy_item_value)
  %copy_item_bytes = call ptr @memcpy(ptr %item_dest, ptr %copy_item_value, i64 %copy_item_len)
  %next_cursor = getelementptr i8, ptr %item_dest, i64 %copy_item_len
  %next_copy_index = add i32 %copy_index, 1
  br label %copy_loop

finish:
  store i8 0, ptr %cursor
  ret ptr %buffer
}

"#,
    );
}

fn write_string_split_lines_helpers(module: &mut String) {
    module.push_str(
        r#"define private ptr @ax_string_copy_range(ptr %text, i64 %start, i64 %end) {
entry:
  %len = sub i64 %end, %start
  %alloc_len = add i64 %len, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  %src = getelementptr i8, ptr %text, i64 %start
  %copy = call ptr @memcpy(ptr %buffer, ptr %src, i64 %len)
  %nul = getelementptr i8, ptr %buffer, i64 %len
  store i8 0, ptr %nul
  ret ptr %buffer
}

define private { ptr, i32 } @ax_string_split_lines(ptr %text) {
entry:
  %text_len = call i64 @strlen(ptr %text)
  %is_empty = icmp eq i64 %text_len, 0
  br i1 %is_empty, label %empty, label %count_loop

empty:
  %empty_array = call ptr @malloc(i64 0)
  %empty_with_ptr = insertvalue { ptr, i32 } undef, ptr %empty_array, 0
  %empty_slice = insertvalue { ptr, i32 } %empty_with_ptr, i32 0, 1
  ret { ptr, i32 } %empty_slice

count_loop:
  %count_index = phi i64 [0, %entry], [%next_count_index, %count_loop]
  %newline_count = phi i64 [0, %entry], [%next_newline_count, %count_loop]
  %count_ptr = getelementptr i8, ptr %text, i64 %count_index
  %count_byte = load i8, ptr %count_ptr
  %count_is_lf = icmp eq i8 %count_byte, 10
  %count_delta = zext i1 %count_is_lf to i64
  %next_newline_count = add i64 %newline_count, %count_delta
  %next_count_index = add i64 %count_index, 1
  %count_done = icmp eq i64 %next_count_index, %text_len
  br i1 %count_done, label %count_done_block, label %count_loop

count_done_block:
  %last_index = sub i64 %text_len, 1
  %last_ptr = getelementptr i8, ptr %text, i64 %last_index
  %last_byte = load i8, ptr %last_ptr
  %ends_lf = icmp eq i8 %last_byte, 10
  %has_tail = xor i1 %ends_lf, true
  %tail_delta = zext i1 %has_tail to i64
  %line_count = add i64 %next_newline_count, %tail_delta
  %array_bytes = mul i64 %line_count, 8
  %array = call ptr @malloc(i64 %array_bytes)
  br label %fill_loop

fill_loop:
  %index = phi i64 [0, %count_done_block], [%advanced_index, %advance_non_lf], [%after_lf_index, %copy_line]
  %line_start = phi i64 [0, %count_done_block], [%same_line_start, %advance_non_lf], [%after_lf_index, %copy_line]
  %slot = phi i64 [0, %count_done_block], [%same_slot, %advance_non_lf], [%next_slot, %copy_line]
  %fill_done = icmp uge i64 %index, %text_len
  br i1 %fill_done, label %tail_or_finish, label %check_byte

check_byte:
  %byte_ptr = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %byte_ptr
  %is_lf = icmp eq i8 %byte, 10
  br i1 %is_lf, label %emit_line, label %advance_non_lf

advance_non_lf:
  %advanced_index = add i64 %index, 1
  %same_line_start = add i64 %line_start, 0
  %same_slot = add i64 %slot, 0
  br label %fill_loop

emit_line:
  %has_content = icmp ugt i64 %index, %line_start
  br i1 %has_content, label %check_cr, label %copy_line

check_cr:
  %before_lf_index = sub i64 %index, 1
  %before_lf_ptr = getelementptr i8, ptr %text, i64 %before_lf_index
  %before_lf = load i8, ptr %before_lf_ptr
  %has_cr = icmp eq i8 %before_lf, 13
  %cr_adjusted_end = select i1 %has_cr, i64 %before_lf_index, i64 %index
  br label %copy_line

copy_line:
  %line_end = phi i64 [%index, %emit_line], [%cr_adjusted_end, %check_cr]
  %line = call ptr @ax_string_copy_range(ptr %text, i64 %line_start, i64 %line_end)
  %slot_ptr = getelementptr ptr, ptr %array, i64 %slot
  store ptr %line, ptr %slot_ptr
  %next_slot = add i64 %slot, 1
  %after_lf_index = add i64 %index, 1
  br label %fill_loop

tail_or_finish:
  %emit_tail = icmp ult i64 %line_start, %text_len
  br i1 %emit_tail, label %copy_tail, label %finish

copy_tail:
  %tail_line = call ptr @ax_string_copy_range(ptr %text, i64 %line_start, i64 %text_len)
  %tail_slot_ptr = getelementptr ptr, ptr %array, i64 %slot
  store ptr %tail_line, ptr %tail_slot_ptr
  br label %finish

finish:
  %line_count_i32 = trunc i64 %line_count to i32
  %with_ptr = insertvalue { ptr, i32 } undef, ptr %array, 0
  %slice = insertvalue { ptr, i32 } %with_ptr, i32 %line_count_i32, 1
  ret { ptr, i32 } %slice
}

"#,
    );
}

fn write_ascii_trim_space_helper(module: &mut String) {
    writeln!(
        module,
        "define private i1 @ax_is_ascii_trim_space(i8 %byte) {{"
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  %is_space = icmp eq i8 %byte, 32").expect("writing to string cannot fail");
    writeln!(module, "  %is_tab = icmp eq i8 %byte, 9").expect("writing to string cannot fail");
    writeln!(module, "  %is_lf = icmp eq i8 %byte, 10").expect("writing to string cannot fail");
    writeln!(module, "  %is_vt = icmp eq i8 %byte, 11").expect("writing to string cannot fail");
    writeln!(module, "  %is_ff = icmp eq i8 %byte, 12").expect("writing to string cannot fail");
    writeln!(module, "  %is_cr = icmp eq i8 %byte, 13").expect("writing to string cannot fail");
    writeln!(module, "  %space_or_tab = or i1 %is_space, %is_tab")
        .expect("writing to string cannot fail");
    writeln!(module, "  %lf_or_vt = or i1 %is_lf, %is_vt").expect("writing to string cannot fail");
    writeln!(module, "  %ff_or_cr = or i1 %is_ff, %is_cr").expect("writing to string cannot fail");
    writeln!(module, "  %left = or i1 %space_or_tab, %lf_or_vt")
        .expect("writing to string cannot fail");
    writeln!(module, "  %result = or i1 %left, %ff_or_cr").expect("writing to string cannot fail");
    writeln!(module, "  ret i1 %result").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_prelude_exposes_string_and_stdio_abi() {
        let mut module = String::new();
        write_builtin_globals(&mut module);
        write_external_declarations(&mut module);
        write_string_helpers(&mut module);

        assert!(module.contains("@.ax_fmt_i32"));
        assert!(module.contains("@.ax_rt_env_missing"));
        assert!(module.contains("declare i32 @printf(ptr, ...)"));
        assert!(module.contains("declare ptr @getenv(ptr)"));
        assert!(module.contains("declare ptr @strstr(ptr, ptr)"));
        assert!(module.contains("define private i32 @ax_string_len(ptr %text)"));
        assert!(module.contains("define private i1 @ax_is_ascii_trim_space(i8 %byte)"));
        assert!(module.contains("define private ptr @ax_string_concat(ptr %left, ptr %right)"));
        assert!(
            module.contains("define private ptr @ax_string_replace(ptr %text, ptr %from, ptr %to)")
        );
        assert!(
            module.contains("define private ptr @ax_string_replace_empty_from(ptr %text, ptr %to)")
        );
        assert!(module.contains("define private ptr @ax_string_copy_range(ptr %text"));
        assert!(module.contains("define private { ptr, i32 } @ax_string_split_lines(ptr %text)"));
        assert!(module.contains("define private ptr @ax_string_trim(ptr %text)"));
        assert!(module.contains("define private ptr @ax_string_list_new()"));
        assert!(module.contains("define private i32 @ax_string_list_len(ptr %list)"));
        assert!(module.contains("define private ptr @ax_string_list_push(ptr %list, ptr %value)"));
        assert!(module.contains("define private ptr @ax_string_list_get(ptr %list, i32 %index)"));
        assert!(
            module.contains("define private ptr @ax_string_list_join(ptr %list, ptr %separator)")
        );
        assert!(module.contains("define private ptr @ax_i32_to_string(i32 %value)"));
        assert!(module.contains("define private ptr @ax_f32_to_string(float %value)"));
        assert!(module.contains("append_decimal:"));
    }
}
