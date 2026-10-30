---
format: aep.planning-md/3
id: review-result:adversary-protocol-source-in-a-pod-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: a protocol source in a pod'
relations:
- reviews: story:an-unreachable-protocol-source-fails-within-a-deadline
- reviews: story:a-group-writable-snapshot-is-admitted-explicitly
revision: 1
---
# Adversary pass 1: a protocol source in a pod (unit A)

Recorded by the wave coordinator from the adversary's report on commit `3d45b609a` (`cbfa43083` plus
the adversary's two test files). Scratch paths are elided.

```
unit: A
verdict: red
cases: executed 53→61, red 7
origin: introduced 3, pre-existing 0, undecided 4
wrote-outside-worktree: assigned scratch directory only
needs-coordinator: yes
```

Cases added: `crates/edge/aep-project/tests/fetch_deadline_adversary.rs` (6) and
`crates/edge/aep-project/tests/group_writable_adversary.rs` (3). Red on the unit branch:
`an_interrupt_to_aeps_process_group_stops_the_fetch_and_its_transport_with_it`,
`the_largest_whole_number_of_seconds_is_a_deadline_not_a_panic`,
`a_read_only_cache_without_the_pin_is_refused_naming_the_pin_the_cache_and_aep_cache_dir`,
`a_refused_fetch_changes_nothing_in_the_cache`,
`a_transport_that_leaves_a_detached_process_holding_stderr_does_not_outlast_the_deadline`,
`a_sealed_snapshot_in_a_directory_its_group_can_write_is_refused_without_the_declaration`,
`a_snapshot_reached_through_a_symlink_is_not_refused_as_writable_by_others`. Green:
`on_an_fs_group_volume_an_entry_others_can_write_is_named_not_the_group_declaration`.

Attacked and held: only exactly `1` admits the group; the stamp never carries an admission into a
process without one; the executable bit stays pinned; owner-only write loads; manifest and root are
checked; setgid ignored; symlinks inside the tree refused; the kill fallback passes paths as positional
parameters (no injection); a failed spawn sends no signal; no zombies; URL user information refused
before any message; nothing in the manifest is trusted before the write check.

```findings
[
  {"file": "crates/edge/aep-project/src/project.rs", "line": 792, "category": "concurrency", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Git now runs in its own process group, so a SIGINT or group kill aimed at aep (Ctrl-C, GNU timeout) stops aep but not Git, whose transport keeps its connection open with no deadline left to stop it (test fetch_deadline_adversary::an_interrupt_to_aeps_process_group_stops_the_fetch_and_its_transport_with_it)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 801, "category": "edge-case", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "AEP_FETCH_TIMEOUT_SECS=18446744073709551615 passes the positive-integer rule and then panics with 'overflow when adding duration to instant' after Git was spawned; that Git keeps running and wrote FETCH_HEAD into a cache already deleted (test fetch_deadline_adversary::the_largest_whole_number_of_seconds_is_a_deadline_not_a_panic)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 510, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "undecided", "message": "a read-only cache without the pin is refused with a bare 'creating ...: Permission denied' that names neither the pin nor AEP_CACHE_DIR, as the story's outcome requires; it is reached by a pod whose read-only prefilled cache mount lacks a newer pin (test fetch_deadline_adversary::a_read_only_cache_without_the_pin_is_refused_naming_the_pin_the_cache_and_aep_cache_dir)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 516, "category": "contract-drift", "severity": "note", "origin": "undecided", "message": "note: a refused fetch leaves objects.git and its FETCH_HEAD in the cache: acceptance 4's literal list holds, its parenthetical 'refusals change nothing' does not (test fetch_deadline_adversary::a_refused_fetch_changes_nothing_in_the_cache)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 807, "category": "concurrency", "severity": "note", "verdict": "INFEASIBLE", "origin": "undecided", "message": "the deadline bounds Git but not the reading of its output: when Git exits while a detached process still holds its stderr, a 2 s deadline returned after 12 s; I built that process with setsid and found nothing real that reaches it (test fetch_deadline_adversary::a_transport_that_leaves_a_detached_process_holding_stderr_does_not_outlast_the_deadline)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 1268, "category": "edge-case", "severity": "warning", "origin": "undecided", "message": "note: only the snapshot's own entries are checked, so a sealed snapshot whose snapshots/ directory its group can write loads without AEP_CACHE_GROUP_WRITABLE, although that group can swap the snapshot and its manifest (e.g. a shared cache created under umask 002); whether K2 covers the directories above the snapshot is the coordinator's call (test group_writable_adversary::a_sealed_snapshot_in_a_directory_its_group_can_write_is_refused_without_the_declaration)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 1320, "category": "edge-case", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "a snapshot root that is a symlink to a sealed snapshot is refused as writable by others, because the check reads the link's own 0777 mode, which chmod -R o-w cannot fix; nothing found that reaches it (test group_writable_adversary::a_snapshot_reached_through_a_symlink_is_not_refused_as_writable_by_others)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 860, "category": "concurrency", "severity": "note", "origin": "introduced", "message": "note: the group KILL is sent after try_wait has already reaped Git; if no group member is left, the group id is free and could be reused, contrary to the comment at 849-850 (pid wraparound needed, no test possible)"},
  {"file": "crates/edge/aep-project/src/project.rs", "line": 1320, "category": "mutant", "severity": "note", "origin": "introduced", "message": "holds: on an fsGroup snapshot with one entry others can write, the refusal says writable by others and does not offer AEP_CACHE_GROUP_WRITABLE; none of the unit's tests checked this order before (test group_writable_adversary::on_an_fs_group_volume_an_entry_others_can_write_is_named_not_the_group_declaration)"}
]
```

## Coordinator routing

- Git's own process group (line 792, introduced): back to the implementor. Decision: Git stays in
  aep's process group so an interrupt or a group kill aimed at aep reaches it; on the deadline aep
  stops Git's descendants by process id, then Git. This also removes the reused-group-id note (860).
- Overflowing deadline (801, introduced): back to the implementor; a value whose deadline cannot be
  represented is refused naming the variable, before Git is spawned.
- Read-only cache without the pin (510): decided as introduced scope; acceptance 2 covers every
  failure on the way to a fetch. Back to the implementor.
- Pipes held past the deadline (807): fixed in the same round (the read stops at the deadline).
- Symlinked snapshot root (1320): fixed in the same round (the root's mode is read through the link).
- `objects.git` after a refusal (516): decided no-op; acceptance 4 names the snapshot and the
  temporary checkout, and the bare object cache may serve a concurrent process. The case is rewritten
  to assert exactly that, naming this story.
- Parent directory its group can write (1268): pre-existing (no release checked parents); filed as
  `story:a-snapshot-parent-its-group-can-write-is-not-trusted-silently`. The case is rewritten to
  assert today's behaviour, naming that story.
