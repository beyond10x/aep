---
format: aep.planning-md/1
id: review-result:mapper-workspace-review-2
kind: review-result
status: active
title: 'Independent verification pass 2: the lifted predicate does not know which member this store is'
relations:
- reviews: story:migration-mapper-reads-the-declared-workspace
revision: 2
---
# Unit 8 — story:migration-mapper-reads-the-declared-workspace — independent verification, pass 2

```
unit: story:migration-mapper-reads-the-declared-workspace — worktree ess-evolution-mapper-workspace-review-1-20260921 at ef8f263afa89db521caea5224afb50fb4677f628 (base 763d195be)
verdict: red
cases: executed 664→675, red 5
origin: introduced 4, pre-existing 1, undecided 0
wrote-outside-worktree: 14 paths, all under ~/.cache/ess-wave-v2/u8r2/ — enumerated in part 6
needs-coordinator: yes — findings 1 and 2 share one root cause whose fix changes a shared domain predicate (`aep-domain`), which is outside this unit's cited scope; whether that lands here or as its own story is not mine to say
```

**One sentence.** `ArtifactRelation::crosses_to_a_declared_member` — the predicate this unit lifted
so that *one* spelling of the crossing rule would decide both the graph and the receipt — has no
notion of **which member this store is**, and the real `.engineering/workspace.yaml` declares this
store as one of its own members; so an edge written `engineering-protocols/story:x` inside
`engineering-protocols` is counted as a crossing, is not imported as a relation record even though
its destination entity is right there in `identities`, and is exempted from the dangling-edge check
that the code's own comment says was the hole being closed.

Pass 1's five findings are settled and are not re-reported.

---

## 1. `git --no-pager diff --stat` — proof of what I touched

```console
$ git --no-pager diff --stat
                                                   # empty: no tracked file modified

$ git status --porcelain
?? .engineering/.aep-planning-writer-27c29ce0d8bc725db7a26f64933e9cc539c37427ee5f1f98087222fde950fcea.lock
?? .engineering/.aep-planning-writer.lock
?? crates/edge/aep-cli/tests/migration_counts_a_self_member_edge_as_a_crossing.rs
?? crates/edge/aep-cli/tests/migration_inspect_reads_the_declaration_too.rs
?? crates/edge/aep-cli/tests/validate_sees_a_dangling_edge_behind_our_own_member_name.rs
?? crates/plan/aep-planning-migration/tests/a_self_member_prefix_does_not_excuse_a_dangling_edge.rs
?? crates/plan/aep-planning-migration/tests/self_member_reference_is_not_a_crossing.rs
?? crates/plan/aep-planning-migration/tests/self_member_reference_origin_at_base.rs
```

Six paths, every one of them under a `tests/` directory. No implementation file, manifest,
lockfile, schema or document was edited. No git write command was run; no `aep plan artifact`
verb of any kind was run; nothing under `.engineering/` was written by me.

**I name the two non-test paths myself.** The two zero-byte `.aep-planning-writer*.lock` files were
already untracked in this tree when I took the lease — they are the same pair pass 1 and the
correction round each flagged in their own trees, writer-fence artefacts of the repository's own
suite. I did not author them and I left them in place.

**Gate-lane hygiene of my own files**, because the correction round had to fix pass 1's for this:

```console
$ cargo fmt --all -- --check                                        ; FMT_EXIT=0
$ cargo clippy -p aep-cli -p aep-planning-migration --all-targets -- -D warnings ; CLIPPY_EXIT=0
```

Disclosed rather than implied: after the first red run, `cargo fmt --check` reported reflow in two
of my files, so I ran `rustfmt` over my six files and re-ran everything. The reflow moved assertion
line numbers (`self_member_reference_is_not_a_crossing.rs` 121→129 and 147→155) and changed no
assertion, condition or value. Part 2 quotes the **post-format** runs; the pre-format run of the
same assertions is in `caseA-red.log` and `caseB-red.log`.

---

## 2. The cases I added

Written before anything was run, each file run **alone** before the suite. Output verbatim.

| file | case | now |
|---|---|---|
| `crates/plan/aep-planning-migration/tests/self_member_reference_is_not_a_crossing.rs` | `an_unqualified_local_edge_is_a_relation_record_even_when_this_member_is_declared` | green (control) |
| | `a_reference_qualified_with_this_store_s_own_member_name_is_still_a_local_relation_record` | **red** |
| | `the_two_spellings_of_one_local_edge_migrate_to_the_same_number_of_relation_records` | **red** |
| `crates/edge/aep-cli/tests/migration_counts_a_self_member_edge_as_a_crossing.rs` | `an_edge_into_this_store_s_own_declared_member_is_not_a_workspace_crossing` | **red** |
| `crates/plan/aep-planning-migration/tests/a_self_member_prefix_does_not_excuse_a_dangling_edge.rs` | `an_unprefixed_dangling_edge_is_refused_even_when_this_member_is_declared` | green (control) |
| | `the_same_dangling_edge_behind_this_store_s_own_member_name_is_still_a_dangling_edge` | **red** |
| `crates/edge/aep-cli/tests/validate_sees_a_dangling_edge_behind_our_own_member_name.rs` | `an_unprefixed_dangling_edge_is_reported` | green (control) |
| | `a_dangling_edge_behind_this_store_s_own_member_name_is_reported_too` | **red** |
| `crates/plan/aep-planning-migration/tests/self_member_reference_origin_at_base.rs` | `with_no_member_list_the_base_call_refuses_a_self_member_qualified_edge` | green (origin) |
| `crates/edge/aep-cli/tests/migration_inspect_reads_the_declaration_too.rs` | `inspect_refuses_an_unparseable_declaration_at_the_workspace_file` | green (mutant detector) |
| | `inspect_still_answers_for_a_store_that_declares_no_members` | green (control) |

### 2a. The mapper drops a relation whose destination *is* in this authority — red

`aep_domain::workspace::WorkspaceRef` defines the two spellings and what each means:

> | `story:provider-spi` | *this* member's story — whichever store the reference was written in |
> | `entity-runtime/story:provider-spi` | that member's story, wherever it is read from |

`crosses_to_a_declared_member` (`aep-domain/src/artifact.rs:1324`) asks only whether the target
names *a* declared member. `mapping.rs:324` then skips the relation on the stated ground that
"workspace crossings have no destination entity in this authority" — but when the member named is
**this** store's, that destination entity is in `identities` under its unqualified id.

```console
$ TMPDIR=~/.cache/ess-wave-v2/u8r2/tmp CARGO_NET_OFFLINE=true \
  cargo test -p aep-planning-migration --test self_member_reference_is_not_a_crossing
     Running tests/self_member_reference_is_not_a_crossing.rs (target/debug/deps/self_member_reference_is_not_a_crossing-0767e4658194d67f)

running 3 tests
test an_unqualified_local_edge_is_a_relation_record_even_when_this_member_is_declared ... ok
test a_reference_qualified_with_this_store_s_own_member_name_is_still_a_local_relation_record ... FAILED
test the_two_spellings_of_one_local_edge_migrate_to_the_same_number_of_relation_records ... FAILED

failures:

---- a_reference_qualified_with_this_store_s_own_member_name_is_still_a_local_relation_record stdout ----

thread '…' (2354363) panicked at crates/plan/aep-planning-migration/tests/self_member_reference_is_not_a_crossing.rs:129:5:
assertion `left == right` failed: `engineering-protocols/story:local` names an artifact of this very store, so the edge has a destination entity in this authority and must be imported as a relation record exactly as `story:local` is; subjects produced: [Subject { entity: "aep.entity", id: "MIG84a728781f4139e49bea26c2a5233ecbb96808fc30d0d3f2883617c834cef4d5" }, Subject { entity: "aep.entity", id: "MIGbbf44d14d491f11b066306066fa9431a11365ef0d1eea0f3677c278d5ea16079" }]
  left: 0
 right: 1

---- the_two_spellings_of_one_local_edge_migrate_to_the_same_number_of_relation_records stdout ----

thread '…' (2354365) panicked at crates/plan/aep-planning-migration/tests/self_member_reference_is_not_a_crossing.rs:155:5:
assertion `left == right` failed: `engineering-protocols/story:local` and `story:local` are the same artifact read from this store, so they cannot migrate to different numbers of relation records
  left: 0
 right: 1

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-planning-migration --test self_member_reference_is_not_a_crossing`
EXIT=101
```

The control — the identical fixture, the identical member list, the target spelled `story:local` —
produces one relation record and passes. The member declaration is not what decides it; the
spelling is.

### 2b. The receipt's new count, on a store that crosses nowhere — red

The operator-facing half, through the built binary. One store, one member declared, and that member
is itself — `source: ..`, exactly the first entry of this repository's own `workspace.yaml`. Its one
edge is `mine/story:local`, which is `story:local` of this store.

```console
$ TMPDIR=… cargo test -p aep-cli --test migration_counts_a_self_member_edge_as_a_crossing
     Running tests/migration_counts_a_self_member_edge_as_a_crossing.rs (target/debug/deps/migration_counts_a_self_member_edge_as_a_crossing-a185b2c7749a3ff0)

running 1 test
test an_edge_into_this_store_s_own_declared_member_is_not_a_workspace_crossing ... FAILED

---- an_edge_into_this_store_s_own_declared_member_is_not_a_workspace_crossing stdout ----

thread '…' (2373685) panicked at crates/edge/aep-cli/tests/migration_counts_a_self_member_edge_as_a_crossing.rs:94:5:
assertion `left == right` failed: `mine/story:local` is this store's own artifact under its own declared member name, so the one edge is a relation record the migration imports and nothing crosses out of this store; the receipt reports relations=0, workspace_crossings=1: {"applied_commands":0,"audit_records":0,"bare_decisions":0,"bare_events":0,"complete_envelopes":0,"entities":2,"raw_evidence_items":2,"relations":0,"subjects":2,"workspace_crossings":1}
  left: (0, 1)
 right: (1, 0)

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p aep-cli --test migration_counts_a_self_member_edge_as_a_crossing`
EXIT=101
```

The two counts still **sum** to the source count, which is why every sum assertion in the unit's own
suite stays green. The split is what is wrong, and the split is the thing the new field exists to
report.

### 2c. A dangling edge, admitted — red

`mapping.rs:249-253` states the rule the member list exists to enforce: *"a relation into a member
it does not declare stays a dangling edge and is still refused, because a store migrated with edges
that dangle for real is worse than a refusal."* `validate_edges` (`artifact.rs:2526-2533`) states it
again: *"exempting it was a hole: with no workspace file at all, every dangling edge could be hidden
behind a `/`."* Both are true for a member the workspace does not declare. Neither is true for the
member the workspace declares as **itself**.

```console
$ TMPDIR=… cargo test -p aep-planning-migration --test a_self_member_prefix_does_not_excuse_a_dangling_edge
     Running tests/a_self_member_prefix_does_not_excuse_a_dangling_edge.rs (target/debug/deps/a_self_member_prefix_does_not_excuse_a_dangling_edge-b5c5a00be9919616)

running 2 tests
test an_unprefixed_dangling_edge_is_refused_even_when_this_member_is_declared ... ok
test the_same_dangling_edge_behind_this_store_s_own_member_name_is_still_a_dangling_edge ... FAILED

---- the_same_dangling_edge_behind_this_store_s_own_member_name_is_still_a_dangling_edge stdout ----

thread '…' (2419146) panicked at crates/plan/aep-planning-migration/tests/a_self_member_prefix_does_not_excuse_a_dangling_edge.rs:75:5:
`engineering-protocols/story:typo-that-does-not-exist` names this store's own member, so the target is this store's own `story:typo-that-does-not-exist`, which no document declares; prefixing a dangling edge with the reader's own member name cannot turn it into a crossing an assembly will resolve. The store was admitted and produced 1 subject(s).

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-planning-migration --test a_self_member_prefix_does_not_excuse_a_dangling_edge`
EXIT=101
```

The control — the same missing target with no prefix, same declaration — is refused. That is what
makes this a hole rather than a property of the fixture.

### 2d. The read-path half of the same hole — red, and **pre-existing**

```console
$ TMPDIR=… cargo test -p aep-cli --test validate_sees_a_dangling_edge_behind_our_own_member_name
running 2 tests
test an_unprefixed_dangling_edge_is_reported ... ok
test a_dangling_edge_behind_this_store_s_own_member_name_is_reported_too ... FAILED

---- a_dangling_edge_behind_this_store_s_own_member_name_is_reported_too stdout ----

thread '…' (2427766) panicked at crates/edge/aep-cli/tests/validate_sees_a_dangling_edge_behind_our_own_member_name.rs:82:5:
`mine/story:typo-that-does-not-exist` names this store's own member, so it is this store's own `story:typo-that-does-not-exist`, which no document declares; the whole-plan check must report it exactly as it reports the unprefixed spelling. It reported: 1 file(s) in …/.engineering/planning: 1 artifact(s)
1 document(s) predate the event log
valid

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p aep-cli --test validate_sees_a_dangling_edge_behind_our_own_member_name`
EXIT=101
```

`plan artifact validate` prints **valid** for a store whose only edge points at nothing.

### 2e. Origin, settled by running rather than argued — green

At `763d195be` the mapper took no member list: `markdown_boundaries_raw(raw: &MarkdownRawV1)`, and
inside it `report.graph()` (`mapping.rs:181` at base), which is
`graph_in_workspace(std::iter::empty())` (`aep-backend-markdown/src/store.rs:360-362`, unchanged at
head). So the base behaviour for any member-qualified target is exactly the head function handed an
empty member list, and this case runs that call in this tree:

```console
$ TMPDIR=… cargo test -p aep-planning-migration --test self_member_reference_origin_at_base
running 1 test
test with_no_member_list_the_base_call_refuses_a_self_member_qualified_edge ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Base **refused** the store; head admits it and silently imports nothing for the edge. That settles
findings 1 and 2 as `introduced` — the unit's diff reaches a path nothing reached before. I did not
move the tree to the base and built no base checkout; the disk was between 31 and 39 GiB free all
session and a second `aep-cli` build tree was not worth it when the base call is reproducible in
this one.

Finding 3 goes the other way, and I checked it the same way rather than guessing:
`git show 763d195be:crates/govern/aep-domain/src/artifact.rs` lines 2512-2516 hold the exemption
byte-for-byte, and base `planning.rs:5412-5417` already handed `declared_members` to
`graph_in_workspace` for `plan artifact graph` and its siblings. **Pre-existing.**

### 2f. The detector for a line that had none — green by design

`migration_inspect_reads_the_declaration_too.rs` is not a defect; it is the missing mutant detector
named in finding 5. It passes today and goes red the moment `store_command.rs:906` becomes
`unwrap_or_default()`.

```console
$ TMPDIR=… cargo test -p aep-cli --test migration_inspect_reads_the_declaration_too
running 2 tests
test inspect_refuses_an_unparseable_declaration_at_the_workspace_file ... ok
test inspect_still_answers_for_a_store_that_declares_no_members ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

---

## 3. The suite, after the cases existed

```console
$ TMPDIR=~/.cache/ess-wave-v2/u8r2/tmp CARGO_NET_OFFLINE=true \
  cargo test --no-fail-fast -p aep-planning-migration -p aep-cli
…
error: test failed, to rerun pass `-p aep-cli --test migration_counts_a_self_member_edge_as_a_crossing`
error: test failed, to rerun pass `-p aep-cli --test validate_sees_a_dangling_edge_behind_our_own_member_name`
error: test failed, to rerun pass `-p aep-planning-migration --test a_self_member_prefix_does_not_excuse_a_dangling_edge`
error: test failed, to rerun pass `-p aep-planning-migration --test self_member_reference_is_not_a_crossing`
SUITE_EXIT=101
```

Summed across every `test result:` line the runner printed: **670 passed, 5 failed, 0 ignored — 675
executed.** Full log `suite-final2.log`.

`<before>` is **664**, and it is the honest kind: a second run of the same command with my six test
**targets** deselected from the sum — `self_member_reference_is_not_a_crossing`,
`self_member_reference_origin_at_base`, `a_self_member_prefix_does_not_excuse_a_dangling_edge`,
`migration_counts_a_self_member_edge_as_a_crossing`,
`migration_inspect_reads_the_declaration_too` and
`validate_sees_a_dangling_edge_behind_our_own_member_name`. I did **not** run the suite before my
cases existed. 664 + 11 = 675, every one of the 11 in a target I added, and no pre-existing lane
moved: nothing was deleted, skipped, weakened or rewritten.

These are the two package lanes that carry the change, the same scope pass 1 reported (643→655).
They are **not** the correction round's `2113→2134`, which is the whole 16-step gate's `test` step —
naming the step a count came from, per the sub-operator's note at the end of the correction report.
I did not run the full gate: it includes an `npm ci` and a website build, the disk was at 31 GiB
with a real cutover writing, and `fmt`/`clippy` — the two steps my own files could plausibly break —
were run separately and are green (part 1).

---

## 4. Findings

Covering worktree `ess-evolution-mapper-workspace-review-1-20260921` at commit
`ef8f263afa89db521caea5224afb50fb4677f628`, base `763d195be`.

### 1 — a reference to this store's own declared member is counted as a crossing and not imported — `NEEDS-CHANGE` / `introduced` / blocker

- **what was measured** — `crates/plan/aep-planning-migration/tests/self_member_reference_is_not_a_crossing.rs:129` and `:155`, exit 101: `engineering-protocols/story:local` produces **0** `aep.relation` subjects where the identical fixture with `story:local` produces 1. `crates/edge/aep-cli/tests/migration_counts_a_self_member_edge_as_a_crossing.rs:94`, exit 101: the binary's own admitted receipt reports `relations 0, workspace_crossings 1` for a store that crosses nowhere. Root: `crates/govern/aep-domain/src/artifact.rs:1324`, consumed at `crates/plan/aep-planning-migration/src/mapping.rs:324` and `crates/edge/aep-cli/src/store_command.rs:1998`.
- **what reaches it** — this repository's own `.engineering/workspace.yaml` declares `engineering-protocols` with `source: ..` — *this* store — and its comment invites the spelling: *"Named explicitly rather than implied, so every artifact in the assembled graph carries a member and none of them is the special one that does not."* `WorkspaceRef` then defines `engineering-protocols/story:x`, read here, as this store's `story:x`. **Stated plainly and against my own interest: the store carries no such edge today.** `grep` over `.engineering/planning` finds exactly one member-qualified relation, `story/assemble-across-sources.md:11 - informed_by: entity-runtime/story:typed-references`, a genuine crossing. So this cutover, run now, loses nothing; one legal edit to any artifact makes it silent loss and a receipt that says the opposite of the truth. The configuration half is real and in the tree; the edge half I built.
- **the correction, named not applied** — give the migration and the counter this store's **own** member name and resolve a self-qualified reference to its unqualified id before either question is asked — `WorkspaceRef::within` / `resolve` already express this, and `crosses_to_a_declared_member` is the wrong question to be asking at both sites. A second, cheaper shape: take `Option<&MemberName>` for "who I am" and return `false` when the target names it. Either way the fix is in `aep-domain`, which is outside this unit's cited scope, and that is what `needs-coordinator` is about.

### 2 — a dangling edge prefixed with this store's own member name is admitted by the migration — `NEEDS-CHANGE` / `introduced` / blocker

- **what was measured** — `crates/plan/aep-planning-migration/tests/a_self_member_prefix_does_not_excuse_a_dangling_edge.rs:75`, exit 101: `engineering-protocols/story:typo-that-does-not-exist` is admitted and migrates 1 subject, where the same target unprefixed, with the same member declared, is refused.
- **what reaches it** — the same configuration as finding 1, and this half needs no deliberate act: it is what a **typo** does. `mapping.rs:249-253` says in so many words that this must not happen (*"a store migrated with edges that dangle for real is worse than a refusal"*), and `artifact.rs:2526-2533` says the hole was closed. Both sentences are true only for a member the workspace does not declare.
- **relation to finding 1** — one root cause, two different costs. Fixing finding 1 fixes this. I am filing them separately because they fail differently and a coordinator routing on severity should see both.

### 3 — `plan artifact validate` reports *valid* for a store whose only edge dangles — `CONFIRMED` / `pre-existing` / warning

- **what was measured** — `crates/edge/aep-cli/tests/validate_sees_a_dangling_edge_behind_our_own_member_name.rs:82`, exit 101: the whole-plan check prints `valid`; the unprefixed control prints `[undeclared_reference]`.
- **what reaches it** — every read command in a self-declaring workspace, which is every read command in this repository today.
- **origin, settled not guessed** — `git show 763d195be:crates/govern/aep-domain/src/artifact.rs` lines 2512-2516 hold the exemption unchanged, and base `planning.rs:5412-5417` already passed `declared_members`. **This one is not the unit's and should route out of the wave into its own story.** It is in this report because it is the same defect as findings 1 and 2 seen from the read path, and because the unit's diff is what gave that predicate a second caller.

### 4 — `workspace_crossings` is hard zero on four of the five receipts that publish it — `CONFIRMED` / `introduced` / warning

- **what was measured** — by reading, not by running, and I say which. `crates/edge/aep-cli/src/store_command.rs:2335` `inventory_from_histories` builds `InventoryCountsV1::default()` and never assigns `workspace_crossings`. Its value is consumed at `:434` (`apply`), `:851` (`projection rebuild`), `:1054`/`:1060` (`inspect` of an Eventlog store) and `:1539` (`verify`). The field's contract text (`crates/plan/aep-contract/src/migration/command.rs:455-463`) and all seven generated schemas say of the pair: *"the two sum to what the source declares."* On those four receipts the sum is `relations + 0`, which for the AEP store is 521 against a source of 522.
- **what reaches it** — the second half of every one of the six cutovers. The documented procedure is dry-run, read the receipt, apply, read the receipt: `workspace_crossings 1` becomes `workspace_crossings 0` between them with nothing said. Nothing *gates* on it — `verify` compares snapshot identities and projection digests, not inventories (`store_command.rs:1466-1520`) — which is why this is a warning and not a blocker.
- **why there is no failing case** — making it red requires an Eventlog store, which requires `plan store migrate apply`, which my brief forbids on any store anywhere. The correction round's in-process `apply_with_control` route is not open to me either: it reaches the receipt only from inside `store_command.rs`'s own test module, and that is an implementation file I may not edit. Recorded as a read with exact coordinates rather than dressed up as a measurement.
- **the correction, named not applied** — either give `AuthorityObservationV1` its own count type without a field the authority cannot answer, or carry the dry-run receipt's `workspace_crossings` forward into the apply receipt. The one thing that should not stand is a field whose published description is false on four of the five documents that carry it.

### 5 — the `inspect` reader of the declaration had no detector — `CONFIRMED` / `introduced` / note

- **what was measured** — read, then closed. The unit added three edge readers of `Resolved::declared_members()` — `store_command.rs:906` (`inspect`), `:1202` (`dry_run`), `:2114` (`mapped_histories`). Every case the unit and the correction shipped for the unreadable-declaration refusal goes through `dry-run` or the read path; nothing in the tree calls `inspect` at all (`store_command.rs`'s test module never names it, and no integration test pairs `inspect` with `workspace`). Line 906 was therefore a line that could be deleted — replaced by `unwrap_or_default()` — with the whole suite green, and what it opens is the silent failure: `inspect` reporting a crossing as an ordinary relation on a store whose declaration is unreadable, which is the reading this unit exists to remove.
- **what reaches it** — `aep plan store inspect` on any store with a mistyped `workspace.yaml`. The behaviour today is **correct**; this is a measurement gap, and `migration_inspect_reads_the_declaration_too.rs` now closes it. Answering the pass's standing question directly: that is the one added line I found that the suite left undefended. The other added surface is covered — the crossing predicate by M9 and by three of my own files, `ConfigFieldV1::Workspace` by M10, the two counts by the correction's receipt cases and by my 2b, and the text rendering by inspection (`workspace_crossings` is printed; I checked).

---

## 5. Reviewed and could not fault

- **The sum invariant, on the shapes the brief named.** Driven through the built binary on scratch fixtures: zero crossings (`relations 1, crossings 0`), only crossings (`0, 2`), duplicate identical crossings (`0, 2`), duplicate identical local edges (`2, 0`), no declaration at all (`1, 0`) — every one admitted with `relations + workspace_crossings` equal to the source's authored relation count. A crossing to an **undeclared** member and a **self-referential** edge are both refused (`semantic_mismatch` at the source), so neither reaches a receipt; `members: []` is refused at `invalid_project`/config, which matches `workspace.rs:220-228`'s explicit `EmptyDeclaration` rule and is the correct new behaviour rather than a regression. Log `shapes.log`.
- **The split is structurally incapable of breaking the sum** — `inventory` (`store_command.rs:1994-2003`) is one `if`/`else` over one iteration of the same frontmatter, so the two counts always add to the source. That is why finding 1 is about which side an edge lands on and not about the total, and why a sum assertion can never catch it.
- **`workspace_crossings` reaches `--format text`**, not only JSON: verified against the binary on a crossing fixture, `outcome.value.inventory.workspace_crossings 1`. The correction's claim holds.
- **The three edge readers are the only readers on the migration path**, and they call `crate::planning::declared_members`, not a copy. `inspect` and `dry_run` both refuse before anything counts; `mapped_histories` refuses before anything maps. Pass 1's invariant (e) still holds and the correction did not add a fourth view.
- **The correction's amended acceptance, as amended.** A crossing to a member this store is *not* — the real `entity-runtime/story:typed-references` case — behaves exactly as the correction says: admitted, no relation record invented, the authored relation carried in the subject's own body, counted as one crossing. My findings are about the case where the member named *is* this store, which is a different sentence from the one the coordinator amended.
- **No existing case was deleted, skipped, weakened or rewritten**, and the three pass-1 files the correction moved assertions in are untouched by me.

---

## 6. Every path I wrote outside the worktree

All under `~/.cache/ess-wave-v2/u8r2/`, the assigned scratch. **Nothing under `/tmp`.**
`TMPDIR=~/.cache/ess-wave-v2/u8r2/tmp` (**38 bytes**, inside the 45-byte `sun_path`
budget) was exported for every cargo and binary invocation; `CARGO_TARGET_DIR` was never set;
`CARGO_NET_OFFLINE=true` throughout.

| path | what it is |
|---|---|
| `…/u8r2/tmp/` | `TMPDIR` for every cargo run and every `aep` invocation; test fixtures live and die here |
| `…/u8r2/cold-build.log` | the cold `--no-run` build of the three lanes |
| `…/u8r2/caseA-red.log`, `caseA-red-final.log` | 2a, run alone, before and after the `rustfmt` reflow |
| `…/u8r2/caseB-red.log` | 2b, run alone |
| `…/u8r2/caseC.log` | 2f, run alone |
| `…/u8r2/caseD-red.log` | 2c, run alone |
| `…/u8r2/caseE-red.log` | 2d, run alone |
| `…/u8r2/origin.log` | 2e, the origin run |
| `…/u8r2/shapes.sh`, `shapes.log` | the eight-shape probe in part 5 |
| `…/u8r2/suite.log`, `suite-final.log`, `suite-final2.log` | the three suite runs; `suite-final2.log` is the one quoted in part 3 |
| `…/u8r2/suite-targets.txt` | the per-target summary lines the counts were summed from |

Fourteen paths. Scratch is **19 MB**; every probe fixture I created outside a test
(`tmp/dangle`, `tmp/dangle2`, `tmp/probe-text`, `tmp/shapes`, `tmp/only2`) is deleted. The
remaining entries under `tmp/` are the repository suite's own fixtures, not mine.

**What I deliberately did not do.** No `plan store migrate apply`, as a CLI verb or in process, on
any store, anywhere — which is also why finding 4 has no failing case (part 4). No other tree was
touched, read or built. No `git checkout`, `switch`, `stash`, `add`, `commit`, `branch` or
`worktree add`/`remove`; origin was settled with `git show` and one green case in this tree. No
`.engineering/` write and no `aep plan artifact` verb of any kind. No full `task check`, and part 3
says why.

**Disk.** `/` was 44 GiB free at the start and **31 GiB** at hand-back, never near the 15 GiB stop.

**Lease.** `worktree hook session-start --session ess-wave-v2-u8-review-2`, heartbeated through the
session, released with `worktree hook session-end` before hand-back.

---

## 7. Findings block

```findings
- file: crates/govern/aep-domain/src/artifact.rs
  line: 1324
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: crosses_to_a_declared_member has no notion of which member this store is, so an edge written with this store's own declared member name is counted as a workspace crossing and never imported as a relation record although its destination entity is in this authority.
- file: crates/plan/aep-planning-migration/src/mapping.rs
  line: 324
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a dangling edge prefixed with this store's own declared member name is admitted by the migration, which is exactly the hole mapping.rs:249-253 and artifact.rs:2526-2533 both state they close.
- file: crates/govern/aep-domain/src/artifact.rs
  line: 2533
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: plan artifact validate reports `valid` for a store whose only edge points at nothing, because the target is prefixed with a member name the workspace declares and that member is this store; unchanged at 763d195be, so it belongs outside this unit.
- file: crates/edge/aep-cli/src/store_command.rs
  line: 2335
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: inventory_from_histories never assigns the new workspace_crossings field, so the apply, verify, projection-rebuild and Eventlog-inspect receipts all publish 0 while their own schema description promises that relations and workspace_crossings sum to what the source declares.
- file: crates/edge/aep-cli/src/store_command.rs
  line: 906
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the inspect command's reader of the workspace declaration had no case anywhere in the tree, so replacing it with unwrap_or_default left the whole suite green; migration_inspect_reads_the_declaration_too.rs is now its detector.
```
