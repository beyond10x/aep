---
format: aep.planning-md/2
id: review-result:adversary-stage-dir-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: failed publication leaves no stage directory'
relations:
- reviews: story:a-failed-publication-leaves-no-stage-directory
revision: 1
---
# Adversary pass 1 on story:a-failed-publication-leaves-no-stage-directory (wave 0021)

Verdict: nothing found. No case added; one note.

Attacked and not broken: wrong-directory removal, publish races, the test-only fault switch, missed error paths, recovery of a killed process's stage.

```findings
- file: crates/plan/aep-planning-migration/src/projection.rs
  line: 83
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: StageDirectory::create removes an existing directory at its pid-and-counter name, which contradicts the doc claim at :74-76 that the owner removes only what it created; it is reachable only when two pid namespaces share one checkout.
```
