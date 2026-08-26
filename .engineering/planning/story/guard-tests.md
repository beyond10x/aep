---
format: aep.planning-md/1
id: story:guard-tests
kind: story
status: implemented
title: A test that asserts what another crate's test asserts is not evidence of a difference
summary: 'Cross-crate duplicate test bodies are a gate failure: either the assertion belongs in a shared suite, or the test is named for a property it does not assert.'
relations:
- decomposes: epic:evidence-gated-completion
revision: 4
---
# Story: A test that asserts what another crate's test asserts is not evidence of a difference

## Outcome

An implementer citing a test as proof that their store does something the others cannot gets told,
by the gate, when that test proves no such thing.

## Context

On 2026-08-26 a reviewer found `entity-sqlite`'s `a_refused_commit_rolls_back_both_halves` asserting
a refusal that happens at the **pre-check**, before either write — so there were no halves to roll
back. It was byte-for-byte the same assertion as `every_provider_leaves_a_refused_commit_with_no_trace`,
which runs against the providers whose documentation says they cannot keep that promise. It had been
cited as evidence for R-103 across two releases, and the gate was green throughout: the gate checks
that a cited test **passes**, and a test that asserts nothing distinguishing passes beautifully.

## Acceptance

`cargo xtask guards` normalises every `#[test]` body — comments and whitespace out — and reports any
that appear in more than one crate; `guard-check` runs it in the gate and CI runs it beside
`version`; the three parallel command vocabularies (`adp`/`aep`/`aop`) are **allowlisted by name
with the reason inline**, so a reader can argue with the exception rather than discover a blind spot.

**A name heuristic was tried first and is recorded as rejected**: matching `only`, `cannot`,
`rolls_back` and the rest reported 92 findings against correct code, because ordinary test names
contain those words. A check that fires that often on working code is one somebody switches off,
which makes it worse than nothing. A duplicated body is a fact; a suspicious name is a guess.

## Out of Scope

Whether a guard is *correct*. No script can know that, and pretending otherwise would be this same
defect one level up. What it can know is whether two crates are asserting the same thing twice.

## Open Questions

None outstanding.
