---
name: planning
description: Plan engineering work in a governed markdown artifact store — create, relate, move and validate epics, stories, tasks and initiatives through the `protocol` CLI. Use when the user mentions planning, a backlog, an epic, a story, a task, decomposing or breaking down work, an artifact's status ("move this to active", "what is still in draft?", "why can't this be implemented?"), or when the project contains a `.engineering/planning/` directory. Use it at adoption too — the user asks to adopt engineering protocols, to migrate from or replace the track plugin, to start a first backlog, or works in a repository with no `.engineering/` directory at all — because § 5 says how a first store is populated and it is worth nothing after one has been hand-written. Also use before editing any file under `.engineering/planning/`.
---

**Skill version 0.2.0** — the version in `.claude-plugin/plugin.json`.

# Planning in a governed artifact store

## 1. The model

Artifacts are markdown files under `.engineering/planning/<kind>/<slug>.md`: YAML frontmatter the
CLI owns, and a body you and the operator own. Which kinds exist, which statuses each kind may hold
and which moves between them are legal come from validated lifecycle documents, not from convention
and not from this file. The `protocol` CLI is the authority on both, so every question about
vocabulary has a command that answers it.

## 2. Discover, do not memorise

This skill inlines **rules only**. It deliberately carries no list of kinds, statuses, legal moves
or relations. Ask for them at the moment you need them:

| Question | Command |
|---|---|
| What kinds can I create? | `protocol artifact kinds` |
| What edges exist between artifacts? | `protocol artifact relations` |
| What statuses does this kind have, and what moves where? | `protocol artifact lifecycle <kind>` |
| What is already in the store? | `protocol artifact list [--kind k] [--status s] [--format json]` |
| What does it look like as a board? | `protocol artifact board [--kind k]` |
| What is stopped, on what type of thing, and on which item? | `protocol artifact blocked [--type t]` |
| How is it wired together? | `protocol artifact graph` |
| What does this one artifact say, frontmatter and body? | `protocol artifact show <id>` |
| What has happened to it, oldest first? | `protocol artifact history <id>` |
| Why is it at this status — what did the store admit before each move? | `protocol artifact explain <id>` |
| What writes has one side of a hybrid plan taken that the other has not? | `protocol artifact divergences` |
| Is the whole store still consistent? | `protocol artifact validate` |

That is every `protocol artifact` verb that answers a question. The six that are missing from it
write — `new`, `move`, `relate`, `body`, `evidence`, `catch-up` — and guardrail 2 governs those.
Run `protocol artifact --help` when this table and the CLI disagree; the CLI is right.

The reason is the reason this project exists. Lifecycle and relation documents are validated and
versioned; a prose copy of them in a skill file is neither, and it goes stale the first time a kind
gains a status. An agent that recites `draft → proposed → active` from memory will confidently
propose an illegal move in a store that renamed one of them. Reading `protocol artifact
lifecycle story` costs one command and cannot be wrong.

When a store is present but you have not looked at it yet in this session, start with `protocol
artifact list` and `protocol artifact kinds`. Two commands buy you the whole vocabulary.

## 3. Six guardrails

These are inlined because they hold whatever the store's vocabulary is.

**1. A status changes only through `protocol artifact move`.** Never edit the `status:` field in
frontmatter, and never write it into a file with `Edit` or a heredoc. The CLI validates the move
against the kind's lifecycle; a hand-edited status is an unvalidated one, indistinguishable in the
file from a legal one and wrong in exactly the cases that matter.

```console
$ protocol artifact move story:credential-store --to proposed
story:credential-store moved draft -> proposed (revision 2)
```

Some rungs cost evidence — their kind's lifecycle declares a `requires:` entry for them — and the
way to pay is to record the observation, before the move rather than at it:

```console
$ protocol artifact evidence story:credential-store --kind test_result \
    --source "task check" --ref https://ci.example/run/8412 --at 2026-08-30T14:02:00Z
```

`move` finds evidence recorded against the artifact without being told, so nothing is passed to it.
The kind list is **closed**: `protocol artifact evidence --help` documents the flag, and a kind
outside the list is refused with the whole list printed — read it from that refusal rather than
inventing a plausible name for what you observed. `--ref` and `--at` are optional and are what make
the record checkable later. `move --evidence <kind>=<count>` is the asserted form, kept for a
record that lives outside the store; it names no run and no artifact, and the store marks a move
that rests on it as resting on an assertion.

**2. Every store mutation uses `protocol artifact`; never edit a store file directly.** Creation,
relations, status, and prose use `new`, `relate`, `move`, and `body` respectively. Supply the complete
body from a file or standard input — `body --from` after creation, or `new … --from` at creation; the
CLI preserves frontmatter, validates the store, and bumps the revision once when bytes change. For a
kind whose records are immutable (`review-result` refuses `body`), `new --from` is the only way a
body arrives, and `move --to archived` is the only way one is retired.

```console
$ protocol artifact body story:credential-store --from story-body.md
story:credential-store body replaced (revision 2) at .engineering/planning/story/credential-store.md
```

**3. After a batch of edits, run `protocol artifact validate`, and relay its output verbatim.** It
accumulates every problem rather than stopping at the first, and exits 1 if any remain. Do not
summarise it into "validation failed" — the output names each artifact and each defect, which is the
only part the operator can act on.

**4. A refusal is the answer, not an obstacle.** When the CLI refuses a move it exits 1 and names
every status legal from where the artifact stands. Relay that list. Do not retry with a different
spelling, do not route around it by editing the file, and do not pick an intermediate status to
"get there" without saying so.

```console
$ protocol artifact move story:credential-store --to implemented
story:credential-store is proposed; a story may move to: draft, rejected, active
$ echo $?
1
```

The right response is to tell the operator that the story must be active first, and ask whether to
walk it there — not to perform two moves nobody sanctioned.

**5. A request that is already satisfied gets an artifact, not a question.** Finding that the asked-
for behaviour already exists — or should not be built — is a real result and often a better one than
the change. Record it: write the `specification` that states what you found, cite the code and the
command output that show it, and say plainly that you did not build the thing and why. Then file the
gap you actually found, if there is one.

What this rules out is ending the turn on *which of these two would you like?*. A session that stops
to ask has produced nothing, and there is frequently nobody there to answer — the same work run
non-interactively ends with an empty tree and a question into a log. `adp/default` has a terminal
`declined` state for precisely this outcome, and the distinction it draws is the one that matters
here: **a decline that is written down is a result; a decline that is only said is a run that did
nothing.**

This is not licence to argue with the request. Build what was asked for unless you have evidence it
is already there or actively wrong, put that evidence in the artifact with `protocol artifact
body`, and leave the decision where § 4 leaves the ones that are the operator's — with the
operator, who can now read what you found instead of answering a question.

**6. Something parked is recorded, not described.** If `protocol artifact kinds` lists no blocker
kind, that is not the answer — ask `protocol artifact lifecycle <type>-blocker` as well. Where that
ladder answers, file the blocker as a `decision-blocker`, or as the `<type>-blocker` whose ladder
answered, and say in your report that `kinds` does not list it. Where neither answers, the store
cannot hold it as an artifact: record it in the blocked artifact's body through `protocol artifact
body`, and say plainly that it is a paragraph rather than an artifact, so nobody expects
`protocol artifact blocked` to find it.

A blocker artifact is typed by what would clear it and carries a `blocks` edge to the work it is
stopping. Never leave the fact in a status field: an item parked for nine days on a credential and
an item somebody is working on today are both `active`, and only the blocker tells them apart.
Unblocking is `protocol artifact move <blocker> --to cleared`, a move like any other, which is why
the record survives it.

```console
$ protocol artifact lifecycle decision-blocker
decision-blocker starts at open
  cleared -> nothing
  open -> cleared

$ protocol artifact new decision-blocker api-token-scope \
    --title "Nobody has decided which account mints the CI token" \
    --withholds test_result --relate blocks:story:ci-evidence
created decision-blocker:api-token-scope (open) at .engineering/planning/decision-blocker/api-token-scope.md
```

`--withholds` is optional and names the evidence kind nobody can produce while this is open, so
`protocol artifact explain <blocked-id>` answers *why is there no record for the next move*. It only
means something beside `blocks:`, and `validate` says so.

## 4. Who decides

A move is a claim about the state of the world, and for most rungs the store already holds what
settles it. Read it there before asking anybody.

* New artifacts are created in the lifecycle's initial status — `protocol artifact new` does this,
  and it is the correct starting point. Do not immediately move them.
* **Make the move when the store holds what the rung requires.** A rung whose lifecycle declares no
  `requires:` entry costs nothing to reach: on work you were asked to do, make the move and report
  it. A rung that costs evidence is settled by what has been recorded against the artifact — record
  the evidence (guardrail 1), run `protocol artifact move`, relay what it printed. Do not ask
  permission for a move the store would allow: it buys one more question and no work, and whether
  the thing is implemented is a fact the store holds and the operator does not.
* **Ask only when the evidence is missing, and name what is missing.** The refusal writes that
  question for you — it names the rung, what the requirement said, and what nothing was presented
  at. Relay it, name the record that would close it, and say who or what could produce that record.
  A question carrying a named missing record is worth an operator's time; "shall I move this?" is
  not.
* **A decomposition is drafted and reported, not held for confirmation.** Draft the stories, write
  their bodies, and report the set together with what you deliberately did not cover. Every draft
  lands in the initial status and is reversible; an undrafted decomposition is not the safer one,
  it is the one nobody can read.
* Never perform a bulk move autonomously. "Archive everything still in draft" is an instruction;
  inferring it from a tidy-up request is not.

Two things stay with the operator: a move whose evidence does not exist, and a bulk move nobody
asked for. Everything else was already asked for when the work was.

## 5. Starting from a repository that has no store

An empty store in a repository with 40,000 lines of code is not a blank page — it is a plan somebody
has been carrying in their head. Do not open the editor and start typing epics.

```console
$ protocol reverse init --protocols <source> --profile <profile>
$ protocol reverse scan --format json
```

`reverse init` writes `.engineering/project.yaml` and refuses the two things that quietly break
later — an absolute path, and a `git+` source pinned to a branch rather than a commit. `reverse scan`
reads and interprets nothing: it emits located facts — README headings, marked lines, tests that say
they will not run, CI jobs and the variables on them, task targets, packages, published contracts —
each carrying the `path:line` it was read from. It writes nothing and it has no clock and no network,
so two runs over one tree give identical bytes.

Then, in a Git working tree:

```console
$ protocol reverse history --format json
```

The axis the scan has none of. It joins to the scan on `path:line` and dates every marked line and
every disabled test from the commit that wrote it, alongside what the history says about itself:
reverts, commits that hedged (*for now*, *until we*), churn, dormancy, tracker keys. Dates are quoted
from commits and never compared against today, so this is byte-stable too.

**Use it, and lead with what it tells you.** A suite switched off is an observation; a suite switched
off since February 2024 is a finding. The first gets scrolled past.

**Then cite what you file.** An artifact drafted from a bundle entry carries that entry's
`path:line` in its body. This is the whole reason the scan is a separate program from the session
that reads it: a plan you invented from a plausible reading of the code is indistinguishable, later,
from a plan somebody agreed to, and it is the worse of the two because nobody can check it.

The `reverse-engineer` agent does this end to end and reports what it could **not** cite. Use it
rather than reproducing the loop by hand, and read its fourth report section first — the things that
look like real work and have no evidence in the tree are the questions worth the operator's time.

A scan reports what is *written down*. A convention that lives in review comments, or the reason a
module exists, is not in the bundle and must not be invented into one.

## 6. A worked decomposition

An epic, two stories derived from it, one move, one validation.

```console
$ protocol artifact new epic passkey-login \
    --title "Passkey login" \
    --summary "Replace password sign-in with WebAuthn passkeys."
created epic:passkey-login (draft) at .engineering/planning/epic/passkey-login.md

$ protocol artifact new story credential-store \
    --title "Store and retrieve passkey credentials" \
    --relate decomposes:epic:passkey-login
created story:credential-store (draft) at .engineering/planning/story/credential-store.md

$ protocol artifact new story registration-ceremony \
    --title "Register a passkey during sign-up" \
    --relate decomposes:epic:passkey-login
created story:registration-ceremony (draft) at .engineering/planning/story/registration-ceremony.md
```

Then write each story's complete body through `protocol artifact body <id> --from <path|->` — or
hand it to `new … --from <path|->` in the first place — one acceptance statement per story, because
guardrail 2 makes the CLI the store's sole writer.

Then the one move the operator asked for, and the check:

```console
$ protocol artifact move story:credential-store --to proposed
story:credential-store moved draft -> proposed (revision 2)

$ protocol artifact validate
3 file(s) in .engineering/planning: 3 artifact(s)
valid
```

Had `validate` found something, its output would have gone to the operator unedited.

## 7. Scoping what is already there

A store that does not record what its stories touch cannot be sequenced. Two items on one file
conflict whichever order they land in, so *which surfaces does this touch* is the property that
decides what can be worked at once — and in most stores nothing holds it.

The `story-scoper` agent answers it for one artifact: it reads the body, its edges, the symbols it
names and the tree, and returns a `## Scope` section marking every line **cited** or **inferred**.
It is read-only, so run one per story and run them at once; write what they return through
`protocol artifact body`, one at a time, because the journal is append-only and N writers race.

Read the confidence line, not just the surface. A scope that mixes what was read with what was
guessed is worse than none — it gets trusted exactly where it is weakest — which is why the section
separates them and why a scoper reports what it could **not** establish beside what it could.

## Reference

The on-disk format — directory layout, filename and id rules, which frontmatter fields are
machine-owned, and a complete example file — is in
[references/store-conventions.md](references/store-conventions.md). Read it before changing a store
document. Everything else is a question for the CLI.
