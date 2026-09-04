---
title: Glossary
sidebar_position: 4
description: AEP terms, defined once.
---

# Glossary

| Term | Definition |
|---|---|
| **AEP** | Agentic Engineering Protocol: the generic substrate governing how engineering work is performed and proven complete. |
| **ADP / AOP** | Development and operations profiles over AEP, with profile-specific workflows and vocabulary. |
| **Protocol document** | The vocabulary declaration: capabilities, evidence kinds, verifiers, phases, and observable fact families. |
| **Principle** | One enforceable rule: when it applies, what it requires, by when, and who may attest it. |
| **Profile** | A bundle of protocol, workflow, principles, capability policy, and completion condition. |
| **Workflow** | A validated state machine whose transitions are guarded by predicates over evidence. |
| **Phase** | A label on workflow states that principles use to time obligations across different workflows. |
| **Task** | Governed work: objective, kind, profile, context facts, and artifact manifest. |
| **Artifact** | A referenced engineering document with a kind, lifecycle status, provenance, and relations. |
| **Capability** | A named permission. It defaults to deny; deny outranks approval-required, which outranks allow. |
| **Approval floor** | A capability that no profile may grant outright; a matching operation always requires approval or remains denied. |
| **Evidence** | A typed observation with kind, producer, subject, provenance, and observation time. Predicates read facts projected from it. |
| **`observed_at`** | When somebody looked, distinct from when a record was submitted. |
| **Horizon** | The age after which an observation stops satisfying a requirement and becomes Unknown again. |
| **Producer** | The agent or verifier that made an observation. Independent requirements accept verifier-produced records only. |
| **Truth** | `True`, `False`, or `Unknown`. Only True permits a guarded transition. |
| **Harness** | The external system that runs an agent and asks AEP what is owed, permitted, and done. |
| **Execution** | One task's run through a workflow: state, evidence order, events, and audit trail. |
| **Step map** | An `aep.driver-steps/1` document saying what a harness does in each workflow state. |
| **Driven run** | A workflow walk by `aep drive`; the driver asks the engine and performs only the steps it permits. |
| **Planning store** | Markdown artifacts and an append-only journal, written only through `aep plan artifact`. |
| **Backend** | An implementation of the AEP storage command and query contracts. |
| **Contract conformance** | Whether a backend implements the AEP storage contract, checked by `aep plan conformance`. |
| **ESS conformance report** | A standalone report emitted by ESS and optionally converted into AEP evidence by `aep-ess-evidence`. |
| **Trace specification** | Typed expectations over an agent transcript; `aep observe trace check` judges the finished run three-valued. |
| **Gate** | The repository's authoritative `task check` entry point. |
