---
format: aep.planning-md/1
id: story:review-result-cannot-be-authored
kind: story
status: implemented
title: A review-result cannot be authored, retired or removed through the CLI
summary: 'Second adopter (substrate, 2026-08-29): new has no body input, body and move-to-archived are both refused as immutable, and the help sanctions rm while validate flags it. Three refusals, no legal path.'
owner: protocol
tags:
- adoption
- bug
- store
relations:
- informed_by: story:adopter-bugs
- serves: vision:O2
revision: 5
---
# Story: A review-result cannot be authored, retired or removed through the CLI

## Outcome

An adopter records a review through the CLI and it says what the reviewer concluded: the body
arrives with the record, the record's one legal transition is reachable, and the documentation
does not sanction the one operation `validate` then reports as drift.

## Context

Found by the second adopter — `beyond10x/substrate`, 2026-08-29, session `claude-session` —
while recording a backlog review as `review-result:backlog-vs-atlas-objectives` in a fresh store
(13 artifacts, valid). Every step was the skill's own path (`integrations/claude-code/skills/
planning/SKILL.md` § 3, guardrail 2: `new`, then `body`), and every step was refused:

1. **`new` has no body input.** `protocol artifact new --help` lists `--title`, `--summary`,
   `--owner`, `--tag`, `--relate` — nothing that carries a body (`crates/protocol-cli/src/main.rs`,
   the `New` arguments). A `review-result` is therefore created `active` with the template body
   only: one H1, nothing a requirement could rest on.
2. **`body` is refused.** `protocol artifact body review-result:backlog-vs-atlas-objectives --from
   review.md` →
   ``error: conflict: `01MEM0000000000000010` is a aep.review-result/v1, which is immutable: a
   record that can be edited after the fact is not evidence. Archive it, or supersede it with a
   new one.`` (`crates/aep-backend-memory/src/command.rs:483-489`). Correct on its own terms —
   `artifacts/lifecycles/review-result.yaml:1-7` says why — but with (1) it means the body of a
   review-result can never be anything but the template.
3. **The message's own advice is refused.** `protocol artifact move
   review-result:backlog-vs-atlas-objectives --to archived` → the same conflict, exit 1.
   `MoveStatus` calls `require_mutable` (`command.rs:317-321`), whose comment says the guard was
   added so a review-result "became [not] editable after the fact through the new command" — and
   it also closes `active -> archived`, the one transition `review-result.yaml:12` declares.
   `protocol artifact lifecycle review-result` prints `active -> archived`; the kernel refuses it.
4. **`rm` is sanctioned and then flagged.** `protocol artifact --help` says "A plan item you did
   not want is deleted with `rm`" (`crates/protocol-cli/src/main.rs:336-337`). After `rm`,
   `protocol artifact validate` → `review-result:backlog-vs-atlas-objectives was deleted: its log
   ends at event …@1#0~9b47fe58… and the store holds no document — nothing is physically deleted
   through a command, so this was `rm`` (`crates/aep-backend-markdown/src/drift.rs:130-135`),
   exit 1. The adopter recovered by restoring the journal from git, which is not a path the
   documentation offers.

Net: a `review-result` created through the CLI is an empty immutable record that cannot be
completed, retired or removed. The adopter filed its review under its own `docs/reviews/`
instead, which is exactly the outcome the kind exists to replace.

## Acceptance

`protocol artifact new review-result <slug> --title … --from <path|->` creates the record with
its body in the same command; `protocol artifact move review-result:<slug> --to archived` succeeds
from `active` and is the only move it accepts; and `protocol artifact --help` no longer names
`rm` as the way to discard an item — or `validate` accepts a deletion the journal records
through a command.

Evidence that satisfies it:

- `new` gains `--from <path|->` (or `--body`) for every kind, and a test creates a `review-result`
  with a body and asserts `body` on it is still refused;
- `MoveStatus` distinguishes *editing* from *transitioning*: `require_mutable` guards
  `UpdateEntity`; a move on an immutable kind is decided by the lifecycle alone, and a test
  proves `active -> archived` succeeds while `archived -> active` and any `body` are refused;
- the `rm` sentence in `main.rs:336-337` is replaced by the command that discards an item — or
  a `protocol artifact discard <id>` verb records the deletion so `validate` stays green — and
  `drift.rs`'s `Deleted` message names that command;
- `task check` green.

## Out of Scope

Making review-results editable. The lifecycle's reasoning stands; the defect is that the CLI
offers no way to record one correctly, not that it refuses to change one.

## Open Questions

Whether an unwanted item is discarded by a verb that journals it, or by `rm` plus `validate`
accepting an orphaned create event. Decides: protocol owner. Default if nobody answers: **a verb**
— "nothing is physically deleted through a command" is the store's stated model, and a documented
`rm` contradicts it.
