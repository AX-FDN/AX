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

    write_noop_release_helper(module, abi::TCP_SOCKET_RELEASE_HELPER, "socket");
    write_noop_release_helper(module, abi::TLS_STREAM_RELEASE_HELPER, "stream");
    write_noop_release_helper(module, abi::HTTP_CLIENT_RELEASE_HELPER, "client");
    write_noop_release_helper(module, abi::HTTP_SERVER_RELEASE_HELPER, "server");
    write_noop_release_helper(module, abi::DB_CONNECTION_RELEASE_HELPER, "connection");
    write_noop_release_helper(module, abi::ASYNC_TASK_RELEASE_HELPER, "task");
    write_noop_release_helper(module, abi::TIMER_RELEASE_HELPER, "timer");
}

fn write_noop_release_helper(module: &mut String, helper: &str, parameter: &str) {
    writeln!(module, "define private void @{helper}(ptr %{parameter}) {{")
        .expect("writing to string cannot fail");
    writeln!(module, "entry:").expect("writing to string cannot fail");
    writeln!(module, "  ret void").expect("writing to string cannot fail");
    writeln!(module, "}}\n").expect("writing to string cannot fail");
}
