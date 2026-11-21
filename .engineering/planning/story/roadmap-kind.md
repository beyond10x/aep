---
format: aep.planning-md/3
id: story:roadmap-kind
kind: story
status: active
title: 'A roadmap kind: an approved, ordered list of epics that validate checks'
tags:
- requested-by-sdc
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: artifacts/lifecycles/roadmap.yaml
- confidence: cited
  path: artifacts/templates/roadmap.md
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/src/planning/roadmaps.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/roadmap_kind.rs
- confidence: cited
  path: crates/edge/aep-project/src/project.rs
- confidence: cited
  path: crates/govern/aep-domain/src/artifact.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/findings.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/lib.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/roadmap.rs
- confidence: cited
  path: protocols/aep/1.yaml
- confidence: cited
  path: schemas/generated
- confidence: cited
  path: website/docs/concepts/artifacts.md
- confidence: cited
  path: website/docs/concepts/lifecycles.md
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/vocabulary.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T02:33:12Z", actor: "human:timo", revision: 3, executor: "agent:aep-roadmap-session"}
- {from: "proposed", to: "active", at: "2026-11-21T02:33:12Z", actor: "human:timo", revision: 4, executor: "agent:aep-roadmap-session"}
---
# Story: roadmap-kind

## Problem

AEP 0.80.0 has no kind that orders epics. The `sdc` repository's direction stage approves a roadmap
before any epic opens (`sdc/docs/phases.md` § Flow, step 6), and today it keeps that roadmap as an
`initiative` (`aep/initiative/roadmap.md` in a run's work tree) because no kind fits. An initiative
is a body of work, not an order over epics: nothing checks that the epics it names exist, that none
is named twice, or that live epics are on it, and its ladder has no approval rung.

Requested by the `sdc` session: `upstream-blocker:aep-roadmap-kind`, which blocks
`story:roadmap-kind-adopt`; `story:roadmap-reconstruct` and `sdc/docs/orchestration.md` § 4.4 gap 1
are the brownfield use (a store with epics and no roadmap).

## Design

- **Kind.** `roadmap`, a named planning kind (`ArtifactKind::Roadmap`, `is_planning`), listed in
  `protocols/aep/1.yaml` `artifact_kinds`.
- **Entries.** A fenced `roadmap` block in the body, read the way a `review-result`'s `findings`
  block is read: a YAML or JSON sequence of `{epic, horizon, rationale?}`. Order is list position;
  `horizon` is one of `now`, `next`, `later`, an ordered vocabulary like the protocol's `scales`, and
  never decreases down the list. No stored sequence number: inserting an entry would renumber every
  entry below it.
- **Ladder.** `artifacts/lifecycles/roadmap.yaml`: the vision ladder (`draft → in_review →
  approved → superseded | archived`, `rejected`), with `approved` requiring at least one `approval`
  evidence record. A changed roadmap is a successor that `supersedes` the approved one and earns its
  own approval: evidence counts are cumulative per artifact, so a second approval of the same
  artifact could not be told from the first. `body` refuses to rewrite an approved roadmap and names
  that route.
- **Validate.** Problems: an unreadable block; an entry naming an artifact that does not exist or is
  not an epic; an epic listed twice; an approved roadmap no `approval-record` reviews or decides;
  more approved roadmaps than the project's profile allows (one, unless the profile declares the fact
  `roadmap.approved.limit`). Reported and not a problem, refused by `--strict`: a `proposed` or
  `active` epic on no approved roadmap, once the store holds an approved roadmap.

## Acceptance

- `aep plan artifact new roadmap <name>` creates `roadmap:<name>` at `draft` with a body seeded from
  `artifacts/templates/roadmap.md`; `show`, `body`, `relate`, `list --kind roadmap` and `move` work on
  it as on any kind, and `waves` output is the same with and without a roadmap in the store.
- `move roadmap:<name> --to approved` from `in_review` is refused until an `approval` evidence record
  is held, and succeeds once one is.
- `validate` exits 1 naming the defect for each problem listed under Design, and exits 0 on a store
  whose approved roadmap lists every live epic once, with an approval-record that reviews it.
- `validate` lists a `proposed` or `active` epic that no approved roadmap names, exits 0 without
  `--strict` and 1 with it.
- `body` on an approved roadmap is refused and the file is unchanged.
- The CLI reference, the artifact concepts page, the vocabulary page and `CHANGELOG.md` name the kind.

## Out of scope

- `aep plan reverse scan` proposing a draft roadmap: the scan reads and does not interpret, and it
  does not read the planning store today. Drafted as its own story.
- The `agentplugins` skill text that lists kinds: another repository.
