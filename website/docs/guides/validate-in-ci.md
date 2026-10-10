---
title: Validate the plan in CI
sidebar_position: 6
description: Run aep plan artifact validate as a required check on every pull request, with a pinned aep and an optional strict mode.
---

# Validate the plan in CI

`aep plan artifact validate` reads every artifact and evidence file and exits `1` on any problem, so
it is already a gate. Run it on every pull request that touches `.engineering/`, and make the check
required.

## What to pin

| What | Why | How |
|---|---|---|
| the `aep` binary | a newer or older `aep` can read the store differently | download one release archive and check its checksum |
| the governing documents | the lifecycles decide which statuses are legal | `protocols:` in `project.yaml` is a path in the repository or a Git locator pinned to a commit |

A pinned Git source is fetched into a cache on first use (`AEP_CACHE_DIR` moves the cache). The job
therefore needs network access to that repository, or a vendored tree.

## A GitHub Actions job

{/* generated:release-pin:begin version=0.71.2 — kept by `cargo xtask status` */}
```yaml
name: Planning store
on:
  pull_request:
  push:
    branches: [main]
permissions:
  contents: read
jobs:
  validate:
    runs-on: ubuntu-latest
    env:
      AEP_VERSION: "0.71.2"
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0 # the full history: validate dates a review by the commit that added it
      - name: Install aep
        run: |
          base=https://github.com/beyond10x/aep/releases/download/$AEP_VERSION
          archive=aep-$AEP_VERSION-x86_64-unknown-linux-gnu.tar.gz
          curl -sSLO "$base/$archive"
          curl -sSLO "$base/SHA256SUMS"
          sha256sum -c --ignore-missing SHA256SUMS
          tar xzf "$archive"
          echo "$PWD/aep-$AEP_VERSION-x86_64-unknown-linux-gnu" >> "$GITHUB_PATH"
      - name: Validate the planning store
        run: aep plan artifact validate
```
{/* generated:release-pin:end */}

Pin the checkout action to a commit if your policy requires it. The job needs no credentials: it
reads files and writes nothing.

`fetch-depth: 0` matters to a project whose `project.yaml` sets `findings_required_since`. The
checkout's default is a clone of depth 1, in which every file appears to be added by the one commit
it holds, so no review in it can be dated. `validate` leaves such a review undated rather than
date it by the clone, and an undated review with no findings block and no other exemption is a
problem: the problem names the shallow clone and the full history as the remedy. The same holds
for a store validated outside its Git repository, such as a `git archive` export.

## Strict mode

`validate` separates problems (exit `1`) from findings it only reports:

| Reported, exit `0` | Why it is allowed by default |
|---|---|
| a status reached on an assertion (`move --evidence …`) rather than a recorded file | somebody may close a story on the day a runner is down |
| a `review-result` whose findings are prose only | a review written as prose is still a review |
| a review older than `--outcome-within` days (default 14) with no `review_outcome` | outstanding work, not a broken store |

`aep plan artifact validate --strict` exits `1` on any of these, and names which class decided.
A project whose `project.yaml` sets `findings_required_since` counts a review with no findings block
as a problem instead, unless it is exempt; see
[Requiring a findings block](../concepts/reviews.md#requiring-a-findings-block).
Use it where the plan must hold only recorded evidence, for example on `main`. The review age is the
one line that depends on when the job runs.

## Also worth running

| Command | Gate on |
|---|---|
| `aep doctor` | exits `1` on any `fail`: a project file that does not parse, a protocol source that does not resolve, a store `validate` would reject |
| `aep plan workspace crossings --strict` | every edge into another repository of a workspace resolves; only meaningful when every member is checked out |
| `aep plan artifact waves` | exits `2` on a `depends_on` cycle |

## Locally, before you push

The same command works as a pre-push check:

```bash
aep plan artifact validate || exit 1
```
