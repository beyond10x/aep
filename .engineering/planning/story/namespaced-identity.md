---
format: aep.planning-md/1
id: story:namespaced-identity
kind: story
status: implemented
title: An identity is unique across members, and an ambiguous one is refused
summary: 'Two repositories can hold a story of the same name and mean different things. Decide the spelling once, and refuse an ambiguous reference by name rather than resolving it to the nearest match.'
relations:
- decomposes: epic:one-cli-many-repositories
- depends_on: story:workspace-manifest
revision: 4
---
# Story: An identity is unique across members, and an ambiguous one is refused

## Outcome

Nobody is handed the wrong story. Two members can hold a story of the same name and mean different
things by it; asked across a workspace, the command says which members hold it rather than picking
one.

**A correction.** This story and the 0.25.0 changelog both said `story:passkey-login` *exists in more
than one repository today*. It does not — it is held by no member of the shipped workspace, and the
behaviour is tested against a fixture. The mechanism was real and the example was not, which is the
shape of claim this repository exists to catch.

## Context

This is the decision the whole epic is expensive to reverse without: every reference written under
the wrong spelling would have to be rewritten. The temptation is to rename ids on the way into the
assembly, which breaks the property that reading a store through an assembly and reading it alone
give the same artifacts.

## Acceptance

Membership is carried **beside** the id, never folded into it — no document is rewritten and no id
is renamed; a reference qualified by member (`entity-runtime/story:typed-references`) answers from
that member alone; an unqualified reference held by more than one member resolves to
`Resolution::Ambiguous` listing every holder, and the lookup returns **no document** for it, because
returning *a* document for an ambiguous reference is the guess this path exists to refuse; an
unqualified reference is not the same thing as an ambiguous one, and the distinction is documented
where the type is.

## Out of Scope

A global identity registry, or any scheme that makes ids unique by construction. A member can always
be dropped from the workspace without touching a file, and that property is worth more than
uniqueness by fiat.

## Open Questions

None outstanding.
