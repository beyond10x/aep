---
format: aep.planning-md/3
id: story:redact-rewrites-a-review-prose-only-reason
kind: story
status: draft
title: redact rewrites a review's prose_only reason, with records that say which fields they covered
relations:
- serves: vision:O2
revision: 1
---
## Problem

`aep plan artifact redact` rewrites an artifact's title, summary and body. A review-result's `prose_only` reason is free text too: `new --prose-only <reason>` writes it and `show` prints it. A person's name in that reason survives `redact`. Since story:redacted-literal-scan-reads-only-redactable-text, `validate` refuses such a copy with `pack_redacted_literal_exposed`, and no verb can clear it.

## Open question

Teaching `redact` the field changes what a redaction record's count covers. Records written before the change counted only title, summary and body, so a replay that also rewrites `prose_only` would disagree with them. The record needs a way to say which fields it covered (a field list, or a record version) before the replay can include the field.

## Acceptance

- `redact` on a review-result whose `prose_only` reason holds the literal rewrites it there, and the record says it covered that field.
- Replaying a redaction record written before this change still matches its recorded count.
- After `redact`, `validate` on that store reports no `pack_redacted_literal_exposed`, loose and packed.
