# ESS complete-selection evidence

Status: accepted for implementation on 2026-09-06 under story:admit-ess-conformance-coverage
and the standing ESS remediation approval, after binding reviews 1 and 2 and root's final wording
correction. No implementation or compatibility result is established.
Owner: `story:admit-ess-conformance-coverage`. This is the reader prerequisite for ESS
`story:review-conformance-coverage`. The current source baseline is AEP
`62ef3a73112143453319215b9aa31a6c9626ed5f`, whose count reader implementation is `30aeef2`.

## Contract and typed home

The accepted ESS `docs/design/review-conformance-coverage.md` owns suite/5, standalone report/2,
the exact original-suite digest, refusal multiplicity and `complete-selection/1`. Its accepted
`docs/design/review-conformance-coverage-transport.md` binds parent/source/consumer transport.
Both bindings and Atlas ADR0040 are the coordinated implementation authority.

This document adds value representations under existing Evidence, EvidenceEnvelope and
Task.constraints. Core AEP retains IO-free typed values and optional reader interfaces; the
optional `aep-ess-evidence` adapter owns ESS wire transcription. No AEP core package compiles
against ESS modeling crates. There is no new independently identified runtime entity, authored
ESS business domain, source resolver or plugin registry.

Freeze legacy `ess_conformance` and count-only `ess_conformance_v2`, including the latter's
closed report_json/suite_json carrier, suite/1–4 allowlist, facts, APIs, expectation, policy leaves
and inability to qualify complete conformance. Report/2 alone cannot select the new coverage
reader: its paired suite/input determines which separately admitted evidence representation applies.

## Separate carrier and reader

Introduce evidence kind and fact namespace `ess_conformance_coverage_v1`, with this closed carrier:

```json
{
  "kind": "ess_conformance_coverage_v1",
  "report_json": "<original standalone report/2 JSON>",
  "suite_input_json": "<original ess-conformance-input/1 JSON>"
}
```

These placeholders explain the fields and are not executable fixtures. The ordinary evidence
envelope still carries its existing producer, subject, observation and provenance fields.

The source value has private immutable strings and a private optional admitted reading. Raw
construction/Deserialize grants no admission; Serialize and JsonSchema expose no reading,
verification flag or trusted hash assertion. Replacing a source creates a fresh raw value.
Cloning immutable sources may retain an in-process reading, but every mutation/restore boundary
re-admits the original bytes. Failed admission cannot leave an older reading available.

Add a separate pure `EssConformanceCoverageReader` interface and checked, non-deserializable
coverage reading. The current count reader interface and constructor signatures stay supported.
The new optional adapter implementation alone transcribes the shipped coverage wire. Custom Rust
reader implementations are explicitly trusted integration code, not document-selectable authority
or cryptographic attestations.

The coverage reading retains exact suite reference, complete Selection, inventory knowledge,
coverage counts and full refusal occurrences, terminal categories/outcomes, labels, model digest,
profile, recomputed statuses and exact completion instant. Its checked constructor enforces domain
arithmetic, ordering, selection and status invariants; original-byte parsing and suite transcription
remain adapter responsibilities. Reusable count/scalar helpers may be factored without broadening
the frozen count SuiteReference or constructor behavior.

## Complete original-byte admission

`ess-conformance-input/1` is exactly `{format, suite_json, parent_suites}`. The format marker is
required. Parents are original UTF-8 suite/5 strings in nearest-parent-first order. A suite hash
is over the inner original suite string's UTF-8 bytes, never the carrier or a reserialized value.
Hash every original byte, then perform strict parsing; changed layout or newline changes identity.

Reject duplicate keys and unknown fields in the carrier, every suite envelope and typed nested
structure, and every report envelope. Declared scenario payload maps remain payload data.
Admit the whole inherited suite/4 execution vocabulary under suite/5, including unused scenario
metadata/dependencies. A count-suite allowlist edit is not this admission boundary.

Admit every ancestor before its child. Exact explicit-parent references must match the next
original document; the final parent has `filter: all`. Reject missing, reordered, repeated,
surplus or legacy parents. Compare full surviving scenario bodies/dependencies, provenance,
knowledge, scope and origins, source maps, prior outside records and all refusal occurrences.
Add precisely the newly omitted IDs as selection_filter. Filtering cannot promote knowledge or
erase an in-scope gap. Iterate over the flat chain; no new chain-length semantic limit is selected.
There is no filesystem/network recovery of missing parent bytes.

The complete coverage contract remains normative: generated/authored disjoint membership,
selected/outside identity rules, final authored ownership after merge, original source digests,
known refused identities, cause-to-effect mapping, conservative component scope and sorted refusal
multiplicity. The transport's checked source-identity grammar is validated as data; AEP does not
reconstruct a source directory or authenticate a repository from a relative path.

Compare report/2's exact SuiteReference, model digest and terminal outcome union with the selected
admitted suite. Its selection, knowledge, coverage counts and complete refusal list must equal the
suite/5 summary, including every identical repeated occurrence. Recompute producer-specific
execution status and complete-selection/1 conformance status. Zero, unknown or an in-scope refusal
cannot become conformance passed. An observed failed execution stays failed.

Preserve exact unsigned count/time tokens and the full u64 range before any conversion. Use checked
arithmetic. The original embedded report JSON is checked again after JSON/YAML transport and replay;
outer transport cannot legitimize a fractional, exponent or signed report token. Preserve the
existing broader legacy ObservedAt lexical behavior separately. Observation must be an instant
exactly equal to report completion. Any narrower calendar adaptation explicitly refuses before
opening or mutating a planning store.

## Independent expected selection

Add optional closed `constraints.ess_conformance_coverage_v1` with exactly:

| Field | Meaning |
| --- | --- |
| `model` | Independently chosen executable-system-specification ArtifactRef. |
| `suite` | Exact suite/5 version, digest profile and original-byte digest. |
| `selection` | Complete ESS Selection: scope/component, origins, filter and exact parent when explicit. |
| `selected_ids` | Sorted distinct exact selected ScenarioIds, including an explicitly empty list. |

The task's existing subject remains independent and required. Resolve the specifically named model
in the current graph and require its explicit current model digest. Do not use any-model matching,
an empty-graph exception, provenance.tool or report-supplied expectations. Preserve this constraint
through RawTask, Constraints::is_empty, schema generation and plan/snapshot transport.

Qualification compares this record with the complete expected SuiteReference, full Selection and
exact selected IDs. A complete component, origin-only or explicit subset cannot satisfy a broader
expectation. An empty expected selection remains diagnostic. Matching bytes are builder coherence,
not proof that the builder honestly enumerated all obligations; choosing the expected suite is
caller authority under existing governance.

## One per-record decision

Every requirement of the new kind uses the same `qualify_record` decision in requirement evaluation
and evidence.missing. Context-free matches is false. Reject at_least=0 for this kind, including
manually constructed requirements. Only Qualified contributes to the required count, and separate
records cannot supply different halves of one qualifying observation.

Require admitted input, the task/requirement subject, an independent conformance-runner producer,
the current expected model, exact expected suite and Selection, exact observation equality, known
evaluation time and optional freshness horizon. Use checked age arithmetic, preserve horizon
equality and introduce no default TTL. Finally require nonempty complete inventory, no in-scope
refusal and recomputed all-pass execution/conformance.

Keep stable reason/path/detail diagnostics. The decision classifications are:

| Unknown | Contradiction |
| --- | --- |
| MissingAdmission, MissingExpectation, MissingTaskSubject, MissingModel, MissingModelDigest, MissingTime | InvalidRequirement, SubjectMismatch, ProducerMismatch, ModelKindMismatch, ModelDigestMismatch |
| StaleObservation, EmptySelection, UnknownCoverage, InScopeRefusal, ExecutionInconclusive | SuiteReferenceMismatch, SelectionMismatch, ObservationMismatch, FutureObservation, ExecutionFailed |

Wrong-kind records remain Unknown and cannot count. Structural, pairing and parent defects refuse
admission before they become records. Preserve the specific parser/identity issue alongside the
stable qualification reason where applicable. The old count qualifier and its existing outcomes
remain unchanged.

Facts use only the new namespace. The following table is exhaustive: every path starts with
`ess_conformance_coverage_v1.` followed by the exact suffix shown. Braces enumerate separate paths,
not a dynamic path segment. Unless the presence column says otherwise, every admitted reading
emits the fact, including zero counts and false booleans. No numeric fact passes through Number.

| Exact suffix | FactValue type and derivation | Presence |
| --- | --- | --- |
| `counts.{total,passed,failed,error,unsupported,skipped}` | Text: each exact report count as canonical unsigned decimal. | Every admitted reading. |
| `coverage.counts.{generated,authored,outside,refused}` | Text: each checked inventory count as canonical unsigned decimal; refused counts occurrences. | Every admitted reading. |
| `completed_at` | Text: exact completion epoch milliseconds as canonical unsigned decimal. | Every admitted reading. |
| `execution_status` | Text: recomputed `passed`, `failed` or `inconclusive` under the producer profile. | Every admitted reading. |
| `conformance_status` | Text: recomputed `passed`, `failed` or `inconclusive` under complete-selection/1. | Every admitted reading. |
| `producer_profile` | Text: `rust-scenario-status/1` or `go-scenario-status/1`. | Every admitted reading. |
| `policy` | Text: exactly `complete-selection/1`. | Every admitted reading. |
| `spec_digest` | Text: the admitted existing 64-lowercase-hex model digest. | Every admitted reading. |
| `suite.version` | Text: exactly `ess-conformance/5`. | Every admitted reading. |
| `suite.digest_profile` | Text: exactly `sha256-json-bytes/1`. | Every admitted reading. |
| `suite.digest` | Text: the exact inner selected suite byte digest, `sha256:` plus 64 lowercase hex digits. | Every admitted reading. |
| `coverage.knowledge` | Text: `complete_inventory` or `unknown`. | Every admitted reading. |
| `selection.scope.kind` | Text: `system` or `component`. | Every admitted reading. |
| `selection.scope.component` | Text: the checked selected ComponentName without transformation. | Component scope only; absent for system. |
| `selection.origins` | Text: `generated`, `authored` or `generated_and_authored`. | Every admitted reading. |
| `selection.filter.kind` | Text: `all` or `explicit`. | Every admitted reading. |
| `selection.filter.parent.version` | Text: exactly `ess-conformance/5`. | Explicit filter only; absent for all. |
| `selection.filter.parent.digest_profile` | Text: exactly `sha256-json-bytes/1`. | Explicit filter only; absent for all. |
| `selection.filter.parent.digest` | Text: the admitted immediate parent's exact original-byte digest. | Explicit filter only; absent for all. |
| `nonempty` | Bool: exact report total is greater than zero. | Every admitted reading. |
| `failed_zero` | Bool: exact report failed count is zero; it says nothing about errors, unsupported results or skips. | Every admitted reading. |
| `coverage_known` | Bool: knowledge equals `complete_inventory`. | Every admitted reading. |
| `coverage_complete` | Bool: complete_inventory and no refusal occurrence has scope in_scope, independently of execution and nonempty. | Every admitted reading. |

Raw/unadmitted sources emit no facts from this namespace, including no false/default status or
coverage values. Failed re-admission clears the reading before returning its error; source
replacement returns to the same raw projection. An immutable in-process clone with its reading
retained projects the same facts, while serialization/deserialization loses admission and projects
none until checked again. Existing envelope metadata and historical record-count facts retain their
separate behavior; they cannot qualify coverage.

No `passed`, `qualified`, `exists`, per-scenario, outcome-array, selected-ID-array, source-map or
per-refusal fact is added by this kind. The checked reading retains those full structures for
per-record qualification and inspection; projecting a subset of diagnostics does not truncate the
qualification input. In particular, coverage_complete may be true for empty inventory while
nonempty is false and conformance_status is inconclusive. A known inventory with an in_scope refusal
has coverage_known true and coverage_complete false. These facts describe the declared selection,
not an independent task match. Decimal Text preserves exact digits and supports quoted equality
without numeric coercion. Existing Text truthiness remains unchanged: even "0" is truthy when
present, so use nonempty and failed_zero for their specified Boolean checks. Text ordering follows
existing protocol scales and is not integer ordering. The shared qualifier alone decides
task-dependent qualification; it never combines
these facts across records. Existing count-stage facts and their presence remain unchanged.

## All mutation and replay paths

Use a concrete reader configuration with separate optional count and coverage ports, never an
ambient resolver. Existing count-only setters, constructors and parser APIs delegate without
changing their meaning; additive APIs can receive both ports. Setting one reader must preserve an
already configured other reader. Reader-free cores explicitly refuse new coverage admission.

Before mutation, prepare and re-admit every candidate record and its envelope, then perform existing
ordinary validation. A refusal preserves evidence, artifact links, events, observation and execution
state. Apply this to Engine submission, direct checked Execution recording, every snapshot record
and restoration. A serialized reading/cache never bypasses these steps.

Required current owners at the source baseline:

| Owner | Required route |
| --- | --- |
| aep-domain evidence.rs, task.rs, requirement.rs and new coverage values | Carrier/kind dispatch, raw/admitted facts, independent constraint and shared qualification. |
| aep-engine engine.rs and execution.rs | Optional installation, initialization, submission, checked direct recording, restoration and evidence.missing. |
| aep-ess-evidence | Closed report/input/suite/parent transcription and paired adaptation. |
| aep-schema parse.rs and schema.rs | Closed original-source transport, reader-bearing parsing and Rust-owned schema generation. |
| aep-cli app.rs | Evaluate/explain, typed evidence loading, inspection and exact-time display. |
| aep-cli drive.rs | Start, resume, workflow position/consultation and typed read_record. |
| aep-cli planning.rs | Independent report reader and pre-store calendar refusal. |

Existing aep-driver snapshot replay calls configured Engine::restore and observed records call
Engine::submit_evidence. Exercise actual CLI driver start/resume/consultation; no driver production
dependency on the adapter or new minting route is selected. Events remain event claims and cannot
recreate admitted source bytes.

## Explicit CLI and policy leaves

Planning adds `--suite-input` beside `--suite`, mutually exclusive and only valid with `--from`.
Report/1 refuses either pairing control, preserving its original unpaired route. Detailed run/2
is not standalone report/2 and remains refused. Manual kind/source/at/review/outcome conflicts stay.

For report/2 with `--suite`, raw suite/1–4 uses the unchanged count reader. An unfiltered raw
suite/5 uses the new coverage reader after the edge wraps its exact string in a newly issued
input/1 with no parents. Raw explicit suite/5 refuses missing lineage. With `--suite-input`, require
input/1 and suite/5 and retain the original carrier bytes. Never infer coverage by discarding fields
or route a suite/5 report through the frozen count carrier.

Planning records truthful descriptive categories, statuses, coverage, selection, refusals, exact
time, model/suite identity and actual input references after complete admission. Escape supplied
strings so names cannot obscure attribution. This history is not a typed source archive or an
independent conformance qualification. Typed evidence inspection uses the reader-bearing route,
and canonical/alias command equivalence remains required.

Choose these additive opt-in leaves at supported protocol major 1:

| File | Identity and composition |
| --- | --- |
| principles/verification/ess-conformance-coverage.yaml | ess-conformance-coverage version 1; requires the new kind, at least one independent conformance-runner record under the shared qualifier. |
| protocols/adp-ess-conformance-coverage/1.yaml | adp-ess-conformance-coverage version 1, extending adp/1 with the new kind and observable namespace. |
| profiles/development-ess-conformance-coverage.yaml | development.ess-conformance-coverage version 1, extending development.standard with the new protocol/principle. |

Do not inherit the count-only or legacy conformance requirement, change adp/1, or change defaults.
Test actual profile/protocol/principle composition. Generate every changed schema through
`cargo xtask schema`; do not hand-edit the propagated EvidenceKind or closed carrier schema.

## Verification and rollout

Retain count and legacy APIs/wire fixtures and every previous adversary assertion. Independently
author complete, incomplete, unknown and zero fixtures before the ESS writer exists. Exercise
every suite/5 inventory/source/parent/refusal invariant, exact integer boundary, changed original
byte spelling and reason-specific qualification refusal. Mutation-test each guard with real named
cases; a case name or changed fixture alone is not proof that its assertion executed.

Exercise both reader routes, typed JSON/YAML, inspection, direct recording, submission, snapshots
and actual driver replay. Include missing readers, malformed restore after valid records, cross-record
fact mixing, wrong subject/producer/model/selection, future/stale/time mismatch and no-state-change
controls. A valid complete fixture must actually satisfy both evaluation and evidence.missing.
Table-driven projection assertions require exactly the fact paths, types, values and conditional
absence listed above for raw, admitted, re-admitted, replaced and replayed sources, including empty
complete inventory and known inventory with repeated in_scope refusals. Extra aliases fail too.

The independent adversary and full sixteen-step AEP gate precede reader publication. PostgreSQL
remains explicitly skipped without its configured URL. Then freeze actual ESS Rust/Go producers
and run original report/input pairs through both readers and typed replay before ESS writer
publication. The Atlas ADR and log record exact source/public delivery and unresolved installed
callers/generated runtimes. Source publication is not installation, a release or a default change.

## Binding review record

Immutable reviews ess-conformance-coverage-binding-review-pass-1 and -pass-2 are recorded through
AEP. Pass1 report SHA256 is035d8b8fd665457e387a73edfd47fec9b40fbdc85bc6c93a774b48f3f4f8ada3;
pass2 isf13642c04236967cd71ff28096abb1907aa8e1139687243546f3192c345b058a. The exact fact table and
ESS strict-mode story corrections resolve pass1. Root checked the final Text guidance against
facts.rs:205–209 and predicate.rs:399–401 at62ef3a7, preserving both ordinary Text semantics and
the new Boolean meanings. These are document/source checks, not executed implementation tests.
