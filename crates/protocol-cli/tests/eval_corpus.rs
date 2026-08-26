//! Every eval case under `conformance/eval/` is replayed through the checker, in the gate.
//!
//! # What a case is
//!
//! A directory holding a `case.yaml`, a `trace-spec/1` document and a committed transcript. Nothing
//! registers a case anywhere: this file enumerates the corpus root, so a new directory **is** a new
//! case and adding one costs three files and no code.
//!
//! # The verdict is declared, and both directions are refused
//!
//! A case declares `verdict: held` or `verdict: violated` with the expectation ids it expects to
//! gap. A corpus of honest runs measures only whether the documents can be *satisfied*, and a bound
//! that has never been observed to fail is a bound nobody has evidence discriminates — so the
//! violation case is the control, and its gapping set is pinned **exactly**. Repairing a transcript
//! so that a declared violation stops gapping is as red as breaking a row that was passing.
//!
//! `unk` is refused everywhere, in both kinds of case. An undecidable row is how a check stops being
//! a check without anybody noticing, and a violation case that went green because the reader could
//! no longer read the field would certify the opposite of what it claims.
//!
//! # Why this test lives in `protocol-cli`
//!
//! A case ties a transcript to a **workflow state**, so checking one needs the trace checker and the
//! workflow parser at once. This crate has both, and its sibling `workflow_coverage.rs` reads the
//! same two trees for the same reason.
//!
//! The replay itself is `crates/trace-spec/tests/`' idiom, unchanged: read the committed document
//! with `trace_domain::raw::read_spec`, read the transcript with `read_any`, `check`, and assert on
//! the report — printing `report_to_text` on failure, because a verdict nobody can read is a verdict
//! somebody will delete.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use aep_domain::workflow::Workflow;
use trace_spec::check::check;
use trace_spec::reader::read_any;
use trace_spec::report::Verdict;

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Where the corpus lives.
const CORPUS: &str = "conformance/eval";

/// One case as written. Its own types, for the reason `workflow_coverage.rs` gives: this is not a
/// protocol document kind and publishing a schema for it would say otherwise.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    format: String,
    id: String,
    #[allow(dead_code)]
    title: String,
    workflow: String,
    states: Vec<String>,
    #[allow(dead_code)]
    arm: String,
    verdict: DeclaredVerdict,
    task: String,
    expectations: String,
    transcript: String,
    #[serde(default)]
    violated: Vec<Violation>,
    /// Advisory rows this case expects to gap, and why.
    ///
    /// An advisory row judges whether the *evidence* is worth anything, not whether the run did
    /// anything wrong — so a case whose only gap is advisory still `held`. Declared rather than
    /// ignored: an advisory gap nobody pinned is one that can appear or vanish unnoticed, and
    /// a declared one that stops gapping fails this test.
    #[serde(default)]
    advisory_gaps: Vec<Violation>,
}

/// What the case says the check must report.
#[derive(serde::Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
enum DeclaredVerdict {
    /// Exit 0, nothing contradicted, nothing undecidable.
    Held,
    /// Exactly the declared rows gap, nothing else, and nothing is undecidable.
    Violated,
}

/// One expectation a violation case expects to be contradicted, and why.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Violation {
    expectation: String,
    why: String,
}

/// A case, with the directory it was read from.
struct Loaded {
    directory: PathBuf,
    case: Case,
}

/// Every case in the corpus, in directory order.
fn corpus() -> Vec<Loaded> {
    let base = root().join(CORPUS);
    let mut directories: Vec<PathBuf> = std::fs::read_dir(&base)
        .unwrap_or_else(|error| panic!("{CORPUS} is a directory in this tree: {error}"))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    directories.sort();

    assert!(
        !directories.is_empty(),
        "{CORPUS} holds no case directory; this test would then check nothing"
    );

    directories
        .into_iter()
        .map(|directory| {
            let manifest = directory.join("case.yaml");
            let name = directory
                .file_name()
                .expect("a directory name")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&manifest).unwrap_or_else(|_| {
                panic!(
                    "{CORPUS}/{name} has no `case.yaml`. Every directory under the corpus root is a \
                     case; a directory that is not one has nowhere to hide."
                )
            });
            let case: Case = serde_yaml::from_str(&text).unwrap_or_else(|error| {
                panic!("{CORPUS}/{name}/case.yaml must parse as a case:\n{error}")
            });
            assert_eq!(
                case.format, "eval-case/1",
                "{CORPUS}/{name}/case.yaml declares a format this test does not read"
            );
            assert_eq!(
                case.id, name,
                "a case's id and its directory name must agree, or a failing row names something \
                 nobody can find"
            );
            Loaded { directory, case }
        })
        .collect()
}

/// Every workflow document in the tree, by the id it declares.
///
/// Keyed by declared id and never by filename — invariant 10, and the same reason
/// `workflow_coverage.rs` gives.
fn workflows() -> BTreeMap<String, Workflow> {
    let mut found = BTreeMap::new();
    let mut directories = vec![root().join("workflows")];

    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory).expect("the workflow tree is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("yaml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("a workflow document is readable");
            let workflow = aep_schema::parse::workflow(&text, None).unwrap_or_else(|error| {
                panic!("{} must parse as a workflow:\n{error}", path.display())
            });
            found.insert(workflow.id.to_string(), workflow);
        }
    }
    found
}

/// A case's file, resolved and held inside the corpus.
///
/// `development-tests-after-the-code` points its `expectations:` at its sibling's document rather
/// than carrying a copy, which is what lets two runs meet literally the same rows. That makes a
/// relative path with `..` legitimate — and makes it worth refusing one that climbs out of the
/// corpus altogether, because a case judged by a document outside this tree is a case whose subject
/// nobody can enumerate.
fn file(loaded: &Loaded, relative: &str, field: &str) -> PathBuf {
    let path = loaded.directory.join(relative);
    let canonical = path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "{}: `{field}: {relative}` names no file in this tree",
            loaded.case.id
        )
    });
    assert!(
        canonical.starts_with(root().join(CORPUS)),
        "{}: `{field}: {relative}` resolves outside `{CORPUS}`",
        loaded.case.id
    );
    canonical
}

#[test]
fn every_case_is_three_files_and_says_what_it_is_about() {
    // The corpus's own shape, refused per case by name. A case missing its transcript, or naming a
    // workflow that was deleted, is a case that will otherwise sit in the tree looking like
    // coverage.
    let workflows = workflows();
    let mut refusals: Vec<String> = Vec::new();

    for loaded in corpus() {
        let case = &loaded.case;
        file(&loaded, &case.expectations, "expectations");
        file(&loaded, &case.transcript, "transcript");

        if case.task.trim().is_empty() {
            refusals.push(format!(
                "  - {}: `task:` is empty; a case with no statement measures nothing",
                case.id
            ));
        }

        let Some(workflow) = workflows.get(&case.workflow) else {
            refusals.push(format!(
                "  - {}: `workflow: {}` — no document under `workflows/` declares that id",
                case.id, case.workflow
            ));
            continue;
        };

        if case.states.is_empty() {
            refusals.push(format!(
                "  - {}: names no state of `{}`",
                case.id, case.workflow
            ));
        }
        let declared: BTreeSet<&str> = workflow
            .states
            .keys()
            .map(aep_domain::ids::StateId::as_str)
            .collect();
        for state in &case.states {
            if !declared.contains(state.as_str()) {
                refusals.push(format!(
                    "  - {}: state `{state}` is not declared by `{}`, which has {declared:?}",
                    case.id, case.workflow
                ));
            }
        }
    }

    assert!(
        refusals.is_empty(),
        "{} case(s) in {CORPUS} that do not hold together:\n{}",
        refusals.len(),
        refusals.join("\n")
    );
}

#[test]
fn every_case_replays_to_the_verdict_it_declares() {
    // The story's acceptance, end to end and once per case: the committed transcript, the committed
    // document, this build's checker, and the verdict the case wrote down before any of them ran.
    for loaded in corpus() {
        let case = &loaded.case;
        let id = &case.id;

        let document_path = file(&loaded, &case.expectations, "expectations");
        let text = std::fs::read_to_string(&document_path).expect("the document is readable");
        let specification = trace_domain::raw::read_spec(&text)
            .unwrap_or_else(|errors| panic!("{id}: its expectations must validate:\n{errors}"));

        let bytes = std::fs::read(file(&loaded, &case.transcript, "transcript"))
            .expect("the transcript is readable");
        let ir = read_any(&bytes)
            .unwrap_or_else(|errors| panic!("{id}: its transcript must read:\n{errors}"));

        let report = check(&specification, &ir, &[]);
        let rendered = trace_spec::render::report_to_text(&report);

        // Refused in every case, both kinds. A row the reader can no longer decide is how a check
        // stops being a check quietly — and a violation case that went `unk` would report the
        // opposite of what it claims while still looking like a failure.
        assert_eq!(
            rows_with(&report, Verdict::Unknown),
            Vec::<&str>::new(),
            "{id}: every row must decide against its own transcript\n{rendered}"
        );

        let gapped = rows_with(&report, Verdict::Gap);
        let declared_advisory: Vec<&str> = case
            .advisory_gaps
            .iter()
            .map(|row| row.expectation.as_str())
            .collect();
        let blocking: Vec<&str> = gapped
            .iter()
            .copied()
            .filter(|id| !declared_advisory.contains(id))
            .collect();
        for expected in &declared_advisory {
            assert!(
                gapped.contains(expected),
                "{id}: declares an advisory gap for `{expected}` and it did not gap — a declared \
                 observation that stopped happening is a change worth seeing\n{rendered}"
            );
        }

        match case.verdict {
            DeclaredVerdict::Held => {
                assert!(
                    case.violated.is_empty(),
                    "{id}: declares `verdict: held` and lists violations; it cannot say both"
                );
                assert_eq!(
                    blocking,
                    Vec::<&str>::new(),
                    "{id}: declares `verdict: held` and contradicted these rows\n{rendered}"
                );
                assert_eq!(
                    report.exit_code(),
                    0,
                    "{id}: a held case exits 0\n{rendered}"
                );
            }
            DeclaredVerdict::Violated => {
                assert!(
                    !case.violated.is_empty(),
                    "{id}: declares `verdict: violated` and names no expectation. A violation \
                     nobody wrote down is indistinguishable from a broken fixture."
                );

                let declared: Vec<&str> = case
                    .violated
                    .iter()
                    .map(|violation| violation.expectation.as_str())
                    .collect();

                // Pinned in **both** directions, which is the whole point. A row that stopped
                // gapping is a fixture somebody repaired into agreement; a row that started is a
                // bound that has begun catching something else.
                assert_eq!(
                    blocking, declared,
                    "{id}: the rows this run contradicts are not the rows the case declares\n{rendered}"
                );

                for violation in &case.violated {
                    assert!(
                        specification
                            .expectations
                            .iter()
                            .any(|expectation| expectation.id == violation.expectation),
                        "{id}: declares a violation of `{}`, which its document does not contain",
                        violation.expectation
                    );
                    assert!(
                        violation.why.trim().len() > 40,
                        "{id}: `{}` is declared violated with no account of why. The case has to \
                         say which rule was broken and what breaking it costs, or the next reader \
                         learns only that a fixture is red on purpose.",
                        violation.expectation
                    );
                }
            }
        }
    }
}

#[test]
fn every_case_gates_on_something() {
    // A document of nothing but advisory rows is a case that cannot fail, and it reads in a report
    // exactly like a case that passed. Cheap to refuse, and the failure mode a corpus meets as it
    // grows.
    let mut refusals: Vec<String> = Vec::new();

    for loaded in corpus() {
        let document_path = file(&loaded, &loaded.case.expectations, "expectations");
        let text = std::fs::read_to_string(&document_path).expect("the document is readable");
        let specification = trace_domain::raw::read_spec(&text).expect("it validates");

        let gating = specification
            .expectations
            .iter()
            .filter(|expectation| expectation.severity == trace_domain::spec::Severity::Gate)
            .count();

        if gating == 0 {
            refusals.push(format!(
                "  - {}: every row is advisory, so the case cannot fail",
                loaded.case.id
            ));
        }
    }

    assert!(
        refusals.is_empty(),
        "{} case(s) that gate on nothing:\n{}",
        refusals.len(),
        refusals.join("\n")
    );
}

#[test]
fn the_two_development_cases_are_judged_by_one_document() {
    // The corpus's sharpest claim, asserted rather than left to a comment. The honest run and the
    // violating run meet the **same file**, so the only difference between their reports is what
    // the agent did. Two documents, each edited to suit its own transcript, would prove nothing
    // about either — and that drift is silent, because both would be green.
    let cases = corpus();
    let find = |id: &str| -> PathBuf {
        let loaded = cases
            .iter()
            .find(|loaded| loaded.case.id == id)
            .unwrap_or_else(|| panic!("{id} is a case in the corpus"));
        file(loaded, &loaded.case.expectations, "expectations")
    };

    assert_eq!(
        find("development-honest"),
        find("development-tests-after-the-code"),
        "the honest development case and its violation must resolve to the same document"
    );
}

/// The ids of every row with this verdict, in report order.
fn rows_with(report: &trace_spec::report::CheckReport, verdict: Verdict) -> Vec<&str> {
    report
        .expectations
        .iter()
        .filter(|row| row.verdict == verdict)
        .map(|row| row.id.as_str())
        .collect()
}
