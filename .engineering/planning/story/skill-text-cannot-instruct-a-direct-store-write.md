---
format: aep.planning-md/1
id: story:skill-text-cannot-instruct-a-direct-store-write
kind: story
status: archived
title: A skill that instructs a direct store write fails the build
summary: The prohibition on hand-editing planning artifacts is guarded by a test that reads each installed skill's text, not its path.
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
revision: 7
---
# Story: a skill that instructs a direct store write fails the build

## Outcome

A skill file that tells an agent to patch a planning artifact by hand cannot reach `main`. The guard
reads the skill's **text**, not its path.

## Context

`story:adopter-schema-contract-tooling` made `protocol artifact body --from <path|->` the sole
body-mutation path and rewrote both installed planning skills to say so
(`integrations/claude-code/skills/planning/SKILL.md:54`, the codex skill's `:53`). Its last
Acceptance line asked for a test per defect, and eight of nine defects have one.

*Direct store-write instruction* does not. Nothing in this repository reads a `SKILL.md`'s content:
the only code that touches one joins its path and asserts it exists
(`crates/protocol-cli/src/drive.rs:7747`). The prohibition is prose guarded by nothing, which is the
same shape as the defect it was written to fix — the skill previously **told** agents to patch bodies
directly, and no test noticed.

The cost is asymmetric. A skill regressing to "edit the frontmatter directly" ships green, installs
into every adopter, and is discovered only when a store's revision counter stops matching its
history.

## Acceptance

- A test enumerates every installed skill under `integrations/` and refuses one whose text instructs
  a direct write to a planning-store file — editing frontmatter, patching a body, or writing
  `status:` by hand.
- The check is over content, not over a path or a byte count, and it names the file and the offending
  line when it fails.
- Planted on a copy of a real skill, the instruction is caught; the shipped skills pass unmodified.
- The pattern set is written where a reader can extend it, and adding a phrase does not require
  touching the test's control flow.

## Out of Scope

- Grading a skill's prose quality, or any check that a skill teaches the *right* thing beyond this
  one prohibition.
- Validating skills a third party installs outside `integrations/`.
- Reading `SKILL.md` at run time to enforce behaviour. This is a build-time guard on what ships.

## Notes

Derived from the 2026-08-30 audit of active artifacts; see
`story:adopter-schema-contract-tooling` § *Closed on evidence — 2026-08-30* for the enumeration of
which eight of nine lines held.

## Scope

Derived 2026-08-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/protocol-cli`, its integration-test tree — inferred from two cited facts: the only code in the repository that touches a skill file is `crates/protocol-cli/src/drive.rs:7838`, and the only existing guard over committed `integrations/` documents is `crates/protocol-cli/tests/workflow_coverage.rs`
- **Files:** one **new** file under `crates/protocol-cli/tests/` — inferred; the story does not fix its name
- **Files:** `crates/protocol-cli/Cargo.toml` is **not** touched — cited, it declares no `[[test]]` targets, so a new integration test is auto-discovered
- **Files (read, never written):** `integrations/claude-code/skills/{planning,wave,schema-contracts}/SKILL.md`, `integrations/codex/skills/{planning,schema-contracts}/SKILL.md` — cited; the acceptance requires the shipped skills pass unmodified
- **Files (not touched):** `crates/protocol-cli/src/drive.rs` — cited; *Out of Scope* rules out reading `SKILL.md` at run time, so the cited defect site is evidence, not a change target. **The story's citation has drifted**: it names `drive.rs:7747`; the code it describes is at `:7838`, inside `the_committed_step_map_compiles_into_the_exact_argv_a_native_run_is_launched_with` (`:7804`)
- **Symbols:** none pre-existing. The pattern set is new; a `const` slice in the new test file is the shape `crates/protocol-cli/tests/workflow_coverage.rs:38` already uses — inferred
- **Also likely:** `AGENTS.md`, an *Enforced by* line beside the other source-scan guards (`crates/aep-domain/tests/invariants.rs`, `crates/aep-engine/tests/evidence_scan.rs`) — inferred, repository convention rather than an acceptance line
- **Documents:** none changed — the story reads documents and adds a test; it edits no `workflows/`, `principles/`, `protocols/` or `docs/` file — cited
- **Confidence:** **high** — the story names the defect site and the tree confirms that exactly one crate touches `integrations/` skills and already hosts the sibling guard; only the new file's name is open
- **Would collide with:** any unit editing skill prose under `integrations/**/SKILL.md` or `integrations/**/references/*.md` — the collision is **semantic, not textual**: a wave rewriting a skill can trip this new guard, and this guard's pattern set can refuse that rewrite. Also any unit adding a pattern-set document under `integrations/`, or editing `crates/protocol-cli/tests/workflow_coverage.rs`. It does **not** collide with `crates/protocol-cli/src/`, nor with `crates/protocol-cli/Cargo.toml`

**Not established.** *"Every installed skill under `integrations/`"* resolves to five `SKILL.md` files, but `integrations/claude-code/agents/*.md` (six files), `integrations/codex/AGENTS.planning.md` and `skills/*/references/*.md` (two files) ship the same way and could carry the same regression; the story does not say whether they are in. Where the pattern set lives is a real fork: a `const` in the test file changes nothing outside `crates/protocol-cli/tests/`, while a committed data document in the style of `integrations/workflow-coverage.yaml` adds a file under `integrations/` and changes the collision answer. There is no `informed_by` edge to lean on — the defect site came from the body and from `story:adopter-schema-contract-tooling`'s closing section.

## Archived 2026-08-30 — the premise was wrong, and the store already answers it

**The acceptance asked for the wrong instrument.** It asked for a test that reads every installed
skill's text and refuses one that instructs a direct store write. That test was built, attacked
twice, corrected three times, and **deleted before release**. What survives is the three shipped
sentences it caused to be corrected.

**The defect is real and is already caught, one step later and deterministically.** A document
edited outside a command has a revision no event supports. Measured in a scratch store:

```console
$ sed -i 's/^status: draft$/status: active/' <store>/story/probe.md
$ protocol artifact validate
1 problem(s):
  - story:probe drifted from its log: status disagrees with event story:probe@1#0~584ad5e7de54a3c7
    — an edit made outside a command is a change nothing decided
VALIDATE EXIT=1
```

`protocol artifact validate` is already a gate step: `plan-check` is
`cargo run -q -p protocol-cli --bin protocol -- artifact validate` (`Taskfile.yml`). So the build
already fails on the **consequence**. The deleted guard aimed at the **instruction** — English prose
— which is a strictly harder target with a false-positive rate, for a defect the cheaper check
already caught.

**Why it could not converge, measured.** The guard reached **2,153 lines of Rust** classifying
sentences across 18 markdown files. Two adversarial passes and three correction rounds went into it —
roughly three hours of agent time. Attack 1 measured 26 of 32 realistic instructions escaping;
attack 2 measured 188 of 214 list entries deletable with the suite still green. Both were fixed, and
the mutant kill rate reached 283 of 284 over a stated sweep.

Then the base branch moved, somebody edited a skill in the ordinary way, and it went red on **two
false positives**:

- `integrations/claude-code/skills/planning/SKILL.md:39` — *"the six that are missing from it write
  — new, move, relate, body, evidence, catch-up"*. A list of `protocol artifact` subcommand names,
  read as a hand-edit instruction because `body` is also a store surface.
- `integrations/claude-code/skills/wave/SKILL.md:389` — *"you record why in the artifact you write"*.
  A relative clause; the object precedes the verb, which the both-directions search made visible.

Both sentences are correct. It also went red because a new document was added, since the corpus is
pinned in both directions — so adding any `.md` to the plugin reddens the build until a Rust list is
edited. **A guard that fails when the prose it polices changes is a guard somebody disables**, and
prose is the thing that changes most in this repository: the shipped skills *are* the product.

**Two limits worth keeping in mind for whoever revisits this.**
`protocol artifact validate` reports **76 documents predate the event log**; those cannot be
drift-checked. Every new write carries events, so that number only falls. And the deleted guard
never scanned the three `.json`, one `.yaml` and one `.sh` file under `integrations/` that also
carry model-visible instruction text — `plugin.json`'s `longDescription` and `defaultPrompt` among
them.

**What landed and stays:** the three corrected sentences, and an `AGENTS.md` safety-envelope bullet
that now names `validate` as what enforces the rule rather than a scan that no longer exists.
