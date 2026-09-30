---
title: Quickstart
sidebar_position: 2
description: Install aep, adopt a Git repository, plan a story, move it on recorded evidence and validate the plan, in about ten minutes.
---

# Quickstart

This page installs `aep` and uses it in an empty Git repository. By the end you will have:

1. a project file that says which rules apply;
2. an epic and a story, each one Markdown file;
3. a refused move, and the evidence record that makes the same move legal;
4. a plan that `validate` accepts.

Every output below is copied from a real run of `aep 0.63.1`, so a newer `aep` prints its own
version where these print `0.63.1`. The install commands and the pinned protocol commit name the
current release, and so do the lines that echo that commit back. Absolute paths are shortened to
`…`.

## 1. Install

Download the archive for your platform from
[GitHub Releases](https://github.com/beyond10x/aep/releases), check it, and put `aep` on your
`PATH`:

{/* generated:release-pin:begin version=0.67.0 — kept by `cargo xtask status` */}
```bash
VERSION=0.67.0
curl -LO https://github.com/beyond10x/aep/releases/download/$VERSION/aep-$VERSION-x86_64-unknown-linux-gnu.tar.gz
curl -LO https://github.com/beyond10x/aep/releases/download/$VERSION/SHA256SUMS
sha256sum -c --ignore-missing SHA256SUMS
tar xzf aep-$VERSION-x86_64-unknown-linux-gnu.tar.gz
install -m 0755 aep-$VERSION-x86_64-unknown-linux-gnu/aep ~/.local/bin/aep
aep --version
```

Archives exist for `x86_64` and `aarch64`, on Linux (`unknown-linux-gnu`) and macOS
(`apple-darwin`). To build from source instead, with Rust 1.91 or newer:

```bash
cargo install --locked --git https://github.com/beyond10x/aep --tag 0.67.0 aep-cli --bin aep
```
{/* generated:release-pin:end */}

## 2. Adopt a repository

AEP needs two things from a repository: a `.engineering/project.yaml`, and a source for its
governing documents (lifecycles, relations, templates, principles). Pin that source to a commit, so
the rules cannot change underneath you without a commit in your own repository:

{/* generated:release-pin:begin version=0.67.0 commit=a9389f712394387a12b8fd78b8510b3f4788db85 — kept by `cargo xtask status` */}
```shell-session
$ cd shop            # any Git repository
$ aep plan reverse init --profile development.standard \
    --protocols git+https://github.com/beyond10x/aep#a9389f712394387a12b8fd78b8510b3f4788db85
…/shop/.engineering/project.yaml written
  protocol source resolves to …/protocol-sources/cd43e0b7…/snapshots/a9389f712394387a12b8fd78b8510b3f4788db85
  profile development.standard
  store: git (aep.project/5), planning_scope shop
```

The commit above is the `0.67.0` release. `reverse init` fetches that revision once into a local
cache and checks it before writing anything. The file it writes (its explanatory comment omitted):

```yaml
version: aep.project/5
protocol: adp/1
profile: development.standard
protocols: git+https://github.com/beyond10x/aep#a9389f712394387a12b8fd78b8510b3f4788db85
planning_scope: "shop"
store:
  git: {}
```
{/* generated:release-pin:end */}

`aep.project/5` is the Git-native store: the artifact files are the plan, and Git is its history.
[The planning store](./concepts/planning-store.md) explains the layout, and
[`project.yaml`](./reference/project-file.md) lists every field.

## 3. Plan an epic and a story

```shell-session
$ aep plan artifact new epic guest-checkout --title "Guest checkout"
created epic:guest-checkout (draft) at …/.engineering/planning/epic/guest-checkout.md
$ aep plan artifact new story pay-by-card --title "Pay by card as a guest" \
    --relate decomposes:epic:guest-checkout
created story:pay-by-card (draft) at …/.engineering/planning/story/pay-by-card.md
```

An id is `<kind>:<name>`, and the id decides the path. A new story starts with its kind's template
as the body. Replace the body with your own text by passing a file (`-` reads standard input):

```shell-session
$ aep plan artifact body story:pay-by-card --from story.md
story:pay-by-card body replaced (revision 2) at …/.engineering/planning/story/pay-by-card.md
$ aep plan artifact list
epic:guest-checkout  epic   draft  Guest checkout
story:pay-by-card    story  draft  Pay by card as a guest
```

Every write goes through the CLI and bumps the artifact's `revision`. Edit the body by hand if you
like. Leave `status`, `revision` and `transitions` to the CLI.

## 4. Move it, and meet the ladder

A story's lifecycle is `draft → proposed → active → implemented`. Jumping a rung is refused, and the
refusal lists the statuses you can move to:

```shell-session
$ aep plan artifact move story:pay-by-card --to active
story:pay-by-card is draft; a story may move to: proposed, archived
$ aep plan artifact move story:pay-by-card --to active --via
story:pay-by-card moved draft -> proposed (revision 3)
story:pay-by-card moved proposed -> active (revision 4)
```

`--via` walks the intermediate rungs and records each one as its own move. It stops at any rung
that needs evidence.

`implemented` needs evidence. The story lifecycle requires at least one `test_result`:

```shell-session
$ aep plan artifact move story:pay-by-card --to implemented
story:pay-by-card is active; implemented is on the ladder and not yet earned: reaching implemented needs at least 1 test_result record(s). no test_result record is held for this artifact — `aep plan artifact evidence <id> --kind test_result --source <where it came from>` records one
$ echo $?
1
```

*On the ladder and not yet earned* is a different answer from *not on the ladder*. The first one
tells you to go and record something.

## 5. Record the evidence, then move

```shell-session
$ aep plan artifact evidence story:pay-by-card --kind test_result \
    --source "cargo test -p checkout" --ref https://ci.example.invalid/runs/1042
story:pay-by-card: test_result recorded from cargo test -p checkout
  on hand: test_result=1
$ aep plan artifact move story:pay-by-card --to implemented
story:pay-by-card moved active -> implemented (revision 5)
```

The record names what it is about, where it came from and where to look. It is one new file, and it
is never rewritten:

```json
{
  "at": "2026-09-28T08:55:01Z",
  "actor": "human:alex",
  "artifact": "story:pay-by-card",
  "kind": "story",
  "revision": 4,
  "change": {
    "change": "evidence",
    "kind": "test_result",
    "source": "cargo test -p checkout",
    "reference": "https://ci.example.invalid/runs/1042"
  }
}
```

`actor` comes from `AEP_ACTOR` (`human:<name>`, `agent:<name>`, `service:<name>` or `system`). When
that is unset, the actor is `human:$USER`.

## 6. Ask why it is where it is

```shell-session
$ aep plan artifact explain story:pay-by-card
story:pay-by-card in …/.engineering/planning: implemented, revision 5
  draft -> proposed  2026-09-28T08:55:01Z  (revision 3)
    no record: nothing was recorded about how this was decided
  proposed -> active  2026-09-28T08:55:01Z  (revision 4)
    no record: nothing was recorded about how this was decided
  active -> implemented  2026-09-28T08:55:01Z  (revision 5)
    test_result from cargo test -p checkout (https://ci.example.invalid/runs/1042), observed 2026-09-28T08:55:01Z, admitted at revision 4
  next: archived needs no record
```

The story's file now carries that history in its front matter. Each move appended one line:

```markdown
---
format: aep.planning-md/3
id: story:pay-by-card
kind: story
status: implemented
title: Pay by card as a guest
relations:
- decomposes: epic:guest-checkout
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T08:55:01Z", actor: "human:alex", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-28T08:55:01Z", actor: "human:alex", revision: 4}
- {from: "active", to: "implemented", at: "2026-09-28T08:55:01Z", actor: "human:alex", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Pay by card as a guest
…
```

## 7. Validate, commit, check the checkout

{/* generated:release-pin:begin commit=a9389f712394387a12b8fd78b8510b3f4788db85 — kept by `cargo xtask status` */}
```shell-session
$ aep plan artifact validate
2 file(s) in …/.engineering/planning: 2 artifact(s)
valid
$ git add .engineering && git commit -m "plan: guest checkout"
$ aep doctor
ok    binary-version: 0.63.1
ok    project-file: ./.engineering/project.yaml parses: protocol adp/1, profile development.standard
ok    protocol-source: the locator `git+https://github.com/beyond10x/aep#a9389f712394387a12b8fd78b8510b3f4788db85` is well-formed and its snapshot is cached at …
ok    planning-store: ./.engineering/planning (store: git): 2 artifact(s), 1 evidence file(s), no problems
warn  plugin-directory: none given: pass `--plugin-dir <path>` or set `AEP_DRIVE_PLUGIN_DIR`. AEP ships no plugin sources and guesses no path
warn  release-tag: no bare-version tag is reachable from HEAD, so there is nothing to compare version 0.63.1 against — `git fetch --tags` first
```
{/* generated:release-pin:end */}

`validate` exits `1` when it finds a problem, so it works as a CI gate as it is. See
[Validate the plan in CI](./guides/validate-in-ci.md). `doctor` exits `1` on any `fail` line. The two
`warn` lines here are about the agent-driver setup and about release tags, and this quickstart
needs neither.

## What you have

```text
shop/.engineering/
  project.yaml
  planning/epic/guest-checkout.md
  planning/story/pay-by-card.md
  evidence/story/pay-by-card/20260928T085501Z-8506536e056a.json
```

That is the whole store. There is no database, journal or cache to commit.

## Next

- [Plan work](./guides/plan-work.md): relations, bodies, tags, blockers and the board.
- [Gate a move on evidence](./guides/gate-a-move-on-evidence.md): write a lifecycle whose rungs
  cost evidence, or open on a date.
- [Review with findings](./guides/review-with-findings.md): record reviews as data and compare two
  rounds.
- [Govern a task](./guides/govern-a-task.md): the engine half, which decides what an agent may do
  on a task and when the task is complete.
- `aep plan serve` opens the same plan in a browser on `127.0.0.1`, with the legal next rungs as
  buttons.
