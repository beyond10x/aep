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
