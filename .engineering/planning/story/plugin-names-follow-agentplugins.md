---
format: aep.planning-md/1
id: story:plugin-names-follow-agentplugins
kind: story
status: implemented
title: This repository names the renamed plugins
summary: Replace every aep-planning, adp and ess-schema reference — the eval.rs ess-on-PATH prefix, eval and trace fixtures, conformance cases, website guides — with aep-plan, aep-drive and ess-specify.
relations:
- decomposes: epic:area-layout
- serves: vision:O2
- depends_on: story:profile-and-cli-crates-named-after-aep
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: conformance/eval
- confidence: cited
  path: conformance/trace
- confidence: cited
  path: crates/edge/aep-cli/fixtures
- confidence: cited
  path: crates/edge/aep-cli/src/eval.rs
- confidence: cited
  path: crates/observe/trace-spec/tests/fixtures
- confidence: cited
  path: website/docs/guides
- confidence: cited
  path: website/docs/reference/harnesses.md
revision: 7
---
# Story: This repository names the renamed plugins

## Context

The sibling `agentplugins` repository renames `aep-planning` to `aep-plan`, `adp` to `aep-drive`
and `ess-schema` to `ess-specify` (skill `specify`). This repository holds references to the old
names that must follow:

- `crates/edge/aep-cli/src/eval.rs:2067-2642` hardcodes the `ess-schema:` skill prefix for the
  `ess`-on-PATH check; it becomes `ess-specify:`.
- `eval.rs:4904` and `:4946` test fixtures name `aep-planning:planning`, `ess-schema:ess-schema`
  and `beyond10x/agentplugins@aep-planning@<sha>`; the sha is the agentplugins commit that renamed.
- `crates/edge/aep-cli/fixtures/eval-*`, `crates/observe/trace-spec/tests/fixtures/*.jsonl`,
  `conformance/eval/*` and `conformance/trace/*.trace.yaml` carry the old skill and agent ids.
- `website/docs/guides/{govern-a-task,integrate-a-harness,check-a-transcript}.md` and
  `website/docs/reference/harnesses.md` name the plugins.

## Acceptance

Every authored reference names the new plugin, skill and agent ids; `EVAL-RUN-018` fires on either
`ess-specify:` or `ess-schema:` and a test pins both; `task check` exits 0. Recorded
`metaharness.event/1` transcripts are not rewritten, and every expectation row, task statement or
quoted report line judged against or describing a recording keeps the recorded spelling under a
`# recorded-under-this-name` marker (or a prose note in docs), so that `aep trace check` over every
touched recording pair is byte-identical before and after. The released pin
`beyond10x/agentplugins@aep-planning@0.4.0` in `tests/eval_run.rs` keeps its name.

## Notes

Cannot start before the agentplugins rename commit exists; the fixture sha at `eval.rs:4946` is
read from it. Cross-repository dependency, recorded here in prose because the store cannot relate
to another repository's artifact.

Agentplugins adversary (2026-09-03) confirmed the hazard: with the plugin renamed, a case naming
`ess-specify:*` no longer trips `EVAL-RUN-018` (`eval.rs:2642`, `starts_with("ess-schema:")`), so a
labelled live run spawns and pays on a runner with no `ess`. Accept both prefixes for one release,
or ship this story before any agentplugins live run under the new names.
