pub(super) fn write_path_helpers(module: &mut String) {
    module.push_str(
        r#"define private i1 @ax_path_is_separator(i8 %byte) {
entry:
  %is_slash = icmp eq i8 %byte, 47
  %is_backslash = icmp eq i8 %byte, 92
  %result = or i1 %is_slash, %is_backslash
  ret i1 %result
}

define private i64 @ax_path_last_separator_index(ptr %path) {
entry:
  %len = call i64 @strlen(ptr %path)
  br label %loop

loop:
  %index = phi i64 [0, %entry], [%next_index, %body]
  %last_sep = phi i64 [-1, %entry], [%next_last_sep, %body]
  %done = icmp uge i64 %index, %len
  br i1 %done, label %finish, label %body

body:
  %char_ptr = getelementptr i8, ptr %path, i64 %index
  %byte = load i8, ptr %char_ptr
  %is_sep = call i1 @ax_path_is_separator(i8 %byte)
  %next_last_sep = select i1 %is_sep, i64 %index, i64 %last_sep
  %next_index = add i64 %index, 1
  br label %loop

finish:
  ret i64 %last_sep
}

define private i64 @ax_path_file_name_start(ptr %path) {
entry:
  %last_sep = call i64 @ax_path_last_separator_index(ptr %path)
  %has_sep = icmp sge i64 %last_sep, 0
  %after_sep = add i64 %last_sep, 1
  %start = select i1 %has_sep, i64 %after_sep, i64 0
  ret i64 %start
}

define private i64 @ax_path_last_dot_after_start(ptr %path, i64 %start) {
entry:
  %len = call i64 @strlen(ptr %path)
  br label %loop

loop:
  %index = phi i64 [%start, %entry], [%next_index, %body]
  %last_dot = phi i64 [-1, %entry], [%next_last_dot, %body]
  %done = icmp uge i64 %index, %len
  br i1 %done, label %finish, label %body

body:
  %char_ptr = getelementptr i8, ptr %path, i64 %index
  %byte = load i8, ptr %char_ptr
  %is_dot = icmp eq i8 %byte, 46
  %next_last_dot = select i1 %is_dot, i64 %index, i64 %last_dot
  %next_index = add i64 %index, 1
  br label %loop

finish:
  ret i64 %last_dot
}

define private ptr @ax_path_parent(ptr %path) {
entry:
  %last_sep = call i64 @ax_path_last_separator_index(ptr %path)
  %has_parent = icmp sgt i64 %last_sep, 0
  %end = select i1 %has_parent, i64 %last_sep, i64 0
  %parent = call ptr @ax_string_copy_range(ptr %path, i64 0, i64 %end)
  ret ptr %parent
}

define private ptr @ax_path_file_name(ptr %path) {
entry:
  %len = call i64 @strlen(ptr %path)
  %start = call i64 @ax_path_file_name_start(ptr %path)
  %name = call ptr @ax_string_copy_range(ptr %path, i64 %start, i64 %len)
  ret ptr %name
}

define private ptr @ax_path_stem(ptr %path) {
entry:
  %len = call i64 @strlen(ptr %path)
  %start = call i64 @ax_path_file_name_start(ptr %path)
  %last_dot = call i64 @ax_path_last_dot_after_start(ptr %path, i64 %start)
  %dot_after_first = icmp sgt i64 %last_dot, %start
  %end = select i1 %dot_after_first, i64 %last_dot, i64 %len
  %stem = call ptr @ax_string_copy_range(ptr %path, i64 %start, i64 %end)
  ret ptr %stem
}

define private ptr @ax_path_extension(ptr %path) {
entry:
  %len = call i64 @strlen(ptr %path)
  %start = call i64 @ax_path_file_name_start(ptr %path)
  %last_dot = call i64 @ax_path_last_dot_after_start(ptr %path, i64 %start)
  %dot_after_first = icmp sgt i64 %last_dot, %start
  %extension_start = add i64 %last_dot, 1
  %valid = and i1 %dot_after_first, true
  br i1 %valid, label %copy_extension, label %empty

copy_extension:
  %extension = call ptr @ax_string_copy_range(ptr %path, i64 %extension_start, i64 %len)
  ret ptr %extension

empty:
  %empty_text = call ptr @ax_string_copy_range(ptr %path, i64 0, i64 0)
  ret ptr %empty_text
}

define private i1 @ax_path_is_absolute(ptr %path) {
entry:
  %len = call i64 @strlen(ptr %path)
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %no, label %check_first

check_first:
  %first = load i8, ptr %path
  %first_sep = call i1 @ax_path_is_separator(i8 %first)
  br i1 %first_sep, label %yes, label %check_drive_len

check_drive_len:
  %has_drive_len = icmp uge i64 %len, 3
  br i1 %has_drive_len, label %check_drive, label %no

check_drive:
  %colon_ptr = getelementptr i8, ptr %path, i64 1
  %colon = load i8, ptr %colon_ptr
  %has_colon = icmp eq i8 %colon, 58
  %third_ptr = getelementptr i8, ptr %path, i64 2
  %third = load i8, ptr %third_ptr
  %third_sep = call i1 @ax_path_is_separator(i8 %third)
  %drive_abs = and i1 %has_colon, %third_sep
  br i1 %drive_abs, label %yes, label %no

yes:
  ret i1 true

no:
  ret i1 false
}

"#,
    );

    if cfg!(windows) {
        module.push_str(
            r#"define private ptr @ax_process_cwd() {
entry:
  %buffer = call ptr @malloc(i64 4096)
  %alloc_failed = icmp eq ptr %buffer, null
  br i1 %alloc_failed, label %fail, label %read

read:
  %len = call i32 @GetCurrentDirectoryA(i32 4096, ptr %buffer)
  %failed = icmp eq i32 %len, 0
  br i1 %failed, label %fail, label %done

done:
  ret ptr %buffer

fail:
  call void @ax_runtime_error(ptr @.ax_rt_path_failed)
  unreachable
}

"#,
        );
    } else {
        module.push_str(
            r#"define private ptr @ax_process_cwd() {
entry:
  %buffer = call ptr @malloc(i64 4096)
  %alloc_failed = icmp eq ptr %buffer, null
  br i1 %alloc_failed, label %fail, label %read

read:
  %value = call ptr @getcwd(ptr %buffer, i64 4096)
  %failed = icmp eq ptr %value, null
  br i1 %failed, label %fail, label %done

done:
  ret ptr %buffer

fail:
  call void @ax_runtime_error(ptr @.ax_rt_path_failed)
  unreachable
}

"#,
        );
    }

    module.push_str(
        r#"define private ptr @ax_path_resolve(ptr %path) {
entry:
  %is_absolute = call i1 @ax_path_is_absolute(ptr %path)
  br i1 %is_absolute, label %copy_absolute, label %join_cwd

copy_absolute:
  %len = call i64 @strlen(ptr %path)
  %absolute = call ptr @ax_string_copy_range(ptr %path, i64 0, i64 %len)
  ret ptr %absolute

join_cwd:
  %cwd = call ptr @ax_process_cwd()
  %resolved = call ptr @ax_path_join(ptr %cwd, ptr %path)
  ret ptr %resolved
}

"#,
    );
}
