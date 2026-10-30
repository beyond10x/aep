---
format: aep.planning-md/3
id: review-result:security-protocol-source-in-a-pod-pass-1
kind: review-result
status: active
title: 'Security review pass 1: a protocol source in a pod'
relations:
- reviews: story:an-unreachable-protocol-source-fails-within-a-deadline
- reviews: story:a-group-writable-snapshot-is-admitted-explicitly
revision: 1
---
# Security review pass 1: a protocol source in a pod (unit A)

Recorded by the wave coordinator from the security reviewer's report on commit `18677651a`
(`cbfa43083` plus the reviewer's test file). Scratch paths are elided.

```
unit: A
verdict: red
cases: added 4, red 4 (2 green at base, 2 red at base)
origin: introduced 2, pre-existing 1, undecided 1
wrote-outside-worktree: assigned scratch directory only
needs-coordinator: yes
```

Cases added: `crates/edge/aep-project/tests/protocol_cache_security.rs` (4).

```findings
[
  {"file": "crates/edge/aep-project/src/project.rs", "line": 792, "category": "security", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "process_group(0) takes Git out of the caller's process group, so Ctrl-C or coreutils timeout stops aep while Git and its transport keep the connection open with no deadline (test a_fetch_stops_with_the_command_when_the_commands_process_group_is_interrupted, red; green at base)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 801, "category": "edge-case", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "AEP_FETCH_TIMEOUT_SECS near u64::MAX passes validation and then panics with an overflow after Git is spawned, leaving Git unstopped and unreaped instead of refusing (test the_largest_fetch_deadline_the_variable_accepts_ends_in_a_refusal_not_a_crash, red; green at base)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 1051, "category": "integrity", "severity": "warning", "verdict": "INFEASIBLE", "origin": "pre-existing", "message": "the stamp is trusted whatever its own mode, and its write-bit and unchanged checks cover only entries it lists, so whoever can write it can leave out a writable entry and admit bytes the manifest does not hold; no reaching configuration found (tests a_stamp_the_group_can_write_never_admits_a_group_writable_snapshot_without_the_declaration and a_stamp_others_can_write_never_admits_a_snapshot_others_can_write, red; red at base)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 501, "category": "integrity", "severity": "note", "verdict": "INFEASIBLE", "origin": "undecided", "message": "no write check looks above the snapshot root, so whoever can write snapshots/ can replace the snapshot with one carrying a self-consistent manifest; no reaching configuration found, no test"}
]
```

## Coordinator routing

- Git's own process group (792): the same finding as the adversary's first; fixed in correction
  round 1 as decided there.
- Overflowing deadline (801): the same as the adversary's second; fixed in correction round 1.
- Stamp trusted whatever its own mode (1051, pre-existing): fixed in this wave, because K2 makes the
  group a writer the stamp must not let in silently. A stamp others can write is never trusted; a
  stamp its group can write is trusted only with `AEP_CACHE_GROUP_WRITABLE=1`; an untrusted stamp
  falls back to the full verification. Both cases must pass unchanged.
- No write check above the snapshot root (501): the adversary's parent-directory finding; covered by
  `story:a-snapshot-parent-its-group-can-write-is-not-trusted-silently`.
