---
format: aep.planning-md/1
id: story:generator-version-stamp
kind: story
status: draft
title: A generated artefact records what it was made from, not which build made it
revision: 1
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

## Out of Scope

The digests. They are the part that means something and nothing here touches them.

## Open Questions

Whether any adopter reads `generator_version` today. Decides: whoever owns the ESS surface. Default
if nobody answers: **keep and document**, because removing a field somebody parses is the one option
that cannot be undone quietly.
