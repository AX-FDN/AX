use std::fmt::Write;

pub(super) fn write_string_helpers(module: &mut String) {
    write_string_len_helper(module);
    write_ascii_trim_space_helper(module);
    write_string_runtime_helpers(module);
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
