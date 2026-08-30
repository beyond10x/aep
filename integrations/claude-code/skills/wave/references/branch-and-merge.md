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
| the units | `impl/*` branches | yes, and they are not deleted — the evidence cites them |
| the gate's verdict | the closing commit, and the store's evidence record | yes |
| **a driven run's records** | the run directory, which is **gitignored** | **no** |

The last row is the one that bites. Run artefacts are per-worktree and untracked, so anything a
wave learned from them has to be copied out — into the wave page, the closing commit, or a story —
before the worktree goes. A worktree removed with its records unread takes the whole account of
what happened with it.
