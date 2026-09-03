---
format: aep.planning-md/1
id: story:crates-under-area-directories
kind: story
status: implemented
title: Crates move under area directories; names and consumers unchanged
summary: git mv the 22 crates into crates/{govern,plan,drive,observe,profile,edge}; fix member paths, workspace dependency paths, the xtask doc string, the Taskfile mention and the three crate tables; no crate renamed.
relations:
- decomposes: epic:area-layout
- serves: vision:O2
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates
- confidence: cited
  path: website/docs/concepts/overview.md
- confidence: cited
  path: xtask/src/main.rs
revision: 7
---
# Story: Crates move under area directories; names and consumers unchanged

## Context

Every crate sits flat under `crates/`. The dependency graph already has five contexts and an IO
edge; the directory tree does not show them. Cargo workspace members may live at any path, and a
git dependency is resolved by package name across the members, so a path move changes nothing a
consumer pins. Target layout:

```
crates/govern/   aep-domain aep-engine
crates/plan/     aep-contract aep-conformance aep-client aep-backend-{memory,markdown,entity,sqlite,postgres,hybrid}
crates/drive/    aep-driver-spec aep-driver aep-render
crates/observe/  trace-domain trace-spec aep-ess-evidence
crates/profile/  adp-domain aop-domain
crates/edge/     aep-schema aep-project protocol-cli
```

`aep-schema` goes to `edge/`, not `govern/`, because it depends on `aep-backend-markdown`,
`aep-driver-spec` and `trace-domain` (three areas). `xtask/` stays at the root. Known literal
paths: `Cargo.toml` `[workspace] members` and `[workspace.dependencies]`; `xtask/src/main.rs:753`
(a doc string naming two backend manifests); `Taskfile.yml` (one `crates/` mention); the component
tables in `README.md`, `AGENTS.md` § What this repository is, and
`website/docs/concepts/overview.md` § Layers. Anything else `rg 'crates/'` finds is in scope.

The one area-rule exception to document in `AGENTS.md`: `aep-engine` imports `aep_contract::command`
at `crates/govern/aep-engine/src/trail.rs:15` (govern → plan); it is left in place here.

## Acceptance

`git mv` has placed every crate under its area directory, `task check` exits 0 from a clean
checkout, and every literal `crates/…` path that names a workspace crate resolves on disk in every
tracked file, with these exclusions only: `CHANGELOG.md`; `.engineering/planning/journal.jsonl`
(append-only); `docs/design/` and `docs/reviews/` (dated record, not rewritten); recorded
`metaharness.event/1` transcripts and the `case.yaml` and blog lines that quote them. `docs/plan/`
is live (`AGENTS.md` § Normative documents) and is rewritten. Every live (draft, proposed, active)
planning artifact's `scope:` entries are area-qualified, because `aep artifact waves` compares
scope strings without normalising. Both halves are tests in `xtask` that read `git ls-files` and
the store, with an anti-vacuity assertion each.

## Notes

No crate is renamed and no `Cargo.toml` `name` changes. `CHANGELOG.md` gains an Unreleased entry.
The README table gains an "area" column. The `AGENTS.md` crate list is rewritten by area.
