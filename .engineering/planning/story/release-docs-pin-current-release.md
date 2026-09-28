---
format: aep.planning-md/3
id: story:release-docs-pin-current-release
kind: story
status: draft
title: The README and quickstart name the current release, checked at release time
relations:
- serves: vision:O2
revision: 1
---
## Problem

`README.md` and `website/docs/getting-started.md` install `--tag 0.63.1` and pin
`88836a30` (0.63.1's commit); 0.64.0 is released and nothing updates or checks them.

## Outcome

`cargo xtask status` (or the release step that already rewrites the website currency stamps)
rewrites the install tag and pinned commit, and `status-check` fails when they lag the newest tag.

## Acceptance

- After cutting a release, `cargo xtask status --check` fails until README and quickstart name it.
