---
title: Migrate an older store
sidebar_position: 5
description: Move an aep.project/1 Markdown store, a planning directory with no project file, or an event-log store (aep.project/2–4) to the Git-native aep.project/5 store.
---

# Migrate an older store

The current store is `aep.project/5`, the [Git-native store](../concepts/planning-store.md). Find
your starting point from the `version:` line of `.engineering/project.yaml`:

| You have | This release | What to do |
|---|---|---|
| `version: aep.project/5` | reads and writes it | nothing |
| `version: aep.project/1`, or no `version:` | refuses every other planning command, naming this migration | [migrate with this release](#from-aepproject1) |
| a `.engineering/planning/` directory with a `journal.jsonl` and no `project.yaml` | refuses every other planning command, naming this migration | [migrate with this release](#from-a-planning-directory-with-no-project-file), naming the protocol source and profile |
| `version: aep.project/2`, `/3` or `/4` | refuses every planning command | [migrate with the bridge build](#from-an-event-log-store-aepproject24) |

A `/1` store is no longer opened for reading or writing: this release keeps only the read-only
journal reader the migration needs. `aep doctor` reports these stores as `warn` and names the same
command. A `/1` project whose `store:` is SQLite or PostgreSQL is not migrated by this command:
rewrite its `project.yaml` by hand as `aep.project/5` with a `planning_scope` and
`store: { sqlite: { path: <file> } }` or `store: { postgres: { url: <url> } }`.

## From `aep.project/1`

An `aep.project/1` store keeps Markdown documents plus a `journal.jsonl` of moves and evidence.

1. Commit or stash everything under `.engineering/`. The migration refuses a dirty `.engineering`.
2. Look first:

   ```shell-session
   $ aep plan store migrate git --dry-run
   would write 1 document(s) as `aep.planning-md/3` carrying 2 transition(s), and 1 evidence file(s); planning_scope: old (from the origin remote)
   not carried (Git history holds them): 1 created
   dry run: nothing was written
   ```

3. Migrate and verify:

   ```shell-session
   $ aep plan store migrate git --verify
   writes 1 document(s) as `aep.planning-md/3` carrying 2 transition(s), and 1 evidence file(s); planning_scope: old (from the origin remote)
   not carried (Git history holds them): 1 created
   …/.engineering/project.yaml now selects `aep.project/5` with planning_scope: old (from the origin remote): 1 document(s), 2 transition(s), 1 evidence file(s) written
   removed …/.engineering/planning/journal.jsonl; Git history holds it
   verified 1 artifact(s) and 1 evidence record(s): status, revision, title, relations, body, transitions and every evidence record, field by field and in order, equal what the old store answered
   ```

4. Review the diff, run `aep plan artifact validate`, and commit.

What changes:

- Each document becomes `aep.planning-md/3`. Its journalled moves become its `transitions`, in
  journal order, each marked `imported: true`. Other front-matter keys and the body are kept.
- A document the journal never moved whose status is not its kind's initial state gets one imported
  transition from the initial state to its status (and at least revision 2), so `validate` holds it
  to its transitions however the migration is committed.
- Each journalled evidence record becomes one file under `.engineering/evidence/`. Identical records
  stay separate files, because each counts.
- `project.yaml` gets `version: aep.project/5`, `store: {git: {}}` and `planning_scope` set to the
  repository's name: `--planning-scope` when given, else the `origin` remote's last path segment
  without `.git`, else the primary checkout's directory name (also from a linked worktree), else,
  outside Git, the directory holding `.engineering/`. A Git repository with neither an `origin` nor
  a common directory named `.git` (a bare repository's worktree) is refused, naming
  `--planning-scope`. Every other key is kept.
- `journal.jsonl` and any `.aep-batch.pending.json` are removed. Git history keeps them.

The migration refuses **before writing anything** when a document disagrees with its journal: a
status the last move did not reach, a revision not above its move count, or a journal entry with no
document. Fix the named document and run it again. `--verify` exits non-zero on any difference. It
compares each evidence record field by field and in order, and a difference names the artifact,
the record's position, its file and the differing fields, never their values.

## From a planning directory with no project file

Name the rules the new project file should carry:

{/* generated:release-pin:begin commit=2b840f7cdfc25525b6cbec9b83710c3ca53746c8 — kept by `cargo xtask status` */}
```bash
aep plan store migrate git --verify \
  --protocols git+https://github.com/beyond10x/aep#2b840f7cdfc25525b6cbec9b83710c3ca53746c8 \
  --profile development.standard
```
{/* generated:release-pin:end */}

`--protocol` defaults to `adp/1`. These three flags are refused when a `project.yaml` exists.

## From an event-log store (`aep.project/2`–`/4`)

AEP stopped reading event-log stores in `0.62.0`. Every planning command refuses one and says how to
migrate:

```text
error: project document (…/.engineering/project.yaml) is not valid: [unsupported_protocol_version] project.version: this store is an event-log store (aep.project/4); AEP no longer reads it — migrate it with `cargo install --git https://github.com/beyond10x/aep --rev 9c0f1da44429ff935fa0b2d743457945d51e1c51 aep-cli` then `aep plan store migrate git --verify` (hint: the migration writes an `aep.project/5` store this build reads)
```

The build at commit `9c0f1da4` still reads event-log stores and can write the Git-native one. Install
it somewhere that does not replace your current `aep`, run the migration once, then go back:

```bash
cargo install --locked --root ~/.cache/aep-bridge \
  --git https://github.com/beyond10x/aep --rev 9c0f1da44429ff935fa0b2d743457945d51e1c51 aep-cli
~/.cache/aep-bridge/bin/aep plan store migrate git --verify
aep plan artifact validate          # with the current release
```

That build's migration carries recorded moves into `transitions` (marked `imported`) and recorded
evidence into evidence files. `--verify` compares every artifact with what the old store answered.
The event-log history itself (audit records, invocations, migration evidence) is not carried
forward. It stays readable at the last commit before the migration.

## After any migration

- Remove `journal.jsonl`, `state/` or `blobs/` from any `.gitignore` or tooling that expected them.
- If CI pinned an older `aep` to validate the store, move it to the current release. See
  [Validate the plan in CI](./validate-in-ci.md).
- Commands and flags that only worked on event-log stores are gone: every `aep plan store` verb
  except `migrate git`, `aep plan artifact resolve`, `aep plan artifact render`, and
  `aep plan artifact validate --against`.
