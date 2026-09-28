# A Git-native planning store v0.1

Status: accepted by the operator, 2026-09-28. It supersedes the storage half of
`planning-on-entity-runtime-v0.1.md` (the `eventlog-tree` authority, `aep.project/3` and `/4`) for
AEP's planning store. It keeps that design's typed lifecycles: `entity-core` still decides every
move. Entity Runtime and Eventlog stay the storage for runtime systems that need an event log;
AEP stops using them to store plans.

Sources:

- The measurement of 2026-09-27 on a copy of a downstream `aep.project/4` store ("EKR copy":
  199 planning artifacts, 20,083 tracked files, `eventlog-tree/1`, aep 0.61.1), recorded in
  `story:store-commands-cost-what-changed`.
- AEP's own store at `origin/main` `8f3234d0` (331 artifacts).
- Beads (`github.com/gastownhall/beads`), git-bug (`github.com/git-bug/git-bug`,
  `doc/design/data-model.md`) and planning-with-files (`github.com/othmanadi/planning-with-files`),
  read 2026-09-28.

## 0. Decision in one table

| layer | `aep.project/4` today | `aep.project/5` after |
|---|---|---|
| authority | `eventlog-tree` event, group and blob files under `.engineering/state/` | **the Markdown files under `.engineering/planning/`**, one per artifact |
| the Markdown files | a projection, re-rendered in full on every write | the authority; a write changes the files it names and no other |
| history | Entity Runtime records in Eventlog streams | Git history of each file, plus a `transitions` list the CLI appends to on every move |
| audit (actor, executor, correlation, causation) | `aep.audit` and `aep.applied` entities, 2 per command | the transition entry, and commit trailers for edits that are not moves |
| evidence | ER observations on the artifact | one immutable file per evidence record, `.engineering/evidence/<kind>/<name>/<instant>-<digest>.json` |
| lifecycle decision | `entity-core` over definitions registered in the store | `entity-core` over definitions built from the protocol tree, unchanged |
| query | complete snapshot rebuilt per command | read the artifact files (§ 7); a derived index only if the bench misses its target |
| integrity | four layers (Git, event digests, protocol snapshot manifest, legacy-boundary joins), each re-checked in full per command | Git, plus a validator that checks the changed files once: at write, in the pre-commit hook and in CI |
| files per move | 6 groups, 9 events, 33 blobs, ~8 content blobs (EKR commit `f5e01f7a`) | 1 |

## 1. Why the current store is wrong

### 1.1 Measured (EKR copy, CPU user time, no `nice`)

| command | aep 0.61.1 | peak RSS | target of the design it failed (`planning-on-entity-runtime-v0.1.md` § 10) |
|---|---|---|---|
| `plan artifact list` | 4.3–7.5 s | 785 MB | < 1 s at 1,000 artifacts |
| `plan artifact new task` | 13–18.5 s | 1.1 GB | < 2 s |
| `plan artifact validate` | 17.8–25 s | 2.2 GB | < 10 s at 1,000 × 10 events |
| `new review-result --from --findings` | 24.8 s | 1.27 GB | < 2 s |

The store holds 199 artifacts. At 1,000 the costs grow with it: every item below is linear in the
store.

### 1.2 Causes

| # | cause | evidence |
|---|---|---|
| C1 | the complete store snapshot is rebuilt and every content reference resolved on each command | a write opened 32,913 state files and 4,101 content blobs (`strace -e openat`) |
| C2 | legacy-boundary validation re-digests every imported evidence blob on open and on every recorded command | `aep-backend-eventlog/src/lib.rs:2290`; 1,462 evidence blobs in the EKR copy |
| C3 | the whole authority is canonicalised and digested per write | `aep_planning_migration::durable::authority_snapshot_identity`, 9.6% of a write (LBR profile) |
| C4 | all 212 projected files are rewritten per write, byte-identical | mtimes of all 212 moved; `git status` shows 0 modified |
| C5 | the protocol snapshot (14,669 files, 162 MB) was verified ~3× per command | fixed in aep#51, once per process |
| C6 | bookkeeping outweighs content | 199 artifacts; 1,461 relation, metadata, audit, applied and invocation subjects; 2,924 migration subjects |
| C7 | each event is a chain of four files plus a batch blob repeating them | pointer event → recorded-entry blob → record blob → request blob, plus `er.batch` (30 KB per move) |

C1–C4 are what each layer did to prove a store it had not written itself was intact. Git already
proves that for committed bytes: every file is content-addressed by its blob id and every commit
by its tree. The event log re-proves it underneath, per command, from scratch.

### 1.3 Other tools

| tool | one edit writes | read path | source |
|---|---|---|---|
| Beads | cell-level rows in Dolt (the authority); optional `issues.jsonl` export | Dolt tables | Beads README |
| git-bug | 3–4 Git objects (op-pack blob, tree, commit) under `refs/<namespace>/<id>`, nothing in the working tree | "a matter of milliseconds" (README claim, not measured here) | `doc/design/data-model.md` |
| planning-with-files | one Markdown file | read the file | README |

None of them keeps a second event log beside Git.

## 2. Requirements

What `planning-on-entity-runtime-v0.1.md` § 2 required, and what happens to each:

| id | requirement | here |
|---|---|---|
| R1 | typed fields and a lifecycle per kind; an invalid field or illegal move is refused | **kept**: the front-matter schema per kind (§ 4), `entity-core` decides moves (§ 5) |
| R2 | refused commands are recorded | **dropped**. A refusal changes nothing (AGENTS.md invariant 7) and is printed to the caller. A caller that needs a record of refusals keeps its own; the store records what happened to the plan |
| R3 | two branches writing different artifacts merge with GitHub's button | **kept**, trivially: different files |
| R4 | two branches writing the same artifact: a fork is detected | **kept**: every write bumps `revision:`, so the same line changes on both sides and Git reports a conflict; the validator refuses a resolution whose transitions are not a legal walk ending in `status` (§ 4.3) |
| R5 | no store-wide file changes on a write | **kept**: no journal, no ownership file, no manifest |
| R6 | the conformance suites pass | **kept**: the new backend runs `aep-conformance` |
| R7 | `list` < 1 s, write < 2 s at 1,000 artifacts | **tightened** to § 9 |
| R8 | an unchanged store hashes 0 bytes | **kept**: nothing is hashed on read |
| R9 | `validate` is a required check | **kept**: `planning validate` stays required in each adopter |
| R10 | each store is exported once with a fidelity check | **kept** for `/4` → `/5` (§ 8) |
| R11 | no absolute home path in committed bytes | **kept** |
| R12 | moving to SQLite or Postgres needs no schema change | **unchanged**: the SQLite and Postgres backends stay as they are; they are not part of this store |

New:

| id | requirement | check |
|---|---|---|
| N1 | one move or edit writes exactly the artifact's file, plus one evidence file when evidence is recorded | file-count test per verb |
| N2 | the answer to "when did X enter status S, by whom, with what evidence" does not depend on how moves were grouped into commits | `transitions` list (§ 4.2); test: three moves in one commit report three transitions |
| N3 | reading needs nothing but the committed files | a fresh clone answers every query with no cache present |
| N4 | every committed planning byte is a file a person can read and review in a pull request | no binary, no digest-named file under `.engineering/planning/` |

## 3. Files

```
.engineering/
  project.yaml                               aep.project/5
  planning/<kind>/<name>.md                  one artifact: YAML front matter + Markdown body
  evidence/<kind>/<name>/<instant>-<digest>.json   one evidence record, never edited
```

Nothing else. `state/`, `blobs/`, `journal.jsonl` and `.aep-projection-ownership.json` are removed
by the cutover (§ 8).

`project.yaml` for `/5`:

```yaml
version: aep.project/5
planning_scope: aep
protocol: adp/1
profile: development.standard
protocols: ..
store:
  git: {}          # authority = .engineering/planning; no further fields in /5
```

## 4. The artifact file (`aep.planning-md/3`)

### 4.1 Layout

````markdown
---
format: aep.planning-md/3
id: story:store-commands-cost-what-changed
kind: story
title: A planning-store command costs what changed since the last verified head
status: proposed
revision: 3
relations:
  - implements: epic:git-native-planning-store
transitions:
  - {at: 2026-09-28T10:04:11Z, actor: "human:timo", revision: 2, from: draft, to: proposed}
---
## Problem

Prose.

```acceptance
[{"scenario": "a write on 1,000 artifacts costs under 200 ms"}]
```
````

### 4.2 Front matter

| field | owner | rule |
|---|---|---|
| `format` | CLI | exactly `aep.planning-md/3` |
| `id` | CLI | `<kind>:<name>`, equal to the path |
| `kind` | CLI | a kind the protocol tree declares |
| `title` | author | non-empty |
| `status` | CLI | a state of the kind's lifecycle; equals the last transition's `to`, or the initial state when there is none |
| `revision` | CLI | ≥ 1; increments by 1 on every CLI write to this file |
| `relations` | CLI (`relate`) | list of `{<relation>: <id>}`, relation names from the protocol tree |
| `transitions` | CLI (`move`) | append-only list; each entry `{at, actor, revision, from, to, decided_on?}`; `decided_on` is the recorded and asserted evidence counts the move rested on |
| kind-specific fields | per kind | declared by the kind's schema in the protocol tree; unknown keys refused |

The CLI writes the front matter in this key order, one transition per line, so a diff of a move is
one changed `status:` line, one changed `revision:` line and one added transition line.

A hand edit is allowed to `title`, kind-specific author fields and the body. A hand edit to a
CLI-owned field is refused by the validator (§ 6), except `relations`, which a person may edit and
the validator checks against the protocol tree.

### 4.3 Revision rule

- `revision` is 1 plus the number of CLI writes to the file.
- Two branches that each write the same artifact both change the `revision:` line, so Git reports
  a conflict (R4). Different artifacts never conflict.
- After any merge the validator checks:
  - `transitions` is a legal walk of the kind's lifecycle from its initial state;
  - `status` equals the last transition's `to`;
  - `revision ≥ len(transitions) + 1`.
- A resolution that keeps both sides' transitions in an illegal order, or drops one side's and
  leaves `status` from the other, is refused. The resolver re-applies the lost move through the
  CLI.

### 4.4 Fenced blocks

A fenced code block whose info string is a registered block type is data, not prose.

| block | schema | today |
|---|---|---|
| `findings` | the existing findings schema (`aep-backend-markdown/src/findings.rs`) | already parsed and validated |
| `acceptance` | `[{scenario, check?}]` | new |
| `scope` | `{crates?, paths?, symbols?}` | new; replaces the Scope section `story-scoper` writes as prose |

Rules: a block type is registered by the protocol tree; an unregistered info string is prose; a
registered block that fails its schema refuses the file; at most one block of each type per
artifact unless the type says otherwise.

## 5. Writing

1. Take the planning writer lock (`.engineering/.aep-planning-writer*.lock`, per checkout).
2. Read the one artifact file (and, for a move, the evidence it names).
3. Decide with `entity-core`: the kind's lifecycle definition, built from the protocol tree as
   `aep-backend-eventlog/src/typed.rs` builds it today, over the artifact's current state. A refusal
   prints the legal moves and writes nothing.
4. Render the new file; write it to `<name>.md.tmp` in the same directory; `fsync`; `rename`.
5. Nothing else: no other file is read or written.

No step reads another artifact, except `relate` and `move` reading the target ids they name to
check they exist.

Evidence (`aep plan artifact evidence`): write one `<instant>-<digest>.json` under
`.engineering/evidence/<kind>/<name>/`, schema by evidence kind, then, if the evidence is attached
to a move, name its ULID in the transition's `evidence` field. The file is never rewritten; the
validator refuses a changed evidence file by comparing it with its committed Git blob.

Immutable kinds (`review-result`): the validator refuses any change after the creating commit,
checked the same way.

## 6. Validation

| where | what it reads | what it checks |
|---|---|---|
| each CLI write | the one file | schema, lifecycle legality, relations resolve |
| `aep plan artifact validate` | every artifact file and evidence file | the same, plus immutability of evidence and immutable kinds against Git |
| `validate --full` | every file | the same; used in CI |
| pre-commit hook (`aep plan store install-hooks`) | staged planning files | the same as `validate` |
| CI `planning validate` (required) | the PR's changed files, then `--full` | the same |

The protocol snapshot is verified once per process (aep#51).

## 7. Reading

A command reads the artifact files it needs; `list`, `validate` and graph queries read all of them.
AEP's own store is 343 files and 2.9 MB, so no index is built in this cut. A derived SQLite index
under `.engineering/.cache/` (gitignored, keyed by the Git tree id of `.engineering/planning`) is
added only if the bench in § 9 misses its target.

### 7.1 History

| question | answered from |
|---|---|
| status changes, with actor and the evidence counts they rested on | `transitions` (exact, independent of commits: N2) |
| evidence recorded about an artifact | the files under `.engineering/evidence/<kind>/<name>/` |
| every version of the file | `git log --follow -- <path>`, each read with `git show` |
| who edited the body or relations | the commit author of each version |

`aep plan artifact history <id>` lists transitions and evidence records in the journal's entry
vocabulary (`moved`, `evidence`), so `explain` and the evidence gate read them unchanged.

### 7.2 Across repositories

A workspace index derived from each member's files, keyed by `(repository, tree, id)`, is future
work and not part of this design's releases.

## 8. Migration from `aep.project/4`

`aep plan store migrate git` (one verb, once per store):

1. Refuse unless the working tree of `.engineering/` is clean.
2. Read the `/4` store once through the current backend (this costs what a `validate` costs today).
3. For each artifact, write `aep.planning-md/3`: the current projection's front matter and body,
   plus `transitions` rebuilt from the artifact's recorded moves (actor, executor, instant,
   correlation, evidence ids), and `revision` set to its current revision.
4. Write each evidence observation as an evidence file; the transition entries name them.
5. Write `project.yaml` as `/5`.
6. Remove `state/`, `blobs/` and the projection ownership file.
7. Fidelity check (`--verify`): for every artifact, status, revision, relations, body and the
   ordered list of transitions equal what the `/4` backend answers.

The `/4` history (migration evidence, audit subjects, invocations) is not carried forward. It stays
readable at the last `/4` commit: `git show <commit>:.engineering/state/...`. The migration commit
message names that commit.

Adopter stores to migrate: aep, eventlog, entity-runtime, ess, connectors, service-sdk,
epistemic-knowledge-runtime. The migration is one PR per repository, bot-authored, after the
release that carries the verb.

## 9. Performance targets

Release build, fresh process, median of 5, `cargo xtask bench planning` over a generated store of
1,000 artifacts × 10 transitions, on a Git repository.

| operation | target |
|---|---|
| `list` | < 500 ms |
| `move` / `new` / `body` / `relate` | < 200 ms; 1 file written (N1) |
| `validate` after one write | < 200 ms |
| `validate --full` | < 2 s |
| peak RSS, any command | < 100 MB |

## 10. What is removed from AEP

In the release that follows AEP's own migration (§ 13, B1). The Eventlog and Entity Runtime
repositories are not changed; AEP stops depending on their event-log crates.

| crate or module | reason |
|---|---|
| `aep-backend-eventlog` | the `/2`–`/4` authorities |
| `aep-planning-migration` | the `/2` → `/3` import, legacy boundaries, projection publisher |
| AEP's dependencies on `entity-eventlog`, `eventlog-core`, `eventlog-file`, `eventlog-tree` | no longer used |
| `aep plan store` verbs (`inspect`, `migrate`, `verify`, `rebuild`, `init-tree`, `export`, `install-hooks`, `writer-control`), including `migrate git` | they operate on the event-log store; adopters migrate with the commit that merged A (§ 13, R2) |
| `artifact resolve`, `artifact render` | fork resolution and projection rendering of the tree store |

Kept: `entity-core` (it decides lifecycles, invariant 9), `aep-backend-markdown` (the `/1` layout
and the `/5` layout), `aep-backend-hybrid`, `aep-backend-sqlite`, `aep-backend-postgres`.

## 11. Cross-repository effects

- Atlas ADR 0063 recorded AEP planning on Entity Runtime over Eventlog. A new Atlas ADR supersedes
  its unit P1 for AEP's planning store. Entity Runtime and Eventlog are unaffected.
- `ESS-EVOLUTION.md` steps 3–5 describe AEP producing ER definitions and Eventlog persisting them.
  AEP still produces ER definitions (for `entity-core` decisions); the persistence step no longer
  applies to planning. The ESS evolution owner gets a note.
- Agent plugins (`aep:planning`, `aep:implementing`) describe CLI verbs, which do not change. Text
  that describes `state/` or the projection is updated in `agentplugins`.

## 12. Test obligations

- `aep-conformance` at every level over the `/5` backend (N3).
- File-count test per verb (N1).
- Three moves in one commit → three transitions in `history` (N2).
- Merge matrix: two branches move the same artifact → Git conflict; different artifacts → clean;
  a resolution that drops a transition → `validate` refuses.
- Evidence file edited after commit → refused. Review result edited after commit → refused.
- Hand edit of `status` → refused; hand edit of the body → accepted.
- Migration fidelity on a copy of each adopter store (§ 8 step 7).
- Bench (§ 9) as a gate step that fails over the targets.

## 13. Order of work and progress

The plan is tracked in this file, not in a planning store: the old store is the thing being
replaced, and the new one does not exist until step A6.

| step | content | state |
|---|---|---|
| 0 | merge aep#51 (protocol snapshot verified once per process) | done 2026-09-28, `00465378` |
| A2 | `aep.project/5` config (`StoreConfig::Git`); `aep.planning-md/3` with `transitions` | done |
| A3 | the Markdown backend's Git layout: moves into `transitions`, evidence into files, no journal; hydration made linear (an undo record instead of a store copy per command) | done |
| A4 | `aep plan store migrate git [--verify]`, reading the `/3`–`/4` store once; imported moves carry `imported: true` | done |
| A5 | `planning validate` CI builds from the tree; `xtask` release check reads evidence files | done |
| A6 | AEP's own store migrated to `/5`: 333 artifacts, 530 transitions, 254 evidence files, `--verify` equal | done 2026-09-28 |
| B1 | Eventlog removed from AEP: `aep-backend-eventlog`, `aep-planning-migration`, the `plan store` verbs, the eventlog dependencies. The Eventlog and Entity Runtime repositories are not changed | pending |
| R1 | release 0.62.0, after A6 and B1 | pending |
| R2 | other adopters (eventlog, entity-runtime, ess, connectors, service-sdk, epistemic-knowledge-runtime) migrate with the commit that merged A, then install 0.62.0 | pending, theirs |

### 13.1 Measured after migration (2026-09-28, release build, CPU user time)

| command | EKR store, `/4`, aep 0.61.1 | EKR store migrated to `/5` | AEP store migrated to `/5` |
|---|---|---|---|
| `list` | 4.3–7.5 s | 0.42 s | 0.02 s |
| `validate` | 17.8–25 s | 0.44 s | 0.05 s |
| `new task` | 13–18.5 s | 1.02 s (before the hydration fix) | 0.05 s |
| peak RSS | up to 2.2 GB | 57 MB | 35 MB |

The EKR `/5` figures were taken before the hydration fix; its `list` includes one verification of
its Git-pinned protocol snapshot.

## 14. Not in this design

- Sharing plans between repositories as an authority.
- Signing individual transitions. Commits are bot-authored; commit signing is Gates' concern.
- Carrying the `/4` bookkeeping history forward (§ 8).
