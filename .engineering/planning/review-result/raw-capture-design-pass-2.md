---
format: aep.planning-md/1
id: review-result:raw-capture-design-pass-2
kind: review-result
status: active
title: Raw capture contract final technical design review
relations:
- reviews: story:eventlog-planning-authority-migration
revision: 1
---
needs-revision

story:eventlog-planning-authority-migration — Complete Hybrid capture still requires HybridPolicyWordsV1, but no SourceCoordinateV1 or PhaseEvidenceV1 field carries those words, so the mandated phase-evidence reconstruction cannot construct or check the policy and refused or unstable observations lose an obtained policy; add an explicitly present-or-missing resolved-policy value to the hybrid coordinate or evidence, bind it to phase validation, and reconstruct HybridRawV1.policy from it. — docs/design/planning-raw-capture-v0.1.md:80

What I read: one planning artifact (story:eventlog-planning-authority-migration revision 14), the complete revised raw-capture proposal, the accepted parent design, the migration coordinate specification, the immutable pass-one report, the pass-two brief, repository/workspace/organization instructions, the planning skill and critic rubric, and pinned Entity Runtime source excerpts; read-only commands were `git rev-parse`, `git status`, `sha256sum`, `rg`, `wc`, `nl`, `sed`, and `cat`.

Identity checked: AEP HEAD `4eb999e0ae3cc77d1c387152e23a85ad4eae86dc`; revised proposal SHA-256 `e23b99297e2aeb1b650d28a06db1a23a1598b46f12084280771cfa63f1dea47a`; story SHA-256 `916e89b6440f2783932371e72cb604df903476c813fdb3ad8083acb768ba44d0`; immutable pass-one report SHA-256 `45396152351630bb0e5b158d001786cdc74d098ff9e09f42fa34259653654f10`; pinned Entity Runtime HEAD `faadc04f2f273517e21815d32ba3866f3aea7642`.

Pass-one disposition: all five prior findings are resolved by the exact phase rosters and successful prefixes, NotAttempted states, partial catalog records, host-independent relative-path grammar, and canonical comparison/reconstruction rules in the revised proposal; none survives verbatim.

Run mode: non-interactive sub-agent review; no operator stop or AEP mutation was available or required.

What remains unestablished: the authored digest constants, Serde/schema implementation, generated artifacts, tests, builds, and runtime behavior were not executed or verified.

What remains unestablished: catalog query/openers, live SQL acquisition, credentials, writer fencing, durable migration phases, semantic/import/report contracts, qualified ER/Eventlog runtime, and all six cutovers remain outside this pure-value review; the separately rejected SQL prerequisite review was not attempted.

The full owning-story scope remains intact. This final technical-design finding concerns only a value required by the revised pure DTO's own reconstruction and evidence-preservation rules; it does not require deferred IO, runtime, semantic import, or cutover work to be implemented in this unit. No third technical-design round follows.

```findings
- file: docs/design/planning-raw-capture-v0.1.md
  line: 80
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: Complete Hybrid capture still requires HybridPolicyWordsV1, but no SourceCoordinateV1 or PhaseEvidenceV1 field carries those words, so the mandated phase-evidence reconstruction cannot construct or check the policy and refused or unstable observations lose an obtained policy; add an explicitly present-or-missing resolved-policy value to the hybrid coordinate or evidence, bind it to phase validation, and reconstruct HybridRawV1.policy from it.
```
