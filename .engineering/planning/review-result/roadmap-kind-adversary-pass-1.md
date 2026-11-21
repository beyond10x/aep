---
format: aep.planning-md/3
id: review-result:roadmap-kind-adversary-pass-1
kind: review-result
status: active
title: Adversary, pass 1, story:roadmap-kind
relations:
- reviews: story:roadmap-kind
revision: 1
---
```
unit: story:roadmap-kind, branch feat/roadmap-kind at 9dc5ea225; cases in a separate managed worktree, uncommitted there
verdict: NEEDS-CHANGE
cases: executed 24→42, red 10
origin: introduced 9 / pre-existing 0 / undecided 0
wrote-outside-worktree: 10 paths, all under <scratch>
needs-coordinator: commit the two test files through the bot; decide whether redact may change an approved roadmap (finding 8)
```

**Cases added.** `crates/edge/aep-cli/tests/roadmap_kind_adversary.rs`, 16 cases, 8 red at 9dc5ea225:
an epic subkind cannot be placed on a roadmap; `body` rewrites an approved `team-roadmap`; a
`team-roadmap`'s entries are not validated; `mine/epic:first` inside member `mine` is refused as not
an epic; a second `roadmap` block is ignored; a `~~~roadmap` block is not read; only the first
unreadable entry is reported; `redact` changes an approved roadmap's order. The 8 green cases each
kill one mutant the unit's suite let survive (existence namespace guard, dedupe, approval-record kind
filter, `decides`, live statuses, integral limit, body refusal kind check) or read a JSON block with a
null rationale. `crates/plan/aep-backend-markdown/tests/roadmap_adversary.rs`, 2 cases, both red: an
entry-level defect is located at the block's first line, not its entry's.

**Mutation probe**, on a copy: seven guards removed together left the unit's 16 cases green; each
green case above went red under its mutant.

**Attacked and not broken.** CRLF bodies; a block quoted in another fence; an unclosed block; JSON
arrays, merge keys, unknown keys, a number as horizon, an `@version` id; horizon drops after a
plateau; a missing epic listed twice; `reviews` and `decides` both honoured, a story that reviews the
roadmap not counted; `set --status` and `set --kind` refused; limits 0, 1.5, -2 and `two` refused;
deterministic order; `aep doctor` counts problems only; `waves` unchanged.

```findings
- file: crates/plan/aep-backend-markdown/src/roadmap.rs
  line: 199
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: off_roadmap lists epic subkinds via is_a(Epic) while the entry check refuses any namespace but epic, so a store with a live platform-epic can never pass --strict
- file: crates/edge/aep-cli/src/planning/roadmaps.rs
  line: 93
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a roadmap subkind (team-roadmap) climbs the roadmap ladder but is never validated, counted against the limit, or refused by body once approved
- file: crates/plan/aep-backend-markdown/src/roadmap.rs
  line: 138
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: only the first top-level roadmap block is read, so a second block naming a missing epic validates
- file: crates/plan/aep-backend-markdown/src/roadmap.rs
  line: 156
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an unreadable block reports only its first bad entry, against the validation-accumulates invariant
- file: crates/plan/aep-backend-markdown/src/roadmap.rs
  line: 166
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: entry-level errors report the block's first line instead of the offending entry's line
- file: crates/edge/aep-cli/src/planning/roadmaps.rs
  line: 109
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an epic spelled with this store's own member name is refused as not an epic and leaves the epic off-roadmap, contrary to ArtifactId::local_to
- file: crates/plan/aep-backend-markdown/src/findings.rs
  line: 567
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a tilde-fenced roadmap block renders as the roadmap but is not read, so its entries are never validated
- file: crates/edge/aep-cli/src/planning.rs
  line: 3963
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: redact rewrites an approved roadmap's entries and the approval stands, bypassing the changed-only-by-a-successor rule
- file: crates/edge/aep-cli/tests/roadmap_kind.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: seven guards can be removed with the unit's suite staying green
```
