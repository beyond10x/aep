---
title: "0.19.0 — the plan has a history"
description: >
  The markdown store gains an append-only journal, so what happened to an artifact becomes a
  question you can ask rather than a git log you reconstruct. A corrupt line is skipped and counted,
  never silently dropped.
slug: the-plan-has-a-history
tags: [release, aep, planning]
date: 2026-08-25T23:54:35+02:00
release_tag: "0.19.0"
release_commit: ab48bc8884620805a8efcce4a1aebc9b3eb6239d
---

The planning store is markdown files in git, so its history was always *there* — as `git log`, in a
form you had to reconstruct. This release makes it a question you can ask.

```console
$ protocol artifact history outbound-claim:q3-uptime
2026-08-26T00:08:16Z  operator  created as draft (revision 1)
2026-08-26T00:08:20Z  operator  approval recorded from legal review (https://example.invalid/approvals/814) (revision 1)
2026-08-26T00:08:20Z  operator  moved draft -> cleared (revision 2)
2026-08-26T00:08:20Z  operator  moved cleared -> sent (revision 3)
```

{/* truncate */}

## Two details are the design

**A corrupt line is skipped *and counted*.** A history that quietly shortens is worse than one that
says it is damaged: the second is a bug report, the first is a wrong answer nobody questions.

**Append-only, and not a cache.** The journal cannot be regenerated from the files — which is
exactly why it can answer questions the files cannot, such as what an artifact's status was before
somebody corrected it. A derived index would lose that on its first rebuild.

## What it does not close

This is the markdown store's own journal, not the AEP storage contract's. The sixteen conformance
suites still do not run against this store, and this release does not move that. Worth saying,
because "the durable store now has a journal" reads like it might have.
