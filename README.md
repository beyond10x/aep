# AEP

AEP (Agentic Engineering Protocol) keeps a repository's engineering plan as Markdown files that a
program governs. Stories, epics, reviews and blockers sit in `.engineering/planning/`. Each one
moves along a lifecycle declared in YAML. A move that a lifecycle gates on evidence is refused until
that evidence is recorded. Git holds the history.

The same rules apply to people and to coding agents. An agent that says "the tests pass" has not
moved a story. A `test_result` recorded against the story has.

AEP also includes a deterministic engine for governed tasks, a reference workflow driver and a
checker for agent transcripts. The [documentation](https://beyond10x.github.io/docs/aep/) covers
all of them.

## Why

- **A status is earned, not typed.** A lifecycle can require at least one `test_result` before a
  story reaches `implemented`. When the record is missing, `move` refuses and says which one.
- **The plan is plain files.** Each artifact is one Markdown file with YAML front matter, so a
  status change shows up as a small diff in a pull request. Two branches that touch different
  artifacts never conflict.
- **Every refusal lists your options.** An illegal move prints the statuses you can move to.
  `explain` shows what each earlier move rested on and what the next rung costs.
- **Rules are data.** Lifecycles, relations, principles and profiles are validated YAML. You can
  add your own without changing AEP's code.

## Install

Download a release archive for your platform from
[GitHub Releases](https://github.com/beyond10x/aep/releases). Each archive has an `aep` binary and a
`SHA256SUMS` entry.

```console
$ VERSION=0.63.1
$ curl -LO https://github.com/beyond10x/aep/releases/download/$VERSION/aep-$VERSION-x86_64-unknown-linux-gnu.tar.gz
$ curl -LO https://github.com/beyond10x/aep/releases/download/$VERSION/SHA256SUMS
$ sha256sum -c --ignore-missing SHA256SUMS
$ tar xzf aep-$VERSION-x86_64-unknown-linux-gnu.tar.gz
$ install -m 0755 aep-$VERSION-x86_64-unknown-linux-gnu/aep ~/.local/bin/aep
```

To build from source instead, use Rust 1.91 or newer:

```console
$ cargo install --locked --git https://github.com/beyond10x/aep --tag 0.63.1 aep-cli --bin aep
```

## A minute with it

In a Git repository, point AEP at a pinned copy of its governing documents. Then plan a story and
try to finish it:

```console
$ aep plan reverse init --profile development.standard \
    --protocols git+https://github.com/beyond10x/aep#88836a30f28ab2fddc3ab63d1ac54956973fa25e
$ aep plan artifact new story pay-by-card --title "Pay by card as a guest"
created story:pay-by-card (draft) at …/.engineering/planning/story/pay-by-card.md
$ aep plan artifact move story:pay-by-card --to active --via
story:pay-by-card moved draft -> proposed (revision 2)
story:pay-by-card moved proposed -> active (revision 3)
$ aep plan artifact move story:pay-by-card --to implemented
story:pay-by-card is active; implemented is on the ladder and not yet earned: reaching implemented needs at least 1 test_result record(s). …
$ aep plan artifact evidence story:pay-by-card --kind test_result --source "cargo test -p checkout"
story:pay-by-card: test_result recorded from cargo test -p checkout
  on hand: test_result=1
$ aep plan artifact move story:pay-by-card --to implemented
story:pay-by-card moved active -> implemented (revision 4)
$ aep plan artifact validate
1 file(s) in …/.engineering/planning: 1 artifact(s)
valid
```

The [quickstart](https://beyond10x.github.io/docs/aep/getting-started) walks through the same
steps and shows the files they write.

## Documentation

| Goal | Page |
|---|---|
| see what AEP is | [Overview](https://beyond10x.github.io/docs/aep/) |
| try it in ten minutes | [Quickstart](https://beyond10x.github.io/docs/aep/getting-started) |
| understand the model | [Concepts](https://beyond10x.github.io/docs/aep/concepts/overview) |
| look up a command | [CLI reference](https://beyond10x.github.io/docs/aep/reference/cli) |
| move an older store to the current format | [Migrate an older store](https://beyond10x.github.io/docs/aep/guides/migrate-an-older-store) |
| install the Claude Code or Codex plugins | [beyond10x/agentplugins](https://beyond10x.github.io/agentplugins/) |
| see what shipped | [CHANGELOG.md](CHANGELOG.md) and [docs/status.md](docs/status.md) |

If you are contributing to this repository, start with [`AGENTS.md`](AGENTS.md). It covers the
repository's layout, invariants and gate.

## Build and verify

The gate needs Rust (1.85 for the libraries, 1.91 for the CLI), [go-task](https://taskfile.dev) and
Node for the documentation site.

```console
$ task check
```

The gate never calls a model and never spends money.

## License

Apache-2.0. See [`LICENSE`](LICENSE).

<!-- b10x-docs:start -->
## Documentation

[AEP documentation](https://beyond10x.github.io/docs/aep/) · [Start](https://beyond10x.github.io/) · [Ecosystem](https://beyond10x.github.io/ecosystem/) · [Impact](https://beyond10x.github.io/changes/) · [Releases](https://beyond10x.github.io/releases/)
<!-- b10x-docs:end -->
