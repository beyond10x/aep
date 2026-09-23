---
format: aep.planning-md/1
id: review-result:raw-capture-design-pass-1
kind: review-result
status: active
title: Raw capture contract technical design review round 1
relations:
- reviews: story:eventlog-planning-authority-migration
revision: 1
---
needs-revision

story:eventlog-planning-authority-migration — Refused carries only one evidence value, so a successful first Markdown scan followed by a failed second scan or a hybrid failure after its SQL snapshot cannot retain every acquired pass; revise the outcome to carry ordered, phase-labelled evidence for every method and define which phases are required or may be absent. — docs/design/planning-raw-capture-v0.1.md:60
story:eventlog-planning-authority-migration — SqlEvidenceV1 requires all five row-family observations but EnumerationTerminalV1 has no not-attempted state, so an early failure forces later families to claim either an empty complete enumeration or a refusal that never occurred; add an explicit not-attempted stage state and method-order validation. — docs/design/planning-raw-capture-v0.1.md:71
story:eventlog-planning-authority-migration — Refused SQL evidence can store only a wholly present or missing SqlSchemaV1, which cannot retain a catalog enumeration that obtained some tables, columns, or constraints before refusal while keeping metadata absent distinct from query skipped; add per-catalog-family observed lists with terminal coordinates and codes, and admit SqlSchemaV1 only after all are complete. — docs/design/planning-raw-capture-v0.1.md:71
story:eventlog-planning-authority-migration — The relative HostPathV1 rule does not define a platform-independent component grammar for Unix bytes and Windows UTF-16 units, so validators on different hosts must invent separator, prefix, NUL, and unpaired-surrogate behavior; specify those lexical rules and their canonical refusal cases. — docs/design/planning-raw-capture-v0.1.md:32
story:eventlog-planning-authority-migration — Unstable requires multiple attempts and a nonempty changed set but does not define attempt cardinality or phase compatibility or derive changed coordinates from compared evidence, so contradictory evidence can validate; define the comparison relation and require changed to equal its canonical diff. — docs/design/planning-raw-capture-v0.1.md:88

What I read: one planning artifact (story:eventlog-planning-authority-migration revision 13), the complete raw-capture proposal and accepted parent design, the migration coordinate specification, the repository/workspace/organization instructions, the planning skill and critic rubric, and pinned Entity Runtime plus relevant AEP source excerpts; read-only commands were `git rev-parse`, `git status`, `sha256sum`, `rg`, `rg --files`, `wc`, `nl`, and `sed`.

Identity checked: AEP HEAD `4eb999e0ae3cc77d1c387152e23a85ad4eae86dc`; proposal SHA-256 `da641cb8da2fad651c907e86bd03e503a157595e43efa4da2c96beeb1ebb75cd`; story SHA-256 `175668d38d7b3cb56125c5ff743101634b544b05f2fdc2362f28b853f39b6241`; pinned Entity Runtime HEAD `faadc04f2f273517e21815d32ba3866f3aea7642`.

What remains unestablished: the authored digest constants, Serde/schema implementation, generated artifacts, tests, builds, and runtime behavior were not executed or verified.

What remains unestablished: catalog query/openers, live SQL acquisition, credentials, writer fencing, phase and semantic/import/report contracts, qualified ER/Eventlog runtime, and all six cutovers remain outside this pure-unit review; the separately rejected SQL prerequisite review was not attempted.

The full owner scope remains intact. These findings address only whether the independent raw-value unit can represent its claimed evidence and be implemented deterministically without inventing IO facts; they do not require the remaining migration work to be implemented in this unit and do not claim any operational evidence.

```findings
- file: docs/design/planning-raw-capture-v0.1.md
  line: 60
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: Refused carries only one evidence value, so a successful first Markdown scan followed by a failed second scan or a hybrid failure after its SQL snapshot cannot retain every acquired pass; revise the outcome to carry ordered, phase-labelled evidence for every method and define which phases are required or may be absent.
- file: docs/design/planning-raw-capture-v0.1.md
  line: 71
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: SqlEvidenceV1 requires all five row-family observations but EnumerationTerminalV1 has no not-attempted state, so an early failure forces later families to claim either an empty complete enumeration or a refusal that never occurred; add an explicit not-attempted stage state and method-order validation.
- file: docs/design/planning-raw-capture-v0.1.md
  line: 71
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: Refused SQL evidence can store only a wholly present or missing SqlSchemaV1, which cannot retain a catalog enumeration that obtained some tables, columns, or constraints before refusal while keeping metadata absent distinct from query skipped; add per-catalog-family observed lists with terminal coordinates and codes, and admit SqlSchemaV1 only after all are complete.
- file: docs/design/planning-raw-capture-v0.1.md
  line: 32
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: The relative HostPathV1 rule does not define a platform-independent component grammar for Unix bytes and Windows UTF-16 units, so validators on different hosts must invent separator, prefix, NUL, and unpaired-surrogate behavior; specify those lexical rules and their canonical refusal cases.
- file: docs/design/planning-raw-capture-v0.1.md
  line: 88
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: Unstable requires multiple attempts and a nonempty changed set but does not define attempt cardinality or phase compatibility or derive changed coordinates from compared evidence, so contradictory evidence can validate; define the comparison relation and require changed to equal its canonical diff.
```
