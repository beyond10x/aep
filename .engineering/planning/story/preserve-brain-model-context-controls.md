---
format: aep.planning-md/1
id: story:preserve-brain-model-context-controls
kind: story
status: draft
title: Publish the context and model controls required by continuous brain extraction
relations:
- decomposes: epic:reference-driver
revision: 1
---
## Consumer contract and observed divergence
Continuous brain extraction requires an immutable JSON request delivered through each Claude model step's declared context files, medium reasoning effort, explicit five-minute prompt-cache duration, a bounded turn count, exact credential selection and unchanged tool denial. An installed local 0.54.0 variant supplies these controls, but the public main source at 35b5c9949d7b9e1caa3a5e5de7502af42c2dbc5e does not forward Claude step context files or the effort/cache fields in its argv builder. Replacing the local binary by the public build would regress the actual consumer contract despite sharing the same reported version.

## Requested work and ownership
The AEP repository agents should inspect the missing source contract, integrate supported controls through the owning driver/runtime boundaries and publish a verified compatible version. Keep immutable input identity, deterministic command steps, original archived maps, selected authentication and enforced hooks. Unsupported fields must refuse rather than silently disappearing. Use independently authored synthetic requests and exact child-argv/received-context tests. Include positive and refusal cases for context delivery, effort, cache duration, credential choice, turn limits and unknown fields, then run the owning gates. Do not copy private instance inputs, customer data, local paths or operator identities into public evidence.

This is a scoped consumer request. The brain session does not implement, merge or release AEP; it will validate a published replacement against the existing consumer contract before changing its active runtime pin.

## AEP 0.55 and metaharness 0.7 qualification

Rechecked against released AEP 0.55.0 (28abe09bb6e5b0a6b4db839f6bf5693957d39324) and metaharness 0.7.0 (c4caba93ca0e3fd15b9de2812559fc0239b379b5). The model execution host now moves to metaharness aep drive. Brain is beginning that consumer migration; this existing control contract remains a release blocker.

The actual AEP 0.55 executable refuses the public brain continuous-extract step map during offline workflow instruction generation: states.extract.steps: unknown field `effort`, expected one of `prompt`, `context`, `scope`, `skills`, `harness`, `description`. No model or run allocation was involved. The same public map requires prompt_cache_ttl: 5m. RawLlmStep in the released AEP source contains neither field, and the metaharness 0.7 concrete AEP host contains no effort/cache forwarding. Metaharness's underlying RunSpec does expose effort, but that does not prove the governed step carries it.

Please coordinate the neutral map vocabulary in AEP with concrete delivery in metaharness, retaining the closed reader and failing explicitly for unsupported values. Verify declared request-file delivery, medium effort, five-minute cache duration, bounded launch terms, credentials and native tool denial end to end, plus retained-map and resumed-run compatibility. Removing the fields from the consumer or silently accepting and ignoring them does not satisfy this request. Preserve previous maps and charge evidence; no private instance data belongs in fixtures. The brain session will implement only its host selection, launch/resume and deployment adoption, and will qualify the owning products' published compatible pair.

## Request continuity

This current-main request carries forward story:preserve-brain-model-context-controls from the published request/brain-runtime-contracts branch at de5fe47fcacf5ebd4232822a98737dc959398537. Its original artifact and journal remain on that branch. This fresh governed record includes the released-host qualification and descends from the repository's required Gates adoption baseline; no earlier journal event is rewritten.
