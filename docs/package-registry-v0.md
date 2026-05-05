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

Host IO packages are allowed only when their runtime boundary is explicit. HTTP
and raw TCP package experiments should wrap `std.http` and `std.net`; database
packages should begin with pure AX types and protocol helpers until TCP/TLS/byte
buffer/runtime ABI support is mature enough. See
[`host-runtime-packages.md`](./host-runtime-packages.md).

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

Start with a repository-owned curated index inside the AX repository during 0.2:

```text
registry/
  index.json
  packages/
    api_tools.json
    auth_tools.json
    bytes_tools.json
    cache_tools.json
    collection_tools.json
    config_rules.json
    database_tools.json
    encoding_tools.json
    hash_tools.json
    http_tools.json
    json_tools.json
    jwt_tools.json
    log_tools.json
    markdown_tools.json
    math_rules.json
    migration_tools.json
    net_tools.json
    number_tools.json
    pagination_tools.json
    queue_tools.json
    report_tools.json
    result_tools.json
    retry_tools.json
    schema_tools.json
    url_tools.json
    text_tools.json
    validation_tools.json
```

This in-repo registry is the 0.2 prototype and test fixture. Package source can
live in the foundation package repository:

```text
https://github.com/AX-FDN/AX-PKG.git
git@github.com:AX-FDN/AX-PKG.git
```

The registry metadata uses the HTTPS URL by default so read-only install works
without SSH keys. Maintainers can use the SSH URL when pushing package source.
`AX-PKG` is expected to work as a package monorepo, so each metadata entry may pin
a `source.path` such as `packages/text_tools`.

Package checksums are computed over package directory files in stable relative
path order. `.git` and `target` directories are ignored so source checksums do
not depend on repository internals or local build artifacts.

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
    "url": "https://github.com/AX-FDN/AX-PKG.git",
    "rev": "0123456789abcdef",
    "path": "packages/text_tools"
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
    "url": "https://github.com/AX-FDN/AX-PKG.git",
    "rev": "0123456789abcdef",
    "path": "packages/text_tools"
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

Phase 0.2, current preview:

```powershell
axc pkg search text
axc pkg info text_tools
axc pkg tree
axc pkg check
axc pkg add text_tools <project>
axc pkg add text_tools <project> --dry-run
axc pkg install <project> --dry-run
axc pkg hash <package-dir>
```

`search`, `info`, `tree`, `check`, `add`, `install --dry-run`, `install`, and
`hash` are the first implemented slice. `pkg add <package> <project>` resolves
the latest curated package version and writes a registry dependency into
`AX.toml`; `--dry-run` keeps the old preview-only behavior. `AX.toml` can parse
registry dependency intent, but unresolved registry packages are reported as
preview package-layer blockers instead of being loaded as source. `pkg install
<project>` can now write registry-only `AX.lock` schema v2 preview entries and
has the first git/cache installer path: when metadata pins a real `rev` and real
checksum it will clone/fetch the package source, checkout the rev, verify
`source.path`, hash the package, and materialize it under the local AX cache.
Current preview metadata with all-zero revs or `sha256:preview-*` checksums is
skipped with a package-layer note instead of pretending it was verified. Mixed
local path + registry lock writing remains a future slice. The project loader
now recognizes registry dependencies through AX.lock schema v2 and the local
cache: missing lockfiles produce `PX0112`, and locked-but-not-cached packages
produce `PX0116`.

Package registry validation now has two smoke entry points:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-package-registry.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-package-registry-aot.ps1
```

The first smoke validates `pkg add -> pkg install -> check -> run` for the
curated package catalog. The AOT smoke then feeds that generated registry-backed
project into the LLVM AOT parity harness. It requires a link-capable native
toolchain; on Windows, LLVM `clang.exe` alone may still fail if MSVC or MinGW
link libraries such as `libcmt.lib`, `oldnames.lib`, or the MinGW CRT import
libraries are not installed.

Phase 0.2, download/install preview:

```powershell
axc pkg add text_tools
axc pkg install <project>
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

The installer also keeps git checkouts under:

```text
%USERPROFILE%\.ax\git\
```

`AX_HOME` can override the cache root for tests or isolated installs.

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
2. Author opens a package-index proposal. For the foundation preview, package
   source usually lands under `AX-FDN/AX-PKG/packages/<package-name>`.
3. Maintainers review metadata, license, module names, examples, and validation.
4. The package metadata is merged into the curated index.
5. Users can add and install it through `axc pkg add` and `axc pkg install`.

This keeps AX's early package ecosystem useful without opening the security and moderation surface of public upload too early.
