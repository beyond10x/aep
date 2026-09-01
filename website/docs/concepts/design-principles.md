---
title: Design principles
sidebar_position: 2
description: The behavioral rules AEP enforces and the limits it states explicitly.
---

# Design principles

## Evidence, not assertion

Completion predicates read evidence records, not prose claims. A record identifies its kind,
producer, observation time, subject, and relevant revision. “Tests pass” is text; a `test_result`
with `failed == 0` is a fact the engine can evaluate.

**Limit:** producer independence is structurally checked from the submitted identity. AEP does not
cryptographically prove who controlled that producer.

## Unknown is not false

A failed check is a contradiction. A missing or stale check is unknown. Both block a requirement,
but they demand different responses: fix the first; observe the second.

## Deny by default

Capabilities start denied. Profiles may grant only what no stronger layer denies. A later document
cannot silently grant back an explicit denial.

## Approval binds to a revision

An approval of version 3 does not approve version 7. The relation between approval and artifact
revision is part of the evidence record rather than an assumption made from chronology.

## Deterministic core, named edges

Domain and engine code use ordered collections and caller-supplied time. Filesystems, processes,
networks, stores, and models exist only at named edges. This keeps replay meaningful and refusals
testable.

## Parse, then validate

Raw input types accept syntax. Domain constructors validate semantics and accumulate independent
problems. Validated types are not deserialized directly, so a value cannot acquire invariants merely
because JSON had the right field names.

## Refusals are results

A refusal names the rule, missing fact, or illegal move and changes nothing. Drivers and providers
must preserve that property; an error after a partial write is not a refusal.

## The reference driver proves a boundary

The repository ships one driver so the engine contract has a caller. The driver is not the product
boundary for models, credentials, marketplaces, or routing. Those remain explicit external inputs.

## Separate domains meet through reports

System modeling and agent plugins are independent products. ESS can contribute conformance evidence
only through its standalone report and the optional AEP adapter. Agent plugins are selected only at
the execution boundary. Repository co-location is not used as hidden authority.

## Public bytes are a contract

Generated schemas and compatibility-command output are held byte-for-byte. A serialized meaning,
identity rule, digest rule, or command behavior changes only through an explicit migration with its
consumers.
