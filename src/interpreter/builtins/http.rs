use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const HTTP_MAX_BYTES: usize = 1024 * 1024;

struct HttpUrl {
    host: String,
    port: u16,
    path: String,
}

impl<'a> Interpreter<'a> {
    pub(super) fn call_http_get_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0140",
                format!(
                    "function `http_get` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let url = arguments
            .into_iter()
            .next()
            .expect("http_get argument should exist");
        let Value::String(url) = url else {
            return Err(self
                .runtime_error(
                    "R0141",
                    format!(
                        "function `http_get` requires a `string` url, got `{}`",
                        url.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `http_get` with an HTTP URL like `http_get(\"http://127.0.0.1:8080/\")`",
                ));
        };

        Ok(http_get_response(&url))
    }
}

fn http_get_response(url: &str) -> Value {
    match try_http_get(url) {
        Ok((status, headers, body)) => {
            response_value(status, (200..300).contains(&status), headers, body)
        }
        Err(error) => response_value(-1, false, String::new(), error),
    }
}

fn try_http_get(url: &str) -> Result<(i32, String, String), String> {
    let parsed = parse_http_url(url)?;
    let mut stream = TcpStream::connect((parsed.host.as_str(), parsed.port))
        .map_err(|error| format!("failed to connect to `{}`: {error}", parsed.host))?;
    stream
        .set_read_timeout(Some(HTTP_TIMEOUT))
        .map_err(|error| format!("failed to set HTTP read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(HTTP_TIMEOUT))
        .map_err(|error| format!("failed to set HTTP write timeout: {error}"))?;

    let request = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: axc/0.2-package-preview\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        parsed.path, parsed.host
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write HTTP request: {error}"))?;

    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(|error| format!("failed to read HTTP response: {error}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > HTTP_MAX_BYTES {
            return Err(format!(
                "HTTP response exceeded {} bytes; AX host HTTP v0 caps response bodies for deterministic package tests",
                HTTP_MAX_BYTES
            ));
        }
    }

    parse_http_response(&bytes)
}

fn parse_http_url(url: &str) -> Result<HttpUrl, String> {
    let Some(rest) = url.strip_prefix("http://") else {
        return Err(
            "host HTTP v0 supports only `http://` URLs; HTTPS needs a TLS/native runtime ABI"
                .to_string(),
        );
    };
    let without_fragment = rest.split_once('#').map(|(left, _)| left).unwrap_or(rest);
    let (authority, path) = without_fragment
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{path}")))
        .unwrap_or((without_fragment, "/".to_string()));
    if authority.trim().is_empty() {
        return Err("HTTP URL is missing a host".to_string());
    }
    if authority.contains('@') {
        return Err("HTTP URL userinfo is not supported in host HTTP v0".to_string());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) => {
            let port = port
                .parse::<u16>()
                .map_err(|error| format!("HTTP URL port is invalid: {error}"))?;
            (host, port)
        }
        _ => (authority, 80),
    };
    if host.trim().is_empty() {
        return Err("HTTP URL is missing a host".to_string());
    }
    Ok(HttpUrl {
        host: host.to_string(),
        port,
        path,
    })
}

fn parse_http_response(bytes: &[u8]) -> Result<(i32, String, String), String> {
    let text = String::from_utf8_lossy(bytes).into_owned();
    let (header_text, body) = text
        .split_once("\r\n\r\n")
        .or_else(|| text.split_once("\n\n"))
        .map(|(headers, body)| (headers.to_string(), body.to_string()))
        .unwrap_or_else(|| (String::new(), text));
    let status_line = header_text.lines().next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "HTTP response did not include a status code".to_string())?
        .parse::<i32>()
        .map_err(|error| format!("HTTP response status code is invalid: {error}"))?;
    Ok((status, header_text, body))
}

fn response_value(status: i32, ok: bool, headers: String, body: String) -> Value {
    let mut fields = BTreeMap::new();
    fields.insert("status".to_string(), Value::I32(status));
    fields.insert("ok".to_string(), Value::Bool(ok));
    fields.insert("headers".to_string(), Value::String(headers));
    fields.insert("body".to_string(), Value::String(body));
    Value::Struct {
        name: "std.http.HttpResponse".to_string(),
        fields,
    }
}
