---
format: aep.planning-md/2
id: story:journal-is-a-hash-chain
kind: story
status: implemented
title: A journal record cannot be changed without the store saying which one
summary: Each appended journal record carries the digest of the one before it, so validate fails at the exact record a tamper touched — detection inside a log, not attestation.
relations:
- decomposes: epic:evidence-gated-completion
- serves: vision:O2
- informed_by: story:journal-reconciliation
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/chain.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/journal.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/provider.rs
revision: 5
---
## Context

`docs/plan/gap-register.md:108` records a measurement taken during driver run `EVAL-1/1` on
2026-08-29: a step wrote `revision: 99` straight into a planning document with an ordinary file
write, and `protocol artifact validate` exited **0** on the well-formed result.

`story:journal-reconciliation` closed half of that. A document claiming a revision no event records
is a `forged` finding and fails the gate. It closed only half, because it reconciles the document
*against the journal* and nothing protected the journal. An actor who writes `revision: 99` into
`story/forge.md` **and** appends a matching event line to `journal.jsonl` is consistent with
himself, and self-consistency is the whole of what that check can measure. Reproduced 2026-09-17
against a scratch store: the two-file edit printed `valid` and exited 0, and `--strict` printed
`valid` too.

## Acceptance

- Every journal record this store appends carries the digest of the record before it and a digest of
  its own bytes, written under a lock and flushed before the lock is released.
- `aep plan artifact validate` walks the chain and **fails**, naming the exact record, when one has
  been edited, inserted, removed or reordered. A break is a problem — the hard tier — not a
  `--strict` class, because no operator workflow produces one.
- A revision forged into the document **and** the journal together is refused by name, where before
  it printed `valid` and exited 0.
- A journal written before the chain existed validates, is reported as not covered, and is a finding
  of no tier including under `--strict`. Every store in this workspace has one.
- The first record sealed onto such a journal names the whole legacy prefix as its parent, so the
  older lines become tamper-evident without being rewritten.
- A half-written line left by a killed process is debris rather than an attack: the chain steps over
  it and the next sealed record re-anchors across it.
- `validate` says out loud how many records are sealed and how many lines predate the chain, so a
  reader who sees `valid` can tell whether the record behind it is sealed or merely self-consistent.

## Out of Scope

**Prevention.** This is a reading taken at the gate, after the fact. Preventing the write needs to
know who wrote a file, which is writer-side identity — gap register **D-3**, proposed and not
accepted. Nothing here signs, holds a key, or attests.

**A log replaced wholesale, and a log truncated at the tail.** A chain is computed from the bytes it
protects, so anybody who can rewrite every byte can rewrite a consistent chain over them, and
lopping the last ten records off a hundred-record journal leaves ninety that link perfectly.
Detecting either needs the head of the chain held somewhere the log does not control — a signature,
a notary, a second store — which is **D-3** again. This story deliberately implements none of it.
What it buys is that tampering inside a log is no longer free and no longer silent; not that
tampering is impossible.

**Enforcement on the `native`/`b10x` arm.** Parts (1)–(4) of the gap register row stand exactly as
written; the arm's store-integrity cells still report *compliance* or *not observable*.

## Evidence for the gap

`docs/plan/gap-register.md:108` — the original measurement, and the row this closes the detection
half of. The reproduction against a scratch store on 2026-09-17 is recorded in the module
documentation of `crates/plan/aep-backend-markdown/src/chain.rs`.

## Controlled violation

The gate is shown to fail on the thing it claims to catch, rather than only to pass:

- `a_revision_99_forged_into_the_document_and_the_journal_together_is_refused_by_name`
- `a_journal_entry_edited_in_the_middle_is_named_and_the_records_after_it_are_counted`
- `a_record_removed_from_the_middle_leaves_the_next_one_naming_a_parent_that_is_not_there`
- `two_records_swapped_break_the_chain_even_though_neither_was_edited`
- `stripping_the_seals_off_the_tail_is_a_break_and_not_a_downgrade_to_legacy`
- `an_appended_record_that_forges_a_revision_cannot_be_sealed_by_copying_a_neighbour`

and, on the other side, that it does not go red on an honest store:

- `a_store_written_only_through_the_cli_reports_a_verified_chain_and_validates`
- `a_journal_written_before_the_chain_existed_validates_and_is_not_reported_as_tampered`
- `a_chain_that_starts_on_a_legacy_journal_seals_the_lines_that_came_before_it`
- `a_half_written_line_is_debris_and_the_next_append_re_anchors_across_it`

## Open Questions

Whether the chain head should be recorded outside the log. Decides: store owner, as part of
**D-3**. Default if nobody answers: **no** — this story states the limit rather than closing it,
and a half-attestation nobody registered a key for would read as more than it is.
