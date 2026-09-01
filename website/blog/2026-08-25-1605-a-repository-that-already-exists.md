---
title: "0.14 — the protocol reads a repository that already exists"
description: >
  Release 0.14.0 adds protocol reverse: four verbs that point the tooling at a repository written
  with none of this in mind. Three of the four write nothing, so you can run them against your own
  code before deciding anything.
slug: a-repository-that-already-exists
tags: [release, aep, adoption]
date: 2026-08-25T16:05:51+02:00
release_tag: "0.14.0"
release_commit: 1fc6bf878fd45ac0f5e01d23b03235133bba7c75
---

Every worked example on this site starts from documents that already exist. That is the wrong end
for anybody deciding whether to adopt: your repository has a README, a test layout, a CI file and
six years of git history, and none of it was written with a protocol in mind.

`0.14.0` adds `protocol reverse` — four verbs that read what a repository *already says about
itself*. **Three of the four write nothing at all**, which is the part that matters when you are
evaluating a tool you do not trust yet: the worst case is a report you disagree with.

{/* truncate */}

## `reverse scan` — what the repository says about itself

```shell-session
$ protocol reverse scan .
aep.reverse-scan/1

readme headings: 119
  README.md:1  aep
  README.md:19    Two halves, one seam
  README.md:34    What that looks like
  README.md:76    Is this for you
  README.md:89    Where it sits
  README.md:104    Status
  ...
```

Headings, declared toolchains, gates, test layout — assembled into an `aep.reverse-scan/1` bundle
that renders as `text`, `yaml` or `json`. Nothing is written.

## `reverse history` — what the git history says

This is the one that surprises people. Run against this repository:

```shell-session
$ protocol reverse history . --top 5
aep.reverse-history/1

span: 213 commits, 1 author(s), 2026-08-19 -> 2026-08-26
tags: 33  [0.23.2, 0.23.1, 0.23.0, 0.22.0, 0.21.0]

commit types: feat 95  docs 66  fix 22  chore 18  ci 3  refactor 3  test 3  style 2
tickets: 4
  W4-2  5 commit(s), last 2026-08-24
  W4-1  3 commit(s), last 2026-08-22
reverted: 0
stated expiry: 1
  2026-08-20 87310c3  refactor(conformance): drop the per-suite workaround for colliding identifiers
churn: 5
  CHANGELOG.md  109 commit(s), 1 author(s), last 2026-08-26
  AGENTS.md  39 commit(s), 1 author(s), last 2026-08-25
  crates/protocol-cli/src/app.rs  36 commit(s), 1 author(s), last 2026-08-25
dormant: 0
```

**Churn** is where change concentrates. **Dormant** is what nobody has touched recently — a
different question from what is untested, and often a more useful one. **Stated expiry** finds
commits that said something was temporary, which is the class of comment that outlives its author's
memory of it.

`--recent` sets how many commits count as recent for dormancy; `--top` sets how many entries each
ranked section reports.

## `reverse openapi` — a specification you already published

```shell-session
$ protocol reverse openapi openapi.yaml --domain acme.billing
```

Drafts an `ess/1` domain from an OpenAPI document that already exists. Standard output when `--out`
is absent, because a verb that scatters files over a working tree the first time somebody tries it
is a verb nobody tries twice.

## `reverse init` — the one that writes

```shell-session
$ protocol reverse init \
    --protocols git+https://github.com/beyond10x/aep#<40-hex> \
    --profile development.standard
```

Writes the `project.yaml` that makes a repository an adopting project, and resolves the protocol
source first unless `--no-verify` says not to.

`--protocols` takes a path **or a pinned git locator**, and the pin is not decoration: a governing
document tree that could move under you is a gate whose meaning changes without a commit in your
repository.

## Where to go next

The walkthrough is
[Adopting a repository that already exists](https://github.com/beyond10x/aep/blob/main/docs/guide/adopting.md),
and the verb-by-verb tables are in the [CLI reference](/docs/reference/cli).
