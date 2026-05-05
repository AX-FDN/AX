pub(super) fn write_process_helpers(module: &mut String) {
    module.push_str(
        r#"define private i32 @ax_process_run(ptr %command) {
entry:
  %status = call i32 @system(ptr %command)
  ret i32 %status
}

"#,
    );

    if cfg!(windows) {
        module.push_str(
            r#"define private i32 @ax_process_run_in(ptr %dir, ptr %command) {
entry:
  %cwd = call ptr @ax_process_cwd()
  %changed = call i32 @SetCurrentDirectoryA(ptr %dir)
  %change_failed = icmp eq i32 %changed, 0
  br i1 %change_failed, label %fail, label %run

run:
  %status = call i32 @system(ptr %command)
  %restored = call i32 @SetCurrentDirectoryA(ptr %cwd)
  %restore_failed = icmp eq i32 %restored, 0
  br i1 %restore_failed, label %fail, label %done

done:
  ret i32 %status

fail:
  call void @ax_runtime_error(ptr @.ax_rt_process_failed)
  unreachable
}

"#,
        );
    } else {
        module.push_str(
            r#"define private i32 @ax_process_run_in(ptr %dir, ptr %command) {
entry:
  %cwd = call ptr @ax_process_cwd()
  %changed = call i32 @chdir(ptr %dir)
  %change_failed = icmp ne i32 %changed, 0
  br i1 %change_failed, label %fail, label %run

run:
  %status = call i32 @system(ptr %command)
  %restored = call i32 @chdir(ptr %cwd)
  %restore_failed = icmp ne i32 %restored, 0
  br i1 %restore_failed, label %fail, label %done

done:
  ret i32 %status

fail:
  call void @ax_runtime_error(ptr @.ax_rt_process_failed)
  unreachable
}

"#,
        );
    }

    if cfg!(windows) {
        module.push_str(
            r#"define private ptr @ax_process_open_read(ptr %command) {
entry:
  %pipe = call ptr @_popen(ptr %command, ptr @.ax_process_mode_read)
  ret ptr %pipe
}

define private i32 @ax_process_close(ptr %pipe) {
entry:
  %status = call i32 @_pclose(ptr %pipe)
  ret i32 %status
}

"#,
        );
    } else {
        module.push_str(
            r#"define private ptr @ax_process_open_read(ptr %command) {
entry:
  %pipe = call ptr @popen(ptr %command, ptr @.ax_process_mode_read)
  ret ptr %pipe
}

define private i32 @ax_process_close(ptr %pipe) {
entry:
  %status = call i32 @pclose(ptr %pipe)
  ret i32 %status
}

"#,
        );
    }

    module.push_str(
        r#"define private ptr @ax_process_capture(ptr %command) {
entry:
  %pipe = call ptr @ax_process_open_read(ptr %command)
  %missing = icmp eq ptr %pipe, null
  br i1 %missing, label %fail, label %alloc

alloc:
  %buffer = call ptr @malloc(i64 128)
  %alloc_failed = icmp eq ptr %buffer, null
  br i1 %alloc_failed, label %fail_close, label %init

init:
  %buffer_slot = alloca ptr
  %capacity_slot = alloca i64
  %index_slot = alloca i64
  store ptr %buffer, ptr %buffer_slot
  store i64 128, ptr %capacity_slot
  store i64 0, ptr %index_slot
  br label %loop

loop:
  %ch = call i32 @fgetc(ptr %pipe)
  %is_eof = icmp eq i32 %ch, -1
  br i1 %is_eof, label %finish, label %ensure_capacity

ensure_capacity:
  %index = load i64, ptr %index_slot
  %capacity = load i64, ptr %capacity_slot
  %next_index = add i64 %index, 1
  %has_room = icmp ult i64 %next_index, %capacity
  br i1 %has_room, label %store, label %grow

grow:
  %old_buffer = load ptr, ptr %buffer_slot
  %new_capacity = mul i64 %capacity, 2
  %new_buffer = call ptr @malloc(i64 %new_capacity)
  %grow_failed = icmp eq ptr %new_buffer, null
  br i1 %grow_failed, label %fail_close, label %copy_grown

copy_grown:
  %copied = call ptr @memcpy(ptr %new_buffer, ptr %old_buffer, i64 %index)
  store ptr %new_buffer, ptr %buffer_slot
  store i64 %new_capacity, ptr %capacity_slot
  br label %store

store:
  %byte = trunc i32 %ch to i8
  %current_buffer = load ptr, ptr %buffer_slot
  %slot = getelementptr i8, ptr %current_buffer, i64 %index
  store i8 %byte, ptr %slot
  store i64 %next_index, ptr %index_slot
  br label %loop

finish:
  %final_index = load i64, ptr %index_slot
  %final_buffer = load ptr, ptr %buffer_slot
  %nul = getelementptr i8, ptr %final_buffer, i64 %final_index
  store i8 0, ptr %nul
  %status = call i32 @ax_process_close(ptr %pipe)
  %failed = icmp ne i32 %status, 0
  br i1 %failed, label %fail, label %done

done:
  ret ptr %final_buffer

fail_close:
  %closed_after_fail = call i32 @ax_process_close(ptr %pipe)
  br label %fail

fail:
  call void @ax_runtime_error(ptr @.ax_rt_process_failed)
  unreachable
}

define private ptr @ax_process_capture_in(ptr %dir, ptr %command) {
entry:
  %cwd = call ptr @ax_process_cwd()
"#,
    );

    if cfg!(windows) {
        module.push_str(
            r#"  %changed = call i32 @SetCurrentDirectoryA(ptr %dir)
  %change_failed = icmp eq i32 %changed, 0
"#,
        );
    } else {
        module.push_str(
            r#"  %changed = call i32 @chdir(ptr %dir)
  %change_failed = icmp ne i32 %changed, 0
"#,
        );
    }

    module.push_str(
        r#"  br i1 %change_failed, label %fail, label %capture

capture:
  %output = call ptr @ax_process_capture(ptr %command)
"#,
    );

    if cfg!(windows) {
        module.push_str(
            r#"  %restored = call i32 @SetCurrentDirectoryA(ptr %cwd)
  %restore_failed = icmp eq i32 %restored, 0
"#,
        );
    } else {
        module.push_str(
            r#"  %restored = call i32 @chdir(ptr %cwd)
  %restore_failed = icmp ne i32 %restored, 0
"#,
        );
    }

    module.push_str(
        r#"  br i1 %restore_failed, label %fail, label %done

done:
  ret ptr %output

fail:
  call void @ax_runtime_error(ptr @.ax_rt_process_failed)
  unreachable
}

"#,
    );
}
