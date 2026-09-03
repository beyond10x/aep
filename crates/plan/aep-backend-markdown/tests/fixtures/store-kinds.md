# Which kinds the equivalence test must cover

`kernel_equivalence.rs` compares the kernel's verdict with `ArtifactLifecycle::permits_transition`
for every kind below and every ordered pair of `ArtifactStatus`. The list is committed rather than
read from the stores at test time, for the same reason the ladder pin on the other side is
committed: a test that reads a sibling checkout says something different on a machine that does not
have one, and a kind that quietly leaves a store must be a decision, not a silent loss of coverage.

| store | commit | kinds in use |
|---|---|---|
| `aep` (this repository) | working tree, 2026-08-25 | `story` 62, `task` 25, `epic` 8, `specification` 3, `initiative` 1 |
| `agentic-principles` | `8c1460b`, 2026-08-25 | `story` 5, `specification` 1, `initiative` 1, `epic` 1, `architecture-decision-record` 1 |

`agentic-principles` ships no `artifacts/lifecycles/` of its own: it is governed by the nine
documents in this repository, which is why those nine are the fixture and its store contributes
only the kinds it uses. Both sets are covered by the union below.

The test also covers every kind this repository declares a ladder for — `design`, `review-result`,
`vision` — even where no store holds one yet, plus two cases no store can produce on its own: the
permissive fallback a kind with no ladder anywhere in its lineage gets, and a custom kind that
reaches a ladder through `ArtifactKind::parent`.

`vision` joined on 2026-08-25 and is the first ladder here that is **not** a unit of work and **not**
implementable: it is `specification`'s ladder with `implemented` removed. It is covered for the same
reason `design` is — the ladder exists, so the kernel has to agree about it whether or not a store
holds one yet.

```
architecture-decision-record
blocker
design
epic
executable-system-specification
initiative
obligation
outbound-claim
review-result
specification
story
task
vision
```

## Refreshing this

Re-derive with `grep -h '^kind:' <store>/.engineering/planning/*/*.md | sort -u`. A kind that
appears in a store and not here is a gap in the comparison; the test fails on it by name.
