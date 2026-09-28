---
format: aep.planning-md/3
id: blocker:upstream-github-branch-update-provenance
kind: blocker
status: open
title: Gates cannot admit the upstream GitHub branch-update commit
relations:
- blocks: task:finish-er-evidence-upstream-publication
withholds: test_result
revision: 1
---
Gates publish of 8913a69427e31bd9e78a29a79137d9aaaad639e3 was refused on 2026-09-28 with: merged pull request missing or ambiguous. Current AEP main contains GitHub-generated intermediate commit 74956edad272b9489193248b347692edfffdeda6, titled Merge branch main into docs/overhaul. Its associated PR64 was merged by b10x-bot[bot], but that PR's merge_commit_sha is 938320e349ac4b0e5e8a21feb497b881dc5bd993, not the intermediate commit. The authenticated evidence therefore has zero completed PR merges matching the intermediate commit, while the delivery guard requires exactly one.

This is a delivery-authority refusal, not an ER behavior or conformance failure. No private policy or guard was changed, and publication was not attempted through an alternative route. The final CI result cannot be produced until delivery is admitted. The next owner must resolve the existing ancestry through the governed Gates process before resuming PR61. ER's pinned 18a18a3f implementation is already published and passed all required checks.
