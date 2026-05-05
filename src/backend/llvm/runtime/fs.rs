pub(super) fn write_fs_helpers(module: &mut String) {
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
  store i8 92, ptr %separator
  %name_dest = getelementptr i8, ptr %buffer, i64 %name_start
  %copy_name_bytes = call ptr @memcpy(ptr %name_dest, ptr %name, i64 %name_len)
  %end = getelementptr i8, ptr %buffer, i64 %combined_len
  store i8 0, ptr %end
  ret ptr %buffer

fail:
  call void @ax_runtime_error(ptr @.ax_rt_path_failed)
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
  store i8 92, ptr %slash
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
  call void @ax_runtime_error(ptr @.ax_rt_path_failed)
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
