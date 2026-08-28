---
format: aep.planning-md/1
id: story:completion-audit-join
kind: story
status: implemented
title: What made this done, answerable from the store
summary: The admitted record is joined to the artifact through the journal, so the question a reviewer asks three months later is answered by the store rather than by git archaeology.
owner: protocol
tags:
- evidence
- store
relations:
- decomposes: epic:evidence-gated-completion
- depends_on: story:journal-backed-store
- depends_on: story:completion-needs-evidence
- depends_on: story:history-from-the-event-log
revision: 6
---
# Story: What made this done, answerable from the store

## Outcome

Somebody auditing a closed story three months later types one command and gets the record that closed
it — the suite, the transcript digest, the verifier class — instead of reconstructing it from commit
archaeology.

## Context

The gate makes closing a story require evidence; this makes the evidence findable afterwards. The
store has no audit join today (**D-P3**), so the only trace of *why* is the commit that changed the
line, and a commit message is not a record the engine admitted. P3's journal is where the join
belongs, which is why this depends on it rather than inventing a second place to keep history.

## Acceptance

- The admitted record is joined to the artifact through the journal and is retrievable by artifact id.
- The join names the revision the artifact was at when the record was admitted, so a later edit cannot
  make an old record look like it was about the new text.
- `protocol explain` can answer *what made this done* for a story in this store.
- Removing the record's source file does not silently unlink the join — the join is a stored fact, not
  a path.

## Shipped as `protocol artifact explain <id>` — 2026-08-28

The story asked for `protocol explain`; the verb is **`protocol artifact explain <id>`**, because
`protocol explain` already means a policy evaluation and this is a plan question. Its doc comment
says so.

| line | how it holds |
|---|---|
| the admitted record is joined to the artifact through the journal and retrievable by artifact id | one reading path, `entries_from_the_contract` — the event log, not the markdown file — so markdown, SQLite, Postgres and the hybrid answer alike |
| the join names the revision the artifact was at when the record was admitted | the evidence **event's** revision, never the current one; guarded by `what_made_a_story_done_names_the_revision_each_record_was_admitted_at`, verified by breaking it (re-dating onto the current revision fails with *the two revisions are indistinguishable: 2 -> 2*) |
| `explain` answers *what made this done* for a story in this store | `crates/protocol-cli/src/planning.rs`, text and `--format json` |
| removing the record's source file does not unlink the join | `a_joined_record_outlives_the_file_its_reference_names` — record with a `--ref`, delete the file, the record is still there |

Open question default taken: the join is **one-to-many**; two records on one move are both shown.
Ordering is **log order and not instant comparison**, because `--at` is back-datable and a record
back-dated behind a move it was written after did not make that move.

**Building it exposed a defect older than it.** `entry_from_event` read a move's `decided_on` only
as a JSON object, and it travels as JSON *text* — so over **SQLite and Postgres every provenance
account silently defaulted to empty**, and a move made on a bare `--evidence KIND=COUNT` was
indistinguishable from one the store held a record for, in exactly the two stores with no journal
file to fall back on. Fixed with the verb, because `explain` could not have been honest over it.

Standing risk, recorded: the revision the join names is the evidence event's, which holds only while
recording evidence stays an observation at an unchanged revision. If a backend ever bumps the
revision on `RecordEvidence`, the join silently re-dates — which is what the revision test pins.

Evidence: `cargo test -p protocol-cli --test store_selection` → 10 passed, 2026-08-28, over
markdown, SQLite and hybrid.

## Out of Scope

Reconstructing joins for stories closed before the journal existed. A rule applied backwards to
records made under a different rule reports noise.

## Open Questions

Whether the join is one-to-one or many. Decides: protocol owner. Default if nobody answers: many —
a story satisfied by a suite and a transcript check has two records, and forcing a choice between them
would lose one.
