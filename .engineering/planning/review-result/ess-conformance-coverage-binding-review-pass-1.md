---
format: aep.planning-md/2
id: review-result:ess-conformance-coverage-binding-review-pass-1
kind: review-result
status: active
title: ESS complete-selection binding review
relations:
- reviews: story:admit-ess-conformance-coverage
revision: 1
---
unit: suite/5 proposed binding review 1; exact six snapshots in manifest.json
verdict: NEEDS-CHANGE
cases: not executed; document review only, with no implementation-test result
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: none
needs-coordinator: bind the AEP observable vocabulary and correct the ESS story's strict-mode wording
git --no-pager diff --stat
```text
 .engineering/planning/journal.jsonl                |  8 ++++
 .../planning/story/review-conformance-coverage.md  | 47 +++++++++++++++++-----
 2 files changed, 45 insertions(+), 10 deletions(-)
```

This tracked diff existed on arrival and was unchanged at final inspection. It is coordinator-owned planning preparation, not this review's work. The untracked transport proposal also existed on arrival. This assignment expressly permits only read-only inspection plus an immutable report in the assigned scratch directory, so no source/test/store change was made and the implementation-attack sequence does not apply. No suite, executable counterexample, formatter, compiler, browser or compatibility harness ran. This is separate from closed wave8 and is not a third implementation attack.

1. Exact reviewed subject

Manifest SHA256: `577eaca7a6e70f091a97a706f135ce73d4e9dfd1c389b6f9e1767ca262f969d9`.
All six snapshot hashes were checked against that manifest. Each full document and both complete draft stories, including their acceptance sections, were read. The accepted ESS coverage snapshot also exactly matches its Git object at the stated ESS baseline; its historical source citations were distinguished from present production.

| Owner | Original source | Bytes | SHA256 |
|---|---|---:|---|
| ess | home-path:sha256:d610381eae78b88c2af2b0b4fc8270a031d804bff428556ebea28f900ba4c4d4 | 73049 | `3443846603fc474a32217310f8e0d1c1885b2117ab9f9c472fde990034d3b5f8` |
| ess | home-path:sha256:268398418f457389d9cb820036107d2c5edf1de22a92c6ce6eb5af67c0cc6929 | 13288 | `6b0e55e00f7d4d35979c3184949d993d15bd3016b510c4eddf05f6394d0073bf` |
| ess | home-path:sha256:2e9f66e127badfb2cbde89b0f6ccfa68d9941de86e56b04b411028af6e138b72 | 8098 | `c17f34d79ef3a14cc25713e89017c4c6e79a69b9396cee8946e72fce55f23cc2` |
| aep | home-path:sha256:8656238c8f4ea05710738a84edb0b47003b6a7af797f3dbc004568d40bf2a0a0 | 15385 | `e9c0c84f249b5b6a32b69e53ecc16f1ee4f317402bc1e14be934f0de2cce163d` |
| aep | home-path:sha256:8dc0c4f9624ec19685e372cf7c2676ee790b751435aad15a18f42a403da82610 | 10288 | `e43116505aa1073f9a46d68694654a18b727182ad97defc1b7824dc586d4e63d` |
| atlas | home-path:sha256:7df9269199d348858672ca86bb21e078e238d752856987bca7b5d8e92c024e1e | 7123 | `d4f5ee85817c1b6518c2a25ad636d95fca2ce149fb02b2bd53e3413506e3e16c` |

Snapshots are below this report's `ess/`, `aep/` and `atlas/` directories, at the paths specified verbatim in manifest.json. Baseline heads are ESS `be0eefd7ec125d46bb3b664c4b95b8d638a2b1fe`, AEP `62ef3a73112143453319215b9aa31a6c9626ed5f`, and Atlas `34fa907ff3ffcbdbb6bbf37d4dd674c05904bb20`. Findings refer to the frozen snapshot line numbers and repo-relative paths under the owner named below, not to later coordinator edits.

2. Concrete findings

| Owner / file:line | Category | Severity | Verdict | Origin | Finding |
|---|---|---|---|---|---|
| AEP / docs/design/ess-conformance-coverage-evidence.md:146 | contract-drift | blocker | NEEDS-CHANGE | introduced | The new coverage fact namespace lacks an exhaustive path, type and presence contract, leaving raw and admitted source observables to incompatible implementation choices. |
| ESS / .engineering/planning/story/review-conformance-coverage.md:84 | acceptance | warning | NEEDS-CHANGE | pre-existing | The ESS story requires incomplete-coverage acceptance in strict mode although the accepted policy makes allow-incomplete diagnostic and rejects combining it with strict mode. |

F1 — Bind the observable vocabulary before implementing its consumers.

What was inspected: AEP proposal lines146–150 specify a namespace and general scalar families, while saying Boolean facts “may” describe three properties. They do not select the complete paths, required/optional presence, raw-source projection or distinction between known inventory and complete coverage. Lines203–205 simultaneously select a public principle/protocol/profile and an observable namespace. No section or table elsewhere in the six snapshots completes that projection contract.

Concrete ambiguity: for an admitted nonempty all-pass report with complete_inventory and one in_scope refusal, `coverage_known = true` and `coverage_complete = false` describe different facts; a consumer cannot infer which path exists or what “complete declared coverage” means. Likewise `coverage.knowledge` versus `knowledge`, and absence of all semantic facts versus a projected false/default fact for raw unadmitted input, produce different predicate and inspection behavior. The prohibition on a global qualified fact does not settle those choices.

What reaches it: `aep-domain/src/evidence.rs:1905` currently projects count-stage reading facts only after admission; `ess_conformance_v2.rs:449` enumerates the current exact paths and types, including counts as Text, suite identity and Boolean facts. That precedent is a concrete public API, but the proposal explicitly freezes it and does not adopt its suffix table for the new kind. New `Evidence::facts`, FactStore consumers, inspection and the protocol's `ess_conformance_coverage_v1.**` observables need the new contract. This is an implementation-critical public data choice, not a request to select private Rust struct names.

Precise resolution: add one exhaustive new-namespace fact table with each exact path, FactValue type, derivation, presence condition and enum vocabulary. Explicitly require raw/unadmitted sources to emit no semantic coverage/count/status facts, or select another safe representation explicitly; specify behavior after failed admission/source replacement. Distinguish inventory knowledge from complete coverage (complete_inventory and no in_scope refusal) and nonempty selection. Keep task-dependent qualification solely in the shared per-record decision, with no global qualified alias. Name table-based projection assertions in the AEP verification section. Existing count facts remain byte/meaning unchanged.

Origin: introduced in this new proposed AEP document. A read-only Git object check confirms the document is absent from baseline62ef3a7. The issue is the new binding's unresolved vocabulary; no implementation defect or executed failing case is claimed. “blocker” applies to accepting this public fact contract for implementation, not to any closed count-stage source.

F2 — Correct the retained story's strict-mode acceptance wording.

What was inspected: ESS story line84 explicitly requires “incomplete-coverage acceptance in strict mode”; line47 also says strict execution requires explicit acceptance of incomplete coverage. Accepted coverage contract lines245–247 and P7/P10 instead specify that strict succeeds only for passed conformance, `--allow-incomplete` chooses diagnostic mode, and explicit strict plus allow-incomplete is invalid. Transport lines118–120 preserve those accepted meanings.

What reaches it: the story's Required classification checks are the implementor's test acceptance instructions. A fixture executing an incomplete suite/5 with report/2, strict and allow-incomplete cannot simultaneously meet the story's phrasing and the binding's mandatory conflict refusal. Current CLI clap declarations at main.rs:482 already make the flags conflict, and line2499 selects conformance status for strict mode.

Precise resolution: replace both retained sentences with an explicit requirement that incomplete/unknown/empty coverage never succeeds in strict mode; diagnostic execution with allow-incomplete preserves truthful classification; combining strict and allow-incomplete refuses before execution. Keep default movement excluded. This is a coordinator-owned story update through the planning CLI, not a source change by this reviewer.

Origin: pre-existing. The same two contradictory phrases are present in the story's Git object at ESSbe0eefd, before the proposed transport/scoping preparation. The accepted design already resolves the behavior, so this is a warning to align the story, not a new unresolved producer policy.

3. Reviewed decisions with no further concrete contradiction found

- Parent closure: transport42–62 and AEP72–78 require complete nearest-parent-first suite/5 lineage ending in all, reject missing/reordered/repeated/surplus parents, compare full surviving bodies/dependencies and provenance/scope/origins/knowledge, and preserve source maps, previous outside records and refusal occurrences. Changed parent layout changes its exact reference; filtering cannot promote knowledge. The flat chain has no invented semantic length cap.
- Source identity/ownership: checked relative identity precedes duplicate compilation; exact UTF-8 contents and root-relative names are retained; repeated identity refuses even for identical contents; separate identical files remain separate identities. The new CLI path preserves direct-directory discovery and refuses symlink traversal. Final accepted source ownership follows merging, while rejected known IDs and every refusal occurrence remain. This matches authored.rs:1439–1452's current deterministic ownership point without silently reinterpreting legacy Source.origin.
- Refusal semantics: accepted coverage lines105–140 and M37–M49/M59–M62 explicitly permit selected/outside/refused identity coexistence, retain final source owners and missing-check versus candidate effects, preserve identical occurrence multiplicity and conservative scope, and keep filtering from erasing in-scope gaps. Counts distinguish selected execution results from refusal occurrences.
- Browser: transport124–161 selects a closed paired replay/1 constructor from actual EssIr and admitted input, checks model/provenance/exact selected reference and lineage before state creation, and requires exact integer admission and actual generic-browser controls. web.rs:75–185 confirms that today's projection drops literal assignment values and reduces view behavior. The proposal expressly preserves those limits; F15's separate literal/sets+move/order/parameter/unknown-marker acceptance remains open. No fidelity fix or browser execution is established here.
- Impact: current impact.rs:97 declares ess-impact/3 and EssImpact at700–717 persists provenance/invalidation without exact selection fields. The transport's explicit current-baseline correction and in-memory selection context avoid an unversioned persisted extension; incomplete v5 coverage and missing input/lineage refuse narrowing, while the legacy DTO path cannot discard suite/5 coverage. Stale impact/2 lines in the historical design are expressly identified for correction, not treated as current truth.
- AEP qualification: independent task subject, specifically named current ESS model/digest, exact suite/full Selection/selected IDs, independent conformance-runner producer, exact observation/current time/freshness, nonempty complete inventory and all-pass terminal outcomes are all required in one shared per-record decision. Context-free matches is false, at_least0 cannot bypass it, and evidence.missing counts only Qualified records. Missing context and contradiction remain distinct; record facts cannot combine halves of a qualification.
- AEP mutation/replay: separate optional count/coverage ports, additive APIs and preservation of the other installed port are explicit. Raw source transport has no trusted reading flag, and candidate submission, checked direct recording and every snapshot restore re-admit before mutation. Current engine.rs:279,425 and execution.rs:222,313,701 identify those actual owners. CLI app evaluation/inspection and drive start/resume/consultation/read_record are explicitly reserved; actual aep-driver run.rs:763,1000 delegates to configured Engine restore/submit without needing a new adapter dependency or minting route.
- CLI/policy/rollout: raw suite1–4 stays count-only, raw unfiltered suite5 can be wrapped without inventing received carrier bytes, raw explicit suite5 lacks required parents and refuses, and suite-input is a distinct mutually exclusive source. Report1/detailed-run refusal and pre-store narrower-calendar refusal remain explicit. New opt-in policy leaves extend standard/adp1 without inheriting count-only conformance, while reader source publication precedes frozen actual producer correspondence and ESS writer publication. No default, installation, release or external adopter readiness follows from these documents.

These are source/document checks, not proof that a future implementation enforces the decisions. In particular, declared inventory honesty and publisher authentication are not established by matching hashes; the proposal explicitly assigns expected-suite choice to caller authority.

4. Scope and closure

ESS and AEP current repository instructions and the installed aep-drive0.8.0 adversary charter were read. No tests or suites ran because these additions have no implementation yet and the assignment forbids such execution. No external integration was used. Reads of absent guessed paths were corrected through rg; they are not findings.

Only this report and its matching structured findings file were authored, both below the assigned binding-review-1 directory. No source, existing test, planning document/journal, Git index/ref/object, foreign repository file, shared cache, worktree registry or lifecycle was written by this review. No cleanup, publication or approval action occurred. The six input snapshots remain unchanged. This report is immutable on return and all scratch writes are relinquished.

```findings
- file: docs/design/ess-conformance-coverage-evidence.md
  line: 146
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The new coverage fact namespace lacks an exhaustive path, type and presence contract, leaving raw and admitted source observables to incompatible implementation choices.
- file: .engineering/planning/story/review-conformance-coverage.md
  line: 84
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: The ESS story requires incomplete-coverage acceptance in strict mode although the accepted policy makes allow-incomplete diagnostic and rejects combining it with strict mode.
```
