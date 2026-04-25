# Import/Module Minimal Design

Status: frozen design for `P1-15`, with the first parser / project / semantic-check slice now implemented. Package-system expansion and later module-system depth remain future work.

## Why AX needs this now

AX is no longer at the "single file toy example" stage.

- The shared AX-side foundation in [`../foundation/`](../foundation/) already carries reusable helpers for CLI guards, reports, text analysis, search, file-kind filters, and workspace labels.
- Multiple project-backed examples now load both `../../foundation` and `lib` through `AX.toml` `sources = [...]`.
- Real projects are already large enough that the flat global namespace is being held together mostly by naming discipline.

That means the next missing piece is no longer "one more helper container".
It is a deterministic way to separate shared foundation code from project-private code without immediately jumping to a full package system.

## Design goals

The first `import / module` slice must:

- keep the current `AX.toml` + `sources = [...]` project model alive
- preserve AX's AI-first preference for unique, explicit, deterministic rules
- solve namespace pressure for shared foundation + project-local libraries
- avoid opening a full package manager / visibility / ecosystem track too early

## Frozen minimal scope

### 1. File discovery stays in `AX.toml`

`import / module` does not replace project discovery.

- `entry` still names the one executable entry file.
- `[package].sources` still lists the support files or support directories that belong to the project.
- `check / run / build / fmt` still load files from the manifest.
- No implicit filesystem crawling is added beyond what `sources` already defines.

So in v1:

```toml
manifest_version = 1

[package]
name = "project_workspace_search_report"
entry = "src/main.ax"
sources = ["../../foundation", "lib"]
```

The manifest remains the source-of-truth for "which AX files are part of the project".

### 2. One support file = one module

Support-source files get a required module declaration:

```ax
module foundation.search;
```

Rules:

- `module` is allowed only once per support file.
- `module` must appear before any non-import item.
- A module file may still declare multiple top-level functions, structs, and enums.
- The entry file does not declare a module in the minimal design; it remains the manifest entry unit that owns `fn main() -> i32`.

This keeps the first step small:

- support files become namespaced
- the entry file imports and calls them
- we do not need to redesign the entire entry model yet

### 3. Module path is derived from source-root + relative file path

The compiler computes the expected module path from the manifest source entry.

Rules:

- Directory source entries derive a root module segment from their basename.
- File source entries derive a root module segment from their file stem.
- Nested directories append one segment per path component.
- The declared `module` path must exactly match the expected path.

Examples:

- `sources = ["../../foundation", "lib"]`
- `../../foundation/search.ax` => `module foundation.search;`
- `../../foundation/workspace.ax` => `module foundation.workspace;`
- `lib/report.ax` => `module lib.report;`
- `lib/audit/totals.ax` => `module lib.audit.totals;`
- `sources = ["src/report.ax"]` => `src/report.ax` must declare `module report;`

Additional constraints:

- Duplicate root aliases are rejected. Example: two source entries whose basename is both `lib`.
- Path segments are ASCII identifier segments and should match the file naming convention already used by AX examples: lower snake_case.
- A file-path / module-path mismatch is a hard error.

This is intentionally strict because AX should prefer a single obvious mapping over flexible magic.

### 4. `import` is explicit and top-of-file

Minimal syntax:

```ax
import foundation.search;
import foundation.workspace;
import lib.report;
```

Rules:

- `import` lines appear after an optional `module ...;` line and before all other items.
- Each `import` names exactly one module path.
- No wildcard import.
- No grouped import.
- No relative import.
- No alias import in v1.
- Duplicate imports are rejected.

The point of `import` in v1 is not shorthand. Its job is to make dependencies explicit and machine-checkable.

### 5. Cross-module references use fully qualified item paths

Inside the same module, local items can still be referenced by their plain name.

Across modules, references use the full module path plus the item name:

```ax
let stats: foundation.search.SearchStats =
    foundation.search.search_text(text, needle);

let label: string = foundation.workspace.display_label(path, depth);
let summary: string = lib.report.build_summary(root_dir, needle, totals);
```

Rules:

- Cross-module function calls use `module.path.item(...)`.
- Cross-module type references use `module.path.TypeName`.
- Struct and enum names follow the same qualified form.
- Using a qualified path without a matching `import` is an error.

This is the most AI-friendly choice for the first version because:

- there is exactly one stable spelling
- there is no hidden aliasing layer
- diagnostics can point to the exact missing module path

### 6. Visibility stays flat in v1

The minimal module design does not add `pub`, `private`, or re-export rules yet.

Rules:

- All top-level items declared in a module file are importable.
- There is no visibility keyword in v1.
- Re-exporting items from another module is not supported.

This keeps the first resolver slice focused on namespacing, not on access control.

### 7. Import graph is for visibility, not initialization order

AX support files do not contain top-level executable statements.
They contain declarations.

Because of that, the first module system should treat imports as a visibility contract rather than an initialization-order contract.

That means:

- import cycles do not need special semantic meaning in v1
- the compiler can build the full module registry first, then resolve references
- we avoid pulling package-loader complexity into the first module milestone

## Compatibility and migration boundary

### Keep current projects working

Current projects that only use manifest-level source loading remain valid.

Example:

- project uses `AX.toml`
- support files are loaded through `sources = [...]`
- all top-level names still merge into the current flat namespace

That stays supported until projects opt into module mode.

### Module mode trigger

The minimal proposal uses a simple project-level trigger:

- if any support source declares `module ...;`, or
- if the entry file contains `import ...;`

then the project is treated as a module-mode project.

In module mode:

- every support source must declare its module path
- cross-file references must be resolved through modules
- old flat cross-file name lookup is disabled

This gives a clean migration boundary instead of a blurry mixed model.

### Migration path

1. Keep `AX.toml` unchanged.
2. Add `module ...;` to support files under shared roots such as `foundation/` and `lib/`.
3. Add `import ...;` lines to the entry file.
4. Rewrite cross-file references to fully qualified module paths.
5. Leave same-file helper calls unchanged.

That migration is intentionally mechanical so both humans and AI can do it predictably.

## Example: current project to module-mode project

Current project:

```toml
manifest_version = 1

[package]
name = "project_workspace_search_report"
entry = "src/main.ax"
sources = ["../../foundation", "lib"]
```

`../../foundation/search.ax`

```ax
module foundation.search;

struct SearchStats {
    match_count: i32,
}

fn search_text(text: string, needle: string) -> SearchStats {
    return SearchStats { match_count: 0 };
}
```

`lib/report.ax`

```ax
module lib.report;

fn build_summary(root_dir: string, needle: string, count: i32) -> string {
    return root_dir + needle + to_string(count);
}
```

`src/main.ax`

```ax
import foundation.search;
import foundation.workspace;
import lib.report;

fn main() -> i32 {
    let text: string = "hello";
    let needle: string = "he";

    let stats: foundation.search.SearchStats =
        foundation.search.search_text(text, needle);
    let label: string = foundation.workspace.display_label("src", 0);
    let summary: string =
        lib.report.build_summary(label, needle, stats.match_count);

    println(summary);
    return 0;
}
```

## First diagnostics to add when implementation starts

The first implementation slice should add dedicated diagnostics for:

- module declaration missing in module mode
- file path does not match declared module path
- duplicate module path
- imported module not found
- duplicate import
- cross-module qualified path used without import
- imported module exists but named item does not exist
- entry file tries to declare `module` in the minimal v1 model

These should get stable diagnostics and AI rule cards from day one, not as an afterthought.

## Explicit non-goals for the first slice

The first `import / module` milestone does **not** include:

- package manager support
- remote dependencies
- version resolution
- visibility keywords such as `pub`
- wildcard imports
- alias imports
- relative imports
- re-exports
- package-level initialization
- module-level top statements
- automatic filesystem discovery beyond manifest `sources`

## Implementation order after design freeze

Once the design is accepted, the implementation order should be:

1. parser support for `module` and `import`
2. project/source-root to expected-module-path mapping
3. module registry build step
4. resolver support for fully qualified cross-module item paths
5. diagnostics + AI guidance for module mistakes
6. one migrated project-backed example using shared `foundation` + `lib`
7. benchmark and snapshot updates

That keeps the next coding phase on the critical path instead of letting modules explode into a full package-system rewrite.
