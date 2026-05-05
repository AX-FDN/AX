# AX Module System v1 Skeleton

This document defines the module and import behavior that must be frozen for
AX 1.0.

## Module Declarations

Each source file may declare a module path. The module path determines how other
files import its public items in the current module/import model.

The v1 spec must define:

- valid module path syntax
- relationship between file path and module path
- duplicate module diagnostics
- module visibility defaults

## Imports

Current imports are explicit module imports. The v1 spec must define:

- importing project modules
- importing `std.*`
- importing local path packages
- importing registry packages
- diagnostics for missing modules
- diagnostics for duplicate or ambiguous imports

Wildcard imports, aliases, and visibility modifiers are not required for 1.0
unless explicitly promoted.

## Project Sources

`AX.toml` controls project entry and support sources. The v1 spec must define:

- default entry
- explicit entry
- source roots
- recursive source directories
- conflict rules between source roots and package aliases

## Package Module Roots

Package aliases become module roots. For example:

```toml
[dependencies]
text_tools = { registry = "ax", version = "0.1.0" }
```

This makes package modules importable through the `text_tools.*` root after the
package is installed and locked.
