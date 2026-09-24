---
format: aep.planning-md/2
id: story:cross-repository-rev-uniformity-and-pin-order
kind: story
status: draft
title: One git URL, one commit across the dependency graph — and pins wait for the review above them
summary: ER pinned to an Eventlog commit still under review; AEP's lock then resolved two eventlog-core
owner: aep
relations:
- informed_by: task:aep-pin-verify-once-vector
- informed_by: review-result:aep-pin-vector-review-1
revision: 2
---
## Outcome

A pin chain across repositories is bumped in dependency order, or the tool that bumps it refuses.
One git URL resolves to one commit **across the whole dependency graph**, not merely within one
workspace.

## Why

2026-09-21, this wave. The chain is Eventlog → Entity Runtime → AEP: AEP depends on `entity-*` and
on `eventlog-core`/`eventlog-file` directly, and `entity-*` depend on `eventlog-*` directly too, so
every one of them must name the same Eventlog commit.

Unit 4 bumped Entity Runtime to `c698923` while unit 3 was still at `c698923` and under review.
Unit 3 then took a correction round (`9f234c5`, the prefix re-hash that restores the refusals
`c698923` lost) and a coverage round (`db608cd`), so the Eventlog head moved twice after ER was
pinned to it. When AEP's ten `rev` sites were then bumped to `db608cd` and `e535aded`,
`cargo update --offline` resolved **two** `eventlog-core` crates — `c698923` from ER's four
manifests and `db608cd` from AEP's two — which is the same defect unit 4 found inside one
workspace, now spanning two repositories.

It cost one extra bump and one extra commit in Entity Runtime. It could have cost a qualified
binary built against the provider version whose `resume` trusts a prefix it never re-read, which is
precisely what unit 3's correction exists to prevent. The lock is what caught it; nothing else
would have, because the three feature builds that fail on a split pin are not enabled by any gate
step (story:one-git-url-one-rev-across-the-workspace, entity-runtime).

Two rules fall out, and only the first is repository-local:

1. **Do not pin a downstream repository to an upstream commit that is still under review.** A
   review that returns findings moves the head, and every pin below it is then stale.
2. **Rev uniformity is a property of the graph, not of a workspace.** A per-workspace check would
   have passed at every step here.

## Acceptance

- A check the operator can run across a named set of checkouts reports every git URL that resolves
  to more than one commit, reading the manifests **and** the lockfiles, and names each site.
- **AEP's own `dep-check` closes the same gap in-repo.** `xtask/src/main.rs:727` reads only the
  `entity-*` packages, so `deps()` returns `Ok` over a lockfile carrying `eventlog-core` at two
  commits — measured by independent pass 1 over the pin against exactly the state this wave reached
  (review-result:aep-pin-vector-review-1, N1, CONFIRMED, pre-existing). The step must fail on a
  second commit of **any** git URL in the lock, not only on the packages it happens to enumerate.
- Red first: the state this wave actually produced — ER at `c698923`, AEP at `db608cd` — is
  reported, and the corrected state is not.
- The rule "do not pin to an upstream commit still under review" is written where a wave operator
  meets it, with this instance cited.
- The check is Rust where it runs in a repository gate (entity-runtime/AGENTS.md:312-314); an
  operator-side script is not bound by that rule and may be whatever the operator runs.
