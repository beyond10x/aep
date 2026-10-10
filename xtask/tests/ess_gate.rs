//! The gate on `ess/`, the ESS specification of the `aep plan reverse init` and
//! `aep plan store migrate git` surface and of the review-result findings rule
//! (`aep plan artifact new` and `validate`) and of how `aep plan artifact evidence --from
//! <report/2> --suite <suite>` admits a suite, and on the projections committed under
//! `generated/ess/`.
//!
//! * `cargo xtask ess --check` holds: the specification validates under its pinned release
//!   (`--strict-requires`), compiles, and every committed projection equals a fresh one. Two
//!   negative controls break a copy of the tree and require the check to fail naming the file.
//! * The pin in `ess/ess-inputs.yaml` is the release CI installs.
//! * The specification's open questions (`UNMAPPED:` markers) are exactly the recorded ones, and
//!   `ess verify conform synthesize` produces exactly the recorded scenario and refusal counts.
//!   Neither is zero yet: both ledgers make a change to them a reviewed edit of this file rather
//!   than something a green gate hides.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The release `ess/ess-inputs.yaml` pins and `.github/workflows/ci.yml` installs.
const PINNED: &str = "0.56.0";

/// The marker of an open question, in two halves so this file does not carry it whole.
const OPEN_QUESTION: &str = concat!("UNMAPPED", ":");

/// Every open question `ess/` records, as the text after the marker on its first line. Each names
/// what ess 0.56.0 refused and what would settle it; remove an entry with its marker.
const OPEN_QUESTIONS: &[&str] = &[
    "the rule's third clause, \"including a non-whitespace character\" (project.rs:71-73",
    "the file also gets `store: { git: {} }` (reverse.rs:1469-1470); ess 0.56.0",
];

/// What `ess verify conform synthesize` makes of `ess/` today: 16 scenarios and 61 refusals.
///
/// The scenarios are `aep.plan.ReverseInit/outcome/flag-unfit`, the seven outcomes of
/// `aep.review.RecordReview` other than `findings-required`, the `aep.review.ReviewResult`
/// invariant after each of its three recording outcomes, and the five outcomes of
/// `aep.evidence.RecordFromReport`.
///
/// Nearly all refusals are `ESS-SYNTH-001`. In `aep.plan` (47) both commands read two row sets
/// ("a scenario arranges the rows of one selector per command in this cut"), and neither entity
/// has a creating command in aep (a repository is a directory, an `aep.project/1` file was written
/// by an earlier build). In `aep.review` (14): `findings-required` needs a project file that sets
/// `findings_required_since`, which no declared command writes; every `ValidateFindings` outcome
/// and the invariant after each one need a review whose `created_before_opt_in` and
/// `store_requires_findings` are arranged, and `RecordReview` leaves both to `validate`; and
/// `aep.plan.UtcDate` is published by no view (`ESS-SYNTH-013`). A conformance run needs seeded
/// rows and a target that drives the `aep` binary; neither exists yet.
const SYNTHESIZED: (u64, u64) = (16, 61);

/// The tree under test, read at run time.
fn repo_root() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is unset: run this test through cargo");
    Path::new(&manifest)
        .parent()
        .expect("xtask lives one level below the repository root")
        .to_path_buf()
}

/// A fresh directory under `CARGO_TARGET_TMPDIR` for one case, distinct per tree under test.
fn scratch(case: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    repo_root().hash(&mut hasher);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("aep-ess-gate")
        .join(format!("{:016x}", hasher.finish()))
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("remove the previous scratch copy");
    }
    fs::create_dir_all(&dir).expect("create scratch");
    dir
}

/// `cargo xtask ess --check --root <root>`.
fn check(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["ess", "--check", "--root"])
        .arg(root)
        .output()
        .expect("the xtask binary runs")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap_or_else(|error| panic!("create {}: {error}", to.display()));
    for entry in
        fs::read_dir(from).unwrap_or_else(|error| panic!("read {}: {error}", from.display()))
    {
        let path = entry.expect("directory entry").path();
        let target = to.join(path.file_name().expect("entry has a name"));
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            fs::copy(&path, &target)
                .unwrap_or_else(|error| panic!("copy {}: {error}", path.display()));
        }
    }
}

/// A copy of `ess/` and `generated/ess/` under a scratch root, on which the check passes.
fn tree_copy(case: &str) -> PathBuf {
    let root = scratch(case);
    copy_dir(&repo_root().join("ess"), &root.join("ess"));
    copy_dir(
        &repo_root().join("generated/ess"),
        &root.join("generated/ess"),
    );
    root
}

#[test]
fn committed_projections_equal_the_specification() {
    let output = check(&repo_root());
    assert!(output.status.success(), "{}", text(&output));
}

/// Negative control: a specification edit that is not regenerated fails the check, naming a
/// projection it changes.
#[test]
fn a_specification_edit_left_unprojected_fails_the_check() {
    let root = tree_copy("unprojected-edit");
    let domain = root.join("ess/domains/plan.yaml");
    let original = fs::read_to_string(&domain).expect("domain reads");
    let edited = original.replacen(
        "  - name: aep.plan.ProjectRoot\n",
        "  - name: aep.plan.Probe\n    kind: newtype\n    of: String\n  - name: aep.plan.ProjectRoot\n",
        1,
    );
    assert_ne!(original, edited, "the probe edit applies");
    fs::write(&domain, edited).expect("domain written");

    let output = check(&root);
    let printed = text(&output);
    assert!(!output.status.success(), "{printed}");
    assert!(
        printed.contains("aep.plan.Probe.schema.json") && printed.contains("generated/ess/schema"),
        "the check names the projection the edit changes:\n{printed}"
    );
}

/// A copy of the tree with one byte appended to `relative`, and what the check prints on it.
fn check_after_hand_edit(case: &str, relative: &str) -> (bool, String) {
    let root = tree_copy(case);
    let file = root.join(relative);
    let mut bytes = fs::read(&file).expect("projection reads");
    bytes.extend_from_slice(b"\n");
    fs::write(&file, bytes).expect("projection written");
    let output = check(&root);
    (output.status.success(), text(&output))
}

/// Negative control: a hand edit of the schema projection, which `ess --check` compares, fails
/// the check naming the file.
#[test]
fn a_hand_edited_schema_projection_fails_the_check() {
    let (passed, printed) = check_after_hand_edit(
        "hand-edit-schema",
        "generated/ess/schema/schema/types/aep.plan.ScopeSource.schema.json",
    );
    assert!(!passed, "{printed}");
    assert!(
        printed.contains("aep.plan.ScopeSource.schema.json"),
        "the check names the edited file:\n{printed}"
    );
}

/// Negative control: a hand edit of the Rust types projection, which xtask compares because
/// `ess generate types` has no `--check`, fails the check naming the file.
#[test]
fn a_hand_edited_types_projection_fails_the_check() {
    let (passed, printed) = check_after_hand_edit("hand-edit-types", "generated/ess/rust/types.rs");
    assert!(!passed, "{printed}");
    assert!(
        printed.contains("generated/ess/rust: types.rs: differs"),
        "the check names the edited file:\n{printed}"
    );
}

#[test]
fn the_pin_is_the_release_ci_installs() {
    let inputs =
        fs::read_to_string(repo_root().join("ess/ess-inputs.yaml")).expect("ess-inputs reads");
    assert!(
        inputs
            .lines()
            .any(|line| line.trim() == format!("requires: ess {PINNED}")),
        "ess/ess-inputs.yaml pins `requires: ess {PINNED}`:\n{inputs}"
    );
    let ci = fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).expect("ci reads");
    let installed: Vec<&str> = ci
        .lines()
        .filter_map(|line| line.trim().strip_prefix("ESS_VERSION:"))
        .map(|value| value.trim().trim_matches('"'))
        .collect();
    assert_eq!(
        installed,
        [PINNED],
        "ESS_VERSION in .github/workflows/ci.yml"
    );
}

#[test]
fn open_questions_are_the_recorded_ones() {
    let mut found = Vec::new();
    let mut files = Vec::new();
    collect(&repo_root().join("ess"), &mut files);
    files.sort();
    for file in files {
        let content = fs::read_to_string(&file).expect("specification file reads");
        for line in content.lines() {
            if let Some((_, after)) = line.split_once(OPEN_QUESTION) {
                found.push(after.trim().to_owned());
            }
        }
    }
    assert_eq!(
        found, OPEN_QUESTIONS,
        "ess/ declares other open questions than the ones recorded here"
    );
}

#[test]
fn synthesis_answers_the_recorded_counts() {
    let suite = scratch("synthesis").join("suite.json");
    let output = Command::new("ess")
        .args(["verify", "conform", "synthesize", "--path", "ess", "--out"])
        .arg(&suite)
        .current_dir(repo_root())
        .output()
        .expect("`ess` is on PATH");
    let printed = text(&output);
    assert!(output.status.success(), "{printed}");
    let summary = printed
        .lines()
        .rev()
        .find(|line| line.contains("refusal(s)"))
        .unwrap_or_else(|| panic!("no summary line:\n{printed}"));
    let counted = |label: &str| -> u64 {
        let before = &summary[..summary
            .find(label)
            .unwrap_or_else(|| panic!("`{label}` in {summary}"))];
        before
            .split(|c: char| c.is_whitespace() || c == ',')
            .rfind(|token| !token.is_empty())
            .and_then(|token| token.parse().ok())
            .unwrap_or_else(|| panic!("a count before `{label}` in {summary}"))
    };
    assert_eq!(
        (counted("scenario(s)"), counted("refusal(s)")),
        SYNTHESIZED,
        "synthesis changed; record the new counts with the reason:\n{printed}"
    );
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("directory reads") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect(&path, files);
        } else {
            files.push(path);
        }
    }
}
