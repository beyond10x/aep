---
format: aep.planning-md/1
id: story:stop-ambiguous-native-retries
kind: story
status: draft
title: Let a consumer stop after one ambiguous native launch
relations:
- informed_by: story:retry-budgets
- serves: vision:O3
revision: 1
---
## Failing consumer contract
A workflow consumer requires a successful terminal native session before permitting another model launch. A following command can validate a returned transcript, but cannot govern retries that happen inside the preceding LLM step. Infrastructure failure after session start can leave cost and completion unknown. Repeating the launch before returning control violates that consumer boundary.

## Evidence
In the installed AEP 0.54.0 driver, an LLM executor exiting with an infrastructure error after a partial event stream was launched a second time in the same state/step, with no intervening session-validation command. Both launch reservations remained durable. No raw adopter evidence is included here. Current repository source independently explains this behavior: crates/drive/aep-driver-spec/src/map.rs:69 sets LLM_RETRIES to 1, and Step::retry_budget at lines 515–519 always returns that value for LLM steps. RawLlmStep is closed and declares no retry control. The regression at crates/drive/aep-driver/tests/driving.rs for an exhausted no-verdict retry budget covers the existing behavior.

## Requested outcome
Provide an explicit supported consumer policy that returns control after the first failed or ambiguous native launch, preserving the attempt, partial transcript and unresolved reservation. Preserve compatibility for retained maps/runs. Explicit recovery remains a separate caller decision. Successful completed proposals may still be retried by the workflow's declared validation transitions. The implementation and contract choice belong to AEP owners; this is a bounded consumer request, not an implementation or release.

## Acceptance
Use an offline fake executor that records launch count and yields (a) a partial native stream followed by failure and (b) no stream followed by failure. With the selected no-retry policy, each case launches once, emits no successful completion evidence, retains its unresolved reservation and returns an honest incomplete/refused result. Resume cannot silently reset authority and repeat the ambiguous launch. Demonstrate that declared completed-proposal retry behavior and legacy maps remain compatible. Include deterministic tests, contract/schema handling as applicable, and the repository gate before release.

## Consumer mitigation and limit
For an attended numbered recovery the consumer can grant one launch reservation equal to the full run cap, then pause at the first iteration and review exact native evidence. This keeps a second launch outside that run's authority but is not a substitute for an explicit no-retry contract in multi-visit workflows. It does not establish zero cost for an empty transcript.

## Handoff
Next owner: AEP driver agents. Next action: choose the supported compatibility policy and implement the failure-count fixtures before a release. No cross-product implementation, merge or release is performed by this requesting session. One scoped request, so no decomposition critic panel is needed.

```scope
repo: beyond10x/aep
path: crates/drive/aep-driver-spec/src/map.rs
path: crates/drive/aep-driver/src/run.rs
path: crates/drive/aep-driver/tests/driving.rs
path: schemas/generated/**
path: .engineering/planning/**
```
