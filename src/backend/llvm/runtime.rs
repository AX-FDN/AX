use std::fmt::Write;

pub(super) fn write_builtin_globals(module: &mut String) {
    writeln!(
        module,
        "@.ax_fmt_i32 = private unnamed_addr constant [3 x i8] c\"%d\\00\""
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
}

pub(super) fn write_external_declarations(module: &mut String) {
    writeln!(module, "declare i32 @printf(ptr, ...)").expect("writing to string cannot fail");
    writeln!(module, "declare void @exit(i32)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @strcmp(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @strstr(ptr, ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @strncmp(ptr, ptr, i64)").expect("writing to string cannot fail");
    writeln!(module, "declare i64 @strlen(ptr)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @malloc(i64)").expect("writing to string cannot fail");
    writeln!(module, "declare ptr @memcpy(ptr, ptr, i64)").expect("writing to string cannot fail");
    writeln!(module, "declare i32 @snprintf(ptr, i64, ptr, ...)")
        .expect("writing to string cannot fail");
}

pub(super) fn write_string_helpers(module: &mut String) {
    write_string_len_helper(module);
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
        assert!(module.contains("declare i32 @printf(ptr, ...)"));
        assert!(module.contains("declare ptr @strstr(ptr, ptr)"));
        assert!(module.contains("define private i32 @ax_string_len(ptr %text)"));
        assert!(module.contains("define private ptr @ax_string_concat(ptr %left, ptr %right)"));
        assert!(module.contains("define private ptr @ax_i32_to_string(i32 %value)"));
    }
}
