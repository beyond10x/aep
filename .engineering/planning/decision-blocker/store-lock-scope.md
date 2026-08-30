---
format: aep.planning-md/1
id: decision-blocker:store-lock-scope
kind: decision-blocker
status: cleared
title: Is the store lock per store, or per project?
relations:
- blocks: story:protocol-drive-verb
revision: 4
---
# Blocker: is the store lock per store, or per project?

## Resolution — 2026-08-30: **per project**, and the documentation changes

Taken by the wave coordinator, on the ground that the decision the blocker asked for was never
one. Recorded rather than deleted, because how it was filed is the finding.

**The lock stays where the code puts it.** `crates/protocol-cli/src/drive.rs:683` takes it at
`runs_directory(&inputs.project)`, and `--store` defaults to `<project>/.engineering/planning`
(`crates/protocol-cli/src/planning.rs:59`), so a store belongs to a project unless somebody
overrides the flag to point at another project's. **Nobody was shown to do that.** The remaining
work is to make the words match: `crates/aep-driver/src/lock.rs:9` and
`crates/protocol-cli/src/drive.rs:92` both say *"one fixed path per store"*, and
`story:protocol-drive-verb`'s first acceptance line says the same. Three lines of prose, no
behaviour change, no operator.

## Why this was filed as a blocker, which is the part worth keeping

The adversarial pass on `story:protocol-drive-verb` built a fixture with two project directories
pointed at one store by an explicit `--store`, ran `drive` in both, and both proceeded. That is a
correct observation and a real red case
(`adversary_a_second_drive_over_one_store_from_another_project_is_refused_and_writes_nothing`,
`crates/protocol-cli/tests/drive_cli.rs:2315`).

The coordinator then promoted it to *"two `protocol drive` runs walking one set of documents"* and
held the story on an operator decision — **severity nobody measured, on a configuration nobody was
shown to use.** A fixture can build any conditions it likes; that it breaks under them says what
the code does, not that anybody arrives there. The step that was skipped was asking *what reaches
this* — the caller, the flag, the default, the documented workflow — and the answer was: only an
explicit override, with no known user.

The verified defect underneath is two doc comments disagreeing with the code. That is smaller than
what was filed, and small enough that filing it as a decision cost more than fixing it would have.

The rule now lives in the skill: `integrations/claude-code/skills/wave/SKILL.md` § *Before you
promote a finding, check the scenario is one somebody reaches*.

## What Is Blocked

Nothing, now. `story:protocol-drive-verb` is held only by
`story:unreadable-lock-refuses-its-own-escape-hatch`.

## What Would Clear It

Done: the decision above.

## Who Can Clear It

Cleared by the wave coordinator, 2026-08-30.

## What We Are Doing Meanwhile

The red case stays red and stays in the tree. It now asserts a guarantee this project does not
make, which is the honest state until the three prose lines change; when they do, the case is
rewritten to assert the per-project scope or deleted with the reason.
