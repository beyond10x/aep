---
title: Gate a move on evidence
sidebar_position: 2
description: Write a lifecycle for your own kind of work whose rungs cost evidence, record that evidence from CI or a person, and see what the move rested on.
---

# Gate a move on evidence

A lifecycle can put a price on a rung: *at least one `test_result`*, *two `approval`s*. This guide
adds a new kind, `data-migration`, whose `rehearsed` rung costs a test result and whose `applied`
rung costs two approvals. The commands and outputs are from a real run.

## 1. Own a document tree

Lifecycles come from the project's `protocols:` source. A source pinned to AEP's repository gives
you AEP's lifecycles and nothing else, so to add your own, vendor the tree into your repository.
Start from a release and keep its layout:

{/* generated:release-pin:begin version=0.70.0 — kept by `cargo xtask status` */}
```bash
mkdir governance
curl -sL https://github.com/beyond10x/aep/archive/refs/tags/0.70.0.tar.gz \
  | tar xz --strip-components=1 -C governance \
      aep-0.70.0/protocols aep-0.70.0/principles aep-0.70.0/workflows \
      aep-0.70.0/profiles aep-0.70.0/artifacts aep-0.70.0/drivers
aep plan reverse init --protocols ../governance --profile development.standard
```
{/* generated:release-pin:end */}

`--protocols` is relative to `.engineering/`, and `project.yaml` now says `protocols: ../governance`.
If the repository already has a project file, change that one line instead of running `reverse init`.

## 2. Write the lifecycle

`governance/artifacts/lifecycles/data-migration.yaml`:

```yaml
kind: data-migration
initial: draft
transitions:
  draft: [rehearsed, abandoned]
  rehearsed: [applied, abandoned]
  applied: []
  abandoned: []
requires:
  rehearsed:
    - evidence: test_result
  applied:
    - evidence: approval
      at_least: 2
descriptions:
  draft: Written down, not yet run anywhere.
  rehearsed: Run against a copy of production data, with a test result recorded.
  applied: Run in production, after two approvals.
  abandoned: Decided against.
```

`evidence:` must be an evidence kind the protocol declares. The set is closed on purpose, so a
caller cannot invent the kind of proof a gate asks for. The kinds are listed in the
[vocabulary reference](../reference/vocabulary.md#evidence-kinds). `at_least` defaults to `1`.

Check the tree and the new ladder:

```shell-session
$ aep govern validate --root governance
61 file(s): 5 protocol(s), 24 principle(s), 6 workflow(s), 10 profile(s), 14 lifecycle(s), 2 step map(s)
valid
$ aep plan artifact lifecycle data-migration
data-migration starts at draft
  abandoned -> nothing
  applied -> nothing
  draft -> abandoned, rehearsed
  rehearsed -> abandoned, applied
```

## 3. Read the price before paying it

```shell-session
$ aep plan artifact new data-migration split-orders-table --title "Split the orders table"
created data-migration:split-orders-table (draft) at …/planning/data-migration/split-orders-table.md
$ aep plan artifact explain data-migration:split-orders-table
data-migration:split-orders-table in …/.engineering/planning: draft, revision 1
  no status move is recorded
  next: abandoned needs no record
  next: rehearsed needs 1 test_result record(s); held: 0
```

## 4. Record, then move

```shell-session
$ aep plan artifact evidence data-migration:split-orders-table --kind test_result \
    --source "rehearsal on staging snapshot" --at 2026-09-27T14:00:00Z
data-migration:split-orders-table: test_result recorded from rehearsal on staging snapshot
  on hand: test_result=1
$ aep plan artifact move data-migration:split-orders-table --to rehearsed
data-migration:split-orders-table moved draft -> rehearsed (revision 2)
$ aep plan artifact evidence data-migration:split-orders-table --kind approval --source "dba on-call"
data-migration:split-orders-table: approval recorded from dba on-call
  on hand: test_result=1, approval=1
$ aep plan artifact move data-migration:split-orders-table --to applied
data-migration:split-orders-table is rehearsed; applied is on the ladder and not yet earned: reaching applied needs at least 2 approval record(s)
$ aep plan artifact evidence data-migration:split-orders-table --kind approval --source "service owner"
data-migration:split-orders-table: approval recorded from service owner
  on hand: test_result=1, approval=2
$ aep plan artifact move data-migration:split-orders-table --to applied
data-migration:split-orders-table moved rehearsed -> applied (revision 3)
```

`--at` records when the observation was made. It defaults to now. Record the time the check
actually ran, not the time you got round to recording it.

## 5. See what it rested on

```shell-session
$ aep plan artifact explain data-migration:split-orders-table
data-migration:split-orders-table in …/.engineering/planning: applied, revision 3
  draft -> rehearsed  2026-09-28T09:01:29Z  (revision 2)
    test_result from rehearsal on staging snapshot, observed 2026-09-27T14:00:00Z, admitted at revision 1
  rehearsed -> applied  2026-09-28T09:01:29Z  (revision 3)
    approval from dba on-call, observed 2026-09-28T09:01:29Z, admitted at revision 2
    approval from service owner, observed 2026-09-28T09:01:29Z, admitted at revision 2
```

Each record is named against the revision the artifact was at when the record was admitted. A later
edit to the body therefore cannot make an old record look as if it were about the new text.

## Recording evidence from CI

A CI job records the result of the check it ran, and names the run:

```bash
aep plan artifact evidence story:pay-by-card --kind test_result \
  --source "ci: cargo test --workspace" --ref "$CI_RUN_URL"
```

The evidence file is a new file under `.engineering/evidence/`, so the job must commit it (or open
a pull request with it) for the record to reach the plan.

For an ESS conformance report, `--from <report>` reads the kind, source and completion time out of
the report instead of taking them from the command line. See the
[CLI reference](../reference/cli.md#plan-artifacts).

## When the evidence is not in the store

If the check really ran but nobody recorded it, `move --evidence <kind>=<count>` states the count on
the command line:

```shell-session
$ aep plan artifact move story:guest-receipt --to implemented --evidence test_result=1
story:guest-receipt moved active -> implemented (revision 4)
  decided partly on asserted evidence nothing checks: test_result=1
  `aep plan artifact evidence story:guest-receipt --kind <kind> --source <where>` records it instead
```

The transition records `decided_on: {"asserted": {"test_result": 1}}`. `validate` then lists the
artifact under *closed on an assertion*, and `validate --strict` exits `1`, so a gate can refuse
assertions while a person at a terminal can still close a story on a day the CI runner is down.

## A rung that opens on a date

`when:` gates a rung on a date the artifact records in its own front matter, rather than on
evidence. See [Lifecycles and rungs](../concepts/lifecycles.md#a-rung-can-open-on-a-date).
