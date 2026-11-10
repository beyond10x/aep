---
title: The Git-native planning store
sidebar_label: The planning store
sidebar_position: 5
description: aep.project/5 keeps the plan as one Markdown file per artifact and one JSON file per evidence record. Git is the history. What a write touches, how branches merge, and what validate checks.
---

# The Git-native planning store

Since `0.62.0` a project keeps its plan in the **Git-native store**, selected by
`version: aep.project/5` and `store: {git: {}}` in `.engineering/project.yaml`. It has three parts,
all plain files you commit:

```text
.engineering/
  project.yaml                                        aep.project/5
  planning/<kind>/<name>.md                           one artifact (aep.planning-md/3)
  evidence/<kind>/<name>/<instant>-<sequence>-<digest>.json      one evidence record, never rewritten
```

The store has no journal, event log, database or generated projection. The artifact files are the
authority, and Git is their history.

## One write, one file

| Command | Writes |
|---|---|
| `new` | the new artifact's file |
| `move` | the artifact's file: `status`, `revision`, and one appended `transitions` line |
| `body`, `set`, `scope`, `relate`, `unrelate` | the artifact's file, with `revision` + 1 |
| `evidence` | one new file under `evidence/<kind>/<name>/`; the artifact's file is unchanged |

Each write is atomic: the new file is written beside the old one, synced and renamed into place. A
refused command writes nothing. A move therefore shows up in a pull request as three changed lines:

```diff
@@ -4,5 +4,5 @@ id: story:pay-by-card
 kind: story
-status: active
+status: implemented
 title: Pay by card as a guest
-revision: 3
+revision: 4
 transitions:
@@ -10,2 +10,3 @@ transitions:
 - {from: "proposed", to: "active", at: "2026-09-28T08:58:38Z", actor: "human:alex", revision: 3}
+- {from: "active", to: "implemented", at: "2026-09-28T08:58:38Z", actor: "human:alex", revision: 4, decided_on: {"recorded":{"test_result":1}}}
 ---
```

## History

| Question | Answered from |
|---|---|
| when did it enter each status, by whom, on what evidence | the `transitions` list, which does not depend on how moves were grouped into commits |
| what evidence was recorded about it | the files under `evidence/<kind>/<name>/` |
| every version of the file, and who edited the body | `git log --follow -- .engineering/planning/<kind>/<name>.md` |

`aep plan artifact history <id>` merges the first two into one timeline, oldest first:

```shell-session
$ aep plan artifact history outbound-claim:q3-uptime
2026-09-28T08:57:37Z  human:alex  approval recorded from legal review (https://example.invalid/approvals/814) (revision 1)
2026-09-28T08:57:37Z  human:alex  moved draft -> cleared (revision 2)
2026-09-28T08:57:37Z  human:alex  moved cleared -> sent (revision 3)
```

`explain` reads the same records per status reached, as shown in the [quickstart](../getting-started.md#6-ask-why-it-is-where-it-is).

## Branches and merges

- Two branches that write **different** artifacts change different files, so they merge cleanly.
- Two branches that write the **same** artifact both change its `revision:` line, so Git reports a
  conflict and a person resolves it. After the merge, `validate` checks that the resolved
  `transitions` still end at the resolved `status`. If one side's move was lost, re-apply it through
  the CLI rather than editing the lines.
- Evidence files are named by instant and content digest, so two branches recording evidence
  about the same artifact add different files.

## Hand edits

The files are meant to be read and reviewed, and some fields are yours to edit: the body, `title`,
`summary`, `owner`, `tags`, `relations`, and front-matter keys your lifecycle reads, such as a `due`
date. `status`, `revision` and `transitions` belong to the CLI. A hand edit to `status` that
disagrees with the last transition is reported by `validate`:

```text
planning.transitions: `status: archived` and the last transition, which moved to `implemented`, disagree (hint: a status is changed by a move, which appends the transition that says so; re-apply the move through the CLI rather than editing either line)
```

Relations are the one structural field meant for hand editing: an edge to another repository in a
[workspace](./workspaces.md) can only be written by hand.

## What `validate` checks

`aep plan artifact validate` reads every artifact and evidence file and reports, in one list:

- a file that cannot be read, or whose path does not match its id;
- an edge to an artifact the plan does not hold, a cycle, or a duplicate id;
- a status the kind's lifecycle does not declare;
- transitions that are not continuous, or that do not end at `status`.

It also **reports without failing**: statuses reached on an assertion rather than a record,
`review-result`s that state their findings only as prose, and reviews older than
`--outcome-within` days (14 by default) that no `review_outcome` names. `--strict` turns these into
exit `1`. `validate` exits `1` on any problem, which makes it a CI gate on its own. See
[Validate the plan in CI](../guides/validate-in-ci.md).

## Packing closed work

A long-lived store is mostly closed work: a story at `implemented`, `rejected` or `archived`, an
archived `review-result`, and the evidence recorded against each. `aep plan store pack` moves those
files into `aep.pack/1` files under `.engineering/packs/` and sets `project.yaml` to
`aep.project/6`. Nothing is deleted: a pack holds each file's bytes verbatim, and Git keeps the
history. Only committed bytes are packed; a file that is untracked or changed since `HEAD` stays
loose and the command names it.

```bash
aep plan store pack --dry-run      # what would move, the pack name, file counts before and after
aep plan store pack                # write the pack, remove the loose files
git add -A && git commit -m "pack closed artifacts"
aep plan store unpack story:login  # restore one artifact and its evidence
aep plan store unpack --all        # restore everything; the store returns to aep.project/5
```

`validate`, `list`, `graph`, `history`, `explain` and `findings` print the same output and exit with
the same codes before packing, after it and after unpacking; only the number of tracked files
changes. An open artifact keeps its edges to a packed one. A command that would rewrite a packed
artifact (`move`, `body`, `set`, `relate`, `redact`) refuses and names the pack: unpack the
artifact first. `new` refuses a packed id.

`pack` keeps an evidence file loose when it is not a `.json` file, when a Git `filter` or `ident`
attribute applies to it, when its mode is not 100644, or when it is a symbolic link, and says why.

Every read verb verifies before it answers. It refuses a store where a path is held twice with
different bytes, naming both places, and a pack whose record digest or name does not match its
content, naming the pack. Never edit a pack by hand. `aep plan store unpack <id>` resolves a
doubly held path (`--from <pack>` when two packs disagree). `validate` also checks that the
paths are the store's own, that each record matches what Git committed, that only closed artifacts
are packed, and that the project version agrees with the packs present.

After a merge that leaves packs in an `aep.project/5` store, `validate` refuses it until
`aep plan store pack` or `aep plan store unpack` has run; both set the matching version. Both
refuse a symbolic link at `.engineering`, `planning`, `evidence` or `packs`.

**Merging.** Branches that pack different artifacts write different packs and merge cleanly. Two
branches that each unpack from the same pack conflict under Git's default rename detection, which
pairs each rewritten pack with the deleted one. Keep both new packs and remove the old one, or merge
with `-c merge.renames=false`. If a merge leaves a path both loose and packed with different bytes,
`validate` names both places and `aep plan store unpack <id>` resolves it.

**Older aep.** A build that does not know `aep.project/6` refuses the store by name instead of
reading it as `/5`. A store that never packs stays `aep.project/5`.

## Concurrency

Writes in one checkout take a writer lock. Inside a Git repository the lock files live under the
Git common directory (`<git-common-dir>/aep/`), which every linked worktree shares, so they never
appear in the working tree.

## Older stores

| `project.yaml` says | This release |
|---|---|
| `aep.project/5` | reads and writes it |
| `aep.project/6` | reads and writes it: a `/5` store whose closed work `aep plan store pack` moved into packs |
| `aep.project/1`, or no `version:` | refuses it and names `aep plan store migrate git --verify`, which this release runs |
| no `project.yaml`, a `planning/` directory beside it | opens it as a Git-native store (`planning/` and `evidence/`); refuses it and names the migration while it still holds a `/1` `journal.jsonl` |
| `aep.project/2`, `/3`, `/4` (event-log stores) | refuses it and names the migration path |

See [Migrate an older store](../guides/migrate-an-older-store.md). The design record is
[git-native-planning-store-v0.1.md](https://github.com/beyond10x/aep/blob/main/docs/design/git-native-planning-store-v0.1.md).

[Planning stores](../reference/planning-stores.md) lists every supported and removed backend with
its migration path.
