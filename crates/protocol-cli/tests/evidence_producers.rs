//! The three producers `story:evidence-producers-for-the-driven-map` added, through the binary.
//!
//! `drivers/development/default.yaml` declares four evidence kinds it could not produce before this
//! story, and arm c of the three-arm pilot was refused at launch for exactly that
//! (`docs/reviews/2026-08-23-three-arm-pilot-1.md`). Three of the four are produced by a verb that
//! writes its own record — `protocol validate --evidence`, `protocol property evidence` and
//! `protocol specification evidence` — and the fourth is minted from an exit status by a contract
//! runner the map names.
//!
//! # Why these run through the binary rather than beside the code
//!
//! Each module tests its own reading rules directly, which is where a rule belongs. What only a
//! test through the binary can show is the property the map depends on: **the document the verb
//! writes is a document the driver reads back**, with the kind the map declares and a producer that
//! is a verifier and not the agent that wanted the claim. The two halves run in different processes
//! and the only thing joining them is a file, which is the same reason
//! `metaharness_contract_result.rs` exists.
//!
//! One rule is asserted for all three together, at the end: none of them may write a record a
//! person is recorded as having produced. That is invariant 7 at the layer a `record:` path opens —
//! nothing below the driver would stop a step handing the engine an approval it read out of a file.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use aep_domain::evidence::{Evidence, EvidenceKind, Producer};
use aep_domain::verification::Verifier;

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs `protocol` with `args` from the repository root.
fn protocol(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("the protocol binary runs")
}

/// A scratch directory of this test's own, emptied first so a rerun is a fresh run.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("protocol-producers-{name}"));
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// Writes a fixture file, creating the directories above it.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the temporary tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

/// A path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// The one record a producer wrote, read the way the driver's `read_record` reads it.
///
/// Through `aep_schema::parse::evidence_list`, which is the same reader `protocol evaluate
/// --evidence` and the driver both use — so a document this function accepts is one a driven step
/// can submit, and the assertion is about the seam rather than about a struct.
fn only_record(path: &Path) -> aep_schema::parse::EvidenceInput {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let mut records = aep_schema::parse::evidence_list(&text, Some(printable(path)))
        .unwrap_or_else(|error| panic!("{} is not an evidence document: {error}", path.display()));
    assert_eq!(
        records.len(),
        1,
        "a step establishes one thing, and the driver refuses a record file holding several: {text}"
    );
    records.remove(0)
}

/// The verifier a record's producer names, or a panic saying what it named instead.
///
/// A producer that is not a verifier fails here rather than in an assertion further down, because
/// `independent: true` — which `provenance-tracking`, `property-based-testing` and `test-driven` all
/// ask for — is exactly the claim this stamp is read as.
fn verifier_of(record: &aep_schema::parse::EvidenceInput) -> &Verifier {
    match &record.producer {
        Producer::Verifier { verifier } => verifier,
        other => panic!("a driven step's record must be a verifier's, and this is {other:?}"),
    }
}

#[test]
fn protocol_validate_writes_a_verification_record_the_driver_can_submit() {
    let directory = scratch("validate");
    let out = directory.join("verification.yaml");

    let output = protocol(&[
        "validate",
        "--root",
        printable(&root()),
        "--evidence",
        printable(&out),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "this repository's own tree validates: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let record = only_record(&out);
    assert_eq!(
        record.evidence.kind(),
        EvidenceKind::Verification,
        "the kind `drivers/development/default.yaml` declares for this step"
    );
    assert_eq!(
        verifier_of(&record),
        &Verifier::ExternalTool("protocol".parse().expect("a tool reference")),
        "the verifier the map names, so the record and the declaration agree"
    );
    let Evidence::Verification(payload) = &record.evidence else {
        panic!("the payload is a verification record");
    };
    assert_eq!(
        payload.claim.as_str(),
        "document-tree-valid",
        "a claim of its own: `verification.invariant.passed` is read by three shipped documents \
         that mean something else by it, and this walk does not establish theirs"
    );
    assert!(
        payload.status.is_pass(),
        "the tree validated, so the record says so: {payload:?}"
    );
}

#[test]
fn a_tree_that_does_not_validate_still_produces_a_record_and_it_says_failed() {
    // The half that matters. A validator that wrote `passed` whatever it found would pass every
    // assertion in the test above, and would discharge `provenance-tracking` on a broken tree.
    let directory = scratch("validate-red");
    write(
        &directory.join("protocols/broken.yaml"),
        "id: broken\nversion: nine\n",
    );
    let out = directory.join("verification.yaml");

    let output = protocol(&[
        "validate",
        "--root",
        printable(&directory),
        "--evidence",
        printable(&out),
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a tree with a problem exits non-zero, as it always did"
    );

    let record = only_record(&out);
    let Evidence::Verification(payload) = &record.evidence else {
        panic!("the payload is a verification record");
    };
    assert!(
        !payload.status.is_pass(),
        "the tree did not validate, and the record is what says so — the step declares `record:`, \
         so the driver never reads the exit status: {payload:?}"
    );
    assert_eq!(
        payload.counterexamples.len(),
        1,
        "the one problem is named, in the validator's own words: {payload:?}"
    );
    assert!(
        payload.counterexamples[0]
            .note
            .as_deref()
            .is_some_and(|note| note.contains("broken.yaml")),
        "and it says which document: {payload:?}"
    );
}

#[test]
fn protocol_property_evidence_writes_a_property_record_with_the_case_count_it_measured() {
    let directory = scratch("property");
    let out = directory.join("property.yaml");

    let output = protocol(&["property", "evidence", "--out", printable(&out)]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the verdict is in the record, so the verb exits 0 either way: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let record = only_record(&out);
    assert_eq!(record.evidence.kind(), EvidenceKind::PropertyTestResult);
    assert_eq!(
        verifier_of(&record),
        &Verifier::PropertyTester,
        "`principles/verification/property-based-testing.yaml` pins `property-tester` and marks \
         the evidence independent, so a record from anything else does not satisfy it"
    );
    let Evidence::PropertyTestResult(payload) = &record.evidence else {
        panic!("the payload is a property test result");
    };
    assert_eq!(payload.property.as_str(), "kleene-algebra");
    assert_eq!(
        payload.cases, 27,
        "every assignment of three variables over three truth values — the count is measured and \
         not minted, which is why this kind is outside `EvidenceMapping::MINTABLE`"
    );
    assert!(
        !payload.is_reproducible(),
        "an exhaustive run states no seed rather than inventing one"
    );
    assert!(payload.status.is_pass(), "{payload:?}");
}

/// The store a specification producer is run against: one specification, in force, **of this
/// task's work**, with requirements that reach each of the three verdicts.
///
/// The `specifies` edge and the task document beside it are not decoration. Since
/// `story:task-scoped-artifact-requirements`' follow-up the verb selects by
/// `spec-driven.before_implementation`'s own rule — an approved specification whose edge lands on
/// the work the task declares — so a fixture without them is a store the verb correctly refuses,
/// and the record these tests are about is never written.
fn specification_store(directory: &Path) {
    write(
        &directory.join("specification/passkeys.md"),
        "---\nformat: aep.planning-md/1\nid: specification:passkeys\nkind: specification\n\
         status: approved\ntitle: Passkey sign-in\nsummary: What signing in with a passkey must \
         do.\nrelations:\n- specifies: story:passkeys\n---\n# Specification\n\n\
         ## Acceptance\n\n\
         - The unit suite is green: `tests.unit.failed == 0`\n\
         - Static analysis is clean: `static_analysis.errors == 0`\n\
         - The change reads well to a person\n",
    );
    write(&directory.join("task.yaml"), TASK);
}

/// The task the store's specification is about, named with `--task` on every invocation below.
///
/// Named rather than discovered: these tests run from the repository root, so discovery would find
/// *this* repository's task and bind the selection to work that has nothing to do with the fixture.
const TASK: &str = "id: PASSKEYS-1\n\
     kind: feature\n\
     objective: passkey-sign-in\n\
     protocol: adp/1\n\
     profile: development.fast\n\
     derived_from:\n  - story:passkeys\n";

#[test]
fn protocol_specification_evidence_writes_the_requirement_by_requirement_verdict() {
    let directory = scratch("specification");
    specification_store(&directory);
    let out = directory.join("specification.yaml");
    let task = directory.join("task.yaml");

    let output = protocol(&[
        "specification",
        "evidence",
        "--store",
        printable(&directory),
        "--task",
        printable(&task),
        "--out",
        printable(&out),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "an unsatisfied specification is the case the record exists for: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let record = only_record(&out);
    assert_eq!(record.evidence.kind(), EvidenceKind::Specification);
    assert_eq!(
        verifier_of(&record),
        &Verifier::ExternalTool("protocol".parse().expect("a tool reference")),
        "the verifier the map names for this step"
    );
    let Evidence::Specification(payload) = &record.evidence else {
        panic!("the payload is a specification record");
    };
    assert_eq!(payload.requirements_total, Some(3));
    assert!(
        !payload.satisfied,
        "no run has observed anything, and `Unknown` is not satisfied (invariant 5): {payload:?}"
    );
    assert_eq!(
        payload.unsatisfied.len(),
        3,
        "the record names what is not met, one line each — which is the whole reason this kind \
         cannot be minted from an exit status: {payload:?}"
    );
    assert!(
        payload.unsatisfied[2].contains("states no predicate"),
        "a requirement nothing could decide is unmet and says so, rather than being skipped: \
         {payload:?}"
    );
}

#[test]
fn a_store_holding_two_specifications_of_this_tasks_work_is_refused_rather_than_guessed_at() {
    // D5's `Unknown` at the layer above the driver: the verb writes nothing, the step submits
    // nothing, and the run stops at the guard rather than moving on a record about the wrong
    // document.
    //
    // Both specifications carry the edge, so the ambiguity is a real one *inside* this task's
    // work — which is the only ambiguity left since the selection was bound. Another story's
    // approved specification is not a candidate at all any more, and
    // `specification_task_binding.rs` is where that is held.
    let directory = scratch("specification-ambiguous");
    specification_store(&directory);
    write(
        &directory.join("specification/sessions.md"),
        "---\nformat: aep.planning-md/1\nid: specification:sessions\nkind: specification\n\
         status: approved\ntitle: Sessions\nsummary: What a session must do.\n\
         relations:\n- specifies: story:passkeys\n---\n\
         # Specification\n\n## Acceptance\n\n- Sessions expire: `tests.unit.failed == 0`\n",
    );
    let out = directory.join("specification.yaml");
    let task = directory.join("task.yaml");

    let output = protocol(&[
        "specification",
        "evidence",
        "--store",
        printable(&directory),
        "--task",
        printable(&task),
        "--out",
        printable(&out),
    ]);
    let said = String::from_utf8_lossy(&output.stderr).into_owned();

    assert_eq!(output.status.code(), Some(1), "{said}");
    for named in [
        "specification:passkeys",
        "specification:sessions",
        "--artifact",
    ] {
        assert!(
            said.contains(named),
            "the refusal names every candidate and the way to say which; `{named}` is missing:\n\
             {said}"
        );
    }
    assert!(
        !out.exists(),
        "a refusal writes no record: a step that submitted one would be submitting a verdict about \
         a document nobody said the run was about"
    );

    // And the way through, which is what makes this a refusal rather than a wall. It is a way
    // through the ambiguity and not through the binding: `--artifact` names one of the two
    // documents that already specify this task's work.
    let named = protocol(&[
        "specification",
        "evidence",
        "--store",
        printable(&directory),
        "--task",
        printable(&task),
        "--artifact",
        "specification:sessions",
        "--out",
        printable(&out),
    ]);
    assert_eq!(
        named.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&named.stderr)
    );
    let Evidence::Specification(payload) = &only_record(&out).evidence else {
        panic!("the payload is a specification record");
    };
    assert_eq!(payload.requirements_total, Some(1), "{payload:?}");
}

#[test]
fn no_producer_writes_a_record_a_person_is_recorded_as_having_produced() {
    // Invariant 7 at the layer a `record:` path opens, asserted once over all three producers
    // rather than three times: `crates/protocol-cli/src/drive.rs`'s `read_record` refuses a record
    // whose producer is a human, so a producer that wrote one would turn a step into a step that
    // silently submits nothing. The rule that keeps that from happening is that no producer can
    // name its own producer at all.
    let directory = scratch("producers");
    specification_store(&directory);

    let tree = root();
    let task = directory.join("task.yaml");
    let documents = [
        (
            "verification.yaml",
            vec!["validate", "--root", printable(&tree)],
        ),
        ("property.yaml", vec!["property", "evidence"]),
        (
            "specification.yaml",
            vec![
                "specification",
                "evidence",
                "--store",
                printable(&directory),
                "--task",
                printable(&task),
            ],
        ),
    ];

    for (name, verb) in documents {
        let out = directory.join(name);
        let flag = if verb[0] == "validate" {
            "--evidence"
        } else {
            "--out"
        };
        let mut args = verb.clone();
        args.push(flag);
        args.push(printable(&out));
        let output = protocol(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let record = only_record(&out);
        assert!(
            !record.producer.is_human(),
            "{name} names a person as its producer, and a driven step cannot submit one: {:?}",
            record.producer
        );
        assert!(
            !matches!(record.evidence, Evidence::Approval(_)),
            "{name} is an approval, which reaches an execution through a person running \
             `protocol evaluate --evidence` and never through a step"
        );
        assert!(
            matches!(record.producer, Producer::Verifier { .. }),
            "{name} must be a verifier's for `independent: true` to be satisfied: {:?}",
            record.producer
        );
    }
}
