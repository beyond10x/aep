---
format: aep.planning-md/1
id: story:assemble-across-sources
kind: story
status: implemented
title: One artifact graph, built from several stores
summary: Each artifact remembers which member it came from. The failure to design for is a cycle that exists only once two members are read together - find_cycle runs per relation kind today and has never seen a graph it did not fully hold.
relations:
- decomposes: epic:one-cli-many-repositories
- depends_on: story:namespaced-identity
- informed_by: entity-runtime/story:typed-references
revision: 4
---
# Story: One artifact graph, built from several stores

## Outcome

One `board` instead of three. Somebody asks a question once and gets an answer over every member,
with each artifact still saying which repository it came from.

## Context

A `MarkdownStore` answers for one repository. The assembly is what lets the verbs answer for
several without any of them learning about members individually. The failure mode to design for is
partial success: an assembly that quietly answered from two members when it was asked about three
would give a smaller answer that looks exactly like a complete one.

## Acceptance

`Assembly` reads each member's store and keeps the member's name on every artifact; one index
resolves a reference across all members; **nothing is renamed and no id is rewritten**, so reading a
store through the assembly and reading it alone give the same artifacts; a member that failed to
load produces an **empty member rather than an absent one**, and `Assembly::failures` carries every
failure with its member's name attached; `find_cycle` holds over the combined graph.

## Out of Scope

Caching, incremental re-read, and any persistence of the assembled graph. It is built per command.

## Open Questions

None outstanding.
