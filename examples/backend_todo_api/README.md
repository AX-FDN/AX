# AX 1.0 Backend Todo API Target

This directory is the target shape for the 1.0 backend systems demo. It is not a
working application yet. It exists so HTTP, TLS, JSON, database, async, package,
and AOT work converge on one visible goal.

## Target Behavior

The final demo should prove:

- HTTP API server starts from AX source.
- JSON request and response bodies use `std.json`.
- PostgreSQL persistence uses `std.db`.
- outbound TLS client request uses `std.tls` and `std.http`.
- async IO is used for request handling and database/network calls.
- `axc check` succeeds.
- `axc run` works as semantic reference.
- `axc build --emit exe` produces a native executable on Windows and Linux.
- native executable output and behavior match the interpreter reference.
- structured diagnostics explain configuration, package, runtime, and AOT
  failures.

## Planned Routes

```text
GET  /health
GET  /todos
POST /todos
PATCH /todos/:id
DELETE /todos/:id
```

## Non-Goals For The First Demo

- authentication
- multi-tenant deployment
- migrations beyond a minimal schema
- production observability stack
- public package upload

## Why This Demo Exists

AX 1.0 should not be evaluated only by isolated syntax examples. A backend
systems language needs one end-to-end application that exercises std, packages,
runtime ABI, AOT, diagnostics, and AI repair guidance together.
