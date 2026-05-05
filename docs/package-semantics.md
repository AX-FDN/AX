# AX Package Semantics v1 Skeleton

This document defines how packages participate in AX 1.0 language semantics.

## Package Kinds

AX currently supports:

- local path packages
- curated registry packages

Both are source packages. Native binary packages, install scripts, and arbitrary
public upload are outside the 1.0 default plan.

## Registry Packages

A registry package is described by metadata in the compiler repository and
source in AX-PKG.

Required metadata:

- package name
- version
- source git URL
- pinned revision
- source path
- checksum
- exposed modules
- maturity

Allowed maturity values:

- `stable_pure_ax`
- `host_boundary_preview`
- `future_native_preview`

## Locking

Registry package use requires a lockfile entry. The lockfile makes package input
reproducible for check/run/build and for AI repair validation.

## Build Semantics

Build manifests must preserve registry package facts through `registry_packages`.
AOT readiness must use package maturity:

- `stable_pure_ax`: no package-maturity blocker.
- `host_boundary_preview`: `AOT0104`.
- `future_native_preview`: `AOT0105`.

Source validity and backend maturity are separate. A valid package user should
not be rewritten just because native ABI support is incomplete.

## 1.0 Package Non-Goals

- public upload server
- account/login/token system
- package install scripts
- native binary package install
- complete semver solver
- automatic trust of arbitrary package code
