# AX Package Registry v0

> This is the staged package plan for AX. It intentionally starts with a curated index and download/install workflow before public upload.

## Goal

Package Registry v0 should let AX projects depend on reusable AX source packages without pretending that AX already has a mature ecosystem.

The first public shape should support:

- Pure AX source packages.
- Manual/curated package index review.
- Deterministic dependency metadata.
- Checksum-backed lock entries.
- Local cache installation.
- No install scripts.
- No native extensions.
- No public upload server in v0.

## Non-Goals For v0

- Public package upload.
- Account/login/token system.
- Native binary packages.
- Host extension ABI.
- Build scripts or install scripts.
- Dynamic linking.
- SemVer solver beyond exact versions or simple compatible ranges.
- Automatic trust of arbitrary package code.

## Current Baseline

AX already has local path package v0:

```toml
[dependencies]
math_rules = { path = "packages/math_rules" }
```

It also has `AX.lock` v0 for local path package graph reproducibility. Registry v0 should extend this model rather than replace it.

## Registry Shape

Start with a repository-owned curated index:

```text
registry/
  index.json
  packages/
    text_tools.json
    config_rules.json
```

Example package metadata:

```json
{
  "schema_version": 1,
  "name": "text_tools",
  "owner": "ax-core",
  "version": "0.1.0",
  "license": "Apache-2.0",
  "description": "Text helpers for AX projects",
  "source": {
    "kind": "git",
    "url": "https://example.com/ax-core/text_tools.git",
    "rev": "0123456789abcdef"
  },
  "checksum": "sha256:...",
  "modules": [
    "text_tools.normalize",
    "text_tools.stats"
  ]
}
```

## `AX.toml` Dependency Form

Registry dependency:

```toml
[dependencies]
text_tools = { registry = "ax", version = "0.1.0" }
```

Local path dependency remains valid:

```toml
[dependencies]
text_tools = { path = "packages/text_tools" }
```

Do not add install scripts to package manifests in v0.

## Lockfile Extension

Registry packages should extend `AX.lock` with immutable source and checksum data:

```json
{
  "alias": "text_tools",
  "kind": "registry",
  "package": "text_tools",
  "version": "0.1.0",
  "source": {
    "registry": "ax",
    "url": "https://example.com/ax-core/text_tools.git",
    "rev": "0123456789abcdef"
  },
  "checksum": "sha256:...",
  "modules": [
    "text_tools.normalize",
    "text_tools.stats"
  ]
}
```

## CLI Roadmap

Phase 0.1, design only:

```powershell
axc lock <project>
axc lock <project> --check
```

Phase 0.2, download/install preview:

```powershell
axc pkg search text
axc pkg add text_tools
axc pkg install
axc pkg tree
axc pkg check
```

Phase 0.3+, publish beta:

```powershell
axc login
axc publish
axc yank
```

Upload/publish must wait until package ownership, token security, checksums, yank policy, and moderation are defined.

## Cache Layout

Recommended local cache:

```text
%USERPROFILE%\.ax\packages\
  text_tools\
    0.1.0\
      AX.toml
      src\
```

Repository-local vendor/cache remains a future option for offline builds.

## Security Rules

Registry v0 should be intentionally boring:

- Packages are source-only.
- No package install scripts.
- No native binary execution during install.
- Checksums are required for registry packages.
- Package index changes are reviewed by maintainers.
- Lockfile drift is reported as a package graph readiness issue.

## AOT Contract

Registry package AOT should reuse the same rules as local path package AOT:

- Resolve package source into module paths.
- Include package sources in build artifacts.
- Record package graph readiness in manifest/context.
- Monomorphize only concrete instances used by reachable code.
- Emit backend blockers for unsupported native ABI features instead of blaming user source.

## Community Workflow For v0

Until upload exists, third-party packages should enter through proposal/review:

1. Author publishes package source in a public repository.
2. Author opens a package-index proposal.
3. Maintainers review metadata, license, module names, examples, and validation.
4. The package metadata is merged into the curated index.
5. Users can install it through `axc pkg install` once the CLI exists.

This keeps AX's early package ecosystem useful without opening the security and moderation surface of public upload too early.

