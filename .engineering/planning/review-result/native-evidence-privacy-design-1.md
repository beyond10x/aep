---
format: aep.planning-md/3
id: review-result:native-evidence-privacy-design-1
kind: review-result
status: active
title: Independent review of evidence reference withholding contract
relations:
- reviews: design:native-evidence-privacy
revision: 1
---
unit: AEP #90 / Atlas ADR 0066 proposed four-document contract; AEP base 6d7a44d and Atlas base 6574ab9
verdict: NEEDS-CHANGE (one documentation warning; no contract blocker found)
cases: executed 0→0, red 0 (source/design review only; runtime execution prohibited)
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: refresh current consumer inventory, resolve six contract choices, retain implementation and rollout proof gates

## Reviewer change boundary

Reviewer `git --no-pager diff --stat`: no tracked changes by this reviewer. The sole write is this assigned ignored report. The four draft documents were read without edits; all source was inspected read-only. No Cargo, AEP command/store mutation, integration API, commit, push, fixture mutation or runtime test ran. Worktree lease was acquired for this review and released at handoff.

The owner drafts comprise three AEP documents and one Atlas ADR. `git diff --check` passed in both trees, and the accepted Git-store design remains an exact byte prefix of the amended document. These are static document checks, not runtime verification or independent signed evidence.

## Actionable finding

| Location | Verdict / origin / severity | What was established and what reaches it | Required correction |
|---|---|---|---|
| Atlas `architecture/adr/0066-evidence-reference-withholding.md:88`; repeated in AEP `target/native-evidence-privacy/design-handoff.md` source-discrepancy inventory | NEEDS-CHANGE / introduced / warning | The intended native owner integration is now metaharness `19b788d6`, source `3385656e`. `git show` of `crates/metaharness-aep/Cargo.toml` at both revisions names direct `aep-cli`, `aep-domain`, `aep-driver`, `aep-driver-spec`, `aep-engine`, `aep-project` and `trace-domain` dependencies at `6d7a44d3607d2d9a6ffdf0a165993c546c43d0db` (AEP 0.68.0), plus the development `aep-schema` pin. The draft table inventories only historical metaharness `63fe852` pinning AEP `28abe09b`. That historical statement is accurate, but does not inventory the actual intended runtime for the reader-first rollout. | Keep the historical source observation, add the current intended integration/source and AEP pin, and keep installed/staged executable identity and `/6` behavior explicitly unverified. Do not infer that the already updated 0.68 pin understands this proposed format. This is a documentation/rollout correction, not a new implementation dependency or a live compatibility result. |

This warning need not expand the contract or delay preparation of a narrowly scoped owner implementation after the coordinator resolves the proposed choices. It must be corrected before the inventory is used to claim readiness for the native adopter.

## Six choices assessed

1. **Same-slot immutable-original exception:** internally consistent and adequately explicit. It changes the accepted physical-file rule through a new opt-in format, not an implicit permission to edit ordinary evidence. Exact private original bytes, original slot plus digest, one public envelope, unchanged artifact bytes and no new observation are the narrow mechanism. Public history must visibly say withheld; a normal absent `reference` alone would be insufficient.
2. **Typed/version ownership:** consistent with AEP ownership. Rust raw/admitted values and generated schemas own the new value representation; no new evidence kind, ESS domain, compiled ESS dependency, registry platform or database migration is required. `/5` defaults and Markdown `/3` remain intact. Strict target preparation avoids silently changing ordinary permissive `/5` decoding.
3. **Reader boundary:** correctly identifies that the current selector is not a universal fence. All participating history/count/mutation routes must share admission and carry representation metadata or explicit incompleteness. The draft does not pretend a format bump repairs frozen old raw-library callers. Actual exclusion/refusal of still-used old routes is a per-adopter rollout blocker, not an unimplementable requirement that every obsolete binary magically change behavior.
4. **Custody and operation:** ordering is coherent. Validate and construct the candidate, retain and sync the exact original and binding privately, then sync and atomically replace the one public slot. Selector preparation and separate record applications are explicitly separate atomic operations. Retry identifies the original operation and verifies custody; it may not allocate another occurrence simply because the existing public bytes changed.
5. **Proof limit:** accurately constrained. Public validation can establish structure, internal digests, counting and facts supported by available Git history; it cannot authenticate an uncommitted original or prove inaccessible private equality. Private verification supplies exact-byte/replayed-transform evidence, not actor authentication. Hash guessing and remaining historical private bytes are acknowledged. Stronger public cryptographic proof is not silently promised.
6. **Rollout/scope:** appropriate to the reported two uncommitted migrated evidence records. Reader readiness precedes opt-in; prepare must preserve that dirty migration rather than rerun it or publish originals. Review prose, history rewriting, generic redaction and the independent migration-verifier defect remain separate. Routine implementation authority need not be turned into another approval ceremony; acceptance of the proposed physical immutability exception and proof boundary is the actual decision.

## Source-backed checks and implications

**Evidence eligibility can be preserved without the reference.** At the AEP baseline, `crates/edge/aep-cli/src/planning/conformance_count.rs:53` counts ordinary evidence by kind and judges ESS records using kind/source plus the artifact kind/model digest. `reference` is not an input to that judgment. Protected source strings must therefore remain exact, including embedded structured JSON and its diagnostics; the proposal explicitly does not parse/rewrite that text. Review links/outcomes and artifact model digests remain protected. Executable acceptance must compare the full `EvidenceOnHand` map and exact discounted results, not just totals. This source finding clears a design concern; no equality regression was run.

**Unknown/duplicate/raw member treatment is a real new boundary.** `journal.rs:39` Entry and its Evidence variant accept optional defaults and are not closed against unknown members. Preserving presence/value, including explicit null versus omission, cannot be implemented by a plain typed deserialize/serialize round trip. The draft says this directly, requires closed `/6` admission and duplicate-key rejection, and refuses incompatible originals during prepare. Exact whitespace remains in private custody; only the public representation is canonicalized. The transformation changes only the entire present nonempty reference member, never an arbitrary matching substring or another field.

**Old-reader bypass is real in source, not proven safe by this review.** `planning.rs:160` first calls `project_plan_at`, whose `:221` implementation recognizes the expected selector parent, then falls back to raw `Plan::git_at` when no project was recognized. `journal.rs:605` skips unreadable evidence while counting it; `planning.rs:639` takes only `history_git(...).0` for evidence judgment. A new envelope intentionally lacks legacy Entry root fields, so a raw old Entry reader will not silently accept a partial projection, but a caller discarding unreadable counts can still lose the occurrence. The proposed envelope root is therefore necessary but not sufficient. The draft's frozen old CLI and actual embedded-route fixtures, with upgrades/exclusions where refusal fails, are the right rollout obligations. This pass ran no executable alias or old-binary probe.

**Current immutability validation is narrower than the proposed `/6` contract.** `planning/git_record.rs:183` detects working-tree modifications/deletions against HEAD; it does not itself implement an allowed original-to-envelope transition or full available-history checks. The new document explicitly requires original equality when Git has those bytes, keeps public/private proof distinct when it does not, and forbids later downgrade/repath/deletion/reset. That is required implementation work, not a property inherited from the current validator. Do not count the current kind-total migration verifier as proof of it.

**Cooperative concurrency is stated honestly.** `planning_writer_fence.rs` documents advisory canonical project/store locks and limits against older nonparticipating writers. The draft adopts that scope rather than claiming to exclude arbitrary editors. Before any private write, ordinary deterministic refusals must be settled; post-write IO/sync failures have a distinct recovery outcome. Locks/housekeeping do not establish archive security or crash durability by themselves. Path substitution, private permission guarantees and synced-directory ordering remain implementation proof obligations.

**Crash and retry claims do not require a new platform.** An incomplete private creation may be safely retained/refused; it cannot be overwritten as if it were an unrelated free token. A retry after public replacement must recognize the exact envelope before interpreting the original-source precondition as stale, then verify the private original/binding. A sync failure after rename must report committed-or-recovery-required. Two-record correction is two operations, so an intermediate state with one represented record and one original is expected and may not be published while the remaining privacy finding persists. These interpretations follow the draft's explicit commit point and exact retry, and should be made executable in the implementation tests.

## Rollout proof still required; not design defects

- A source-built new AEP CLI, actual intended metaharness embedded AEP route, staged driver, and mandatory automation must all be identified and exercised. An updated standalone CLI or source pin does not establish installed readiness. AEP 0.68 at the current metaharness pin is still an old reader relative to proposed `/6`.
- Frozen old discovery/explicit-store/alias/worktree routes must either refuse before mutation or be explicitly excluded/upgraded for the adopter. Any required bypassing route blocks that adopter's preparation. Do not claim universal old-library refusal.
- Real complete dirty-migration fixtures must retain exact original bytes, slot/order/multiplicity, raw protected-member presence/value, artifact/transition bytes and eligibility/discount results. Test both same-second distinct observations and byte-identical duplicate occurrences, plus ordinary evidence-writer retries after representation.
- Fault injection must cover private original creation, binding creation, syncs, public temporary write, rename and directory sync, including exact retry and participating concurrent writers. No crash-tested or level-4 safety claim is made by this design pass.
- Public-only validation must label unavailable private proof. Private missing/corrupt originals must fail. Closed/deduplicated decoding, repath/downgrade/direct-edit controls and unchanged publication checks must be exercised on the actual candidate. No historical finding is erased by current-tree correction.
- Independent implementation review and the complete owner gate remain necessary. These docs and this source review establish no native delivery, containment, installed-reader compatibility or release.

## Reviewed document identities

- `docs/design/evidence-reference-withholding-v0.1.md`: SHA256 `b49ea280d1d333c45303a4da3de4284dc2f2a3757a916395c21c2926625975ee`.
- `docs/design/git-native-planning-store-v0.1.md`: SHA256 `de0d713d639b5a45ee0e44d05293e389cc58550417b89e9832f52935eced71b0`.
- `docs/design/README.md`: SHA256 `ff5393d62bf0d76974b25c5a3808a553a8809cab044abf4d9e1c28fa1eba459c`.
- `atlas/architecture/adr/0066-evidence-reference-withholding.md`: SHA256 `963e4ed03d516207c0262ef560ef9808a5f302d93e8df2d4eea04e16a9a3de28`.

No machine-local private path values, retained original contents or credential values were read into this report. The current metaharness source inventory was read from exact local Git objects, without changing that repository.

```findings
- file: architecture/adr/0066-evidence-reference-withholding.md
  line: 88
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The relying-party inventory names only historical metaharness 63fe852 and must also identify intended integration 19b788d6/source 3385656e with its AEP 6d7a44d pin while keeping installed and proposed-format compatibility unverified.
```
