---
format: aep.planning-md/1
id: story:generator-version-stamp
kind: story
status: implemented
title: A generated artefact records what it was made from, not which build made it
revision: 5
---
# Story: A generated artefact records what it was made from, not which build made it

## Outcome

Cutting a release stops rewriting 120 files that did not change. Whoever reads a release diff sees
the work in it, instead of the work plus 120 lines of `0.26.0` replacing `0.27.0`.

## Context

Every generated projection carries a provenance block:

```json
"source_digest":     "13577b…",
"contract_digest":   "4ccf8a…",
"compiler_version":  "0.26.0",
"generator_version": "0.26.0",
```

The two digests content-address the input. The two versions name the build — and on 2026-08-26,
setting the workspace version to match the release tag (so `protocol --version` could finally say
which build it was) rewrote **120 generated files whose content was otherwise identical**, plus both
committed conformance evidence documents.

That is now a permanent tax: every release fails `generate-check` until somebody regenerates, and a
release cut without regenerating fails the gate *at the tag*. It fails loudly with the fix in the
message, which is why it was accepted rather than fixed on the spot — but 120 files of noise per
release is how a real change hides in a diff nobody reads to the end.

**The argument for removing them.** `source_digest` and `contract_digest` already answer *what was
this made from*. `generator_version` answers *by which build* — which is only actionable when the
generator's output actually differs, and when it does, the output differs and the diff shows it. A
version stamp that changes when nothing else does is a second copy of the tag, and the second copy
is where the drift starts (the same argument `docs/status.md` § *the tags are the record* makes).

**The argument for keeping them.** An artefact that cannot say which build produced it is exactly
what made the stale `protocol` binary invisible for a day. The difference is that a *binary* has no
other identity, and a generated artefact has two digests.

## Acceptance

A decision, recorded, and then whichever of these it chose:

- **Removed** — `compiler_version` and `generator_version` leave the provenance block; the digests
  stay; `cargo xtask generate` produces byte-identical output across a version bump, proven by a
  test that bumps the version and regenerates; the format change is a changelog entry, because an
  adopter parsing that block is relying on it.
- **Kept** — the release procedure in `AGENTS.md` § *Releases* names the regeneration step, so
  cutting a tag without it is a documented mistake rather than a surprise at the gate.

## Decided and shipped — **Removed**, 2026-08-28

**The Open Question is answered with evidence, and it answers the story.** *Does anything read
`generator_version` today?* — **nothing reads it to decide anything.**

| kind of site | where | what it does |
|---|---|---|
| schema | `schemas/generated/evidence.schema.json` | declares it **optional**; `required` never held it |
| copier | `ess-conformance/src/scenario.rs`, `evidence.rs` | copies it onward, never branches on it |
| display | `ess-synth/src/web/page.rs` | one row in the lab page |
| test | `ess-gen/tests/{openapi,asyncapi}.rs`, `ess-conformance/tests/suite.rs` | asserts the stamp is carried |
| **consumer** | `website/src`, `scripts/`, `integrations/`, `conformance/` | **0 hits** |

And it was not actionable even where it was read: every stamped value in the tree was `0.31.0` —
the workspace version, which is the tag, because `AGENTS.md` § *Releases* regenerated **at** the
tag. The field was `git describe` copied into 115 files, and between tags it named the *previous*
release rather than the build. So the default *keep and document* was refused on the evidence rather
than followed on principle, and the **Removed** branch was taken.

**Nothing replaced it, and the plan's own default was refused too.** `docs/plan/next-ten-steps.md`
D4 proposed a digest over the generator's own source. It answers the same question no better and
moves on **every commit to the generator** rather than once per release — the same defect with a
shorter period. `source_digest` and `contract_digest` already content-address the input, and they
are untouched.

Removed beyond the story's two fields, because acceptance line 5 could not otherwise hold:
`synthesizer_version` (suites), `planner_version` (synth plans) and `infra`'s `compiler_version` —
leaving them, a release still rewrote 12 files and § *Releases* kept its regeneration block.

| | |
|---|---|
| acceptance: byte-identical output across a version bump | `a_version_bump_rewrites_no_generated_file` (xtask). Before: *115 generated files spell the build version `0.31.0`* — after: 1 passed |
| files changed | 143 — 111 `generated/`, 15 in 7 crates, 9 docs, 3 suites, 3 examples, 2 schema/xtask |
| `AGENTS.md` § *Releases* | lost the regeneration paragraph, the four `cargo xtask` lines and the `bridge.js` copy; **kept** the two `ess conform evidence` commands, whose `implementation: billing-reference 0.31.0` is the version of the binary the record is *about* — content, not a writer's stamp |
| the decision, recorded | `docs/design/reconciliation-v0.2.md` § 5 deviation 13, the normative register |

**Two fields survive as read-only rather than deleted**, and the reason is a refusal that would
otherwise be invisible: `EssConformanceResult` is `deny_unknown_fields`, so deleting them outright
would **refuse every conformance record already written**. They are `#[serde(default,
skip_serializing)]` — accepted, never written. Removing them for real is a format-version bump and
is not this story. The same holds for `infra-ir/1`'s mirror.

First thing to check if this breaks something: a consumer that pinned on the *presence* of
`x-ess-provenance.generator_version` rather than on its value. The field is now absent, not empty.

## Out of Scope

The digests. They are the part that means something and nothing here touches them.

## Open Questions

Whether any adopter reads `generator_version` today. Decides: whoever owns the ESS surface. Default
if nobody answers: **keep and document**, because removing a field somebody parses is the one option
that cannot be undone quietly.
