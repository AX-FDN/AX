# AX Host Runtime Packages

This document defines the current prerequisite layer for packages that need host
IO, especially HTTP, raw TCP networking, and future database clients.

## Current Rule

AX packages should prefer AX source wrappers over hidden native behavior:

```text
AX package source
  -> std.* wrapper
  -> explicit host builtin
  -> interpreter support in axc run
  -> AOT readiness feature + blocker until native ABI exists
```

That rule keeps package behavior understandable to users and to AI agents. If a
package runs in the interpreter but is not yet native-lowerable, `axc build`
must report a runtime/native ABI blocker instead of pretending that user source
is wrong.

In current minimal module mode, projects that import `std.http` or `std.net`
must include the standard library source root in `AX.toml`:

```toml
[package]
sources = ["../../std"]
```

## HTTP v0

AX now has an interpreter-backed HTTP v0 surface:

```ax
import std.http;

fn main() -> i32 {
    let response: std.http.HttpResponse = std.http.get("http://127.0.0.1:8080/");
    if (response.ok) {
        println(response.body);
        return 0;
    }
    println(response.body);
    return 1;
}
```

Public AX surface:

- `std.http.HttpResponse { status: i32, ok: bool, headers: string, body: string }`
- `std.http.get(url: string) -> HttpResponse`
- `std.http.try_get(url: string) -> std.result.Result<HttpResponse, string>`
- `std.http.status_text(response: HttpResponse) -> string`

Host builtin:

- `http_get(url: string) -> std.http.HttpResponse`

Interpreter behavior:

- Supports plain `http://` GET.
- Supports explicit ports such as `http://127.0.0.1:8080/`.
- Uses a bounded response size for deterministic package tests.
- Returns `HttpResponse { status: -1, ok: false, headers: "", body: error }` on
  host errors instead of raising a runtime diagnostic.

Current boundary:

- HTTPS/TLS is not supported yet.
- Redirect handling, custom headers, POST/PUT, streaming, cookies, and binary
  bodies are not part of HTTP v0.
- AOT marks this as `host_http` and reports `AOT0301` until a native HTTP/TLS
  runtime ABI exists.

## TCP Networking v0

AX now has a minimal raw TCP exchange surface for package experiments:

```ax
import std.net;

fn main() -> i32 {
    let response: std.net.TcpResponse = std.net.tcp_exchange(
        "127.0.0.1",
        6379,
        "PING"
    );
    if (response.ok) {
        println(response.data);
        return 0;
    }
    println(response.error);
    return 1;
}
```

Public AX surface:

- `std.net.TcpResponse { ok: bool, data: string, error: string }`
- `std.net.tcp_exchange(host: string, port: i32, request: string) -> TcpResponse`
- `std.net.try_tcp_exchange(host: string, port: i32, request: string) -> std.result.Result<string, string>`
- `std.net.status_text(response: TcpResponse) -> string`

Host builtin:

- `net_tcp_exchange(host: string, port: i32, request: string) -> std.net.TcpResponse`

Interpreter behavior:

- Opens one TCP connection.
- Writes the request string.
- Shuts down the write side.
- Reads until EOF or timeout.
- Returns string data using UTF-8 lossy display semantics.

Current boundary:

- This is not a long-lived socket API.
- It is not async and does not expose connection handles.
- It does not support TLS.
- It is intended for first package-level protocol experiments and local smoke
  tests.
- AOT marks this as `host_net` and reports `AOT0301` until a native socket ABI
  exists.

## Database Packages

Database packages should not start with a fake `db_query` builtin. The correct
staging path is:

1. Use pure AX parsing and connection-string helpers where possible.
2. Use `std.net.tcp_exchange` for simple local text protocols and early protocol
   experiments.
3. Add byte buffers before serious binary protocols.
4. Add TLS before real PostgreSQL/MySQL-style remote connections.
5. Add native/runtime ABI support before claiming AOT parity for database IO.

`axc build` already reserves `host_db` as an AOT readiness feature for future
`db_*`, `std.db.*`, or `std.database.*` calls. Until the runtime ABI is real,
database host IO should be reported as a runtime/native ABI blocker, not as a
user source error.

## Validation

The fast local validation entry is:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-host-network-runtime.ps1
```

This smoke:

- Starts a local TCP server.
- Runs an AX project using `std.http` and `std.net`.
- Verifies `axc check`.
- Verifies `axc run`.
- Runs `axc build --emit ir --no-link`.
- Confirms `build-manifest.json` reports `host_http`, `host_net`, and `AOT0301`.

## Package Author Guidance

For `AX-PKG` packages:

- HTTP helper packages can now wrap `std.http`.
- Network helper packages can now wrap `std.net`.
- Database packages should begin with pure AX types, DSN parsing, result/error
  modeling, and local protocol experiments.
- Do not claim native/AOT support for HTTP, TCP, TLS, or database IO until
  `host_http`, `host_net`, or `host_db` has a native runtime ABI.
- Classify host IO packages as `host_boundary_preview` and database/security
  shape packages as `future_native_preview` until their native/runtime ABI exists.
  See [`package-maturity.md`](./package-maturity.md).
