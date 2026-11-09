---
format: aep.planning-md/3
id: story:relate-and-unrelate-write-the-revision-they-print
kind: story
status: active
title: relate and unrelate write the revision they print, and history shows a removed edge
refs:
- provider: github
  reference: beyond10x/aep#153
relations:
- decomposes: epic:zero-open-issues-2026-11-09
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: inferred
  path: crates/edge/aep-cli/src/planning/git_record.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests/git_store.rs
- confidence: inferred
  path: crates/plan/aep-backend-markdown/src/backend.rs
- confidence: inferred
  path: crates/plan/aep-backend-markdown/src/journal.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/projection.rs
- confidence: cited
  path: docs/design/git-native-planning-store-v0.1.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:41Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:42Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"review_outcome":1}}}
---
# Story: relate and unrelate write the revision they print, and history shows a removed edge

## Outcome

Somebody who runs `aep plan artifact unrelate` on a Git-native store (`aep.project/5`) finds, in
the artifact file and in `aep plan artifact history`, the revision the command printed and the
removal it made.

## Context

GitHub issue beyond10x/aep#153, reported from an adopter's store on aep 0.79.0.

Reproduced 2026-11-09 with aep 0.79.0 in a throwaway `aep.project/5` store holding `epic:e1` and
`story:s1 decomposes epic:e1`, both committed at revision 1:

```console
$ aep plan artifact unrelate story:s1 decomposes epic:e1
story:s1 decomposes epic:e1 removed (revision 2)
$ grep revision .engineering/planning/story/s1.md
revision: 1
$ aep plan artifact history story:s1
story:s1: nothing recorded
$ aep plan artifact relate story:s1 decomposes epic:e1
story:s1 decomposes epic:e1 (revision 2)
$ grep revision .engineering/planning/story/s1.md
revision: 1
```

The edge is removed from the file and put back; the `revision:` line never changes, so both verbs
print a revision the store does not hold. `docs/design/git-native-planning-store-v0.1.md` § 4.2
says `revision` "increments by 1 on every CLI write to this file", and § 4.3 rests conflict
detection on that line changing.

The printed number comes from the in-memory document (`crates/edge/aep-cli/src/planning.rs`, `fn
unrelate`, `document.frontmatter.revision`), not from what the backend wrote. The backend's handling
of `Command::RemoveRelation` and `Command::CreateRelation` is in
`crates/plan/aep-backend-markdown/src/projection.rs`.

## Acceptance

Three outcomes, each checked on its own by a test on an `aep.project/5` store whose artifact is
committed at revision N before the command runs:

1. **`unrelate` writes what it prints.** After `aep plan artifact unrelate <id> <relation>
   <target>` exits 0 and prints `(revision M)`: the artifact file's `revision:` line reads `M`,
   `M = N + 1`, and `aep plan artifact history <id>` lists one entry at revision `M` naming the
   removal, the relation and the target. Before this story the file reads `N` and `history` prints
   `nothing recorded` (reproduced above).
2. **`relate` writes what it prints.** After `aep plan artifact relate <id> <relation> <target>`
   exits 0 and prints `(revision M)`: the artifact file's `revision:` line reads `M` and `M = N + 1`.
   Before this story the file reads `N` (reproduced above).
3. **A refused `unrelate` writes nothing.** `aep plan artifact unrelate` naming an edge the artifact
   does not hold exits 1, prints no `(revision` text, and every file under `.engineering/` is
   byte-identical before and after.

In each of 1 and 2, `aep plan artifact validate` prints `valid` on the uncommitted change and again
after it is committed.

## Decisions taken at filing (coordinator, not read from the issue)

- The issue offers "writes and records" or "refuses". Writing is taken: the design already says
  every CLI write bumps `revision`.
- Where the removal is recorded is the implementor's to propose, inside these bounds: it is written
  by the one write path, it is in files the store already owns, it survives a squash merge (so it is
  not read from `git log`), and § 7.1 of the design is updated to say so.

## Out of scope

SQLite and Postgres stores: `story:history-shows-a-removed-edge` owns the removed edge there.
