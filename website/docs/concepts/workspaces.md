---
title: Workspaces
sidebar_position: 8
description: One question across several repositories' plans — members, cross-repository edges, and why a member nobody has checked out is a normal condition.
---

# Workspaces

A plan often spans several repositories: a story in one waits on a story in another.
`.engineering/workspace.yaml` names those repositories, and `aep plan workspace …` answers across
all of them.

```yaml
version: aep.workspace/1
members:
  - name: shop          # this repository, named explicitly
    source: ..
  - name: accounts
    source: ../../accounts
  - name: billing
    source: git+https://github.com/example/billing#0123456789abcdef0123456789abcdef01234567
```

`source` takes the same kind of locator as `project.yaml`'s `protocols:`. It can be a path relative to
`.engineering/`, or a `git+ssh://`, `git+https://` or `git+file://` URL **pinned** to a 40-hex commit.
Absolute paths are refused, because they are true on one machine and false in CI.

## Members

```shell-session
$ aep plan workspace members
3 member(s) in .
  shop                     ok          .engineering/planning
  accounts                 ok          ../accounts/.engineering/planning
  billing                  unresolved  git+https://github.com/example/billing#0123456789abcdef0123456789abcdef01234567
    pinned Git member; pass --fetch to materialize it
```

A member that is not available locally does not stop the answer: the other members still answer, and
the output says which member was not read. `members --fetch` materializes a pinned Git member. It is
off by default, because it is the one workspace command that reaches the network.

## Across members

```shell-session
$ aep plan workspace list --kind story
shop/story:guest-receipt                             draft        Email a receipt to a guest
shop/story:pay-by-card                               implemented  Pay by card as a guest
shop/story:refund-guest                              draft        Refund a guest order
shop/story:save-card                                 draft        Save a card for next time
accounts/story:legacy-login                          active       Log in with a password
5 artifact(s) across 2 member(s)
1 member(s) not read:
  billing: pinned Git member; pass --fetch to materialize it
$ aep plan workspace show story:legacy-login
accounts/story:legacy-login  active  Log in with a password
```

`show` takes `kind:name`, or `member/kind:name` when more than one member holds the id.

## Edges that cross a member boundary

An edge into another member is spelled `<member>/<kind>:<name>`. `relate` checks the target against
this repository's own store, so it cannot create a crossing edge. Write it by hand in the artifact's
`relations:` list:

```yaml
relations:
- decomposes: epic:guest-checkout
- depends_on: accounts/story:legacy-login
- depends_on: billing/story:invoices
```

```shell-session
$ aep plan workspace crossings
shop/story:save-card depends_on accounts/story:legacy-login  [accounts]
shop/story:save-card depends_on billing/story:invoices  [absent]
2 crossing relation(s), 1 unresolved, 0 cycle(s)
1 member(s) not read:
  billing: pinned Git member; pass --fetch to materialize it
```

`crossings` exits `0` even with unresolved edges, because an edge into a member you have not checked
out is not a defect. `--strict` exits `1` when any crossing does not resolve. Use it as a gate in a
workspace where every member is present.

An edge whose member prefix names this repository's own member is a local edge, and `validate`
checks it like any other.
