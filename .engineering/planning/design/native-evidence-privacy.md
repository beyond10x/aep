---
format: aep.planning-md/3
id: design:native-evidence-privacy
kind: design
status: draft
title: Evidence reference withholding with exact private retention
relations:
- serves: vision:O2
revision: 1
---
# Evidence reference withholding v0.1

Status: proposed, 2026-10-03; not accepted or implemented. Owner decision: AEP issue #90.

Note, 2026-10-07: Atlas allocated ADR 0066 to another decision before this one landed, so the
coordinating Atlas ADR named below as "proposed Atlas ADR 0066" has no number yet. This design is
not accepted: it changes the store's evidence-file immutability rule (§ 7) and waits on that
decision, recorded as `decision-blocker:evidence-file-in-place-representation`.

This is the proposed amendment to the accepted [Git-native store](git-native-planning-store-v0.1.md),
§§ 2–7, coordinated by proposed Atlas ADR 0066, *Evidence reference withholding preserves the
original occurrence*. Approval of this document is not a release, migration or publication.

## 1. Source facts and the bounded decision

Source baseline: AEP `6d7a44d3607d2d9a6ffdf0a165993c546c43d0db` (0.68.0).
The following are source observations, not results of running a migration or test:

| Fact | Source at that revision |
|---|---|
| One evidence file is one occurrence; duplicate observations are retained and same-second order is encoded in filenames. | `crates/plan/aep-backend-markdown/src/journal.rs:447`, `:492` |
| Entry has artifact, revision, observation time/actor and change, but no record UUID. The optional reference is one string. | `crates/plan/aep-backend-markdown/src/journal.rs:39`, `:124` |
| Unreadable evidence is skipped and counted; evidence-gated CLI decisions currently discard that unreadable count. Entry's deserializer does not close unknown fields. | `crates/plan/aep-backend-markdown/src/journal.rs:605`; `crates/edge/aep-cli/src/planning.rs:630` |
| Changed/deleted committed evidence is currently rejected without a representation exception. | `crates/edge/aep-cli/src/planning/git_record.rs:183` |
| Ordinary explicit-store access consults an enclosing project selector; the project version is closed. | `crates/edge/aep-cli/src/planning.rs:221`; `crates/govern/aep-domain/src/project.rs:85` |
| Migration verification compares evidence totals by kind, not complete ordered evidence values. | `crates/edge/aep-cli/src/store_command/migrate_git.rs:162`, `:632` |

Proposal: one owner operation withholds **only the complete, present, nonempty optional
`change.reference`** of one evidence occurrence. It first retains the exact original file privately,
then atomically replaces that occurrence's existing public slot with one versioned representation
and receipt. The receipt labels an address as withheld, not originally absent or publicly retrievable.
No replacement URL, evidence observation, status move, artifact revision or completion claim is made.

This changes the physical-file immutability rule only through an explicit successor format. The
original remains immutable in private custody; its public representation retains the same slot
and logical occurrence. If that exception is not accepted, the publication blocker stays refused.
The older design's ULID wording in § 5 is not a source-backed evidence identity: this proposal uses
the original repository-relative slot plus full original byte digest, never an invented UUID.

## 2. Typed home and versions

Following the value-representation precedent in AEP's
[coverage evidence design](ess-conformance-coverage-evidence.md) and Atlas ADR 0040, this is a
concrete serialized representation of existing Evidence. It is not a new business entity, evidence
kind, separately mutable registry or ESS domain. Rust raw types, validation and admitted values own
the contract; generated schemas come only from the existing Rust schema generator. There is no
hand-maintained second model or compiled ESS modeling dependency. IO remains at the existing
backend/edge boundary; comparison and digest functions accept caller data and time.

| Contract | Proposed selection |
|---|---|
| Project selector | Explicit opt-in `aep.project/6`, Git stores only; `/5` remains supported and remains the default. |
| Evidence representation | Closed JSON `aep.evidence-representation/1`, containing exactly `format`, `receipt`, `entry`. |
| Transformation | Exactly `withhold-evidence-reference/1`. |
| Artifact Markdown | `aep.planning-md/3` unchanged, including review immutability and transition semantics. |
| Databases, legacy migration formats, Entity Runtime and ESS wires | Unchanged; none acquires this operation. |

These version names are proposed allocations to accept together, not capabilities of 0.68.0.
Ordinary writes preserve the selected supported version; initialization still selects `/5`.
No read or ordinary write implicitly prepares or upgrades a store.

The root deliberately has no legacy Entry fields beside `entry`. An old Entry decoder must fail
on missing required fields; it must not accept the envelope while ignoring its receipt. This is a
secondary fence, not enough for an old library that skips unreadable records. Every participating
consumer must first adopt the new reader or be excluded from a `/6` store.

## 3. One slot, one entry, one receipt

The public file remains at its **original repository-relative evidence pathname**, including
sequence and duplicate-occurrence suffix. Neither the public entry digest nor receipt allocates a
new evidence filename. The JSON root has this shape; placeholders below describe types:

```text
format: "aep.evidence-representation/1"
entry: <admitted original Evidence Entry with only change.reference removed>
receipt:
  rule: "withhold-evidence-reference/1"
  subject: <original repository-relative evidence pathname>
  artifact: <the Entry's existing artifact id>
  original_sha256: <64 lowercase hexadecimal digits>
  original_length: <exact original byte length, unsigned 64-bit integer>
  public_entry_sha256: <domain-separated digest defined below>
  archive_id: <opaque 256-bit token encoded as 64 lowercase hexadecimal digits>
  at: <caller-supplied operation instant>
  actor: <declared public-safe actor string>
  field: "change.reference"
  original_presence: "present"
```

All listed fields are required; unknown keys, duplicate keys at any decoded object level, unknown
versions/rules, malformed values and non-Evidence entries are refused. Subject must equal the
actual slot and its artifact directory must match both artifact ids. `at` is a valid RFC3339
instant; `actor` is nonempty attribution, not authenticated identity. Neither changes the Entry's
original actor/time. The archive token is a locator-independent private index key, not an address,
signing identity or new observation id. It must carry no private location or meaningful label.

The represented entry preserves every non-reference JSON member's presence and value, including
explicit null versus absent optional members, and its typed Entry interpretation. A typed
round-trip that drops defaults/nulls is not this transformation. The only removed JSON member is
`entry.change.reference`; null, missing or empty original references are not eligible. Exact
original whitespace/encoding survives in the private original even though the public JSON is
canonicalized. No code parses or rewrites text embedded in source strings.

Digest rules are part of this version. `original_sha256` is SHA-256 over exact original bytes.
Define `C` as compact UTF-8 JSON with recursively lexicographically sorted object keys, arrays in
original order, unsigned integers in decimal without leading zeros and strings encoded with the
Rust JSON serializer's escaping. Freeze escaping and optional-member vectors with the Rust-owned
format; no floating-point values are admitted by these shapes. `public_entry_sha256` is SHA-256
of UTF-8 `aep.evidence-entry/1`, one NUL byte, then `C(entry)`. The operation identity is SHA-256
of UTF-8 `aep.evidence-representation/1`, one NUL byte, then `C(the complete envelope)`. It is
returned and indexed privately, not embedded in its own hash input. This binds every receipt
field without a self-referential digest. Request/plan digests use their own versioned domains and
bind the selector, source, artifact and archive destination preconditions.

A digest is a content commitment, not authentication or a secrecy proof. Low-entropy hidden
values can be guessed against commitments. The existing slot already contains a digest prefix;
this proposal makes no blanket confidentiality claim beyond removing the selected plaintext
address from the current publication slot and diagnostics.

## 4. Invariants and exact-original custody

Under the participating-writer fence, compare complete ordered logical evidence before/after,
marking only the selected address as withheld. Preserve slot identity, multiplicity, same-second
ordering, evidence kind/source, review/outcome, artifact kind/id/revision, observation actor/time,
all other raw member values and eligibility/discount decisions. A receipt never enters counts,
pays a lifecycle requirement or becomes `Change::Evidence`. Every artifact file and transition,
including `decided_on`, remains byte-identical. Review bodies/findings/signatures are not rewritten.
Totals alone cannot establish these properties; the existing migration verifier is insufficient.
Its separate content-fidelity correction is not silently included in #90.

The explicit private archive must be outside **every Git work tree**, not merely ignored by one.
Require owner-private access, no symlink traversal, descriptor-anchored containment and safe file
creation; refuse unsupported permission/durability guarantees. Archive identity must not depend on
an ambient default directory. Store the exact original bytes create-only under the archive token,
and a create-only private binding of token, original digest/length, occurrence and operation.
Conflicting existing bytes or bindings refuse. Sync and reread both before changing the public slot.

This is ordinary owner-controlled filesystem retention, not WORM storage or a recovery service.
The operator owns backup and retention; private verification detects later loss or corruption but
cannot recreate lost bytes. No original, request, private locator, parser excerpt or matched value
may be printed to public diagnostics, copied to another publishable filename or written as a new
Git object by this operation. Receipts and public-safe actor declarations still pass unchanged
publication checks; a label is not intrinsically safe because it fits a schema.

## 5. Explicit selector preparation and one-record commands

Proposed verbs, not existing commands:

1. `aep plan store privacy prepare --dry-run` checks the selected complete `/5` Git store with
   the proposed `/6` reader. `prepare --apply --expect-selector <digest>` changes only the selector
   version under the existing writer fence. Preserve every other selector field and every artifact,
   transition and evidence byte. A complete, valid **uncommitted** `/1`-to-`/5` migration is eligible;
   do not demand a clean tree, publish originals first, rerun migration or reset its baseline.
   Incomplete/legacy, database and selectorless stores refuse. Temporary `/6` bytes and their
   containing directory are synced around atomic selector replacement. Zero representations is a
   valid prepared state. `/5` legacy behavior is not retroactively tightened: prepare separately
   refuses records the strict target reader cannot preserve, including unknown or duplicate fields.
2. `privacy preview --request <protected-request>` requires prepared `/6`, reads one exact slot
   and writes nothing. The
   request supplies expected source digest, private archive destination/token, public-safe actor
   and operation time. It binds selector bytes and current artifact identity/revision/content into
   a sanitized plan with rule, slot, digests and operation preconditions. The private destination
   is bound but never printed; plan hashes inherit the commitment limits above. There is no generic
   field selector: only the complete optional reference is eligible.
3. `privacy apply --request <protected-request> --plan <plan>` revalidates the exact plan under the
   same fence used by evidence/move writers. Require `/6`, strict original decoding and unchanged
   source/selector/artifact preconditions. A stale plan, malformed record, conflicting archive,
   unsupported field or unrelated existing representation refuses before mutation. Each operation
   handles one occurrence. Different occurrences are separate operations, not a batch transaction.
4. `privacy verify --private-request <protected-request>` rereads the private original, checks its
   exact bytes and binding, and replays the admitted transformation against the public envelope.
   A missing/corrupt original is a nonzero private-verification failure. Ordinary public validation
   requires no archive locator and reports its weaker proof explicitly (§ 7).

No command edits the index, commits, pushes or invokes a gate bypass. No artifact revision changes
because no artifact or observation changes; the receipt records the representation operation.

## 6. Commit point, failures and retries

Prepare and apply are independent atomic operations, not one cross-filesystem transaction.
For apply, finish deterministic preflight and construct/validate the candidate in memory first.
Then create, sync and verify the private original and its binding. Write only the public-safe
envelope to a same-directory temporary file, sync it, atomically replace the original slot, and
sync that directory. The replacement is the logical commit point. Recheck target/preconditions
before it under the cooperative fence; do not follow a substituted path or pretend the fence
excludes arbitrary editors or obsolete nonparticipating writers.

Readers see either the original or the complete envelope/receipt, never half of either. Before
replacement, a crash/IO failure can leave private recovery material while the public original
remains. After replacement, reporting/IO failure must say committed-or-recovery-required with an
operation identity, not claim a refusal changed nothing. Deterministic refusals write nothing;
post-write IO/recovery outcomes are a separate result class. Unsupported directory-sync guarantees
refuse rather than claim durable cross-filesystem atomicity.

Exact apply retry recognizes the same envelope and verifies its archive/binding again before
returning no-op success. A partial private archive can be completed only after all unchanged
preconditions and retained bytes are verified. A mismatch never overwrites it. Ordinary evidence
writer retries must recognize a validated representation of the same original canonical Entry and
occurrence and return the same slot, or refuse without writing; they cannot allocate a duplicate
because its public bytes now differ. Two identical originals in different slots remain two records.
A crash between two requested corrections leaves one original and one represented occurrence,
each counted once, and resumption handles the remaining occurrence.

## 7. Reader boundary, immutability and proof limits

One version-aware evidence-file decoder serves history, whole-store reads, counts, validators and
all evidence-gated mutations. `/5` ordinary decoding/behavior stays compatible; representations
are forbidden there. `/6` admits only strict ordinary Evidence entries and strict envelopes. The
prepare preflight verifies that target contract before moving the selector. No envelope-shaped or
unknown-version record may be treated as a legacy Entry or silently skipped. Mutation admission
fails on invalid/unreadable evidence before using counts; partial inspection may show readable
records only with an explicit incomplete result. Public text/JSON history retains receipt metadata
and visibly distinguishes withheld from originally absent references. Internal Entry count views
may be derived only from admitted records; raw serialization cannot recreate admission.

Keep the ordinary changed-evidence refusal. Add exactly one authorized transition at an existing
slot: the original to an envelope bound to its exact digest and preserving every protected field.
When Git holds the original, validate equality against it. When it was never committed, private
apply/verify establishes that equality and public validation says it cannot recheck that original.
After the envelope is committed, any alteration, deletion, downgrade, move to a different pathname
or duplicate under another slot is invalid. Do not treat deletion/recreation, selector removal or
format relabeling as a fresh baseline. Available Git versions must obey that one-transition rule;
missing historical/private material cannot be promoted to verified provenance.

Public validation can establish closed shape, internal digest consistency, occurrence identity,
one-entry counting and the constraints supported by available Git history. It can return valid
public structure with an explicit count of originals not privately verified. It cannot establish
custody of an inaccessible archive, authenticate an uncommitted original, prove CLI authorship of a
self-hashed receipt, or establish equality to hidden bytes without those bytes. Private verify adds
exact-original/replayed-transform proof, not authenticated actor identity. A stronger publicly
verifiable hidden-original proof is outside this contract and requires a different design.

Freeze old 0.68.0 behavior as a compatibility fixture. Ordinary discovery, explicit-store paths,
relative/absolute aliases and linked-worktree openings of `/6` must refuse before writing. Raw old
library callers that bypass project selection can skip unknown records today; a selector alone
cannot make them safe. Such consumers must be upgraded or excluded before any affected store
opts in. Detached evidence-directory copies with the selector discarded are not supported exports.
No source shipment by itself proves installed/embedded reader readiness. Specifically, 0.68.0's
`StoreLocation::plan` falls back to `Plan::git_at` when `project_plan_at` does not recognize the
expected selector location (`crates/edge/aep-cli/src/planning.rs:160`, `:221`). Do not extrapolate
ordinary explicit-path refusal to every alias. If a still-required old route fails the actual
refusal fixture, opt-in is blocked until that route is upgraded or excluded; new prose cannot
repair an old executable.

## 8. Reader-first rollout and acceptance evidence

Proposed Atlas ADR 0066 owns the relying-party inventory and order: AEP CLI plus embedded backend
readers, metaharness's direct AEP dependencies, installed/retained old readers, unchanged Gates,
and every other project/5 adopter. Accept the owner/Atlas contracts first; implement, independently
review and fully gate readers before making the writer available to an adopter. Demonstrate both
standalone and actual embedded routes. Only an explicit adopter decision may prepare its selector
and apply named corrections. No default movement, publication, release, dependency promotion,
installation, organization-wide adoption or Atlas store migration is authorized by this proposal.

Required synthetic executable evidence before implementation can be called complete:

- Original failing cases followed by green for complete dirty migration preparation; exact
  private byte preservation; same-slot correction of two records; zero artifact/revision changes.
- Full protected-field comparisons, same-second order, identical duplicate observations and exact
  ESS eligibility/discount decisions; receipts contribute no evidence; review immutability stays.
- Closed/duplicate-key decoding, unknown version/field, missing receipt, stale plans, archive
  collisions/corruption, unsafe locators, ordinary direct edits, repath/downgrade/deletion refusal.
- Fault injection around each sync/replacement boundary and concurrent participating writers;
  old-or-envelope visibility, no lost original, exact retry and no duplicate evidence allocation.
- Real frozen old CLI openings and actual embedded readers, not only parser unit tests; new `/5`
  behavior retained and `/6` unsupported routes refused before mutation.
- Public-only clone output that states unavailable private proof; private corruption nonzero;
  diagnostics/index/publication scans free of synthetic protected references; unchanged Gates
  evaluates the actual candidate, not a simulated relaxed policy.
- AEP's complete repository gate, using the built owner CLI, after independent review. No such
  execution is claimed by this source-only document.

A currently safe working tree does not erase private values in ancestors, tags or cached objects.
ADR 0018 still applies to the actual publication surface; any historical finding needs its own
owner disposition. This operation never rewrites history. Review prose, findings redaction,
arbitrary-field projection, remote archives and multi-record atomicity are explicitly out of scope.

