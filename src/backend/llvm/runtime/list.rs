use std::fmt::Write;

use super::super::abi;

pub(super) fn write_string_list_helpers(module: &mut String) {
    writeln!(module, "; string list ABI: {}", abi::STRING_LIST_ABI_NAME)
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "; string list layout: header={} len_off={} cap_off={} data_off={} initial_cap={} data_bytes={}",
        abi::STRING_LIST_HEADER_BYTES,
        abi::STRING_LIST_LEN_OFFSET,
        abi::STRING_LIST_CAPACITY_OFFSET,
        abi::STRING_LIST_DATA_OFFSET,
        abi::STRING_LIST_INITIAL_CAPACITY,
        abi::STRING_LIST_DATA_BYTES
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "define private void @{}(ptr %list) {{",
        abi::STRING_LIST_RELEASE_HELPER
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  ret void").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
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
