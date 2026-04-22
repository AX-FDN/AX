# AX Diagnostics Schema

## Scope

This document covers the stable JSON output of:

- `axc check <file> --json`
- `axc check <file> --json --ai`

It does not document:

- text-mode diagnostic rendering
- private internal compiler structs
- the on-disk format of `--ai-session` state files

## Command-Level Contract

### `axc check <file> --json`

- exit code `0`
  The file passed all checks and stdout is `[]`.
- exit code `1`
  One or more diagnostics were emitted and stdout is a JSON array of diagnostic objects.
- exit code `2`
  CLI usage error such as missing path or invalid flag combination.

### `axc check <file> --json --ai`

This keeps the base diagnostic shape intact and may add an optional `ai` field per diagnostic.

Important compatibility rule:

- base fields do not change meaning when `--ai` is enabled
- diagnostics without a matched AI rule simply omit the `ai` field

### `axc check <file> --json --ai --ai-session <path>`

This enables session-scoped teaching escalation.

Stable behavior:

- base diagnostic ordering stays the same
- base code, message, file, span, notes, expected, and suggestion stay the same
- only AI-layer teaching fields such as `teaching_level`, `repeat_count`, and richer examples may change

## Base Diagnostic Object

Current JSON shape:

```json
{
  "code": "S0022",
  "message": "cannot initialize `value` of type `bool` with `i32`",
  "file": "examples/type_mismatch.ax",
  "span": {
    "start": 37,
    "end": 38
  },
  "notes": [
    "AX does not implicitly convert `i32` to `bool`"
  ],
  "expected": [],
  "suggestion": "change the expression or annotation so both sides use the same AX type"
}
```

Fields:

- `code: string`
  Stable compiler diagnostic code such as `L0001`, `P0001`, or `S0022`.
- `message: string`
  Primary human-readable message.
- `file: string`
  Display path of the source file as seen by the compiler.
- `span.start: integer`
- `span.end: integer`
  UTF-8 byte offsets into the source file.
- `notes: string[]`
  Additional ordered notes.
- `expected: string[]`
  Ordered expectation hints, mainly used by parser diagnostics.
- `suggestion: string | null`
  Optional repair hint. When absent internally, JSON serializes it as `null`.

## Span Semantics

`span.start` and `span.end` are not line and column numbers.

They are byte offsets into the original UTF-8 source text:

- `start`
  Inclusive start offset.
- `end`
  Exclusive end offset.

Consumers that need line and column should recompute them from the original source text.

## AI Extension Object

When `--json --ai` is enabled and a diagnostic matches a registered AI rule, the diagnostic may include:

```json
{
  "ai": {
    "rule_id": "type_match_required",
    "teaching_level": "L1",
    "repeat_count": 1,
    "repair_goal": "Change the expression or the declared type so both sides use the same AX type.",
    "rule_card": {
      "summary": "AX requires assignments, arguments, returns, and conditions to use the declared type exactly."
    },
    "fixits": [
      "make the expression and the expected AX type agree"
    ]
  }
}
```

Current AI fields:

- `rule_id: string`
  Stable AI rule identifier.
- `teaching_level: "L1" | "L2" | "L3"`
  Session-scoped teaching depth.
- `repeat_count: integer`
  Count of repeated occurrences for the same normalized rule pattern in the current session.
- `repair_goal: string`
  Stable repair objective for the diagnostic.
- `focus_item?: object`
  Optional high-level item containing:
  - `kind: string`
  - `name: string`
  - `signature?: string`
  - `span: { start, end }`
- `relevant_spans?: Span[]`
  Related source regions for the repair context.
- `related_symbols?: object[]`
  Optional symbol summaries with:
  - `kind`
  - `name`
  - `signature?`
  - `span`
- `rule_card: object`
  Stable teaching card containing:
  - `summary: string`
  - `pattern?: string`
  - `minimal_example?: string`
  - `anti_pattern?: string`
- `fixits?: string[]`
  Ordered repair hints. If the base diagnostic has a suggestion, it is usually surfaced here first.
- `context_snippets?: object[]`
  Optional higher-detail snippets with:
  - `label: string`
  - `text: string`
  - `span`

Fields marked with `?` may be omitted from JSON when empty or unavailable.

## Teaching Levels

Current teaching escalation policy:

- `L1`
  Short summary only.
- `L2`
  Summary plus structural pattern.
- `L3`
  Summary plus pattern, example, anti-pattern, related symbols, and context snippets.

Session escalation thresholds:

- first occurrence: `L1`
- repeat count `2-3`: `L2`
- repeat count `4+`: `L3`

Without `--ai-session`, every invocation starts at `L1`.

## Compatibility Rules

The repository currently treats the following as stable public behavior:

- base diagnostic array output for `--json`
- optional `ai` field addition for `--json --ai`
- omission of `ai` when no rule matches
- stable meaning of `rule_id`
- stable meaning of `teaching_level`
- stable meaning of `repair_goal`

The following are intentionally allowed to evolve without breaking the public contract:

- exact English wording of `message`, `notes`, or `rule_card.summary`
- the amount of `context_snippets` detail at higher teaching levels
- the specific set of diagnostics currently mapped to AI rules

## Example: Base Versus AI

Base mode:

```json
[
  {
    "code": "P0001",
    "message": "expected `;` after expression statement",
    "file": "examples/missing_semicolon.ax",
    "span": { "start": 38, "end": 39 },
    "notes": [
      "found `return` instead",
      "AX uses explicit semicolons after `let`, assignments, expression statements, and `return`."
    ],
    "expected": ["`;`"],
    "suggestion": "insert `;` before the next statement or closing `}`"
  }
]
```

AI-enhanced mode:

```json
[
  {
    "code": "P0001",
    "message": "expected `;` after expression statement",
    "file": "examples/missing_semicolon.ax",
    "span": { "start": 38, "end": 39 },
    "notes": [
      "found `return` instead",
      "AX uses explicit semicolons after `let`, assignments, expression statements, and `return`."
    ],
    "expected": ["`;`"],
    "suggestion": "insert `;` before the next statement or closing `}`",
    "ai": {
      "rule_id": "statement_terminator_required",
      "teaching_level": "L1",
      "repeat_count": 1,
      "repair_goal": "Insert the missing semicolon so the statement terminates correctly.",
      "rule_card": {
        "summary": "AX requires `let`, assignment, expression, and `return` statements to end with `;`."
      },
      "fixits": [
        "insert `;` before the next statement or closing `}`"
      ]
    }
  }
]
```

## Known Limits

Current JSON diagnostics do not include:

- line and column numbers
- severity levels
- nested child diagnostics
- machine-applicable text edits with exact replacement ranges

If any of those become public contracts later, they should be added in a schema-versioned way instead of changing the meaning of existing fields.
