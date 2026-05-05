# AX Error Model v1 Skeleton

AX 1.0 must keep error layering as a first-class language-toolchain contract.

## Required Diagnostic Fields

Structured diagnostics and build/readiness blockers should converge on:

```json
{
  "layer": "...",
  "code": "...",
  "severity": "...",
  "summary": "...",
  "repair_goal": "...",
  "ai_action": "...",
  "safe_to_edit": false,
  "validation": []
}
```

Existing outputs do not need to break immediately, but new work should move
toward this shape.

## Required Layers

- `source_input`
- `lexer`
- `parser`
- `semantic`
- `hir_lowering`
- `mir_lowering`
- `interpreter_runtime`
- `aot_readiness`
- `monomorphization`
- `runtime_abi`
- `llvm_lowering`
- `toolchain_link`
- `package_registry`
- `package_cache`
- `internal_compiler_error`

## AI Action Categories

The AI-facing action should distinguish:

- edit source
- explain unsupported backend/runtime feature
- verify lockfile
- install package or toolchain
- inspect toolchain failure
- retry validation
- report compiler bug

## Source Edit Safety

If user code is valid under `axc check` and `axc run`, but native build is
blocked by runtime ABI, package maturity, toolchain, or lowering gaps, the
diagnostic must mark source edits as unsafe by default.

This rule protects users from AI rewrites that remove valid business logic to
make an incomplete backend pass.
