# AX Backend Profile v1 Draft

Backend Profile v1 is the 1.0 native-build contract. A program inside this
profile should pass:

```text
axc check
axc run
axc build --emit exe
native executable parity with axc run
```

## Platform Target

- Windows: first-tier development workflow.
- Linux: first-tier server workflow.
- macOS: not part of the 1.0 commitment unless explicitly promoted later.

## Language Surface

Backend Profile v1 should include:

- functions, typed parameters, typed returns
- `let`, `let mut`, assignment, `return`
- `i32`, `bool`, `f32`, `string`, `bytes`
- arrays, slices, structs, unit enums, payload enums
- `if`, `while`, `for`, `for in`, `break`, `continue`
- match statements and expressions for current stable pattern subset
- `Option` and `Result` conventions
- `?` for supported `Result<T,E>` instances
- module/import and project sources
- local path and registry package source dependencies

Preview or later:

- full trait dispatch
- full generic impl/method ABI
- async trait, async closure, macro systems
- FFI/native extension ABI

## Standard Library Surface

Backend Profile v1 should include:

- pure std: result, option, text, json, url, config, validation, collection
- host std: fs, env, process, path, time, log
- backend std: bytes, net, tls, http, db, async, crypto boundary

`std.crypto` must not be claimed as production security until it has a separate
reviewed contract. Non-cryptographic checksums stay separate from cryptographic
hashing and signing.

## Package Surface

Package support in profile:

- source-only registry packages
- checksum-backed install
- stable lockfile entries
- package maturity in build manifests
- AOT readiness blockers for host-boundary and future-native packages

Profile does not include public upload, install scripts, or native binary
packages.

## AOT Requirements

For profile features:

- unsupported behavior must be expressed as a structured blocker while it is
  being implemented.
- once promoted into profile, the feature must be covered by run-vs-exe parity.
- build failures must distinguish frontend, runtime ABI, lowering, toolchain,
  package, and internal compiler layers.

The profile is not complete until Windows and Linux CI both validate the native
path.
