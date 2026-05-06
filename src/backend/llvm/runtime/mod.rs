mod bytes;
mod core;
mod fs;
mod host;
mod list;
mod path;
mod process;
mod string;

pub(super) fn write_builtin_globals(module: &mut String) {
    core::write_builtin_globals(module);
}

pub(super) fn write_external_declarations(module: &mut String) {
    core::write_external_declarations(module);
}

pub(super) fn write_runtime_error_helper(module: &mut String) {
    core::write_runtime_error_helper(module);
}

pub(super) fn write_string_helpers(module: &mut String) {
    string::write_string_helpers(module);
    list::write_string_list_helpers(module);
}

pub(super) fn write_bytes_helpers(module: &mut String) {
    bytes::write_bytes_helpers(module);
}

pub(super) fn write_host_helpers(module: &mut String) {
    host::write_host_handle_abi(module);
    fs::write_fs_helpers(module);
    path::write_path_helpers(module);
    process::write_process_helpers(module);
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
        assert!(module.contains("@.ax_rt_env_missing"));
        assert!(module.contains("declare i32 @printf(ptr, ...)"));
        assert!(module.contains("declare ptr @getenv(ptr)"));
        assert!(module.contains("declare ptr @strstr(ptr, ptr)"));
        assert!(module.contains("define private i32 @ax_string_len(ptr %text)"));
        assert!(module.contains("define private i1 @ax_is_ascii_trim_space(i8 %byte)"));
        assert!(module.contains("define private ptr @ax_string_concat(ptr %left, ptr %right)"));
        assert!(
            module.contains("define private ptr @ax_string_replace(ptr %text, ptr %from, ptr %to)")
        );
        assert!(
            module.contains("define private ptr @ax_string_replace_empty_from(ptr %text, ptr %to)")
        );
        assert!(module.contains("define private ptr @ax_string_copy_range(ptr %text"));
        assert!(module.contains("define private { ptr, i32 } @ax_string_split_lines(ptr %text)"));
        assert!(module.contains("define private ptr @ax_string_trim(ptr %text)"));
        assert!(module.contains("define private ptr @ax_string_list_new()"));
        assert!(module.contains("define private i32 @ax_string_list_len(ptr %list)"));
        assert!(module.contains("define private ptr @ax_string_list_push(ptr %list, ptr %value)"));
        assert!(module.contains("define private ptr @ax_string_list_get(ptr %list, i32 %index)"));
        assert!(
            module.contains("define private ptr @ax_string_list_join(ptr %list, ptr %separator)")
        );
        assert!(module.contains("define private void @ax_string_list_release(ptr %list)"));
        assert!(module.contains("define private ptr @ax_i32_to_string(i32 %value)"));
        assert!(module.contains("define private ptr @ax_f32_to_string(float %value)"));
        assert!(module.contains("append_decimal:"));
    }

    #[test]
    fn runtime_prelude_exposes_bytes_abi() {
        let mut module = String::new();
        write_builtin_globals(&mut module);
        write_external_declarations(&mut module);
        write_bytes_helpers(&mut module);

        assert!(module.contains("bytes ABI: ax.bytes.opaque_buffer_v0"));
        assert!(module.contains("bytes layout: header=8 len_off=0 cap_off=4 data_off=8"));
        assert!(module.contains("define private void @ax_bytes_release(ptr %bytes)"));
        assert!(module.contains("define private ptr @ax_bytes_empty()"));
        assert!(module.contains("define private ptr @ax_bytes_from_string(ptr %text)"));
        assert!(module.contains("define private i32 @ax_bytes_len(ptr %bytes)"));
        assert!(module.contains("define private i32 @ax_bytes_get(ptr %bytes, i32 %index)"));
        assert!(module.contains("define private ptr @ax_bytes_push(ptr %bytes, i32 %value)"));
        assert!(module.contains("define private ptr @ax_bytes_to_string_lossy(ptr %bytes)"));
        assert!(module.contains("define private ptr @ax_bytes_to_hex(ptr %bytes)"));
        assert!(module.contains("define private i8 @ax_bytes_hex_digit(i32 %value)"));
    }

    #[test]
    fn runtime_prelude_exposes_host_handle_abi() {
        let mut module = String::new();
        write_host_helpers(&mut module);

        assert!(module.contains("host handle ABI: ax.host.handle_v0"));
        assert!(module.contains("host handle type: ptr runtime-owned opaque handle"));
        assert!(module.contains("host error ABI: ax.host.error_v0"));
        assert!(
            module.contains("host error type: { i32, ptr } where code=0 means ok and message is null")
        );
        assert!(module.contains("define private { i32, ptr } @ax_host_error_ok()"));
        assert!(module.contains(
            "define private { i32, ptr } @ax_host_error_new(i32 %code, ptr %message)"
        ));
        assert!(module.contains("define private void @ax_tcp_socket_release(ptr %socket)"));
        assert!(module.contains("define private void @ax_tls_stream_release(ptr %stream)"));
        assert!(module.contains("define private void @ax_http_client_release(ptr %client)"));
        assert!(module.contains("define private void @ax_http_server_release(ptr %server)"));
        assert!(module.contains("define private void @ax_db_connection_release(ptr %connection)"));
        assert!(module.contains("define private void @ax_async_task_release(ptr %task)"));
        assert!(module.contains("define private void @ax_timer_release(ptr %timer)"));
    }
}
