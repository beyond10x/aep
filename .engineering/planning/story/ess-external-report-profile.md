---
format: aep.planning-md/3
id: story:ess-external-report-profile
kind: story
status: active
title: Evidence admits an ESS report from an outside runner
relations:
- informed_by: epic:evidence-gated-completion
- serves: vision:O2
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T08:09:46Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T08:09:46Z", actor: "human:timo", revision: 3}
---
## Outcome

An ESS conformance report written from an outside runner's per-scenario results is recorded as
evidence, and the record keeps the fact that the results were supplied, never executed by ESS.

## Acceptance

- `aep plan artifact evidence --from <report> --suite <suite>` admits an `ess-conformance-report/2`
  whose `producer_profile` is `external-scenario-status/1` or
  `external-scenario-status/1;runner=<name>@<version>`.
- The recorded evidence and the reading's `producer_profile` fact keep the exact profile.
- Any other spelling still refuses with `UnsupportedProducerProfile`.

## Origin

ESS 0.48.0 adds `ess verify conform report`; a downstream specification runs its suite with its own
runner and needs the report as evidence to move to `conforming`.
