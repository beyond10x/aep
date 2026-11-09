---
format: aep.planning-md/3
id: decision-blocker:which-parent-directories-a-snapshot-trusts
kind: decision-blocker
status: open
title: Nobody has decided which parent directories of a protocol snapshot are checked, or how a per-user group is told from a shared one
relations:
- blocks: story:a-snapshot-parent-its-group-can-write-is-not-trusted-silently
revision: 1
---
# Decision blocker: Nobody has decided which parent directories of a protocol snapshot are checked, or how a per-user group is told from a shared one

The defect is still present at `824869fec`: `refuse_shared_write` (`crates/edge/aep-project/src/project.rs:1412-1460`) checks the snapshot root, its manifest and the entries inside, and nothing above. `crates/edge/aep-project/tests/group_writable_adversary.rs:41` pins that a snapshot in a group-writable parent loads today.

The story's acceptance leaves two things open, and both are trust decisions about an operator's machine rather than facts the code can read: how far up the walk goes (to the cache root, to `AEP_CACHE_DIR`, to the filesystem root), and how a group only its owner is in is told from a shared one. A wrong answer either refuses every cache under a group-writable home, or keeps trusting the directory the story is about.

Cleared by: the protocol owner's answer to those two questions, written into the story's acceptance.

Filed 2026-11-09 by the coordinator of `epic:zero-open-issues-2026-11-09`, in a non-interactive run; the story stays `draft` and is not in the epic.
