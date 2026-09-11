---
format: aep.planning-md/1
id: story:empty-findings-are-recorded
kind: story
status: draft
title: Preserve empty and historical review findings without rewriting reviews
relations:
- derived_from: story:structured-findings-on-review-result
- serves: vision:O2
revision: 4
---
## Outcome

An explicitly empty findings block is treated as recorded zero findings, while
an absent block remains distinguishable and malformed blocks remain errors.

## Evidence and scope

Connectors' 2026-09-11 validation reports 139 warnings, including 88 records whose
bodies already contain a valid findings block holding an empty YAML sequence.
crates/edge/aep-cli/src/planning.rs tests parse(body).is_empty() rather than the
parser's existing opens_a_block(body), conflating absence with zero findings.
The source story structured-findings-on-review-result explicitly promises a
warning only when the block is absent. Fix the CLI diagnostic and add regression
coverage for empty, absent and malformed blocks, retaining strict behavior.

The remaining 51 consumer records use older immutable report formats. Preserve
them and add verification-report artifacts tagged review-findings-supplement,
with exactly one verifies relation to the original review-result, a
review-body-sha256 external reference containing the SHA-256 of its exact UTF-8
body, and a canonical findings block. These are transcriptions, not critic runs
or new approvals. Existing artifact kinds and document metadata suffice; this
introduces no runtime entity or lifecycle.

Reject missing or stale source binding, duplicate supplements, absent or malformed
blocks, the wrong kind or target, and supplements for already structured reviews.
Use the same resolution for validation, show (including served reads), findings
comparison and review-value counts. Expose the source artifact in show and leave
the original body, reviewer identity, review chronology and approval evidence
unchanged. Missing structure continues to warn; invalid supplements fail. The
consumer must transcribe actual findings and verdicts, never infer approval from
a filename or insert an empty block solely to silence a warning.

## Acceptance

A production CLI test distinguishes a valid empty findings block from no block,
continues to reject malformed blocks, and the affected consumer's false warnings
disappear while genuinely missing blocks remain visible.

## Verification

Run the focused regression before and after the fix, the required repository
gate, and consumer validation; preserve exact logs and source/binary identities.
Keep work isolated from the dirty AEP primary and coordinate source integration.

## Result observed on 2026-09-11

Implemented explicit-empty detection, checked source-bound verification-report
transcriptions, shared resolution across readers, and unspecified severity for
ungraded historical observations. Original body bytes and review counts remain
unchanged. Connectors consumed 51 transcriptions containing 227 historical findings,
18 ungraded, and its 283-artifact store validates without the original 139 warnings.

The empty-block regression failed before the fix. The focused findings tests and
full task check pass, including Clippy, workspace tests, generated documentation
and schemas, Rust 1.85, and the website build. PostgreSQL's optional gate is not
live qualification when ENTITY_POSTGRES_URL is absent. Raw task-owned logs are
retained under .local/tmp/review-warnings/. The full gate uses a task-owned TMPDIR
outside any project: the existing aep-project test explicitly requires that shape.
The initial in-project TMPDIR run exposed that fixture assumption; no product
source was changed for it.

Connectors pins public AEP source 4eb999e0ae3cc77d1c387152e23a85ad4eae86dc and a
reproducible patch with SHA-256
73d78e50369adf0853e457a226002bcc84e2b43a50903f28ed3e6afdd046e68f. This is a local
consumer patch on 0.55.0, not an upstream released fix. Primary AEP integration
and any release remain pending; this artifact stays draft while that handoff is
outstanding. No immutable review was rewritten or retired.
