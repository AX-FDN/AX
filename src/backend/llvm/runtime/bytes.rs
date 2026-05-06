use std::fmt::Write;

use super::super::abi;

pub(super) fn write_bytes_helpers(module: &mut String) {
    writeln!(module, "; bytes ABI: {}", abi::bytes_abi_name())
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "; bytes layout: header={} len_off={} cap_off={} data_off={} initial_cap={}",
        abi::BYTES_HEADER_BYTES,
        abi::BYTES_LENGTH_OFFSET,
        abi::BYTES_CAPACITY_OFFSET,
        abi::BYTES_DATA_OFFSET,
        abi::BYTES_INITIAL_CAPACITY
    )
    .expect("writing to string cannot fail");
    module.push_str(
        r#"define private ptr @ax_bytes_empty() {
entry:
  %buffer = call ptr @malloc(i64 8)
  store i32 0, ptr %buffer
  %capacity_ptr = getelementptr i8, ptr %buffer, i64 4
  store i32 0, ptr %capacity_ptr
  ret ptr %buffer
}

define private ptr @ax_bytes_from_string(ptr %text) {
entry:
  %len64 = call i64 @strlen(ptr %text)
  %len = trunc i64 %len64 to i32
  %payload_bytes = sext i32 %len to i64
  %alloc_len = add i64 %payload_bytes, 8
  %buffer = call ptr @malloc(i64 %alloc_len)
  store i32 %len, ptr %buffer
  %capacity_ptr = getelementptr i8, ptr %buffer, i64 4
  store i32 %len, ptr %capacity_ptr
  %data_ptr = getelementptr i8, ptr %buffer, i64 8
  %copy = call ptr @memcpy(ptr %data_ptr, ptr %text, i64 %payload_bytes)
  ret ptr %buffer
}

define private i32 @ax_bytes_len(ptr %bytes) {
entry:
  %len = load i32, ptr %bytes
  ret i32 %len
}

define private i32 @ax_bytes_get(ptr %bytes, i32 %index) {
entry:
  %len = load i32, ptr %bytes
  %below_zero = icmp slt i32 %index, 0
  %past_end = icmp sge i32 %index, %len
  %out_of_bounds = or i1 %below_zero, %past_end
  br i1 %out_of_bounds, label %fail, label %ok

fail:
  call void @ax_runtime_error(ptr @.ax_rt_index_oob)
  unreachable

ok:
  %data_ptr = getelementptr i8, ptr %bytes, i64 8
  %index64 = sext i32 %index to i64
  %byte_ptr = getelementptr i8, ptr %data_ptr, i64 %index64
  %byte = load i8, ptr %byte_ptr
  %value = zext i8 %byte to i32
  ret i32 %value
}

define private ptr @ax_bytes_push(ptr %bytes, i32 %value) {
entry:
  %len = load i32, ptr %bytes
  %capacity_ptr = getelementptr i8, ptr %bytes, i64 4
  %capacity = load i32, ptr %capacity_ptr
  %needs_grow = icmp sge i32 %len, %capacity
  br i1 %needs_grow, label %grow, label %store

grow:
  %current_cap = select i1 %needs_grow, i32 %capacity, i32 %capacity
  %base_cap = icmp eq i32 %current_cap, 0
  %doubled_cap = mul i32 %current_cap, 2
  %new_cap = select i1 %base_cap, i32 8, i32 %doubled_cap
  %new_cap64 = sext i32 %new_cap to i64
  %new_bytes = add i64 %new_cap64, 8
  %new_buffer = call ptr @malloc(i64 %new_bytes)
  %old_len64 = sext i32 %len to i64
  %old_total = add i64 %old_len64, 8
  %copy = call ptr @memcpy(ptr %new_buffer, ptr %bytes, i64 %old_total)
  %new_capacity_ptr = getelementptr i8, ptr %new_buffer, i64 4
  store i32 %new_cap, ptr %new_capacity_ptr
  br label %store

store:
  %target = phi ptr [%bytes, %entry], [%new_buffer, %grow]
  %target_data = getelementptr i8, ptr %target, i64 8
  %slot_index = sext i32 %len to i64
  %slot = getelementptr i8, ptr %target_data, i64 %slot_index
  %byte = trunc i32 %value to i8
  store i8 %byte, ptr %slot
  %next_len = add i32 %len, 1
  store i32 %next_len, ptr %target
  ret ptr %target
}

define private ptr @ax_bytes_to_string_lossy(ptr %bytes) {
entry:
  %len = load i32, ptr %bytes
  %len64 = sext i32 %len to i64
  %alloc_len = add i64 %len64, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  %data = getelementptr i8, ptr %bytes, i64 8
  %copy = call ptr @memcpy(ptr %buffer, ptr %data, i64 %len64)
  %nul = getelementptr i8, ptr %buffer, i64 %len64
  store i8 0, ptr %nul
  ret ptr %buffer
}

define private ptr @ax_bytes_to_hex(ptr %bytes) {
entry:
  %len = load i32, ptr %bytes
  %len64 = sext i32 %len to i64
  %hex_bytes = mul i64 %len64, 2
  %alloc_len = add i64 %hex_bytes, 1
  %buffer = call ptr @malloc(i64 %alloc_len)
  %data = getelementptr i8, ptr %bytes, i64 8
  br label %loop

loop:
  %index = phi i32 [0, %entry], [%next_index, %body]
  %cursor = phi ptr [%buffer, %entry], [%next_cursor, %body]
  %done = icmp sge i32 %index, %len
  br i1 %done, label %finish, label %body

body:
  %index64 = sext i32 %index to i64
  %byte_ptr = getelementptr i8, ptr %data, i64 %index64
  %byte = load i8, ptr %byte_ptr
  %unsigned = zext i8 %byte to i32
  %hi = lshr i32 %unsigned, 4
  %lo = and i32 %unsigned, 15
  %hi_char = call i8 @ax_bytes_hex_digit(i32 %hi)
  %lo_char = call i8 @ax_bytes_hex_digit(i32 %lo)
  store i8 %hi_char, ptr %cursor
  %cursor_after_hi = getelementptr i8, ptr %cursor, i64 1
  store i8 %lo_char, ptr %cursor_after_hi
  %next_cursor = getelementptr i8, ptr %cursor, i64 2
  %next_index = add i32 %index, 1
  br label %loop

finish:
  %nul = getelementptr i8, ptr %buffer, i64 %hex_bytes
  store i8 0, ptr %nul
  ret ptr %buffer
}

define private i8 @ax_bytes_hex_digit(i32 %value) {
entry:
  %is_small = icmp ult i32 %value, 10
  br i1 %is_small, label %digit, label %alpha

digit:
  %ascii = add i32 %value, 48
  %byte = trunc i32 %ascii to i8
  ret i8 %byte

alpha:
  %adjusted = sub i32 %value, 10
  %ascii = add i32 %adjusted, 97
  %byte = trunc i32 %ascii to i8
  ret i8 %byte
}

"#,
    );
}
