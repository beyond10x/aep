---
format: aep.planning-md/1
id: story:structured-model-output-contract
kind: story
status: proposed
title: Declare and preserve a structured model output contract
relations:
- serves: vision:O3
revision: 2
---
## Consumer contract
The brain engine asks a tool-free governed model step for one JSON proposal batch and independently parses and validates it before any kernel write. A successful model terminal event can still contain malformed JSON. Strict refusal is correct, but a long-running consumer cannot treat native session success as a syntactically usable proposal.

## Requested capability
Provide an explicitly declared structured-output schema for an LLM step, bound to immutable map identity and forwarded through the owning metaharness capability contract when the selected provider supports it. Report unsupported capability before launching, preserve the provider's structured result and original transcript, and retain all cost and refusal evidence. Omitted schema keeps current behavior. Brain still performs its own exact-input, revision, reference, semantic and kernel admission checks; schema-conforming JSON is not a correctness guarantee.

## Evidence and acceptance
The current public reference step-map and Claude launch surfaces do not expose a JSON output-schema control. A neutral malformed result such as {"items":[{"id":"sample","reason":"text"} demonstrates the syntactic failure class without including adopter evidence. Tests should distinguish native model success, syntactic/schema conformance, downstream refusal and failed provider completion, including unsupported adapters, map resume identity, result extraction and cost retention. Coordinate provider support with metaharness through its public contract. Do not repair or rewrite original model output to manufacture success.

This is a consumer request for the owning AEP agents. No AEP or metaharness implementation, merge or release is performed by the brain session.
