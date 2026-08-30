# The unit brief

One file per unit, written by the coordinator at dispatch and passed **by path** — not pasted into
the prompt. Before it, the invariants were retyped into every message: six briefs in one wave, some
forty lines of them each, and nothing detects the one that was left out, because an agent that is
not told about a file simply does not know it exists. A brief on disk also survives a compaction, so
a coordinator that lost its context can read what it told each agent.

Copy this template, fill the `<placeholders>`, write it into the unit's scratch directory, and give
the agent the path.

## Identity

```
story:  <story-id>              the id, not a paraphrase of it
branch: impl/<story-slug>       forked from <integration-branch>
base:   <base-sha>              a sha, not a branch name — a branch name moves under you
```

Review your own change with `git diff <base>...<head>` — three dots, for the reason in
[branch-and-merge.md](branch-and-merge.md).

## The triple

```
worktree:  <wave-root>/<unit>/tree           work only here; never the sibling trees
build dir: <wave-root>/<unit>/tree/target    inside the worktree
scratch:   <wave-root>/<unit>/scratch        everything uncommitted and untracked; never /tmp
```

All three are the coordinator's to create and to remove. Any path written outside them goes in the
report header.

## The file assignment

| | |
|---|---|
| **yours** | `<path>`, `<path>` — edit freely |
| **shared, split by symbol** | in `<path>`: `<symbol>`, lines `<first>`–`<last>`, is yours; the rest of the file is another unit's. Paste your diff hunk headers in the report so the ranges can be checked |
| **not yours** | `<path>` — the coordinator's. If you need a change there, write the patch to `<scratch>/<name>.patch` and name it in the report. Do not edit it |

## Repository invariants

Filled in **once per repository** and copied unchanged into every brief; an adopting repository
replaces this list wholesale. This repository's, as the worked example:

* each worktree builds into its own `target/` **inside the worktree** — never set
  `CARGO_TARGET_DIR`, never point two trees at one build directory
* `cargo fmt -p <crate>`, never `cargo fmt --all`
* never write under `.engineering/planning/`, and no `protocol artifact` write verb — reads
  (`show`, `list`, `--help`) are fine
* the gate is **package-scoped** here; the full gate runs once, later, on the integration branch
* no `git worktree` command, and no `git commit`, `git add`, `git stash` or branch command —
  changes are left in the working tree
* nothing under `/tmp`

## The gate

```
<gate-command>
```

Run it before reporting, and quote its summary line in the report verbatim.

## The report header

Six lines, before any prose:

```
unit: <story-id>
verdict: <green|red|blocked>
cases: executed <before>→<after>, red <n>
origin: <the brief or finding this round answers>
wrote-outside-worktree: <paths, or none>
needs-coordinator: <what you could not do and who owns it, or none>
```

## A correction round

A correction brief is **not** the brief again. It carries only what changed:

1. the path of the original brief, still authoritative and unchanged;
2. the finding table — one row per finding, each with the file, the line, and what is wrong;
3. anything that moved since dispatch: a new base sha, a file that left the assignment.

Re-sending the invariants invites a silent edit to one of them that nobody notices.
