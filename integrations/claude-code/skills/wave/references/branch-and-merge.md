# Branch and merge conventions for a wave

Read this before running one. Until it was written here, every rule below existed only in `git log`
— which meant each wave rediscovered it, and a reader of the repository could not find it at all.

## The names

| Kind | Form | Example |
|---|---|---|
| the wave's integration branch | `wave/<name>` | `wave/five` |
| one unit of work | `impl/<story-slug>` | `impl/blocker-relation` |
| a run of the driver, not a unit of work | `run/<name>` | `run/b1-native` |

The slug is the story's own — the id's name part, unchanged. A branch whose name a reader cannot
join back to a story is a branch nobody can audit later.

**These prefixes are this repository's** — `impl/<story-slug>` and `wave/<name>`, binding in
`AGENTS.md` § *Branches and waves*. An adopting repository may use others; the harness this skill
ships in names a unit branch `wt/*`. Whichever it uses, **the teardown glob in `SKILL.md` is derived
from the prefixes named in this table**, and the two move together. A glob that matches nothing
deletes nothing and reports a clean sweep, which is how eight merged branches were left standing
after a wave that ran its cleanup step (`9c286ad7#155`).

## The shape

Every `impl/*` branch forks from the **same base**: the wave branch at the moment the wave was
picked up. They are merged back serially. That base is what makes the gate run at the end meaningful
— every unit was written against one tree, so the merged result is the only thing that has ever held
all of them at once, and it is the thing that gets gated.

```
main ──┬─────────────────────────────────────── merge wave/five ──▶
       └── wave/five ──┬── impl/a ──┐
                       ├── impl/b ──┼──▶ merged serially, then gated once
                       ├── impl/c ──┤
                       └── impl/d ──┘
```

When the base moves under a long wave, merge `main` into the wave branch and say so in the subject —
`Merge main into wave/five: <what it brought> (<sha>)` — or rebase the unit branch and record the
commit it was rebased onto. Either is fine; leaving it unsaid is not.

## When two units need one file

The rule is one agent, one surface. When no disjoint pair is left — every ready story collides with
every other — the fallback is to split **one file by function** between two units. It held once, on
`xtask/src/bundle.rs` across two agents, and it held because of three things, none of them optional:

1. **The brief declares the split by symbol and line range** — `fn routes_of`, `bundle.rs:381-560`
   — not "do not collide". A range is checkable; an instruction to be careful is not.
2. **Each agent proves compliance by pasting its diff hunk headers** into its report: `@@ -381,6
   +381,10 @@`, `@@ -568,6 +572,285 @@`. The coordinator checks those against the ranges it issued
   without reading a line of the diff.
3. **The merge is dry-run before the first unit lands** — not at integration time, when both
   agents' work is already spent.

The dry run is the step that earns its place, because it catches what a clean textual merge hides.
Two agents editing disjoint hunks produce no conflict, and one of them may have renamed a symbol the
other still calls: the merged file then holds a caller of a function that no longer exists. Neither
agent can see it, because each one's own tree still builds.

```console
$ git merge-tree --write-tree --merge-base=<base> <a> <b>
bf50416acd4c1a8b147d78fb0aae19a0c37e88c8
$ git grep -n 'routes_of' bf50416 -- xtask/src/bundle.rs
```

Exit 0 and a tree oid mean the merge is textually clean. The tree is then **read**, not trusted:
`git grep <symbol> <tree-oid>` resolves against the written tree with no checkout and no build. On
the fixture this text was checked against, the dry run exited 0 and the merged blob held
`fn route_ids` on line 1 and a call to `routes_of()` on line 42 — the rename and the stale caller,
both present, no conflict raised. One command, and it needs neither worktree nor build.

**`--merge-base` is a flag.** The three-argument `git merge-tree <base> <a> <b>` is the old
trivial-merge form and rejects `--write-tree` with a usage error (exit 129, git 2.55.0).

## The messages

**A merge subject names the branch and then what the work *did*** — not what the branch was called
twice:

```
Merge impl/blocker-relation: a blocker is typed by what would clear it, and a
parked item stops looking like a moving one
```

The wave is bracketed by two store commits, and they are the pair a reader looks for:

```
chore(store): wave of five picked up — five stories active, each serving an objective
chore(store): wave of five implemented on the gate's record, and one finding filed
```

**The opening commit carries the selection and its reasoning**: the pool it was chosen from, the
three properties it was chosen on, one line per story naming the objective it serves and what it
does, and the statement that each is implemented on its own branch and closed on the gate's record.
That commit is the only durable record of *why these five and not five others*.

**The closing commit carries the evidence**: the gate run that closed them — its step count, suite
count, test count and exit status — the commit that evidence was recorded against, and anything
found along the way that was outside the wave and got filed rather than fixed.

**Write the merge message to a file — `git merge -F -` does not read stdin.** Unlike `git commit`,
`git merge` does not take `-` as standard input: it answers `error: could not read file '-'` and
exits 129 (git 2.55.0), so a heredoc piped into it loses the whole message. Two commands, not one:

```console
$ printf '%s\n' 'Merge impl/blocker-relation: a blocker is typed by what would clear it' \
    > <scratch>/merge-msg.txt
$ git merge --no-ff impl/blocker-relation -F <scratch>/merge-msg.txt
```

The file goes in the unit's scratch directory, never `/tmp`. The same file answers the other hazard
`AGENTS.md` § *Commits* names: a message containing backticks, passed as `-m "…"`, is command
substitution rather than text.

**The review base for a unit is `git diff <base>...<head>` — three dots, always.** `<base>` is the
integration branch, `<head>` the unit branch, and three dots diffs from the **merge base**: the fork
point the unit was actually written against, which does not move. Two dots compares the two tips, so
the moment another unit merges and the integration branch advances, the diff attributes that unit's
work to this one — as removals of files it never opened. On a two-branch fixture, after one other
unit landed, `git diff wave..u` reported `mine.txt | 1 +` **and** `shared.txt | 1 -`, while
`git diff wave...u` reported `mine.txt | 1 +` alone. Two agents lost time to this in one wave and
one filed it as a possible scope violation before working it out (`114c2340#188`). Put the three-dot
form in the brief; do not leave the agent to derive it.

## One gate, one record

The gate runs **once**, on the merged result, and one test result recorded against that merge
commit closes every story in the wave. This is a real rule with a real consequence: a story's
evidence names a commit that contains the other four units too. That is correct — it is the tree
that was actually gated — and a per-unit gate run would be evidence about a tree that was never
shipped.

## What a wave leaves behind

| | where | survives the wave? |
|---|---|---|
| the selection and its reasoning | the opening `chore(store)` commit | yes |
| the plan | a page under `docs/plan/` | yes |
| the units | `impl/*` branches | **no** — deleted with `git branch -d` once merged; the evidence cites commits, which live on the base |
| the gate's verdict | the closing commit, and the store's evidence record | yes |
| **a driven run's records** | the run directory, which is **gitignored** | **no** |
| **the unit's triple** — the worktree | one per unit | **no** — the coordinator removes it at the end |
| — its build directory | one per worktree, **inside it** | **no** — it goes down with the tree here; tens of GB per wave |
| — its scratch directory | one per agent, **assigned by the coordinator** under the wave root | **no** — removed with the other two |

A merged unit branch holds no commit the base does not, so deleting it loses nothing that the
evidence points at — an evidence record naming a commit resolves after the branch is gone, because
the commit is reachable from the base. Use `git branch -d`, never `-D`: `-d` refuses an unmerged
branch, and an unmerged branch means a unit that left the wave or work that never landed. Keep that
one and name it in the closing report.

Leaving merged branches is how a repository grows a `wt/*` list nobody can read — two waves put ten
on one repository here, every one merged, none of them telling you which wave it came from.

**The triple is one record, and it is the record that gets skipped.** `git worktree list` in the
next wave's pre-flight refuses rather than cleans, so a wave that leaves its trees standing pays
nothing and the wave after it pays everything. Write all three paths down as you create them, named
after the unit: that record is the cleanup step's only input, and after a compaction it is the
coordinator's only memory of what it made. Teardown removes all three.

**The build directory belongs inside the worktree.** In this repository that is not a preference:
`cargo xtask` bakes the repository root in at build time, and
`crates/protocol-cli/tests/store_selection.rs` asserts `CARGO_TARGET_TMPDIR` lies under the
repository root, so eleven tests fail whenever the target is elsewhere (`AGENTS.md` § *Gate*, "Two
worktrees must not share a `CARGO_TARGET_DIR`"). Each worktree builds into its own `target/`;
`CARGO_TARGET_DIR` is not set, and never points two trees at one directory. This row read "usually
outside it" until 2026-08-30, a session followed it, and the correction cost a gate run
(`2e81f991#199`) — which is what a reference costs when it is wrong. Where an adopting repository
does put build output outside the tree, it survives `git worktree remove` untouched and `git` will
not find it for you: one wave here left a **16 GB** build directory standing, every worktree it
belonged to long since removed.

**The scratch directory is assigned, not chosen.** The coordinator gives each agent a path under the
wave root and names it in the brief ([unit-brief.md](unit-brief.md)). An agent given none does not
stop writing — it picks somewhere, and the somewhere is outside the wave: one implementor wrote a
rendered bundle tree into `~/.cache/pty-probe/`, another fifteen files into `/tmp` (`9c286ad7`). Two
scratch directories that *were* under the wave root measured 95 MB and 17 MB. The agent files
already require every path written outside the worktree to be reported; that report is worth nothing
if the record it feeds has no field to hold it.

None of the three goes away with `--force`: a worktree that refuses to go is holding uncommitted
work, and that is a finding for the operator, not an obstacle to clear.

The driven-run row is the one that bites. Run artefacts are per-worktree and untracked, so anything a
wave learned from them has to be copied out — into the wave page, the closing commit, or a story —
before the worktree goes. A worktree removed with its records unread takes the whole account of
what happened with it.
