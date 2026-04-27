# Why Not Existing Language Subsets?

This is the sharp positioning question AX has to answer:

> Why not just use a constrained Rust / Go / Python subset and add some lints?

Short answer:

If all you want is familiar syntax plus a little discipline, an existing language subset is the simpler answer.
AX only deserves to exist if four things are owned together and measured together:

- canonical syntax
- structured diagnostics
- repair contract
- benchmark evidence

If those four do not create measurable repair lift, AX should narrow its scope or lose the argument.

## The Real Claim

AX is not claiming:

- "new syntax is automatically better"
- "tokenizer-aware keywords are enough"
- "another language is valuable by default"

AX is claiming something narrower and harder:

> an AI-first tool language can make source form, diagnostics, repair feedback, and benchmark evidence work together more stably than treating a general-purpose language as an accidental prompt format

That claim only makes sense when all four layers move together.

## 1. Canonical Syntax

A subset can restrict what is *allowed*.
It usually does not fully control what is *normal*.

General-purpose languages still bring:

- multiple equivalent spellings
- style drift across teams and eras
- larger latent syntax surfaces than the benchmark actually wants
- formatter and idiom conventions not designed around low-entropy model consumption

AX is trying to own the surface directly:

- fewer equivalent forms
- tighter formatter control
- smaller prototype syntax surface
- less ambiguity about what "the normal way to write this" looks like

If the goal is stable generation and stable repair, that control matters more than syntax familiarity alone.

## 2. Structured Diagnostics

A general-purpose compiler mostly exists to serve human developers.
Its machine-readable diagnostics are often secondary, unstable, or too broad for a narrow repair workflow.

AX is trying to make structured diagnostics part of the product surface:

- stable codes
- stable spans
- stable JSON shape
- diagnostics that are designed to be consumed by both humans and agents

That matters because repair does not start from syntax alone.
It starts from what the compiler can say, consistently, about the failure.

## 3. Repair Contract

This is the layer most "just use a subset" arguments underestimate.

AX is not only emitting diagnostics.
It is trying to emit a repair contract:

- `rule_id`
- `repair_goal`
- `focus_item`
- `relevant_spans`
- `rule_card`
- `fixits`

That contract comes from the same language semantics that produced the failure.

With a subset plus post-hoc lints, the repair story often fragments:

- compiler errors come from one place
- lints come from another
- prompts come from a third
- benchmark wrappers come from a fourth

Once that happens, you get drift between:

- what is wrong
- what the agent is told is wrong
- what the scoring harness expects to be fixed

AX exists to make those three layers line up.

## 4. Benchmark Evidence

Without evidence, "AI-friendly language" is marketing.

AX is trying to make the evidence loop first-class:

- fixed broken cases
- exported bundles
- replay candidates
- scoring scripts
- mode comparisons
- smoke checks in CI

That is why the repository matters as more than a parser prototype.
It is trying to make the repair claim measurable instead of anecdotal.

The next evidence layer is [`Repair Archaeology v0`](./repair-archaeology.md).
It does not replace the benchmark; it explains individual replay cases as JSON / Markdown evidence objects so readers can inspect what failed, what repaired, which contract was used, and how to reproduce the result.

## Why Not Just Add These Four To A Subset?

You can.
In fact, that is exactly the right baseline to compare against.

But once you do it seriously, you are no longer only choosing a familiar syntax.
You are defining and maintaining the AI-facing surface of a language:

- what source surface is canonical
- what diagnostics contract is stable
- what repair bundle is emitted
- how benchmark evidence is gathered

At that point the real question becomes:

> do you want to own that protocol end-to-end, or keep borrowing it from a language and toolchain that were built for different priorities?

AX is the "own it end-to-end" answer.
It only wins if the benchmark eventually shows that ownership matters.

## The Honest Boundary

There is one honest constraint here:

If AX cannot eventually outperform constrained existing-language baselines on the target tasks, then the project should not pretend syntax novelty is enough.

The future benchmark has to test exactly that:

- same task
- same model
- same retry budget
- same tool access
- AX versus constrained Rust / Go / Python subsets

Until then, the right framing is:

- AX is an AI-first tool language with an owned source protocol, diagnostics contract, repair contract, and benchmark loop
- AX is not yet a universally proven replacement for constrained existing-language subsets

That framing is sharper and more credible than calling it a language revolution too early.
