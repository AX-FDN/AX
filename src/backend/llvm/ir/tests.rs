use super::render_program;
use crate::frontend::analyze;
use crate::source::SourceFile;

fn render(source_text: &str) -> String {
    let source = SourceFile::anonymous(source_text);
    let output = analyze(&source);
    assert!(
        output.diagnostics.is_empty(),
        "source should analyze before LLVM IR rendering: {:?}",
        output.diagnostics
    );
    render_program(output.mir.as_ref().expect("MIR should exist")).expect("LLVM IR should render")
}

#[test]
fn renders_minimal_main_return() {
    let rendered = render(
        "\
fn main() -> i32 {
return 0;
}
",
    );

    assert!(rendered.contains("define i32 @main(i32 %argc, ptr %argv)"));
    assert!(rendered.contains("store i32 %argc, ptr @.ax_argc"));
    assert!(rendered.contains("store ptr %argv, ptr @.ax_argv"));
    assert!(rendered.contains("ret i32 0"));
}

#[test]
fn skips_unreachable_unsupported_function_signatures_v0() {
    let rendered = render(
        "\
fn unused(items: string_list) -> string_list {
return items;
}

fn main() -> i32 {
return 0;
}
",
    );

    assert!(rendered.contains("define i32 @main(i32 %argc, ptr %argv)"));
    assert!(!rendered.contains("@ax_unused"));
}

#[test]
fn renders_i32_function_calls_and_arithmetic() {
    let rendered = render(
        "\
fn add(left: i32, right: i32) -> i32 {
return left + right;
}

fn main() -> i32 {
return add(1, 2);
}
",
    );

    assert!(rendered.contains("define i32 @ax_add(i32 %arg0, i32 %arg1)"));
    assert!(rendered.contains("call { i32, i1 } @llvm.sadd.with.overflow.i32"));
    assert!(rendered.contains("= call i32 @ax_add(i32 1, i32 2)"));
}

#[test]
fn renders_i32_overflow_guards_v0() {
    let rendered = render(
        "\
fn ops(left: i32, right: i32) -> i32 {
let negated: i32 = -left;
return negated + (left + right) + (left - right) + (left * right) + (left / right) + (left % right);
}

fn main() -> i32 {
return ops(8, 2);
}
",
    );

    assert!(rendered.contains("declare { i32, i1 } @llvm.sadd.with.overflow.i32(i32, i32)"));
    assert!(rendered.contains("declare { i32, i1 } @llvm.ssub.with.overflow.i32(i32, i32)"));
    assert!(rendered.contains("declare { i32, i1 } @llvm.smul.with.overflow.i32(i32, i32)"));
    assert!(rendered.contains("call { i32, i1 } @llvm.sadd.with.overflow.i32"));
    assert!(rendered.contains("call { i32, i1 } @llvm.ssub.with.overflow.i32"));
    assert!(rendered.contains("call { i32, i1 } @llvm.smul.with.overflow.i32"));
    assert!(rendered.contains("i32_neg_overflow"));
    assert!(rendered.contains("i32_add_overflow"));
    assert!(rendered.contains("i32_sub_overflow"));
    assert!(rendered.contains("i32_mul_overflow"));
    assert!(rendered.contains("i32_div_overflow"));
    assert!(rendered.contains("i32_rem_overflow"));
    assert!(rendered.contains("@.ax_rt_neg_overflow"));
    assert!(rendered.contains("@.ax_rt_add_overflow"));
    assert!(rendered.contains("@.ax_rt_sub_overflow"));
    assert!(rendered.contains("@.ax_rt_mul_overflow"));
    assert!(rendered.contains("@.ax_rt_div_overflow"));
    assert!(rendered.contains("@.ax_rt_rem_overflow"));
}

#[test]
fn renders_division_by_zero_guards_v0() {
    let rendered = render(
        "\
fn div_i32(left: i32, right: i32) -> i32 {
return left / right;
}

fn rem_i32(left: i32, right: i32) -> i32 {
return left % right;
}

fn div_f32(left: f32, right: f32) -> f32 {
return left / right;
}

fn main() -> i32 {
println(div_f32(1.0, 2.0));
return div_i32(8, 2) + rem_i32(9, 4);
}
",
    );

    assert!(rendered.contains("icmp eq i32"));
    assert!(rendered.contains("fcmp oeq float"));
    assert!(rendered.contains("i32_div_zero"));
    assert!(rendered.contains("f32_div_zero"));
    assert!(rendered.contains("@.ax_rt_div_zero"));
    assert!(rendered.contains("@.ax_rt_mod_zero"));
    assert!(rendered.contains("define private void @ax_runtime_error(ptr %message)"));
    assert!(rendered.contains("call i32 @fputs(ptr %message"));
    assert!(rendered.contains("call void @exit(i32 1)"));
    assert!(rendered.contains("sdiv i32"));
    assert!(rendered.contains("srem i32"));
    assert!(rendered.contains("fdiv float"));
}

#[test]
fn renders_f32_core_values_arithmetic_and_formatting_v0() {
    let rendered = render(
        "\
fn calc(value: f32) -> f32 {
return ((value + 2.5) * 2.0) - (1.0 / 2.0);
}

fn main() -> i32 {
let result: f32 = calc(1.5);
let negated: f32 = -result;
let text: string = to_string(result);
let mut total: i32 = 0;
if (result > 7.4) {
    total = total + 1;
}
if (result == 7.5) {
    total = total + 2;
}
if (result != 7.0) {
    total = total + 4;
}
if (negated < 0.0) {
    total = total + 8;
}
if (string_len(text) == 3) {
    total = total + 16;
}
println(result);
return total;
}
",
    );

    assert!(rendered.contains("define float @ax_calc(float %arg0)"));
    assert!(rendered.contains("fadd float"));
    assert!(rendered.contains("fmul float"));
    assert!(rendered.contains("fdiv float"));
    assert!(rendered.contains("fneg float"));
    assert!(rendered.contains("fcmp ogt float"));
    assert!(rendered.contains("fcmp oeq float"));
    assert!(rendered.contains("call ptr @ax_f32_to_string(float"));
    assert!(rendered.contains("fpext float"));
}

#[test]
fn renders_f32_composites_v0() {
    let rendered = render(
        "\
const SCALE: f32 = 2.0;

struct Reading {
label: string,
value: f32,
}

fn slice_sum(values: [f32]) -> f32 {
let mut total: f32 = 0.0;
for (let value: f32 in values) {
    total = total + value;
}
return total;
}

fn main() -> i32 {
let values: [f32; 3] = [1.5, 2.5, 3.5];
let same: [f32; 3] = [1.5, 2.5, 3.5];
let other: [f32; 3] = [1.5, 2.5, 4.5];
let left: [f32] = values[1:3];
let same_slice: [f32] = same[1:3];
let other_slice: [f32] = other[1:3];
let reading: Reading = Reading { label: \"ok\", value: values[1] * SCALE };
let same_reading: Reading = Reading { label: \"ok\", value: 5.0 };
let other_reading: Reading = Reading { label: \"ok\", value: values[2] * SCALE };
let mut total: i32 = 0;
if (values == same) {
    total = total + 1;
}
if (values != other) {
    total = total + 2;
}
if (left == same_slice) {
    total = total + 4;
}
if (left != other_slice) {
    total = total + 8;
}
if (reading == same_reading) {
    total = total + 16;
}
if (reading != other_reading) {
    total = total + 32;
}
if (slice_sum(values) == 7.5) {
    total = total + 64;
}
if (string_len(to_string(reading)) > 0) {
    total = total + 128;
}
println(values);
println(left);
println(reading);
println(slice_sum(left));
return total;
}
",
    );

    assert!(rendered.contains("define float @ax_slice_sum({ ptr, i32 } %arg0)"));
    assert!(rendered.contains("[3 x float]"));
    assert!(rendered.contains("%ax_struct_Reading = type { ptr, float }"));
    assert!(rendered.contains("getelementptr float"));
    assert!(rendered.contains("fadd float"));
    assert!(rendered.contains("fcmp oeq float"));
    assert!(rendered.contains("slice_eq_loop"));
    assert!(rendered.contains("call ptr @ax_f32_to_string(float"));
}

#[test]
fn renders_i32_and_bool_println_calls() {
    let rendered = render(
        "\
fn main() -> i32 {
println(7);
println(true);
return 0;
}
",
    );

    assert!(rendered.contains("declare i32 @printf(ptr, ...)"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_i32, i32 7)"));
    assert!(rendered.contains("select i1 1, ptr @.ax_text_true, ptr @.ax_text_false"));
}

#[test]
fn renders_short_circuit_logical_ops_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let ready: bool = true;
let has_input: bool = false;
let should_run: bool = ready && !has_input || false;
if (false && 8 / 0 == 0) {
return 1;
}
if (should_run || true) {
return 7;
}
return 0;
}
",
    );

    assert!(rendered.contains("logical_rhs"));
    assert!(rendered.contains("logical_done"));
    assert!(rendered.contains("alloca i1"));
    assert!(rendered.contains("br i1 0, label %logical_rhs"));
    assert!(rendered.contains("br i1 %"));
}

#[test]
fn renders_string_literal_println_calls() {
    let rendered = render(
        "\
fn main() -> i32 {
println(\"hello\");
println(\"C:\\\\AX\");
return 0;
}
",
    );

    assert!(
        rendered.contains("@.ax_str_0 = private unnamed_addr constant [6 x i8] c\"hello\\00\"")
    );
    assert!(
        rendered.contains("@.ax_str_1 = private unnamed_addr constant [6 x i8] c\"C:\\5CAX\\00\"")
    );
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr @.ax_str_0)"));
}

#[test]
fn renders_top_level_i32_bool_and_string_consts() {
    let rendered = render(
        "\
const EXIT_OK: i32 = 7;
const ENABLED: bool = true;
const LABEL: string = \"const-ready\";

fn main() -> i32 {
if (ENABLED) {
    println(LABEL);
}
return EXIT_OK;
}
",
    );

    assert!(
        rendered
            .contains("@.ax_str_0 = private unnamed_addr constant [12 x i8] c\"const-ready\\00\"")
    );
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr @.ax_str_0)"));
    assert!(rendered.contains("br i1 1, label"));
    assert!(rendered.contains("ret i32 7"));
}

#[test]
fn renders_string_locals_params_and_return_values() {
    let rendered = render(
        "\
fn choose(left: string, right: string) -> string {
return right;
}

fn main() -> i32 {
let text: string = choose(\"ignored\", \"kept\");
println(text);
return 0;
}
",
    );

    assert!(rendered.contains("define ptr @ax_choose(ptr %arg0, ptr %arg1)"));
    assert!(rendered.contains("store ptr %arg0, ptr %local"));
    assert!(rendered.contains("ret ptr %t"));
    assert!(rendered.contains("= call ptr @ax_choose(ptr @.ax_str_0, ptr @.ax_str_1)"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %t"));
}

#[test]
fn renders_string_len_and_content_comparisons() {
    let rendered = render(
        "\
fn same(left: string, right: string) -> bool {
return left == right;
}

fn main() -> i32 {
let text: string = \"AX\";
if (same(text, \"AX\") && text != \"BY\") {
    return string_len(text) + len(\"tool\");
}
return 1;
}
",
    );

    assert!(rendered.contains("declare i32 @strcmp(ptr, ptr)"));
    assert!(rendered.contains("define private i32 @ax_string_len(ptr %text)"));
    assert!(rendered.contains("call i32 @strcmp(ptr"));
    assert!(rendered.contains("icmp eq i32"));
    assert!(rendered.contains("icmp ne i32"));
    assert!(rendered.contains("call i32 @ax_string_len(ptr"));
}

#[test]
fn renders_argv_builtins_from_native_main_args_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
if (argv_len() == 0) {
    return 0;
}
let first: string = argv_get(0);
println(first);
return string_len(first);
}
",
    );

    assert!(rendered.contains("@.ax_argc = private global i32 0"));
    assert!(rendered.contains("@.ax_argv = private global ptr null"));
    assert!(rendered.contains("define i32 @main(i32 %argc, ptr %argv)"));
    assert!(rendered.contains("load i32, ptr @.ax_argc"));
    assert!(rendered.contains("load ptr, ptr @.ax_argv"));
    assert!(rendered.contains("getelementptr ptr, ptr"));
}

#[test]
fn renders_env_builtins_from_native_environment_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
if (env_has(\"PATH\")) {
    println(env_get(\"PATH\"));
    return 0;
}
return 1;
}
",
    );

    assert!(rendered.contains("declare ptr @getenv(ptr)"));
    assert!(rendered.contains("call ptr @getenv(ptr"));
    assert!(rendered.contains("icmp ne ptr"));
    assert!(rendered.contains("icmp eq ptr"));
    assert!(rendered.contains("call void @ax_runtime_error(ptr @.ax_rt_env_missing)"));
}

#[test]
fn renders_process_builtins_from_native_process_abi_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let cwd: string = process_cwd();
let status: i32 = process_run(\"exit 0\");
let nested: i32 = process_run_in(\".\", \"exit 0\");
let output: string = process_capture(\"echo AX\");
let nested_output: string = process_capture_in(\".\", \"echo OK\");
println(cwd);
println(output + nested_output);
return status + nested;
}
",
    );

    assert!(rendered.contains("declare i32 @system(ptr)"));
    if cfg!(windows) {
        assert!(rendered.contains("declare i32 @SetCurrentDirectoryA(ptr)"));
        assert!(rendered.contains("declare ptr @_popen(ptr, ptr)"));
        assert!(rendered.contains("declare i32 @_pclose(ptr)"));
    } else {
        assert!(rendered.contains("declare i32 @chdir(ptr)"));
        assert!(rendered.contains("declare ptr @popen(ptr, ptr)"));
        assert!(rendered.contains("declare i32 @pclose(ptr)"));
    }
    assert!(rendered.contains("define private i32 @ax_process_run(ptr %command)"));
    assert!(rendered.contains("define private i32 @ax_process_run_in(ptr %dir, ptr %command)"));
    assert!(rendered.contains("define private ptr @ax_process_capture(ptr %command)"));
    assert!(rendered.contains("define private ptr @ax_process_capture_in(ptr %dir, ptr %command)"));
    assert!(rendered.contains("call ptr @ax_process_cwd()"));
    assert!(rendered.contains("call i32 @ax_process_run(ptr"));
    assert!(rendered.contains("call i32 @ax_process_run_in(ptr"));
    assert!(rendered.contains("call ptr @ax_process_capture(ptr"));
    assert!(rendered.contains("call ptr @ax_process_capture_in(ptr"));
}

#[test]
fn renders_fs_read_builtins_from_native_filesystem_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let path: string = \"examples/aot_fs_read.ax\";
if (fs_exists(path) && fs_is_file(path) && !fs_is_dir(path)) {
    let text: string = fs_read_to_string(path);
    println(text);
    return fs_file_size(path);
}
return 0;
}
",
    );

    assert!(rendered.contains("define private i1 @ax_fs_exists(ptr %path)"));
    assert!(rendered.contains("define private i1 @ax_fs_is_file(ptr %path)"));
    assert!(rendered.contains("define private i1 @ax_fs_is_dir(ptr %path)"));
    assert!(rendered.contains("define private i32 @ax_fs_file_size(ptr %path)"));
    assert!(rendered.contains("define private ptr @ax_fs_read_to_string(ptr %path)"));
    assert!(rendered.contains("declare ptr @fopen(ptr, ptr)"));
    assert!(rendered.contains("declare i32 @fgetc(ptr)"));
    assert!(rendered.contains("declare void @rewind(ptr)"));
    if cfg!(windows) {
        assert!(rendered.contains("declare i32 @GetFileAttributesA(ptr)"));
    } else {
        assert!(rendered.contains("declare i32 @access(ptr, i32)"));
        assert!(rendered.contains("declare ptr @opendir(ptr)"));
    }
    assert!(rendered.contains("call i1 @ax_fs_exists(ptr"));
    assert!(rendered.contains("call i1 @ax_fs_is_file(ptr"));
    assert!(rendered.contains("call i1 @ax_fs_is_dir(ptr"));
    assert!(rendered.contains("call ptr @ax_fs_read_to_string(ptr"));
    assert!(rendered.contains("call i32 @ax_fs_file_size(ptr"));
}

#[test]
fn renders_fs_read_dir_as_native_string_slice_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let entries: [string] = fs_read_dir(\"examples\");
println(entries[0]);
return len(entries);
}
",
    );

    assert!(rendered.contains("define private { ptr, i32 } @ax_fs_read_dir(ptr %path)"));
    assert!(rendered.contains("define private void @ax_sort_string_ptrs(ptr %data, i32 %len)"));
    assert!(rendered.contains("call { ptr, i32 } @ax_fs_read_dir(ptr"));
    assert!(rendered.contains("extractvalue { ptr, i32 }"));
    assert!(rendered.contains("getelementptr ptr, ptr"));
}

#[test]
fn renders_fs_write_builtins_from_native_filesystem_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let source: string = \"examples/.aot_fs_write_source.tmp\";
let copied: string = \"examples/.aot_fs_write_copied.tmp\";
let renamed: string = \"examples/.aot_fs_write_renamed.tmp\";
let nested: string = \"build/aot_fs_create_dir_all_test/nested\";
fs_create_dir_all(nested);
if (fs_is_file(source)) {
    fs_remove_file(source);
}
if (fs_is_file(copied)) {
    fs_remove_file(copied);
}
fs_write_string(source, \"hello\");
let bytes: i32 = fs_copy_file(source, copied);
fs_rename(copied, renamed);
fs_remove_file(renamed);
fs_remove_file(source);
fs_remove_dir_all(\"build/aot_fs_create_dir_all_test\");
return 0;
}
",
    );

    assert!(rendered.contains("@.ax_fs_mode_write_binary"));
    assert!(rendered.contains("declare i32 @remove(ptr)"));
    assert!(rendered.contains("declare i32 @rename(ptr, ptr)"));
    assert!(rendered.contains("declare i32 @fputc(i32, ptr)"));
    if cfg!(windows) {
        assert!(rendered.contains("declare i32 @CreateDirectoryA(ptr, ptr)"));
        assert!(rendered.contains("declare i32 @DeleteFileA(ptr)"));
        assert!(rendered.contains("declare i32 @RemoveDirectoryA(ptr)"));
        assert!(rendered.contains("declare ptr @FindFirstFileA(ptr, ptr)"));
        assert!(rendered.contains("declare i32 @FindNextFileA(ptr, ptr)"));
    } else {
        assert!(rendered.contains("declare i32 @mkdir(ptr, i32)"));
        assert!(rendered.contains("declare i32 @nftw(ptr, ptr, i32, i32)"));
    }
    assert!(rendered.contains("define private void @ax_fs_write_string(ptr %path, ptr %text)"));
    assert!(rendered.contains("define private void @ax_fs_remove_file(ptr %path)"));
    assert!(rendered.contains("define private void @ax_fs_rename(ptr %from, ptr %to)"));
    assert!(
        rendered.contains("define private i32 @ax_fs_copy_file(ptr %source, ptr %destination)")
    );
    assert!(rendered.contains("define private void @ax_fs_create_dir_all(ptr %path)"));
    assert!(rendered.contains("define private void @ax_fs_remove_dir_all(ptr %path)"));
    assert!(rendered.contains("call void @ax_fs_write_string(ptr"));
    assert!(rendered.contains("call i32 @ax_fs_copy_file(ptr"));
    assert!(rendered.contains("call void @ax_fs_create_dir_all(ptr"));
    assert!(rendered.contains("call void @ax_fs_remove_dir_all(ptr"));
    assert!(rendered.contains("call void @ax_fs_rename(ptr"));
    assert!(rendered.contains("call void @ax_fs_remove_file(ptr"));
}

#[test]
fn renders_path_runtime_builtins_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let joined: string = path_join(\"build\", \"artifact.txt\");
let parent: string = path_parent(joined);
let resolved: string = path_resolve(parent);
println(path_file_name(joined));
println(path_stem(joined));
println(path_extension(joined));
if (path_is_absolute(resolved)) {
    return string_len(parent);
}
return 0;
}
",
    );

    assert!(rendered.contains("define private ptr @ax_path_join(ptr %base, ptr %name)"));
    assert!(rendered.contains("define private ptr @ax_path_parent(ptr %path)"));
    assert!(rendered.contains("define private ptr @ax_path_resolve(ptr %path)"));
    assert!(rendered.contains("define private ptr @ax_path_file_name(ptr %path)"));
    assert!(rendered.contains("define private ptr @ax_path_stem(ptr %path)"));
    assert!(rendered.contains("define private ptr @ax_path_extension(ptr %path)"));
    assert!(rendered.contains("define private i1 @ax_path_is_absolute(ptr %path)"));
    if cfg!(windows) {
        assert!(rendered.contains("store i8 92, ptr %separator"));
        assert!(rendered.contains("store i8 92, ptr %slash"));
        assert!(!rendered.contains("store i8 47, ptr %separator"));
    } else {
        assert!(rendered.contains("store i8 47, ptr %separator"));
    }
    assert!(rendered.contains("call ptr @ax_path_join(ptr"));
    assert!(rendered.contains("call ptr @ax_path_parent(ptr"));
    assert!(rendered.contains("call ptr @ax_path_resolve(ptr"));
    assert!(rendered.contains("call ptr @ax_path_file_name(ptr"));
    assert!(rendered.contains("call i1 @ax_path_is_absolute(ptr"));
}

#[test]
fn renders_string_concat_and_to_string_values() {
    let rendered = render(
        "\
fn describe(value: i32, enabled: bool, label: string) -> string {
return label + \"=\" + to_string(value) + \", enabled=\" + to_string(enabled);
}

fn main() -> i32 {
let message: string = describe(7, true, \"count\") + \" done\";
println(message);
return string_len(message);
}
",
    );

    assert!(rendered.contains("declare ptr @malloc(i64)"));
    assert!(rendered.contains("declare ptr @memcpy(ptr, ptr, i64)"));
    assert!(rendered.contains("declare i32 @snprintf(ptr, i64, ptr, ...)"));
    assert!(rendered.contains("define private ptr @ax_string_concat(ptr %left, ptr %right)"));
    assert!(rendered.contains("define private ptr @ax_i32_to_string(i32 %value)"));
    assert!(rendered.contains("call ptr @ax_string_concat(ptr"));
    assert!(rendered.contains("call ptr @ax_i32_to_string(i32"));
    assert!(rendered.contains("select i1"));
}

#[test]
fn renders_string_predicate_builtins() {
    let rendered = render(
        "\
fn main() -> i32 {
let text: string = \"AX compiler\";
if (string_contains(text, \"comp\") && string_starts_with(text, \"AX\") && string_ends_with(text, \"iler\")) {
    return 17;
}
return 1;
}
",
    );

    assert!(rendered.contains("declare ptr @strstr(ptr, ptr)"));
    assert!(rendered.contains("declare i32 @strncmp(ptr, ptr, i64)"));
    assert!(rendered.contains("call ptr @strstr(ptr"));
    assert!(rendered.contains("call i32 @strncmp(ptr"));
    assert!(rendered.contains("string_suffix_compare"));
    assert!(rendered.contains("phi i1"));
}

#[test]
fn renders_string_trim_builtin() {
    let rendered = render(
        "\
fn main() -> i32 {
let text: string = string_trim(\"  AX compiler\\n\");
println(text);
return string_len(text);
}
",
    );

    assert!(rendered.contains("define private i1 @ax_is_ascii_trim_space(i8 %byte)"));
    assert!(rendered.contains("define private ptr @ax_string_trim(ptr %text)"));
    assert!(rendered.contains("call ptr @ax_string_trim(ptr"));
    assert!(rendered.contains("call i32 @ax_string_len(ptr"));
}

#[test]
fn renders_string_replace_builtin() {
    let rendered = render(
        "\
fn main() -> i32 {
let text: string = string_replace(\"AX compiler AX\", \"AX\", \"A\");
println(text);
return string_len(text);
}
",
    );

    assert!(
        rendered.contains("define private ptr @ax_string_replace(ptr %text, ptr %from, ptr %to)")
    );
    assert!(
        rendered.contains("define private ptr @ax_string_replace_empty_from(ptr %text, ptr %to)")
    );
    assert!(rendered.contains("call ptr @ax_string_replace(ptr"));
    assert!(rendered.contains("call i32 @ax_string_len(ptr"));
}

#[test]
fn renders_string_split_lines_builtin_as_read_only_string_slice() {
    let rendered = render(
        "\
fn main() -> i32 {
let lines: [string] = string_split_lines(\"alpha\\nbeta\\ngamma\\n\");
println(lines[1]);
return len(lines);
}
",
    );

    assert!(rendered.contains("define private ptr @ax_string_copy_range(ptr %text"));
    assert!(rendered.contains("define private { ptr, i32 } @ax_string_split_lines(ptr %text)"));
    assert!(rendered.contains("call { ptr, i32 } @ax_string_split_lines(ptr"));
    assert!(rendered.contains("extractvalue { ptr, i32 }"));
    assert!(rendered.contains("load ptr, ptr"));
}

#[test]
fn renders_string_list_runtime_helpers_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let mut items: string_list = string_list_new();
items = string_list_push(items, \"alpha\");
items = string_list_push(items, \"beta\");
println(string_list_join(items, \", \"));
println(string_list_get(items, 1));
return len(items);
}
",
    );

    assert!(rendered.contains("define private ptr @ax_string_list_new()"));
    assert!(rendered.contains("define private i32 @ax_string_list_len(ptr %list)"));
    assert!(rendered.contains("define private ptr @ax_string_list_push(ptr %list, ptr %value)"));
    assert!(rendered.contains("define private ptr @ax_string_list_get(ptr %list, i32 %index)"));
    assert!(
        rendered.contains("define private ptr @ax_string_list_join(ptr %list, ptr %separator)")
    );
    assert!(rendered.contains("call ptr @ax_string_list_new()"));
    assert!(rendered.contains("call ptr @ax_string_list_push(ptr"));
    assert!(rendered.contains("call ptr @ax_string_list_join(ptr"));
    assert!(rendered.contains("call ptr @ax_string_list_get(ptr"));
    assert!(rendered.contains("call i32 @ax_string_list_len(ptr"));
}

#[test]
fn renders_for_in_over_runtime_string_slices() {
    let rendered = render(
        "\
fn main() -> i32 {
let lines: [string] = string_split_lines(\"alpha\\nbeta\\ngamma\\n\");
let mut total: i32 = 0;
for (let line: string in lines) {
    total = total + string_len(line);
}
return total;
}
",
    );

    assert!(rendered.contains("call { ptr, i32 } @ax_string_split_lines(ptr"));
    assert!(rendered.contains("extractvalue { ptr, i32 }"));
    assert!(rendered.contains("getelementptr ptr, ptr"));
    assert!(rendered.contains("call i32 @ax_string_len(ptr"));
}

#[test]
fn renders_fixed_array_literals_index_reads_and_len() {
    let rendered = render(
        "\
fn pick(values: [i32; 4], index: i32) -> i32 {
return values[index];
}

fn main() -> i32 {
let values: [i32; 4] = [3, 5, 8, 13];
return values[0] + pick(values, len(values) - 1);
}
",
    );

    assert!(rendered.contains("define i32 @ax_pick([4 x i32] %arg0, i32 %arg1)"));
    assert!(rendered.contains("insertvalue [4 x i32] undef, i32 3, 0"));
    assert!(rendered.contains("store [4 x i32] %t"));
    assert!(rendered.contains("getelementptr [4 x i32], ptr %local"));
    assert!(rendered.contains("icmp slt i32"));
    assert!(rendered.contains("icmp sge i32"));
    assert!(rendered.contains("call i32 @ax_pick([4 x i32]"));
}

#[test]
fn renders_empty_array_literals_from_zero_length_context_v0() {
    let rendered = render(
        "\
fn keep(values: [i32; 0]) -> [i32; 0] {
return values;
}

fn main() -> i32 {
let values: [i32; 0] = [];
let copied: [i32; 0] = keep([]);
println(values);
return len(copied);
}
",
    );

    assert!(rendered.contains("define [0 x i32] @ax_keep([0 x i32] %arg0)"));
    assert!(rendered.contains("store [0 x i32] zeroinitializer"));
    assert!(rendered.contains("call [0 x i32] @ax_keep([0 x i32] zeroinitializer)"));
    assert!(rendered.contains("ret i32 0"));
}

#[test]
fn renders_fixed_array_element_assignments() {
    let rendered = render(
        "\
fn main() -> i32 {
let mut values: [i32; 3] = [1, 2, 3];
values[1] = values[0] + 8;
return values[1];
}
",
    );

    assert!(rendered.contains("insertvalue [3 x i32] undef, i32 1, 0"));
    assert!(rendered.contains("getelementptr [3 x i32], ptr %local"));
    assert!(rendered.contains("store i32 %t"));
    assert!(rendered.contains("icmp slt i32 1, 0"));
    assert!(rendered.contains("icmp sge i32 1, 3"));
}

#[test]
fn renders_fixed_array_formatter_and_direct_print_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let values: [i32; 3] = [1, 2, 3];
println(values);
return string_len(to_string(values));
}
",
    );

    assert!(rendered.contains("extractvalue [3 x i32]"));
    assert!(rendered.contains("call ptr @ax_i32_to_string(i32"));
    assert!(rendered.contains("call ptr @ax_string_concat(ptr"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %"));
}

#[test]
fn renders_slice_formatter_and_direct_print_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let values: [i32; 4] = [1, 2, 3, 4];
let middle: [i32] = values[1:4];
println(middle);
println(values[0:2]);
return string_len(to_string(middle));
}
",
    );

    assert!(rendered.contains("slice_to_string_loop"));
    assert!(rendered.contains("slice_to_string_separator"));
    assert!(rendered.contains("getelementptr i32, ptr"));
    assert!(rendered.contains("call ptr @ax_string_concat(ptr"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %"));
}

#[test]
fn renders_for_in_over_fixed_arrays_with_read_only_slice_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let values: [i32; 3] = [2, 4, 6];
let mut total: i32 = 0;
for (let value: i32 in values) {
    total = total + value;
}
return total;
}
",
    );

    assert!(rendered.contains("insertvalue { ptr, i32 } undef"));
    assert!(rendered.contains("extractvalue { ptr, i32 }"));
    assert!(rendered.contains("getelementptr i32, ptr"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_for_in_over_slice_ranges_with_read_only_slice_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let values: [i32; 5] = [1, 2, 3, 4, 5];
let mut total: i32 = 0;
for (let value: i32 in values[1:4]) {
    total = total + value;
}
return total;
}
",
    );

    assert!(rendered.contains("slice_order_invalid"));
    assert!(rendered.contains("sub i32"));
    assert!(rendered.contains("extractvalue { ptr, i32 }"));
    assert!(rendered.contains("getelementptr i32, ptr"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_slice_range_reads_with_read_only_slice_v0() {
    let rendered = render(
        "\
fn sum_pair(values: [i32]) -> i32 {
return values[0] + values[1];
}

fn main() -> i32 {
let values: [i32; 5] = [1, 2, 3, 4, 5];
let middle: [i32] = values[1:4];
return len(middle) + middle[0] + middle[2] + sum_pair(values[2:4]) + sum_pair(values);
}
",
    );

    assert!(rendered.contains("define i32 @ax_sum_pair({ ptr, i32 } %arg0)"));
    assert!(rendered.contains("icmp sgt i32"));
    assert!(rendered.contains("slice_order_invalid"));
    assert!(rendered.contains("sub i32"));
    assert!(rendered.contains("call i32 @ax_sum_pair({ ptr, i32 }"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_mutable_slice_element_assignment_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let values: [i32; 3] = [1, 2, 3];
let mut view: [i32] = values[0:2];
view[0] = 9;
return values[0] + view[0];
}
",
    );

    assert!(rendered.contains("call ptr @malloc(i64"));
    assert!(rendered.contains("call ptr @memcpy(ptr"));
    assert!(rendered.contains("store i32 9, ptr"));
    assert!(rendered.contains("extractvalue { ptr, i32 }"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_struct_literals_params_returns_and_field_reads() {
    let rendered = render(
        "\
struct Point {
x: i32,
y: i32,
}

fn shift(point: Point, delta: i32) -> Point {
return Point { x: point.x + delta, y: point.y + delta };
}

fn main() -> i32 {
let point: Point = shift(Point { y: 5, x: 2 }, 3);
return point.x + point.y;
}
",
    );

    assert!(rendered.contains("%ax_struct_Point = type { i32, i32 }"));
    assert!(
        rendered.contains("define %ax_struct_Point @ax_shift(%ax_struct_Point %arg0, i32 %arg1)")
    );
    assert!(rendered.contains("insertvalue %ax_struct_Point undef, i32 2, 0"));
    assert!(rendered.contains("insertvalue %ax_struct_Point %t"));
    assert!(rendered.contains("getelementptr %ax_struct_Point, ptr %local"));
    assert!(rendered.contains("ret %ax_struct_Point"));
    assert!(rendered.contains("= call %ax_struct_Point @ax_shift(%ax_struct_Point %t"));
}

#[test]
fn renders_struct_field_assignments() {
    let rendered = render(
        "\
struct Point {
x: i32,
y: i32,
}

fn main() -> i32 {
let mut point: Point = Point { x: 2, y: 5 };
point.y = point.x + 10;
return point.y;
}
",
    );

    assert!(rendered.contains("%ax_struct_Point = type { i32, i32 }"));
    assert!(rendered.contains("getelementptr %ax_struct_Point, ptr %local"));
    assert!(rendered.contains("store i32 %t"));
    assert!(rendered.contains("ret i32 %t"));
}

#[test]
fn renders_struct_formatter_and_direct_print_v0() {
    let rendered = render(
        "\
struct Summary {
count: i32,
ready: bool,
label: string,
}

fn main() -> i32 {
let summary: Summary = Summary { ready: true, label: \"ok\", count: 3 };
println(summary);
return string_len(to_string(summary));
}
",
    );

    assert!(rendered.contains("%ax_struct_Summary = type { i32, i1, ptr }"));
    assert!(rendered.contains("c\"Summary { \\00\""));
    assert!(rendered.contains("c\"count: \\00\""));
    assert!(rendered.contains("c\"label: \\00\""));
    assert!(rendered.contains("c\"ready: \\00\""));
    assert!(rendered.contains("extractvalue %ax_struct_Summary"));
    assert!(rendered.contains("call ptr @ax_string_concat(ptr"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %"));
}

#[test]
fn renders_unit_enum_values_params_returns_and_comparisons() {
    let rendered = render(
        "\
enum Flag {
Off,
On,
}

fn choose(flag: Flag) -> Flag {
return flag;
}

fn score(flag: Flag) -> i32 {
if (flag == Flag.On) {
    return 9;
}
return 2;
}

fn main() -> i32 {
let flag: Flag = choose(Flag.On);
return score(flag);
}
",
    );

    assert!(rendered.contains("define i32 @ax_choose(i32 %arg0)"));
    assert!(rendered.contains("ret i32 %t"));
    assert!(rendered.contains("call i32 @ax_choose(i32 1)"));
    assert!(rendered.contains("store i32 %t"));
    assert!(rendered.contains("icmp eq i32 %t"));
    assert!(rendered.contains("call i32 @ax_score(i32 %t"));
}

#[test]
fn renders_unit_enum_match_statement_tests() {
    let rendered = render(
        "\
enum Flag {
Off,
On,
}

fn score(flag: Flag) -> i32 {
match (flag) {
    Flag.On => {
        return 9;
    }
    Flag.Off => {
        return 2;
    }
}
}

fn main() -> i32 {
return score(Flag.On);
}
",
    );

    assert!(rendered.contains("define i32 @ax_score(i32 %arg0)"));
    assert!(rendered.contains("icmp eq i32 %t"));
    assert!(rendered.contains("br i1 %t"));
    assert!(rendered.contains("ret i32 9"));
    assert!(rendered.contains("ret i32 2"));
}

#[test]
fn renders_payload_enum_constructors_payload_reads_and_match_tests() {
    let rendered = render(
        "\
enum Maybe {
None,
Some(i32),
}

fn score(value: Maybe) -> i32 {
match (value) {
    Maybe.Some(number) => {
        return number;
    }
    Maybe.None => {
        return 0;
    }
}
}

fn main() -> i32 {
let value: Maybe = Maybe.Some(7);
return score(value);
}
",
    );

    assert!(rendered.contains("%ax_enum_Maybe = type { i32, ptr }"));
    assert!(rendered.contains("define i32 @ax_score(%ax_enum_Maybe %arg0)"));
    assert!(rendered.contains("call ptr @malloc(i64 4)"));
    assert!(rendered.contains("store i32 7, ptr %t"));
    assert!(rendered.contains("insertvalue %ax_enum_Maybe undef, i32 1, 0"));
    assert!(rendered.contains("extractvalue %ax_enum_Maybe %t"));
    assert!(rendered.contains("load i32, ptr %t"));
    assert!(rendered.contains("call i32 @ax_score(%ax_enum_Maybe %t"));
}

#[test]
fn renders_match_expression_with_payload_binding_and_block_arm() {
    let rendered = render(
        "\
enum Maybe {
None,
Some(i32),
}

fn score(value: Maybe) -> i32 {
return match (value) {
    Maybe.Some(number) => {
        let bonus: i32 = 1;
        number + bonus
    },
    Maybe.None => 0,
};
}

fn main() -> i32 {
return score(Maybe.Some(7));
}
",
    );

    assert!(rendered.contains("match_arm_"));
    assert!(rendered.contains("match_done_"));
    assert!(rendered.contains("alloca i32"));
    assert!(rendered.contains("extractvalue %ax_enum_Maybe"));
    assert!(rendered.contains("store i32"));
    assert!(rendered.contains("load i32"));
}

#[test]
fn renders_i32_range_match_patterns() {
    let rendered = render(
        "\
fn classify(status: i32) -> i32 {
return match (status) {
    200..=299 => 2,
    400..=499 => 4,
    _ => 0,
};
}

fn main() -> i32 {
return classify(204);
}
",
    );

    assert!(rendered.contains("icmp sge i32"));
    assert!(rendered.contains("icmp sle i32"));
    assert!(rendered.contains("and i1"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_or_match_patterns() {
    let rendered = render(
        "\
enum Mode {
Check,
Run,
Build,
}

fn score(mode: Mode) -> i32 {
return match (mode) {
    Mode.Check | Mode.Run => 1,
    Mode.Build => 2,
};
}

fn main() -> i32 {
return score(Mode.Check);
}
",
    );

    assert!(rendered.contains("icmp eq i32"));
    assert!(rendered.contains("or i1"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_guarded_match_arms() {
    let rendered = render(
        "\
enum Token {
Number(i32),
End,
}

fn score(token: Token) -> i32 {
return match (token) {
    Token.Number(value) if value > 9 => value,
    Token.Number(_) => 1,
    Token.End => 0,
};
}

fn main() -> i32 {
return score(Token.Number(12));
}
",
    );

    assert!(rendered.contains("match_guard_"));
    assert!(rendered.contains("icmp sgt i32"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_payload_enum_equality() {
    let rendered = render(
        "\
enum Status {
Code(i32),
Label(string),
Done,
}

fn score() -> i32 {
let code: Status = Status.Code(7);
let same_code: Status = Status.Code(7);
let other_code: Status = Status.Code(8);
let label: Status = Status.Label(\"ok\");
let same_label: Status = Status.Label(\"ok\");
let other_label: Status = Status.Label(\"bad\");
let mut total: i32 = 0;
if (code == same_code) {
    total = total + 1;
}
if (code != other_code) {
    total = total + 2;
}
if (label == same_label) {
    total = total + 4;
}
if (label != other_label) {
    total = total + 8;
}
if (Status.Done == Status.Done) {
    total = total + 16;
}
if (code != Status.Done) {
    total = total + 32;
}
return total;
}

fn main() -> i32 {
return score();
}
",
    );

    assert!(rendered.contains("enum_eq_tags_match"));
    assert!(rendered.contains("call i32 @strcmp(ptr"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_fixed_array_equality_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let left: [i32; 3] = [1, 2, 3];
let same: [i32; 3] = [1, 2, 3];
let other: [i32; 3] = [1, 2, 4];
let names: [string; 2] = [\"ax\", \"lang\"];
let same_names: [string; 2] = [\"ax\", \"lang\"];
let other_names: [string; 2] = [\"ax\", \"tool\"];
let mut total: i32 = 0;
if (left == same) {
    total = total + 1;
}
if (left != other) {
    total = total + 2;
}
if (names == same_names) {
    total = total + 4;
}
if (names != other_names) {
    total = total + 8;
}
return total;
}
",
    );

    assert!(rendered.contains("extractvalue [3 x i32]"));
    assert!(rendered.contains("extractvalue [2 x ptr]"));
    assert!(rendered.contains("call i32 @strcmp(ptr"));
    assert!(rendered.contains("and i1"));
    assert!(rendered.contains("or i1"));
}

#[test]
fn renders_struct_and_enum_struct_payload_equality_v0() {
    let rendered = render(
        "\
struct Summary {
count: i32,
label: string,
}

enum Packet {
Summary(Summary),
Empty,
}

fn main() -> i32 {
let left: Summary = Summary { count: 3, label: \"ok\" };
let same: Summary = Summary { count: 3, label: \"ok\" };
let other: Summary = Summary { count: 4, label: \"ok\" };
let packet: Packet = Packet.Summary(left);
let same_packet: Packet = Packet.Summary(same);
let other_packet: Packet = Packet.Summary(other);
let mut total: i32 = 0;
if (left == same) {
    total = total + 1;
}
if (left != other) {
    total = total + 2;
}
if (packet == same_packet) {
    total = total + 4;
}
if (packet != other_packet) {
    total = total + 8;
}
return total;
}
",
    );

    assert!(rendered.contains("extractvalue %ax_struct_Summary"));
    assert!(rendered.contains("call i32 @strcmp(ptr"));
    assert!(rendered.contains("enum_eq_tags_match"));
    assert!(rendered.contains("and i1"));
    assert!(rendered.contains("or i1"));
}

#[test]
fn renders_non_generic_impl_methods_v0() {
    let rendered = render(
        "\
struct Point {
x: i32,
y: i32,
}

impl Point {
fn make(x: i32, y: i32) -> Point {
    return Point { x: x, y: y };
}

fn sum(self: Point) -> i32 {
    return self.x + self.y;
}

fn offset_sum(self: Point, delta: i32) -> i32 {
    return self.sum() + delta;
}
}

fn main() -> i32 {
let point: Point = Point.make(4, 5);
println(point.sum());
return point.offset_sum(3);
}
",
    );

    assert!(rendered.contains("define %ax_struct_Point @ax_Point_make(i32 %arg0, i32 %arg1)"));
    assert!(rendered.contains("define i32 @ax_Point_sum(%ax_struct_Point %arg0)"));
    assert!(
        rendered.contains("define i32 @ax_Point_offset_sum(%ax_struct_Point %arg0, i32 %arg1)")
    );
    assert!(rendered.contains("call %ax_struct_Point @ax_Point_make(i32 4, i32 5)"));
    assert!(rendered.contains("call i32 @ax_Point_sum(%ax_struct_Point"));
    assert!(rendered.contains("call i32 @ax_Point_offset_sum(%ax_struct_Point"));
}

#[test]
fn renders_generic_struct_instances_v0() {
    let rendered = render(
        "\
struct Box<T> {
value: T,
}

fn main() -> i32 {
let mut number: Box<i32> = Box { value: 7 };
number.value = number.value + 1;
return number.value;
}
",
    );

    assert!(rendered.contains("%ax_struct_Box_i32_ = type { i32 }"));
    assert!(rendered.contains("alloca %ax_struct_Box_i32_"));
    assert!(rendered.contains("getelementptr %ax_struct_Box_i32_"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_generic_impl_methods_v0() {
    let rendered = render(
        "\
struct Box<T> {
value: T,
}

impl<T> Box<T> {
fn get(self: Box<T>) -> T {
    return self.value;
}
}

fn main() -> i32 {
let number: Box<i32> = Box { value: 9 };
println(number.get());
return number.get();
}
",
    );

    assert!(rendered.contains("%ax_struct_Box_i32_ = type { i32 }"));
    assert!(rendered.contains("define i32 @ax_Box_get_i32_(%ax_struct_Box_i32_ %arg0)"));
    assert!(rendered.contains("call i32 @ax_Box_get_i32_(%ax_struct_Box_i32_"));
}

#[test]
fn renders_generic_method_type_params_v0() {
    let rendered = render(
        "\
struct Pair<T, U> {
left: T,
right: U,
}

impl<T> Pair<T, i32> {
fn replace_right<U>(self: Pair<T, i32>, right: U) -> Pair<T, U> {
    return Pair { left: self.left, right: right };
}
}

fn main() -> i32 {
let pair: Pair<string, i32> = Pair { left: \"ax\", right: 1 };
let changed: Pair<string, string> = pair.replace_right(\"ok\");
println(changed.left + \":\" + changed.right);
return 0;
}
",
    );

    assert!(rendered.contains("%ax_struct_Pair_string__i32_ = type { ptr, i32 }"));
    assert!(rendered.contains("%ax_struct_Pair_string__string_ = type { ptr, ptr }"));
    assert!(
        rendered.contains(
            "define %ax_struct_Pair_string__string_ @ax_Pair_replace_right_string__string_"
        )
    );
    assert!(
        rendered.contains(
            "call %ax_struct_Pair_string__string_ @ax_Pair_replace_right_string__string_"
        )
    );
}

#[test]
fn renders_generic_functions_v0() {
    let rendered = render(
        "\
struct Box<T> {
value: T,
}

fn identity<T>(value: T) -> T {
return value;
}

fn unwrap_box<T>(box: Box<T>) -> T {
return box.value;
}

fn main() -> i32 {
let number_box: Box<i32> = Box { value: identity(9) };
let text_box: Box<string> = Box { value: identity(\"ax\") };
println(unwrap_box(text_box));
println(unwrap_box(number_box));
return unwrap_box(number_box);
}
",
    );

    assert!(rendered.contains("define i32 @ax_identity_i32_(i32 %arg0)"));
    assert!(rendered.contains("define ptr @ax_identity_string_(ptr %arg0)"));
    assert!(rendered.contains("define i32 @ax_unwrap_box_i32_(%ax_struct_Box_i32_ %arg0)"));
    assert!(rendered.contains("define ptr @ax_unwrap_box_string_(%ax_struct_Box_string_ %arg0)"));
    assert!(rendered.contains("call i32 @ax_identity_i32_(i32 9)"));
    assert!(rendered.contains("call ptr @ax_identity_string_(ptr"));
    assert!(rendered.contains("call i32 @ax_unwrap_box_i32_(%ax_struct_Box_i32_"));
    assert!(rendered.contains("call ptr @ax_unwrap_box_string_(%ax_struct_Box_string_"));
}

#[test]
fn renders_generic_type_aliases_v0() {
    let rendered = render(
        "\
struct Box<T> {
value: T,
}

type Boxed<T> = Box<T>;
type TextBox = Boxed<string>;

fn unwrap_text(boxed: TextBox) -> string {
return boxed.value;
}

fn main() -> i32 {
let boxed: TextBox = Box { value: \"ax\" };
println(unwrap_text(boxed));
return 0;
}
",
    );

    assert!(rendered.contains("%ax_struct_Box_string_ = type { ptr }"));
    assert!(rendered.contains("define ptr @ax_unwrap_text(%ax_struct_Box_string_ %arg0)"));
    assert!(rendered.contains("call ptr @ax_unwrap_text(%ax_struct_Box_string_"));
}

#[test]
fn renders_trait_bound_generic_functions_v0() {
    let rendered = render(
        "\
trait Label {
fn label(self: Self) -> string;
}

struct Command {
name: string,
}

impl Label for Command {
fn label(self: Command) -> string {
    return self.name;
}
}

fn render<T: Label>(value: T) -> string {
return value.label();
}

fn main() -> i32 {
let command: Command = Command { name: \"build\" };
println(render(command));
return 5;
}
",
    );

    assert!(rendered.contains("define ptr @ax_render_Command_(%ax_struct_Command %arg0)"));
    assert!(rendered.contains("call ptr @ax_Command_label(%ax_struct_Command"));
    assert!(rendered.contains("call ptr @ax_render_Command_(%ax_struct_Command"));
}

#[test]
fn renders_generic_trait_impl_bound_dispatch_v0() {
    let rendered = render(
        "\
trait Label {
fn label(self: Self) -> string;
}

struct Box<T> {
value: T,
}

impl<T> Label for Box<T> {
fn label(self: Box<T>) -> string {
    return to_string(self.value);
}
}

fn render<T: Label>(value: T) -> string {
return value.label();
}

fn main() -> i32 {
let number: Box<i32> = Box { value: 42 };
println(render(number));
return 0;
}
",
    );

    assert!(rendered.contains("define ptr @ax_Box_label_i32_(%ax_struct_Box_i32_ %arg0)"));
    assert!(rendered.contains("define ptr @ax_render_Box_i32__(%ax_struct_Box_i32_ %arg0)"));
    assert!(rendered.contains("call ptr @ax_Box_label_i32_(%ax_struct_Box_i32_"));
    assert!(rendered.contains("call ptr @ax_render_Box_i32__(%ax_struct_Box_i32_"));
}

#[test]
fn renders_non_generic_trait_impl_methods_v0() {
    let rendered = render(
        "\
trait Label {
fn label(self: Self) -> string;
}

struct Command {
name: string,
}

impl Label for Command {
fn label(self: Command) -> string {
    return self.name;
}
}

fn main() -> i32 {
let command: Command = Command { name: \"build\" };
println(command.label());
return string_len(command.label());
}
",
    );

    assert!(rendered.contains("define ptr @ax_Command_label(%ax_struct_Command %arg0)"));
    assert!(rendered.contains("call ptr @ax_Command_label(%ax_struct_Command"));
    assert!(rendered.contains("call i32 @ax_string_len(ptr"));
}

#[test]
fn renders_enum_array_payload_equality_v0() {
    let rendered = render(
        "\
enum Packet {
Values([i32; 3]),
Empty,
}

fn main() -> i32 {
let left: Packet = Packet.Values([1, 2, 3]);
let same: Packet = Packet.Values([1, 2, 3]);
let other: Packet = Packet.Values([1, 2, 4]);
let mut total: i32 = 0;
if (left == same) {
    total = total + 1;
}
if (left != other) {
    total = total + 2;
}
if (left != Packet.Empty) {
    total = total + 4;
}
return total;
}
",
    );

    assert!(rendered.contains("enum_eq_tags_match"));
    assert!(rendered.contains("extractvalue [3 x i32]"));
    assert!(rendered.contains("and i1"));
    assert!(rendered.contains("or i1"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_slice_equality_v0() {
    let rendered = render(
        "\
fn main() -> i32 {
let left: [i32; 4] = [1, 2, 3, 4];
let same: [i32; 4] = [0, 2, 3, 9];
let other: [i32; 4] = [0, 2, 4, 9];
let a: [i32] = left[1:3];
let b: [i32] = same[1:3];
let c: [i32] = other[1:3];
let mut total: i32 = 0;
if (a == b) {
    total = total + 1;
}
if (a != c) {
    total = total + 2;
}
return total;
}
",
    );

    assert!(rendered.contains("slice_eq_loop"));
    assert!(rendered.contains("extractvalue { ptr, i32 }"));
    assert!(rendered.contains("getelementptr i32"));
    assert!(rendered.contains("and i1"));
    assert!(rendered.contains("or i1"));
}

#[test]
fn renders_enum_slice_payload_equality_v0() {
    let rendered = render(
        "\
enum Packet {
Window([i32]),
Empty,
}

fn main() -> i32 {
let left_values: [i32; 4] = [1, 2, 3, 4];
let same_values: [i32; 4] = [0, 2, 3, 9];
let other_values: [i32; 4] = [0, 2, 4, 9];
let left: Packet = Packet.Window(left_values[1:3]);
let same: Packet = Packet.Window(same_values[1:3]);
let other: Packet = Packet.Window(other_values[1:3]);
let mut total: i32 = 0;
if (left == same) {
    total = total + 1;
}
if (left != other) {
    total = total + 2;
}
if (left != Packet.Empty) {
    total = total + 4;
}
return total;
}
",
    );

    assert!(rendered.contains("enum_eq_tags_match"));
    assert!(rendered.contains("slice_eq_loop"));
    assert!(rendered.contains("extractvalue { ptr, i32 }"));
    assert!(rendered.contains("getelementptr i32"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_enum_f32_payload_v0() {
    let rendered = render(
        "\
const SCALE: f32 = 2.0;

enum Metric {
Value(f32),
Series([f32; 3]),
Window([f32]),
Empty,
}

fn classify(metric: Metric) -> i32 {
let mut result: i32 = 0;
match (metric) {
    Metric.Value(number) => {
        if ((number * SCALE) == 5.0) {
            result = 5;
        }
    }
    Metric.Series(values) => {
        result = len(values);
    }
    Metric.Window(values) => {
        result = len(values);
    }
    Metric.Empty => {
        result = 0;
    }
}
return result;
}

fn main() -> i32 {
let value: Metric = Metric.Value(2.5);
let same_value: Metric = Metric.Value(2.5);
let other_value: Metric = Metric.Value(3.5);
let series: Metric = Metric.Series([1.5, 2.5, 3.5]);
let same_series: Metric = Metric.Series([1.5, 2.5, 3.5]);
let other_series: Metric = Metric.Series([1.5, 2.5, 4.5]);
let base: [f32; 3] = [1.5, 2.5, 3.5];
let other_base: [f32; 3] = [1.5, 2.5, 4.5];
let window: Metric = Metric.Window(base[1:3]);
let same_window: Metric = Metric.Window(base[1:3]);
let other_window: Metric = Metric.Window(other_base[1:3]);
let mut total: i32 = 0;
if (value == same_value) {
    total = total + 1;
}
if (value != other_value) {
    total = total + 2;
}
if (series == same_series) {
    total = total + 4;
}
if (series != other_series) {
    total = total + 8;
}
if (window == same_window) {
    total = total + 16;
}
if (window != other_window) {
    total = total + 32;
}
if (classify(value) == 5) {
    total = total + 64;
}
if (classify(series) == 3) {
    total = total + 128;
}
if (classify(window) == 2) {
    total = total + 256;
}
println(value);
println(series);
println(window);
return total;
}
",
    );

    assert!(rendered.contains("%ax_enum_Metric = type { i32, ptr }"));
    assert!(rendered.contains("call ptr @malloc(i64 4)"));
    assert!(rendered.contains("extractvalue [3 x float]"));
    assert!(rendered.contains("getelementptr float"));
    assert!(rendered.contains("fcmp oeq float"));
    assert!(rendered.contains("slice_eq_loop"));
    assert!(rendered.contains("call ptr @ax_f32_to_string(float"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %t"));
}

#[test]
fn renders_enum_to_string_formatter_v0() {
    let rendered = render(
        "\
enum Status {
Code(i32),
Flag(bool),
Label(string),
Done,
}

fn main() -> i32 {
let code: Status = Status.Code(7);
let flag: Status = Status.Flag(true);
let label: Status = Status.Label(\"ok\");
let done: Status = Status.Done;
println(to_string(code));
println(to_string(flag));
println(to_string(label));
println(to_string(done));
return string_len(to_string(code)) + string_len(to_string(flag)) + string_len(to_string(label)) + string_len(to_string(done));
}
",
    );

    assert!(rendered.contains("c\"Status.Code\\00\""));
    assert!(rendered.contains("c\"Status.Flag\\00\""));
    assert!(rendered.contains("c\"Status.Label\\00\""));
    assert!(rendered.contains("c\"Status.Done\\00\""));
    assert!(rendered.contains("enum_to_string_done"));
    assert!(rendered.contains("call ptr @ax_i32_to_string(i32"));
    assert!(rendered.contains("select i1 %t"));
    assert!(rendered.contains("call ptr @ax_string_concat(ptr"));
    assert!(rendered.contains("alloca ptr"));
    assert!(rendered.contains("load ptr, ptr"));
}

#[test]
fn renders_direct_enum_println_with_formatter_v0() {
    let rendered = render(
        "\
enum Status {
Code(i32),
Flag(bool),
Label(string),
Done,
}

fn main() -> i32 {
let code: Status = Status.Code(7);
let flag: Status = Status.Flag(true);
let label: Status = Status.Label(\"ok\");
let done: Status = Status.Done;
println(code);
println(flag);
println(label);
println(done);
return 58;
}
",
    );

    assert!(rendered.contains("enum_to_string_done"));
    assert!(rendered.contains("call ptr @ax_i32_to_string(i32"));
    assert!(rendered.contains("select i1 %t"));
    assert!(rendered.contains("call ptr @ax_string_concat(ptr"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %t"));
}

#[test]
fn renders_enum_array_payload_formatter_v0() {
    let rendered = render(
        "\
enum Packet {
Values([i32; 3]),
Empty,
}

fn main() -> i32 {
let packet: Packet = Packet.Values([1, 2, 3]);
println(packet);
return string_len(to_string(packet));
}
",
    );

    assert!(rendered.contains("c\"Packet.Values\\00\""));
    assert!(rendered.contains("extractvalue [3 x i32]"));
    assert!(rendered.contains("call ptr @ax_i32_to_string(i32"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %t"));
}

#[test]
fn renders_enum_struct_and_slice_payload_formatter_v0() {
    let rendered = render(
        "\
struct Summary {
count: i32,
label: string,
}

enum Packet {
Summary(Summary),
Lines([string]),
Empty,
}

fn main() -> i32 {
let summary: Packet = Packet.Summary(Summary { count: 3, label: \"ok\" });
let lines: Packet = Packet.Lines(string_split_lines(\"alpha\\nbeta\\n\"));
println(summary);
println(lines);
return string_len(to_string(summary)) + string_len(to_string(lines));
}
",
    );

    assert!(rendered.contains("c\"Packet.Summary\\00\""));
    assert!(rendered.contains("c\"Packet.Lines\\00\""));
    assert!(rendered.contains("call ptr @malloc(i64 16)"));
    assert!(rendered.contains("call { ptr, i32 } @ax_string_split_lines(ptr"));
    assert!(rendered.contains("extractvalue %ax_struct_Summary"));
    assert!(rendered.contains("slice_to_string_loop"));
}

#[test]
fn renders_struct_match_patterns() {
    let rendered = render(
        "\
struct Point {
x: i32,
y: i32,
}

fn score(point: Point) -> i32 {
return match (point) {
    Point { x, y } => x + y,
};
}

fn main() -> i32 {
return score(Point { x: 20, y: 22 });
}
",
    );

    assert!(rendered.contains("extractvalue %ax_struct_Point"));
    assert!(rendered.contains("store i32"));
    assert!(rendered.contains("ret i32 %"));
}

#[test]
fn renders_concrete_generic_result_and_option_instances() {
    let rendered = render(
        "\
enum Option<T> {
None,
Some(T),
}

enum Result<T, E> {
Ok(T),
Err(E),
}

fn option_or(value: Option<i32>, fallback: i32) -> i32 {
return match (value) { Option.Some(found) => found, Option.None => fallback };
}

fn value_or_zero(result: Result<i32, string>) -> i32 {
return match (result) { Result.Ok(value) => value, Result.Err(_) => 0 };
}

fn main() -> i32 {
let present: Option<i32> = Option.Some(5);
let ok: Result<i32, string> = Result.Ok(7);
return option_or(present, 0) + value_or_zero(ok);
}
",
    );

    assert!(rendered.contains("%ax_enum_Option_i32_ = type { i32, ptr }"));
    assert!(rendered.contains("%ax_enum_Result_i32__string_ = type { i32, ptr }"));
    assert!(rendered.contains("define i32 @ax_option_or(%ax_enum_Option_i32_ %arg0"));
    assert!(rendered.contains("define i32 @ax_value_or_zero(%ax_enum_Result_i32__string_ %arg0"));
    assert!(rendered.contains("call ptr @malloc(i64 4)"));
    assert!(rendered.contains("extractvalue %ax_enum_Result_i32__string_"));
}

#[test]
fn renders_concrete_generic_enum_formatter_and_direct_print_v0() {
    let rendered = render(
        "\
enum Option<T> {
None,
Some(T),
}

enum Result<T, E> {
Ok(T),
Err(E),
}

fn main() -> i32 {
let present: Option<i32> = Option.Some(5);
let missing: Option<i32> = Option.None;
let ok: Result<i32, string> = Result.Ok(7);
let err: Result<i32, string> = Result.Err(\"bad\");
println(to_string(present));
println(missing);
println(to_string(ok));
println(err);
return string_len(to_string(present)) + string_len(to_string(missing)) + string_len(to_string(ok)) + string_len(to_string(err));
}
",
    );

    assert!(rendered.contains("%ax_enum_Option_i32_ = type { i32, ptr }"));
    assert!(rendered.contains("%ax_enum_Result_i32__string_ = type { i32, ptr }"));
    assert!(rendered.contains("c\"Option.Some\\00\""));
    assert!(rendered.contains("c\"Option.None\\00\""));
    assert!(rendered.contains("c\"Result.Ok\\00\""));
    assert!(rendered.contains("c\"Result.Err\\00\""));
    assert!(rendered.contains("call ptr @ax_i32_to_string(i32"));
    assert!(rendered.contains("call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr %t"));
}

#[test]
fn renders_generic_static_result_constructors_from_expected_type_v0() {
    let rendered = render(
        "\
enum Result<T, E> {
Ok(T),
Err(E),
}

impl<T, E> Result<T, E> {
fn ok(value: T) -> Result<T, E> {
    return Result.Ok(value);
}

fn err(error: E) -> Result<T, E> {
    return Result.Err(error);
}
}

fn parse_demo(text: string) -> Result<i32, string> {
if (text == \"ok\") {
    return Result.ok(7);
}
return Result.err(\"bad:\" + text);
}

fn main() -> i32 {
let ok: Result<i32, string> = Result.ok(5);
let err: Result<i32, string> = Result.err(\"missing\");
println(match (ok) { Result.Ok(value) => to_string(value), Result.Err(message) => message });
println(match (err) { Result.Ok(value) => to_string(value), Result.Err(message) => message });
println(match (parse_demo(\"no\")) { Result.Ok(value) => to_string(value), Result.Err(message) => message });
return 0;
}
",
    );

    assert!(rendered.contains("%ax_enum_Result_i32__string_ = type { i32, ptr }"));
    assert!(
        rendered
            .contains("define %ax_enum_Result_i32__string_ @ax_Result_ok_i32__string_(i32 %arg0)")
    );
    assert!(
        rendered
            .contains("define %ax_enum_Result_i32__string_ @ax_Result_err_i32__string_(ptr %arg0)")
    );
    assert!(rendered.contains("call %ax_enum_Result_i32__string_ @ax_Result_ok_i32__string_"));
    assert!(rendered.contains("call %ax_enum_Result_i32__string_ @ax_Result_err_i32__string_"));
}

#[test]
fn renders_generic_result_err_specialization_by_return_type_v0() {
    let rendered = render(
        "\
enum Result<T, E> {
Ok(T),
Err(E),
}

impl<T, E> Result<T, E> {
fn ok(value: T) -> Result<T, E> {
    return Result.Ok(value);
}

fn err(error: E) -> Result<T, E> {
    return Result.Err(error);
}
}

fn parse_text(flag: bool) -> Result<string, string> {
if (flag) {
    return Result.ok(\"text\");
}
return Result.err(\"text missing\");
}

fn parse_count(flag: bool) -> Result<i32, string> {
if (flag) {
    return Result.ok(7);
}
return Result.err(\"count missing\");
}

fn main() -> i32 {
let text: Result<string, string> = parse_text(false);
let count: Result<i32, string> = parse_count(false);
println(match (text) { Result.Ok(value) => value, Result.Err(message) => message });
println(match (count) { Result.Ok(value) => to_string(value), Result.Err(message) => message });
return 0;
}
",
    );

    assert!(rendered.contains(
        "define %ax_enum_Result_string__string_ @ax_Result_err_string__string_(ptr %arg0)"
    ));
    assert!(
        rendered
            .contains("define %ax_enum_Result_i32__string_ @ax_Result_err_i32__string_(ptr %arg0)")
    );
    assert!(
        rendered.contains("call %ax_enum_Result_string__string_ @ax_Result_err_string__string_")
    );
    assert!(rendered.contains("call %ax_enum_Result_i32__string_ @ax_Result_err_i32__string_"));
}

#[test]
fn renders_result_try_early_return_with_error_rewrap() {
    let rendered = render(
        "\
enum Result<T, E> {
Ok(T),
Err(E),
}

fn parse(text: string) -> Result<i32, string> {
if (text == \"ok\") {
    return Result.Ok(7);
}
return Result.Err(\"bad\");
}

fn render_score(text: string) -> Result<string, string> {
let score: i32 = parse(text)?;
return Result.Ok(\"score=\" + to_string(score));
}

fn main() -> i32 {
let result: Result<string, string> = render_score(\"ok\");
return match (result) { Result.Ok(text) => string_len(text), Result.Err(message) => string_len(message) };
}
",
    );

    assert!(rendered.contains("%ax_enum_Result_i32__string_ = type { i32, ptr }"));
    assert!(rendered.contains("%ax_enum_Result_string__string_ = type { i32, ptr }"));
    assert!(rendered.contains("define %ax_enum_Result_string__string_ @ax_render_score"));
    assert!(rendered.contains("call %ax_enum_Result_i32__string_ @ax_parse"));
    assert!(rendered.contains("try_err_"));
    assert!(rendered.contains("try_ok_"));
    assert!(rendered.contains("ret %ax_enum_Result_string__string_"));
}

#[test]
fn renders_generic_result_adapter_inferred_from_nested_call_argument_v0() {
    let rendered = render(
        "\
enum Result<T, E> {
Ok(T),
Err(E),
}

enum ConfigError {
Io(string),
}

fn from_io<T>(value: Result<T, string>) -> Result<T, ConfigError> {
return match (value) {
    Result.Ok(found) => Result.Ok(found),
    Result.Err(message) => Result.Err(ConfigError.Io(message)),
};
}

fn read_text() -> Result<string, string> {
return Result.Ok(\"cfg\");
}

fn validate() -> Result<string, ConfigError> {
let contents: string = from_io(read_text())?;
return Result.Ok(contents);
}

fn main() -> i32 {
let result: Result<string, ConfigError> = validate();
return match (result) {
    Result.Ok(text) => string_len(text),
    Result.Err(_) => 1,
};
}
",
    );

    assert!(rendered.contains("%ax_enum_Result_string__string_ = type { i32, ptr }"));
    assert!(rendered.contains("%ax_enum_Result_string__ConfigError_ = type { i32, ptr }"));
    assert!(rendered.contains("define %ax_enum_Result_string__ConfigError_ @ax_from_io_string"));
    assert!(rendered.contains("call %ax_enum_Result_string__string_ @ax_read_text()"));
    assert!(rendered.contains("call %ax_enum_Result_string__ConfigError_ @ax_from_io_string"));
}

#[test]
fn renders_result_statement_match_payload_binding_with_concrete_type_v0() {
    let rendered = render(
        "\
enum Result<T, E> {
Ok(T),
Err(E),
}

fn read_text() -> Result<string, string> {
return Result.Ok(\"cfg\");
}

fn main() -> i32 {
let result: Result<string, string> = read_text();
match (result) {
    Result.Ok(report_path) => {
        println(\"ok=\" + report_path);
        return 0;
    }
    Result.Err(message) => {
        println(\"err=\" + message);
        return 1;
    }
}
return 2;
}
",
    );

    assert!(rendered.contains("%ax_enum_Result_string__string_ = type { i32, ptr }"));
    assert!(rendered.contains("call %ax_enum_Result_string__string_ @ax_read_text()"));
    assert!(rendered.contains("call ptr @ax_string_concat(ptr"));
    assert!(rendered.contains("ret i32 0"));
}
