# Changelog

Notable changes to AEP. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html), where a **major**
version is a breaking change to a protocol's semantics, not merely to a Rust API.

Entries record what changed for someone using the protocol. Rationale that does not fit in a line
belongs in the commit message or in `docs/design/`.

## [Unreleased]

### Added

- A `review-result` body may carry a fenced ```` ```findings ```` block of YAML — entries of
  `{file, line, category, severity, verdict, origin, message}`, where `file`, `category`, `severity`
  and `message` are required and an unwritten `origin` reads as `undecided`. `aep artifact new`
  parses it and refuses a malformed one **with the body line it is wrong on**; `aep artifact show
  --format json` returns it as a `findings` array; `aep artifact validate` reports a `review-result`
  that states its findings as prose only, without failing. `severity` is `blocker|warning|note`,
  `verdict` the adversary's `CONFIRMED|NEEDS-CHANGE|INFEASIBLE` or a critic's
  `approve|needs-revision`, `origin` `introduced|pre-existing|undecided`. Before this, a review's
  findings were sentences: the next round of the same reviewer started from nowhere.
- `aep artifact findings <artifact-id> [--from <review-result-id> --to <review-result-id>]` says
  what the second review of an artifact found that the first did not — `carried`, `new`, `resolved`
  over the two most recent `review-result` records that `reviews` it, or the two named. A finding is
  the same finding when its file, its category and its normalised message match and its line is
  within three, so a finding that moved because somebody added an import is carried rather than new.
  The reviewer is printed and never matched on. Always exits 0.
- Evidence kind `review_outcome`, and the two flags it is made of:
  `aep artifact evidence <reviewed-id> --kind review_outcome --review <review-result-id> --outcome
  no-op|fixed|escalated`. A review is immutable once recorded, so what became of it is a later record
  naming it rather than an edit to it. Refused when that review declares no `reviews` edge to the
  artifact, when either flag is given without the kind, and when the kind is given without both.
  `aep artifact show <review-result-id>` prints every outcome recorded against it, and
  `aep artifact validate --outcome-within <days>` (14 by default) reports a review older than that
  which nothing says happened to — reported, not failed on.
- `aep artifact review-value [--since <YYYY-MM-DD>]` prints one row per reviewer — the
  `review-result`'s `owner`, or a `reviewer:` key its document carries — with reviews, findings,
  `no-op`, `fixed`, `escalated`, and what the runs behind them cost where a run manifest named by one
  of the review's `--ref` values said so. **No score, no ranking and no percentage**: the position
  `aep eval matrix` takes, for the same reason. A cost nobody recorded prints as `unknown`, never as
  `0`.
- Planning stories carry `scope:` — the surfaces a story lands on, each entry a `path` and a
  `confidence` of `cited` (read out of the artifact, a diff or a file somebody opened) or `inferred`
  (worked out and not read anywhere). `aep artifact scope <id> --add <path> [--inferred]` and
  `--remove <path>` are the doors; `show` prints the list and `show --format json` carries it as an
  array. Story only. A path already declared is replaced rather than doubled, one command is one
  write and one revision, and the write is journalled like every other mutation. Nothing is
  normalised: `crates/x` and `crates/x/src/lib.rs` are two surfaces on purpose.
- `aep artifact waves` derives which stories may be implemented at once, from `scope` and
  `depends_on` and nothing else. Inside a wave no two stories share a scope path; a story is never
  in the same wave as anything it depends on, nor before it, including through a story that was not
  itself placed. Every pair sharing a path prints as `collision: <a> <b> <path>`, marked
  `(inferred)` when either side worked the path out rather than reading it. A story with no `scope`
  prints under `unassessed` and is never placed — an unassessed story reads exactly like a safe one,
  which is the defect. A `depends_on` cycle prints its own ids and exits `2`. `--kind` defaults to
  `story` and `--status` narrows to one part of the board. It reads and prints: choosing the wave
  stays the operator's.
- `aep artifact validate` lists every story past its ladder's first rung and short of its end that
  declares no `scope`. Reported, never failed on, and deliberately not a `--strict` class: 24 of 40
  stories would have gone red on the day it landed, which is how a check gets muted.
- `aep reverse openapi` drafts a `relations:` block on every type whose schema states one. Two
  signals and no others: a property whose schema is a `$ref` to a schema that becomes an entity
  (`cardinality: one`, or `many` when the property is an array of that `$ref`), and a property named
  `<x>_id` or `<x>Id` whose type is the type of entity `X`'s own identity property. Every relation
  is `kind: references` — an OpenAPI document says a payload carries a reference and says nothing
  about whose life bounds whose, so `owns` is never inferred and every relation read from an id
  field carries an `UNMAPPED: ownership` line. A property that states its target two ways, such as
  `oneOf: [Carrier, [Carrier]]`, is emitted with `UNMAPPED: cardinality` over the placeholder
  `cardinality: one`; a property with neither signal produces nothing at all, and a schema with no
  signal carries no block. The second adopter's plan defect was an undecided domain relation, and
  this puts the ones the contract does state into the draft with the undecided half named.

### Fixed

- `aep artifact new --ref <provider>:<key>` writes the reference to the document. It was accepted,
  echoed in the result and dropped, so no artifact created with it carried the reference and
  `aep artifact list --ref` could not find it. On a `review-result` this was the only door — `set` is
  refused on an immutable kind — which is why no review could name the run manifest its cost is read
  from. Shipped broken in 0.41.0.

## [0.42.0] — 2026-09-02

### Added

- `aep doctor` reports whether a checkout is in a state the other verbs will accept — the binary's
  version, `.engineering/project.yaml`, the `protocols:` source it names, the planning store
  (through `artifact validate`'s own accumulation), each `--plugin-dir` or `AEP_DRIVE_PLUGIN_DIR`
  directory, and the newest bare-version tag reachable from `HEAD` — one line each as
  `ok`/`warn`/`fail`, exit `1` on any `fail`. It fixes nothing, reads no clock and opens no
  connection: a pinned `git+…#<40-hex>` source is checked for shape and for a cached snapshot, never
  fetched. Asked for by the second adopter, whose report ended "I am still unsure if I even used
  them properly".
- The epic, story and specification templates seed an `## Ambiguities` section. Each gap is
  classified `inferable`, with the `path:line` or artifact id that settles it, or
  `requires-stakeholder-input`, with who decides — and the second kind is raised as a
  `decision-blocker` with a `blocks` edge to the artifact. The story template's `## Open Questions`
  points there rather than restating it. An undecided domain relation was the second adopter's
  plan defect; this gives it a line somebody reads.

### Changed

- The README and the documentation site name `beyond10x/agentplugins` and its install page as the
  source of the Claude Code and Codex plugins, with the `/plugin marketplace add
  beyond10x/agentplugins` then `/plugin install aep-planning@beyond10x` form. The 2026-09-01 split
  removed this repository's marketplace manifest; an adopter following the 0.14.0 instruction added
  a marketplace source that no longer exists.

## [0.41.0] — 2026-09-02

### Added

- Planning artifacts carry `refs:` — a record of the same work in a system AEP does not own, such as
  `jira:DEV-630`. Written as a mapping (`{provider: jira, reference: DEV-630}`) and accepted as the
  shorthand; `aep artifact new --ref`, `aep artifact set --ref/--unref` and
  `aep artifact list --ref <provider>:<key>` are the doors. A ticket id kept in a body paragraph is
  invisible to every verb that knows what an artifact is, which is what this closes.
- `.engineering/project.yaml` takes a `providers:` map — one URL pattern per external system, each
  carrying `{key}`. `aep artifact show` renders the link beside the reference; a provider with no
  pattern renders as text rather than as a guessed URL. The pattern is configured once instead of
  copied into every artifact, so a tracker migration is one edit.
- `aep reverse tickets` joins the tracker keys a repository's history mentions to the plan: it
  reports the references already held, proposes an `artifact set --ref` line for every key an
  artifact spells in its own prose without recording, and lists the keys the history carries that no
  artifact names. It writes nothing — the join is a text match, which is reason enough to show a
  command and not to run one.
- `aep artifact list --format json` and `board --format json` carry `refs` on every entry, always
  written and `[]` when empty, so a board can be joined to a tracker without parsing prose.

## [0.40.3] — 2026-09-01

### Fixed

- Complete the canonical-command sweep across AEP's own profiles, driver prompts, planning
  templates, examples, and public documentation. The `protocol` executable remains the exact
  compatibility alias; newly authored operational guidance now consistently invokes `aep`.

## [0.40.2] — 2026-09-01

### Changed

- Public guides and command references now use canonical `aep` examples throughout while retaining
  `protocol` as the exact output-and-exit compatibility alias.
- The optional ESS adapter remains a closed transcription of `ess-conformance-report/1`; ESS owns
  the report and AEP alone chooses how to translate it into `ess_conformance` evidence.
- Entity Runtime is pinned once at `0.17.5`, the sanitized release carrying the completed AEP
  rename in its engineering record.

## [0.40.1] — 2026-09-01

### Fixed

- Repinned Entity Runtime to its sanitized `0.17.4` release so fresh AEP builds resolve the public
  provider history after the coordinated repository privacy migration.

## [0.40.0] — 2026-09-01

### Added

- `aep` is now the canonical command, with `protocol` retained as an exact stdout, stderr and exit
  status compatibility alias.
- The optional `aep-ess-evidence` crate converts the closed standalone ESS conformance report into
  AEP `ess_conformance` evidence without linking AEP to ESS modeling types.
- `release.source` and `release.dependency-chain` profiles now govern source release verification
  and provider-first dependency promotion. Their workflows require default-branch provenance,
  repository gates, review, convergence evidence, and explicit human approval before any
  post-merge release is created.
- Public harness-integration documentation now gives the local `protocol drive` installation and
  run sequence, the free native comparison command, and a feature/confinement matrix for Claude
  Code, Codex, and the b10x native loop. It distinguishes an inner harness transcript, an outer
  metaharness attestation, and substrate measurements, and states where Codex is not yet a drive
  selector rather than turning absence into support.

### Changed

- Workspace dependencies were refreshed to their current compatible releases. The direct
  cryptographic and JSON Schema dependencies move to `sha2 0.11` and `jsonschema 0.52`; digest
  rendering now formats bytes explicitly instead of relying on the removed `LowerHex` wrapper.

## [0.39.2] — 2026-08-31

### Fixed

- CI now installs Go 1.25.10, the exact minimum declared by the pinned go-task 3.52.0 release,
  instead of refusing the task runner under Go 1.21 with automatic toolchain upgrades disabled.

## [0.39.1] — 2026-08-31

### Fixed

- CI and Pages now use the Node 24 generations of GitHub's checkout, Go and Node setup actions;
  the Go setup explicitly disables its inapplicable root-module cache, removing the missing
  `go.sum` warning from release gates.

## [0.39.0] — 2026-08-31

### Fixed

- Driven vendor steps now compile the step map's ordered `scope:` rules into the sealed
  `metaharness.frame/1` `subjects` field. `allowed` admits file reads, edits and whole-file writes;
  `partial-only` admits reads and targeted edits; `denied` preserves reads while refusing both
  write granularities. File patterns are `file:`-qualified so a write catch-all cannot govern
  process subjects. The original unscoped canonical frame remains byte-identical, while the
  independent contract reader now verifies that scoped frames survive projection and that scope
  changes break the digest.
- The pinned b10x adapter contract result now consumes the five-vector harness 0.9.1 contract,
  including deterministic unpublished-tool and approval-denial enforcement captures.

## [0.38.1] — 2026-08-31

- Expose `aep_project::load_bundle` so deployment tools can validate an immutable definition tree
  and print the exact source-byte digest that `load_pinned_bundle` subsequently verifies.

## [0.38.0] — 2026-08-31

### Added

- `aep-client` now publishes the service route catalog and derives JSON Schema from every HTTP
  request, success and problem document, so servers and documentation can project one
  machine-readable contract without restating paths or payload shapes.

## [0.37.1] — 2026-08-31

### Fixed

- Driven b10x steps now declare metaharness's delivered `observe` decision mode explicitly instead
  of inheriting its vendor-oriented `frame` default, matching the direct-provider adapter's honest
  no-decision-seam contract.

## [0.37.0] — 2026-08-31

### Added

- `protocol drive run` now refuses maps with model steps before allocating a run unless
  `METAHARNESS_LIVE=1`, `--budget-usd` and `--assume-usd-per-run` explicitly authorize them. It
  records exact integer-microdollar reservations before each metaharness spawn, stops with the
  typed `budget-exhausted` run outcome before a launch would cross the cap, and makes resumes
  inherit or narrow—but never raise—the remembered ceiling. Command-only maps remain free.

## [0.36.4] — 2026-08-31

- Kept the `protocol serve` test client's startup pipe open for the server's lifetime, preventing
  the read-only startup notice from racing a closed pipe and resetting the first HTTP request.

## [0.36.3] — 2026-08-31

- Serialized PostgreSQL test-schema cleanup so parallel backend tests no longer race incidental
  migrations in the shared public schema.

## [0.36.2] — 2026-08-31

- Pinned Entity Runtime 0.17.3 so fresh PostgreSQL command sessions can acquire absent-identity
  advisory locks and execute indexed containment queries with valid parameter types.

## [0.36.1] — 2026-08-31

- Fresh PostgreSQL command sessions can now retain a pinned lifecycle registry while using
  provider-reserved identity ranges, so service startup definitions govern both writes and type
  discovery.

## [0.36.0] — 2026-08-31

* Central-service retries now compare canonical caller-controlled command intent, and the query contract exposes bounded paginated history alongside complete version-1 history.
* The official client prefers strict wire-v2 history pages, drains them for complete history, and falls back to byte-compatible version 1 when v2 is not served.
* PostgreSQL offers a fresh command-session backend that locks and hydrates only one command's idempotency record and direct dependencies, then commits accepted or refused evidence in the same outer transaction.
* Startup definition trees can be validated and pinned by a deterministic SHA-256 digest over sorted relative paths and bytes.
* The workspace now pins Entity Runtime 0.17.1 for recorded decisions, indexed document containment, and caller-scoped PostgreSQL sessions.

## [0.35.0] — 2026-08-31

### Added

- `aep-client` now publishes the strict `application/vnd.aep.service+json;version=1` wire, a
  runtime-neutral official client that implements `CommandService` and `QueryService` directly,
  injected transport and credential ports, and credential-free constructed exchanges shared with
  `aep-service`. Request documents omit actor, executor, request identity and time; nullable members
  are mandatory, unknown members are refused, no-response failures map locally to `Unavailable`,
  and bearer credentials are redacted from diagnostics.

### Fixed

- `protocol serve` now explicitly completes the response side of each loopback connection after
  writing its framed HTTP answer, preventing an abortive socket close from discarding that answer.
- Memory-backed commands now publish their candidate state only after the complete command succeeds,
  so a refused multi-effect command leaves no entity, relation, history, event, revision or
  idempotency mutation and records exactly one rejection audit.
- Durable contract commands now commit every entity, event, relation, audit and idempotency record
  through one `entity-runtime` 0.14.0 atomic batch using the pre-command optimistic view. Failed or
  stale batches publish no local prefix; markdown recovers a persisted batch intent before reads,
  and hybrid contract access goes through its declared authority.
- Entity, relation and audit queries now validate and apply their opaque `offset-<n>` cursors after
  deterministic filtering and ordering, advance by the number returned, and stop emitting a cursor
  when no matches remain instead of repeating the first page.

* **Eval records cannot claim outcomes without naming and sealing their inputs.** The matrix reader
  now refuses missing, null, empty, or malformed specification ids, specification digests,
  transcript digests, and expectation rows with stable `EVAL-RECORD-*` codes. Expectations must be
  non-empty and both digests are checked on every record; an absent or null verdict remains
  unobservable.

* **A resumed driven run cannot observe a mixed state or silently repeat an uncertain effect.**
  Snapshot and cursor now publish together through a hash-verified immutable generation; complete
  legacy pairs migrate before execution and partial or tampered pairs are refused. Every outside
  attempt is persisted before dispatch and a resume requires its exact id through
  `--retry-in-flight`, or an explicit `--record-in-flight-no-verdict`; dependency circuit counts
  survive resume as part of the cursor.

### Changed

* **Validated ESS specifications and compiled IR are sealed.** `Specification` and `EssIr` expose
  read-only accessors instead of public fields, so raw files and the compiler are now the only
  construction entrances. Compilation re-runs complete validation before resolution, downstream
  projections consume the accessor surface, and provenance refuses a serialization failure instead
  of hashing empty bytes. Tests that forged invalid validated or resolved records were replaced by
  raw-document fixtures and a boundary regression.

* Project discovery, schema parsing and Git acquisition now live in the `aep-project` edge crate;
  `aep-engine` retains only the semantic registry and no longer depends on schema or driver types.
  Pinned Git sources use a bare object cache plus read-only, hash-manifested snapshots whose full
  membership, modes and bytes are revalidated before reuse.

* CI now provisions the declared Rust, MSRV, Go, Node, WebAssembly, PostgreSQL and go-task
  environment and invokes `task check` once; release verification reuses that workflow. `cargo
  xtask status --check` holds the delegation and every generated volatile status region, while
  reader-facing current-state prose links to annotated tags, the planning store and gap register.

## [0.34.0] — 2026-08-31

### Added

* **`protocol serve` puts the plan in a browser, and moves it from there.** A board is a shape and
  a terminal prints lines, so triage was the thing the CLI was worst at. The page shows the status
  columns `artifact board` prints, one artifact's fields and body as `artifact show` prints them, and
  the rungs it may take next with what each costs — a rung whose price is unmet is drawn *with the
  price on it*, so a cost is read rather than learnt by being refused. Clicking a rung moves the
  artifact through the same decision `artifact move` makes, graph re-validation included, and a
  refusal comes back with every status the ladder *would* have permitted, each one a button.

  **It binds `127.0.0.1` and there is no flag that widens it.** Reaching it from another machine is
  `ssh -L`, which puts authentication where authentication belongs. The URL it prints carries a token
  for the run and a request without that token is refused, which is what stops another page in the
  same browser writing to your store — that is not authentication, and the module doc says so.
  `--read-only` answers reads and refuses every transition by name.

  **No new dependency**: the HTTP is about five hundred lines over `std::net::TcpListener`, and the
  refusal is recorded where the refusal happens. A framework would have meant the first async runtime
  in a workspace that wrote a busy-polling `block_on` to avoid one, and an MSRV that moves through the
  lockfile without a commit of ours touching a line of Rust. A backend is opened per request, exactly
  as the CLI opens one per invocation, so the page and a terminal can be used at the same time.

* **The Claude Code plugin says which version it is, and a gate step holds it to that.**
  `integrations/claude-code/.claude-plugin/plugin.json` is `0.2.0`; it read `0.1.0` through nine
  plugin commits and 968 changed lines on 2026-08-29/30, so a session could not tell the skill it
  had loaded from the one on disk — and on 2026-08-30 a coordinator paid for it: `3d86d5b`
  changed `skills/wave/SKILL.md` eighteen minutes into a running wave, the loaded copy lacked the
  section the coordinator needed, `/reload-skills` said nothing had changed, and the wave halted at
  the merge boundary after five commits. Every `SKILL.md` now opens with `**Skill version 0.2.0**`
  and the wave skill's stage-1 proposal quotes it. `plugin-check` (`cargo xtask plugin`), the
  twelfth of the gate's twenty-two steps, fails when anything under `integrations/claude-code/`
  differs from the newest release tag and the manifest's `version` does not, when a skill does not
  state that version, or when the new `integrations/claude-code/roster.json` disagrees with the
  skills and agents on disk. The roster is a sibling of the manifest rather than keys inside it,
  because Claude Code reads `skills` and `agents` in `plugin.json` as path overrides that replace
  the directory scan. It is what an adopter that pins the plugin's agent list reads — harness
  pinned it by hand and went red when two agents shipped.
  `story:plugin-and-skills-carry-a-version`, `story:plugin-roster-is-data`.

* **`task release-check` says whether the newest release was cut completely, and `task install`
  ends the procedure.** Five lines: the workspace version and the `CHANGELOG.md` heading match the
  tag, the tag is on `origin`, a GitHub Release exists, and the planning store holds a
  `test_result` naming the tag's commit. Run against `0.33.0` the day it shipped it prints four
  `ok` and one `MISSING` — no gate record names `304d198`. It reaches the network, so it is not a
  step of `task check`. The word *release* had been undefined for long enough that a coordinator
  conflated it with a merge to `main` and the operator had to spell it out (2026-08-30:
  *changelog update, commit, new tag, gh release*); `AGENTS.md` § *Releases* now ends its loop with
  `task install` and `task release-check`, because the ambient `protocol` was older than the store
  it read on six recorded occasions and each time answered `validate` or `history` wrongly with no
  line saying why. `story:release-is-a-checklist-with-a-check`.

* **`AGENTS.md` § *Gate* carries a generated step list.** `cargo xtask status` now owns a region
  there — the count and the ordered names, read from `Taskfile.yml`'s `check:` block exactly as the
  website's currency line already was — and `status-check` fails when it drifts. The hand-written
  count was wrong twice in three days: *eighteen* against *Nineteen* inside one file on 2026-08-28,
  *Twenty* against a Taskfile with twenty-one on 2026-08-30.
  `story:agents-md-is-generated-where-it-counts`.

* **A specification can now say how a run's usage *moved*, not only what it totalled.** Two
  expectation kinds, taking `trace-spec/1` to fifty-three: `usage.trend` states which way a named
  per-request usage field went across the run's API requests
  (`{usage.trend: {field: cache_read_input_tokens, trend: non_decreasing}}`), and `usage.share`
  bounds any one request's share of the run's own total for that field
  (`{usage.share: {field: cache_creation_input_tokens, at_most: 0.6}}`). This is what catches a
  context strategy that has quietly stopped working: `cache.read_tokens` stays healthy while the
  ramp goes flat.

  `usage.trend` is three-valued and the third value is the point. A pair of requests that moved
  the **wrong** way is a `gap`, naming the first one. A run that moved the right way somewhere and
  the wrong way nowhere is `ok`. A run that **never moved at all** is `unk` — a run of repeated
  values is consistent with `non_decreasing` and `non_increasing` at once, so passing it would
  publish a verdict that holds whichever direction you wrote. Two runs with an identical
  cache-read total, a ramp and a dead-flat line, are an `ok` and an `unk` where the totals cannot
  tell them apart at all. Write `on_unknown: gap` to make the flat run fail. A run of **one**
  request satisfies a trend vacuously.

  The `ok`/`unk` line is drawn over the **series** and not over a pair, because `unk` means *this
  run is consistent with both directions* and only a run that never moved is. A single flat stretch
  inside a run that moved elsewhere is an `ok`: `cache_creation_input_tokens` of `20168, 0, 0` is
  front-loaded and `non_increasing` says so, even though its last pair stood still — the opposite
  direction gaps on the same series, so there is nothing left to be undecided about.

  Four fields are readable — `input_tokens`, `output_tokens`, `cache_read_input_tokens`,
  `cache_creation_input_tokens` — and a name outside that list is refused when the document is
  read rather than reported `unk` for ever. So is a share bound the share cannot be on both sides
  of: it is `peak / total` with a zero total refused first, so it is above 0 and at most 1, and
  `at_most: 1.5`, `at_least: -0.5`, `at_most: 1`, `at_least: 0` and `at_most: 0` are all
  expectations whose verdict does not depend on the run. `at_least: 1` is the one bound at an end
  that survives — it says one request took the whole total, and a run can fail it. A share's denominator is the series' own sum and never the terminal
  record's aggregate, so a reader can add the cited requests up. A run that made no API request
  reports `unk` with the reason *this transcript records no API request*, and so does a run where
  any request is missing the field — named `requests[].<field>`, so it reads apart from the
  run-wide kinds' `usage.<field>`. No statistics: no means, no percentiles, no smoothing.
  `story:usage-series-assertions`.

* **The Claude Code plugin ships a `wave` skill**, and the branch convention it follows is written
  down for the first time. A wave — N stories implemented at once, each on its own `impl/<slug>`
  branch off one base, merged serially onto `wave/<name>`, gated **once** on the merged result and
  closed by one `test_result` against that merge — has been this repository's practice since
  `5b6259b` and existed in `git log` and nowhere else. It is now `AGENTS.md` § *Branches and waves*,
  with the long form in the skill's own reference. The skill runs in two stages with a stop between:
  it proposes and halts, because a sub-agent cannot ask an operator anything and the approval has to
  live where a stop is possible. It evaluates no gate — green is `task check`'s exit status — and it
  owns every `protocol artifact` call itself, because the planning journal is append-only, committed
  and unmergeable, so N branches each moving their own story would merge into a document whose
  revision no event supports. Measured against this store while writing it: 36 of 40 draft stories
  are dependency-ready, so selection is judgement rather than a query; 24 of 40 cite no crate path,
  so the skill reports *blast radius unknown* rather than assuming disjointness; and 1 of 40 carries
  a `serves:` edge, so the wave adds them as part of the proposal.
  `story:wave-as-a-surface`.

* **A `story-scoper` agent, and the `wave` skill's cycle.** The scoper works out where one story
  would actually land — crate, files, symbols, documents — and returns a `## Scope` section marking
  every line **cited** or **inferred**, because a scope that mixes what was read with what was
  guessed is worse than none: it gets trusted exactly where it is weakest. It is **read-only**, which
  is what makes running one per story at once safe — the planning journal is append-only and one
  file, so N agents writing it would race; they return sections and the caller writes them. Selection
  now scopes candidates before choosing between them, and a story whose scope cannot be established
  is treated as **unassessed rather than safe**. The skill also states the cycle a wave is one turn
  of — `replan → dispatch → integrate → release → replan` — and that **`release` may not be
  automated**: it is a written procedure nothing enforces, and it has already slipped once here when
  a gate piped into `tail` reported `tail`'s exit status and two commits were pushed claiming a gate
  that never ran past its first step. `story:a-story-records-where-it-lands`.

  **Measured while dry-running it, and it corrects a fear rather than confirming one.** A
  package-scoped build in a fresh worktree costs **1.6 G and 30 s**, not the 28 G a full workspace
  target costs — so five parallel worktrees cost ~8 G, and what bounds a wave is how many candidates
  are ready, not disk. Scoping four candidates concurrently took about four minutes and changed the
  selection: one unit was dropped as under-specified (its acceptance names a declaration site that
  does not exist in the tree, and a new workflow key needs a Rust field because `RawState` is
  `deny_unknown_fields`), one was found to edit `README.md` through a hard-coded count that appears
  in six files, and one candidate turned out to have largely shipped four days earlier with the
  artifact never moved. `docs/plan/wave-reading-what-is-recorded.md` § 2a keeps what the unscoped
  first pass got wrong, because that is the argument for scoping before selecting.

* **`AGENTS.md`, `README.md` and the plugin README now say what `integrations/` is for.** The
  specification is the product; a plugin surface demonstrates that one of its rules can be carried
  by a real harness, and sometimes finds the shape of a rule not yet written down so that it can be.
  A harness that drives these workflows deterministically reads the documents, not the surfaces. The
  question to ask of anything added under `integrations/` is *which rule does this demonstrate* —
  never *is this the product*. Recorded because it answers, in one place, the reading that
  `README.md`'s refusal of "an LLM orchestration framework" otherwise leaves open every time a
  surface is added.

* **The Claude Code plugin ships an `implementor` and an `adversary` agent**, taking two of the four
  states the `no-implementor-or-verifier-agent` gap held — `establish_verifiers` and `implement` to
  the first, `adversarial_verify` to the second. The gap's two arguments are answered rather than
  dropped, and the narrowed row (`no-verify-surface`, over `verify` alone) carries the answers.
  *An implementor has nothing left to learn because `protocol drive` puts the capability policy in
  force per state* was true of driven runs only; an interactive session has no drive loop, and
  `implementor.md` teaches there what the `test.exists` guard mechanises elsewhere — the case is
  written and **watched fail** before the code exists. *A verifier agent would emit evidence that
  looks independent and is not* is also true, and is why `adversary.md` is **not a verifier and
  emits no evidence**: its cases are run by the test runner, whose record the engine already reads
  as independent, and its judgement findings become a `review-result` from an agent, which satisfies
  no `human: true` review requirement and gates nothing. `independent: true` is unchanged and gap
  register **D-3** stays proposed — the verifier agent that does need a signed record stays
  unbuilt. The adversary's bound is that it may not edit what it attacks; its file states that in a
  session this is an instruction rather than a mechanism, and its report leads with `git diff
  --stat` so a reader can check the bound held. `story:implementor-and-adversary-agents`.

* **`task docs-check` is a step of the gate**, eleventh of twenty-one: every verb the CLI answers
  must have an entry in `website/docs/reference/cli.md`, and the step fails naming each one that
  does not. The verb list comes from `clap` rather than from parsing `--help`, so it is the tree the
  binary dispatches on and a rendering change cannot move it. Seven of seventy-seven verbs had no
  entry when this landed — `protocol workspace list|crossings|show|members`, `protocol artifact
  divergences|catch-up` and `protocol property evidence` — and the `workspace` family had been
  undocumented for **eight releases**, while the site's own roadmap page told readers it had
  shipped in 0.25.0. All seven now have entries, under three new sections: *Workspace surface*,
  *Property surface*, and a hybrid-store pair on the planning surface.

* **An edge has one spelling, whichever of the two you type.** `protocol artifact relate <id>
  <relation>:<target>` is now accepted beside `relate <id> <relation> <target>`, split at the first
  colon exactly as `new --relate` has always split it. Both issue the same command and leave the
  same journal entry, so nobody reading the store three months later can tell which was typed. The
  reason this cost anything is that the two verbs disagreed: `new --relate serves:vision:O2` created
  the edge and `relate story:… serves:vision:O2` was refused for want of a third positional, in the
  middle of a run where both stories were already `active` and the store went red over a spelling.
  `story:one-spelling-for-an-edge`.

* **Appending a section, rewriting one section and changing a frontmatter field are each one
  command.** `protocol artifact body <id> --from <path|-> --append` adds to the end;
  `--section "<heading>" --from …` replaces the prose under that `##` heading, or adds the section at
  the end when the document has none; `protocol artifact show <id> --body-only` prints the body bytes
  and nothing else, which is what `--from` would write straight back. `protocol artifact set <id>`
  changes `--title`, `--summary`, `--owner`, `--tag` and `--untag` through the same door the prose
  goes through. Every one of them is one `update` in the journal.

  This is what five sessions were doing with `python`, heredocs and `cat >>` instead: three ad-hoc
  frontmatter splitters across sub-agents in one run, four duplicate `## Scope` sections in another,
  and an append with a shell redirect that wrote the bytes and skipped the journal entirely. `set`
  accepts `--status`, `--revision`, `--id` and `--kind` and **refuses each by name**, because the
  person who typed `--status` has a question and `unexpected argument` answers none of it: a status
  is a decision `move` takes against a lifecycle, a revision is the store's own count of its writes,
  and an id and a kind are identity, fixed at `new`. `story:body-edits-have-a-verb`.

* **`protocol artifact validate --strict` fails on what `validate` reports and does not fail on.**
  Three classes — a status reached on somebody's assertion rather than on a record, a document that
  predates the event log, drift between a document and what the log says was written to it — are
  legal in a store people are working in, and refusing an assertion outright would stop anybody
  closing a story on the day a runner is down. A **gate** has every reason to refuse all three.
  Without the flag the lines and the exit code are exactly what they were; with it the same lines are
  printed, one more names which class decided, and the exit is 1. `plan-check` is deliberately not
  flipped: the story that flips it is `story:completion-needs-evidence`.
  `story:validate-strict-refuses-what-it-reports`.

* **`protocol artifact move` refuses a move that would leave the store invalid**, in the finding's
  own words. It runs the graph rules on the store the move *would* leave and compares that to what
  the store already reports, so a finding it was already carrying does not block an unrelated move,
  and a new one is printed with its own text and its own hint. What this stops is one command making
  work for the next: `move --to active` succeeded and the very next `validate` answered
  `[empty_declaration] … is active and serves no objective`, because grounding ran only in
  `validate`. Nothing is written when it refuses. `story:validate-strict-refuses-what-it-reports`.

* **`protocol artifact body --from` on an empty body is refused, naming the flag.** A pipe that
  produced nothing used to write the empty string over the prose and bump the revision, so the store
  held a document with nothing in it and a record saying somebody meant that. There is no undo in a
  tool that has not committed anything. `story:validate-strict-refuses-what-it-reports`.

* **`protocol artifact move --to <status> --via` walks the ladder to get there**, journalling every
  rung it crosses. `draft -> proposed -> active` was two commands per story on every wave, and one
  session wrote a `python` loop issuing four commands for each of eight stories. It is still N
  entries in the history, because a walk that recorded one hop would be a record saying the story was
  never proposed. It crosses only rungs **nothing guards**: a rung with a `requires:` or a `when:`
  stops the walk — with that rung's own refusal when the evidence is short, and saying the walk is
  what is refused when it is not — because arriving somewhere by not being asked is exactly what an
  evidence gate exists against. `story:cli-ergonomics-round-2`.

* **`protocol artifact explain` ends by saying what each legal next rung costs**: one line per rung,
  `next: <status> needs <n> <kind> record(s); held: <m>`, counted against the records the store holds
  about that artifact. The requirement used to be learnt by being refused by `move`, twice. A rung's
  price is a line in a lifecycle document and there was no reason to be refused to see it. A kind
  whose lineage declares no ladder gets no lines, because *anything is legal* is not an answer.
  `story:refusals-name-what-to-do-next`.

### Changed

* **Thirteen published JSON Schemas now refuse identifiers they used to accept, and bound their
  length.** A published `pattern` and the constructor it is published for were two hand-written
  statements of one rule, so they drifted: `adp/2` was valid to an editor and an error to the
  loader, and the step maps an editor called fine would not load. Every published pattern in
  `aep-domain` and `aep-driver-spec` is now derived from one body per charset — fourteen
  identifiers and references, not the two the defect was reported against, because fixing one of
  two leaves the defect. `maxLength` carries the 200-character bound a regular expression cannot
  express without counted repetition, from the single `aep_domain::ids::MAX_LENGTH` that
  `validate` reads. **A document that loads today still loads**: every change is a tightening
  toward the constructor, verified over 56,594 strings with zero divergences.
  **Three divergences of the same class are left open and pinned by exact count**, so the case
  goes red the day one moves: `FactPattern` puts `*` inside its character class (96, all
  schema-looser; the three shipped protocols' 46 observables all agree today), `EntityType` and
  `DomainEventType` accept an empty or bare-hyphen segment (66 and 49; neither type appears in any
  generated schema), and `ArtifactId` refuses a leading separator its constructor takes (601, all
  schema-stricter).

* **Three shipped skill documents no longer suggest editing a planning artifact by hand.**
  `protocol artifact` is the store's only writer.
  `skills/planning/references/store-conventions.md` said correcting a title by hand is harmless; it
  now says the title is set by `protocol artifact new --title` and no verb changes one afterwards,
  which was checked against `replace_body`. Both planning skills said *put that evidence in the
  artifact* and now name `protocol artifact body`.
  **A source scan that read shipped prose and failed the build on it was written for this and
  removed before release.** The store already catches the defect where it lands rather than where
  it is described: a document edited outside a command has a revision no event supports, and
  `protocol artifact validate` — which the gate runs as `plan-check` — reports it and exits 1. The
  prose scan was 2,153 lines classifying English and went red on ordinary edits to skill text, and
  a check that fails when the prose it polices changes is one somebody switches off.

* **The wave skill absorbs the two retros written on 2026-08-30**, one from harness wave 3
  (`docs/reviews/2026-08-30-wave-3-retro.md`) and one from substrate's byte-plane wave — eighteen
  changes, none of which had reached `skills/wave/SKILL.md` when the retros were filed. The wave
  page is written whether or not the operator pre-approved, and it is a file stage 2 updates: per
  unit the branch, head, worktree, build directory, scratch directory and stage — because a
  session that compacted twice mid-wave had the wave's whole state in a context window and nowhere
  else. A unit's record is a triple (worktree, build directory inside it, scratch directory the
  coordinator assigns) and teardown removes all three; the brief is a file
  (`references/unit-brief.md`) so 240 lines of invariants are not retyped across six dispatches
  with nothing to catch an omission. The close step writes each unit's learned `## Scope` back
  through `protocol artifact body`, so the next wave selects on what was measured rather than on
  the two-of-five inferred lines that were wrong. Sub-agent reports open with a six-line header
  the coordinator routes on. The attack budget records findings per pass and who verifies the last
  correction; free disk is re-read as each unit returns (84 G became 62 G inside one wave); the
  closing report records per-agent tokens, tool uses and duration (one wave spent 2,025,848
  sub-agent tokens with nothing merged and no number to plan the next one from). A wave of one is
  a wave. Teardown derives its branch glob from `references/branch-and-merge.md` instead of the
  `wt/*` it had hard-coded, which had left eight merged branches standing here; that reference now
  puts the build directory *inside* the worktree, as `AGENTS.md` does and as one gate run was
  spent discovering; shows the `git merge -F <file>` form because `-F -` does not read stdin;
  uses `A...B` for a unit's diff; and names `git merge-tree --write-tree --merge-base=<base> <a>
  <b>` as the dry run before the first merge when two units had to split one file by function. A
  429 mid-wave resumes the unit from its branch head and never re-dispatches a branch with commits;
  N defaults to 4. `story:wave-skill-applies-the-two-retros`, `story:fan-out-survives-a-rate-limit`,
  `story:one-session-per-checkout`.

* **The adversary names whose defect it is, and attacks the contract the unit wrote first.** Its
  findings table has an `origin` column — `introduced` / `pre-existing` / `undecided` — that two
  adversaries in one wave had each invented in prose ("CONFIRMED (out of scope of this change)");
  the coordinator's routing reads it. The attack surface opens with any vector, fixture, schema or
  contract document the unit itself added, because a unit that ships `state: cancelled` in a
  vector and code returning `exited` passes every gate step when both halves are its own. Mutation
  to probe a dead guard is permitted on a scratch copy or inside an added test and nowhere else,
  and the report leads with `git diff --stat` to prove the tree was restored. A red case that
  constructs a state nobody reaches is `INFEASIBLE`, not `CONFIRMED`. The README row that still
  said the adversary records a `review-result` now matches the charter, which has run no
  `protocol artifact` command since `5a04cb8`. `story:adversary-names-whose-defect-it-is`.

* **The implementor reports how many cases ran, and answers the class rather than the instance.**
  Per test lane, `executed <before> → <after>, exit <code>`, from the runner's own summary line —
  a green exit with an unchanged count where cases were added is the first line of the report,
  because a lane that selected host cases by substring ran 8 of 58 and was green for weeks. A
  correction names the class the finding is an instance of and shows the rest of the class is
  clean; where the class is machine-checkable the fix is the check. The build directory stays
  inside the worktree, a `## Scope` mechanism claim is a hypothesis to measure before building on
  it (one was followed as written: five red of five), and a file the unit does not own comes back
  as a patch under `needs-coordinator: yes` rather than as an edit.
  `story:implementor-counts-what-ran`.

* **The planning skill says what the CLI does, in six places it did not.** § 4 no longer calls a
  status move a *proposal until confirmed* — a move is made when the store holds the rung's
  evidence and the agent asks only when it is missing, naming what — after the operator rejected
  the old behaviour four times ("is it implemented or not? if yes, why do you ask me?"). The
  discovery table lists the read verbs `protocol artifact --help` answers with, not seven of
  eighteen; `protocol artifact evidence` is named, with the fact that its kind list is closed and
  that `move` finds a record without being told; every example edge is `decomposes:`, which is what
  130 of 130 stories in this store use, not `derived_from`; *correcting a typo by hand is
  harmless* is gone from `references/store-conventions.md`; and guardrail 6 branches on whether
  `protocol artifact kinds` and `lifecycle <type>-blocker` answer, instead of assuming a blocker
  kind exists. The skill's `description` triggers on adoption — a repository with no
  `.engineering/`, migrating off the `track` plugin — so § 5 is in context before a first store is
  hand-populated, as one was, three minutes ahead of the skill.
  `story:planning-skill-says-what-the-cli-does`, `story:skill-loads-before-the-store-is-touched`.

* **`protocol artifact kinds` lists what can be created, not what this binary shipped knowing.**
  After the compiled vocabulary it now lists every kind the document tree declares a lifecycle for,
  and one row for the open `<type>-blocker` family, which no list can enumerate because the type is
  whatever would clear the blockage. `kinds | grep -i block` returned nothing at all while
  `protocol artifact lifecycle third-party-blocker` answered, which made the verb the skill names as
  the authority on what can be created the one verb that could not say.

  **`protocol artifact blocked` says when the store has no blocker kind at all.** It used to answer
  `nothing is blocked` in a store whose pin predates the blocker ladder — good news about a mechanism
  that did not exist there — and now says so and points at `kinds`.
  `story:blocker-kinds-are-discoverable`.

* **`--title` and `--summary` take a value beginning with `-`**, on `new` and on `set`. A title is
  prose and prose starts with a dash often enough; the workaround, `--summary=…`, is a thing you have
  to already know. And **`relations` is always a list in `list --format json` and `show --format
  json`**, `[]` where the artifact has no edges, so a documented `jq` shape does not break on the
  first artifact without one. `story:cli-ergonomics-round-2`.

* **An evidence-kind refusal ends with the two kinds people actually reach for**: an observation of a
  running system is a `health_observation`, and a dependency on another store's artifact is an
  `artifact`. One session wrote `measurement`, another wrote `cross_repo_dependency`; both kinds
  exist and neither guess is findable by scanning a list of fifteen for a word you do not have.
  `story:refusals-name-what-to-do-next`.

* **The website's claims about this repository are generated from the tag, not typed.**
  `cargo xtask status` already owned `docs/status.md`'s delivered-waves table; it now owns three
  more regions, and `status-check` — the second step of the gate — fails when any of them drifts.
  The landing page's release chip read `0.7.1-infra-waves-1-4`, **twenty-six tags** after that was
  true, and it is the first version number a visitor sees; `where-this-stands.md` said *as of the
  tag `0.32.1`* and named twenty gate steps. All three are now derived — the tag and its date from
  the newest tag reachable from `HEAD`, the step list from `Taskfile.yml`'s own `check:` block —
  so a release cut without them fails the gate at the tag, and **no step is added to the release
  procedure**: `AGENTS.md` § *Releases* already runs `cargo xtask status` after the tag exists.
  The page under `website/docs/` carries an MDX comment where `docs/status.md` carries an HTML one,
  because Docusaurus compiles it as MDX 3 and MDX 3 refuses `<!--`.
  **One candidate was examined and left alone**: `limitations.md`'s *"two durable backends, as of
  `0.27.0`"* is a claim about **when** something became true, not a currency stamp, and generating
  over it would have made it false.

### Fixed

* **A lifecycle the pinned kernel cannot hold is a refusal, not a panic.** `protocol artifact move`
  against a ladder naming a condition operator the pinned `entity-core` did not know printed a Rust
  backtrace — `panicked at crates/aep-backend-markdown/src/kernel.rs:226:33: … unknown condition
  operator 'after'` (2026-08-25) — where a person needed one sentence. `definition_for` now
  returns the kernel's refusal instead of unwrapping it, `decide` answers a fourth verdict,
  `Undecidable`, and the move is refused as *the move to `<status>` cannot be decided: … raise the
  `entity-runtime` pin, or change the lifecycle document to what this kernel reads*. A definition
  the kernel reads and then refuses to register — which used to answer `NotOnTheLadder` and list
  legal moves on a ladder the kernel had just refused — takes the same door, and `describe`
  returns no descriptor for it rather than an invented one. `story:refusals-name-what-to-do-next`.

* **An evidence-gate refusal says what to type, not where the rule looked.** *reaching implemented
  needs at least 1 test_result record(s). Nothing was presented at `$args.evidence.test_result`*
  leaked the kernel's own address into two adopters' terminals; it now reads *… no test_result
  record is held for this artifact — `protocol artifact evidence <id> --kind test_result --source
  <where it came from>` records one*, and an undated rung names `--at <iso8601>`. An address the
  translator has not heard of is printed as it came rather than guessed at.
  `story:refusals-name-what-to-do-next`.

* **The landing page's status panel said three things that were not true.** It published
  *"106 suites and 1811 tests"* — a hand-written count, which this repository forbids on every other
  surface because four of them drifted apart in its first 48 hours, and which `task check` now
  measures at 192 suites and 2890 tests. It dated that measurement to a tag twenty-six releases old.
  And it told a first-time reader *"There is no durable backend"*, which stopped being true at
  `0.27.0` — `website/docs/status/limitations.md` has said so, two clicks away, ever since. The
  counts are gone rather than updated, the tag comes from the generated chip, and the backend
  sentence is replaced by the limit that does still hold. The panel is called `HonestStatus`.

## [0.33.0] — 2026-08-30

### Added

* **`regex:` matchers work in a `trace-spec/1` document; they used to be refused by name.** Write
  `args: {command: {regex: "(&&|\\|\\||;)\\s*\\w"}}` and the checker runs it, with the same
  three-valued reading as every other matcher — an argument the call does not carry is still not a
  match, and a field a transcript does not record is still `unk` rather than a failure. **A `regex:`
  searches the field, where a `glob:` has to be the whole of it**, so anchoring is `^` and `$` and
  yours to write; and `.`, `+`, `|` and `(` are metacharacters in a pattern and literals in a glob.
  Nothing you have already written changed meaning: a `glob:` is still anchored, still literal, and
  still digests to the same value, so no specification was renamed by this. `TRACE-SPEC-008` has
  not gone away — it still means *this matcher cannot be run* and now fires on a pattern the engine
  will not compile, at validation, quoting the engine's complaint, before a transcript is opened.
  A pattern that matches every text there is (`{regex: ".*"}`) is refused under `text.matches` for
  the same reason `{glob: "*"}` always was; `{regex: "^$"}` — *the final message is empty* — is
  not, because a run can fail it.

* **Parked on a credential no longer reads as actively worked.** A blocker is typed by *what would
  clear it* and the type is the kind — `credential-blocker`, `decision-blocker`,
  `third-party-blocker` — so it costs a name and no release. `protocol artifact list` and
  `protocol artifact board` mark an artifact a blocker still stops with `blocked: <type>`, so a
  parked item and a moving one stop looking alike without anybody opening a file, and
  `--format json` carries a `blocked_by` list on every row. **New verb**
  `protocol artifact blocked [--type <type>]` answers the question a backlog cannot: what is
  stopped, by what type, and on which single item — grouped by the blocker, so five stories waiting
  on one decision arrive as one row with five lines under it rather than as five separate
  conversations. A blocker stops blocking when it reaches the end of its own ladder
  (`protocol artifact move <blocker> --to cleared`), which leaves the record in the journal instead
  of editing away the fact that anything was ever stuck. The six starting types — `decision`,
  `review`, `credential`, `third-party`, `capacity`, `deploy` — are written down in
  `artifacts/kinds/blocker.yaml` with what clears each, and the list is **open**: a
  `procurement-blocker` of your own works with nothing added to it.

* **A blocker can say which evidence it is withholding, and `explain` names it.** A new optional
  `withholds:` key on a planning document takes an evidence kind — `protocol artifact new
  credential-blocker ci-token --withholds test_result --relate blocks:story:x` — and records *why a
  required fact does not exist*: the gate wants a `test_result`, the job that would produce one
  cannot mint a read-scope token. `protocol artifact explain story:x` prints
  `blocked by credential-blocker:ci-token (credential), withholding test_result` above the status
  history, so the audit question *why is there no record* is answered out of the store.
  `protocol artifact validate` refuses a `withholds` on an artifact that blocks nothing
  (`missing_declaration`) and a spelling outside the evidence vocabulary
  (`undeclared_evidence_kind`).

* **A story says which objective it serves, and `validate` holds it to that.** A fourteenth
  relation, `serves`, points at an objective — a `vision` artifact — and nothing else (`validate`
  refuses a `serves` into any other kind). Where a store declares at least one `vision`, every
  `proposed`, `approved` or `active` story or task must `serves` one, and `protocol artifact
  validate` names each that does not; `draft`, `implemented` and `archived` are exempt, so nothing
  agreed before objectives existed is rewritten and nothing nobody has agreed to is charged. A
  store with no `vision` artifact has declared nothing to serve and is untouched by the rule. The
  objectives themselves are atlas `ROADMAP.md`'s `O1`–`O6`, held here as `vision:O1`…`vision:O6`
  (atlas ADR 0005). `protocol artifact relate <id> serves vision:<objective>` is the line.

* **`protocol drive transition` — the governor a native flow consults at a section boundary.**
  `b10x-harness workflow run` walks a flow `protocol workflow flow` projected from a workflow, and
  that projection is an ordering, not a government: no guard travels. The loop asks a `transition`
  hook before a section is entered and after it leaves; this verb answers it from the engine —
  `evaluate` for `enter`, `transition` on a copy of the execution for `leave` — so a native walk is
  governed by the same documents that govern a driven run, with no crate dependency in either
  direction (harness design 0003 § 7, E2; atlas ADR 0004). Reads the loop's document on stdin;
  exit `0` proceeds, `2` refuses with `{"reason": …}` in the engine's words, anything else is a
  verb that could not answer, which the loop reads fail closed. `--run <id>` positions the engine
  on that run's snapshot over the store as it is now; without it, on the state the flow path
  names — a step node after its state, a retreat group `<first>-to-<last>` at its first state on
  `enter` and its last on `leave`. The root is the flow's own container: entering it proceeds,
  leaving it is the engine's answer from a run's cursor and proceeds without one (the first paid
  native walk was refused at `enter root` and ran nothing; fixed the same night). A section that
  came out failed is left alone. Decides only: writes nothing, takes no lock, and a consultation
  leaves a run's cursor byte-identical.

* **A stolen lock is in the taking run's record, and a lock refusal says what the holder is
  doing.** `protocol drive run --take-lock` and `protocol drive resume --take-lock` now write the
  lock they superseded — the holder's run id, pid and host — into the taking run's own cursor as
  `took_lock_from`, so `protocol drive status` prints *this run took the lock from pid 4711 of run
  `AUTH-142/2`* out of the record rather than out of a terminal's scrollback. It was built and
  printed before, and never persisted: the cursor had a printer for a field that was always empty.
  The theft is written on the way **in**, so a run that supersedes a lock and then blocks, breaks
  its store or spends its budget without executing a step still leaves it in the record; a later
  resume over a free lock never clears it, and a resume that supersedes again records the newer
  theft over the older. The refusal a live lock answers with now names the holder's **state**,
  read from the holder's own `cursor.json` and never from `lock.json` — a lock file is written once
  and the state moves after every step — and says `state unknown` when that cursor is missing or
  will not parse, rather than dropping the clause or crashing on somebody else's file. Beside it,
  tested rather than asserted: a resume against a lock another live run holds is refused and
  writes nothing; a lock naming another host is refused at the binary and `--take-lock` does not
  pass it; an approval pause is the lock released with the `current` pointer kept. The story is
  `story:operator-resume-ux`, and the change is the product of this repository's own governed run
  `W4-3` on 2026-08-29, integrated from the run's workspace after the driver defects that run
  surfaced were fixed under this heading.

* **`protocol artifact validate` now says when a document claims a revision no write produced.**
  A `revision:` higher than anything the event log records for that document is reported as its own
  finding — `story:x claims revision 99, and no write produced it: 3 event(s) logged, the highest at
  revision 4 (event …)` — with a `forged` list in `--format json` beside `drift` and `deleted`, and
  exit 1. Until now such a document was reported as ordinary drift, which reads as *somebody edited
  a field* and sends you to the log for a state that was never in it. The test is against the
  highest revision the log **records**, not against the number of events it holds: a store's
  history predates the event log, and an observation is an event at the current revision that
  writes nothing, so a document sitting under more events than its revision is not a forgery.
  **This detects; it does not enforce.** No write path refuses a forged revision and `validate`
  grew no refusal — refusing one means knowing who wrote the document, which is gap register **D-3**
  (attestation by signature) and is still proposed. Unchanged: a document with no events at all is
  still only counted as predating the log (`N document(s) predate the event log`, `pre_provider` in
  JSON), whatever revision it claims.

* **A `command` step can name the task document its run was started from: `{task}`.** It joins
  `{run_directory}` and `{transcript}` as the third and last placeholder a step map may write, and
  the driver expands it to the absolute path of the task the run was launched with — the one
  `protocol drive run --task <file>` named, or the one discovery found when no flag did, and the
  same path again on a resume. Until now a map could not say *this run's task*, so
  `protocol specification evidence` reached its own discovery and bound to the task
  `project.yaml` names: a run driven with `--task <a path that is not the project's>` decided
  another story's specification, or refused over it, while the run's own cursor said something
  else. `drivers/development/default.yaml` now passes `--task {task}`, so a driven run binds
  explicitly rather than by discovery. **If you write step maps:** a misspelled placeholder is
  still refused at load, and the hint now offers all three names.

* **A store write can say who made it, and a driven step says so by itself.** `AEP_ACTOR` declares
  the actor every `protocol artifact new`, `move`, `body`, `relate` and `evidence` is journalled
  with — `human:<name>`, `agent:<name>`, `service:<name>` or `system` — so
  `protocol artifact history` and `protocol artifact explain` can tell an agent's move from a
  person's. Until now every write in a store said `human:$USER` whoever made it, which is why an
  approval recorded by a driven run was indistinguishable in the record from the operator's own. A
  value that does not parse is **refused, naming the variable**, never quietly replaced by
  `$USER`; unset, nothing changes. `protocol drive` sets it to `agent:<execution id>` on every
  process it starts for a step, and that is the same actor an approval is refused from as
  self-approval, so a run cannot approve its own work under the name it writes under. **What it
  does not yet reach:** an `llm` step's model session, whose environment metaharness constructs
  from a fixed allowlist rather than inheriting — that session's own `protocol artifact move` is
  still journalled as `human:$USER`, and closing it needs a flag on the metaharness side
  (`story:the-store-knows-who-wrote-it`).

* **An `operator` step can be answered by a named agent, and the run says who answered.**
  `protocol drive run --pause-on-approval --approver agent:<name>` admits one named non-human
  actor's recorded approval at an `operator` step; a person's approval is admissible without being
  named, on every run, as before. The run still stops at the step — the approver records a granted
  `approval` against the run's snapshot while it is stopped — and the resume reads what arrived:
  the cursor records who answered (`answered …` in `protocol drive status`), an approval by the
  run's own actor (its task, its execution, the harness its `llm` steps run under) is refused as
  self-approval whoever named it, and an approval by an agent nobody named is refused naming the
  flag. With an approver named, a resume that finds no admissible approval stops again and says who
  would be admissible. Naming a person, `system`, a service or the run itself is refused before the
  run starts. Attestation is exactly as strong as the record: the approver is whatever `producer`
  the approval carries, and gap register D-3 (attestation by signature) stays proposed.

* **A run directory records which binary each `command` step actually ran.** `commands.jsonl` holds
  one line per step attempt with the program the map wrote, the program that was spawned and which
  of the two it was; each step's `.log` now opens with the same fact. Substituting a binary
  silently is its own kind of lie, and a reader can now tell a step that used the driver's build
  from one that used something it found.

* **`protocol drive run` and `protocol drive resume` refuse before allocating a run id when a map's
  `command` steps say `protocol` and the driver cannot guarantee they get this build.** It fires
  only where the driver cannot name its own binary and the `protocol` on its `PATH` is a different
  version; the refusal names both versions, and it names the fix correctly —
  `cargo install --root ~/.local` puts the binary where a *session* looks, while a driver-side
  `PATH` is the operator's own shell, so putting `~/.local/bin` first in it is part of the answer.

* **A profile can say *read the public channels, never a direct message*, and one that forgets is
  refused instead of validating clean.** `network.read` is now scoped by **audience**, the way
  `deployment.create` is scoped by environment. `network.read:private` covers a read of
  correspondence addressed to a bounded audience — a direct message, a group DM, a private channel,
  a mailbox, a ticket's internal comment; `network.read:public` covers material published to an
  unbounded one; unscoped `network.read` is still both, and every profile, principle and task that
  writes it means exactly what it meant before. `aep/1` puts `network.read:private` in its approval
  floor, so `allow: [network.read]` on its own no longer resolves: the refusal names the entry that
  was forgotten and the hint says to put it in `deny` or `require_approval`. The line that fixes it
  is `deny: [network.read:private]` beside the grant. **Membership is not the control** — a token
  that *can* read a direct message is exactly the case a denial is for — and a harness that cannot
  tell which audience a read will reach refuses rather than guesses, without having to remember to:
  a request that does not say its audience asks for the wildcard, and `network.read:public` does
  not cover the wildcard. The driver's tool table asks the private question for the same reason, so
  a profile that may not read privately is offered no `WebFetch`/`WebSearch`, which no table can
  promise is public. Nothing here *enforces* the rule at a Slack API — a capability is a declared
  authorisation decision and the actor that honours it is the harness — but a profile that forgot
  the rule is now a validation error rather than a paragraph somebody was asked to follow
  (`story:private-message-denial`, adopter register row `D-I2`).

* **Both harnesses can be pointed at one gateway, which is what makes a harness comparison one.**
  `--claude-endpoint` and `--claude-model` join the existing `--b10x-endpoint`/`--b10x-model`, so a
  `harness: claude-code` step and a `harness: b10x` step can run against the same model. Without
  this a comparison of the two arms measured the two *models* at least as much as the two harnesses,
  and no scorer could separate them afterwards. metaharness requires `--credentials none` alongside
  an endpoint — a child pointed at a foreign endpoint must hold no operator credential — so the
  driver passes it rather than making the caller remember. An endpoint with no model, or a model
  with nowhere to go, is not a gateway and is ignored: metaharness refuses each alone, and passing
  half of one would turn a flag mistake into a launch refusal several states into a paid run.

* **`protocol artifact show <id>` prints one artifact.** Its frontmatter fields — id, kind, status,
  title, summary, owner, tags, relations, revision — and then its markdown body, verbatim. There was
  no verb for an id in hand: `list` prints the whole plan, `board` arranges it, `history` prints the
  event log, `explain` answers what made a status happen, and `body` writes. A driven session in run
  `W4-3/1` typed `show` five times and was answered `unrecognized subcommand` every time, which is
  why the verb is called `show`. An id the plan does not hold is refused naming it. Read through the
  contract, so a markdown, SQLite, Postgres or hybrid plan gives one answer.

* **`task audit-check` is a step of the gate**, fourth of twenty: seven units of
  `.engineering/checks/`, four seconds, re-resolving every `file:line`
  `docs/guide/open-vocabulary.md` cites. A commit that shifts a cited declaration now fails the gate
  instead of leaving a *closed* verdict pointing at a serde attribute — which is what had happened:
  eight doc-comment lines landed above `pub enum EvidenceKind` and the audit's citation rotted
  silently, because the suite that catches exactly this was not in the gate.

### Changed

* **`protocol workflow flow` emits every state as a section.** Each non-terminal state is a group
  named for the state, holding its steps as `<state>-1`, `<state>-2`, … in the map's order — one
  node when the map gave it one step or none — and a retreat is a group of those groups. It was a
  bare node for a state with one step or none, and `b10x-harness workflow run` asks its
  `transition` hook at a group boundary and nowhere else (harness design 0003 § 3): the fifth paid
  native walk of 2026-08-29 was consulted four times, all at `root`, and never about `receive`,
  `specify` or `decompose`. Now `protocol drive transition` is asked on both sides of every state.
  The document's header says so; `b10x-harness workflow plan` shows every state as a section, and
  the harness's committed fixture `adp-default.projected.yaml` is refreshed from this verb. A
  reader keyed on node ids sees `receive-1` where it saw `receive`; the state a node is in has
  always been `run.state`, and still is.

* **Where a driven step may write is decided by the step map, not by the driver.** A driven `llm`
  step's writes — `Write`, `Edit`, `NotebookEdit` on the vendor arm — are now answered from that
  step's own `scope:` in the step map: `denied` refuses any writer, `partial-only` refuses the two
  that replace a whole file and admits a targeted `Edit`, `allowed` admits all three, first
  matching rule wins. The refusal names the rule that matched and the globs the step *may* write,
  so the map is where you go to change it. Until now the same decision was a Rust function
  spelled in one vendor's tool names, which is why the planning store's protection existed for
  `claude` runs and for no other harness; the native arm has been given the identical rules as
  `--write-scope` since 0.29.0, and both arms now read one declaration. **A step map that declares
  no `scope:` restricts nothing** — an undeclared scope is a map that said nothing, not a map that
  said *everything* — so a map driving work that must not rewrite the planning store has to say
  so; both maps shipped here (`drivers/development/default.yaml`, `drivers/development/checks.yaml`)
  now do. What stayed in the driver is the one rule no scope can express: an `Edit` whose
  `old_string` or `new_string` crosses a planning document's closing `---` is still refused,
  because that is a judgement about an edit's text rather than about its path. `protocol drive
  hook`, which the native loop spawns, now answers that content rule alone and leaves whole-file
  writes to the declared scope.

* **The approval floor accepts a broad grant that explicitly denies the floored entry.** Until now
  any outright grant overlapping a floor entry was refused, whatever else the profile said, so
  `allow: [network.read]` with `deny: [network.read:private]` beside it was refused for granting
  something it had just forbidden. The floor now asks its own question — *what does this policy
  decide about the floored capability?* — and a `deny` of it is an answer, because a denial cannot
  be granted back by any later document. An **approval gate on the narrow slice is still not**
  accepted as that answer: the broad grant would keep deciding `Allowed` for every scope the gate
  does not name, and refusing that shape is what the floor is for. A denial of something *else* the
  floor also covers changes nothing — a floor on `deployment.create` for every environment is not
  discharged by denying production.

* **The `unknown capability` refusal now lists the scoped spellings too.** It read
  `… approval.request, deployment.create[:env] and deployment.rollback[:env]`; it now names
  `network.read[:public|private]` in the same list, so every capability a document may write appears
  in the one vocabulary listing most adopters ever meet.

* **A green cell in `protocol eval`'s table does not mean the same thing on every arm, and the arm
  word now says which.** A clean store-integrity row — no `store_broken`, `census.denied = 0` —
  means *the call was refused* on arm `driven`, where every tool call is answered at a seam, and
  only *the model did not do it* on arms `raw` and `native`, where nothing adjudicates a call at
  all. The two were printed in one column, in the same words, and read as the stronger of them.
  The rule is written where a reader of the table meets it: as a table of the four arms under the
  new **Evaluation surface** section of the CLI reference (which is also the first time
  `protocol eval` is documented there), as a line the text rendering prints under any table
  holding a `native` cell, and on `--arm`'s own help. **There is no new column** — a column is a
  change to a printed table's format, and that is the operator's decision, not this release's
  (`docs/design/native-arm-store-integrity-design-v0.1.md` § 6 O1 and § 8 OQ4). The same note
  states the other half: `denied: 0` is *nobody asked me*, not *nothing was refused*, and only one
  of those is a fact about the run.

* **A quoted metacharacter is an argument, not a composition.** The rule refusing `&&`, `|`, `;`,
  `>` and `$(…)` scanned the bare characters, so `grep -n "StolenLock\|took_lock_from" crates/` —
  one invocation whose `|` belongs to grep — was refused. It surfaced within minutes of the readers
  being admitted, three times in one state of a live run: a tool admitted and then refused the
  natural way to use it, which tells a session two things and lets it believe neither. The check now
  tracks single quotes, double quotes and a backslash escape, and asks whether the metacharacter is
  outside both. `$(` and a backtick still compose inside **double** quotes, because there they still
  substitute; inside single quotes they are literal. It is not a shell parser and does not try to be.

* **A driven state that admits reading can now read at scale.** `repository.read` renders `Glob` and
  `Grep`; Claude Code 2.1.247 offers neither, and its own error tells the model to *search file
  contents with `grep` via the Bash tool instead* — which the driver refused. So a session was told
  to do the one thing the driver denied, and run `W4-3/1` spent 19 of its 215 calls discovering that
  and never searched anything. The shell now also admits `grep`, `rg`, `ls`, `cat`, `head`, `tail`
  and `wc`, and only when the state admits reading. It is not a general shell: `sed` and `awk` write,
  `find` has `-delete` and `-exec`, `xargs` and `env` run something else, and composition and
  redirection were already refused — which is the only reason a reader cannot become a writer here.

### Fixed

* **A date written east of Greenwich is no longer a claim about the future, and one bad record no
  longer discards the whole evidence file.** A bare calendar date in `observed_at` —
  `observed_at: 2026-08-30` — is a *day*, and it meant midnight UTC — so a store at UTC+2 writing local calendar dates wrote a future instant for the
  last two hours of every UTC day: 20 of one adopter's 215 records, refused, and one refused record
  refused the entire document. A bare calendar date is now compared against the moment that day
  begins in the most-ahead timezone in use, UTC+14, so it is refused only once it is in the future
  for every writer on earth; the epoch-millisecond spelling keeps the exact comparison it always
  had, because a caller who wrote an instant meant one. The two spellings of one instant are
  therefore no longer the same value. **Nothing is relaxed and nothing is clamped**: a date that has
  begun in no timezone is still refused, no observation time is rewritten to the engine's clock, and
  the refusal is not downgraded to a warning. What changed is what it refuses — *that record*, named
  by file, by position in the file (`record 12`) and by the date as the writer wrote it, instead of
  an epoch pair and the document. `protocol evaluate` submits the file's other records, prints its
  evaluation and exits 1; a document whose every record is future-dated still fails.
  `protocol evidence inspect` puts every record to the same comparison, so the two verbs answer
  identically about one file — `--at`, which pins the comparison to the end of the named day rather
  than the wall clock, is the one place they still differ and the help now says so.
  `examples/evidence-horizons-corpus/writers-day.yaml` is the case, with the pair that carries it:
  two records naming one instant, one written as a day and admitted, one written as an instant and
  refused.

* **A `review-result` can be recorded with its body, and retired.** Found by the second adopter
  (`story:review-result-cannot-be-authored`): `protocol artifact new` took no body, `body` refused
  the kind as immutable, and `move --to archived` — the one transition
  `artifacts/lifecycles/review-result.yaml` declares — was refused by the same guard, so a review
  recorded through the CLI was an empty record nobody could complete or retire. Now
  `protocol artifact new … --from <path|->` writes the body with the record, for every kind; a move
  on an immutable kind is decided by its lifecycle alone, so `active -> archived` succeeds and the
  way back is refused by the ladder; `body` on one stays refused. `protocol artifact --help` no
  longer says an unwanted item is deleted with `rm` — it is retired with `move --to archived` — and
  `validate`'s deletion finding names that command and the recovery.

* **`protocol workflow flow --map` no longer takes a step map and throws it away.** The flag was
  accepted, its own help promised that each node would carry *what a harness actually does in that
  state*, and the projected document came out identical with it and without it — every node holding
  the state's name and summary and nothing a harness could run. It now carries the step into the
  node: a state with one step keeps one node and gains that step's fields, a state with several
  becomes a group named for the state whose steps are chained in the order the map wrote them, and a
  state the map is silent about is unchanged. An `llm` step travels as its `prompt`, `context`,
  `scope` — written `<glob>=<word>` in the map's own order, because first match wins — `harness` and
  description; a `command` step as its argv and the evidence running it establishes; an `operator`
  step as what it asks the person for. The header names the map and the pin it was written against,
  so a projection of `development/default` can be told from one of `development/checks`. A map
  pinned to another version of the workflow being projected is refused before anything is written,
  in the same words `protocol drive run` refuses it in. **Without `--map` the output is byte for
  byte what it was**, which is what the b10x harness's committed fixture walks.
* **A logged write that changed nothing no longer reads as a hand-edited revision.** `protocol
  artifact body` handed an empty body records an `update` event at the next revision with
  `changed: {}`; `validate` compared the document's revision against the last event that *changed
  something* — the create, one revision back — and reported `revision disagrees with event …@1`
  on a store nobody had touched by hand. Seen on a driven native run on 2026-08-29, where the
  driver's own validator step then exited 1 on every attempt and the run spent its budget in
  `receive`. Drift's revision check now reads the highest revision the log records, which is
  what the `forged` check already read; `status` and the written fields still fold from the last
  event that wrote them. `crates/aep-backend-markdown/src/drift.rs`.
* **`protocol eval` no longer tells you `native` is not an arm.** A run manifest naming an arm this
  build does not have is refused by name, and the refusal lists the arms there are — but that list
  was written when there were three arms, and the commit that added the fourth never reached it. So
  a manifest with a typo in `arm:` was answered with ``is not one of the three arms this evaluation
  has: `raw`, `plugin`, `driven` `` while `arm: native` was, and had been, perfectly readable. The
  refusal now lists all four and states no count, so the next arm cannot make it wrong again, and a
  test checks the list against the enum's own variants rather than against a second hand-written
  one.

* **An approved specification of somebody else's story no longer lets your task start
  implementing.** `spec-driven` and `clean-room` asked for `kind: specification, status: approved`
  and nothing more, which is a query over the whole artifact store — so in any store holding more
  than one piece of work, the guard before `implement` was satisfied by a specification of a
  different story. Run `NATIVE-1/1` passed it holding zero approvals of its own. Both rules now
  bind to the work the task declares (`relation: {kind: specifies, target: task}`), and the row
  reads `artifact specification (approved) which specifies this task`. **What to do if it now
  refuses you:** your task document names the story or task it is for, under `derived_from:`, and
  the specification carries `specifies: <that story>` — that is the edge a driven run's `specify`
  state already writes. A task that declares no `derived_from` is matched only by a specification
  naming the task itself (`specifies: task:<id>`); the refusal says which of the two ends is
  missing rather than leaving you to guess.

* **The record saying your specification is satisfied is now about *your* specification.**
  `protocol specification evidence` picked any in-force specification in the store, so in a store
  holding more than one piece of work it wrote the `specification` record — the one `spec-driven`
  reads as `specification.satisfied` before completion — about another story's document, and about
  one the guard before `implement` would itself have refused. It now selects by that guard's own
  rule: an approved specification whose `specifies` edge lands on the work the task declares. The
  task comes from `--task <file>` or from the project you are standing in; with neither in reach
  the selection is unbound and behaves exactly as it did. `--artifact` still says *which*
  specification to decide and no longer lets you name one that is not this task's — that is
  refused, naming what is declared and what your task said it was about — but it does still lift
  the status, so a `draft` can be asked whether it states anything a fact could decide. **What to
  do if it now refuses you:** the specification needs `specifies:` the story your task names under
  `derived_from:`, which is the edge a driven run's `specify` state already writes.

* **A run that walked past an `operator` step with nothing recorded now says so.** Until now the
  pause advanced the cursor and the resume carried on, and a run nobody approved was
  indistinguishable in its own record from one somebody did — `NATIVE-1/1` moved
  `establish_verifiers -> implement` holding zero approvals. The behaviour is unchanged for a
  person who moved the artifact the prompt named and resumed (the guard on the way out still
  decides); the report now carries one line saying the record holds nobody's answer.

* **A `command` step of a step map that says `protocol` now runs the build that is driving the
  run.** A `command` step is spawned by the driver (`crates/protocol-cli/src/drive.rs`) with the
  driver's own environment, so the name resolved against whatever `protocol` came first on the
  operator's shell `PATH`. Run `W4-3/1` hit
  a 0.28.0 install predating the `property` verb: `protocol property evidence` wrote nothing,
  the driver correctly reported *nothing was observed*, and the step spent its whole retry budget
  three times with the cause invisible in the message. The driver now spawns its own
  `current_exe()` for any step whose program's file name is `protocol`, which also guarantees the
  version agreement the run's evidence is recorded against. Every other program a step names —
  `cargo`, `bash`, `git` — resolves exactly as before.

* **A driven step on the native loop is told which programs it may start.** `harness-tools`
  withholds `run` outright when no allowlist was supplied, which is that loop's rule everywhere — a
  tool outside the surface does not exist rather than being refused. The driver never passed one, so
  a session asked to record something in the planning store, whose only route is the `protocol` CLI,
  had no way to start a process: it spent 30 `tool_search` calls, 28 of them distinct, hunting for
  `run`, `exec`, `shell`, `spawn` and `execute`. The list is the same decision `driven_surface`
  enforces on the vendor arm — the CLI, plus the readers a reading state may use — rendered rather
  than re-decided, and the native rendering is the stronger of the two: a program not on it has no
  tool to reach it, where the vendor arm refuses the call after the model has spent the turn.

* **The tool audit asks each harness in the vocabulary that harness answers in.** A vendor harness
  publishes one tool per act, so `offered_tools` is its answer; the b10x loop publishes three verbs
  over a catalogue — `tool_search`, `tool_describe`, `tool_invoke` — and states its reach as
  `available_operations`. Comparing a rendered catalogue against those three verbs reported every
  entry as missing: a run whose record published `file.read`, `dir.list`, `search`, `file.write` and
  `file.edit` was told, once per state, that it lacked all five. An audit that fires on a session
  holding exactly what it needs is worse than no audit, because the next true one is read as noise.

* **A shipped conformance spec now gates the *outcome* of the call it counts.**
  `conformance/trace/expectations.trace.yaml`'s `created-through-the-cli` is a `tool.called` row:
  it says a `protocol artifact new` was reached for, and nothing at all about what came back — so
  a recorded run whose only creating call errored satisfied it, and the eval passed on a store
  that got nothing. A second row beside it, `the-creating-call-succeeded`, decides the outcome
  over the same selector. It is a `tool.error_rate` at `at_most: 0.99` — *not every such call
  failed*, deliberately not *none did*: a refused chained command line followed by a clean
  re-issue is correct behaviour under the plugin's own guardrails, and a zero bound would report
  the guardrail working as a `gap`. The scope names `tools: [Bash, run]` **and**
  `operations: [command.execute, shell]`, which union, so the row decides on the native arm
  instead of going `unk` there. Against the committed fixtures it reads `ok` on both plugin-eval
  transcripts (0 of 2 calls failed) and `ok` on the driven honest step (1 of 2, rate 0.500).

* **The resume line the driver prints is a line that works.** A stopped run printed `resume with:
  protocol drive resume <run>` (`crates/protocol-cli/src/drive.rs`), and that command re-read none
  of `--map`, `--task`,
  `--pause-on-approval` or `--plugin-dir` — so an operator who typed exactly what they were told got
  a different run, or an error. The run directory now holds a `launch.json` recording how the run
  was started, and `resume` fills in anything the caller left out; a flag still wins over it. A run
  started before this exists resumes exactly as it did.

* **A resumed run gets the budget the operator typed.** `--max-iterations` was compared against the
  cursor's *lifetime* count, so a run that had already spent 25 iterations was `budget-exhausted`
  before evaluating anything — W4-2's first resume reported `steps 0 run` having done nothing, and
  no flag could have changed it. The bound is now on the call. The lifetime count stays in the
  cursor and the stop message reports both, because *how far did this run get* is a real question.

* **`protocol drive` refuses a run whose sessions could not reach the `protocol` CLI, instead of
  spending on one.** A driven `llm` step reaches the planning store through that CLI and nothing
  else — the state's shell exists for it and admits no other program. metaharness **constructs** the
  child environment rather than inheriting it, so the session's `PATH` is
  `$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin` and a `target/debug` exported before the command
  reaches the driver and never the model. Run `W4-3/1` walked two states being answered `exit 127,
  command not found` and submitted nothing, for $1.03. The refusal names the constructed `PATH` and
  the one install that lands on it — `cargo install --path crates/protocol-cli --root ~/.local`,
  because cargo's own default is `$CARGO_HOME/bin`, which the session does not search either.

* **A driven run of a project loads that project's own plugin without being asked.** Started
  without `--plugin-dir`, `W4-3/1`'s sessions loaded **no** plugin and answered `Unknown skill:
  planning` to the first thing the step map asks for — while offering the operator's personal
  skills and tools, because a session with no plugin is not a session with no inventory.
  `<project>/integrations/claude-code` is now the last fallback after the flag and the environment
  variable. A flag nobody remembers does not fail loudly; it produces a run that walks, spends and
  records the wrong thing.

* **A driven session is told which task the run drives.** A step map is written once and driven
  many times, so its prompt can only say *read the task under `.engineering/`* — and a repository
  that has driven more than one run has several sitting there. Run `W4-3/1` read `task.yaml`, which
  belongs to `W4-1`, described that objective instead, found its intake already in the store and
  reported the work done; the engine's cursor said `W4-3` the whole time. Nothing was violated —
  the guards held, the store was untouched, every transition was the engine's — the run was simply
  *about* something other than its own audit trail said. `StepContext` now carries the task and the
  prompt names it, with what it is derived from, before the map's own words
  (`story:governed-dogfood-run`).

* **The audit's own checks read the store with this tree's build, not the ambient `protocol`.**
  Four store helpers, `H1` and `F2` still resolved whatever was on `PATH` — here a **0.28.0** binary
  against a 0.32.x store, which is how five stories were reported as drifted that had not drifted.
  One resolved binary now, version-checked against the workspace, and exported so an inner run on a
  copy with no `target/` reads with the same build rather than falling back to the stale one.

## [0.32.1] — 2026-08-28

### Fixed

* **The evidence-coverage pre-flight runs before the machine check.** `protocol drive run` refused a
  map whose `llm` steps it could not spawn — *`metaharness` is not on PATH* — **before** asking
  whether the map could produce the evidence the plan demands. On any machine without that binary a
  real coverage gap was never looked for, so the test that guards it passed while guarding nothing.
  Coverage is decidable from the two documents and says *this map can never finish this plan*
  everywhere; the machine check says *this machine cannot run it today*. The document defect is
  reported first. An operator whose map has both problems now sees the one that will not go away by
  installing something.

* **0.32.0's `Release` workflow failed and published no GitHub Release**, on the two `drive_cli`
  tests that needed `metaharness` installed. Both now assert the same thing on a machine with the
  binary and one without: the whole gate is green with it hidden.

## [0.32.0] — 2026-08-28

### Added

* **A driven run of the cargo map produces every kind its plan demands.** `protocol drive run --map
  drivers/development/default.yaml` on a `kind: feature` task starts with **no
  `--allow-evidence-gap`**: the refusal that named `contract_result`, `property_test_result`,
  `verification` and `specification` now names none. Each is minted by a `command` step the driver
  ran, carrying `producer: verifier` — never a producer a model could forge — and a record that is
  missing or unreadable submits nothing and says why, rather than being fabricated. Three producers
  are new verbs; the `property_test_result` one is an exhaustive checker over all 27 `Truth`
  assignments rather than a sampled suite (`story:evidence-producers-for-the-driven-map`).

* **The eleven ladders `entity-runtime` re-expresses are held equal to ours by a test.** 77 edges,
  compared in both directions against a pinned copy of their definitions — an edge invented there
  fails, and an edge our ladder grows and theirs does not express fails too. The mapping is
  **accepted in part**: states, initial states and edges accepted; the eleven operation names stay
  theirs and unendorsed, because `move --to implemented` and `execute --operation implement` are
  different published surfaces (`story:entity-runtime-mapping`).

* **`task plan-check` is a step of the gate.** `protocol artifact validate` over this repository's
  own planning store, third of nineteen steps: an unparseable document, a relation into a repository
  the workspace manifest does not declare, or a status no lifecycle permits now fails `task check`.
  A status reached on an assertion rather than a record is **reported and does not fail**, which is
  deliberate — refusing it would stop anybody closing a story on the day a runner is down
  (`story:own-engineering-store`).

* **`protocol artifact explain <id>` answers *what made this done*.** Per status the artifact
  reached: the move, the evidence records it rested on, and — the point of the verb — **the revision
  the artifact was at when each record was admitted**, so an edit made afterwards cannot make an old
  record look like it was about the new text. A status reached with no record says so. The join is
  one-to-many and is a stored fact, not a path: deleting the file a record's `--ref` names does not
  unlink it. Read through the contract, so markdown, SQLite, Postgres and the hybrid answer alike
  (`story:completion-audit-join`).

### Fixed

* **`protocol drive` reads a crossing relation the way `validate` does.** A relation into another
  repository that `.engineering/workspace.yaml` declares was a dangling edge to the driver and a
  declared one to `validate`, so `drive` refused to start — *the planning store cannot be trusted* —
  on a store the other verb had just called valid. One edge in this repository's own store was
  enough to stop every driven run before its first step. A store with no manifest beside it still
  refuses, unchanged.

* **`protocol artifact validate` is green from anywhere inside a project.** It resolved the
  workspace manifest against the *working directory*, so the same store validated at the repository
  root and reported every cross-repository relation as undeclared one directory down — exit 0 and
  exit 1 for one store, depending on where the person was standing. It now walks up to the project
  the way discovery does (`story:own-engineering-store`).

* **Over SQLite and Postgres, every provenance account was silently empty.** A move's `decided_on`
  travels as JSON *text* and was read only as a JSON object, so a move made on a bare
  `--evidence KIND=COUNT` was indistinguishable from one the store held a record for — in exactly
  the two stores that have no journal file to fall back on. `protocol artifact history` over them
  prints the `(on asserted evidence)` marker it was dropping. Found while building `explain`, which
  could not have been honest over it.

* **`synth-check` can run in a `git worktree`.** `go build` stamps the building repository's VCS
  state, and a linked worktree's `.git` is a file, so the step failed with `error obtaining VCS
  status: exit status 128` on a tree byte-identical to the one where it passed. `GOFLAGS=-buildvcs=false`
  is pinned beside `GOPROXY=off` and `GOTOOLCHAIN=local`; the synthesised module's identity is its
  specification's digest and no binary the step builds is kept.

### Removed

* **A generated artefact no longer says which build made it.** `compiler_version` and
  `generator_version` are gone from every projection's provenance block, from the three conformance
  suites (with `synthesizer_version`), from the synthesised trees (`planner_version`) and from the
  compiled cluster IR. `source_digest` and `contract_digest` are untouched and still answer what the
  artefact was made *from*. Every stamp was the release tag copied into 118 files, so a version bump
  rewrote all of them with nothing else changed — and a release cut without regenerating failed the
  gate at the tag. An `ess_conformance` record written before this still reads: both fields stay
  declared on the published schema and accepted on input, never written, and neither was ever
  `required`, so nothing that validated stops validating. If you parsed a build version out of a
  generated file, read the tag the file shipped under instead. Held by
  `a_version_bump_rewrites_no_generated_file` (`story:generator-version-stamp`).

* **`task plugin-eval` and `task driven-eval` are gone.** Both invoked
  `integrations/claude-code/eval/`, which `epic:metaharness-migration` deleted on 2026-08-22 — the
  agent-eval checks and their recorded transcripts live at metaharness
  `evals/aep/`, and `run.sh` retired with the hooks it inspected. Neither was a
  step of `check`, so nothing was failing and nothing noticed for six days. Removed rather than
  repointed: the eval is no longer this repository's to run. `task codex-eval` is unaffected
  (`story:driven-eval-acceptance`, archived).

### Changed

* **`story-completion-evidence-design-v0.1.md` is accepted in part.** A story still cannot reach
  `implemented` without a `test_result`, and that is unchanged; what the verdict settles is what
  comes next. The principle that would demand a `trace_conformance` record for the run that did the
  work is accepted **and not written**, because measured on this store on the day of the verdict it
  would put 38 of 38 implemented stories in deviation: no artifact here carries such a record yet.
  It ships when the first driven run has closed a story. The engine judging a producer's
  *independence* from what it reports on is **refused** as part of this rule and carried by
  `story:evidence-producers-for-the-driven-map`. Gap-register row 39 closes on the verdict
  (`story:completion-needs-evidence`).


## [0.31.0] — 2026-08-28

### Changed

* **An edge no longer moves its source document's revision.** `protocol artifact relate` (and
  `new --relate`) wrote the edge into the source's frontmatter and counted that as a revision, so
  the same plan answered `revision 2` on markdown and `revision 1` on SQLite. The contract never
  counted a relation as a revision of either endpoint, and now no store does: the document changes,
  the event lands at the revision the document already had, and `store_selection.rs` compares
  revision numbers exactly again. The golden fixture was re-recorded with this build; `expected/`
  and `reads/` are no longer 0.28.0's bytes for that one verb, and `golden_plan.rs` says so
  (`story:relation-bumps-a-document-revision-but-not-an-entity`).

### Fixed

* **`protocol artifact evidence` accepts the instant it defaults to.** Without `--at` the verb
  produced `YYYY-MM-DDTHH:MM:SSZ` and its own reader knew only a date or epoch milliseconds, so
  every recording had to type `--at`. The reader now takes the second-resolution form too
  (`story:evidence-verb-refuses-its-own-default-instant`). Fixing it exposed two more things the
  same defect had hidden: `store_selection.rs` had compared two identical *failures* and called
  the stores alike — it now asserts every write's exit code — and a SQLite or Postgres plan counted
  no evidence on hand at all, because an accepted command's audit record names its subject and not
  the kind of evidence it recorded (the contract keeps `decision` for refusals). Evidence on hand is
  now counted from the entity's events, the same source `history` reads, so an evidence-gated move
  over SQLite is decided on what was recorded.

* **`epic:planning-store-as-backend` is `implemented`**: fifteen stories implemented, one archived
  as superseded (`story:sqlite-backend`), one follow-on filed and delivered above.

## [0.30.0] — 2026-08-28

### Added

* **`project.yaml` names the store, and every verb opens it** (H1). `aep.project/1` gains `store:`
  — `markdown` (the default, so no existing project changes meaning), `sqlite: <path>` (relative to
  `.engineering/`, as every project path is), `postgres: <url>`, and `hybrid: {authority, read,
  on_unreachable, on_divergence, local, replica}`, whose words are carried and checked — a hybrid
  missing one is refused by name at `protocol validate`, which now validates the discovered
  `project.yaml` too — and not yet opened (`story:hybrid-backend`). Every `protocol artifact` verb,
  `protocol drive` (through `aep_driver::PlanSource`, which the driver now reads its plan from) and
  `protocol conformance --backend project` open through it; `--store <dir>` remains the markdown
  override. `protocol artifact history` over a SQLite or Postgres plan reads the entity's event log
  back as journal entries, so the same history prints over every store.
  `examples/planning-passkeys/.engineering/project.sqlite.yaml` is the example on SQLite, and
  `store_selection.rs` runs every verb over both, each as its own process, and compares the output.
  Two things were found by that comparison and written down rather than hidden: an edge moves a
  document's revision and not an entity's
  (`story:relation-bumps-a-document-revision-but-not-an-entity`), and `validate`'s drift lines
  have no counterpart where there is no second record.

* **An observation is an event on the entity it is about.** The `Identity` projection — the shape
  of a SQLite or Postgres plan — writes an event at the entity's unchanged revision for a relation
  (on its source) and for recorded evidence (on its target), where before it wrote nothing: a
  relation is a record of its own and evidence changes nothing, so the contract named no affected
  entity. That is what a rebuilt history and a second process's evidence count read. The pin moved
  to `entity-runtime` 0.13.0: 0.12.2's providers accept an observation at a reached revision
  (SQLite and Postgres previously failed the second one on the primary key; the file store dropped
  it silently) and its provider suite has a tenth case holding them to it; 0.13.0 adds the
  serialisable `Divergence` and `Hybrid::remember` the hybrid plan's verbs need.

* **`store: hybrid` — the plan kept twice, under four declared words** (H4,
  `story:hybrid-backend`). `aep-backend-hybrid` is the adapter, shaped by the plan's own projection,
  over `entity-runtime`'s `Hybrid<MarkdownProvider, R>`: markdown for pull requests and a SQLite or
  Postgres replica for tooling, with `authority`, `read`, `on_unreachable` and `on_divergence` typed
  in `project.yaml` and none defaulted. The atomicity guarantee is the runtime's, cited rather than
  chosen (`store-v0.1.md` § 10). Two verbs are ours: `protocol artifact divergences` lists what one
  side took and the other did not and says which side is authoritative (exit 1 while anything is
  outstanding); `protocol artifact catch-up` replays them at the side that missed them and writes
  back what it could not. Divergences live in `divergences.jsonl` beside the plan between commands,
  because every verb is its own process — which is what `entity-runtime` 0.13.0's serialisable
  `Divergence` and `Hybrid::remember` exist for. The sixteen suites run against the composite with
  either side as authority; `protocol conformance --backend hybrid` runs them from the command line,
  and `--backend project` resolves a hybrid project to that. `MarkdownProjection` now hydrates any
  plan-shaped store (`PlanStore`) through its `Store` traits rather than reading the directory, so
  a hybrid's declared read path governs hydration too. `examples/planning-passkeys/.engineering/
  project.hybrid.yaml` is the example kept twice, and `store_selection.rs` runs every verb over it
  beside the other two stores, then makes the replica refuse a write, lists the divergence from a
  second process and catches it up from a third.

### Fixed

* **A seeded plan kept its tags.** `seed::from_manifest` carried title, summary and owner and not
  tags, so a plan seeded into SQLite answered `list --kind` and a board with untagged artifacts.

* **`describe_type` reports the ladder** (D-P5 closed). `TypeDescriptor::lifecycle` is filled for
  every planning kind — the initial status, every status, every `from -> [to]` edge, and, new on the
  descriptor, `requires`: which rungs cost evidence, of which kind, how many. Rendered from the same
  `EntityDefinition` the kernel decides a move with, so what a harness reads and what the store
  enforces cannot drift; the memory, SQLite and markdown backends report identically, and the edges
  equal `protocol artifact lifecycle <kind>`'s output for every kind.

* **`aep-backend-postgres`: the contract over PostgreSQL** (P5, as a type). `PostgresBackend` is
  `EntityBackend<entity_postgres::PostgresStore>` — the runtime's provider with a server, one
  transaction per commit, writers of one instance serialised by a row lock. Two processes writing
  one artifact resolve to one accepted write and one refusal naming the revision it lost to; the
  loser latches and reopens. The sixteen suites and the faulty-backend guard run against a server
  when `ENTITY_POSTGRES_URL` names one; the gate's new `postgres-check` step prints
  `postgres-check: skipped, ENTITY_POSTGRES_URL unset` when it does not, and CI runs them against a
  `postgres:16` service. `protocol conformance --backend postgres --store <url>` runs them from the
  command line. The pin moved to `entity-runtime` 0.12.1.

### Changed

* The ladder bridge (`kernel`) moved from `aep-backend-markdown` into `aep-backend-entity`, where
  the adapter renders and decides with it; the markdown crate re-exports it under the same path.
  `Identity::with_lifecycles` gives a SQLite plan its ladders.

## [0.29.0] — 2026-08-28

**Wave G of `docs/plan/store-waves-f-g-h.md`: the plan's own store is a provider.** The markdown
documents are an `entity-store` provider held to the runtime's own suite, `MarkdownBackend` is the
one adapter over it, history and audit come from the event log, and `validate` reports an edit made
in an editor or an `rm`. Deviations D-P2, D-P3 and D-P4 close. Four stories, accepted, implemented
and moved to `implemented` on recorded evidence the same day.

### Added

* **The plan's documents are an `entity-store` provider.** `aep_backend_markdown::provider::MarkdownProvider`
  implements `StateProvider`, `EventProvider` and `Store` over `.engineering/planning/`: a document's
  frontmatter is the instance's fields, `status` its state, `revision` its revision, the body a
  `body` field; `journal.jsonl` is the event log, one event per line beside the entries already
  there. It passes `entity-runtime`'s own provider suite — nine cases written by somebody who has
  never seen a planning document — and the suite is shown to catch a deliberately wrong copy of it.
  A refused commit changes neither file; a document a person wrote by hand loads with an empty log;
  every one of this repository's own documents round-trips byte for byte.

* **History and audit are answered from the event log** (D-P3 closed in full). `history()` and
  `audit()` on every `EntityBackend` read the provider's events — a second process answers exactly
  what the first wrote — and the in-memory records are a cache of the log, not its source. For a
  plan, the accepted commands another process ran arrive in `audit()` with their command ids and
  the revision they produced; a document with no events yet answers what it answered before.

* **`protocol artifact validate` reports drift and deletion** (D-P2 and D-P4 closed by detection). A
  document whose frontmatter disagrees with the fold of its events is **drift**, per document, naming
  the fields and the event; a document the log has events for and the store no longer holds is
  **deleted**, naming the last event; both exit 1. A document with no events at all predates the log
  and is counted (`N document(s) predate the event log`; `pre_provider` in JSON), not blamed.
  Prevention — a hook, a lock — was considered and refused on the record: a check in the gate cannot
  be routed around.

### Changed

* **`MarkdownBackend` is the adapter over that provider.** It is
  `aep_backend_entity::EntityBackend<MarkdownProvider, MarkdownProjection>` behind the same `open`;
  its hand-written `persist`, latch and `journal::append` are gone — `backend.rs` went from 883
  lines to 196, and what left is not kept beside the new path. What stays is the plan's shape, in
  `projection.rs`: the prose preserved, relations added into frontmatter and never removed, the
  ladder consulted on a status that arrives on a plain update, and the journal's own vocabulary
  noted on every event. **Nobody can tell**: a fixture recorded with 0.28.0 pins that `new`,
  `relate`, `body`, `move` and `evidence` leave byte-identical documents and that `list`, `board`,
  `graph`, `validate`, `lifecycle` and `history` print the same bytes. `journal::read` answers the
  same entries for a line written before the provider and one written after it.

* **The adapter has a projection seam.** `EntityBackend<S, P = Identity>`: `Identity` is the shape
  a SQLite plan takes — every record under its own type, hydrated on open; a plan-shaped projection
  lives beside the plan's provider. Nothing changes for `SqliteBackend`.

* The pin moved to `entity-runtime` 0.11.0; every event the adapter writes carries `args` — the
  command's payload, which is what the operation was decided on.

## [0.28.0] — 2026-08-28

**Wave F of `docs/plan/store-waves-f-g-h.md`: the storage layer starts becoming `entity-runtime`'s.**
One runtime version instead of two, one adapter over any of its stores, events that reach the file,
a SQLite plan that reopens, and a conformance verb that names what it ran against. The plan and its
five stories were accepted, implemented and moved to `implemented` on recorded evidence the same day.

### Added

* **`aep-backend-entity`: the contract over any `entity_store::Store`.** `EntityBackend<S>` is what
  `SqliteBackend` was — apply in `aep-backend-memory`, then `commit` to the provider with a **read**
  expectation, latch on failure, latch covers reads — with the provider as a type parameter. The
  sixteen suites and the faulty-backend guard run against it over `SqliteStore` and over the
  runtime's `MemoryStore`. The next durable backend is a line: `EntityBackend<TheirStore>`.

* **A SQLite plan file now holds its own history.** Every accepted command writes one event per
  affected entity into the store, in the same transaction as the instance: the command's type as the
  event type, the status before and after, the fields written, and — in the event's `payload` — who
  issued it, when, in which flow, what caused it, and what a status move was decided on. A second
  process reading the file gets the history; before this it got the instances and an empty log. A
  refused command writes nothing, a replayed one writes nothing, and the stored events fold back to
  the stored instance through the runtime's own `rehydrate`.

* **The pin moved to `entity-runtime` 0.10.0**, which carries `StateProvider::ids` — what
  hydration enumerates with. `EntityBackend::over` is fallible now, because opening reads.

* **`protocol conformance --backend memory|markdown|sqlite [--store <path>]`.** The verb ran the
  sixteen suites against the in-memory reference backend and nothing else, while its help said so
  and a story said otherwise. It now runs them against the backend you name — `--store` says where a
  durable one lives; without it `sqlite` gets an in-memory database and `markdown` a scratch
  directory, because the suites write — and the report's first line says `ran against: …`, with a
  `ran_against` field in JSON and YAML. The default stays `memory`, so no existing invocation changes
  meaning; `--backend memory --store …` is refused rather than ignored.

* **A populated SQLite plan is read back on open.** `SqliteBackend::open` hydrates: every entity
  with its metadata and history, every relation not since removed, every audit record — a
  refusal's too — and every applied command, all under the identities the first process stored.
  A second process sees what the first wrote, continues past it with fresh identities, and
  recognises a replayed command. The refusal *"the database already holds `…`, and this backend
  did not write it"* is gone with the defect it guarded against. A store holding a row the backend
  cannot read back refuses to open, naming the row, rather than answering about part of it.
  Measured over this repository's own plan: 124 artifacts and 241 relations reopen in about
  120 ms (debug build); seeding them through the contract took 11.5 s, three SQLite transactions
  per command.

### Changed

* **`aep-backend-sqlite` is an instantiation.** `SqliteBackend` is `EntityBackend<SqliteStore>`
  behind the same `open`, `in_memory`, `latched`, `len` and `is_empty`; nothing a caller writes
  changes. Its conformance tests moved to the adapter's crate — `cargo xtask guards` is what says
  they moved rather than copied — and what stays in `aep-backend-sqlite` is what only a file can
  show: the row read back through a second handle, and the foreign-row refusal.


* **One `entity-runtime`, at `0.9.1`.** Two were compiled into this workspace: `entity-core` 0.5.2
  under `aep-backend-markdown`, which decides every `protocol artifact move`, and 0.8.0 under
  `aep-backend-sqlite` — `cargo tree -i entity-core` answered *"specification is ambiguous"*. The
  tag is now declared **once**, in `[workspace.dependencies]`, for all three `entity-*` crates, and a
  new gate step, `dep-check` (`cargo xtask deps`), fails naming both versions if that ever splits
  again.

  What the move-deciding kernel gains from 0.5.2 → 0.9.1: `DomainEvent` records `from_state`,
  `to_state` and the fields it `changed` (their R-89), and `entity_core::rehydrate` folds an instance
  from its events, refusing any event whose transition the definition does not declare — including a
  forged creation event, closed in their 0.8.0 (R-97). Every ladder verdict is unchanged:
  `tests/kernel_equivalence.rs` passes against the single version, and nothing you type changes.

## [0.27.3] — 2026-08-26

**A release cut later retroactively failed the gate of a release cut earlier**, and `0.27.1` sat on
the remote for an hour as a tag nobody could find a release for.

### Fixed

* **`status --check` and `version-check` asked the clone what had shipped, not the commit.**
  `0.27.1` and `0.27.2` left in one `git push`. The Release workflow for `0.27.1` checked out
  `0.27.1`'s commit — with *both* tags fetched, because a clone holds the tag namespace and not a
  snapshot of it. `docs/status.md` at that commit recorded 38 tags; `git tag` answered 39; the
  drift check refused a release that had been correct when it was cut, and the publish step after
  it never ran.

  Every tag lookup in `xtask` now asks for the tags reachable from `HEAD`, so each release's gate
  answers the question it means to ask: what had shipped *as of this commit*. `status`,
  `version_check` and `previous_tag` all took the same filter. At the tip of `main` the two sets
  are identical, so nothing about the everyday check changes.

  Pinned by a test that builds the same shape — two tagged commits, `HEAD` detached at the older
  one — and which fails with the workflow's own error message when the filter is removed. The
  check that was supposed to catch drift could not catch its own release, which is the recurring
  shape of this repository's defects: the safety net needed a second one behind it.

  `0.27.1`'s GitHub Release is now published from its `CHANGELOG.md` section — byte-for-byte what
  the workflow would have produced, since this repository's Release workflow attaches no binaries.

## [0.27.2] — 2026-08-26

**Six documents were still describing a deviation that closed in 0.27.0.** Cutting a release does
not update the prose that describes what it changed, and this repository's own rule is that a claim
the code does not support is not made. These were making one.

### Changed

* **`README.md`, `docs/status.md` and `where-this-stands.md`** said *"no durable backend implements
  the storage contract — the contract still has one implementor and it is in memory"*. It has three,
  and the sixteen suites run against all of them.

* **`docs/guide/backend.md`** told an adopter writing their own backend that `aep-backend-markdown`
  *"implements neither trait and the suites cannot be pointed at it"* — the page whose whole job is
  to explain what implementing the contract means. It now says what the two durable backends do,
  including that neither reimplements the contract: each hands every command to the in-memory
  reference and adds durability, so idempotency, revision conflicts and the audit a refusal leaves
  are decided once rather than twice.

* **`website/docs/status/limitations.md`** carried the old storage limitation. It now carries the
  two that are real — `protocol conformance` runs only in-memory, and `describe_type` reports no
  lifecycle — plus one that was not written down anywhere: **`aep-backend-sqlite` does not hydrate
  on open and refuses a row it did not write**, because deferring hydration is a decision and
  destroying data is not.

* **`harness-planning-and-driver-design-v0.1.md`** marks **D-P1 closed** and **D-P5 still open** —
  the latter noted with the fact that an acceptance line claimed it closed until a review caught it.

### Fixed

* Nothing in the code. Everything here is prose that had stopped being true, and it is listed rather
  than quietly corrected because *when* a document stopped matching the code is the part that tells
  you how it happened.

## [0.27.1] — 2026-08-26

### Fixed

* **The lab's copy of the emitted glue had been stale for two releases, and only CI could see it.**
  `website/src/pages/lab/_bridge.mjs` must be byte-identical to
  `generated/web/billing/bridge.js`; the version stamp went to `0.26.0` and the copy stayed at
  `0.1.0`, so anybody opening the lab ran glue built by a compiler two releases old.

  The check lived **only** in the Website workflow. `task check` was green while CI was red, through
  0.26.0 and 0.27.0 — which is this repository's own rule failing in the direction nobody watches: a
  check that exists only in CI cannot be run before pushing, so it reports after the tag rather than
  before it.

  Fixed three ways rather than one: the copy is synced, `lab-check` is now a step in `task check`,
  and `AGENTS.md` § *Releases* lists the copy among the regeneration steps — it named four and there
  were five.

## [0.27.0] — 2026-08-26

The planning store implements the contract it is held to, and every verb that writes goes through
one door. **Two independent reviews of this work found 27 defects between them; all of them are
fixed here**, and the ones that were false claims are named rather than quietly corrected.

### Added

* **`aep-backend-markdown` is a contract implementation.** `MarkdownBackend` implements
  `CommandService` and `QueryService`; the sixteen `aep-conformance` suites run against it and are
  shown to **fail** it under injected faults, because a suite that has never failed is not evidence
  that it can. Deviation **D-P1** is closed.

  The contract logic is not written twice: every command goes to `aep-backend-memory` and this crate
  adds durability. Idempotency, revision conflicts, "a refusal still leaves an audit record" — each
  is a decision whose wrong version looks right, and two implementations drift in exactly the ways a
  suite run months apart discovers.

* **`aep-backend-sqlite`** — the first database backend. One file, no server, adapted over
  `entity-runtime`'s transactional store rather than written again here.

* **Two new commands, and their absence is why D-P1 stayed open.** `aep.status.move/v1` and
  `aep.evidence.record/v1`. The planning store's ladders are data with an open status vocabulary,
  and an evidence record is the input to the evidence-gated move — neither had a command, so the CLI
  wrote both behind the contract. `UpdateEntity`'s own documentation says a `status` key there is a
  mistake; there was simply no command that named the move.

* **`eval.matrix/1` has a fourth column, `advisory`.** An advisory row judges whether the *evidence*
  is worth anything, not whether the run did anything wrong; counting one in `violated` published
  `violated: 1` against runs that behaved perfectly. Omitted when zero, so a matrix written before it
  reads unchanged.

* **Three gate steps**: `cargo xtask guards` (no test body duplicated), `cargo xtask claims` (a
  released `### Fixed` entry names something that existed to be broken), `cargo xtask version` (the
  workspace version matches the newest tag, so `protocol --version` can say which build it is).

* `protocol artifact validate` reconciles the journal against the files, and reports every status
  that reached its rung on an **asserted** rather than a **recorded** provenance.

### Fixed

* **A status could reach a document with no ladder consulted.** The contract permits a `status` key
  on an `UpdateEntity` — its own suites use one — so the store is the only layer that can refuse an
  illegal move, and it did not: a story at `draft` reached `active` with `draft: [proposed,
  archived]` declared. A `MoveStatus` stays exempt, because the engine has already decided it
  against the ladder **and the evidence presented**.

* `MoveStatus` bypassed the immutability check, so a **review result** — the one kind that exists to
  be uneditable — could be edited after the fact. It also declared `expected_revision` and never
  enforced it, silently dropping a caller's assertion about what it had read.

* **A relation was journalled as `Change::BodyReplaced`**, a repeated `--relate` bumped a new artifact to
  revision 3, and a replayed command re-wrote the file — three defects compounding, all from the
  same place: the no-op guard ran after the revision had already been incremented, so it was dead.

* `protocol artifact new --relate` with an unresolvable target wrote the artifact, journalled it,
  and then failed — leaving the caller told the command failed and holding a document without the
  edge they asked for. Targets are resolved before anything is written.

* **The latch did not cover reads**, in either durable backend, though both modules said it did.

* `protocol artifact evidence --at` was ignored once the verb went through a command. The instant
  travels on the command now, because the clock is read at the edge and handed in.

* Seeding refused any store holding a **cross-repository relation**, which broke `protocol artifact
  new` on this repository's own plan. A crossing is not a dangling edge.

### Changed

* `AGENTS.md`, `docs/plan/gap-register.md` and `aep-backend-markdown`'s own module documentation all
  said D-P1 was open and the contract had one implementor. They said so after it had three.

## [0.26.0] — 2026-08-26

**A review of 0.25.0 found that three of its published claims were false, and that it had quietly
weakened a check that had nothing to do with workspaces.** Two independent reviewers, run against the
released commit; two of the three were found by both. Nothing in 0.25.0 has been rewritten — a
published section stays as it was published, and the corrections are here.

### Corrections to 0.25.0

* **"Cycle detection holds over the combined graph" was not true — it was absent.** No combined graph
  was ever built. `find_cycle` had one caller, over one manifest, and `crossing_relations` resolved
  each edge without asking what shape they made together. See below.

* **"`story:passkey-login` exists in more than one repository today and they are different stories"
  is not a fact.** It is held by **no** member of the shipped workspace. The behaviour is real and
  tested — against a fixture — but the example given for it was not. `story:namespaced-identity`
  carries the correction in its own body.

* **A member listed by `members` is reported as `absent`, not `unresolved`.** The changelog used a
  word the code does not. They are different states and the distinction is the point of the feature,
  so the code's word is the one that stands.

### Fixed

* **Cross-member cycles are now found, because now something looks for them.** `Assembly::cycles()`
  builds the combined graph — every artifact addressed `member/id`, every resolved crossing an edge —
  and walks it per relation kind. `protocol workspace crossings` reports what it finds and **exits 1
  on a cycle whatever `--strict` says**: `--strict` is about unresolved members, which is a fact
  about your checkout, and a cycle is a fact about the plan, which is wrong on every machine.

  An unresolved crossing contributes no edge. A cycle that depended on which repositories happen to
  be checked out here would not be a fact about the plan either.

* **Any `member/` prefix disabled cycle detection inside a single store.** The same loop passed or
  failed depending only on whether its edges were written `story:beta` or `self/story:beta`. This was
  a regression 0.25.0 introduced against behaviour that worked before it.

* **The dangling-reference check was relaxed for every repository, workspace or not.** The exemption
  applied to any target whose namespace contained a `/` — unconditionally, with no workspace file
  needed. In a plain single-repository store every dangling edge could be hidden behind one
  character, and a misspelled member (`entity-runtme/story:typo`, which is nobody's repository)
  passed silently as a deliberate crossing.

  A graph now carries the members its workspace declares. A target naming one of them is a crossing;
  a target naming anything else is a dangling edge, checked exactly as a local target is.
  `ArtifactGraph::build_in_workspace` and `StoreReport::graph_in_workspace` are the workspace-aware
  forms; `build` and `graph` declare no members, which is the safe default.

* **`protocol workspace show` reported a fact about your checkout as a fact about the plan.** It
  discarded the unresolved-member list, so a member you had not checked out produced *"held by no
  member of this workspace"* — indistinguishable from an artifact that genuinely does not exist. It
  now says *"no member of this workspace that could be read"*, lists the members it could not read,
  and names the member the reference asked for.

* **A member whose source is a pinned git revision vanished from the assembly entirely.** It was
  dropped rather than arriving as an empty member, so with `beta` spelled as a path an ambiguous
  reference was refused, and with the same `beta` spelled as a git locator the same reference
  resolved to `alpha` and exited 0. The unresolved list now reaches every verb that can report it.

* `parse::workspace`'s documentation was `parse::project`'s, copied. It is a public item and it
  described the wrong file.

## [0.25.0] — 2026-08-26

One CLI across repositories. Until now every repository was an island — `protocol artifact` read one
store, and the limitations page said so: *no federated artifact graphs across repositories*. A story
here that is blocked by a story in `entity-runtime` can now say so, and one command answers over
both.

### Added

* **A workspace names its member repositories, and pins them.** `.engineering/workspace.yaml` at
  `version: aep.workspace/1`, each member a name and a `source`. `source` is the same locator
  `project.yaml`'s `protocols:` takes, which means it arrives carrying two refusals already argued
  for: an **absolute path is refused**, because a path rooted somewhere only one machine has is true
  on that machine and false in CI; and an **unpinned git locator is refused**, because a tree that
  can move under you is a dependency whose meaning changes with no commit in your repository. A
  generated JSON Schema ships beside it.

* **`protocol workspace` — `list`, `crossings`, `show`, `members`.** The plan across every member,
  the relations whose two ends are in different repositories, one artifact, and what resolved.

  ```console
  $ protocol workspace crossings
  aep/story:assemble-across-sources informed_by entity-runtime/story:typed-references  [entity-runtime]
  ```

* **A member nobody has checked out is a normal condition, not a broken workspace.** `members` exits
  `0` with an unresolved member listed as unresolved. A command that failed because a colleague's
  repository is missing from your disk is a command nobody could use.

* **An ambiguous reference is refused by name.** `story:passkey-login` exists in more than one
  repository today and they are different stories. An unqualified reference held by more than one
  member resolves to `Ambiguous`, listing every holder, and the lookup returns **no document** —
  returning *a* document for an ambiguous reference is exactly the guess this path exists to refuse.
  An unqualified reference is not the same thing as an ambiguous one, and the two are distinguished
  where the type is defined.

* **Membership is carried beside the id, never folded into it.** Nothing is renamed and no id is
  rewritten on the way into the assembly, so reading a store through a workspace and reading it on
  its own give the same artifacts — and a member can be dropped from the workspace without touching
  a file.

* **A member that failed to load is reported, never skipped.** It produces an empty member rather
  than an absent one, with its failures attached to its name. An assembly that quietly answered from
  two members when it was asked about three would give a smaller answer that looks exactly like a
  complete one.

* **A relation whose target lives in another member.** The vocabulary is unchanged — `blocks`,
  `depends_on`, `derived_from`; what is new is that the target resolves elsewhere, and that an
  unresolvable one is a typed fact rather than an error. Cycle detection holds over the combined
  graph, including a cycle that only exists once two members are read together.

### Notes

* Every `protocol workspace` verb **reads**. Nothing here writes to another member's store, which
  would need a permission model, a lock and a review path that do not exist.

## [0.24.0] — 2026-08-26

### Added

- **A release workflow, so a tag cuts its own GitHub Release.** `.github/workflows/release.yml`
  fires on a version tag, runs `ci.yml` itself — called rather than copied, so a tag cannot ship
  against a shorter gate than `main` does — and creates the release with the tag's own section of
  this file as its notes.

  No binaries are attached. Every release this repository has cut carries none, and `protocol` is
  built from source (`cargo build --release -p protocol-cli`). Adding archives later is a matrix job
  beside this one.

- **`cargo xtask notes <version>`**, which prints one release's notes: its CHANGELOG section,
  reflowed. Release notes render as **GFM**, where a single newline becomes a `<br>`, so a file
  hard-wrapped at 100 columns arrives broken after *"added"* and before *"the"*, in spots no author
  chose. The file stays wrapped — that is the right shape for something reviewed in a diff — and
  only the notes are joined.

  `cargo xtask notes --self-test` holds the eight shapes the reflow must not damage: fenced code,
  tables, headings, blockquotes, list-item boundaries, paragraph joining, blank-line separation, and
  a line ending in two spaces. It runs in the workflow **before** the notes are generated, because
  the failure it catches is silent — nobody re-reads a release they already cut.

### Fixed

- **The delivered-waves check reads the tags when the run is on a tag.** The first run of the new
  release workflow failed here, against a `docs/status.md` that was correct in the commit it was
  checking. The job already asked for `fetch-depth: 0` and `fetch-tags: true`, and on a **tag** ref
  — which is how `release.yml` calls the gate — that still left the other tags behind, so a 34-row
  record was compared against a near-empty tag list. An explicit `git fetch --tags --force --prune`
  does not depend on which ref triggered the run, and the step now prints how many tags it can see.

- **A deliberate line break is no longer eaten by the reflow.** A line ending in two spaces is
  Markdown's own request for a break; the reflow ended the paragraph there but dropped the two
  spaces, leaving the break standing on the very GFM quirk the reflow exists to remove. The spaces
  are kept now. Found by the self-test above on its first run.

### Documentation

- **The website describes the tooling as it now is.** It had drifted badly: since `0.13.0` the
  repository gained 7,805 lines and the site gained three. What changed:

  | page | what it now says |
  |---|---|
  | *Lifecycles, decided as data* (new) | the ladder is a YAML file, decided by `entity-core`; evidence-gated and date-gated rungs; the journal; and what that kernel explicitly does **not** do |
  | *CLI reference* | `protocol reverse` — four verbs, 2,782 lines, and **no mention on the site at all** until now — plus `artifact history` and `artifact evidence` |
  | *Where this stands* | current as of `0.23.2` rather than `0.10.0-horizons-dogfood-lab`; the ladder, the engine's four new mechanisms, and adoption from the other end |
  | *Roadmap* | the delivered table ran to `0.10.0` and now runs to `0.23.2` |
  | *Limitations* | the markdown store **does** have a journal and a history since `0.19.0`; the contract gap is unchanged and says so |
  | *Vocabulary* | artifact kinds and statuses are open to authors, and why `evidence_kinds` is closed |
  | `/releases` | **one post per release — all 33 tags, plus this one.** Three existed; 30 were written for this release |

- **A release post says which release it is.** Each carries `release_tag` and `release_commit` in
  its front matter, and its filename carries the hour and minute, so `website/blog/` sorts in
  release order in an editor and under `ls` alike — several releases were cut on the same day, and a
  date-only prefix put them in alphabetical order instead. The timestamp is the **tagged commit's**
  committer date rather than the tag's own: the first three tags were cut in one batch and share a
  creation timestamp to the second, which sorted two releases wrong. File modification times are set
  to match.

- **Every release now has a page explaining, in prose, what changed from the one before it.** The
  site had three posts covering `0.5.0`, `0.6.0` and `0.7.0`, and thirty tags with nothing. The
  backported posts are **marked as written retrospectively** and carry no command output: the code
  has moved on, and inventing a transcript for a released version would be worse than describing it.
  Posts for `0.13.0` onward carry real output.

- **Counts that had drifted, each re-derived from the thing it describes**: top-level CLI verbs
  17 → **20**; artifact lifecycles 8 → **12**; this repository's own plan 59 → **101** artifacts;
  the document tree 45 → **49** files; the gate ten → **twelve** steps, in `AGENTS.md`, `README.md`,
  `docs/status.md`, `docs/guide/adopting.md` and four website pages. `AGENTS.md` also still claimed
  the billing suite runs 27 scenarios where its own guard asserts 29.

- **The boundary with `metaharness` is written down rather than remembered.** Two rules, because
  both were being reconstructed differently each time they came up: harness-specific transcript
  readers belong there and the normalised `metaharness.event/1` stream is the production path here
  (the two direct vendor readers that predate it are named, with what each is for and what it is
  checked against); and the evaluation *machinery and corpus* are here while the *paid runs and
  their results* are there — a finer split than "evals moved", and the table now says which is
  which.

## [0.23.2] — 2026-08-26

### Fixed

- **CI is green again.** It had been red for **eleven consecutive releases** — every tag from
  0.13.0 to 0.23.1 — with a locally green gate the whole time.

  Two independent causes, and both were in the two jobs `task check` did not run:

  | job | cause |
  |---|---|
  | `MSRV 1.85` | `idna_adapter@1.2.2` pulled in `icu_*@2.3.0`, which need rustc **1.88** |
  | `Website` | a markdown link from `website/docs/` into the repository tree, which Docusaurus resolves at build time |

  The MSRV break came in **through the lockfile**, with no commit of ours touching a line of Rust —
  a transitive dependency raised *its* `rust-version` and the declared 1.85 stopped holding.
  `idna_adapter` is pinned to 1.1.0, which uses `unicode-normalization` instead of `icu` and drops
  eight crates from the tree. The alternative was raising the MSRV, which would have quietly broken
  the promise `README.md:164` makes to anybody building this.

### Changed

- **`task check` now runs `msrv` and `website`.** Its own description said *"everything CI runs"*
  while those two were missing, which is how a red CI survived eleven releases behind a green local
  gate. A gate that covers less than the gate it claims to be is worse than one that admits its
  scope.

## [0.23.1] — 2026-08-26

### Changed

- `aep-backend-markdown` pins **entity-core 0.5.2** instead of 0.5.0. `entity-core`'s source is
  byte-identical between the two tags — 0.5.1 and 0.5.2 carried an examples-only change — so nothing
  this crate links moves. The pin recorded a tag two releases behind what exists, and a pin that is
  stale by label is a pin nobody trusts to be current by content either.

### Fixed

- `outbound-claim` starts at `draft`, not `drafted`. Every other shipped ladder starts at the
  built-in `draft`, and an invented rung one letter from a built-in is a typo wearing a vocabulary's
  clothes. The vocabulary is open to authors, not to near-misses.

## [0.23.0] — 2026-08-26

### Added

- **A claim that left the boundary is now a thing this repository can model.** Gap-register `:70`,
  closed — and it is the proof the whole entity-runtime programme was for: **a YAML file, and no
  Rust change at all.**

  Every other ladder here models evidence flowing *inward*. An outbound claim runs the other way: a
  number in a customer's inbox, a status page saying "resolved", an availability figure in a renewal
  deck.

  ```console
  $ protocol artifact new outbound-claim q3-uptime --title "Q3 uptime figure to Acme"
  created outbound-claim:q3-uptime (drafted)

  $ protocol artifact move outbound-claim:q3-uptime --to cleared
  cleared is on the ladder and not yet earned: reaching cleared needs at least 1 approval record(s)
  ```

  **The property the ladder exists to enforce: sending is not undoable.**

  ```console
  $ protocol artifact move outbound-claim:q3-uptime --to drafted
  outbound-claim:q3-uptime is sent; an outbound-claim may move to: correction-owed, standing
  ```

  A ladder that let `sent` return to `drafted` would model retraction as an **edit** — the claim
  would simply stop having been made — and the customer would still have the email. So a wrong claim
  moves forward only. `correction-owed` (*sent, known wrong, audience not yet told* — the most
  expensive state an organisation can be in and the easiest to leave undocumented) is a **rung**,
  non-terminal and never `is_approved`, so it shows up in a `board` column instead of a thread.

  **`corrected` requires two approvals, and that number is a finding.** Written as one, it passed
  instantly: the approval that cleared the *original* claim was still on the record, because evidence
  is append-only. The claim would have been corrected on the strength of somebody approving the thing
  that was wrong.

  Two is the exact cumulative count — one to send it, one to correct it — not a safety margin. The
  general limitation is written into the ladder where somebody will meet it: `requires:` counts every
  matching record ever made, and there is no way today to say *an approval recorded since this claim
  entered `correction-owed`*.

  Every mechanism from this programme composing at once — journal, provenance, evidence, open
  vocabularies, ladder-as-data:

  ```console
  $ protocol artifact history outbound-claim:uptime
  … created as drafted (revision 1)
  … approval recorded from Priya, VP Eng (revision 1)
  … moved drafted -> cleared (revision 2)
  … moved cleared -> sent (revision 3)
  … moved sent -> correction-owed (revision 4)
  … approval recorded from Priya, VP Eng — correction text (revision 4)
  … moved correction-owed -> corrected (revision 5)
  ```

  No `retracted` rung: a retraction is itself an outbound claim, because you have to tell somebody.
  No `expired`: a claim does not become false by ageing — the world moves and a person notices, which
  is `correction-owed`.

## [0.22.0] — 2026-08-26

### Added

- **The harness-neutrality claim has met a second harness.** Gap-register `:38`, `partial` tier.

  `trace_spec::codex` reads codex-cli's session rollout JSONL into the same `TraceIr` the Claude
  adapter produces, so one specification decides both. Correlation stays `TraceIr::new`'s job rather
  than being re-derived — two adapters correlating separately are two places for the pairing to
  disagree.

  **What the second adapter bought immediately** — three places where an empty value would have been
  a *claim* rather than a reading, invisible while only one harness existed:

  | field | reads | because the empty value would say |
  |---|---|---|
  | `tools` | `None` | *no tool was available* — which nobody observed |
  | `operations` | empty | mapping `apply_patch` → `file.write` is a rendering's question, answered invisibly in Rust |
  | `is_error` | `None` | *the call succeeded* — the adapter asserting, not reading |

  Each is now pinned by a test. This is the whole argument for a second implementation: a vocabulary
  tested against one harness is shaped like that harness, and nobody can tell which from inside.

  **A verified finding about the format, found on the way.** Reading 400 real rollouts from a local
  codex-cli 0.145.0 install: **374 read, 26 refused** — and in **23** of those a record began
  *mid-line*, two JSON objects concatenated with no newline between them. A torn append, not
  truncation: only 3 of the 26 were at end-of-file.

  The reader refuses those by line number rather than recovering. A reader that guesses where a
  record starts produces records nobody can trust, and this reader's entire job is to be the thing a
  verdict rests on. This is the substance the *refused, with a reason* tier was written for —
  **format instability, exactly as the research predicted, and not an enforcement gap**.

  **Not done: the `full` tier** — one live `llm` step run under Codex and decided against the same
  specification file. That costs money and needs a person at the keyboard, and the gate reaches no
  network.

  The committed fixture is **synthetic**, written in the verified format rather than copied from a
  session, and the test says so. Real rollouts are somebody's actual working transcripts; a fixture
  whose provenance is unstated is a fixture nobody can weigh.

### Fixed

- **`ess impact` no longer reports more owed artifacts than exist.** Gap-register `:46`.

  ```console
  $ protocol ess impact --from ... --to ... --generated generated/
  -105 of 23 generated artifact(s) owed regeneration
  +23 of 23 generated artifact(s) owed regeneration
  +82 committed file(s) the `--from` model derives nothing for, and this analysis cannot follow
  ```

  105 = 23 + 82, exactly. Files the `--from` model derives nothing for were going into the
  numerator and never into the denominator. **An unfollowed file is not an artifact that owes
  regeneration** — it is a file the analysis cannot speak about — so it belongs to neither side.

  They are still counted and printed, on their own line. Trading an impossible number for a missing
  one is not a fix: *82 files this cannot follow* is a real finding about the analysis.

  The row recorded `56 of 38`; against the committed tree today it was `105 of 23`. Reproducing
  before fixing was what showed it had grown.

- **The `horizon` schema publishes what the parser accepts.** Gap-register `:45`.

  The generated schemas said `"type": "integer"` while the parser accepts `7d` — and the error
  message a wrong value earns *recommends* `7d`. **An editor validating against the schema
  false-flagged the spelling the tool itself tells you to write**, which is worse than no schema: it
  makes a correct document look wrong.

  Now a two-type union — an integer in `1..=3650`, or a string matching the parser's own tolerance.

  Writing the pattern found a second defect **in the other direction**: the first version admitted
  `0`, which the parser refuses. The schema would have published a value the tool then rejects — the
  same failure, reversed. The test caught it before it shipped.

  One looseness, stated rather than hidden: a regex cannot bound a magnitude, so the string branch
  admits `9999d` where the parser refuses anything over 3650 days. The integer branch carries the
  bound exactly.

- **Three prose literals now match the counts their own gates print.** Gap-register `:47`.

  | said | actually | how it is held now |
  |---|---|---|
  | `27 of 27 scenarios` | **29** | a test reads the count out of the suite's own assertion |
  | `17 claims held` | **21** | counted at run time; the literal is gone |
  | `nine scenarios` | **ten** | re-derived from the command the README quotes |

  The `27` had been copied into a second place — a doc comment the gap row did not name — which is
  what a wrong number does if you leave it alone. Correcting both copies without tying them together
  would only have reset the clock, so the scenario figure is now derived from
  `tests/conformance.rs`'s assertion and fails with it.

  The smoke test's count cannot drift again at all: `check()` increments a counter and the summary
  prints what was actually checked.

## [0.21.0] — 2026-08-26

### Added

- **A task can say what it is about, and evidence about anything else is refused.** Gap-register
  `:72`, closed.

  **The failure this is named after:** an end-to-end job held a legacy service while a deployment
  rolled its successor, and produced *weeks of green* about a component nobody was shipping. Every
  record was true. Every record was about the wrong thing, and nothing in the loop could say so.

  ```yaml
  # a task document
  id: T-1
  kind: feature
  subject: service:auth-api      # new: what this task is about
  ```

  ```text
  this evidence is about `service:auth-api-legacy`, and T-1 is about `service:auth-api`;
  a fact observed of one thing does not move another
  ```

  Both names are printed, because the difference between them is the entire content of the finding
  and a message naming one of them is one the reader has to go and complete.

  Refused **before the record is built**, not filtered afterwards: a fact about the wrong thing that
  is stored and later filtered is still a fact that anything forgetting to filter will read.

  Declaring a subject turns the guard on **fully** — evidence naming *no* subject is refused too.
  There is deliberately no half state, because admitting unsubjected evidence would leave omitting
  the subject as the way around a guard that looked mandatory.

  **A task that declares no subject is unchanged**, and that is asserted rather than assumed
  (`a_task_that_declares_no_subject_is_unchanged`): a task that has not said what it is about cannot
  be the judge of whether a fact is about it.

  `task.subject` is a readable fact, so a rule may condition on it without the engine hard-coding
  what any particular subject means.

  C3 (a test naming no revision of the environment it observed), C4 (no determinism model for
  verifiers) and C5 (a verifier's own coverage is not a fact) are the same family and are **not**
  taken here. They stay named in the register rather than quietly folded into this.

- **A check can now be introduced gently instead of not at all.** Gap-register `:71`, closed.

  There was one enforcement level: a check blocks or it is deleted. No state for *not ready to block
  yet* — so a check that would be noisy on day one simply never got written.

  ```yaml
  requires:
    predicates:
      - tests.unit.failed == 0     # blocks
    advisory:
      - owner: platform-team
        exit_criterion: when the flaky runner is replaced
        require:
          predicates:
            - coverage.line >= 90  # checked, reported, counted — does not block
  ```

  ```text
  ✗ coverage.line >= 90 [advisory — platform-team until when the flaky runner is replaced]
  ```

  **`owner` and `exit_criterion` are required to write one down at all**, refused at parse time.
  An advisory gate with no route back to blocking is a muted gate with better manners: nobody is on
  the hook, nothing says when it stops being advisory, and a permanent warning becomes scenery. A
  default owner is nobody and a default exit criterion is never, so neither is defaulted.

  The row is **evaluated for real**, not skipped — a check that never runs cannot tell anybody its
  exit criterion has been met. `RequirementReport::advisory_gaps()` counts the failures;
  `unmet()` deliberately excludes them, because folding them in would make the tier
  indistinguishable from blocking at every call site that asks what is outstanding.

  This generalises the trace checker's `--advisory`, which had the same three properties in one
  place: the downgrade moves the exit code, the record names every downgraded id, and the underlying
  fact ignores the flag.

- **A dependency that keeps failing stops being called.** Gap-register `:78`, circuit-break half.

  ```yaml
  - kind: command
    run: [deploy, --to, staging]
    retries: 5
    depends_on: staging-cluster
    circuit_breaker: 2        # after 2 failures of staging-cluster, stop attempting it
  ```

  **What this saves a person:** a run against a service that has been down since the first state no
  longer spends five retries per step producing five indistinguishable timeout lines. The first two
  failures are real information; the rest is noise that buries the run's actual history.

  Keyed by **dependency name, not by step** — which is the entire difference from the retry budget
  that already existed. Retry bounds *this step keeps crashing* and resets when the run moves on. A
  breaker bounds *this dependency keeps failing*, does not reset, and is shared: two steps calling
  the same service share its fate. The test asserts exactly that — the **second** step is skipped
  because the **first** step's dependency failed.

  A skipped step is recorded as **skipped, never as failed**. A step nobody ran produced no
  observation, and recording a failure for it would fabricate one — the same rule the driver already
  applies to a step that crashed. The difference a reader needs is between *we looked and it was
  broken* and *we stopped looking*.

  A breaker with no `depends_on`, an empty dependency name, or a threshold of `0` is **refused at
  load**. A map whose author believes it has a circuit breaker and does not is worse than one with
  none, because the belief is what stops them adding a real one.

  The **retry** half of `:78` turned out to be closed by code already and simply unrecorded; the
  register now says so. The third half — a dependency declared as *simulated* against a named ESS
  specification, so a workflow touching a third party has an offline form — is a real build and is
  **not** done here.

- **A run that did something its state was not allowed to do now fails a check.** Gap-register
  `:40`, by the weaker of the two routes the register named — and it says so.

  `protocol drive` writes, per `llm` step, a `trace-spec/1` document beside the step's frame with one
  `tool.absent` row for every operation the state's tool set did **not** admit:

  ```json
  { "id": "refused-file-write",
    "severity": "gate", "on_unknown": "gap",
    "expect": { "tool.absent": { "operations": ["file.write"] } } }
  ```

  Keyed by the **neutral operations** vocabulary, never by a vendor's tool names: a row saying
  `tools: [Edit, Write]` selects nothing at all on a harness that spells a write `workspace_write`,
  and reports green for it.

  The refused set is the **complement computed from the one existing table**, not a second
  hand-written list. A hand-written vocabulary missing `file.edit` would emit a specification that
  never checks for an edit and reports green — so a test asserts admitted and refused partition the
  vocabulary exactly.

  `on_unknown: gap`, because a transcript that cannot say whether a refused operation happened is not
  evidence that it did not. The point is to stop reading silence as compliance.

  A test reads the emitted document back through `trace_domain::raw::read_spec` — the same door the
  CLI uses — so this cannot become a file nobody consumes that looks like an audit. It caught three
  real shape errors while being written.

  A state that refuses nothing writes **no** document, which is `trace-spec/1`'s own rule: *a report
  with no content reads exactly like a report with no gaps*. Absence stays readable because the frame
  is written unconditionally — a frame with no refusal file means everything was admitted; no frame
  means the step never ran.

  **This is strictly weaker than the audit `:40` originally promised, and the register said so before
  it was built.** It catches a tool that was offered *and used*; it cannot see one offered and never
  reached for. The stronger route — a harness-side record of the effective allowlist — is still not
  this repository's to build.

## [0.20.0] — 2026-08-26

### Added

- **Evidence names what it is about, so `implemented` stops being a number somebody typed.**
  `protocol artifact evidence <id> --kind test_result --source 'task check' --ref <url>` records an
  observation *about* an artifact. `move` finds it; nobody has to assert it:

  ```console
  $ protocol artifact evidence story:passkey-login --kind test_result --source "task check"
  story:passkey-login: test_result recorded from task check
    on hand: test_result=1

  $ protocol artifact move story:passkey-login --to implemented
  story:passkey-login moved active -> implemented (revision 4)
  ```

  **The thing this fixes for a real person:** before today, anyone could satisfy *"implemented needs
  a test result"* by typing `--evidence test_result=1`, and a reader of the plan six months later had
  no way to tell that story apart from one whose tests actually ran. Now they can, always — evidence
  recorded against `story:one` is worth nothing to `story:two`
  (`tests/journal.rs::evidence_counts_only_for_the_artifact_it_names`), and a move that leaned on a
  typed number is **labelled as such** in the output and in the history:

  ```console
  $ protocol artifact move story:other --to implemented --evidence test_result=1
  story:other moved active -> implemented (revision 4)
    decided partly on asserted evidence nothing checks: test_result=1

  $ protocol artifact history story:other
  2026-08-26T09:14:02Z  operator  moved active -> implemented (on asserted evidence) (revision 4)
  ```

  `--evidence` is **kept on purpose**. A CI run nobody recorded is real evidence, and refusing it
  would only push people to record a fiction to get past the gate — which is worse than an honest
  assertion, because a fiction is indistinguishable from a record. So both are accepted, counted
  apart, and both written down. The claim is not that every move is proven. It is that **no move can
  be mistaken for proven**.

  Recording and moving are **separate commands**, deliberately. One command that recorded evidence
  and then moved the artifact would make the evidence a formality of the move rather than a thing
  that existed before it.

  **This closes the provenance half of gap-register `:39`**, whose mechanism half closed in 0.18.0.
  What stays open is the engine's judgement — whether a producer was *independent* of what it
  reports on — which is `:72`/`:80` and a different row.

### Changed

- `journal`'s `moved` entry carries `decided_on`, the split of what the move rested on. Entries
  written by 0.19.0 have no such field and read back as an **empty** account, which is the honest
  reading of them: *nothing was recorded about how this was decided*. Defaulting the other way would
  have made every historical move claim it was evidence-backed
  (`tests/journal.rs::a_move_written_before_provenance_existed_claims_nothing`).

## [0.19.0] — 2026-08-25

### Added

- **The planning store has a journal, and therefore a history.** Every write — create, move, relate,
  body — appends an entry recording who, when, which artifact, which revision and what changed.
  `protocol artifact history <id>` reads it back:

  ```console
  $ protocol artifact history story:journalled
  2026-08-25T21:52:32Z  operator  created as draft (revision 1)
  2026-08-25T21:52:32Z  operator  moved draft -> proposed (revision 2)
  2026-08-25T21:52:32Z  operator  moved proposed -> active (revision 3)
  ```

  **Why not git.** This crate's own description says *git as the log*, and git is a fine log for a
  person reading diffs. It is a poor one for a tool: a rename is a guess, a squash loses the moves,
  a rebase rewrites the times, and none of it answers *which of these was a status move* without
  parsing markdown out of a patch. The journal records the change the store actually made, in the
  shape the protocol reasons about.

  **Append-only**, which is invariant 16 applied to the record of what was done: a mistake is
  corrected by a later entry, never by editing an earlier one. A line that will not parse is skipped
  rather than fatal — one half-written entry from a killed process must not make a year of history
  unreadable — and the count of skipped lines is *printed*, because a shorter history reported as if
  it were complete is the quiet failure this whole file exists against.

  A source scan asserts that every verb which writes also records, so a verb added next year that
  forgets is a visible omission rather than a hole in the history. It is checked against a planted
  unrecorded write, because a guard nobody has broken on purpose is a guard nobody knows works.

  **This closes one third of gap-register `:37`.** The row names three absences — a journal, an
  audit join, and a history — and this is the journal and the history. It is **not**
  `CommandService`: that needs command envelopes, idempotent replay and revision conflicts, and it
  has an architectural question inside it, because the contract's `execute` is async and this store
  is synchronous file IO. That is worth deciding deliberately rather than in passing, so the row
  stays open and now says which third is closed.

## [0.18.0] — 2026-08-25

### Added

- **`blocker` — what is stopping something, typed by what would clear it.** Gap-register `:73`'s
  third and last mechanism. The `blocks` edge already existed; the *type* did not, and the type is
  what turns five stuck items into one conversation: *parked on a credential* and *parked on a
  person* look identical in a backlog and are not the same problem.

  **The type is the kind, not a field.** `credential-blocker`, `person-blocker` and
  `decision-blocker` all resolve to one ladder through `ArtifactKind::parent`, which reads the last
  hyphen segment — so a team's own category of blocker costs a name and nothing else. No document
  per type, no enum, no release. That machinery is what gap-register `:77` was about, and this is
  the first thing to spend it.

  `cleared` is terminal on purpose: something blocked again is a new blocker with its own date, not
  the old one reopened, or *how long were we stuck* has no answer.

### Changed

- **The relation vocabulary stays closed, and now says why.** `story:ova-relation-vocabulary` was
  opened because the audit found a closed row whose guarantee cell read `none`. The decision is to
  keep it closed, and the guarantee is real: a relation name is something the engine *acts on*, not
  only records. `supersedes` gates a status, `reviews` is mandatory on a review-result, and cycles
  are checked once per relation kind. None of that can move into a document — which is exactly the
  difference from artifact status, where the guarantee could move to the ladder and did. An open
  relation vocabulary would let you write an edge that looks like it should mean something and that
  no rule will ever read.

### Fixed

- **`delivers` was in the binary and in no row of `artifacts/relations/relations.yaml`.** Thirteen
  relations in the type, twelve in the document, so the engine accepted an edge that file said did
  not exist. It has a row now. What remains genuinely advisory is the `source`/`target` pairings —
  the file says so in its own header, nothing in `crates/` reads them, and that is a different gap
  this decision does not close.

## [0.17.0] — 2026-08-25

### Added

- **`obligation` — a commitment on a clock nobody here controls.** Gap-register `:74`. A filing due
  on a date this repository does not set, satisfied by a person, which must never block a commit
  because blocking a commit cannot close one.

  `artifacts/lifecycles/obligation.yaml`: `open → met | slipped`, `slipped → met`, `met` terminal.
  `slipped` is dated (`when: { slipped: { after: due } }`), so *overdue* is a fact about the calendar
  rather than somebody's opinion on a Monday. **A slipped obligation can still be met** — escalation
  is not an ending, and that is the shape `:74` asked for.

  ```console
  $ protocol artifact move --to slipped obligation:annual-filing --at 2026-09-01
  obligation:annual-filing is open; slipped is on the ladder and not yet earned: slipped is not
  reachable until this artifact's due has passed

  $ protocol artifact move --to slipped obligation:annual-filing --at 2026-10-01
  obligation:annual-filing moved open -> slipped (revision 2)
  ```

  **No Rust changed.** `obligation`, `open`, `met` and `slipped` are all names no enum holds — the
  kind vocabulary has been open since it was written and the status vocabulary since 0.13.0. A whole
  artifact kind, its ladder and a dated rung, in two documents. That is what the open-vocabulary
  work was for, and this is the first time it has been spent.

  It is a **kind of its own and not a rung on somebody else's**. Widening `story` to hold `slipped`
  would have put a date nobody controls in the path of work everybody does. A test asserts that no
  ladder can wait on an obligation — structurally true, because a requirement names an evidence kind
  and a guard names a frontmatter key, and neither can name another artifact. If a future field
  makes it expressible, that test fails and the argument gets made again on purpose.

### Fixed

- A refusal about a kind beginning with a vowel read "a obligation". The kind vocabulary is open, so
  a refusal a person reads should not be where they notice it was widened.

## [0.16.0] — 2026-08-25

### Added

- **A lifecycle document may declare when a rung opens.** `when:` beside `requires:` names a date
  the *artifact itself* records, and the rung is shut until it has passed:

  ```yaml
  when:
    superseded:
      after: expires_at
  ```

  Gap-register `:73`'s second concept — *time-based transitions of any kind, which today live in
  scripts `explain` cannot see*. One now lives in the document that governs the rung.

  ```console
  $ protocol artifact move --to superseded adr:dated --at 2026-08-25
  adr:dated is accepted; superseded is on the ladder and not yet earned: superseded is not
  reachable until this artifact's expires_at has passed

  $ protocol artifact move --to superseded adr:dated --at 2026-09-02
  adr:dated moved accepted -> superseded (revision 2)
  ```

  **The clock is read at the edge and nowhere else.** `--at` defaults to the system clock, read in
  `protocol-cli`; `aep-domain` has a banned-token scan that would refuse one, and a decision that
  read the clock itself could not be replayed. The instant is an *argument* to the move.

  **Three refusals, not two.** *Nobody said when* is unobservable and names `$args.now`; an artifact
  that records no such date names `$fields.<key>`; an instant nobody can read — `yesterday`, or one
  carrying a `+02:00` offset — is unobservable too. Only *the date has not passed* is a plain
  refusal. `after` is strict, so the instant itself does not open the rung.

  The date comes from the artifact's own frontmatter, not from the caller: an `expires_at` is a fact
  the artifact records, and letting a mover supply one would make the guard something they choose.
  Frontmatter keys this format does not name are already preserved verbatim, so no schema change was
  needed to carry one.

### Changed

- `Verdict` and `MoveRefusal`'s `RefusalReason` rename `EvidenceUnobserved`/`EvidenceInsufficient`
  to `Unobservable`/`NotEarned`. Evidence is no longer the only thing a rung can cost, and a name
  that says *evidence* for a refusal about a date would be a name that lies. Breaking for library
  callers; `move_status` also takes the instant to judge a dated rung against.
- `entity-core` moves to release tag **0.5.0**, which is where `before`/`after` live. Nothing above
  is possible without them.

## [0.15.0] — 2026-08-25

### Added

- **A lifecycle document may declare what a rung costs, and `protocol artifact move` checks it.**
  `requires:` beside `transitions:` says which evidence a status needs and how much of it:

  ```yaml
  requires:
    implemented:
      - evidence: test_result
        at_least: 1
  ```

  `artifacts/lifecycles/story.yaml` carries the first real one. Every other rung on every ladder is
  still free, and a test records exactly which rungs cost what, so a rung gaining or losing a cost
  is seen there rather than discovered by somebody's move failing.

  The move is decided by `entity-core` against evidence the caller presents, and there are **three**
  outcomes rather than two. That split is the whole of gap-register `:39`:

  ```console
  $ protocol artifact move --to implemented story:x
  story:x is active; implemented is on the ladder and not yet earned: reaching implemented needs at
  least 1 test_result record(s). Nothing was presented at $args.evidence.test_result

  $ protocol artifact move --to implemented story:x --evidence test_result=0
  story:x is active; implemented is on the ladder and not yet earned: reaching implemented needs at
  least 1 test_result record(s)

  $ protocol artifact move --to implemented story:x --evidence test_result=2
  story:x moved active -> implemented (revision 4)
  ```

  The first sends an author to produce a record; the second to argue about the one that exists. A
  store reporting both as "refused" is the prose rule the register complains about, wearing a type.

  **What this closes and what it does not.** It closes the *mechanism* half of `:39` — a status can
  now cost something, declared in a document rather than in Rust. It does **not** establish
  provenance: `--evidence test_result=2` says a count was presented, not that the records are sound,
  about this artifact, or produced independently. Those are the engine's judgements and they are
  `story:completion-needs-evidence`. The planning store holds markdown, not evidence records, so
  what is on hand comes from the caller — the same shape the kernel already demands of a clock.

  `StatusRequirement` is deliberately smaller than `EvidenceRequirement`: a ladder declaring
  `independent: true` without the engine evaluating it would be a document making a promise nothing
  keeps. The requirement sits beside the rung it guards rather than in the artifact-kind document,
  because the refusal a person reads names a rung.

  Breaking for library callers: `PlanningDocument::move_status` takes the evidence on hand and
  returns `Box<MoveRefusal>`; `MoveRefusal` carries a `reason`.

- `entity-core` moves to release tag **0.4.0**, which adds typed references between entities and a
  crate that draws them. Nothing here uses either yet; the bump keeps the gate exercising the kernel
  this repository actually depends on rather than one release behind it.

## [0.14.0] — 2026-08-25

### Added

- **`protocol reverse` — the verbs for a repository that already exists.** Every other verb starts
  from a document somebody wrote; these start from a codebase, which is the state an adopter is
  actually in.

  - **`reverse init`** writes `.engineering/project.yaml`. It refuses an unpinned `git+` source and
    an absolute path before writing anything, resolves the tree so the failure surfaces at adoption rather than at the
    first command that needed it, and leaves no file behind when it does not resolve. `--no-verify`
    skips the resolution for an offline machine. This replaces hand-authoring the file from
    `docs/guide/adopting.md`.
  - **`reverse scan`** reads a repository and emits `aep.reverse-scan/1`: README headings, marked
    lines, CI jobs and the variables on them, task-runner targets, packages by language, published
    interface documents, and loose root markdown — each entry carrying the `path:line` it was read
    from. It writes nothing, interprets nothing, and has no clock, no network and no `read_dir`
    order dependence, so **two runs over one tree produce identical bytes**. Interpreting the bundle
    is judgement and stays with whoever is planning; producing it is not, and is now a program.
  - **`reverse openapi`** drafts an `ess/1` domain from an OpenAPI document — types, errors, commands
    and their inputs. Everything it can see and cannot decide (entities, lifecycles, invariants,
    actors) is emitted as an `UNMAPPED:` comment naming the choice, because a reader cannot tell an
    absent lifecycle from an absent decision about one.

- **`protocol reverse history` — the axis a working tree does not have.** `reverse scan` reads the
  tree as it stands, so it can report that a suite is switched off and never that it has been off
  since February 2024. Only one of those is a finding somebody acts on.

  It reports, all derived from the commits reachable at `HEAD`: **`line_ages`** — every marked line
  and every disabled test, dated from the commit that wrote it and ordered oldest first, keyed by
  `path:line` so it joins straight onto a scan bundle; **`stated_expiry`** — commits whose message
  said *for now*, *until we*, *temporarily* or *workaround*, each a decision with an implied expiry
  and nothing enforcing it; **`reverted`** — what somebody already tried to undo; **`churn`** and
  **`dormant`** — where the work keeps landing and where nothing recent has; **`tickets`** — the
  `ABC-123` keys the messages carry, which is how a repository tells you it changed tracker two years
  ago; plus the commit-type mix, the span, and the tags.

  **Nothing reads a clock.** Dates are quoted from the commits that carry them and never compared
  against today, so a fixed `HEAD` gives fixed bytes and a committed bundle stays true. Author counts
  are counts: a bundle meant to be committed and pasted into a ticket does not carry names. A tree
  that is not a Git working tree gets one sentence saying so rather than nine empty sections that
  read like nine findings, and a shallow clone is warned about on stderr because its oldest dates are
  wrong.

- **`disabled_tests` in a scan.** Tests that declare they will not run — `t.Skip`, `#[ignore]`,
  `it.skip`, `@pytest.mark.skip`, `@Disabled` and the rest — with the reason they state and, the part
  that matters, **whether anything can still turn them on**. A guarded skip is an opt-in and appears
  in every healthy repository; an unguarded one is a test that runs on no machine, and a green
  pipeline reports the two identically. Found by reading the tree, not the history — the history is
  what pointed at the gap.

- **`artifacts/lifecycles/vision.yaml`.** `vision` was a declared kind with no ladder, so it resolved
  to the permissive fallback and `protocol artifact move vision:… --to implemented` succeeded — the
  first artifact an adopting repository writes was the one nothing validated. The ladder is
  `specification`'s with `implemented` removed: a vision is never implemented, the work under it is,
  and `superseded` is the only terminal a standing statement has.

  `product-requirements` has the same hole and deliberately does **not** get a ladder here, because
  opening the status vocabulary in 0.13.0 made declaring one a larger act than it was the week
  before. While every kind fell back to the permissive ladder, a missing lifecycle document shrugged
  at whatever an adopter's externally-tracked artifact said. Now that a status is accepted for a
  write only if the kind's lifecycle declares it, **writing that document decides what an adopter's
  `prd:*` is allowed to say** — and two example manifests already carry one at `status: active` from
  an external provider (`examples/billing-conformance/artifacts.yaml:12`,
  `examples/development-passkeys/artifacts.yaml:12`). `vision` has no such adopter surface, which is
  why it ships and this does not. The ladder is owed a decision, not a gap-fill.

- **`integrations/claude-code/agents/reverse-engineer.md`.** Drafts the first plan for a repository
  that has none: reads it through `reverse scan`, creates draft artifacts that each cite the
  `path:line` they rest on, and reports what it could *not* cite as questions rather than filing them
  as work. Creates drafts only; never moves an artifact.

### Changed

- **An absolute path in `.engineering/project.yaml` is refused.** `protocols:` was the one field
  exempt from a rule every other path in the file already followed, on the reading that where a tree
  sits is the adopter's business. It is — and that is not what the value decides. A project file is
  committed, so `protocols: /opt/aep` means a different thing on every machine that
  clones the repository and nothing at all in CI, where the failure arrives as a missing directory
  naming a path nobody on that machine wrote.

  **This is a breaking change** for a project file carrying one. Replace it with a path relative to
  `.engineering/`, or with a pinned `git+ssh://`, `git+https://` or `git+file://` locator — the
  locator is the right answer for anything other people clone. A `git+file:///srv/mirror.git#<sha>`
  is unaffected: it is absolute inside a URL, and it names a repository and a commit, so it resolves
  to the same tree wherever that repository is reachable.

  The check is in `ProtocolSource::parse` — the one reader every command goes through — so a file
  hand-edited past `reverse init` fails identically. It covers a leading `/`, `~`, a drive letter and
  a UNC path, and is decided by spelling rather than by `Path::is_absolute`, which answers for the
  platform it runs on: a Linux build used to accept `schemas: C:\registry` and a Windows one
  `schemas: /registry`. The sibling check for `artifacts`, `task`, `principles`, `profiles` and
  `schemas` now shares that one rule.

- `kernel_equivalence.rs` derives its pair count from the ladders it found instead of asserting
  `800`. The number was arithmetic about how many ladders shipped, so adding one failed the gate on
  the wrong thing; coverage is still held by
  `every_ladder_this_repository_ships_is_named_by_the_fixture`.

## [0.13.0] — 2026-08-25

### Changed

- **The artifact status vocabulary is open.** A lifecycle document may declare any rung — a status
  is no longer required to be a variant of `ArtifactStatus`. The rung an adopter asked for and could
  not have, `correction-owed` (sent, known wrong, audience not yet told), is now one line in
  `artifacts/lifecycles/<kind>.yaml` and no release of ours.

  **It is open to authors, not to typos.** A status is accepted for a *write* only if the kind's
  lifecycle declares it, so `protocol artifact move --to correction-owed` refuses with *"its
  lifecycle declares draft, proposed, rejected, active, implemented, archived"* until the rung is in
  the document, and works the moment it is. The guarantee the old closure bought — *a status name
  means the same rung to every tool* — is now bought by the ladder instead of by the type. Reading a
  status for a *filter* (`artifact list --status`) accepts any well-formed name, because asking for
  a status nothing holds is an empty answer rather than an error.

  Two things it deliberately does not do. An invented rung is never `is_approved` and never
  `is_retired`: this repository cannot know what a rung it has never seen means, and reading an
  unknown name as *agreed and relied on* is the one mistake an open vocabulary must not make. And a
  descriptor document under `adp-`/`aop-domain` still takes named statuses only, because it has no
  ladder in scope to check an invented one against — a deliberate closure, with its reason written
  beside it.

  **Breaking for library callers**: `ArtifactStatus` is no longer `Copy` (it carries a `String`, as
  `ArtifactKind` already did), `as_str` borrows, and `ArtifactLifecycle::permits`/`permits_transition`
  take references. The three generated schemas publish a string with a pattern and examples instead
  of a closed `oneOf`. Closes the vocabulary half of gap-register `:70` and the last instance of
  `:76`; `docs/guide/open-vocabulary.md` and `website/docs/status/limitations.md` carry the new
  verdict.

- **The status ladder is now decided as data, by `entity-core`, instead of by a lookup written
  here.** `protocol artifact move` refuses exactly what it refused before — every legal and illegal
  move of every artifact kind produces the same verdict, and
  `crates/aep-backend-markdown/tests/kernel_equivalence.rs` holds that over all 800 ordered pairs of
  the ten statuses, the permissive fallback and a custom kind resolving its ladder through its
  lineage. Nothing about the vocabulary opened, no message changed, and the refusal still names
  where an artifact may go instead.

  Why it matters even though nothing moved: `ArtifactStatus` is a ten-variant Rust enum, so every
  rung this protocol can express is fixed at compile time and an adopter who needs `correction-owed`
  cannot have it without a release of ours. That is the meta-defect `story:open-vocabulary-audit`
  opened, and gap register `:70` is it in the register. With the ladder evaluated as data, adding a
  rung becomes a line in a YAML file. This change takes none of that — it moves the decision so the
  later change is possible — and it is reversible by deleting one module and one manifest line.

  `crates/aep-backend-markdown` takes `entity-core` (`github.com/beyond10x/entity-runtime`) by git
  revision, the first dependency here pinned that way and the only one crossing to another
  beyond10x repository. The direction is fixed by `atlas/architecture/adr/0002`: nothing of ours
  enters a manifest of theirs, at any version, ever.

### Added

- **Projects now declare one custom JSON Schema registry, and `protocol` supplies the reusable
  contract tooling.** The relative `schemas` path in `.engineering/project.yaml` defaults to
  `.engineering/schemas`. `protocol schema validate` discovers that registry and validates JSON
  instances offline by matching their `schema` selector to a schema's absolute `$id`; `protocol
  schema typescript` selects by the same identity and produces a deterministic, drift-checkable
  TypeScript projection. Adopters no longer need repository-local validators or handwritten type
  copies around their JSON Schemas.

- **Planning bodies now have a store-aware mutation command.** `protocol artifact body <id> --from
  <path|->` replaces only the markdown body, preserves CLI-owned frontmatter, increments one
  revision on changed bytes, and does nothing for identical input.

### Changed

- **The planning store is now single-writer.** Plugin instructions and driven-run enforcement deny
  every direct `Edit`, `Write`, or `NotebookEdit` under `.engineering/planning/**`; creation,
  relations, lifecycle moves, and prose all cross the `protocol artifact` surface. Both Codex and
  Claude integrations also ship a `schema-contracts` skill for registry discovery, validation,
  projection, and drift checks.

## [0.12.0] — 2026-08-24

### Fixed

- **A validation error about a selector's subject no longer reads as "no subject was written".** A
  `subject:` that was written and could not be read left the selector usable, so a specification
  with a malformed subject was accepted and then decided nothing.

- **An expectation about writing a file no longer misses the write.** A `trace-spec/1` call selector
  took one tool name, so `tool: Write` was blind to a run that used `Edit` — and the first live pilot
  of the evaluation programme did exactly that, editing files that already existed. The rows that
  assert *the test was written before the code* came back `never_occurred`: the checker reporting
  that nothing happened about work visible in the run's own working tree. A selector now takes a
  **set** — `tools: [Edit, NotebookEdit, Write]`, the same three verbs `protocol drive` renders to
  `repository.write` — and `tool:` beside `tools:` is refused rather than resolved, as are an empty
  set, a blank name and a repeated one. Re-ingesting the recorded run: **3 ok / 2 gap / 4 unknown
  becomes 7 ok / 2 gap / 1 unknown**, with both ordering rows now deciding.

  The claims did not get weaker. Widening names what may *witness* an assertion, never what it
  asserts, and the tool scope is kept rather than dropped because `Read` carries a `file_path` too —
  *read the test first* is not *wrote the test first*. What a run must never do keeps a **narrower**
  set than what may witness it: the store guardrail forbids `Write` and `NotebookEdit` under
  `.engineering/planning/` and deliberately permits `Edit`, because a targeted body edit is what the
  planning skill asks for and a whole-file rewrite is what the CLI owns.

  For a Codex run these path-scoped rows answer `unk` and say so out loud: a Codex write crosses the
  seam as `apply_patch`, whose path lives inside a patch envelope under `command` rather than in a
  `file_path` argument. That is nobody-found-out rather than a failure, and it is not papered over by
  widening the set — which would make the row look cross-harness while still not deciding.

- **A run that priced itself is charged what it cost, not the estimate.** A live Claude run stated
  `total_cost_usd: 0.7977854999999999` and the eval runner's ledger printed
  `1 run(s), $0.250000 spent` — the `--assume-usd-per-run` rate — while its manifest carried no cost
  at all. What you hit: a paid sweep under-reporting what it spent, in the ledger *and* in the
  matrix's cost column, with the runs looking as though their wire had simply not priced them.
  Neither the ledger nor the field it read was at fault. `0.7977854999999999` is the shortest text
  that round-trips the `f64` sum of that run's per-turn costs, and the cost reader — written for
  amounts a **person** types, where refusing anything not exactly convertible is right — refused it
  for having more than six decimal places. That refusal was then thrown away with `.ok()`, which
  collapsed *there is a number here I cannot convert* into *there is no number*; only the second may
  be charged an estimate. Three things changed: a number a **wire** stated is now rounded half-up to
  the nearest millionth by integer arithmetic (never `value * 1_000_000.0`), so `0.7977854999999999`
  reads as `$0.797785`; a stated cost that still cannot be converted is **refused by name**
  (`EVAL-STREAM-011`) instead of becoming an assumption, and no manifest is written for it; and the
  runner now prints `charged:  $0.797785 (stated)` per run, because a ledger that does not say which
  of its two numbers it used is one nobody can audit. `--budget-usd` and `--assume-usd-per-run` are
  unchanged and still refuse anything inexact — a person typing `1e-7` has made a mistake worth
  naming. No committed bytes move: every fixture cost has six decimal places or fewer.

- **A run manifest can now say "the harness never named a model", and the plugin digest comes from
  the instrument rather than from the vendor.** The first live pilot run — Codex, arm a — refused
  twice, and both refusals were correct while both fields were wrong. That distinction is the entry:
  the boundary did its job and stopped a guess reaching the one document the matrix trusts; what it
  had been told to read was mistaken, and a recorded stream is what corrected it.
  - **`plugin_digest` is read from `session.started.hermetic.installed_plugins`**, not from the
    top-level `plugins` beside it. The two answer different questions: `plugins` is the **vendor's**
    own init list, echoed — Claude Code writes one and Codex writes `null`, because metaharness will
    not mint a vendor field it did not receive — and `installed_plugins` is the **instrument's**
    record of what it injected, written on every adapter. What changes for you: a Codex run with the
    plugin injected now assembles instead of being refused for attesting nothing, and a Claude run
    whose vendor echo is missing or edited assembles identically, because nothing reads that row any
    more. The refusal for a stream with no instrument row names `hermetic.installed_plugins`, so the
    next reader is told which of the two lists matters.
  - **`model` may be written as an explicit `null`.** Codex's wire names no model at
    `session.started` at all — a 62-event pilot run never states one — so `model` is now a written
    field on exactly `plugin_digest`'s terms: the key is **required**, an explicit `null` is legal
    and means *the harness did not say*, and a key nobody wrote is still refused, because a runner
    that dropped it would otherwise produce the same document. Inventing `gpt-5-codex` there because
    it is the likely answer would be writing down a model nobody observed. `protocol eval matrix`
    renders it as `"model": null` in JSON; its text rendering has no model column, and
    `protocol eval run` tells a person `model:    (unstated)`.
  - **Committed bytes moved, deliberately.** `crates/protocol-cli/fixtures/eval-run/dry-run.matrix.json`
    changes in five places and no others: four `transcript_digest` values, because all four ingested
    streams gained `hermetic.installed_plugins` (and `hermetic.decisions`, now that the live run has
    shown its spelling), and the Codex run's `"model"` going from `"gpt-5-codex"` to `null`. Every
    count in the matrix is unchanged. The two eval-corpus transcripts under `conformance/eval/` gained
    the instrument's row as well — an empty one for the run that had no plugin, and one without a
    digest for the run that predates the attestation, which is the honest fixture for *a plugin whose
    bytes nobody can name*.

### Added

- **A workflow step now declares the files it is given and the paths it may write, and the toolset
  refuses the rest.** Two keys on an `llm` step in `aep.driver-steps/1`. `context:` names files the
  run is handed rather than has to find; `scope:` is an ordered list of `paths:`/`write:` rules
  where `write:` is `allowed`, `partial-only` or `denied`, first match wins, and the last rule must
  name `**` so a path nobody thought of has an answer rather than a default.

  `partial-only` is the word that earns the shape, and it is the planning store's own rule: the CLI
  owns the frontmatter, so a body edit is legitimate and a whole-file rewrite re-types the
  frontmatter by hand. No set of operations can say that — `file.write` and `file.edit` are both
  writes — so the document speaks **granularity, not identity**, and which of an adapter's
  operations replace a file whole stays the adapter's own fact. A step-map author never writes an
  operation name.

  The rule existed already, in `crates/protocol-cli/src/drive.rs`'s `store_integrity`, written in
  one vendor's tool names — `Write`, `NotebookEdit`, `file_path` — so every arm but one walked past
  it for a year. Measured on `eval-case/development-default`, arm `native`, 2026-08-24: **10 of 11
  expectations held and one was contradicted before; 11 held after**, with discovery calls falling
  from ten of fifty-two to three of forty-nine. With the same scope bound but **unstated**, five
  calls were refused and the rule still held — the toolset stopped the run, rather than the prose
  warning it. `docs/reviews/2026-08-24-scope-cache-and-the-native-arm.md` records all four runs with
  their transcript digests. The operation is refused; the run continues.

- **One verb now runs an arm of an eval case, and it will not spend your money by accident.**
  `protocol eval run --case <dir> --arm raw|plugin|driven --harness claude|codex --out <dir>` drives
  `metaharness` as a **tool** — the way this repository drives `git` — and leaves the three documents
  a later reader needs: the raw event stream, the `trace-report/1` record its own checker produced,
  and the `eval.run-manifest/1` document `protocol eval matrix` pairs it with. What you hit if the
  binary is not installed: a refusal naming it and **exit 2**, its own code, so *install something*
  and *fix what you wrote* are distinguishable without reading prose off stderr. What you hit if you
  meant to spend money and did not say so: nothing spawns without `METAHARNESS_LIVE=1` **and**
  `--budget-usd`, and the cap is checked *before* each launch against `--assume-usd-per-run`
  (default `$0.25`) rather than after it, because a cap enforced afterwards is a receipt. A run whose
  wire prices it `null` — which is what a Codex stream does today — counts against the budget at the
  assumed rate **and** states no cost in its manifest: writing `0` would make it look free, and
  ignoring it would let an unpriced wire spend without limit. Amounts never go through a float:
  `0.0714 × 1_000_000.0` is `71399`, so the number's own decimal text is parsed digit by digit or the
  amount is refused. The three arms differ in exactly what the experiment says they differ in — arm
  `raw` gets the workflow's committed instruction document from `generated/instructions/` in front of
  the task, arm `plugin` gets the **task alone** plus `--plugin-dir integrations/<harness>` because
  the plugin *is* its treatment, and both are spawned `--decisions observe` into the same hermetic
  scratch home. **Arm `driven` is refused by name**: `protocol drive run` launches one, and this verb
  reads the stream it wrote (`--arm driven --stream <file>`), because a second way to launch a driven
  session would be a second policy to forget. `--stream` in general is the runner minus the spawn and
  spends nothing — it is also how a paid run is re-ingested after the manifest's rules change.
  `--observed-at` is **required** and deliberately not defaulted to now, unlike
  `protocol trace evidence`: a manifest is a committed document that has to assemble to the same
  bytes twice.

- **The run manifest is assembled from what the session already said, and metaharness gained no
  field for it.** The plan proposed emitting the manifest across the seam; it is refused, and the
  split is why. `harness_version` (as `claude 2.1.239` — two harnesses at `0.145.0` are not one pin),
  `model` and `plugin_digest` are read out of `session.started` — the digest from its
  `hermetic.installed_plugins` row, the instrument's rather than the vendor's — and
  `transcript_digest` is what the runner's own check states about the bytes it judged. `arm`, `workflow`, `case` and `observed_at`
  are the runner's, because nothing in a stream could know them — metaharness runs a *session*, and
  *this is arm b of case X* is a claim about an experiment. What changes for anybody handing the
  runner a stream: a `session.started` that does not state a field the manifest needs is **refused by
  name** and no manifest is written at all — `EVAL-STREAM-003` … `EVAL-STREAM-012`, twelve codes
  beside the sixteen the matrix already had. Three of them are about the experiment rather than the
  document: arm `plugin` over a stream attesting **no** plugin is the treated arm without its
  treatment (`-006`), arm `raw` over a stream attesting one is the control arm with it (`-007`), and
  a plugin attested with no `digest` cannot say which bytes were measured (`-008`). The digest the
  manifest carries is the attested string **verbatim** — never a hash of the plugin directory on
  disk, which would attest bytes the session never saw. `model` is read from the stream rather than
  from what the runner asked for, which is a narrowing to the plan's field list: a runner writing
  down the model it *requested* would record one the run may not have used.

- **The whole three-arm pipeline now runs in `task check`, for nothing.** Four committed streams —
  two harnesses, all three arms, one of them the eval corpus's own declared violation — go through
  the runner's ingest path and `protocol eval matrix`, and the matrix is asserted **byte for byte**
  against `crates/protocol-cli/fixtures/eval-run/dry-run.matrix.{json,txt}`: 34 facts held, 2
  contradicted, 4 runs. No vendor binary, no credential, no network and no spend, so a machine
  without `metaharness` runs the same green gate — the one test that would need it **skips by name**
  and says so. What this catches that nothing caught before: an edit to a case's expectations, its
  transcript, the manifest's field list or the runner's layout convention now fails with the row that
  moved. The streams are structurally faithful and **not observed** — a failure there is a change in
  this repository, never a finding about a model — and `crates/protocol-cli/fixtures/eval-run/README.md`
  states their derivation line by line, names the metaharness `c1-plugin-injection` vector this side
  corresponds to, and says plainly that until that side replays these exact bytes it is one
  implementation agreeing with a transcription of another.

- **Which plugin surface teaches which workflow is now a checked document, and the states nothing
  teaches are named rather than absent.** `integrations/workflow-coverage.yaml` maps every workflow
  under `workflows/` — keyed by the id the document declares, not by its filename — onto the skills
  and agents in `integrations/claude-code/` and `integrations/codex/` that teach each of its states,
  or onto a gap that says why none does. What changes for anybody reading it: **four of the nine
  states of the development workflow are taught by nothing either plugin carries**
  (`establish_verifiers`, `implement`, `verify`, `adversarial_verify`), `review` and `complete` are a
  second gap for a different reason, and `release/progressive`, `incident/standard` and
  `migration/forward-only` are uncovered end to end. All of that was true before and was written down
  nowhere. Coverage is **total** — `crates/protocol-cli/tests/workflow_coverage.rs` refuses a state
  that is neither covered nor gapped, a state claimed as both, a workflow with no entry, an entry for
  a workflow no document declares, a `document:` path that no longer holds the id beside it, a state
  name no workflow declares, and a `surface:` that is not a file under `integrations/` — each by
  name. Adding a workflow file without a map entry now fails the gate and names the workflow.

- **An eval case is three committed files, and the gate replays every one of them.**
  `conformance/eval/` holds one directory per case: a task statement, a `trace-spec/1` expectation
  document and a committed transcript. `crates/protocol-cli/tests/eval_corpus.rs` enumerates the
  directory rather than listing cases, so adding one costs three files and no code. Each case
  declares the verdict it must reach — `held`, or `violated` with the expectation ids it expects to
  be contradicted — and the check is pinned in **both** directions: a row that stops gapping is as
  red as a row that starts. `unk` is refused in every case of both kinds, because an undecidable row
  reads exactly like a passing one. Five cases ship: two over `workflows/development/default.yaml`
  (one honest, one that wrote the code before the test and contradicts two named rows), one over
  `workflows/releases/progressive.yaml`, and the two agent-charter cases from
  `specification:agent-charter-eval-cases`, whose transcript half this closes — that specification's
  own Out of Scope had left the documents held only by an offline mode nothing in `task check`
  invokes. Nothing is scored and nothing is model-judged: a case's output is a verdict per row with
  the events that produced it.

### Changed

- **The `development` workflow now carries that declaration.** Every `llm` step in
  `drivers/development/default.yaml` names `integrations/claude-code/skills/planning/SKILL.md` as
  context and the store scope as `scope:`. A preloaded file is replayed on every turn of a stateless
  loop, so it is not free — measured at about **$0.02 a run**, almost all of it cache-read, against
  the six discovery calls it replaces, each of which is a call, a turn *and* a result that then
  joins the same replay.

- **A contract run that reports a breaking change now stops a change entering review, and one that
  merely went red does not.** `principles/development/contract-testing.yaml` owes
  `contracts.breaking_changes == 0` **before the review phase** as well as before completion, so in
  `adp/default` the move `adversarial_verify -> review` is refused — by name, quoting the line — when
  the submitted `contract_result` reports a breaking change. The two counts are deliberately not
  interchangeable here: `failed` is *the contract run is red*, which is what a review is for, and
  `breaking_changes` is *a consumer was told something that is no longer true*, which nobody in a
  review is in a position to decide. It is also the first guard in that workflow that only a contract
  **runner** can answer — `tests.contract.failed` is an alias any test runner satisfies with a suite
  it happened to name `contract`, and `contracts.breaking_changes` has exactly one producer — so a run
  that never heard from one does not pass it by saying nothing: the count is unobserved, and
  unobserved is not zero. What is unchanged: the rule is scoped by the principle's `applies_when`, so
  a task declaring `change.code: false` owes nothing here, and no profile, workflow or step map moved.
  Until now nothing anywhere gated on a `contract_result`, which
  `story:contract-result-ingestion` said in its own *Out of Scope*.
  Acceptance: `story:contract-result-gates`.

- **`protocol contract evidence` refuses a record that does not state `checked`, `failed` or
  `breaking_changes`, instead of reading the omission as zero.** The payload gives each count a
  default, so a runner that renamed the field or stopped emitting it used to go on reading green while
  saying nothing at all — and zero on `breaking_changes` is exactly the claim the gate above reads as
  a pass. Both spellings of *nothing* are refused by the name of the count: absent and `null`. A
  record that states its counts, whatever they say, is minted as before.
- **`conformance/README.md` now lists all five of its tenants.** `trace/` had been an undocumented
  directory there since the migration, and `eval/` joins it. The table also says what the two have in
  common with `fixtures/`, `scenarios/` and `expected/` and what they do not: the first three judge a
  backend against the command and query contract, the last two judge an agent run against an authored
  expectation document.

- **`protocol drive run` and `protocol drive resume` refuse at launch when `metaharness` is not
  installed, instead of finding out at the first model step.** A map with an `llm` step needs the
  binary — that is how a session is spawned — and without it the run used to allocate a directory,
  take the store lock and then report *no verdict* for a step that never ran. The refusal names the
  one command that fixes it (`cargo install --path crates/metaharness-cli` from a metaharness
  checkout) and happens before anything is allocated. A map whose steps are all `command` and
  `operator` steps drives exactly as before, on a machine with no harness at all.

- **Everything harness-shaped left this repository for metaharness**
  (`epic:metaharness-migration`, waves 1–4 delivered). Every `llm` step now streams through
  `metaharness run claude --decisions ask`, and the driver's own `decide_tool` answers each
  `tool.requested` at decision time: the two `PreToolUse` shell hooks are ported to Rust case for
  case (frontmatter fence, machine-owned keys, one simple `protocol artifact|trace` invocation)
  plus the per-state allowlist that used to ride on `--allowedTools`. Decisions are `tool.decided`
  events in the run's own transcript; `hook-decisions.jsonl`, the step-context file, the settings
  file and `claude_argv` no longer exist. `integrations/claude-code/hooks/` is deleted; the plugin
  is skills and agents. The eval — logic, recorded transcripts, contracts and results — migrated
  whole to metaharness `evals/aep/` (with `run.sh` retired alongside its
  subject). The three trace expectation documents
  moved to `conformance/trace/` as domain specifications. Suspended by name at the time: the
  trace-spec join over fresh transcripts. The reader it was waiting for landed in this same
  release (see *Added*, below); switching the migrated eval's § 3.4 back on is a change in the
  metaharness repository, not here.

### Fixed

- **`integrations/codex/` is back.** The migration first moved the whole directory to
  metaharness, and that overshot: the codex planning plugin — instruction surface, skill and its
  instruction-surface check — is this repository's product for Codex users, exactly parallel to
  the claude plugin's skills and agents, which never left. Corrected on the operator's call,
  2026-08-22: the plugin returns; only harness-driving machinery lives in metaharness, and
  driving Codex is its `metaharness-codex` adapter.

### Added

- **`protocol contract evidence --record -` reads the record from standard input.** The runner is
  already at the end of a pipe, so the loop is now one line —
  `metaharness conformance claude --contract | protocol contract evidence --record - --observed-at
  2026-08-23` — and what comes back is the same document the file form produces, byte for byte apart
  from the lines naming where the bytes came from. `--record <file>` stays the form to reach for and
  the record says which was used (`inputs: [standard input]` against `inputs: [claude.json]`), because
  the point of a path was never convenience: bytes on a pipe exist nowhere a later reader can go and
  compare against the digest in the provenance.
- **A workflow can now be handed to somebody as instructions, and the instructions are a committed
  artifact rather than a prompt somebody typed.** `protocol workflow instruct --id adp/default`
  writes the workflow as prose — the states work moves through, what opens each move between them,
  and the principles that time obligations against the phases those states declare, joined to the
  states each one lands on. That last part is the sentence neither document contains on its own:
  `test-driven` times an obligation against the `implementation` phase, `adp/default` says
  `implement` is the state in that phase, and the rendering says *before entering `implement`, the
  implementation phase: `test.exists`, `test.first_result == failed`*. Without `--id` it writes one
  document per workflow into `--out DIR`, with an index. Everything in the output is the documents'
  own text; nothing is evaluated, so a principle's `applies_when` is printed as the condition it is
  and never as a verdict about a task. The four documents this tree produces are committed under
  [`generated/instructions/`](generated/instructions/) and held byte-identical, so an instruction
  that no longer matches its workflow turns the gate red instead of quietly misinforming whoever was
  handed it.
- **`protocol eval matrix` turns a directory of checked runs into one table of facts — and refuses
  to turn them into a score.** An evaluation run now writes a versioned `eval.run-manifest/1`
  document beside the report `protocol trace check --format json` produced about its transcript
  (which arm it was — `raw`, `plugin` or `driven` — which harness, workflow, case, model, harness
  version pin and plugin digest, and optionally what it cost, in tokens, micro-dollars and
  milliseconds). The verb reads those pairs and prints, per expectation and per harness × arm ×
  workflow, **how many expectations held, how many the run contradicted and how many nobody could
  find out**, in a sorted, deterministic rendering as JSON or as three tables.
  What you cannot get out of it is a single number: an expectation nobody could decide is not a
  failure and is not a pass, and folding the third column into either is the only way to produce
  one. The verb exits `0` whatever the counts say, for the same reason — a matrix is a report, not
  a gate.
  Refused where the documents enter, each by a code: an arm that is not one of the three
  (`EVAL-MANIFEST-002`), a field that is missing — with every other missing field named in the same
  message rather than one per run (`EVAL-MANIFEST-003`), a `plugin_digest` on arm `raw` or a `null`
  one on arm `plugin` (`EVAL-MANIFEST-005/006`; the key must always be *written*, because a key
  somebody forgot must not be able to claim a run had no plugin), a manifest and a record that
  describe different transcripts (`EVAL-PAIR-003`), one transcript arriving twice
  (`EVAL-PAIR-004`), one specification arriving at two digests (`EVAL-PAIR-005`), and a directory
  with no runs in it (`EVAL-PAIR-006`) — an empty matrix renders as a table with no failures in it,
  which reads exactly like a clean sheet. A row whose verdict is `null` or absent counts as
  **unobservable**, never as held.

- **The sealed frame document a driven step travels on is now pinned against the rules that refuse
  it, and one canonical frame is committed for anyone reading the seam from the other side.**
  `crates/protocol-cli/fixtures/metaharness-frame-canonical.json` is a real
  `metaharness.frame/1` document as `protocol drive` writes one — deterministic, reproducible, with
  nothing account-level in it — and its digest is
  `43a6f845a21f3475569323950a9d276bfed3df11979adc3edf18878da6963a12`. Beside it,
  `crates/protocol-cli/tests/metaharness_frame_contract.rs` writes out the consumer's reader (tag,
  then shape, then digest) rather than linking it — no dependency crosses between a public
  repository and a private one — and refuses that document once per refusal class: untagged,
  misshapen, and a digest that no longer describes the contents after a single byte moved. What
  changes for anybody building against this seam: the frame's canonical form is now a committed
  artifact you can hash and compare, instead of a rule two codebases each believed separately.

- **`protocol drive run` refuses a step map that cannot produce the evidence the plan will ask
  for.** Before the first step, the resolved plan's evidence demands are compared with what the
  map's steps declare, and a kind nothing can produce is a refusal with exit 1 — one line per kind,
  naming the principle that asks for it and the transition or completion it holds shut. Governed run
  `W4-2/1` learned the same thing at a guard six states in, having spent 76 minutes and $31.46 on a
  question two documents on disk had already answered. The check respects `applies_when:` scoping (a
  task declaring `change.code: false` is not refused for `contract_result` or `property_test_result`;
  a task declaring nothing still is), ignores requirements on states no run can reach, and **warns
  rather than refuses** where nothing can be decided from documents — a record only a person can
  produce, or a demand pinning a verifier no step names. `--allow-evidence-gap` starts anyway,
  printing the same gap; it changes no rule the engine enforces, so such a run still stops at the
  guard. Both shipped maps currently report a gap: `verification` and `specification`, plus
  `contract_result` and `property_test_result` for a task that declares no `change.code`.
  Acceptance: `story:plan-map-coverage`.

- **A driven step's tool calls are decided by the engine too, and the execution's own record says
  so.** Until now a decision existed only in the run's event stream: the driver's per-call policy
  answered, and `Engine::authorize` — the call that writes a decision into the execution — was
  never made while a step ran. It is now. Every call the policy admits is rendered as the
  `ActionRequest` it is (a `Read` is a repository read of that file, a `Bash` is a command
  execution of that program, a `WebFetch` is a reading network request to that URL) and put to the
  engine, so `action_requested` plus `action_allowed` or `action_denied` land in the execution and
  survive into `snapshot.json`. What changes for a driven run: **the engine's refusal now wins over
  the driver's allow** — a state whose rendered tool set is wider than the capabilities the plan
  grants will see those calls denied, naming the capability and what is missing — and a `tool.decided`
  reason now says which layer refused. The order is policy first, because it is the only layer that
  sees a call's arguments. Two offered tools are never put to the engine, deliberately: a skill load
  takes no action, and a web *search* names no URL a request could honestly carry.

- **`protocol trace check|inspect|evidence` reads a driven run's transcript.** A `metaharness.event/1`
  event stream — what every `llm` step writes since the migration — is lifted into `trace-ir/1` and
  judged by the same `trace-spec/1` documents as a recorded Claude Code transcript, with the same
  arguments: which reader runs is decided from the file's own first line, and the report names it
  (`adapter metaharness/event-stream`). The stream-json reader is unchanged and still reads the
  recorded fixtures. What a driven run gains: **a denial the seam took is counted**, so
  `permission.denied` means something in a run where the vendor's own array is empty because
  enforcement worked first — one refused call is one denial however many layers wrote it down.
  What a driven run could not answer when this reader was written — `skill.completed`,
  `tokens.thinking`, `iterations`, `speed`, and a `cost.total` scoped to one model — it can answer
  now, and the entry below says how. What still reads `unk`: `tool.failed` and `tool.error_rate`
  over a result that recorded no `is_error`, because absence is not success on a wire that may be
  carrying any vendor. Acceptance: `story:event-stream-trace-adapter`.

- **Five more expectation kinds now decide a driven run, because the seam started carrying what
  they read.** `skill.completed`, `tokens.thinking`, `iterations`, `speed` and a `cost.total`
  scoped to one model reported `unk` against every driven transcript — not a defect in the run, a
  reader with no field to read. metaharness protocol **amendment a9** adds the fields (a
  `tool_use_result` on `tool.result` carrying the vendor's own per-tool record verbatim, and
  `thinking_tokens`, `iterations`, `speed` and `cost_usd` on every `usage` payload) and this reader
  lifts them. Measured against the committed driven fixture, `conformance/trace/expectations.trace.yaml`
  goes from 34 ok / 3 gap / 4 unk to **39 ok / 3 gap / 0 unk**, and the driven-step document from
  11 / 0 / 1 to **12 / 0 / 0** — with no word of any document changing.
  **A `null` still reads `unk`, and that is the part to rely on.** A vendor that reported nothing
  writes the key with a `null` value, and every one of these kinds stays undecidable against it —
  which is what a Codex-driven run looks like for four of the five. A specification that gates on
  billed thinking will go `unk` (exit 3, *nobody found out*) rather than green when it meets a
  harness that does not report it. Two things the reader will not do to fill a gap: `tokens.thinking`
  is never taken from the harness's live `thinking.estimate`, and `iterations` is never a count of
  the `usage` events that went past.

- **`harness: metaharness` — a second executor on the seam that was waiting for one.** An `llm`
  step naming it is spawned through `metaharness run claude` instead of a bare `claude` argv: the
  step's per-state surface travels as a sealed `metaharness.frame/1` document (digest-verified by
  the binary, cross-checked byte-for-byte against it with no crate link between the
  repositories), the governed tree travels as the `--cwd` declaration, and per-call denials are
  `tool.decided` events in the event stream the executor writes as the transcript — not a
  side-channel log a forgotten `--plugin-dir` can silence, which is how all eight post-fix
  sessions of run `W4-2` ran unenforced while looking clean. Operation rendering mirrors
  `allowed_tools` decision-for-decision; `subagent.spawn` is never offered. The default
  `claude-code` executor is unchanged. What frame mode does not carry — the hooks' per-argument
  narrowing — is stated in the executor's doc comment and waits for `--decisions ask`.
  Acceptance: `story:metaharness-executor`.

- **`env.mcp_servers` — the fifty-first expectation kind, and the first thing that can say a
  session was hermetic.** A scratch `CLAUDE_CONFIG_DIR` isolates a directory: it keeps the
  operator's plugins, skills and output style out, and it does not keep out account-level MCP
  servers, which are attached to the login and arrive over the network. Two of the four model
  sessions of governed run `W4-1/1` listed three of them in their init event, in a config home
  with no `mcpServers` key and a tree with no `.mcp.json`. All three were `status: needs-auth`
  and exposed no tool, so the inventory was 28 with servers and without — which is why
  `env.tool_available` cannot see this and why the new kind is a bound on a count. `{count:
  {at_most: 0}}` is the hermetic claim; a missing field is `unk` and never `ok`, because absence
  of evidence is not hermeticity. The init event's `mcp_servers` is lifted into `trace-ir/1` as
  `SessionStart.mcp_servers`, with an absent field and an empty list kept apart all the way down.

- **The eval sessions now launch with zero MCP servers, and the assertion gates everywhere.** The
  register row that produced `env.mcp_servers` said no directory the runner controls can exclude
  account-level servers — a flag can: `eval/run.sh` passes `--strict-mcp-config`, which ignores
  every MCP configuration not on its own command line. With the exclusion in place the expectation
  gates at `{at_most: 0}` in all three specifications, including the interactive one where it had
  been advisory: a server in the init event no longer reports somebody's account, it reports a
  broken exclusion. The driver's own session launcher gains the same flag with the driver wave.

- **A second step map, `development/checks`, for work whose acceptance is written in checks.**
  `drivers/development/default.yaml` names `cargo` in every state that names a verifier, which is
  what wedged governed run `W4-1/1`: the model wrote nine red shell checks and the step after it
  ran `cargo test --workspace` green, recording `test.first_result = passed`, which never changes.
  The decision the register row offered is taken: that file is a Rust map and says so in its
  header, and `drivers/development/checks.yaml` is the map beside it — one verifier command,
  `bash .engineering/checks/run.sh`, run in `establish_verifiers` before any implementation exists
  so the first recorded suite is the red one; `protocol validate` and `protocol artifact validate`
  as contract suite and static analysis, so no compiler is named anywhere. It carries the two
  steps its sibling lacks: an `operator` step that asks a person to approve the specification,
  because `spec-driven.before_implementation` wants `approved` and a run that approved its own
  specification would satisfy the principle by writing to the document the principle is about; and
  a `command` step running `protocol trace evidence`, so a driven run mints its own
  `trace_conformance` instead of a person typing the verb afterwards. Two maps now fit
  `adp/default/1`, so `protocol drive run` refuses to choose and names both — `--map` says which.

- **A step map can name the document a verifier wrote.** `evidence.record: <path>` on a `command`
  step: the driver reads that document and submits what it says instead of minting a record from
  the exit status. This is what makes `trace_conformance` reachable from a map at all — its record
  carries a specification digest, a transcript digest and three counts, and an exit status carries
  none of them, so minting one would state numbers nobody read. Two placeholders are expanded in a
  step's `run` words and in `record:` — `{run_directory}` and `{transcript}`, the transcript of
  the `llm` step this one follows — because a run directory is allocated when the run starts and a
  document in the repository cannot name one. An unknown placeholder, and a `{transcript}` in a
  state with no `llm` step before it, are refused at load.

- **A Codex variant of the planning instructions**, at `integrations/codex/`: the same four
  guardrails and the same read-the-vocabulary-from-the-CLI rule, as a Codex skill and an
  `AGENTS.md` fragment, verified against codex-cli 0.145.0. Instruction surface only — no
  enforcement hooks, no transcript adapter and no live eval, each refused with its reason in the
  README. `task codex-eval` checks the surface itself: free, no API call, no model — nine checks
  against the files, so drift in the instructions fails a command instead of a reader.

- **`protocol contract evidence` — an outside contract runner's own record becomes a fact the
  engine reads.** metaharness contract-tests each of its vendor adapters and prints the outcome as
  one JSON object in the `contract_result` shape this repository defines; until now nothing here
  read one, so the two repositories shared a vocabulary that no bytes had ever crossed. Hand the
  runner's output to this verb with the day it was made
  (`protocol contract evidence --record claude.json --observed-at 2026-08-23`) and what comes back
  is a document `protocol evaluate --evidence` reads directly — the runner's counts untouched,
  `producer: verifier / contract-runner`, and the SHA-256 of the bytes it was handed in the record's
  provenance. No new evidence kind, no schema change and no document change: `contract_result`,
  the `contract-runner` verifier and `contracts.**` have been in `protocols/aep/1.yaml` since the
  base protocol, which is what the shared vocabulary was for. The two captured records are
  committed at `crates/protocol-cli/fixtures/metaharness-contract-result-{claude,codex}.json`
  (20 and 10 vectors, both green) and a metaharness-side wave pins the same bytes, so the two sides
  disagree loudly or not at all.
  **Two records are refused rather than minted, and the refusal names why.** A record stating
  `checked: 0` asserts nothing — measured against `examples/billing-conformance` it would discharge
  the `contract_result` obligation the `contract-testing` principle places on a task, and pass two of
  that principle's three predicates vacuously, on the strength of a run that checked nothing. A
  record whose `breaking_changes` exceed its `failed` describes no run, since a breaking change is
  one of the failures. Bad news is not refused: a record reporting failures is written down and
  exits `0`, because the verdict belongs in the record and the engine is what decides on it — and
  `contracts.breaking_changes` is the line that moves, so a red run that is the runner's own
  machinery reads differently from one where the vendor moved.
  **`--observed-at` is required**, unlike `protocol trace evidence`'s: that verb runs the check
  itself and may stamp its own clock, this one is handed a record from another process on another
  day, and a default of *now* would be a freshness claim nobody made.
  Acceptance: `story:contract-result-ingestion`.

## [0.11.0-ground-truth-and-docs] — 2026-08-22

### Fixed

- **`protocol evidence inspect` no longer refuses a record the day it is written.** The reference
  is a civil date, and the future check compared wall-clock milliseconds against the day's first
  millisecond — so a record stamped 14:07 today read as "has not happened yet at" today, and the
  verb's primary use was its failing case. The check now runs at the reference's own granularity:
  an observation is future only when its civil date is after the reference date. A planned check
  dated tomorrow is still refused. Found by the docs overhaul re-running every quoted command.

### Changed

- **An `llm` step is now told what guards the way *out* of its state.** The step context carried
  `Evaluation.requirements` — what must hold *while in* the state — and never the outgoing
  transition's, so in `W4-1/1` the model was never told that `implement` needed a red suite and an
  approved specification, and $8.36 of one state went on work the guard then refused.
  `StepContext` carries `reaching` beside `requirements`: one line per requirement that does not
  hold yet on a way out, prefixed with where that transition goes, under its own heading in the
  prompt and as a `reaching` array in `step-context.json` (additive to
  `aep.drive-step-context/1`).

- **An `operator` step is a question asked once.** The pause is the step's completion, so the
  cursor moves past it and `protocol drive resume` carries on from the step after — whether the
  person did what was asked is decided by the guard on the way out, which refuses with one line
  per unmet requirement. A cursor left pointing at the step that paused re-presented the same
  question on every resume, so no map with an `operator` step before its last state could ever
  move past one.

- **The driver's model sessions launch with `--strict-mcp-config`**, so a session's MCP surface is
  what the launch line gave it, which is nothing. An account's MCP servers arrive with the login
  and a scratch `CLAUDE_CONFIG_DIR` cannot exclude them; `env.mcp_servers` gates at zero in the
  driven specifications, so a driven run without the flag failed its own transcript check on an
  account property no document here controls.

- **The public docs, the guides and the control documents caught up with the tree.** Four
  parallel review passes over README, AGENTS.md, `docs/guide/`, the plugin READMEs and all
  26 website pages, with one rule: every number from a command run this session, every reference
  resolving, every quoted output reproduced. What that surfaced and fixed, beyond prose: two Rust
  snippets that no longer compiled (`EvidenceSubmission::new` takes `observed_at` now), four
  evidence documents shown in shapes that no longer validate, sixteen CLI leaf verbs absent from
  the reference page (and zero phantom ones), a limitations page rebuilt from the gap register's
  20 open rows, the driver documented as shipped instead of planned — including the first
  governed run and where it stopped — and the evidence concepts page teaching the two-times
  model and decay to Unknown. Blog posts keep their published text and carry dated
  "since publication" notes where a claim aged. Three literals that drifted inside generated or
  checked surfaces are register rows rather than hand edits, beside two defects the overhaul
  found (`horizon` published as an integer where the parser wants `7d`; `ess impact` counting
  56 of 38).

- **The guard-efficacy review's last two loose ends, closed.** Every substantive finding of
  `docs/reviews/2026-08-20-guard-efficacy-review.md` was fixed by later waves — the refusal that
  authorised, the unenforced `Deserialize` ban, the one-directional approval floor, the untested
  Kleene negation, the audit disjunct, the Decimal rejection test, proptest phase 1 — and two small
  ones were not: the two guards the review caught reporting a bare `left == right` now say what
  broke and why it matters (`kleene_conjunction_keeps_false_ahead_of_unknown`, the Decimal
  structural assertions), and the `identity` conformance suite's module doc records the mutation-11
  efficacy evidence the review's D5 accepted in place of an in-CI fault, so the one suite whose
  efficacy CI cannot verify carries its measured proof where a reader of the suite looks.

- **The horizons corpus is ground truth now, and the scanner reads two more positions.** The
  adopter fixed their reference implementation against the vendored corpus and re-issued
  `expected.json` as ground truth: 43 raw annotations, 43 parsed, `missed_by_reference: 0`, with
  `reference_is_not_ground_truth` kept as a field and the reason recorded beside it. The revision
  adds position 7 — a backticked annotation mid-line, after prose, whose live instance had a
  one-day horizon and was already stale — and the rule in the other direction: an annotation
  inside a fenced code block is an illustration, excluded from parsing and from the coverage
  denominator both, because otherwise every document that explains the convention reports a
  permanent, unfixable coverage gap. Inline backticks cannot carry that meaning — positions 6 and
  7 are real claims written in them — so the rule is one-directional: fence it if you are
  illustrating, anything else parses. `aep-backend-markdown` finds 43/43 with divergence 0; the
  fence-stripping in the raw counter is implemented separately from the parser's on purpose, so
  the denominator stays independent evidence. The durable-fact lesson from the same evening — an
  answered question must not leave a permanent re-check obligation behind it — is
  `story:claim-retirement`, not a code change here.

## [0.10.0-horizons-dogfood-lab] — 2026-08-21

### Added

- **Evidence horizons — a green result from three weeks ago is not a fact.** An evidence record now
  carries two times. `observed_at` is when somebody looked, is required, is supplied by the caller
  and is the identity of the fact; `produced_at` remains the engine's, and says when the record
  entered the log. A value in the future is refused outright (`observation_in_future`): a
  scheduled-but-never-performed check stored as an observation reads as the freshest record in the
  log, and the store can no longer answer whether anybody has ever looked.

  An evidence requirement may declare a `horizon: 3d`. Past it the requirement reads `Unknown` —
  never `False`, because a lapsed check has not failed, nobody has run it — with a reason naming the
  horizon, the observation date and the day it lapsed. The transition it used to permit is refused,
  including when the guard reads a fact rather than the requirement: a lapsed record's facts are
  withheld from the store under the strictest horizon the plan declares for its kind, and an absent
  fact is `Unknown`. `evidence.lapsed` joins `evidence.missing` so a stale gate is distinguishable
  from an empty one.

  The horizon is on the requirement and nowhere else. A record has no horizon field, there is no
  operation anywhere that mutates one, and a source scan over five crates refuses both `.horizon =`
  and any `fn` taking `&mut self` with `horizon` in its name — because if `extend` is as easy to
  call as `re-check`, it is the one that gets called. Re-submitting an identical record restores
  nothing; only a new observation time does, and a test says so.

  `aep-backend-markdown` gained a scanner for the one-line dated-claim annotation convention, which
  finds all 42 annotations in the vendored corpus at `examples/evidence-horizons-corpus/` — the
  reference implementation the fixture's expectations came from finds 37 and names the five it
  misses. It reports its own coverage: raw occurrences seen versus records produced, per file, a
  divergence being a finding rather than a silent drop. New verbs: `protocol evidence scan` and
  `protocol evidence inspect`; `--observed-at` on `ess conform evidence` and `trace evidence`.

  Design: `docs/design/evidence-horizons-design-v0.1.md`, corrected by adversarial review
  (19 CONFIRMED / 15 NEEDS-CHANGE / 3 INFEASIBLE, all applied).

- **The lab runs the specification instead of replaying it.** `/lab` on the website used to step a
  hardcoded array of eleven steps with real names in it. It now fetches
  `billing_web_realized.wasm` — the browser realization this repository synthesises from
  `examples/billing/`, linked with the hand-written behaviour in `examples/billing-realization/` —
  and sends five commands over its boundary: create, issue, pay, cancel-a-paid-invoice, and one
  refused amount. The middle panel is what `{"request":"catalog"}` answers, the right panel is the
  outcomes, published events, binding invocations and view rows that came back, and the lines the
  left panel highlights are found in the file itself rather than written down. Same module, same
  glue and same engine as the page, asserted outside a browser by `npm run test:lab`; the run is
  deterministic, so two loads of the page produce the same stream byte for byte. `task lab` builds
  the module — it is a build artifact and stays uncommitted, and a page opened without one says so
  rather than showing a run it did not do.

### Changed

- **The first governed run of a real story, and the record of where it stopped.** `protocol drive`
  walked `story:agent-eval-cases` out of this repository's own `.engineering/` store under
  `development.driven` and the shipped step map, with four headless model sessions, the plugin's
  hooks as the enforcement arm and `cargo` as the verifier. It **blocked in `establish_verifiers`**
  and never reached the person it was meant to stop at, for two reasons the engine printed: the
  specification it had created was `draft` where `spec-driven` wants `approved`, and
  `test.first_result` was `passed` where `test-driven` wants `failed` — because the model wrote its
  failing tests as shell checks, which is the idiom the story's acceptance is written in, and
  `drivers/development/default.yaml` can only run `cargo`. Nothing was changed to make the run go
  through; the run is the finding. The enforcement half held and is recorded with numbers: 80 hook
  decisions, 69 allow and 11 deny, and 11 `permission_denials` — one for one, a second independent
  confirmation of F13 on a map the eval never touched. `protocol artifact validate` is exit 0 over
  the 58 artifacts the run left. Record: `docs/plan/harness-wave-4-governed-dogfood.md` § W4.1,
  *The first run*.

### Fixed

- **Four things the documents invited an adopter to declare, which the engine then refused, ignored
  or could not reach.** All four came from the first adopter's report — the first document tree
  written against this specification that is not this repository's own — and all four were found by
  writing a tree rather than by reading the guide, which is the only way this class of defect gets
  found at all. The report is a repo-local review, held and unpublished
  (`docs/reviews/2026-08-21-first-adopter-report.md`); what follows is what changed in the code, and
  stands on its own.

  **A lifecycle document may leave out `kind:`, and it becomes the tree's fallback.** The field had
  been documented as "absent for the fallback lifecycle" since it was written, and a document that
  left it out was refused — so the sentence described a mechanism that did not exist, and a team's
  own artifact kinds were governed by nothing: every status legal, a misspelt one a shrug rather
  than a refusal. A kind-less document now registers the lifecycle every kind with no nearer one is
  held to. A tree may declare **at most one**, and a second is refused by name rather than
  overwriting the first, because which of two files won would otherwise depend on the order the
  directory was walked in.

  **A kind's parent is the kind its last hyphen segment names — for every kind, not just the ones
  this repository ships.** `architecture-design` is a `design` because the suffix is the noun and
  what precedes it narrows it; that rule was written out as a list of five variants, so
  `observation-log` was *not* a `log` and an organisation's own family of kinds could not share one
  ladder. Each of them needed its own lifecycle document saying the same thing. The rule is now the
  rule: a custom kind's parent is the kind its last segment names, aliases excluded (`openapi-spec`
  is a custom `spec`, not a `specification` — an alias is a spelling somebody typed, not a claim
  about lineage), and a single-segment kind is the top of its own family. One lifecycle registered
  on `log` now governs every `*-log` a team invents.

  **`on_failure` refuses a parameter its action does not take.** `{action: retry, retry: {to:
  write}}` used to validate: the `retry` key named nothing, was dropped, and what remained was a
  bare retry falling back to `block`. The document said one thing, the engine did another, and a
  reviewer had no way to tell — a policy that validates and does nothing is a gate that cannot fire.
  Each action now has a closed parameter set, every invented key is named at once rather than one
  per round, and the published JSON Schema says the same: one closed form per action, in place of
  "a string or a mapping of anything". Every committed document still parses unchanged.

  **The project directory's name is a default, not a constant.** `.engineering` was fixed at compile
  time, so a repository that spends that name on something else, or whose team calls this
  `.workflow`, could not be discovered at all. `AEP_PROJECT_DIR` renames it, honoured by walk-up
  discovery and by everything the CLI resolves against a project. It is read **once per process** —
  a value that could change between two reads would give one run two different projects — and it is
  read in the engine, at the edge that touches the filesystem, never in `aep-domain`, which stays
  free of the environment, the clock and the disk.

### Changed

- **`docs/guide/adopting.md` now says which documents a project may add without owning a tree, and
  which oblige it to own one.** The page showed a repository with `workflows/` and `protocols/` in
  it and left the reader to assume that a project pointing at somebody else's tree could do the
  same. It cannot: the project-local merge covers **principles and profiles**, and nothing else — a
  workflow under `.engineering/` is not read at all, and the failure surfaces where the workflow is
  named rather than where the file sits. The guide now states that plainly, with the refusal it
  produces, a table of what needs a tree of your own, and the `protocols: .` layout written out —
  including the two lines of `project.yaml` that keep the tree from being loaded twice, which is the
  part that is easy to miss and refuses with a duplicate id. The merge is not being extended here;
  that is tracked as a gap.

## [0.9.0-harness-waves-2-3] — 2026-08-21

### Added

- **`protocol drive` — a workflow you specified is now a workflow that runs, and a step the protocol
  does not permit does not happen (harness wave 3).** The engine has always decided; nothing in this
  repository had ever *done* what it decided. `protocol drive run` holds the loop: rebuild the
  artifact graph from the store, ask the engine what is owed, run the next step of the state the run
  is in, submit what a verifier produced, and ask to move. It evaluates no gate of its own — a driver
  that could would be a second protocol implementation with none of the conformance suites behind it,
  and the first time the two disagreed the one nobody tested would win.

  **A step map is the missing half of a workflow, and it is a document.** `drivers/<family>/<name>.yaml`
  is the fifth document kind in the tree, loaded, validated and schema-generated like the four before
  it: per state, an ordered list of steps, and the engine is asked to move when the list is done. It
  **pins the workflow it was written against** — `workflow: adp/default/1`, mandatory — so a workflow
  that reaches version 2 orphans the map at load, refused and naming both versions, rather than
  quietly applying instructions to a state graph nobody wrote them for.

  **Three step kinds, and the important one is what an `llm` step cannot do.** A `command` step runs
  a program and records `producer: verifier`, which is how `independent: true` is honestly satisfied.
  An `operator` step prints the engine's explanation verbatim, persists, releases the lock and exits
  0 with a line somebody else can resume from — because a driver holding a terminal open for a person
  loses the run when the terminal closes. An `llm` step **has no `evidence` field**: not a rule that
  could be relaxed, a variant with nothing to put a claim in. Anything a model was supposed to
  achieve that is checkable is observed by the `command` step after it.

  **What a run leaves behind, and what a refusal reads like.** `.engineering/runs/<task>/<n>/` holds
  the engine's snapshot and the driver's cursor side by side — two documents because they have two
  owners — plus each step's log and each model session's transcript. A blocked run prints the
  engine's own sentences and does not reword them: on the shipped fixture, six moves from `receive`
  to `adversarial_verify` and then `adversarial_verify -> review: guard: evidence.missing == 0`,
  character for character in both the report and the cursor. **A crash submits nothing**, because
  absence is the fact not being in the store, and collapsing a crash into a failure sends an agent to
  fix code nobody ran. **One run per store**: a lock at one fixed path, created before a run id
  exists, refused by name to a second invocation with the holder's run id, pid and host, and gone on
  every exit path the driver controls. `--resume` re-takes it and refuses if the workflow, the step
  map or the engine moved underneath.

  **The neutrality claim is now a test rather than a sentence.** A second harness — a shell script
  with no model, no network and no credential in it — implements the same three adapter points and
  runs inside `task check`: the same step map, the same `tool_config`, the same checker, and a
  transcript in a dialect of its own that mints a `trace_conformance` record. The Claude Code adapter
  refuses that dialect, which is asserted, because two readers that accepted each other's formats
  would be one reader tested twice.

- **A driven agent's shell holds only the protocol verbs, and a hand-edit of machine-owned
  frontmatter comes back refused with a reason.** The plugin now ships two `PreToolUse` hooks, and
  they are the driver's enforcement arm rather than a second, weaker driver.

  `store-integrity` is **always on**, with or without a run: under `.engineering/planning/**` a
  whole-file `Write` or `NotebookEdit` is denied by path, and an `Edit` is denied when it crosses the
  `---` fence or writes `id`, `kind`, `status`, `revision`, `relations` or `format`. A targeted body
  edit below the fence stays legal, because prose is not the CLI's business and there is no verb for
  it. What comes back is not "denied": it names the field, says that `status` moves only through
  `protocol artifact move`, and says why — a hand-retyped frontmatter is indistinguishable from a
  silently-altered one until something downstream breaks.

  `driven-surface` is **inert outside a driven run** and, inside one, holds a shell to one simple
  invocation of `protocol artifact …` or `protocol trace …` — no pipes, no redirection, no `&&`, no
  command substitution, because a composed command line is a second command wearing the first one's
  name. Both hooks **fail closed**: with neither `jq` nor `python3` on `PATH` they refuse rather than
  pass an unread call through, and every decision is appended to the run's own
  `hook-decisions.jsonl`, which is the only record that can tell *denied* from *never attempted*.

  **A new profile, `development.driven`, and it is not a relaxation.** It is `development.standard`
  plus `command.execute`, and it exists because the planning store has no tool surface other than the
  `protocol` CLI: under the two older development profiles a driven step can be told to write a
  specification as an artifact and has no way to create one. The narrower grant cannot be written —
  `command.execute:protocol` is a parse error, since capability scoping exists for deployment
  environments and nothing else — so the grant's outer bound is the profile and its inner bound is
  the hook. The approval floor is untouched, and the store's write guard no longer rests on there
  being no shell: it rests on the shell not being able to say `sed -i`. Choose it only for a run
  under `protocol drive`; interactive work and any harness without an equivalent constraint want
  `development.standard`.

  Both are exercised by a second eval, `integrations/claude-code/eval/run-driven.sh`, which drives a
  real model through an honest step and a deliberately refused one and then judges the result by the
  store, the decision log and two trace specifications. Like its neighbour it reaches the API and
  costs money, so it is **not** part of `task check`.

- **`protocol workflow render` — a workflow, and a run over it, as a picture.** Until now a workflow
  could only be printed as YAML. Four formats behind one scene: a standalone `svg`, a self-contained
  `html` page that fetches nothing, a `png` by way of `rsvg-convert`, and a `tui` frame for the
  terminal. Hand it `--run` and it draws where the run has been, how often it entered each state,
  what evidence it produced and why it stopped — with the engine's reasons **verbatim**, never
  summarised, because a picture that paraphrased a refusal would be answering a question it did not
  evaluate. `--watch` redraws the terminal frame as a run advances.

  It **decides nothing**: the overlay is handed in as a plain value and the crate depends on the
  domain types alone, not on the engine and not on the driver. Rendering is **byte-stable** — the
  same workflow and the same run produce the same bytes — so a committed figure does not turn up in a
  diff for a reason nobody chose.

## [0.8.0-harness-wave-1-trace-wave-1] — 2026-08-21

### Added

- **`protocol trace check` — what an agent run did, judged by a typed document instead of a shell
  pipeline (trace wave 1).** A harness transcript (first adapter: Claude Code `stream-json`)
  normalizes into a content-addressed, harness-neutral event IR, and a `trace-spec/1` document
  states expectations over it — forty-nine kinds, from *the skill completed* and *this tool was
  called with these arguments* through ordering, token, cost, cache, rate-limit, tool-traffic and
  per-step timing bounds. Verdicts are three-valued: `ok`, `gap`, and `unk` for the event the
  adapter could not read or the field this transcript does not carry — exit 0/1/3, the same
  contract as `ess conform`, because "the format moved under us" wakes a different person than
  "the agent did the wrong thing". Every verdict cites the transcript event indices behind it,
  and `protocol trace inspect` prints the census — event families, per-tool traffic in both
  directions, each step's `gen`/`exec` split — from the same IR the checker judges.

  The plugin eval now runs on it: five assertions in three shell idioms became
  `integrations/claude-code/eval/expectations.trace.yaml`, forty-one expectations with the
  observed value beside every bound, checked against two committed real transcripts by the
  ordinary gate — so a bound that stops holding is caught without a paid run.

- **`protocol trace evidence` — a passing check becomes an evidence record the engine admits.**
  `Evidence::TraceConformance` carries the verdict, the three counts, every gapped expectation's
  id, any command-line downgrades, and the digest pair binding it to exactly one transcript and
  one specification; the producer is the `trace-checker` verifier class, so an agent's own claim
  of conformance never satisfies the kind. The loop is asserted end to end: the emitted document
  feeds back into `protocol evaluate --evidence` and is accepted. This is the mechanism the
  future reference driver's model-calling steps rely on — an `llm` step cannot carry evidence by
  type, and the command step that observes it now can.

### Changed

- **The vision's refusal of "a workflow engine" is narrowed: a reference driver is now in scope —
  decided and designed, not yet built.** Nothing about the engine changes; it still only decides.
  What changes is that the harness contract in [`docs/guide/harness.md`](docs/guide/harness.md) —
  seven calls, three rules — is going to get a first implementation inside this repository, the way
  the storage contract has `aep-backend-memory`. A published contract that nothing implements is the
  same defect as an invariant that nothing enforces, and it had been that since the guide was
  written.

  What did **not** move is worth as much as what did. Gates are still evaluated by the engine and
  never by a driver. Invariant 7 is untouched: an agent's own statement never satisfies an
  independence requirement, and in the driver's design that is a type rather than a rule — a step
  that calls a model has no field to put evidence in. "External systems do the work; this project
  decides what the results permit" still holds, with the driver as the first of those external
  systems, kept in-tree the way the website is.

  [`docs/VISION.md`](docs/VISION.md) carries the argument;
  [`docs/plan/control-document-updates.md`](docs/plan/control-document-updates.md) carries the
  record of who decided it and when;
  [`docs/design/harness-planning-and-driver-design-v0.1.md`](docs/design/harness-planning-and-driver-design-v0.1.md)
  § 4 is the design, which is architecture with six named open problems and is explicitly **not**
  accepted for build.

### Added

- **`protocol artifact` — planning artifacts live in your repository, and a status move is checked
  before it happens (harness wave 1).** Epics, stories, tasks and initiatives are markdown files
  under `.engineering/planning/<kind>/<slug>.md`: frontmatter the CLI owns, a body you own. Ten
  verbs — `new`, `move`, `relate`, `list`, `board`, `graph`, `validate`, `kinds`, `relations`,
  `lifecycle`.

  **A refused move tells you where you can actually go.** `move story:credential-store --to
  implemented` from `draft` does not print "illegal transition"; it prints that `implemented` is not
  reachable from `draft` and names the statuses that are. A refusal that sends you off to read a
  lifecycle file is a refusal that gets guessed around, which is the one outcome a validated
  lifecycle exists to prevent.

  **`validate` reports everything, not the first thing.** A store with four unresolvable relation
  targets reports four, each naming the artifact and the edge, and exits 1. Run it after a batch of
  edits; it is also what catches a status somebody hand-edited into a file, which a file store
  cannot prevent and this is honest about.

  **An id is declared and never allocated.** An artifact's id is `<kind>:<slug>` and must agree with
  its path. There is no counter, because two branches that both ask a counter for the next number
  both get it, both merge cleanly, and the store then holds two artifacts with one id — a corruption
  git cannot see, because nothing was in conflict. Slugs collide only when two people meant the same
  thing, and then git conflicts on the path. A consequence worth having: `story:dev-399` is a legal
  id, so a team whose tickets are named elsewhere can keep the name.

  **No timestamps in the file.** Git already knows when the file changed and who changed it, and it
  cannot be made to say otherwise by editing a line. The cost is real and stated: "how long has this
  been in draft" has no answer inside the store, and `git log` is the answer until the journal
  milestone.

  The on-disk format, `aep.planning-md/1`, belongs to `aep-backend-markdown` — `aep-domain` gained
  no types for it, and no other backend is obliged to store anything this way. It is described all
  the same: `schemas/generated/planning-document.schema.json` is generated from the parser's own
  type, so the published description of the format cannot drift from the code that refuses a bad
  one. The `format:` line is optional and defaults to `aep.planning-md/1`, so a file you write by
  hand does not need it. Unknown frontmatter keys are preserved rather than stripped, so another
  tool writing into the same file does not lose its fields.

  This is a store and **not** an implementation of the storage contract: it writes through its own
  functions rather than through `CommandService`, so the sixteen conformance suites do not run
  against it and it has no journal or audit trail yet. Both facts, and what closes them, are in
  [`docs/plan/gap-register.md`](docs/plan/gap-register.md).

- **Three new artifact lifecycles: `epic`, `task` and `initiative`.** Each mirrors the ladder
  `story` already had — `draft → proposed → active → implemented`, with `rejected` and `archived`
  where they belong — so all four planning kinds move by one set of rules that an operator learns
  once. Every status word already existed in the vocabulary, so nothing else changed to make room
  for them.

- **A Claude Code plugin, at [`integrations/claude-code/`](integrations/claude-code/).** One
  `planning` skill and two agents: `decomposer`, which breaks an epic into stories and produces
  drafts only, and `plan-reviewer`, which reads the store and changes nothing. Install it from the
  marketplace entry at the repository root.

  **The skill carries rules and no vocabulary.** It does not list the kinds, the statuses or the
  legal moves; it asks `protocol artifact kinds`, `relations` and `lifecycle <kind>` at the moment it
  needs them. A prose copy of a validated document inside a skill file is neither validated nor
  versioned, and it goes stale the first time a kind gains a status — after which the agent recites
  last month's ladder confidently and proposes a move that does not exist.

  What it does inline is four guardrails: a status changes only through `protocol artifact move`,
  the body is edited directly because the CLI does not own prose, `validate` runs after a batch of
  edits and its output is relayed verbatim, and a refusal is the answer rather than an obstacle — the
  legal moves get reported to you, not routed around by editing the file.

  **No hooks, on purpose.** Deterministic interception of what an agent may do is the reference
  driver's job, and a hook layer would be a second, weaker driver — one that sees tool calls instead
  of workflow states and cannot ask the engine anything. There is no `commands/` directory either:
  the CLI is the command surface, and a slash command wrapping a verb is a second spelling that
  drifts from the first.

  Its behaviour is checked by a repeatable eval — `integrations/claude-code/eval/run.sh` runs a
  headless session in a scratch directory and then inspects the store it left behind, asserting on
  the artifacts, statuses and edges rather than on the model's wording; it is deliberately not a step
  of `task check`, because the gate reaches no network and this calls a model.

  The eval runs **hermetically**: a scratch `CLAUDE_CONFIG_DIR` carrying only the login
  credentials, so the operator's own plugins, skills and output style cannot leak into the run —
  asserted, not assumed, from the session's init event, alongside eight other mechanical checks.
  Every report also carries run metrics (tool traffic in bytes and tokens, failing and repeated
  calls, per-step `gen`/`exec` timing derived from recorded timestamps, cache use, rate-limit
  state) and an **adversarial review**: a second, independent session reads the task, the verdict,
  the metrics, a timing-annotated timeline and the created artifacts verbatim, and reports what
  assertions cannot see. The review is advisory by design and never moves the exit code — and it
  has already earned its place once: it caught the agent re-typing machine-owned frontmatter via
  whole-file writes, the skill's guardrail was tightened in response, and the next runs switched
  to targeted edits and a clean advisory.

## [0.7.1-infra-waves-1-4] — 2026-08-21

### Added

- **`protocol infra project --spec <file> --path <bundle|ir> --out <dir>` — the gaps, handed back
  as a diff you can read (infra wave 4).** `simulate` tells you a container declares no limits.
  This writes the patch that would declare them, into a directory you review, edit and apply with
  your own hands. Nothing is applied and nothing reaches a cluster — the output is files.

  **Every value in a generated file came from the gap or from you.** A replica count outside
  `[2, 4]` has one nearest acceptable number and the range says which, so that one is written. An
  image tagged `latest` has no mechanically-nearest replacement, so it is not: you get an
  obligation naming the decision — *choose the version of `registry.local/flaky-agent` that
  container `agent` should run* — instead of a patch containing a version somebody's generator
  picked. Resources and probes sit on both sides of that line: state the values once as a
  `remedy:` on the expectation and they are written; state nothing and they are owed.

  **A patch is against the object that was observed**, not a rewritten manifest — a whole manifest
  regenerated from a snapshot silently drops every field the observation model does not keep.
  Container-level changes are emitted as *strategic* merge patches and say so in the filename,
  because an RFC 7386 patch naming one container deletes every container it does not mention.

  **The projection closes what it opens.** Raising a workload from one replica to two satisfies
  "replicas within [2, 4]" and immediately breaks "a disruption budget covers every multi-replica
  workload" — so it simulates its own changes, sees the gap it opened, marks it *induced* and
  writes the budget in the same tree. Applying the whole directory leaves more expectations holding
  and none newly broken; the test suite applies the emitted files to the bundle, recompiles and
  re-simulates to prove exactly that, including that no unrelated verdict moved.

  **`OBLIGATIONS.md` is a file of its own**, because a tree that closed nine gaps and quietly left
  sixteen decisions unmade reads, in a pull request, exactly like a tree that closed everything.
  `SUMMARY.md` carries the counts and the digests of both inputs it was computed from — a name is
  not an identity, and two revisions of your specification share a name.

  Two expectations that disagree are not silently reconciled: the one the emitted patch does not
  satisfy comes back **refused**, naming the expectation that contradicts it. Exit 0 whatever it
  finds, as `simulate` and `diagnose` already behave.

- **`infra-spec/1` expectations may carry a `remedy:`** — the value a projection writes where the
  expectation finds a field empty:

  ```yaml
  - id: shop-resources
    scope: {namespace: shop}
    expect: resources_declared
    remedy:
      resources:
        requests: {cpu: 25m, memory: 64Mi}
        limits: {cpu: 500m, memory: 256Mi}
  ```

  **A remedy never changes a verdict.** Nothing evaluates it; `resources_declared` still means
  "declares requests and limits" and nothing else, so adding one to a specification you already
  committed cannot move a simulation you already reviewed. Two new refusals guard it: a remedy
  beside a kind that can never write it is `INFRA-SPEC-009`, and one that states nothing, names a
  probe the expectation never asks for, or writes a port as `"8080"` in quotes is
  `INFRA-SPEC-010`.

- **`examples/k3d-dev-cluster/projection/` — a real patch tree in the repository.** Seven committed
  files for the committed specification and observation: two strategic patches, three generated
  disruption budgets, `SUMMARY.md` and `OBLIGATIONS.md`. Twenty-three gaps go in; nine come back as
  changes, sixteen as decisions nobody can take for you. Drift-checked by `cargo xtask infra
  --check` with an orphan scan, so a patch file nothing generates any more cannot sit there looking
  like a proposal somebody still stands behind.

- **`protocol infra simulate --spec <file> --path <bundle|ir>` — what you expected, against what
  was observed, with a third answer beside yes and no (infra wave 3).** You write an
  `infra-spec/1` document saying how the cluster ought to be, and every expectation in it comes
  back `ok`, `gap` or `unk`. A `gap` says what would have to change — *`storefront-server` declares
  2 replicas and no disruption budget covers it*, not "failed". An `unk` says why the snapshot
  cannot decide — *the `svclb` daemonset declares no replica count*, *`redis:7-alpine` names no
  registry so which one resolves it is not observed*, *namespace `payments` was not observed*, *the
  bundle did not scan `poddisruptionbudgets`*, *the bare `debug-shell` pod has an underivable
  controller, so pod counts in its namespace are a lower bound*, *the scope selects no subject*.

  **`unknown` is never quietly a failure**, and an expectation cannot pass by selecting nothing:
  a scope that matches no workload comes back `unk`, not `ok`. An expectation with one contradicted
  subject and one undecidable subject is a `gap` — something *was* observed to be wrong.

  Twelve expectation kinds, kept small and decidable: a workload exists; replicas within a range;
  requests and limits declared; probes declared; images from a registry allowlist, not `latest`,
  pinned by digest; a disruption budget covers every multi-replica workload; a service selector
  matches a pod; every required configmap and secret reference resolves; workloads only in listed
  namespaces; and a labelled predicate over eighteen `workload.*` facts, using the protocol's own
  three-valued predicate language rather than a second one. Scopes are the whole cluster, one
  namespace, or workloads carrying a set of labels.

  **No expectation asks what time it is.** Nothing compares a timestamp and there is no way to
  write a duration, so the same specification and the same snapshot always produce the same
  report — which is what lets a report be committed and reviewed as a diff at all.

  **Simulating is a report, not a gate**: exit 0 whatever the verdicts say, exactly as
  `protocol infra diagnose` behaves. Exit 1 means the input could not be simulated — a
  specification this build refuses, a bundle that is not valid, an IR document somebody edited.

- **`protocol infra diff --from <ir> --to <ir>` — what moved between two scans of one cluster.**
  Sixteen typed change kinds over the *declared* state: objects added and removed, replicas,
  images, containers, resource bounds, probes, environment, workload and service fields, ingress
  routing, configuration content, claim phases, and references that broke or healed. A configuration
  change names which keys moved and never what they hold. Pods are deliberately absent — they are
  renamed on every rollout, and a report listing a thousand of them is one nobody reads. Reordering
  a template's containers is not a change. It refuses one thing: two snapshots scanned in different
  kubeconfig contexts.

- **`examples/k3d-dev-cluster/` grew a desired state and a second scan.** `expected.yaml` is 28
  expectations reaching all three verdicts on the example cluster (11 hold, 12 gaps, 5 undecidable),
  `observation.drifted.json` is the same cluster twenty documented mutations later, and
  `simulation.json` and `drift.json` are the two reports — committed and drift-checked by
  `cargo xtask infra --check` beside the compiled IR, so a rule that starts answering `false` where
  it answered `unknown` shows up as a reviewable diff.

### Changed

- **`cargo xtask infra` writes three documents instead of one**, and `task infra-check` checks all
  three. The CI job is renamed to match.

## [0.7.0-ess-wave-7] — 2026-08-21

### Added

- **One specification, two running applications, one surface (ESS wave 7, W7.5).**
  [`examples/gatepass/`](examples/gatepass) is a new application specification — visitor passes for
  a building — and `protocol ess synthesize` now emits, for Rust *and* for Go, a binary that serves
  it over HTTP. Start either one and it writes the same three lines of JSON about itself, answers
  the same seven routes, and publishes the same contract and the same documentation, byte for byte.
  `cargo xtask synth` starts both on ephemeral ports and holds them to it.

  **A component can now say where its callers are, and that is what forces HTTP.** The model gained
  one word: `reached_by: network` on a component, against `in_process`, which is what silence has
  always meant. It names no protocol. What follows is a derivation and not a preference — a surface
  whose callers are not deployed with it has to exist on a wire, and this repository projects
  exactly one contract for a component's command surface, the `OpenAPI` document, which is an HTTP
  contract. A synthesised server speaking anything else would contradict the document committed
  beside it. **A specification that says nothing keeps everything it had**: the word is left out of
  the resolved model when unstated, so every committed artifact of every existing specification
  keeps the digest it had, and no server is emitted for a system that never asked for one.

  **A view is exposed only where the specification says something outside reads it.** The `OpenAPI`
  projection has always refused to give a view a path, because nothing in the model said how one is
  read; it still refuses, unless the component declares a network surface. Then each view gets
  `GET /{domain}/views/{view}`, its rows under one key, its declared filter in the description and
  its consistency as `x-ess-consistency` — and still no page size, no cursor, no ordering and no
  filter parameter, because the specification states none of them. A component that declares a
  network surface and has neither a command to accept nor a view to project is refused, naming what
  is missing.

  **A server and its contract cannot disagree about a path.** `ess_gen::http` holds one route
  mapping and one status mapping, and the published document, the Rust server and the Go server all
  read them. `GET /openapi.json` serves the committed contract and `GET /docs` the committed
  Markdown, both embedded at emission rather than rebuilt at run time — a server that regenerated
  its own contract could publish one nobody reviewed. A path the contract does not declare is a
  404, a declared path under another method is a 405, a body the schema refuses is a 400, and an
  obligation nothing has satisfied is a 501 naming it; none of the four is a status the contract
  declares, because each is a fact about a transport rather than about a command.

  **Neither tree takes a dependency**, and neither chooses a realization. Rust serves over
  `std::net::TcpListener`; Go over `net/http` and `encoding/json`, with generated codecs beside the
  types because a generated Go type carries an unexported field `encoding/json` cannot see. The
  hand-written halves live outside `generated/` as they always have —
  [`examples/gatepass-realization/`](examples/gatepass-realization) and
  [`examples/gatepass-go-realization/`](examples/gatepass-go-realization) — each with a linker that
  resolves exactly one implementation per obligation and names both rather than choosing when two
  are offered. The two were written from the specification rather than from each other, which is
  what makes "they answer the same way" a claim about the specification.

  **The startup record splits what the model determines from what the process does.** Everything
  outside a declared `runtime` member — the system, its version, both digests, the components, the
  plan's disposition counts, the served component, its reach, the transport, and every route — is
  derived from the specification and must be identical in every language. `runtime` carries the
  language, the address and the port. The gate *removes* `runtime` and refuses a line that has
  none, rather than comparing a list of members, so a fact the record gains tomorrow is compared
  without anyone editing the comparison.

  The browser target refuses this transport out loud rather than emitting one: a page holds the
  system in one tab and binds no socket, so a network surface is one a page would call rather than
  contain. `task check` gains no step — `synth-check` grew a fifth reason to fail.

- **`protocol ess synthesize --target web` — the billing system in a browser, and the third
  emitter behind one plan (ESS wave 7, W7.3b).** The same specification now synthesises a
  `WebAssembly` bridge and the page that drives it, committed under `generated/web/` and
  drift-checked in the gate. Open it and you can send any declared command with a typed form,
  watch the outcome it took, read the event log the transport published, redeliver an occurrence
  to see the duplicate `at_least_once` explicitly permits, watch the binding invoke the next
  command with the input it filled, and read every declared view's rows — all of it from the
  model. **Nothing about any system is typed into the HTML.** The command list, the input controls,
  the event names, the views and the lifecycles are all built at load time from a `catalog.json`
  the module carries, so a specification that changes changes the page in the same regeneration
  rather than leaving one artifact nobody regenerated.

  The plan did not change to admit a target that is not a language: `PLAN.md` and `plan.json` are
  byte-identical in all three trees. **No `wasm-bindgen`, no `wasm-pack`, no build tool and no
  third-party crate** — the boundary is three exported functions passing JSON over linear memory
  with fifty lines of hand-written glue, because `cargo build` inside the emitted tree is a
  gate step and a step that resolves a crate is a step that reaches the network.

  **The bridge chooses no realization.** Built on its own, every command answers with the typed
  refusal naming the obligation the plan owes, and the page shows that obligation's contract beside
  it — which is the honest empty state rather than an empty screen. A host crate that links one
  implementation per obligation and exports `ess_realize` turns the same page into a running
  system; `examples/billing-web/` is that host, forty lines, and gap register D-2 is untouched.

  **What a browser cannot carry is written down.** `TARGET.md` beside the plan carries six
  weakenings, none of them about a language: a `#[no_mangle]` export is flagged by rustc's own
  `unsafe_code` lint, so this one generated crate cannot declare `#![forbid(unsafe_code)]` (it
  contains no `unsafe`); a JSON boundary carries no type parameter, so an illegal lifecycle move is
  a run-time refusal here rather than a build that failed; instances are observable only as far as
  a declared view projects them, because the synthesised system holds no entity store; an
  `Integer` past 2^53 is rounded by the browser, not by the bridge; the tree is a front end over
  the Rust target's crates rather than standalone; and redelivery is a request a person makes,
  because nothing here advances a clock. A command no component accepts is a **target-stage
  refusal**: the page lists it, says why there is no form, and emits nothing to dispatch it.

  The gate builds the module for `wasm32-unknown-unknown`, checks that the page calls exactly the
  exports the module has — a page naming an export that does not exist is HTML's version of a
  dangling reference, and nothing in a browser would refuse it — and then loads the realized module
  outside a browser with Node, through the page's own `bridge.js`, and holds seventeen claims about
  one round trip. `task check` now needs the `wasm32-unknown-unknown` target and Node beside the Go
  toolchain, and says which is missing rather than skipping.

- **`protocol ess synthesize --target go` — a second emitter, and the proof that the synthesis
  plan is language-neutral (ESS wave 7, W7.3).** The same specification now synthesises a
  standard-library-only Go module beside the Rust workspace, committed under `generated/go/` and
  drift-checked in the gate along with `gofmt -l`, `go build ./...` and `go vet ./...`. The plan
  did not change to admit it: `PLAN.md` and `plan.json` are byte-identical in both trees. Go was
  chosen because it has no sum type, so every tagged union, enum and command outcome had to be
  encoded honestly — a **sealed interface**, one unexported marker method per variant set, which no
  other package can join — or refused out loud. A lifecycle becomes one type per state with
  transitions as methods on exactly the states that declare them, so an illegal move is a method
  that does not exist, as it is in Rust; a newtype becomes a struct with an unexported field, a
  constructor and an accessor, because `type Email string` would let an untyped constant become an
  `Email` by assignment.

  **What Go holds more weakly is written down, never silently downgraded.** Each module carries a
  `TARGET.md` (and `target.json`) beside the plan with four weakenings — a `switch` over a sealed
  interface is not checked for exhaustiveness, Go's zero value needs no constructor, refinement
  from a runtime state therefore answers `(value, ok)` where Rust's is total, and `==` is undefined
  for a type carrying a list, a map or bytes — each also stated in the generated doc comment where
  a reader meets it. Two things Go cannot represent at all become **target-stage refusals**, marked
  as such so they can never read as facts about the model: a `Map<Bytes, _>` (a Go map key must be
  comparable) and two obligation seams of one component that derive the same method name (a Go type
  has one method set). A refusal travels the way dependence does — the command that holds the
  unrepresentable input is refused, and so is the port that accepts it — rather than emitting a
  surface with one handler quietly missing.

- **`protocol ess diff` compares entities, commands, views and bindings (ESS wave 7, W7.2).** Ten
  construct families now, in the canonical order `system, type, entity, command, event, error,
  view, actor, component, binding`; 74 new typed change kinds — lifecycle moves and routes, an
  entity's identity, an outcome's guard, subject, emitted events, payload table and error, a
  view's filter and consistency promise, a binding's trigger, mapping and failure policy — each
  reported by name, none with a direction. Where a construct carries a predicate, the comparison
  is conservative canonical equality (gap register D-1, executed): two spellings the parser
  normalises to one form are no change, anything canonically different is *changed*, and whether
  the new predicate implies the old stays refused. An edit that used to arrive as an empty delta
  and put **everything** back to owed — a strengthened entity invariant, a moved `when:`, an
  erased payload mapping — now arrives as a named change, and `ess impact` narrows through it: on
  the worked revision pair (now six changes, ten scenarios) the invariant edit owes nine scenarios
  and twelve artifacts, the guard edit ten and ten, and neither owes a type schema. The
  fail-closed uncompared-family arm shrank to what still has no family — conversions, workloads,
  and each domain's naming, the last closing a fail-open gap where a domain's wire name, display
  name or summary could move without either a change entry or the catch-all firing. `ess-diff/1`
  documents are unchanged in shape; the new change kinds are additive rows, and pre-W7.2 deltas
  still read back.

- **`protocol infra view --path <bundle|ir> [--namespace <ns>]`** — the component view as one
  self-contained HTML page, written and opened (`$BROWSER`, else `xdg-open`). `infra graph`
  gains `--format html` for the same page on stdout. The page badge-colours each component by
  its worst finding and scopes findings and directions to the namespace when one is given; its
  only external reference is a version-pinned Mermaid script tag the viewer's browser fetches.
- **The infrastructure observation reads five more kinds, and analysis grows invariants,
  directions and an HTML component view (IW2.5).** Replicasets, jobs, cronjobs, pod disruption
  budgets and autoscalers join the model — each *optional* in a bundle, so a scan taken before
  the scanner grew them still validates and their absence stays `None`, never "none exist". Pod
  ownership is exact where the chain was observed (pod→replicaset→deployment, pod→job→cronjob,
  each edge's site the `ownerReferences` that states it); the `pod-template-hash` derivation
  remains as the old-bundle fallback and names itself on the edge. Six new diagnosis rules
  (`INFRA-DIAG-015`…`020`; none can fire on a bundle that did not scan its kind), per-workload
  properties widened to observed/ready replicas, registry-split images and budget/autoscaler
  coverage, `INFRA-PROP-001`…`003` invariant candidates (uniformity with exceptions carried as
  evidence, never as violations), a severity-ranked directions summary grouped by shared root
  cause, and `infra_analyze::render_html` — one self-contained page, Mermaid from a
  version-pinned CDN tag, optional namespace filter. Library-level; CLI flags follow. The IR
  model grew, so every `infra-ir/1` digest moves.
- **Every generated artifact carries a `contract_digest` — the digest of the model slice it
  derives from — beside its whole-model `source_digest` (ESS wave 7, W7.1).** The 36 projections
  under `generated/`, each committed conformance suite and each synthesised workspace are stamped
  through the one existing provenance mechanism: comment headers gain a `contract digest` line and
  the serialized forms (`x-ess-provenance`, suite provenance, `plan.json`) gain the field. The
  slice is the artifact's seed constructs closed over everything they rest on, by the same
  dependency graph `ess impact` walks; membership resolves every doubt by including more — a
  too-big slice costs a regeneration, a too-small one a false "still current". A suite document
  now requires the field on read: a pre-wave-7 suite no longer parses, and regenerating it is the
  fix.
- **`protocol ess impact` answers for the generated artifacts, not only the suite.** `--suite`
  is now optional and `--generated <dir>` reads the committed projection tree. The report —
  `ess-impact/2`; the document gained an `artifacts` section, `suite`/`invalidation` appear only
  when a suite was given, and the churn counts `generated_artifacts_total`/`_owed` — narrows "the
  specification moved, everything is owed" to the artifacts whose slice the delta reached, one
  path hop per line, under wave 5's exact polarity: an artifact absent from the answer was not
  reached, never "still current". Everything the analysis cannot follow is owed, stated as such —
  unreadable provenance (every pre-wave-7 artifact included), a contract digest the slice does not
  compute, a committed file the model derives nothing at, a derived artifact the tree lacks. A
  *suite* whose contract digest its own model does not compute is refused outright, because the
  short list it would produce looks exactly like a correct one.

- **`protocol infra graph` — the observed cluster as a typed dependency graph.** Edges exist
  where a reference resolved — service→workload by selector match, ingress→service,
  workload→configmap/secret/claim/service-account per env, `envFrom` and volume site,
  statefulset→governing service, pod→node, pod→workload — each under one of ten closed
  relations and carrying the sites in the dependent that state it. Deployment pods are tied to
  their deployment through the `pod-template-hash` label without observing ReplicaSets; a pod
  whose controller cannot be derived on that evidence is a typed underived-owner fact with the
  reason, never a guess. `--format json` is the canonical document (nodes, edges, sites,
  ownership facts, and the source IR's digest); `--format mermaid` draws the configuration
  topology grouped by namespace and leaves the runtime layer to the JSON; `--namespace`
  restricts either.
- **`protocol infra diagnose` — what is wrong, typed and coded, and never a refusal.**
  Fourteen rules, each finding under a stable `INFRA-DIAG-001`…`014` code with a severity
  registered on the code (error / warning / info) and named evidence: dangling selectors,
  missing required vs. optional references (an absent optional ref is info, a required one is
  an error), containers without resource bounds or probes, `:latest`/untagged images, single
  replicas, containers stuck in `CrashLoopBackOff` and kin, high restart counts,
  controller-managed pods that are not ready, orphaned configmaps/secrets/claims, unbound
  claims, and duplicate service selectors. The command exits 0 whatever the findings say — a
  diagnosis is a report about a cluster that is allowed to be wrong, not a gate; exit 1 means
  the input itself was invalid. `--min-severity` filters the listing and keeps the totals.
- **A persisted `infra-ir/1` document can be read back — through validation, never
  `Deserialize`.** `graph`, `diagnose` and `inspect --properties` accept either a bundle or a
  compiled IR document; the read-back re-verifies the digest (`INFRA-IR-002`) and re-minted
  handle by handle refuses any hand-written `resolved` reference whose key its map does not
  hold (`INFRA-IR-004`), so a forged document is refused instead of panicking a total lookup.
  Graphing the bundle and graphing its committed IR are byte-identical.
- **`protocol infra inspect --properties`** — per workload, the observed invariant-like facts
  IW3 will diff a desired state against: replica count, images parsed into
  repository/tag/digest, and the request/limit envelope per container.
- **The observation model reads two more runtime facts**: a waiting container's reason
  (`CrashLoopBackOff`, kept verbatim) and a claim's phase (`Bound`/`Pending`/`Lost`). Both are
  digested semantic state, so IRs compiled before this change carry a different digest.

- **`protocol infra validate | compile | inspect` — a scanned cluster becomes a validated,
  content-addressed IR.** An external scanner (`infra-scout`, its own repository) writes an
  `infra-observation/1` bundle; `validate` refuses a broken one with every problem in one run,
  each under a stable `INFRA-` code; `compile` turns a valid one into an `infra-ir/1` document —
  `BTreeMap`-normalized, references resolved to compiler-minted handles, danglings carried as
  typed unresolved facts rather than refused, digest = full SHA-256 of the canonical model bytes,
  provenance (`scanned_at`, `context`, `scout_version`) outside the digest so two scans of an
  unchanged cluster address the same content; `inspect` summarizes either format and refuses a
  persisted document whose digest no longer matches its content. The boundary is deliberate:
  the scanner holds the credentials, and nothing in this workspace reaches a cluster or a
  network — an observation arrives as a file or not at all.
- **A bundle carrying a plain-string secret value is refused (`INFRA-SECRET-001`), and the
  refusal never echoes the value.** The scanner already writes secrets as `{sha256, length}`;
  this rule is the second, independent enforcement, so a secret value cannot enter the IR even
  through a bundle the scanner never touched. Configmap values are digested the same way at
  validation — keys and change-detection survive, content does not.
- **`examples/k3d-dev-cluster/`** — a trimmed, reviewed observation derived from a real k3d
  scan, and the committed IR it compiles to, drift-checked in the gate (`task infra-check`,
  `cargo xtask infra --check`) and in its own CI job.

### Changed

- **A stale contract digest fails the gate as its own finding.** `generate-check`, `suite-check`
  and `synth-check` now read the contract digest out of both the committed and the freshly
  generated artifact, and a mismatch is reported as *a false claim about the model slice it
  derives from* — beside, not instead of, the byte-drift message. Same three steps, no new step.

## [0.6.1-ess-wave-6.5] — 2026-08-21

### Changed

- **The model digest is the full SHA-256 — 64 hex characters, not 16 (gap register D-4).** Since
  gate G19 a task's completion can rest on a conformance record's `spec_digest`, and since wave 5
  `protocol ess impact` refuses a suite whose digest mismatches; a 64-bit truncation is fine
  against drift and weak against construction, so the width had to follow the responsibility.
  Every provenance header, committed projection, conformance suite and synthesised workspace now
  carries the full digest, regenerated in one pass. A record written before the widening still
  parses — `SpecDigest` accepts 16 to 64 characters — and is refused where it always would be: at
  the comparison, which names both digests so the holder knows what to re-run.

### Added

- **The three invariants that were enforced by nothing are now enforced** (wave 6.5 chunk A, gap
  register): an engine that constructs an evidence payload outside its test code fails the build
  (`aep-engine/tests/evidence_scan.rs`, invariant 7); a clock, RNG or unordered map in `aep-domain`
  or `ess-gen` fails the build (`tests/determinism.rs` in each, invariant 8 — `ess-diff` and
  `ess-synth` already scanned themselves); and a second write path on the contract — any new public
  trait method beside `CommandService::execute` and the seven queries — fails the build naming
  itself (`aep-contract/tests/write_surface.rs`, invariant 14). Every scan carries an inverse
  assertion, so a scan that silently stops seeing violations fails instead of passing.
- **Property-based testing, phase 1 (`proptest`, dev-only, fixed seed).** The Kleene laws of
  `Truth` hold over generated expressions (`aep-domain/tests/truth_laws.rs`), and any generated
  adversarial specification is either refused with at least one reason or compiles to byte-identical
  canonical JSON twice — no panic, no hang, no third outcome
  (`ess-compiler/tests/adversarial.rs`). Seeds are fixed so the gate cannot be flaky; raise
  `PROPTEST_CASES` to widen a local run.
- **An outcome can declare where an emitted event's payload comes from** (wave 6.5 chunk B, gap
  register). `payload:` on a command outcome maps an event's fields onto the command's input
  (`amount: input.amount`) or a literal, with the binding mapping's own discipline read in the
  other direction: target field checked against the event, types checked with the same
  declared-conversion escape hatch (`ESS-COMMAND-002`, and the new `ESS-COMMAND-003` for a field
  the event does not carry), duplicates refused while the document form can still show them. The
  block is optional per field, and the absence is a statement: an undetermined field — a minted
  identity — is asserted for presence and type and never for a value, and there is no
  `unmapped_payload_field` refusal. Synthesis asserts the declared values, which closes the one
  fault the matrix recorded as caught by nothing: `wrong-event-payload` is now caught by
  `billing.invoice.Invoice/transition/settle/by/billing.invoice.PayInvoice/settled`, blast
  radius 2.
- **A value object's invariants are read at the field positions that hold one** (design §20's last
  unsynthesised slice, wave 6.5 chunk B). New scenario family
  `<type>/invariant/at/<view>/<field>`: the type's own predicate rebased onto each observable view
  position — `Money`'s `amount >= 0` becomes `total.amount >= 0` — required of every row with at
  least one row demanded. Billing's suite grows 27→29 and its refusal count drops to zero; what
  has no witness keeps a refusal under the honest new cause (`ESS-SYNTH-013`) instead of "not
  synthesised yet". A new deliberate fault, `negative-projected-total`, corrupts one projection's
  rows and is caught by the scenario at exactly that position while the sibling position stays
  green.
- **`ess impact` fails closed on a change the delta cannot see** (mechanism 6). The construct
  families wave 5 deliberately does not compare — entities, commands, views, bindings, conversions,
  topology — are checked for canonical equality, and any difference owes the whole suite: a
  payload-only change arrives as an empty delta and `Invalidation::Whole
  { because: uncompared-family-changed }`, never as a narrowing to nothing. The arm shrinks by
  construction as W7.2 teaches the delta each family.

## [0.6.0-ess-wave-6] — 2026-08-20

### Added

- **The generated code passes the generated tests — wave 6's criterion, executed.** The committed
  billing suite (`suites/generated/billing/suite.json`, exactly as wave 4 wrote it, 27 scenarios,
  digest-checked against the workspace's plan) now runs against the synthesised workspace linked
  with hand-written obligation implementations, and passes 27 of 27. The falsifiability half runs
  beside it: one obligation implementation deliberately corrupted — `accepts-any-amount`, the
  `CreateInvoice` guard dropped — and the same unchanged suite fails exactly
  `billing.invoice.CreateInvoice/outcome/rejected`, with a blast radius of one. Both halves are
  part of `synth-check`, so CI executes the criterion rather than trusting it.
- **`examples/billing-realization` — the hand-written half of the synthesised workspace.** One
  implementation per obligation in the generated `PLAN.md`, written by reading
  `examples/billing/`: the amount guard on the wire rendering (never a float), lifecycle moves
  through the generated typestate, both view projections, the provider stand-in for `SendEmail`,
  and the escalation that records the delivery that was given up on. Hand-written code satisfies
  generated interfaces by import and never enters `generated/`.
- **The linker never chooses (gap register D-2).** Assembling the system takes exactly one offered
  implementation per obligation: zero offers is an unsatisfied obligation, two is an ambiguity
  error naming both claimants, and refusals accumulate — a linker with three empty slots reports
  three. The linker's obligation list is held equal to the committed plan by a test.
- **The generated transport is now observable where a conformance run needs it.** The system crate
  records every command a binding invoked with the input it passed (`BindingInvocation`, read by
  the runner's mapping check), and grows `redeliver` — one already-published occurrence delivered
  to its bindings again, the duplicate `at_least_once` permits, without publishing a second
  occurrence.

### Fixed

- **A binding's failure policy now answers the declared refusal, not an unfinished workspace.**
  The generated delivery arm matched `is_err()`, which conflated the provider refusing an address
  (the declared `failed` outcome) with the behaviour behind the port not being implemented yet
  (an `UnmetObligation`). It now matches the outcome enum: an error-carrying outcome takes the
  declared policy — escalate, retry, or drop — and an unmet obligation propagates out of `pump`,
  because escalating it would publish a domain event no domain fact caused. Found by W6.3's suite
  run: under the old shape a forced `SendEmail` failure produced no `DeliveryEscalated`, and
  `notify-on-invoice-created/binding/on-failure` failed.

- **`protocol ess synthesize` now emits component skeletons and one transport.** The plan's scope
  grows from `semantic-types` to `component-skeletons`: each component becomes its own generated
  crate whose port is the specification's declared surface — accepted commands as typed handlers,
  declared views as typed queries, published events as a typed outbox — and a system crate wires
  the bindings over the one transport the specification's own words determine (`at_least_once`,
  in-process, standard library only; the log of published events is the observable record). On
  billing, three of the interaction-layer refusals become generated — the binding's
  transformation and delivery, and both component ports — so the plan moves from 43 capabilities
  (29 generated / 7 obligations / 7 refused) to 45 (33 / 8 / 4). A binding is now three
  capabilities with three honest dispositions; the new obligation is the escalation, because the
  declared `DeliveryEscalated` event says nothing about how its fields are filled. A binding
  whose command zero or several components accept is refused rather than routed by guesswork.
- **Every obligation is now a typed stub in the generated workspace.** Each owed behaviour, query,
  conversion, transformation and escalation gets a trait beside its contract and an
  `Unimplemented` implementation whose body returns `UnmetObligation { capability, source }` —
  a value naming the plan entry, never `todo!()` and never a panic — so a workspace built
  entirely on stubs compiles and reports exactly what it cannot yet do. The plan's obligation
  list and the workspace's stub set are held to a bijection by the emitter and by a test.

- **`protocol ess synthesize --path <spec> [--out <dir>] [--target rust]`** — the part of an
  implementation that was never yours to write, plus the typed list of exactly what remains. Every
  semantic capability of the specification gets exactly one disposition in a language-neutral
  `SynthesisPlan`: **generated**, **obligation** (the contract is declared, the behaviour is yours —
  with the reason, in the specification author's own words where the spec declares one), or
  **refused** (with the reason, and the stage that refused). Zero guessed business logic: on the
  billing example the plan holds 43 capabilities — 29 generated, 7 obligations, 7 refusals — and
  `calculate_tax`-shaped inventions are unrepresentable, because no disposition means "generated,
  roughly".

  The Rust emitter writes a standalone zero-dependency workspace: newtypes distinct from their
  representations, tagged unions as enums, events and declared errors as types, one outcome enum
  per command with the refusal branches beside the successes, views as row types — and lifecycles
  as typestate, where the transition the specification refuses is a method that does not exist:
  `Paid → Cancelled` on the billing invoice does not compile. `PLAN.md` and `plan.json` travel
  inside the workspace. `--target` takes `rust` today; the plan itself never names a language.
- **`generated/rust/billing/` is committed and gated.** `cargo xtask synth` regenerates it,
  `cargo xtask synth --check` — a new step in `task check` and its own CI job — fails on a
  byte-level drift from the specification *and* runs `cargo check` inside each committed workspace,
  so "it compiles" is executed rather than claimed. `Cargo.lock` and `target/` inside a generated
  workspace are the toolchain's, ignored and never committed.

## [0.5.0-ess-wave-5] — 2026-08-20

### Added

- **`protocol ess impact --from <dir> --to <dir> --suite <file>`** — which scenarios a change
  invalidates, and *why*. Every impact carries the path that produced it, one hop per line — `type
  Money has a field of type type Currency` → `type Headline wraps type Money` → `entity PriceList has
  a field of type type Headline` — because an impact nobody can explain is an impact nobody acts on.

  It narrows what has to be re-established and can never widen what survives. Marking something still
  valid is not a thing the code can express: there is no such verdict in the vocabulary, the only
  combinator is a join whose top element is "invalidate the whole thing", a change to the system
  header invalidates everything, and a dependency the graph does not recognise invalidates everything.
  A suite whose digest does not match the earlier revision is refused rather than narrowed.

  On the normative example, moving an actor's grant narrows 27 owed scenarios to 7. Changing an enum
  variant narrows 27 to 23, and that is worth knowing rather than hiding: nearly every scenario acts
  on an entity, so a type most entities reach is genuinely reached by most scenarios. Authority
  changes are where the narrowing pays; type changes are where it barely does.
- **`protocol ess diff --from <dir> --to <dir>`** — what actually moved between two revisions of a
  specification, as typed changes rather than as text. On the worked fixture pair, 208 changed lines
  across three files, one of them renamed, reduce to **four** semantic changes: renaming a file,
  reordering blocks, rewriting a comment and writing out a default that was already implied all reach
  nothing, and each of those is asserted by name rather than left to chance.

  Six construct families are compared field by field — the system header, types, events, errors,
  actors and components — with 65 typed changes and no untyped catch-all. Entities, commands, views,
  bindings and topology are deliberately left out of this first slice: comparing their invariants and
  conditions means comparing predicates, which is where an undecidable answer lives.

  A change carries a direction where one can be derived mechanically and only there: a grant added or
  an enum variant added *widens*, either removed *narrows*, and everything else is simply changed.
  Three relations rather than the seven the design proposed, because four of them could not fire in
  this slice, and a variant nothing can produce is the same defect as a test that cannot fail.

## [0.4.0-ess-wave-4] — 2026-08-20

### Changed

- **A command outcome that changes an entity must say which instance.** `creates:`, `moves:` and
  `updates:` named the entity; `instance:` now names the field carrying its identity, and an outcome
  with a subject and no instance is refused. **This will refuse a specification that used to be
  accepted** — every state-changing outcome needs one word added.

  The reason is a measurement rather than a preference. A generated conformance suite could not test
  a single lifecycle transition without it: `PayInvoice` settles *an* invoice, and nothing connected
  its input to that invoice's identity, so twenty-eight scenarios across the two example
  specifications refused to generate rather than fabricate an id — and a fabricated id fails a
  *correct* implementation, which is worse than generating nothing. With the link declared, those
  twenty-eight became scenarios.

  It is declared rather than inferred, because inference has no answer when a command carries two
  fields of the identity's type and no answer when it carries none — and because an inferred link
  would silently change which scenarios exist when someone adds an unrelated field, while stored
  conformance results are keyed on exactly those names. It hangs on the outcome, not the command,
  for the reason the subject does: a command's branches disagree about what they touch, and a
  command-level key would attach an instance to a refusal.

  `creates:` is the exception and points at an event rather than the input: a created instance does
  not exist when the caller calls, so its identity is published rather than supplied.

### Added

- **A specification generates its own conformance scenarios.** All five families: one per reachable
  command outcome with the refusal branch asserting the success event did *not* occur, an externally
  decided branch reached by configuring the fault rather than by an input, a lifecycle transition
  proved and an illegal one refused, an entity's invariants checked after each state-changing command,
  and a binding checked for its mapping, its delivery guarantee and its failure policy. The normative
  example yields twenty-seven scenarios and the oracle fixture thirty-one. Nothing executes them yet.
- **The generated suite is checked against implementations that are deliberately wrong.** Ten faults,
  each injected one at a time: a wrong event, an accepted invalid amount, an illegal transition
  allowed, a dropped binding, a swapped mapping, a stale read-your-writes view, an ignored external
  outcome. Seven are caught by the scenario that exists to catch them — named, not merely "the run
  went red" — and the matrix asserts each fault's blast radius against an allowance, so a suite that
  starts over-reaching fails rather than looking thorough.
- **A command can say what happens when it is attempted in the wrong state**, and an author writes
  only the error. `wrong_state: true` with an `error:` is a fourth kind of outcome beside a guarded
  branch, a default branch and an externally decided one. The *states* are not written down: the
  lifecycle already says which states each transition may be taken from, so everything else is wrong
  by construction — add a `from:` to a transition and the branch narrows without anyone editing a
  second list.

  Until now a generated suite could only check that something went wrong, not that the right thing
  went wrong. An implementation that refuses with the wrong error passed all twenty-seven scenarios of
  the normative example; it now fails the scenario that exists to catch it. Omitting `wrong_state:` is
  still valid — the scenario is still generated, and the suite says plainly that the specification
  declares no answer for it.

  For anyone generating contracts: the branch surfaces in OpenAPI as `409`, not `422` — the caller's
  request was well formed, and telling them to fix it would send them looking for a mistake they did
  not make.
- **Two of those three faults are now caught.** A command may no longer announce an event belonging to
  a branch it did not take: every event the specification declares and the branch does not emit is
  asserted absent, scoped to that invocation. And a read-your-writes view whose command returned no
  consistency token is no longer quietly read at whatever is current — the check fails, naming the
  command that owes the token, because a weaker read that passes is a skip wearing a pass's clothes.
- **An event's payload is checked for shape.** Every declared field must be present and of its
  declared type, down to the leaves. Its *value* still is not, and cannot be: nothing in the model
  relates a command's input to an emitted event's payload, so `InvoiceCreated.amount` matching
  `CreateInvoice.amount` is a coincidence of field names rather than something the specification says.
  Closing that needs a construct in the shape `mapping:` already has, and until then the fault stays
  recorded as uncaught with its reason narrowed.
- **A view assertion names the instance the scenario acted on**, rather than meaning "the view holds
  some row". The weaker form was correct only because scenarios are isolated, and would have passed
  against a shared target for reasons unrelated to the rule being tested.
- **Three faults are caught by nothing, and the matrix records that too.** An event may be published
  with any payload, and a command may announce an event belonging to a branch it did not take, because
  synthesis asserts an event by name and writes no payload; and a target that returns no consistency
  token gets a weaker read instead of a reported failure. Each is recorded as an uncaught fault with
  the reason, and the test asserts it is *still* uncaught — so closing one of these holes breaks the
  row rather than being quietly forgotten.
- **`protocol ess conform`** — `synthesize` writes a suite from a specification, `run` executes one
  against an implementation. It can run the two reference implementations this repository ships, and
  its help says outright that it cannot run yours, with the four-line adapter recipe rather than an
  implication that more is there. Exit codes distinguish the three answers that matter: `0` conformant,
  `1` the implementation contradicted the specification or could not expose something required, `3` the
  run could not be carried out at all — because telling a harness the system is wrong when nobody found
  out is its own kind of lie.
- **The generated suites are committed and drift-checked**, under `suites/generated/`, as a seventh
  step of the gate and a CI job of its own. They sit beside the projections rather than inside them,
  because that tree has one owner and an orphan scan that deletes what its owner did not produce — two
  writers there would each delete the other's committed contract.

  The committed index also lists every construct that got **no** scenario, with the reason. A suite
  quietly holding fewer checks than it used to is the one failure a passing run cannot show you, and
  now it is a line in a diff.
- **A generated suite runs against an implementation.** A `ConformanceTarget` offers nine methods,
  each traceable to something the specification declares — execute a command, query a view, observe
  events, configure an externally decided outcome, redeliver an event, isolate a scenario. There is no
  assertion method and no escape hatch: if a step cannot be executed through concepts the model
  declares, that is a finding about the model rather than a method on the trait. All twenty-seven
  scenarios of the normative example pass against a hand-written reference implementation, and two
  runs produce byte-identical reports, because the runner owns the clock and the id source and nothing
  beneath it reaches for an ambient one.
- **A scenario the target cannot observe fails the run rather than passing quietly.** `unsupported` is
  its own status beside `passed`, `failed` and `error`, and a required scenario that ends in it makes
  conformance fail — a skip that reads as a pass is how a suite comes to certify what it never checked.
- **A binding's promises are each a test.** The mapping is asserted field by field, so a swap between
  two same-typed fields is caught rather than passing. `at_least_once` delivers the event twice and
  requires the consequence to survive it — not to happen exactly once, which is the assertion that
  looks right and fails a correct at-least-once handler. An escalation asserts the event the model
  now requires it to name.
- **`on_failure: drop` generates a refusal rather than a scenario**, saying so in the suite: a policy
  that gives up silently publishes nothing, so there is nothing to assert, and the hint says to write
  `escalate:` if it has to be provable. The refusal is the honest output — a scenario would have to
  invent an observation the specification declines to make.

- **`protocol ess graph --format mermaid`.** The system graph as a Mermaid flowchart, unfenced, so it
  can be piped into a Markdown file, a docs site or a pull request without going through the generated
  documentation tree. `dot`, `json` and `yaml` are the other spellings; `--format text` still means DOT
  and is kept as an alias of it.

### Fixed

- **An artifact evidence record could not be written in a document at all.** The evidence envelope is
  tagged by `kind`, and this one kind of record also had a field called `kind` — so the parser
  consumed the key as the tag and then reported the field it had just consumed as missing. Every
  attempt failed with `missing field 'kind'`, however it was written. The field is `artifact_kind` on
  the wire now.

  The consequence was wider than one record type: `design-by-contract` and `preserve-evidence` both
  require artifact evidence, so no `development.critical` task could satisfy either through a
  document, and none could reach `implement`. The variant existed, was documented, appeared in the
  published schema, and was unreachable from the one place a person writes evidence.
- **The CLI and the documentation page were drawing two different system graphs.** The command line
  showed no actors and no grants at all, and it grouped a command by which component *owns* its domain
  while the page grouped by what a component *accepts* and *publishes* — and the model allows those to
  differ, since a component may accept a command from a domain it does not own. Two pictures of one
  system, from two code paths, with nothing comparing them. There is now one renderer and a test that
  runs the real binary and the real generator and requires their output to match.

## [0.3.3-ess-wave-3.5] — 2026-08-20

### Added

- **A command outcome says which entity it acts on, and a transition nobody takes is refused.** An
  outcome declares `creates:`, `moves:` or `updates:`, so `CreateInvoice.accepted` creates an invoice
  and `CreateInvoice.rejected` creates nothing — the distinction lives on the outcome because a
  subject on the command would attach a state change to a refusal. A lifecycle transition no outcome
  takes is now `missing_causation`: it is a state change nothing can trigger, which is the lifecycle's
  version of a type no value can inhabit, and the refusal names the outcome that could take it.
- **The published schemas accept every spelling the parsers do.** `component:` beside `name:` in a
  specification, `id:` beside `name:` on a binding, `type:` beside `kind:` in a task, `require:` beside
  `requires:` in a workflow, and fourteen more. An editor loaded with
  `schemas/generated/ess.schema.json` marked this repository's own normative example invalid, and
  offered no fix, because the spelling it objected to was the spelling the guide's examples use. The
  aliases were always deliberate; the schema simply did not know about them, since a `#[serde(alias)]`
  is invisible to schema generation. Fifteen of the seventeen were in documents nobody had checked.
- **Conformance evidence is bound to the revision it was produced against.** A run against yesterday's
  specification no longer satisfies a requirement about today's, and a specification artifact that
  records no model digest is conformed to by nothing. The second half is deliberate and is the
  uncomfortable one: unproven is not proven, so a specification whose artifact carries no digest leaves
  its conformance requirement permanently owed until someone records one. The alternative — treating an
  unrecorded digest as "probably fine" — is how evidence outlives the thing it was evidence for.
- **`ess-conformance`** — the one piece the verification oracle cannot start without: a candidate
  command input projected into facts, and a guard decided against it. It answers with four outcomes
  rather than a boolean, because "this value does not satisfy the guard" and "this guard cannot be
  decided at all" are different answers and only the first means *try another value*. A guard ordering
  two pieces of text with no declared scale, or reading a path no type declares, is unevaluable — and
  saying so is the point, since treating it as a failure would report a specification's defect as a
  flaky test.
- **A binding that escalates must say what that emits.** `on_failure: escalate` on its own is now
  refused: write `escalate:` with `emits:` naming a declared event. "Surface it to a person" is not
  something a conformance target can be asked to prove, so a failure policy that said only that was a
  promise nobody could be held to. `retry` and `drop` are unchanged and stay single words — a retry is
  observable as another invocation, and a drop is unobservable on purpose, which is the whole reason it
  has to be typed out.
- **A property-test result carries the seed that reproduces it.** A counterexample you cannot re-run
  is a bug report without a repro, so `seed` is now part of the record — an opaque string rather than a
  number, because proptest, Hypothesis, fast-check and a fuzz corpus each spell a seed differently and
  a numeric field would force three of them to encode a lie.
- **Conformance evidence names the specification it attests**, by digest and not by a free-text string.
  A record that cannot say which specification produced it proves that some implementation passed some
  suite; it cannot prove that the implementation in front of you conforms to the specification in front
  of you.

### Changed

- **`version: 4294967296` is refused rather than silently becoming `4294967295`.** The two spellings of
  a version now agree: `v4294967296` was already refused, while the numeric form saturated, so two
  documents that disagreed about a version compared equal.
- **A YAML mapping key written twice is refused in every document this repository reads.** It was
  already refused in a specification; a protocol, principle, workflow, profile or lifecycle silently
  kept the last of the two. A profile that granted a capability twice lost one of them with no
  diagnostic.
- **A number a document cannot round-trip is refused.** `1e400` parses as an infinity, and JSON has no
  spelling for one — so it was published as `null`, turning a guard the author wrote into a guard
  nobody wrote. `.nan` likewise slipped past the constructor into a type whose documentation promises
  it cannot exist, which made ordering unreliable for every comparison against it.
- **A type or predicate nested deeper than 32 levels is refused instead of overflowing the stack.** A
  refusal names the construct and the limit; the abort it replaces named nothing.

### Fixed

- **A refused approval no longer authorises the action it refused.** A reviewer who read a change,
  refused it, and recorded that refusal was granting the production write — at three separate places in
  the engine. Also: a capability a principle denied could be downgraded to merely requiring an
  approval, an approval floor on `deployment.create:production` did not catch a profile granting the
  broader `deployment.create`, and the audit trail accepted a record that claimed a refusal and listed
  the rows it changed.
- **A validated type can no longer be conjured from a document.** Adding `Deserialize` to a type that
  is supposed to be reachable only through validation compiled and passed every check; the invariant
  every other guarantee rests on was enforced by nothing. It is now enforced mechanically.

## [0.3.2-ess-wave-3] — 2026-08-20

### Added

- **The specification now produces the documentation and the contracts, and they are in the
  repository.** [`generated/`](generated/) holds 27 files projected from
  [`examples/billing/`](examples/billing/): Markdown with Mermaid diagrams, one JSON Schema per
  command input, event payload, error payload and named type, an OpenAPI 3.1 document per component
  and an AsyncAPI 3.0 document per component. Committed rather than built on demand, because a
  contract a consumer cannot read without first installing a toolchain is a contract they copy by
  hand — and once it is committed it can be checked, so a specification change nobody regenerated
  fails the build instead of shipping a document that describes last week's system.
- **Every generated artifact says which specification produced it.** The system and its version, a
  digest of the resolved model, the compiler version and the generator version — at the top of every
  file, as a comment a person reads and as data (`x-ess-provenance`) a tool reads. When two checkouts
  disagree about an OpenAPI document the only question anyone asks is which of the two is stale, and
  the answer is now in the file rather than in whoever remembers running the generator. The digest is
  over the resolved model, not the source text, so it does not move when a comment does.
- **A named type stays a named type in every projection.** `Email` and `EmailAddress` are both a
  `String` underneath, and a projection rendering both as `{"type": "string"}` throws away the one
  distinction the model exists to make. Each keeps its own definition, its own reference and its own
  name in the schemas and in both contracts, so a code generator reading them emits two types. The
  limit is stated rather than papered over: on the wire both are a bare JSON string, so **an instance
  with the two values swapped still validates** — JSON Schema constrains structure and cannot carry
  nominal identity.
- **Where an OpenAPI path or an AsyncAPI channel comes from is a stated convention, and the generated
  document states it.** The model has no `exposures:` or `transport:` construct, so nothing in a
  specification names a method, a path, a status or a topic. Rather than invent one silently, each
  generator writes its rule into the document it produces. A command is always `POST`, at
  `/{domain wire name}/commands/{command wire name}` — `/invoices/commands/create-invoice`, with the
  `commands` segment there to stop the path pretending to be a resource, and the command's qualified
  name as the `operationId`. An event's channel address is its declared `naming.wire` or else its
  full qualified name, and every channel carries `x-ess-address-source` so a reader can tell an
  address somebody chose from one that was derived. Each of those is a rule a reviewer can disagree
  with, which is why it is written down; when `exposures:` exists it should override the convention
  rather than replace it.
- **A status code comes from the outcome, and `external` is not the caller's fault.** An outcome that
  was taken is `202`, a refusal the input decides is `422`, and a refusal decided outside the request
  is `502`. Reporting an `external` branch as a `4xx` would tell the caller to go fix the one thing
  it cannot fix and tell every retry layer in between that retrying is pointless. Outcomes sharing a
  status stay distinguishable — one response, `oneOf` the outcome schemas, each pinning its own
  `outcome` — because a status that collapsed two branches would lose the branch. `servers`,
  `security`, pagination, `201`, `ETag` and the other things an OpenAPI document usually has are
  absent: no specification backs them, and a plausible default in a contract is a claim nobody made.
- **A binding's `delivery` and `on_failure` survive the trip into the contracts.** A command some
  binding invokes with `delivery: at_least_once` gets a **required** `Idempotency-Key` header, because
  the consequence of at-least-once lands on the receiver and a surface with no way to say "this is the
  same invocation as the last one" leaves it deduplicating with no key. A command no binding invokes
  gets no header. On the messaging side both facts reach the subscriber's document, the publisher's
  document and the prose description — including `on_failure: drop`, where the work being abandoned is
  the publisher's event, so the publisher's document has to be able to say so.
- **Regenerating is byte-identical, and CI fails on a diff.** `task generate` writes the tree,
  `task generate-check` fails when the committed output is not what the specification produces, and it
  runs both inside `task check` and as a CI job of its own — "Projections up to date" — so a drifted
  contract is reported as drift rather than surfacing as an unrelated test failure. No clock, no RNG,
  `BTreeMap`/`BTreeSet` only, and a test per projection that generates twice and compares bytes.
- **A committed artifact no generator produces any more is reported as an orphan, not quietly kept.**
  A check that only compares the files a generator emits cannot see the other direction: a schema that
  was renamed or withdrawn leaves its file behind, and a consumer goes on validating against a
  contract this repository no longer stands behind. `cargo xtask generate --check` names those files
  and fails; `cargo xtask generate` removes them.
- **`protocol ess generate --kind docs|schema|openapi|asyncapi`** — and every projection at once when
  `--kind` is not given. Read-only unless `--out` is given: without it the artifacts are listed rather
  than written, because a verb that scatters files over whatever directory you happened to be in is a
  verb nobody tries twice. `--format json|yaml` carries their contents for a consumer that does not
  want a directory.
- **An entity, a view and an actor are on the generated pages.** An entity arrives with its identity
  by name and not only by type, its fields in declaration order, its invariants as the author wrote
  them, and its lifecycle as a state diagram that also lists the moves the specification does *not*
  permit — a page showing only the legal arrows reads as though the others were never considered. A
  view arrives with the entity it projects, its filter, and what its consistency level obliges a
  generated test to do: an `eventual` view asserted once races the projection, and the repair everyone
  reaches for is a sleep. An actor arrives with the commands it may invoke, drawn as edges in the
  system graph, so design §9's first arrow — somebody asking for something — is on the page instead of
  apologised for.
- **Two documents generated from one model cannot disagree about what is valid.** Every projection
  publishing a schema for a construct publishes the *same* schema for it, and a test compares them
  fragment by fragment rather than trusting that three copies of one mapping stayed equal. This
  started as a real divergence: the `AsyncAPI` document accepted an amount that was not a number and
  extra fields nobody declared, both of which the JSON Schema tree refused — so a service validating
  against one document and a service validating against the other disagreed about the same bytes. A
  difference in what a document *accepts* fails the test, and so does a difference in what it *says*
  about a construct, because a code generator reading two documents needs one answer to "which
  construct is this".
- **The published `AsyncAPI` payloads refuse what the model refuses.** They now carry
  `additionalProperties: false`, the `Decimal` pattern, the `Uuid` pattern, base64 `contentEncoding`
  for `Bytes`, `propertyNames` for a map with a non-string key, `anyOf [T, null]` for an optional
  outside a field, and a tagged `oneOf` for a union — so a branch is decidable rather than guessed. If
  you were validating events against the previous documents, messages that used to pass may now fail:
  that is the point, and each failure is something the specification never permitted.
- **An operation says which actors may invoke it** (`x-ess-may-invoke`), and no document invents a
  security scheme. `may:` states who may ask for something; an `OpenAPI` `securityScheme` states how a
  caller proves who it is, and the model says nothing about that — so a generated client would have
  implemented an authentication mechanism no specification backs.
- **A construct the documentation cannot render is named on the page where a reader went looking for
  it.** The list is an allowlist rather than a discovery, so a *new* gap fails a test and a closed one
  is a deleted entry that changes the pages with it. It is currently empty: every construct the
  specification language has reaches the IR and reaches a page. A page that quietly leaves an entity
  out reads exactly like a system that has none, which is why the empty list is a test and not a
  claim.
- **An entity, a view and an actor survive compilation.** The resolved IR carries an entity's
  identity field with its name, its fields in order, its invariants and its lifecycle; a view's source
  entity, filter, exposed fields and consistency; and an actor's grants as references that cannot name
  a command nobody declared. Before this, a specification could declare all three and everything
  downstream saw only the set of an entity's state names — so anything derived from the model was
  derived from a fraction of it.

### Not built

Test synthesis — a generated conformance suite, and an implementation deliberately wrong to prove the
suite bites — is ESS wave 4; Rust structural synthesis is wave 5. Entities, views and actors reach
the documentation but no contract projection derives from them yet: a view is a read model an
`OpenAPI` document could expose and does not, and an actor's grants are authorization rather than
authentication — the model states who may invoke a command and says nothing about how a caller proves
who it is, so no document here emits a security scheme. Every schema each document embeds is
validated against the 2020-12 meta-schema, but the `OpenAPI` and `AsyncAPI` envelopes themselves are
checked structurally rather than against the `OpenAPI` 3.1 and `AsyncAPI` 3.0 meta-schemas: neither is
vendored here.

## [0.3.1-ess-wave-2] — 2026-08-20

### Added

- **A system's decomposition, interaction and runtime shape are part of the specification.** Three
  layers above the domains, each answering something the domains cannot: which component owns which
  bounded context, what happens when an event occurs, and how many instances the design needs to be
  correct. A component is not a deployment — whether `invoice-service` ships as a process or a module
  is the topology's business, and changing that answer changes nothing in `domains/`.
- **A binding says what happens when it fails.** `delivery:` and `on_failure:` are required words, not
  defaults. A binding that can fail silently is the difference between specifying a system and
  specifying a demo, and the way that difference disappears is a default nobody read. `drop` is legal
  and has to be typed: a system that loses work is a decision, and the decision has to be findable in
  the document that made it.
- **A mapping between two bounded contexts is typechecked.** `InvoiceCreated.customer_email` into
  `SendEmail.recipient` is the one place two independently-written contexts must agree about a type,
  so it is the one place a rename in one breaks the other silently. Both sides are resolved, and the
  refusal names both paths, both types, and that no conversion is declared.
- **A type crossing must be declared, with a reason.** `Email` and `EmailAddress` are both a `String`
  underneath, and the whole value of naming them apart is that the model refuses to treat one as the
  other. `conversions:` records the crossings that are intended and requires `because:` — a conversion
  with no reason is exactly what this declaration prevents: a widening someone added to make a build
  pass, which the next reader finds and cannot evaluate. Crossings are directional.
- **`ess-compiler`** — resolution, a normalized IR whose type carries the guarantee that every
  reference resolves, and diagnostics with a stable code, a `file:line` and a machine-readable body.
  A `Specification` holds names that *probably* resolve; anything downstream either re-checked them or
  trusted that someone else had, and both are how a generator emits code for a type that does not
  exist.
- **`protocol ess compile`, `ess inspect`, `ess graph`.** `inspect` resolves a name in any of seven
  namespaces and refuses an ambiguity rather than guessing; `graph` emits DOT with components as
  clusters, and its output is byte-identical across runs.
- Generation is reproducible, and there is a test that says so rather than a comment: the same source
  compiled twice is byte-identical. `BTreeMap`/`BTreeSet` only, no clock and no RNG anywhere in the
  compiler.

### Fixed

- **A legitimate expression tree was refused.** A type reaching itself through a union was treated as
  a forbidden dependency cycle, but `Expr = union {leaf: Integer, pair: Pair}` with
  `Pair = struct {left: Expr, right: Expr}` is perfectly ordinary — every value of it bottoms out in
  a `leaf`. The rule now asks the question that matters, whether any value of the type can exist,
  rather than the shape that usually causes the answer to be no. A union needs one buildable variant,
  not all of them, and the refusal now names which requirement is unmet instead of only that
  something is.
- **A key written twice was silently discarded.** `serde_yaml` keeps the last of two identical mapping
  keys and says nothing, so a document declaring the same workload, type or even `system:` twice lost
  one of them. Reading now goes through a stage that refuses it, with the key and the line — one check
  covering every mapping in the format rather than one per section.
- A binding's mapping could not report an input mapped twice, because the raw form was a map and the
  duplicate was gone before anything could look.

### Changed

- Two new validation codes distinguish faults that were being reported as each other:
  `misspelled_reference` for text written where a reference was meant — `evnt.customer_email` parses
  clean and gets *sent* — and `unsupported_construct` for something this build will implement later, as
  against `unsupported_format_version` for a document it cannot read at all. "Upgrade the tool" and
  "write it another way" are different instructions.

## [0.3.0-ess-wave-1] — 2026-08-20

### Added

- **A system can be specified, and the specification can be refused.** `ess-domain` is the typed
  model for an Executable System Specification: domains, entities with lifecycles, commands with
  outcomes, events, errors, views with declared consistency, actors and a type system with tagged
  unions. `protocol ess validate --path <file-or-directory>` parses one and reports every problem in
  a single run, each with a code and a location.
- **[`examples/billing/`](examples/billing/)** — the single normative example, parsed by a test, and
  checked to exercise *every* construct the model has: each type kind, each primitive,
  `Optional`/`List`/`Map`, both consistency levels, an actor with grants and one without. A construct
  added to the model without reaching the example fails the build, because what the normative example
  leaves out is what nothing checks.
- **A command that can be refused says so.** Outcomes rather than a bare `emits` list: a command with
  a precondition has at least two results, and a specification recording only the happy one generates
  a suite that never checks the branch where the money does not move.
- **An outcome the input cannot decide says that too.** `external: <the cause>` marks a branch caused
  by the world — a mail provider rejecting an address — so a generator injects a fault instead of
  trying to construct an input for it. `when: false` would have claimed the branch was unreachable,
  which is a different and false statement.
- **A projection declares its consistency**, which is what decides whether a generated assertion is
  `eventually` or immediate — rather than a sleep, which makes a suite test the machine it runs on.
- **A declaration is addressable from outside** — `ep://acme/billing/ess-command/billing.invoice.CreateInvoice`,
  the protocol's own scheme rather than a new `ess://` one, so an approval against a command in a
  specification is recorded the same way as an approval against a design.
- **[`schemas/generated/ess.schema.json`](schemas/generated/ess.schema.json)** — an editor validates
  a specification as it is typed. Generated from the same Rust types the validator runs, drift-checked
  in CI, and the generated index now lists every published schema so one cannot land undocumented.
- **[`docs/guide/specification.md`](docs/guide/specification.md)** — how to write one, and what the
  model insists on.
- **[`docs/VISION.md`](docs/VISION.md)** — what this project is for, and how its two halves compose:
  AEP governs how engineering work is performed, ESS specifies what software must exist, and they
  meet at evidence.
- **[`docs/design/ess-implementor-design-v0.1.md`](docs/design/ess-implementor-design-v0.1.md)** —
  the Executable System Specification design: a system described once as a typed semantic model, from
  which contracts, documentation, tests, deployment artifacts and structural code are derived.
- **[`docs/design/ess-review-v0.1.md`](docs/design/ess-review-v0.1.md)** — a review of that design
  against what this repository learned building the same shape twice: eleven findings, three of which
  would make generated tests assert false things, and a narrower recommended v0.1 scope.
- **A task can require conformance to a specification.** `ArtifactKind::ExecutableSystemSpecification`,
  `EvidenceKind::EssConformance` and the `ess-conformance` principle — conditional on the project
  having a specification, and satisfied only by `independent: true` evidence from a
  `conformance-runner`. An agent's own report that its implementation matches the specification is
  not evidence that it does.

### Changed

- **A validation error names what actually went wrong.** A specification had been borrowing the
  protocol's document codes, so a duplicated command name reported `duplicate_principle` and a
  missing event reported `unknown_state`. Nine codes now say what they mean —
  `undeclared_reference`, `duplicate_declaration`, `missing_declaration`, `empty_declaration`,
  `conflicting_declaration`, `type_mismatch`, `unsupported_format_version`,
  `non_exhaustive_branches`, `unreachable_branch` — and sixteen places in the protocol half moved
  onto them too, so an undeclared reference is not one code in a specification and a different one
  in an artifact manifest.
- **The published schemas accept what the parser accepts.** Ten document types had a hand-written
  parser and a derived JSON Schema, so the schema described the *representation* rather than what an
  author writes: a bare `- verification` evidence requirement, a one-line objective, a
  `require_approval` capability, an `in-review` status. Twenty-eight rejections across eighteen of
  this repository's own documents. Every schema is now checked against every document the repository
  ships.
- `v01` and `ess/01` are refused. Both parsed, and both were rejected by the pattern the same build
  published — a document an editor called invalid and the tool accepted.

### Fixed

- **A schema that called the normative example invalid.** `version: v3` is what every document says;
  the published schema required an integer.
- **A guard that could not guard.** The list of validation codes the tests iterate was maintained by
  hand and had fallen five codes behind the enum, while its own comment claimed that adding a variant
  without listing it would fail the test. The enum, its wire strings and the list are now generated
  from one declaration.
- Rules that existed and were never reached: an error's payload types and an event's duplicate fields
  were checked by methods nothing called.
- A specification could name a domain in the header that nothing declares, declare an actor no domain
  owns, define two types that cannot be built without each other, filter a view on a lifecycle state
  the entity does not have, declare a type no value can be, or declare a union with no tag field. All
  six are refused.
- **A misspelt key in a type declaration was silently dropped.** `invarants:` on a value object
  parsed clean and lost the invariant, because a flattened body rules out `deny_unknown_fields` at the
  outer level. It is now a parse error with a line number.
- **A type's invariants are predicates, checked against the type's own fields**, as an entity's
  already were. `nonexistent_field >= 0` on a value object was accepted, and so was text that is not
  a predicate at all.
- A field name must survive into generated code as an identifier. `""` and `not a field name!` were
  accepted.
- An entity invariant may read the identity field. It could not, although a view projecting the same
  entity could — so a valid specification was refused with a message that was not true.
- A field may not shadow the identity's name, which produced two fields with one name and different
  types.
- A state whose only transition returns to itself is a dead end. A self-loop was counted as an exit,
  so an entity could reach a state it can never leave.
- A domain can be given a wire and display name. `naming:` on a domain file was refused, although the
  model has always carried it — so a bounded context's wire name was unreachable from any document.
- A malformed header no longer hides the reference errors under it.
- `protocol ess validate` names the file a problem is in when the specification is one file, refuses
  a directory that is not a specification instead of reading every YAML file it can find, and reads
  each file once when a symlink points back up the tree.
- `cargo xtask schema --check` fails on a schema nothing generates any more, not only on one that
  drifted.

### Not built

No compiler, no OpenAPI, no test synthesis: those are ESS waves 2 and 3 in
[`docs/plan/ess-roadmap.md`](docs/plan/ess-roadmap.md). Conformance evidence is produced by hand.

## [0.2.1] — 2026-08-20

### Added

- **A project can be discovered.** `.engineering/project.yaml` names the protocol, the profile and
  where the protocol tree lives; `protocol resolve` and `protocol evaluate` run with no arguments
  anywhere inside a project, walking up to find it. An adopting team's first command no longer needs
  four paths.
- **Project-local principles and profiles.** `.engineering/principles/` and `.engineering/profiles/`
  are merged over the protocol tree's, because no organisation's rules are entirely somebody else's.
  They are documents in the same format, validated the same way — and a project-local profile still
  cannot grant a capability the protocol's approval floor keeps behind approval.
- `protocol resolve` and `protocol evaluate` report where their inputs came from, so it is never
  ambiguous whether a flag or the project supplied them.

### Fixed

- **The approval floor was inert for every `adp/1` and `aop/1` profile.** `Protocol::extend` merged
  capabilities, evidence kinds, verifiers, phases, observables and scales — but not the approval
  floor, and neither derived protocol declares one of its own. A profile written against `adp/1`
  could therefore grant `production.write` outright and resolution would accept it, while three
  documents claimed that was impossible. The shipped profiles were unaffected because each
  hand-writes `require_approval`; the check meant to make the mistake impossible was doing nothing.
  Now inherited, with a regression test over the real documents that fails without the fix.
- **The CLI crashed when its reader stopped reading.** `protocol inspect | head -3` ended in a panic
  and a stack trace, because Rust's `println!` panics on a closed pipe. Output now ends quietly.

## [0.2.0-wave-3] — 2026-08-20

### Added

- **`aep-conformance`** — sixteen black-box suites a backend runs against itself to prove it
  implements the contract: identity, command execution, idempotency, optimistic concurrency, query,
  consistency, relations, history, immutability, audit, rejected-action audit, correlation, causation,
  provenance, events and type discovery. Reports name the *property* that failed, not the assertion,
  so a failure says what to fix.
- **Conformance levels** — `core`, `audited`, `full`. A backend states what it claims and the suite
  proves or refutes it, instead of a README asserting it.
- **`FaultyBackend`** — a wrapper that breaks exactly one property at a time. The crate's own tests
  assert that the suite responsible for each fault fails and the others still pass, because a suite
  that passes everything tells you nothing about whether it would catch anything.
- **`protocol conformance --level core|audited|full [--suite <name>] [--inject <fault>]`** — runs the
  suites, and can deliberately break a property to show which suite catches it.
- **`adp-domain`** — development types (`adp.specification/v1`, `adp.test-plan/v1`,
  `adp.acceptance-criteria/v1`, `adp.change/v1`) and commands (`adp.story.start/v1`,
  `adp.story.complete/v1`, `adp.test-plan.record/v1`, `adp.specification.satisfy/v1`). A
  specification declared satisfied by no evidence is refused — the exact claim the protocol exists to
  stop.
- **`aop-domain`** — operations types (`aop.incident/v1`, `aop.runbook/v1`, `aop.release/v1`) with
  their status ladders, and commands (`aop.incident.acknowledge|mitigate|resolve/v1`,
  `aop.release.promote|rollback/v1`). Promoting to production without naming an approval is refused
  at the command, which is a second defence beside the protocol's approval floor.
- **`docs/guide/`** — how to adopt the protocol, wire a harness to the engine, and implement and
  prove a backend.
- `Fault::caught_by()` names the suite responsible for each fault, and the crate's own tests assert
  that suite fails when the fault is injected. `DropAffected` fails eight suites, which is a finding
  about how load-bearing `affected` is rather than a flaw in the suites, and is recorded as such.

### Changed

- The in-memory backend now **refuses an update to an immutable type**. A review result records what
  someone concluded at a moment; editing it afterwards changes what the record says a person decided.
  Archiving stays available — keeping a record and editing it are different acts.

## [0.2.0-wave-2] — 2026-08-20

### Added

- **Identity.** Every addressable thing now has an opaque `EntityId`, a logical `EntityLocator`
  (`ep://acme/payments/design/passkeys-auth`), a versioned `EntityType` (`aep.design/v1`) and a
  monotonic `EntityRevision`. `AUTH-142` is a key in a locator, not identity — so two repositories can
  refer to the same design, and an approval can name the exact revision it approved.
- **`ActorRef`** — `human:alice`, `agent:planning-agent`, `service:release-controller`, `system`.
  Distinct from an evidence `Producer`: an actor bears responsibility, a producer made an observation.
  Commands carry both an actor and an executor, so "alice authorised it, agent-17 ran it" is
  answerable, and a trail that collapses them can answer neither question.
- **`aep-contract`** — the storage-independent interaction contract: `CommandService` and
  `QueryService`, command envelopes with the six identifiers that make a trail reconstructable,
  consistency tokens giving read-your-writes without sleeps, a typed failure taxonomy, and
  `TypeDescriptor` so a harness can ask what a design is instead of hard-coding it.
- **Commands** (`aep-domain::command`) — six generic (`CreateEntity`, `UpdateEntity`,
  `CreateRelation`, `RemoveRelation`, `ArchiveEntity`, `SupersedeEntity`) and three domain
  (`SubmitDesignReview`, `ApproveDesign`, `AcceptAdr`). A domain command can be validated where a
  generic patch cannot: `ApproveDesign{design@7, review}` checks that the review is about *that*
  revision.
- **Domain events** (`aep-domain::domain_event`) — a versioned event vocabulary with an open
  `Custom` variant, separate from the protocol's execution events. An event caused by a command
  names that command as its cause.
- **Audit records** (`aep-domain::audit`) — actor and executor, correlation and causation, decision
  records and change records with before/after revisions, and **rejected attempts**: a denied command
  changes nothing and still leaves a record, which is the half most systems lose.
- **`aep-backend-memory`** — a complete in-memory implementation of both contract surfaces, so the
  contract is exercised by something before anyone builds a durable backend. It passes the
  specification's nineteen-step reference scenario, including idempotent replay, stale-revision
  conflicts and the audit record a refusal leaves behind.
- **`aep-engine::trail`** — protocol decisions become audit records, and a command issued during an
  execution inherits its correlation, execution and task. A refusal by the protocol and a refusal by
  a backend now land in the same trail, queryable the same way.
- Evidence may be submitted as an entity reference, so the trail points at the stored evidence rather
  than at the engine's copy of it.
- `RelationKind::Delivers`, and `ArtifactKind::entity_type()` mapping the human-facing artifact
  vocabulary onto entity types.
- **CLI**: `protocol entity list|get|history|relations`, `protocol audit [--correlation|--entity|
  --rejected]` and `protocol describe <type>`, backed by an in-memory backend seeded from an artifact
  manifest through real commands — so seeding produces history and audit records like anything else.

### Changed

- **Nine new `ValidationCode`s** — `self_reference`, `empty_change`, `refusal_mutated_state`,
  `unreconstructable_change`, `unexplained_decision`, `redaction_inconsistent`,
  `event_payload_mismatch`, `incomplete_event_subject`, `missing_causation`. Previously these
  failures all reported `unknown_state`, so a caller could not tell "this audit record claims a
  refusal changed something" from "this workflow references a state that does not exist".
- Minimum supported Rust version is 1.85 (`Waker::noop`, which lets the contract define `async fn`
  traits without an executor dependency or a line of `unsafe`).
- A protocol may declare an **approval floor** — capabilities no profile may grant outright.
  `aep/1` declares `production.write` and `deployment.create:production`, and a profile that grants
  one fails to resolve.

## [0.2.0-wave-1] — 2026-08-20

### Added

- **The execution core.** `aep-engine` resolves a task against a document tree and answers what is
  owed, what may be done, which transitions are permitted and whether the task is complete:
  - `registry` — the documents in force, with the cross-document checks (unknown references, pinned
    version mismatches, undeclared capabilities and evidence kinds, evidence no verifier can
    establish);
  - `load` — reads a document tree, reporting every bad file with its path rather than the first;
  - `resolve` — task + registry → execution plan: `extends` chains merged, principles filtered by
    applicability, capabilities composed with the document responsible recorded for each entry,
    obligations collected, and the whole configuration checked for rules that could never fire;
  - `execution` — live state with derived facts (`evidence.first_seq.*`, `test.first_result`,
    `evidence.missing`) and a serialisable snapshot;
  - `evaluate`, `policy`, `explain` — what is owed, capability decisions naming the rule that
    decided, and the `✓ / ✗ / ?` completion checklist;
  - `engine` — the `ProtocolEngine` trait, deterministic transitions, an injected `Clock`.
- **The documents.** 42 of them: `aep/1` plus `adp/1` and `aop/1`; 21 principles across intent,
  construction, verification and governance; 4 workflows (development, incident, progressive release,
  forward-only migration); 5 profiles; 5 artifact lifecycles; artifact kind and relation definitions;
  8 templates.
- **`protocol` CLI** — `validate`, `resolve`, `inspect`, `evaluate`, `explain`, `schema`, with
  `--format text|yaml|json`.
- **Worked example** (`examples/development-passkeys/`) — a task, its artifact graph and a five-step
  evidence sequence that walks to completion, replayed by the integration tests.
- **Protocol approval floor.** A protocol may declare capabilities no profile can grant outright;
  `aep/1` declares `production.write` and `deployment.create:production`. A profile that grants one
  fails to resolve.
- **`Action::ProductionMutate`** — production changes that are not deployments now have an action, so
  a policy naming only deployments cannot let them through.
- **CI** — GitHub Actions mirroring `task check`, with schema drift as its own job.

### Fixed

- `evidence.missing` counted evidence required by conditional rules that did not apply, so a task
  could show every requirement met and still be unable to finish.
- The approval floor is now violated by any *overlap*: granting `deployment.create` for every
  environment no longer slips past a floor on `deployment.create:production`.
- A task may name the base protocol its profile refines (`aep/1` with a profile written against
  `adp/1`), which is the form the design documents use.

### Changed

- Evidence files spell the envelope's subject `about`, not `subject`, so it cannot silently consume a
  payload's own `subject` — a review's subject is the artifact reviewed.
- `protocol evaluate` exits `0` whenever it produced a report. A blocked execution is an answer, not
  a failure; `explain --action` still exits `1` when an action is refused.

## [0.1.0] — 2026-08-19

### Added

- **`aep-domain`** — the source-of-truth model: identifiers and versioned references, a three-valued
  predicate language, facts and ordered scales, capabilities with default-deny, actions, evidence with
  provenance, verifiers and counterexamples, the artifact graph with lifecycles and typed relations,
  review semantics with revision-bound approval, requirements over evidence/artifacts/reviews/
  approvals/conditions, principles with phase-timed obligations, workflows, tasks, protocols,
  profiles, execution plans and the audit event vocabulary.
- **`aep-schema`** — document reading that separates syntax from semantic failure, and JSON Schema
  generation for six document types and four interchange types.
- **`xtask schema [--check]`** — schemas are generated from the Rust types, and CI proves they match.
- Repository scaffolding: workspace, `Taskfile.yml` gate, Apache-2.0 licence, `AGENTS.md`.

[Unreleased]: https://github.com/beyond10x/aep/compare/0.38.1...HEAD
[0.38.1]: https://github.com/beyond10x/aep/compare/0.38.0...0.38.1
[0.38.0]: https://github.com/beyond10x/aep/compare/0.37.1...0.38.0
[0.37.1]: https://github.com/beyond10x/aep/compare/0.37.0...0.37.1
[0.37.0]: https://github.com/beyond10x/aep/compare/0.36.4...0.37.0
[0.36.4]: https://github.com/beyond10x/aep/compare/0.36.3...0.36.4
[0.36.3]: https://github.com/beyond10x/aep/compare/0.36.2...0.36.3
[0.36.2]: https://github.com/beyond10x/aep/compare/0.36.1...0.36.2
[0.36.1]: https://github.com/beyond10x/aep/compare/0.36.0...0.36.1
[0.36.0]: https://github.com/beyond10x/aep/compare/0.35.0...0.36.0
[0.35.0]: https://github.com/beyond10x/aep/compare/0.34.0...0.35.0
[0.34.0]: https://github.com/beyond10x/aep/compare/0.33.0...0.34.0
[0.33.0]: https://github.com/beyond10x/aep/compare/0.32.1...0.33.0
[0.32.1]: https://github.com/beyond10x/aep/compare/0.32.0...0.32.1
[0.32.0]: https://github.com/beyond10x/aep/compare/0.31.0...0.32.0
[0.31.0]: https://github.com/beyond10x/aep/compare/0.30.0...0.31.0
[0.30.0]: https://github.com/beyond10x/aep/compare/0.29.0...0.30.0
[0.29.0]: https://github.com/beyond10x/aep/compare/0.28.0...0.29.0
[0.28.0]: https://github.com/beyond10x/aep/compare/0.27.3...0.28.0
[0.27.3]: https://github.com/beyond10x/aep/compare/0.27.2...0.27.3
[0.27.2]: https://github.com/beyond10x/aep/compare/0.27.1...0.27.2
[0.27.1]: https://github.com/beyond10x/aep/compare/0.27.0...0.27.1
[0.27.0]: https://github.com/beyond10x/aep/compare/0.26.0...0.27.0
[0.26.0]: https://github.com/beyond10x/aep/compare/0.25.0...0.26.0
[0.25.0]: https://github.com/beyond10x/aep/compare/0.24.0...0.25.0
[0.24.0]: https://github.com/beyond10x/aep/compare/0.23.2...0.24.0
[0.23.2]: https://github.com/beyond10x/aep/compare/0.23.1...0.23.2
[0.23.1]: https://github.com/beyond10x/aep/compare/0.23.0...0.23.1
[0.23.0]: https://github.com/beyond10x/aep/compare/0.22.0...0.23.0
[0.22.0]: https://github.com/beyond10x/aep/compare/0.21.0...0.22.0
[0.21.0]: https://github.com/beyond10x/aep/compare/0.20.0...0.21.0
[0.20.0]: https://github.com/beyond10x/aep/compare/0.19.0...0.20.0
[0.19.0]: https://github.com/beyond10x/aep/compare/0.18.0...0.19.0
[0.18.0]: https://github.com/beyond10x/aep/compare/0.17.0...0.18.0
[0.17.0]: https://github.com/beyond10x/aep/compare/0.16.0...0.17.0
[0.16.0]: https://github.com/beyond10x/aep/compare/0.15.0...0.16.0
[0.15.0]: https://github.com/beyond10x/aep/compare/0.14.0...0.15.0
[0.14.0]: https://github.com/beyond10x/aep/compare/0.13.0...0.14.0
[0.13.0]: https://github.com/beyond10x/aep/compare/0.12.0...0.13.0
[0.12.0]: https://github.com/beyond10x/aep/compare/0.11.0-ground-truth-and-docs...0.12.0
[0.11.0-ground-truth-and-docs]: https://github.com/beyond10x/aep/compare/0.10.0-horizons-dogfood-lab...0.11.0-ground-truth-and-docs
[0.10.0-horizons-dogfood-lab]: https://github.com/beyond10x/aep/compare/0.9.0-harness-waves-2-3...0.10.0-horizons-dogfood-lab
[0.9.0-harness-waves-2-3]: https://github.com/beyond10x/aep/compare/0.8.0-harness-wave-1-trace-wave-1...0.9.0-harness-waves-2-3
[0.8.0-harness-wave-1-trace-wave-1]: https://github.com/beyond10x/aep/compare/0.7.1-infra-waves-1-4...0.8.0-harness-wave-1-trace-wave-1
[0.7.1-infra-waves-1-4]: https://github.com/beyond10x/aep/compare/0.7.0-ess-wave-7...0.7.1-infra-waves-1-4
[0.7.0-ess-wave-7]: https://github.com/beyond10x/aep/compare/0.6.1-ess-wave-6.5...0.7.0-ess-wave-7
[0.6.1-ess-wave-6.5]: https://github.com/beyond10x/aep/compare/0.6.0-ess-wave-6...0.6.1-ess-wave-6.5
[0.6.0-ess-wave-6]: https://github.com/beyond10x/aep/compare/0.5.0-ess-wave-5...0.6.0-ess-wave-6
[0.5.0-ess-wave-5]: https://github.com/beyond10x/aep/compare/0.4.0-ess-wave-4...0.5.0-ess-wave-5
[0.4.0-ess-wave-4]: https://github.com/beyond10x/aep/compare/0.3.3-ess-wave-3.5...0.4.0-ess-wave-4
[0.3.3-ess-wave-3.5]: https://github.com/beyond10x/aep/compare/0.3.2-ess-wave-3...0.3.3-ess-wave-3.5
[0.3.2-ess-wave-3]: https://github.com/beyond10x/aep/compare/0.3.1-ess-wave-2...0.3.2-ess-wave-3
[0.3.1-ess-wave-2]: https://github.com/beyond10x/aep/compare/0.3.0-ess-wave-1...0.3.1-ess-wave-2
[0.3.0-ess-wave-1]: https://github.com/beyond10x/aep/compare/0.2.1...0.3.0-ess-wave-1
[0.2.1]: https://github.com/beyond10x/aep/compare/0.2.0-wave-3...0.2.1
[0.2.0-wave-3]: https://github.com/beyond10x/aep/compare/0.2.0-wave-2...0.2.0-wave-3
[0.2.0-wave-2]: https://github.com/beyond10x/aep/compare/0.2.0-wave-1...0.2.0-wave-2
[0.2.0-wave-1]: https://github.com/beyond10x/aep/compare/0.1.0...0.2.0-wave-1
[0.1.0]: https://github.com/beyond10x/aep/releases/tag/0.1.0
