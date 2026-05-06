use std::fmt::Write;

use super::super::abi;

pub(super) fn write_host_handle_abi(module: &mut String) {
    writeln!(module, "; host handle ABI: {}", abi::HOST_HANDLE_ABI_NAME)
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "; host handle type: {} runtime-owned opaque handle",
        abi::HOST_HANDLE_LLVM_TYPE
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "; host handle layout: header={} kind_off={} native_off={}",
        abi::HOST_HANDLE_HEADER_BYTES,
        abi::HOST_HANDLE_KIND_OFFSET,
        abi::HOST_HANDLE_NATIVE_OFFSET
    )
    .expect("writing to string cannot fail");
    writeln!(module, "; host error ABI: {}", abi::HOST_ERROR_ABI_NAME)
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "; host error type: {} where code={} means ok and message is null",
        abi::HOST_ERROR_LLVM_TYPE,
        abi::HOST_ERROR_OK_CODE
    )
    .expect("writing to string cannot fail");

    write_host_error_helpers(module);
    write_host_handle_helpers(module);
    write_release_helper(module, abi::TCP_SOCKET_RELEASE_HELPER, "socket");
    write_release_helper(module, abi::TLS_STREAM_RELEASE_HELPER, "stream");
    write_release_helper(module, abi::HTTP_CLIENT_RELEASE_HELPER, "client");
    write_release_helper(module, abi::HTTP_SERVER_RELEASE_HELPER, "server");
    write_release_helper(module, abi::DB_CONNECTION_RELEASE_HELPER, "connection");
    write_release_helper(module, abi::ASYNC_TASK_RELEASE_HELPER, "task");
    write_release_helper(module, abi::TIMER_RELEASE_HELPER, "timer");
}

fn write_host_error_helpers(module: &mut String) {
    writeln!(
        module,
        "define private {} @{}() {{",
        abi::HOST_ERROR_LLVM_TYPE,
        abi::HOST_ERROR_OK_HELPER
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %with_code = insertvalue {} undef, i32 {}, 0",
        abi::HOST_ERROR_LLVM_TYPE,
        abi::HOST_ERROR_OK_CODE
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %with_message = insertvalue {} %with_code, ptr null, 1",
        abi::HOST_ERROR_LLVM_TYPE
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  ret {} %with_message", abi::HOST_ERROR_LLVM_TYPE)
        .expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");

    writeln!(
        module,
        "define private {} @{}(i32 %code, ptr %message) {{",
        abi::HOST_ERROR_LLVM_TYPE,
        abi::HOST_ERROR_NEW_HELPER
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %with_code = insertvalue {} undef, i32 %code, 0",
        abi::HOST_ERROR_LLVM_TYPE
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %with_message = insertvalue {} %with_code, ptr %message, 1",
        abi::HOST_ERROR_LLVM_TYPE
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  ret {} %with_message", abi::HOST_ERROR_LLVM_TYPE)
        .expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");

    writeln!(
        module,
        "define private i1 @{}({} %error) {{",
        abi::HOST_ERROR_IS_OK_HELPER,
        abi::HOST_ERROR_LLVM_TYPE
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %code = extractvalue {} %error, 0",
        abi::HOST_ERROR_LLVM_TYPE
    )
    .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %ok = icmp eq i32 %code, {}",
        abi::HOST_ERROR_OK_CODE
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  ret i1 %ok").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");

    writeln!(
        module,
        "define private ptr @{}({} %error) {{",
        abi::HOST_ERROR_MESSAGE_HELPER,
        abi::HOST_ERROR_LLVM_TYPE
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %message = extractvalue {} %error, 1",
        abi::HOST_ERROR_LLVM_TYPE
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  %missing = icmp eq ptr %message, null")
        .expect("writing to string cannot fail");
    writeln!(
        module,
        "  %selected = select i1 %missing, ptr {}, ptr %message",
        abi::HOST_ERROR_DEFAULT_MESSAGE
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  ret ptr %selected").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}

fn write_host_handle_helpers(module: &mut String) {
    writeln!(
        module,
        "define private ptr @{}(i32 %kind, ptr %native) {{",
        abi::HOST_HANDLE_NEW_HELPER
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %handle = call ptr @malloc(i64 {})",
        abi::HOST_HANDLE_HEADER_BYTES
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  store i32 %kind, ptr %handle").expect("writing to string cannot fail");
    writeln!(
        module,
        "  %native_slot = getelementptr i8, ptr %handle, i64 {}",
        abi::HOST_HANDLE_NATIVE_OFFSET
    )
    .expect("writing to string cannot fail");
    writeln!(module, "  store ptr %native, ptr %native_slot")
        .expect("writing to string cannot fail");
    writeln!(module, "  ret ptr %handle").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");

    writeln!(
        module,
        "define private i32 @{}(ptr %handle) {{",
        abi::HOST_HANDLE_KIND_HELPER
    )
    .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  %kind = load i32, ptr %handle").expect("writing to string cannot fail");
    writeln!(module, "  ret i32 %kind").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}

fn write_release_helper(module: &mut String, helper: &str, parameter: &str) {
    writeln!(module, "define private void @{helper}(ptr %{parameter}) {{")
        .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  call void @free(ptr %{parameter})").expect("writing to string cannot fail");
    writeln!(module, "  ret void").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}
