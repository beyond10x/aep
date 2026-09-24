---
format: aep.planning-md/2
id: review-result:planning-migration-command-contract-pass-2
kind: review-result
status: active
title: Planning migration command contract final review, pass 2
relations:
- reviews: story:eventlog-planning-authority-migration
revision: 1
---
# M3 command/config contract — final independent review pass 2 of 2

## Result

**NEEDS CHANGE.** The corrected companion closes F1, F2, F3 and F5, and it materially
improves F4 and F6. Two concrete residual blockers remain inside the original finding classes:

1. the supported unavailable-order complete-envelope case has no closed representation that keeps
   its exact envelope and immutable record-ID reservation inside the provider-complete public
   consumer transcript (F4); and
2. the one-outcome/one-identity mutation contract cannot represent retained CLI operations that
   execute several separately committed commands, including a later refusal or projection failure
   after earlier commits (F6).

This is the final planned whole review. It does not start a third review unit. The two findings are
bounded contract corrections under the existing M3 owner.

## Frozen review inputs

The AEP candidate was inspected at
`715e52a88df9abcfcdd35752c09ab64b1e5a3fb3`, tree
`d836f759597bd95797ff39968b5802be57824053`. The three frozen input hashes match the brief:

- accepted direction:
  `6ddf871c246a01fe8b6acb8754411e898206b16588c80062dbccbe19dc29a2d6`;
- corrected command/config companion:
  `c69e81c8e2224d44bcd58e457d9da69b4079e4a116bb3714a01b979066a37a33`;
- corrected diagnostic model:
  `f38172f2235cdaf326ff7113f58f67f14df92f877062f7e67bbdc1c2286fc0fb`.

The original pass-1 report is
`1d5e74e7a78cab83f6453c51d926cd9542db1b792209da3a3fedf085a84cc613`.
The correction report is
`5e3870ca469de347111639b2aba78fd6137d485599116047f476f1d769f596b4`.
The public Entity Runtime files used for seam analysis are byte-identical to the files recorded by
pass 1 even though that managed tree has since advanced for unrelated accepted work; their exact
hashes are in `input-manifest.tsv`.

The AEP worktree already contained the planning/design changes and planning records listed by
`0001-input-verification.log`. This reviewer made no repository change.

## Findings

### F4-R1 — unavailable-order complete envelopes do not cross the provider-complete public read boundary

**Blocker.** The corrected contract explicitly admits a supported source witness consisting of a
complete legacy envelope with exact bytes and an original global record ID but without provable
subject or per-kind order. It correctly refuses to coerce that witness into
`ImportedRecordEvidence` and instead promises a `LegacyRecordCoordinate`, an immutable
reservation, and queryable exact evidence
(`planning-store-selection-and-commands-v0.1.md:567-583`).

The public adapter boundary cannot carry that envelope as imported history:
`KnownLegacyOrder` has only `PerKind(u64)` and `Subject(u64)`;
`LegacyEvidence::Envelope` requires an `ImportedRecordEvidence`; and
`RecordLookup::Imported` exposes only that same ordered type
(`entity-store/src/asynchronous/types.rs:457-479,509-523,580-587`).
That limitation is acceptable only if the proposed AEP boundary subject carries the missing exact
evidence and reservation through the ordinary provider-complete transcript.

It currently does not. The companion defines each boundary subject with source snapshot, source
locator, destination subject, evidence kind, original-record-ID presence and order/ordinal
(`:560-565`). The diagnostic entity likewise has no envelope bytes, evidence-blob digest or
typed blob coordinate (`migration.yaml:136-156`). The exact source bytes remain in
`recovery/raw-capture.json` beside the selector (`:508-512`), while
`AepCompleteStoreTranscriptV1` is built from the provider's subject histories
(`:585-605`). A source locator alone neither puts the exact envelope in
`CompleteStoreSnapshot` nor proves the promised immutable reservation set. The text later says
that destination verification enumerates “every reservation and exact evidence blob” (`:574-581`)
but binds neither a closed reservation-set record nor the evidence blob/reference from which those
bytes are enumerated.

A concrete failure follows after cutover: corrupt or remove the unavailable-order envelope bytes in
the external recovery capture while leaving the `LegacyRecordCoordinate` subject unchanged.
`verify_complete_store` can still return the same complete subject transcript and therefore the
same authority-snapshot digest, while history cannot return the exact envelope and the store has no
provider-complete value from which to reconstruct or verify the original-ID reservation. A new
record using that original ID is then prevented only by an implementation-private side table whose
durable source is not fixed by this contract.

This leaves the accepted full-history/global-ID preservation requirement open. The correction can
remain AEP-local: bind a closed ordinary subject/blob representation for the exact unavailable-order
envelope and the complete immutable reservation roster, include both in the provider-complete
transcript and authority snapshot, and define the history/lookup join from those values. No new
Eventlog administration interface follows from this finding.

### F6-R1 — singular mutation outcomes cannot describe retained multi-command invocations

**Blocker.** The corrected mutation contract gives one optional `--command-identity`, one
`DurableCommandReceiptV1`, and one scalar `PlanningMutationResultV1` inside a singular
`PlanningMutationOutcomeV1`
(`planning-store-selection-and-commands-v0.1.md:627-674`). That closes a single recorded command,
including a batch produced by that command. It does not close existing CLI invocations that issue
several business commands.

Two current public witnesses are decisive:

- `artifact move --via` computes several hops, commits once per hop, retains the ordered
  `Vec<Moved>`, and can report already committed hops before a later refusal
  (`crates/edge/aep-cli/src/planning.rs:2337-2433,2505-2570`).
- `artifact new --relate ...` commits the entity creation and then commits each requested relation
  as its own command (`:1961-2046`).

Reusing the one supplied command identity for those distinct canonical requests would trigger the
contract's own same-identity/different-bytes conflict. Minting or deriving child identities is not
specified. The `moved` result has only one `from`, `to` and `revision`, and the `created`
result carries no ordered relation-command results. The `refused` variant carries no prior
receipts. The `committed_projection_failure` variant carries only one receipt/result, so it cannot
state which earlier steps committed, which step's projection failed, or how retry resumes without
re-executing any successful step.

Bundling each invocation into one new atomic business command would also be an unbound semantic
change: the retained implementation deliberately records each hop/relation as the same independent
command used by its standalone verb, and the move surface exposes partial committed progress before
a later refusal. The contract must either preserve that behavior with a closed ordered
multi-command outcome, deterministic child identities, every original receipt/result and exact
resume rules, or explicitly bind and accept a different atomic behavior with compatibility
evidence. The current singular contract cannot satisfy the original “every ordinary mutation”
acceptance.

## F1–F6 disposition

| Original finding | Final disposition | Evidence |
| --- | --- | --- |
| F1 — five store result families lacked closed schemas/render/exit/refusal | **Resolved.** | Scalar grammars, closed refusal and coordinate enums, exact five roots/outcomes, deterministic JSON/text and exits are fixed at companion lines 249-398 and 682-704. The remaining ordinary-mutation defect is F6, not a reopened store-family schema defect. |
| F2 — durable migration identity/layout/selector/marker bytes were unbound | **Resolved.** | Migration-key derivation, complete owned layout, exact intent/marker/phase/current/receipt records, exclusive create/atomic replace/fsync rules and two-copy reconciliation are fixed at lines 399-525. |
| F3 — v2 lacked all public `Authority` coordinates | **Resolved.** | `planning_scope`, `planning_tenant` and `planning_identity` map byte-for-byte to `logical_scope`, `tenant` and `stream_identity` at lines 18-47 and 399-430, matching `entity-eventlog/src/encoding.rs:21-45`. |
| F4 — retained history/order/evidence could not map losslessly | **Needs change.** | Backend mapping, ordered-envelope import and typed store/global order improve the contract, but F4-R1 leaves the explicitly admitted unavailable-order complete envelope and its ID reservation outside the closed provider-complete transcript. |
| F5 — rebuild authority position was unavailable | **Resolved without a native frontier API.** | Lines 585-618 define a domain-separated digest over a complete canonical transcript and double capture. The public seam is reachable: `AsyncRecordedReader::complete_snapshot` returns the data, and an AEP-owned delegating reader can retain the exact snapshot that `verify_complete_store` invokes and validates; this requires no provider administration hook. The digest is honestly not called a native Eventlog position. |
| F6 — ordinary retained mutations lacked receipt-bearing committed-failure semantics | **Needs change.** | Single-command success/failure/retry is defined at lines 620-680, but F6-R1 shows two retained multi-command invocations for which one identity/result/receipt cannot preserve partial commits or retry semantics. |

## Complete surface disposition

| Surface | Final disposition |
| --- | --- |
| v1 parser and Markdown/SQLite/PostgreSQL/all hybrid selections | Contract complete: distinct v1/v2 readers, missing-version v1 meaning and v1 compatibility are preserved. |
| v2 default/explicit selection and path safety | Contract complete under F2/F3: exact authority tuple, canonical selector bytes, containment, alias/nesting/foreign-content refusal and ordinary discovery are bound. |
| explicit legacy `--store` | Contract complete: standalone v1 remains writable; v2 projection/retired source/uncertain ownership fail closed, including an empty projection and external cwd. |
| `inspect` | Contract complete and read-only: observed unready states and resolution refusal have exact outcomes/exits without durable mutation. |
| `migrate dry-run` | Contract complete: exact source/config/authority/mapping/destination assessment and refusal classes are bound; no destination or durable lock effect is admitted. |
| `migrate apply` | Phase protocol complete apart from F4's unavailable-order content: all eight durable effects, restart rules, immutable intent, source/config rechecks, publish/switch ordering and original receipt recovery are fixed. |
| durable phase and recovery files | Contract complete: exact locations, encodings, predecessor chain, current pointer, durability observations and disagreement rules are fixed. |
| Markdown/SQLite/PostgreSQL/hybrid acquisition | No new finding. The accepted complete raw-capture values remain reused, and the companion preserves the backend-specific read-only acquisition and hybrid comparison obligations. No source or server was executed here. |
| ordinary ordered legacy envelopes | Contract complete: eligible envelopes retain original IDs and honest subject/per-kind order through `ImportedRecordEvidence`; store order is additionally retained in boundary records. |
| unavailable-order complete envelopes and reservation | **Needs change (F4-R1).** Metadata is bound, but exact evidence and the durable reservation roster are not in the provider-complete transcript. |
| auxiliary entities and historical facts | Contract direction complete: `aep.entity`, `aep.relation`, `aep.audit`, `aep.applied`, removed relations, refused audit and local-only hybrid evidence are enumerated. F4-R1 applies where their historical envelope has unavailable order. |
| `verify` and authority snapshot | Contract complete after F5: provider-complete double capture, canonical semantic digest, history assurances, boundary subjects and projection comparison are fixed. It must include the eventual F4-R1 values. |
| projection `rebuild` | Contract complete after F5: caller snapshot comparison and pre/post capture occur under the authority writer fence; owned paths, foreign conflicts and watermark are explicit; no command is re-executed. |
| single recorded ordinary mutations | Contract complete after F6 correction: identity conflict, original receipt recovery, committed projection failure, repair and output/HTTP behavior are closed. |
| multi-command ordinary mutations | **Needs change (F6-R1).** `move --via` and `new --relate` cannot be represented or retried by the singular outcome. |
| five store wires, mutation/intent/marker/phase/receipt/watermark wires | Store families and durable migration wires are complete. Mutation wire remains incomplete only for F6-R1. Strict readers/generated schemas/fixtures remain implementation obligations. |
| text/JSON/exits and `aep`/`protocol` alias equality | Contract complete for declared outcomes: compact JSON plus LF and lossless flattening are deterministic. F6-R1 needs a declared multi-command root before its bytes can be tested. |
| writer control and source retirement | Correctly external and fail-closed. Interface tests do not qualify a source; real apply remains `writer_exclusion_unavailable` until B-WRITER supplies actual control. |
| provider qualification and six real cutovers | External and unperformed. This review does not clear B-ADMIN, native provider gaps, adapter qualification, dependency admission or any operational cutover. |

## Whole-contract assessment

The correction does not permanently refuse a currently supported backend. It preserves the six
backend/policy cutover shapes and keeps missing writer control fail-closed. The selector, command
families, durable phases, recovery precedence, public three-part authority, opaque snapshot/rebuild
and single-command committed projection-failure contract are implementable from the cited public
surfaces.

The remaining gaps are not schema cosmetics. F4-R1 loses an admitted history/identity witness from
the provider-complete authority. F6-R1 leaves actual retained mutation behavior without a truthful
receipt/retry result. Implementing the current text would require an unstated side table or silently
changing existing command semantics, so the whole contract is not approved yet.

## External restrictions and non-claims

- B-ADMIN remains open. I did not inspect denied provider-administration source, reviewer trees or
  cases, and did not retry equivalent work.
- The four native provider-stage gaps, adapter qualification, AEP dependency pin and B-WRITER
  remain external. No public consumer reasoning here clears them.
- No real writer fleet, source exclusion, database, Eventlog store, migration, selector, projection,
  cutover, publication or release was exercised.
- The correction author's successful ESS model validation/compile and four JSON parses were hash-
  verified as retained evidence. They prove model syntax and examples, not closure of F4/F6.
- The repository source and planning store were read only.

## Inspected versus executed

Inspected: the complete accepted authority direction; all 744 lines of the corrected companion; the
complete corrected diagnostic model; the original pass-1 report and correction report/manifests;
the accepted raw-capture design and public values where cited; current config/CLI/backend source;
and the byte-identical public Entity Runtime authority, history, lookup, complete-snapshot and
verification interfaces listed in `input-manifest.tsv`.

Executed: read-only Git identity/status, SHA-256/byte-count checks, source searches and numbered
source reads, plus managed-worktree lease operations. The three source-inspection commands and
their exact exit 0 results are retained as `0001`–`0003`. No Cargo, formatter, test, ESS command,
SQL, server, browser, AEP planning command or operational adapter command was run.

## Final pass closure

This completes pass 2 of 2 with two finite residual findings. Correct F4-R1 and F6-R1 under the
existing M3 owner, preserving all resolved clauses and external restrictions. There is no third
whole review budget.
