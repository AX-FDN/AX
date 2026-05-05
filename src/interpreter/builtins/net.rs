use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

const TCP_TIMEOUT: Duration = Duration::from_secs(5);
const TCP_MAX_BYTES: usize = 1024 * 1024;

impl<'a> Interpreter<'a> {
    pub(super) fn call_net_tcp_exchange_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 3 {
            return Err(self.runtime_error(
                "R0142",
                format!(
                    "function `net_tcp_exchange` expected 3 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let host = arguments
            .next()
            .expect("net_tcp_exchange host argument should exist");
        let port = arguments
            .next()
            .expect("net_tcp_exchange port argument should exist");
        let request = arguments
            .next()
            .expect("net_tcp_exchange request argument should exist");

        match (host, port, request) {
            (Value::String(host), Value::I32(port), Value::String(request)) => {
                Ok(tcp_exchange_response(&host, port, &request))
            }
            (host, port, request) => Err(self
                .runtime_error(
                    "R0143",
                    format!(
                        "function `net_tcp_exchange` requires `(string, i32, string)`, got `{}`, `{}`, and `{}`",
                        host.display(),
                        port.display(),
                        request.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `net_tcp_exchange` like `net_tcp_exchange(\"127.0.0.1\", 8080, request)`",
                )),
        }
    }
}

fn tcp_exchange_response(host: &str, port: i32, request: &str) -> Value {
    match try_tcp_exchange(host, port, request) {
        Ok(data) => response_value(true, data, String::new()),
        Err(error) => response_value(false, String::new(), error),
    }
}

fn try_tcp_exchange(host: &str, port: i32, request: &str) -> Result<String, String> {
    if host.trim().is_empty() {
        return Err("TCP host must not be empty".to_string());
    }
    if !(1..=65535).contains(&port) {
        return Err(format!("TCP port must be between 1 and 65535, got {port}"));
    }

    let mut stream = TcpStream::connect((host, port as u16))
        .map_err(|error| format!("failed to connect to `{host}:{port}`: {error}"))?;
    stream
        .set_read_timeout(Some(TCP_TIMEOUT))
        .map_err(|error| format!("failed to set TCP read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(TCP_TIMEOUT))
        .map_err(|error| format!("failed to set TCP write timeout: {error}"))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write TCP request: {error}"))?;
    let _ = stream.shutdown(Shutdown::Write);

    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(|error| format!("failed to read TCP response: {error}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > TCP_MAX_BYTES {
            return Err(format!(
                "TCP response exceeded {} bytes; AX host TCP v0 caps responses for deterministic package tests",
                TCP_MAX_BYTES
            ));
        }
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn response_value(ok: bool, data: String, error: String) -> Value {
    let mut fields = BTreeMap::new();
    fields.insert("ok".to_string(), Value::Bool(ok));
    fields.insert("data".to_string(), Value::String(data));
    fields.insert("error".to_string(), Value::String(error));
    Value::Struct {
        name: "std.net.TcpResponse".to_string(),
        fields,
    }
}
