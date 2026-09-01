# Workflow-declared context and write scope — Design v0.1

> **Repository:** `beyond10x/aep`
> **Status:** **accepted 2026-08-24, milestones 1–3 in build.** Milestones 4–6 are not started and
> are deliberately unsequenced.
> **Audience:** whoever reviews this for acceptance, and whoever would build it afterwards.
> **Relationship to existing design:** additive. It adds two declarations to the step map and one
> field to the frame; every enforcement path it uses already exists and already has a record.

## 1. The two runs that bought this

**A live run of arm `native` against `eval-case/development-default`, 2026-08-24.** It did every
step the case asks for — specification before stories, the failing test before the code, the suite
run, the store validated — and contradicted exactly one row: it wrote artifact files **whole**
(`file.write` on `.engineering/planning/**`) instead of editing bodies and letting the CLI own the
frontmatter. 10 held, 1 violated, 0 undecidable. It also spent six of fifty-two tool calls
discovering the CLI (`--help` five times, then `artifact relations` and `artifact kinds`).

**The arm it is measured against does not have to do either.** `claude+plugin` loads a 157-line
`integrations/claude-code/skills/planning/SKILL.md`. Its guardrail 2 reads, verbatim: *"a targeted
edit below the closing `---`, never a whole-file rewrite"* — the exact rule the native arm broke —
and its § 2 says *"two commands buy you the whole vocabulary"*, which is the discovery the native
arm paid six calls for.

So the matrix is currently comparing **a harness carrying domain knowledge against a harness
carrying none**, and reporting the difference as a property of the harnesses. That is the defect
this design closes, and it closes it in the place both arms read from rather than by handing one arm
a file.

## 2. The rule already exists, in one vendor's words, for one arm

`crates/protocol-cli/src/drive.rs:1610` is `store_integrity`, and it is exactly this rule:

```rust
let target = match tool {
    "NotebookEdit" => input["notebook_path"]...,
    _              => input["file_path"]...,
};
if !target.contains(".engineering/planning/") { return Ok(()); }
if tool == "Write" || tool == "NotebookEdit" { return Err(...whole-file rewrite...) }
for field in ["old_string", "new_string"] { ...refuse text crossing the `---` fence... }
```

Three things follow, and they set the whole design.

**It is written in Claude Code's tool names and argument names.** `Write`, `NotebookEdit`,
`file_path`, `notebook_path`, `old_string`. On the b10x arm a write is `tool_invoke` naming
`file_write` with its path under `arguments.path`, so this function matches nothing and the run in
§ 1 walked straight past a guardrail this repository has enforced for a year. **This is the third
time the same blindness has cost something** — the corpus's write rows, its shell rows, and now the
seam's own policy — and it has the same fix each time: say it in the neutral vocabulary.

**It is code, not declaration.** One arm's driver holds it. Nothing in the workflow says the store
has an owner, so no other arm can honour it and no reader can find it.

**It is two rules wearing one name.** *Whole-file replacement under the store* is a question about
the **path and the granularity**. *Text crossing the `---` fence* is a question about the **content**
of an edit. Only the first is expressible as a scope, and the corpus already says so in terms:
*"that second half is not transcript-decidable from a path"*. This design lifts the first and leaves
the second exactly where it is.

## 3. What is proposed, and in whose vocabulary

Two declarations on an `llm` step in the step map (`aep.driver-steps/1`):

```yaml
- kind: llm
  description: Write the failing test before the code.
  skills: [planning]
  context:
    - integrations/claude-code/skills/planning/SKILL.md
    - crates/protocol-cli/src/planning.rs
  scope:
    - paths: [".engineering/planning/**"]
      write: partial-only     # the CLI owns the frontmatter; edit bodies, never replace files
    - paths: ["crates/**", "docs/**"]
      write: allowed
    - paths: ["**"]
      write: denied
```

### The vocabulary question, answered in three layers

The tempting shape is to key the scope by operation — `file.write: {deny: [...]}`,
`file.edit: {allow: [...]}`. **It should not be.** A step map is a document about *work*, and
`file.write` is a name in metaharness's `Operation` enum; putting it here couples two protocols at
their most volatile point, and every operation added later needs a new key in every map or silently
falls outside every scope. An operation nobody mentioned is the dangerous case, and a shape whose
default is ambiguous is the wrong shape.

Nor should it be fully name-free in the other direction. `write: allowed | denied` alone cannot
express the one rule that motivated this, because `file.write` and `file.edit` are both writes.

So the distinction the document makes is **granularity, not identity**:

| word | means |
|---|---|
| `allowed` | this operation class may act on these paths |
| `partial-only` | it may change part of a file and never replace one whole |
| `denied` | it may not act here at all |

Three words, no operation names, and `partial-only` is not invented for this rule — it is a property
the catalogue already documents: *"`file_write` — Write one file, whole. Replacing an existing file
replaces all of it — use `file_edit` to change part of one."* Which of an adapter's operations are
whole-file replacements is **the adapter's** fact, and it is the one place that knows.

The layering that falls out is the one this repository already uses everywhere else:

| layer | speaks | example |
|---|---|---|
| step map | coarse and agnostic | `paths: [".engineering/planning/**"], write: partial-only` |
| frame | precise and sealed | `file.write` denied for those subjects, `file.edit` admitted |
| seam | per call | operation ∈ admitted **and** subject ∈ scope |

The compilation from the first to the second lives with the vocabulary, in the adapter — the same
place `rendering()` already maps operations to that harness's names. A step map author never writes
an operation name; a frame never carries a glob the seam has to interpret at call time.

### First match wins, and the last rule is mandatory

Rules are ordered and the first whose `paths` match decides. A `scope` whose final rule is not a
catch-all is **refused at validation**: a path nobody mentioned must have an answer, and leaving it
to a default is how a scope silently stops covering the tree it was written for.

## 4. Why the enforcement is nearly free

The frame already decides every call. `Frame.operations: OperationSet` says *which operations are
admitted here*, it is sealed into the frame's digest, the seam denies per call, and every decision
lands in `tool.decided` and the census. What is missing is **where**, and the input it needs now
exists: `tool.requested.subjects` carries `file:crates/protocol-cli/src/planning.rs` and
`proc:/usr/bin/cargo` in the neutral form, on every arm.

So:

| piece | state |
|---|---|
| a neutral operation per call | **exists** |
| a neutral subject per call | **exists** (added 2026-08-24) |
| a sealed per-step admitted set | **exists** (`Frame.operations`) |
| per-call denial at a seam, recorded and counted | **exists** |
| a **subject** scope beside the operation set | new — one field, sealed with the rest |
| the step map declaring it | new — two keys |
| preloaded context reaching the run | new — see § 6 |

The verdict becomes: *admitted operation **and** subject inside its scope*. A call that fails either
is denied with a reason naming which, and the run continues.

## 5. Fail the operation, never the run

The operation is refused; the run carries on. Nothing else is a good default, and the three things
people mean by "fail" are worth separating because only one of them belongs here:

1. **Deny the call.** The write does not happen. This is what the seam already does for an operation
   outside the admitted set, and it is what a subject outside its scope should do too.
2. **Count it.** Every denial already lands in `tool.decided` and the census. A run that stayed
   inside its scope and one that reached outside it forty times are different facts about the model,
   and flattening them is how `nothing-was-refused` stopped saying anything on the driven arm.
3. **Fail the run.** Not here. A run killed for reaching once makes the scope a trap rather than a
   boundary, and the reaching is exactly what a denial is for. `--hermetic strict` and the auditor
   already exist for whoever wants a threshold, and the census in (2) is what makes one choosable
   later without changing any of this.

**The denial has to be usable by the model, or it is a loop.** A refusal that says only "denied"
gets retried until the turn budget runs out — which is money spent on a wall. The reason must name
the path, the class that was refused, and what would work instead, in the run's own vocabulary:

> `.engineering/planning/story/board-json-cli-contract.md` may be changed in part but not replaced
> whole. Use an edit that names the text to replace; the CLI owns the frontmatter.

That is the same sentence `store_integrity` already writes, which is the second reason to lift it
rather than reimplement it: the wording was worked out once, against a real run, and it is good.

## 6. Preloading is not free and the design should say so

A stateless loop replays its conversation every turn, so a preloaded file is paid for on **every**
turn, not once. Measured on the run in § 1: 674,962 input tokens across 24 turns, of which 89% were
cache hits — the replay is cheap but not free, and a large `context` list is a permanent tax.

It is still very likely a saving, because what it replaces is worse: each discovery read is a tool
call, a turn, *and* a result that then joins the same replay. Six calls became six turns became
six results. But the design must not pretend the trade is one-directional, and `context` should be
bounded — a handful of files, declared, not a directory.

Two properties follow:

* **`context` is a list of files, never a glob.** A glob is a promise about a directory's future
  size, and a step map is pinned.
* **A file named in `context` and absent at run time is a refusal**, not a warning. A run given a
  smaller context than its map declares is a run nobody can reproduce from the map.

## 7. What this does not do

* It does not make the workflow a policy engine. Globs only — no regex, no boolean expressions —
  matching the same refusal `trace-spec` makes for the same reason.
* It does not read scope from the model's own claims. The subject comes from the harness, which is
  why it works across arms.
* It does not replace the plugin. A skill is prose that teaches; a scope is a boundary that holds.
  A run should have both, and the point of putting `context` in the map is that **every arm** gets
  the prose, not only the one with a plugin loader.
* It says nothing about `shell` scope. `proc:` subjects exist and the same mechanism would carry
  them, but the declared-program set already bounds execution and adding a second bound before
  anybody has wanted one is speculative.

## 8. Milestones, unsequenced

1. `scope` and `context` in `aep.driver-steps/1`, validated, with the refusals in § 5.
2. `Frame.subjects` beside `Frame.operations`, sealed into the same digest.
3. The seam's verdict reads it; the denial reason names the scope it fell outside.
4. `context` reaches a run: for the b10x arm through the standing instruction, for a vendor arm
   through whatever that adapter already uses to seed a session.
5. `store_integrity`'s path half is deleted from `drive.rs` and declared in the step map instead.
   Its fence half stays in code, because content is not a scope. The driven arm then enforces the
   same declaration every other arm does, from the same document, rather than from a function only
   it can reach.
6. The corpus row `no-artifact-file-was-rewritten-whole` gains a sibling that asserts the **denial**
   rather than the absence — the absence is now guaranteed by construction, and a control that
   cannot fail is a control that has stopped testing anything.

## 9. The measurement that would accept or reject this

Re-run `eval-case/development-default` on arm `native` with a step map carrying `context` and
`scope`, and compare against the run in § 1 and against `claude+plugin`:

| | claude+plugin | native, today | native, expected |
|---|---|---|---|
| held | 11 | 10 | 11 |
| violated | 0 | 1 | **0, and denied rather than avoided** |
| turns | ~19 | 24 | ~18 |
| cost | $0.5216 | $0.6572 | ~$0.45 |

The second row is the one worth reading. Reaching parity on *held* by telling the model the rule is
what the plugin already does. Reaching it because the toolset **refused** is the claim arm `native`
exists to make, and it is the only column where this design wins rather than catches up.
