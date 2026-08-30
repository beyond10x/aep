# Engineering Protocols — Claude Code plugin

Teaches Claude Code to plan engineering work in a markdown artifact store governed by this
repository's `protocol` CLI: epics, stories, tasks and initiatives as files under
`.engineering/planning/`, with kinds, statuses and legal moves supplied by validated documents rather
than by convention.

The plugin carries rules. It deliberately carries no vocabulary — no list of kinds, no status ladder,
no relation names. Those are read at use time from `protocol artifact kinds`, `protocol artifact
relations` and `protocol artifact lifecycle <kind>`, because a prose copy of a validated document is
a copy that goes stale, and drift is the thing this project exists to refuse.

**This plugin is a demonstration, not the product.** The product is the specification —
`protocols/`, `principles/`, `workflows/`, `artifacts/` and the schemas generated from them. Every
surface here exists to show that one of those rules can be carried by a real harness, and sometimes
to find the shape of a rule that is not yet written down, so that it can be. A harness that drives
these workflows deterministically reads the documents; it never reads this directory. When you are
deciding whether something belongs here, the question is *which rule does it demonstrate* — not
whether it would be useful.

## What is in it

| Component | What it does |
|---|---|
| `skills/planning/` | the model, the planning-store guardrails, and how to discover the store's vocabulary. Auto-triggers on planning talk or a `.engineering/planning/` directory; also invocable as `/engineering-protocols:planning` |
| `skills/schema-contracts/` | project schema-registry discovery, offline validation, deterministic TypeScript projection, and drift checks from one JSON Schema source |
| `skills/wave/` | run a wave: pick the next set of stories that can be implemented at once, propose it and stop for approval, then dispatch one implementor per story into its own worktree, route each result through the adversary, and merge what goes green onto one integration branch closed by one gate run |
| `agents/decomposer.md` | takes one epic id, drafts the stories that jointly cover it, each with an acceptance statement. Creates drafts only — never moves an artifact, never touches one it did not create |
| `agents/reverse-engineer.md` | drafts the first plan for a repository that has none: reads it through `protocol reverse scan` and `protocol reverse history`, creates draft artifacts that each cite the `path:line` they rest on, and reports what it could not cite as a question rather than filing it as work |
| `agents/plan-reviewer.md` | read-only semantic audit: stories that no longer cover their epic, finished epics still open, stale work, missing acceptance statements. Proposes moves, performs none |
| `agents/implementor.md` | implements one decomposed unit, test first: writes the case, runs it red, makes the smallest change that satisfies it, runs the whole suite. Reports the suite's own output. Never moves an artifact, never writes into the store |
| `agents/adversary.md` | attacks a change that already passes: boundaries, mutants the suite would miss, contract drift, properties. Writes failing cases and may not edit what it attacks; runs no `protocol artifact` command — its findings return in its report, and the coordinator decides what becomes a `review-result`, a story or nothing |
| `agents/story-scoper.md` | works out where one story would actually land — crate, files, symbols, documents — and returns the `## Scope` section that says so, every line marked `cited` or `inferred`. Read-only, so many run at once; the caller writes what they return |

The hooks and the eval that used to live here **migrated to the metaharness repository**
(`epic:metaharness-migration`, 2026-08-22). The hooks' policy is now Rust inside the driver —
`decide_tool` in `crates/protocol-cli/src/drive.rs`, answering the metaharness seam per call —
and the eval machinery, its recorded transcripts and its results live in metaharness under
`evals/engineering-protocols/`. See "Enforcement" below.

## Prerequisite: `protocol` on `PATH`

The plugin is a set of instructions for driving a CLI; without the CLI it does nothing. From a
checkout of this repository:

```console
$ cargo install --path crates/protocol-cli
$ protocol --version
```

The crate is `protocol-cli`; the binary it installs is `protocol`.

## Install

**Local development** — point Claude Code at the plugin directory, from the repository root:

```console
$ claude --plugin-dir ./integrations/claude-code
```

**From GitHub** — add the marketplace defined at the repository root, then install from it:

```
/plugin marketplace add beyond10x/engineering-protocols
/plugin install engineering-protocols@engineering-protocols
```

The marketplace and the plugin share a name; the `@` form is `<plugin>@<marketplace>`.

**Two load paths, and only one of them follows the tree.** A marketplace install is a snapshot at
the commit it was installed from and updates only when `plugin.json`'s `version` moves; a
`--plugin-dir` load reads the checkout. Running both at once lists every skill and agent twice.
Every session recorded between 2026-08-29 and 2026-08-30 loaded the plugin from the checkout while
the marketplace copy sat disabled at `0.1.0`, three agents and one skill behind. The version a
session is running is the `**Skill version …**` line at the top of every skill, which the wave
skill's stage-1 proposal quotes; `cargo xtask plugin` (gate step `plugin-check`) fails when that
line, `plugin.json` and `roster.json` disagree, or when this directory changed since the newest
release tag and the version did not. A skill or agent edited during a session is not what that
session runs until `/reload-plugins`.

## What is deliberately absent

**No `commands/` directory.** The CLI is the command surface. A `/plan-new` slash command wrapping
`protocol artifact new` would be a second place for the verb to live, and the second place is where
the drift starts: a flag gets added to the CLI, the command does not learn about it, and the model
now has two spellings of the same operation with different capabilities. The skill teaches the CLI;
the CLI stays the only definition of what the CLI does.

## Enforcement, and where the hooks went

Earlier versions of this plugin carried two `PreToolUse` shell hooks — `store-integrity.sh` and
`driven-surface.sh` — as the driver's enforcement arm. Both retired on 2026-08-22 under
`epic:metaharness-migration`, and the reasoning that once kept hooks *out* of this plugin is what
retired them: a hook process could not call `Engine::authorize` and wrote its decisions to a side
log the driver folded in late, and a session launched without the plugin ran unenforced while
looking clean (run `W4-2` paid for eight such sessions).

The same rules hold today, one level down and in one place:

| rule | where it lives now | active when |
|---|---|---|
| the planning store has one writer: `Edit`, `Write`, and `NotebookEdit` are denied under `.engineering/planning/**`; prose goes through `protocol artifact body` | the step's own `scope:` in the step map (`drivers/development/default.yaml`), enforced by `declared_write` in `crates/protocol-cli/src/drive.rs` at decision time — and handed to the native arm as `--write-scope`, so both arms read one declaration | every driven `llm` step whose map declares a scope |
| an edit whose text crosses a planning document's closing `---` is refused, whatever the scope allows | `store_integrity` in `crates/protocol-cli/src/drive.rs`, and `protocol drive hook` for the native loop. It stayed in code because it is a judgement about an edit's *text*: no path can decide it | every driven `llm` step |
| a driven shell is one simple invocation of `protocol artifact …` or `protocol trace …` — no pipes, no redirection, no substitution | `driven_surface`, same place | every driven `llm` step |
| a tool no admitted capability renders to is refused naming the state's surface | `decide_tool`, same place — the allowlist that used to ride on `--allowedTools` | every driven `llm` step |

The decisions are `tool.decided` events in the run's own event stream — the transcript the driver
writes — so *denied* and *never attempted* are distinguishable from the record itself, and there
is no `hook-decisions.jsonl` to forget. Outside a driven run the store's protection is what it
always also was: `protocol artifact validate`, which catches an illegal write whether or not any
seam fired.

## Deferred, and why that is no longer a wait

The skill covers planning only — writing down what is to be done. Earlier versions of this file
deferred two further pieces until the workflow driver existed. **The driver now exists**, and its
arrival did not turn them into work items; it answered one of them and moved the other.

* **A `governed-task` skill is not coming, because `protocol drive` is that loop.** What is owed in
  the current state, what evidence satisfies it and when a transition is legal are `aep-engine`
  questions, and the driver asks them. A skill that approximated the same loop from prose would be
  a second, weaker protocol implementation with none of the conformance suites — and the first time
  the two disagreed, the untested one would be the one holding the session.
* **`implementor` and `adversary` now ship, and the reason they did not has been answered rather
  than dropped.** The argument was two sentences and only one of them survived.

  *An implementor has nothing left to learn, because the driver puts the capability policy in force
  per state.* True — **of a driven run**. An interactive session has no drive loop and no per-state
  policy, and that is the session most of this work happens in. What `implementor.md` teaches there
  is the ordering the driver would otherwise have enforced: the case is written, and watched fail,
  before the code exists.

  *A verifier agent would produce evidence that looks independent and is not, because
  `independent: true` is checked structurally and nothing signs a record.* Also true, and it is why
  **`adversary.md` is not a verifier**. It emits no evidence at all. Its cases are run by the test
  runner, which is a producer the engine reads; its opinion is recorded as a `review-result` from an
  agent, which satisfies no `human: true` review requirement and gates nothing. `gap-register.md`
  D-3 is untouched and stays proposed — nothing here needs it, which is the point.

* **A `verifier` agent is still absent, and that one does wait for D-3.** An agent whose own record
  the engine would count as independent is exactly what cannot be built until a record can be
  signed.

The honest surface today is the one here, plus the driver: plan the work with the skill, and run it
with `protocol drive`, where the enforcement is a program rather than a paragraph. In a session,
every agent's hard rules are the instruction layer only — `adversary.md` says so in its own file,
and its report leads with `git diff --stat` so a reader can check the bound held rather than trust
that it did.
