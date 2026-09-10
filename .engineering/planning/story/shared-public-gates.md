---
format: aep.planning-md/1
id: story:shared-public-gates
kind: story
status: implemented
title: Adopt independent common source gates
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: .github/workflows/shared-gates.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
revision: 5
---
## Intent

The operator-approved Public shared gates, independent of Atlas plan adopts Eventlog, then ESS and AEP after verified producer publication. This story owns AEP adoption. Gates story:public-shared-gates owns producer implementation; Atlas task:eventlog-source-gate-reuse coordinates the authority migration. AGENTS.md refers to a missing repository-local planning skill; the installed aep-plan skill supplies the current CLI-governed workflow. The installed AEP binary predates this checkout's ESS evidence vocabulary, so mutations use the CLI built from this exact checkout.

## Acceptance and scope

Install an immutable beyond10x/gates reusable workflow and coordinated local hooks against current-main baseline 9939b0d1b83747c216a79f127c3f2dd6936f7990. Require the shared check before integration, enable supported GitHub secret scanning and push protection, and retain every AEP correctness and release requirement. Use the same bot identity without an Atlas checkout or organization-wide admission in commit/publish paths. Keep private policy outside public source. Document historical findings separately; do not rewrite history. Scope: AGENTS.md, CHANGELOG.md, .github/workflows/shared-gates.yml and this planning record. No AEP protocol semantics or downstream release changes.

## Evidence required

Published immutable producer and verified assets; local shared gate and receipt reuse; task check; trusted base-branch workflow observation; selected-repository policy secret and required status/security settings readback. Source publication is authorized; AEP release is outside scope.
