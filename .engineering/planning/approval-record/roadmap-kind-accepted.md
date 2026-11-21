---
format: aep.planning-md/3
id: approval-record:roadmap-kind-accepted
kind: approval-record
status: draft
title: 'Accepted: the roadmap kind, requested by sdc, under the operator''s standing rule of 2026-11-21'
tags:
- non-interactive
relations:
- decides: story:roadmap-kind
revision: 1
---
# Approval record: the roadmap kind, accepted under the operator's standing rule

## Which stop

Acceptance of `story:roadmap-kind`, which moves it from `draft` through `proposed` to `active`.
Reason `non-interactive`: this session runs as a sub-agent dispatched by the `sdc` session, and no
operator turn exists inside this run.

## Where the request comes from

The `sdc` repository: `upstream-blocker:aep-roadmap-kind`, which blocks `story:roadmap-kind-adopt`,
with `story:roadmap-reconstruct` and `docs/orchestration.md` § 4.4 gap 1 as the brownfield use.

## The operator's words, as relayed by the `sdc` session

Standing rule of 2026-11-21, for stories the `sdc` session sends:

> all the ones you're ad hoc sending … should be also automatically accepted by default

And today, to the `sdc` session, for AEP: "just do it yourself now and use the bot token for commits
and releases."

## What is not accepted by this record

`story:reverse-scan-reports-epics-without-a-roadmap` stays `draft`. No release is accepted here: the
operator starts it after reading the report.
