---
format: aep.planning-md/1
id: story:preflight-complete-model-input
kind: story
status: draft
title: Expose final model input size before launch reservation
relations:
- serves: vision:O3
revision: 1
---
## Consumer contract

A governed extraction consumer must reject oversized native input before making a reservation or launching a model. Its immutable request plus map prompt can fit a harness bound while the final driver-wrapped prompt does not. AEP owns that wrapper; consumers must not recreate protocol requirement or tool-surface rendering.

## Evidence

In the compatibility source at ec60b24, crates/edge/aep-cli/src/drive.rs::prompt_for adds task identity, workflow requirements, reaching requirements and surface_lines around step.prompt. metaharness b4998a6 crates/metaharness/src/builder.rs::preload_claude_context bounds the sum of spec.prompt and declared context against 2,000,000 bytes. The installed drive CLI exposes run/status/resume/transition/eval but no offline projection of this final input. A neutral reproducer is a request close to the byte ceiling combined with a nonempty task and extraction prompt; request-only admission succeeds but the adapter refuses before terminal session evidence.

## Acceptance

An offline supported AEP operation provides the exact final launch-input sizing or a digest-bound preflight result for the selected task, state, map, document tree and context without allocating a run, reserving spend or invoking a provider, and launch detects drift from that admitted input.

## Handoff

This is an owner request from the brain consumer, not an implementation or release in this session. Brain uses an explicit conservative wrapper allowance in its fixed extraction profile pending this contract. Preserve existing runs, prompts and costs; do not infer zero charge from an empty transcript. Owning AEP agents decide the public interface and coordinate the harness contract. No consumer identities or retained private evidence are included.
