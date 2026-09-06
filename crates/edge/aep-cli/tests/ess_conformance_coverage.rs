//! Actual coverage source admission through the public CLI and its replay routes.
#[path = "../../../observe/aep-ess-evidence/tests/support/count_reader_pass1.rs"]
mod count_fixtures;
#[path = "../../../observe/aep-ess-evidence/tests/support/coverage.rs"]
mod fixtures;

use aep_domain::ess_conformance_coverage::EssConformanceCoverageExpectation;
use aep_domain::evidence::{EvidenceRecord, Producer, Provenance};
use aep_domain::requirement::{EvidenceRequirement, RecordQualification, RequirementContext};
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

struct Context {
    expectation: Option<EssConformanceCoverageExpectation>,
    subject: Option<aep_domain::SubjectRef>,
    graph: aep_domain::ArtifactGraph,
    facts: aep_domain::FactStore,
    evidence: Vec<EvidenceRecord>,
    now: Option<Timestamp>,
}
impl RequirementContext for Context {
    fn ess_conformance_coverage_expectation(&self) -> Option<&EssConformanceCoverageExpectation> {
        self.expectation.as_ref()
    }
    fn task_subject(&self) -> Option<&aep_domain::SubjectRef> {
        self.subject.as_ref()
    }
    fn artifacts(&self) -> &aep_domain::ArtifactGraph {
        &self.graph
    }
    fn facts(&self) -> &dyn aep_domain::facts::FactSource {
        &self.facts
    }
    fn evidence(&self) -> &[EvidenceRecord] {
        &self.evidence
    }
    fn now(&self) -> Option<Timestamp> {
        self.now
    }
}
fn context() -> Context {
    let selected_suite = fixtures::suite();
    // The independently chosen suite, model and selection are fixture inputs, not reader output.
    let expectation = serde_json::from_value(serde_json::json!({
        "model":"executable-system-specification:demo", "suite":fixtures::reference(&selected_suite.to_string()),
        "selection":selected_suite["coverage"]["selection"], "selected_ids":[fixtures::SELECTED]
    })).unwrap();
    let graph = aep_schema::parse::artifact_manifest(&format!("version: aep.artifacts/1\nartifacts:\n  - id: executable-system-specification:demo\n    kind: executable-system-specification\n    status: approved\n    model_digest: {}\n    location: {{path: demo.yaml}}\n", fixtures::MODEL), None).unwrap();
    let (report, input) = fixtures::pair(1);
    let value = aep_ess_evidence::adapt_json_coverage(&report, &input)
        .unwrap()
        .evidence()
        .clone();
    Context {
        expectation: Some(expectation),
        subject: Some("service:demo".parse().unwrap()),
        now: Some(Timestamp::from_epoch_millis(2)),
        graph,
        facts: aep_domain::FactStore::new(),
        evidence: vec![EvidenceRecord {
            id: "coverage-one".parse().unwrap(),
            value,
            observed_at: ObservedAt::new(Timestamp::from_epoch_millis(1)),
            produced_at: Timestamp::from_epoch_millis(1),
            producer: Producer::Verifier {
                verifier: Verifier::ConformanceRunner,
            },
            subject: Some("service:demo".parse().unwrap()),
            provenance: Provenance::default(),
        }],
    }
}
fn requirement() -> EvidenceRequirement {
    serde_json::from_str(r#"{"kind":"ess_conformance_coverage_v1","at_least":1,"independent":true,"verifier":"conformance-runner"}"#).unwrap()
}

fn assert_decision(
    context: &Context,
    required: &EvidenceRequirement,
    expected: &str,
    contradiction: bool,
) {
    let result = required.qualify_record(&context.evidence[0], context);
    assert_eq!(
        matches!(result, RecordQualification::Contradiction(_)),
        contradiction,
        "{result:?}"
    );
    assert_eq!(result.issue().expect("refusal").reason, expected);
    let requirements = aep_domain::requirement::RequirementSet {
        evidence: vec![required.clone()],
        ..Default::default()
    };
    assert!(!requirements.evaluate(context).is_satisfied());
}

fn change_expectation(context: &mut Context, change: impl FnOnce(&mut serde_json::Value)) {
    let mut value = serde_json::to_value(context.expectation.as_ref().unwrap()).unwrap();
    change(&mut value);
    context.expectation = Some(serde_json::from_value(value).unwrap());
}

fn model_graph(kind: &str, digest: Option<&str>) -> aep_domain::ArtifactGraph {
    let digest = digest.map_or_else(String::new, |value| format!("    model_digest: {value}\n"));
    aep_schema::parse::artifact_manifest(&format!("version: aep.artifacts/1\nartifacts:\n  - id: executable-system-specification:demo\n    kind: {kind}\n    status: approved\n{digest}    location: {{path: demo.yaml}}\n"), None).unwrap()
}

#[test]
fn coverage_qualification_requires_every_independent_identity_and_envelope_control() {
    type Change = fn(&mut Context);
    let cases: &[(&str, bool, Change)] = &[
        ("MissingExpectation", false, |c| c.expectation = None),
        ("MissingTaskSubject", false, |c| c.subject = None),
        ("SubjectMismatch", true, |c| c.evidence[0].subject = None),
        ("SubjectMismatch", true, |c| {
            c.subject = Some("service:other".parse().unwrap());
        }),
        ("MissingModel", false, |c| {
            c.graph = aep_domain::ArtifactGraph::new();
        }),
        ("MissingModelDigest", false, |c| {
            c.graph = model_graph("executable-system-specification", None);
        }),
        ("ModelDigestMismatch", true, |c| {
            c.graph = model_graph("executable-system-specification", Some(&"b".repeat(64)));
        }),
        ("SuiteReferenceMismatch", true, |c| {
            change_expectation(c, |v| {
                v["suite"]["digest"] = format!("sha256:{}", "b".repeat(64)).into();
            });
        }),
        ("SelectionMismatch", true, |c| {
            change_expectation(c, |v| v["selection"]["origins"] = "generated".into());
        }),
        ("SelectionMismatch", true, |c| {
            change_expectation(c, |v| {
                v["selection"]["scope"] =
                    serde_json::json!({"kind":"component","component":"other"});
            });
        }),
        ("SelectionMismatch", true, |c| {
            change_expectation(c, |v| v["selected_ids"] = serde_json::json!([]));
        }),
        ("SelectionMismatch", true, |c| {
            change_expectation(
                c,
                |v| v["selection"]["filter"] = serde_json::json!({"kind":"explicit","ids":[fixtures::SELECTED],"parent":fixtures::reference("independent-parent")}),
            );
        }),
        ("ObservationMismatch", true, |c| {
            c.evidence[0].observed_at = ObservedAt::new(Timestamp::from_epoch_millis(2));
        }),
        ("MissingTime", false, |c| c.now = None),
        ("FutureObservation", true, |c| {
            c.now = Some(Timestamp::from_epoch_millis(0));
        }),
        ("MissingAdmission", false, |c| {
            c.evidence[0] =
                serde_json::from_str(&serde_json::to_string(&c.evidence[0]).unwrap()).unwrap();
        }),
    ];
    for (reason, contradiction, change) in cases {
        let mut c = context();
        change(&mut c);
        assert_decision(&c, &requirement(), reason, *contradiction);
    }
    let mut c = context();
    c.evidence[0].producer = Producer::Agent {
        id: "forger".into(),
    };
    c.evidence[0].provenance.tool = Some("conformance-runner".parse().unwrap());
    assert_decision(&c, &requirement(), "ProducerMismatch", true);
    let mut required = requirement();
    required.at_least = 0;
    assert_decision(&context(), &required, "InvalidRequirement", true);
}

#[test]
fn coverage_horizon_equality_and_no_implicit_ttl_share_requirement_evaluation() {
    let mut c = context();
    let mut required = requirement();
    required.horizon = Some(aep_domain::time::Horizon::days(1).unwrap());
    c.now = Some(Timestamp::from_epoch_millis(86_400_001));
    assert_eq!(
        required.qualify_record(&c.evidence[0], &c),
        RecordQualification::Qualified
    );
    c.now = Some(Timestamp::from_epoch_millis(86_400_002));
    assert_decision(&c, &required, "StaleObservation", false);
    required.horizon = None;
    c.now = Some(Timestamp::from_epoch_millis(u64::MAX));
    let requirements = aep_domain::requirement::RequirementSet {
        evidence: vec![required],
        ..Default::default()
    };
    assert!(requirements.evaluate(&c).is_satisfied());
}

#[test]
fn coverage_unknown_refused_and_empty_are_descriptive_even_when_all_executed_cases_pass() {
    for (reason, selected) in [
        ("UnknownCoverage", vec![fixtures::SELECTED]),
        ("InScopeRefusal", vec![fixtures::SELECTED]),
        ("EmptySelection", vec![]),
    ] {
        let mut suite = fixtures::suite();
        match reason {
            "UnknownCoverage" => suite["coverage"]["knowledge"] = "unknown".into(),
            "InScopeRefusal" => {
                suite["coverage"]["refused"] =
                    serde_json::json!([fixtures::refusal(), fixtures::refusal()]);
                suite["coverage"]["counts"]["refused"] = 2.into();
            }
            _ => {
                suite["scenarios"] = serde_json::json!({});
                suite["coverage"]["generated"] = serde_json::json!([]);
                suite["coverage"]["counts"]["generated"] = 0.into();
            }
        }
        let (report, input) = fixtures::pair_for(&suite, &selected, "inconclusive", &[]);
        let mut c = context();
        c.evidence[0].value = aep_ess_evidence::adapt_json_coverage(&report, &input)
            .unwrap()
            .evidence()
            .clone();
        change_expectation(&mut c, |v| {
            v["suite"] = fixtures::reference(&suite.to_string());
            v["selected_ids"] = serde_json::json!(selected);
        });
        assert_decision(&c, &requirement(), reason, false);
    }
}

#[test]
fn coverage_terminal_failures_inconclusive_runs_and_separate_records_never_join_into_qualification()
{
    for (profile, category, status, reason, contradiction) in [
        (
            "rust-scenario-status/1",
            "failed",
            "failed",
            "ExecutionFailed",
            true,
        ),
        (
            "rust-scenario-status/1",
            "unsupported",
            "failed",
            "ExecutionFailed",
            true,
        ),
        (
            "rust-scenario-status/1",
            "error",
            "inconclusive",
            "ExecutionInconclusive",
            false,
        ),
        (
            "go-scenario-status/1",
            "skipped",
            "inconclusive",
            "ExecutionInconclusive",
            false,
        ),
        (
            "go-scenario-status/1",
            "failed",
            "failed",
            "ExecutionFailed",
            true,
        ),
    ] {
        let (report, input) = fixtures::pair(1);
        let mut report: serde_json::Value = serde_json::from_str(&report).unwrap();
        report["producer_profile"] = profile.into();
        report["counts"]["passed"] = 0.into();
        report["counts"][category] = 1.into();
        report["outcomes"]["passed"] = serde_json::json!([]);
        report["outcomes"][category] = serde_json::json!([fixtures::SELECTED]);
        report["execution_status"] = status.into();
        report["conformance_status"] = status.into();
        let mut c = context();
        c.evidence[0].value = aep_ess_evidence::adapt_json_coverage(&report.to_string(), &input)
            .unwrap()
            .evidence()
            .clone();
        assert_decision(&c, &requirement(), reason, contradiction);
    }
    let mut c = context();
    let mut second = c.evidence[0].clone();
    c.evidence[0].subject = None;
    second.producer = Producer::Agent {
        id: "claims-all-pass".into(),
    };
    c.evidence.push(second);
    assert!(!aep_domain::requirement::RequirementSet {
        evidence: vec![requirement()],
        ..Default::default()
    }
    .evaluate(&c)
    .is_satisfied());
    let mut c = context();
    c.graph = aep_schema::parse::artifact_manifest("version: aep.artifacts/1\nartifacts:\n  - id: design:demo\n    kind: design\n    status: approved\n    location: {path: design.md}\n", None).unwrap();
    change_expectation(&mut c, |v| v["model"] = "design:demo".into());
    assert_decision(&c, &requirement(), "ModelKindMismatch", true);
}

#[test]
fn coverage_typed_envelopes_preserve_legacy_observation_lexemes_but_require_exact_report_instants()
{
    let (report, input) = fixtures::pair(1);
    let entry =
        serde_json::to_value(aep_ess_evidence::adapt_json_coverage(&report, &input).unwrap())
            .unwrap();
    let raw = serde_json::json!([entry]).to_string();
    let decimal = raw.replace("\"observed_at\":1", "\"observed_at\":1.0");
    assert_ne!(decimal, raw);
    assert_eq!(
        aep_schema::parse::evidence_list_with_readers(&decimal, None, &readers()).unwrap()[0]
            .observed_at
            .timestamp()
            .epoch_millis(),
        1
    );
    let (report, input) = fixtures::pair(9_007_199_254_740_993);
    let entry =
        serde_json::to_value(aep_ess_evidence::adapt_json_coverage(&report, &input).unwrap())
            .unwrap();
    let raw = serde_json::json!([entry]).to_string();
    let rounded = raw.replace(
        "\"observed_at\":9007199254740993",
        "\"observed_at\":9007199254740993.0",
    );
    assert_ne!(rounded, raw);
    assert!(
        aep_schema::parse::evidence_list_with_readers(&rounded, None, &readers())
            .unwrap_err()
            .to_string()
            .contains("ObservationMismatch")
    );
    let mut bad: serde_json::Value = serde_json::from_str(&raw).unwrap();
    bad[0]["about"] = "service:demo".into();
    bad[0]["envelope_subject"] = "service:demo".into();
    assert!(
        aep_schema::parse::evidence_list_with_readers(&bad.to_string(), None, &readers())
            .unwrap_err()
            .to_string()
            .contains("duplicate field")
    );
}

#[test]
fn coverage_outside_refusal_qualifies_only_the_independently_selected_component() {
    let suite = fixtures::outside_suite();
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
    let mut c = context();
    c.evidence[0].value = aep_ess_evidence::adapt_json_coverage(&report, &input)
        .unwrap()
        .evidence()
        .clone();
    change_expectation(&mut c, |v| {
        v["suite"] = fixtures::reference(&suite.to_string());
        v["selection"] = suite["coverage"]["selection"].clone();
    });
    assert_eq!(
        requirement().qualify_record(&c.evidence[0], &c),
        RecordQualification::Qualified
    );
    change_expectation(&mut c, |v| {
        v["selection"]["scope"] = serde_json::json!({"kind":"system"});
    });
    assert_decision(&c, &requirement(), "SelectionMismatch", true);
}

#[test]
fn coverage_complete_record_matches_every_independent_qualification_input() {
    let context = context();
    assert_eq!(
        requirement().qualify_record(&context.evidence[0], &context),
        RecordQualification::Qualified
    );
    assert!(!requirement().matches(&context.evidence[0]));
}

#[test]
fn coverage_requirement_diagnostics_name_the_separate_kind() {
    let mut c = context();
    c.evidence.clear();
    let mut requirements = aep_domain::requirement::RequirementSet {
        evidence: vec![requirement()],
        ..Default::default()
    };
    assert!(requirements.evaluate(&c).items[0]
        .detail
        .as_ref()
        .unwrap()
        .contains("coverage"));
    requirements.evidence[0].at_least = 0;
    assert!(requirements.evaluate(&c).items[0]
        .detail
        .as_ref()
        .unwrap()
        .contains("coverage"));
}

#[test]
fn coverage_direct_record_without_reader_refuses_before_publishing_state() {
    let context = context();
    let task = aep_schema::parse::task("id: COVERAGE-RAW\nkind: feature\nobjective: coverage admission\nprotocol: adp/1\nprofile: development.standard\nsubject: service:demo\n", None).unwrap();
    let engine = aep_engine::Engine::with_clock(
        aep_project::load_tree(&root()).unwrap(),
        aep_engine::FixedClock::new(2),
    );
    let mut execution = engine
        .initialize_with_artifacts(task, context.graph)
        .unwrap();
    let before = serde_json::to_vec(&execution.snapshot()).unwrap();
    let error = execution
        .record_evidence(context.evidence[0].clone())
        .expect_err("even an in-process reading must be re-admitted by this execution's reader");
    assert!(error.to_string().contains("MissingReader"), "{error}");
    assert_eq!(serde_json::to_vec(&execution.snapshot()).unwrap(), before);
}

fn cli(args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(root())
        .args(args)
        .output()
        .unwrap()
}
fn success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn scratch(name: &str) -> PathBuf {
    let path = root()
        .join("target/ess-conformance-coverage/fixtures")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn readers() -> aep_domain::ess_conformance_coverage::EssEvidenceReaders {
    aep_domain::ess_conformance_coverage::EssEvidenceReaders {
        count: Some(std::sync::Arc::new(aep_ess_evidence::CountStageReader)),
        coverage: Some(std::sync::Arc::new(aep_ess_evidence::CoverageReader)),
    }
}

fn policy_fixture(directory: &Path) -> PathBuf {
    let documents = directory.join("documents");
    for (path, text) in [
        ("protocols/coverage/1.yaml", "id: coverage\nversion: 1\ntitle: Coverage fixture\nphases: [verification, completion]\ncapabilities: [tests.execute, command.execute, repository.read, repository.write]\nevidence_kinds: [ess_conformance_coverage_v1, ess_conformance_v2]\nverifiers: [conformance-runner]\nobservables: ['ess_conformance_coverage_v1.**', 'ess_conformance_v2.**', 'evidence.**', 'state.**', 'workflow.**', 'task.**']\n"),
        ("principles/coverage.yaml", "id: coverage\nversion: 1\ntitle: Coverage\nrequires:\n  before_completion:\n    evidence:\n      - kind: ess_conformance_coverage_v1\n        independent: true\n        verifier: conformance-runner\nverification:\n  - verifier: conformance-runner\n"),
        ("profiles/coverage.yaml", "id: coverage.test\nversion: 1\ntitle: Coverage\nprotocol: coverage/1\nworkflow: coverage/default\nprinciples: [coverage]\ncapabilities:\n  allow: [tests.execute, command.execute, repository.read, repository.write]\ncompletion: [evidence.missing == 0]\n"),
        ("workflows/coverage/default.yaml", "id: coverage/default\nversion: 1\ntitle: Coverage\ninitial: verify\nstates:\n  verify:\n    title: Verify\n    phases: [verification]\n  complete:\n    title: Complete\n    terminal: true\n    phases: [completion]\ntransitions:\n  - from: verify\n    to: complete\n    when: evidence.missing == 0\n"),
    ] {
        let path = documents.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    documents
}

fn coverage_task() -> aep_domain::Task {
    let mut task = aep_schema::parse::task("id: COVERAGE-DRIVE\nkind: feature\nobjective: independently selected coverage\nprotocol: coverage/1\nprofile: coverage.test\nsubject: service:demo\n", None).unwrap();
    task.constraints.ess_conformance_coverage_v1 = context().expectation;
    task
}

fn configured_engine(documents: &Path, now: u64) -> aep_engine::Engine<aep_engine::FixedClock> {
    aep_engine::Engine::with_clock(
        aep_project::load_tree(documents).unwrap(),
        aep_engine::FixedClock::new(now),
    )
    .with_ess_conformance_coverage_reader(std::sync::Arc::new(aep_ess_evidence::CoverageReader))
    .with_ess_conformance_v2_reader(std::sync::Arc::new(aep_ess_evidence::CountStageReader))
}

#[test]
fn coverage_both_decision_routes_and_replay_share_the_exact_record_with_atomic_refusals() {
    use aep_domain::facts::FactSource;
    let documents = policy_fixture(&scratch("execution"));
    let engine = configured_engine(&documents, 2);
    let c = context();
    let mut execution = engine
        .initialize_with_artifacts(coverage_task(), c.graph.clone())
        .unwrap();
    assert_eq!(
        execution
            .fact_store()
            .fact(&"evidence.missing".parse().unwrap())
            .unwrap()
            .to_string(),
        "1"
    );
    execution.record_evidence(c.evidence[0].clone()).unwrap();
    assert!(aep_domain::requirement::RequirementSet {
        evidence: vec![requirement()],
        ..Default::default()
    }
    .evaluate(&execution)
    .is_satisfied());
    assert_eq!(
        execution
            .fact_store()
            .fact(&"evidence.missing".parse().unwrap())
            .unwrap()
            .to_string(),
        "0"
    );
    let before = serde_json::to_vec(&execution.snapshot()).unwrap();
    let before_facts = execution.fact_store().clone();
    let before_time = execution.evaluated_at();
    let raw: aep_engine::execution::Snapshot = serde_json::from_slice(&before).unwrap();
    assert!(raw.evidence[0].record.facts().is_empty());
    let restored = engine
        .restore(coverage_task(), c.graph.clone(), raw)
        .unwrap();
    assert_eq!(serde_json::to_vec(&restored.snapshot()).unwrap(), before);
    assert_eq!(restored.fact_store(), &before_facts);
    for index in 0..2 {
        let mut bad = c.evidence[0].clone();
        let (report, input) = fixtures::pair(1);
        bad.value = aep_domain::Evidence::EssConformanceCoverageV1(
            aep_domain::ess_conformance_coverage::EssConformanceCoverageSources::new(
                report,
                input.replace("demo.core.Read", "demo.core.Other"),
            ),
        );
        let mut snapshot = execution.snapshot();
        if index == 0 {
            snapshot.evidence[0].record = bad.clone();
        } else {
            let mut extra = snapshot.evidence[0].clone();
            extra.record = bad.clone();
            snapshot.evidence.push(extra);
        }
        assert!(engine
            .restore(coverage_task(), c.graph.clone(), snapshot)
            .unwrap_err()
            .to_string()
            .contains("SuiteDigestMismatch"));
        assert!(execution
            .record_evidence(bad)
            .unwrap_err()
            .to_string()
            .contains("SuiteDigestMismatch"));
        assert_eq!(serde_json::to_vec(&execution.snapshot()).unwrap(), before);
        assert_eq!(execution.fact_store(), &before_facts);
        assert_eq!(execution.evaluated_at(), before_time);
    }
    let no_reader = aep_engine::Engine::with_clock(
        aep_project::load_tree(&documents).unwrap(),
        aep_engine::FixedClock::new(2),
    );
    assert!(no_reader
        .restore(
            coverage_task(),
            c.graph.clone(),
            serde_json::from_slice(&before).unwrap()
        )
        .unwrap_err()
        .to_string()
        .contains("MissingReader"));
    assert!(configured_engine(&documents, 0)
        .restore(
            coverage_task(),
            c.graph,
            serde_json::from_slice(&before).unwrap()
        )
        .unwrap_err()
        .to_string()
        .contains("FutureObservation"));
}

#[test]
fn coverage_json_yaml_original_lineage_and_full_time_survive_both_reader_configuration() {
    let parent = fixtures::suite();
    let child = fixtures::child(&parent, &[fixtures::SELECTED]);
    let parent_bytes = format!("\r\n{}\r\n", serde_json::to_string_pretty(&parent).unwrap());
    let mut child = child;
    child["coverage"]["selection"]["filter"]["parent"] = fixtures::reference(&parent_bytes);
    let (report, input) =
        fixtures::pair_for(&child, &[fixtures::SELECTED], "passed", &[parent_bytes]);
    let report = report.replace(
        "\"completed_at\":1",
        &format!("\"completed_at\":{}", u64::MAX),
    );
    let entry =
        serde_json::to_value(aep_ess_evidence::adapt_json_coverage(&report, &input).unwrap())
            .unwrap();
    for text in [
        serde_json::to_string(&vec![entry.clone()]).unwrap(),
        serde_yaml::to_string(&vec![entry.clone()]).unwrap(),
    ] {
        let parsed =
            aep_schema::parse::evidence_list_with_readers(&text, None, &readers()).unwrap();
        assert_eq!(parsed[0].observed_at.timestamp().epoch_millis(), u64::MAX);
        let aep_domain::Evidence::EssConformanceCoverageV1(source) = &parsed[0].evidence else {
            panic!("coverage");
        };
        assert_eq!(source.report_json(), report);
        assert_eq!(source.suite_input_json(), input);
        assert!(aep_schema::parse::evidence_list(&text, None)
            .unwrap_err()
            .to_string()
            .contains("MissingReader"));
        assert!(aep_schema::parse::evidence_list_with_reader(
            &text,
            None,
            &aep_ess_evidence::CountStageReader
        )
        .unwrap_err()
        .to_string()
        .contains("MissingReader"));
    }
    for key in ["verified", "reading", "suite_json", "counts", "extra"] {
        let mut bad = entry.clone();
        bad[key] = true.into();
        assert!(aep_schema::parse::evidence_list_with_readers(
            &serde_json::json!([bad]).to_string(),
            None,
            &readers()
        )
        .unwrap_err()
        .to_string()
        .contains("UnknownField"));
    }
    let suite = count_fixtures::suite();
    let count = serde_json::to_value(
        aep_ess_evidence::adapt_json_v2(&count_fixtures::report(&suite, 1), &suite).unwrap(),
    )
    .unwrap();
    assert_eq!(
        aep_schema::parse::evidence_list_with_readers(
            &serde_json::json!([entry, count]).to_string(),
            None,
            &readers()
        )
        .unwrap()
        .len(),
        2
    );
}

#[test]
fn coverage_actual_driver_reads_typed_record_and_re_admits_on_status_and_resume() {
    let directory = scratch("driver");
    let documents = policy_fixture(&directory);
    let task_path = directory.join("task.yaml");
    std::fs::write(&task_path, serde_yaml::to_string(&coverage_task()).unwrap()).unwrap();
    let (report, input) = fixtures::pair(1);
    let mut entry =
        serde_json::to_value(aep_ess_evidence::adapt_json_coverage(&report, &input).unwrap())
            .unwrap();
    entry["about"] = "service:demo".into();
    let record_path = directory.join("record.json");
    std::fs::write(&record_path, serde_json::json!([entry]).to_string()).unwrap();
    let map_path = directory.join("steps.yaml");
    let map = format!("format: aep.driver-steps/1\nid: fixture/coverage\nworkflow: coverage/default/1\nstates:\n  verify:\n    steps:\n      - kind: command\n        run: [/usr/bin/true]\n        evidence:\n          kind: ess_conformance_coverage_v1\n          verifier: conformance-runner\n          record: {}\n      - kind: operator\n        prompt: confirm the selected inventory\n", record_path.display());
    std::fs::write(&map_path, &map).unwrap();
    let location = [
        "--project",
        directory.to_str().unwrap(),
        "--root",
        documents.to_str().unwrap(),
        "--task",
        task_path.to_str().unwrap(),
        "--map",
        map_path.to_str().unwrap(),
    ];
    let mut args = vec!["drive", "run"];
    args.extend(location);
    args.extend(["--max-iterations", "1", "--pause-on-approval"]);
    let output = cli(&args);
    let read =
        aep_driver::run::RunDirectory::at(directory.join(".engineering/runs/COVERAGE-DRIVE/1"));
    let snapshot = read.read_snapshot().unwrap_or_else(|error| {
        panic!(
            "{error}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(snapshot.evidence.len(), 1);
    assert!(snapshot.evidence[0].record.facts().is_empty());
    let aep_domain::Evidence::EssConformanceCoverageV1(sources) =
        &snapshot.evidence[0].record.value
    else {
        panic!("coverage");
    };
    assert_eq!(sources.report_json(), report);
    assert_eq!(sources.suite_input_json(), input);
    for verb in ["status", "resume"] {
        let mut args = vec!["drive", verb, "COVERAGE-DRIVE/1"];
        args.extend(location);
        if verb == "resume" {
            args.push("--pause-on-approval");
        }
        let output = cli(&args);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !text.contains("MissingReader") && !text.contains("ObservationMismatch"),
            "{text}"
        );
        assert!(text.contains("COVERAGE-DRIVE/1"), "{text}");
        assert_eq!(read.read_snapshot().unwrap().evidence.len(), 1);
    }
    assert_eq!(
        std::fs::read_dir(directory.join(".engineering/runs/COVERAGE-DRIVE"))
            .unwrap()
            .count(),
        1
    );
    let without_record = map
        .lines()
        .filter(|line| !line.trim_start().starts_with("record:"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(aep_schema::parse::step_map(&without_record, None)
        .unwrap_err()
        .to_string()
        .contains("cannot establish `ess_conformance_coverage_v1`"));
}

#[test]
fn coverage_planning_pair_dispatch_refuses_before_store_and_aliases_are_identical() {
    let directory = scratch("planning-boundaries");
    let report_path = directory.join("report.json");
    let input_path = directory.join("input.json");
    let suite_path = directory.join("suite.json");
    let absent = directory.join("absent-store");
    let (report, input) = fixtures::pair(u64::MAX);
    std::fs::write(&report_path, report).unwrap();
    std::fs::write(&input_path, input).unwrap();
    std::fs::write(&suite_path, fixtures::suite().to_string()).unwrap();
    let args = [
        "plan",
        "artifact",
        "evidence",
        "story:missing",
        "--from",
        report_path.to_str().unwrap(),
        "--suite-input",
        input_path.to_str().unwrap(),
        "--store",
        absent.to_str().unwrap(),
    ];
    let output = cli(&args);
    let alias = std::process::Command::new(env!("CARGO_BIN_EXE_protocol"))
        .current_dir(root())
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(alias.status.code(), output.status.code());
    assert_eq!(alias.stdout, output.stdout);
    assert_eq!(alias.stderr, output.stderr);
    assert!(String::from_utf8_lossy(&output.stderr).contains("PlanningTimestampUnsupported"));
    assert!(!absent.exists());
    let mut both = args.to_vec();
    both.extend(["--suite", suite_path.to_str().unwrap()]);
    assert_eq!(cli(&both).status.code(), Some(2));
    let filtered = fixtures::child(&fixtures::suite(), &[fixtures::SELECTED]);
    let (report, _) = fixtures::pair_for(
        &filtered,
        &[fixtures::SELECTED],
        "passed",
        &[fixtures::suite().to_string()],
    );
    std::fs::write(&report_path, report).unwrap();
    std::fs::write(&suite_path, filtered.to_string()).unwrap();
    let args = [
        "plan",
        "artifact",
        "evidence",
        "story:missing",
        "--from",
        report_path.to_str().unwrap(),
        "--suite",
        suite_path.to_str().unwrap(),
        "--store",
        absent.to_str().unwrap(),
    ];
    let output = cli(&args);
    assert!(String::from_utf8_lossy(&output.stderr).contains("MissingParent"));
    assert!(!absent.exists());
    for format in ["ess-conformance-report/1", "ess-conformance-run/2"] {
        std::fs::write(
            &report_path,
            serde_json::json!({"format":format}).to_string(),
        )
        .unwrap();
        assert!(!cli(&args).status.success());
        assert!(!absent.exists());
    }
}

#[test]
fn coverage_planning_admits_original_input_as_descriptive_history() {
    let directory = root()
        .join("target/ess-conformance-coverage/fixtures/planning")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&directory).unwrap();
    let (report, input) = fixtures::pair(1);
    let report_path = directory.join("report.json");
    let input_path = directory.join("input.json");
    std::fs::write(&report_path, report).unwrap();
    std::fs::write(&input_path, input).unwrap();
    let store = directory.join("store");
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        "story",
        "coverage",
        "--title",
        "coverage",
        "--store",
        store.to_str().unwrap(),
    ]));
    let output = cli(&[
        "plan",
        "artifact",
        "evidence",
        "story:coverage",
        "--from",
        report_path.to_str().unwrap(),
        "--suite-input",
        input_path.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
    ]);
    success(&output);
    let output = cli(&[
        "plan",
        "artifact",
        "history",
        "story:coverage",
        "--store",
        store.to_str().unwrap(),
    ]);
    success(&output);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("ess_conformance_coverage_v1"), "{text}");
}

#[test]
fn coverage_planning_wraps_exact_unfiltered_bytes_and_escapes_full_descriptive_diagnostics() {
    let directory = scratch("raw-planning");
    let mut suite = fixtures::outside_suite();
    let message = "outside \"selection\"\nnot an extra history event";
    suite["coverage"]["refused"][0]["message"] = message.into();
    let original = format!("\r\n{suite}\r\n");
    let (report, _) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
    let mut report: serde_json::Value = serde_json::from_str(&report).unwrap();
    report["suite"] = fixtures::reference(&original);
    report["completed_at"] = 253_402_300_799_999_u64.into();
    let label = "fixture \"producer\"\nwith unicode café";
    report["implementation"] = label.into();
    let report_path = directory.join("report.json");
    let suite_path = directory.join("suite.json");
    let store = directory.join("store");
    std::fs::write(&report_path, report.to_string()).unwrap();
    std::fs::write(&suite_path, &original).unwrap();
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        "story",
        "raw",
        "--title",
        "Raw fixture",
        "--store",
        store.to_str().unwrap(),
    ]));
    success(&cli(&[
        "plan",
        "artifact",
        "evidence",
        "story:raw",
        "--from",
        report_path.to_str().unwrap(),
        "--suite",
        suite_path.to_str().unwrap(),
        "--ref",
        "a \"reference\"\nnext line",
        "--store",
        store.to_str().unwrap(),
    ]));
    let output = cli(&[
        "plan",
        "artifact",
        "history",
        "story:raw",
        "--store",
        store.to_str().unwrap(),
        "--format",
        "json",
    ]);
    success(&output);
    let history: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(history.as_array().unwrap().len(), 2);
    let change = &history[1]["change"];
    assert_eq!(change["reference"], "a \"reference\"\nnext line");
    let source: serde_json::Value =
        serde_json::from_str(change["source"].as_str().unwrap()).unwrap();
    assert_eq!(source["implementation"], label);
    assert_eq!(source["completed_at"], "253402300799999");
    assert_eq!(source["input_transport"], "wrapped_raw_suite");
    assert_eq!(source["suite"], fixtures::reference(&original));
    assert_eq!(
        source["selected_ids"],
        serde_json::json!([fixtures::SELECTED])
    );
    assert_eq!(
        source["counts"],
        serde_json::json!({"total":1,"passed":1,"failed":0,"error":0,"unsupported":0,"skipped":0})
    );
    assert_eq!(
        source["coverage"]["counts"],
        serde_json::json!({"generated":1,"authored":0,"outside":1,"refused":1})
    );
    assert_eq!(source["coverage"]["refused"][0]["message"], message);
    assert_eq!(source["conformance_status"], "passed");
}

#[test]
fn coverage_opt_in_profile_composes_without_inheriting_count_requirements() {
    let task = aep_schema::parse::task("id: COVERAGE-POLICY\nkind: feature\nobjective: complete exact coverage\nprotocol: adp-ess-conformance-coverage/1\nprofile: development.ess-conformance-coverage\nsubject: service:demo\n", None).unwrap();
    let registry = aep_project::load_tree(&root()).unwrap();
    let plan =
        aep_engine::resolve(&task, &registry).expect("the explicit coverage policy composes");
    let ids: Vec<_> = plan
        .principles
        .iter()
        .map(|principle| principle.id.as_str())
        .collect();
    assert!(ids.contains(&"ess-conformance-coverage"));
    assert!(ids.contains(&"contract-testing"));
    assert!(!ids.contains(&"ess-conformance-v2"));
    assert!(!ids.contains(&"ess-conformance"));
}

#[test]
fn coverage_cli_inspection_admits_complete_original_input_and_exact_time() {
    let directory = root().join("target/ess-conformance-coverage/fixtures/inspection");
    std::fs::create_dir_all(&directory).unwrap();
    let (report, input) = fixtures::pair(1);
    let path = directory.join("record.json");
    let wire = serde_json::json!([{
        "kind":"ess_conformance_coverage_v1", "report_json":report, "suite_input_json":input,
        "observed_at":aep_domain::time::ObservedAt::new(aep_domain::time::Timestamp::from_epoch_millis(1)), "producer":{"producer":"verifier","verifier":"conformance-runner"}, "about":"service:demo"
    }]);
    std::fs::write(&path, wire.to_string()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(root())
        .args([
            "observe",
            "evidence",
            "inspect",
            path.to_str().unwrap(),
            "--at",
            "2026-09-06",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("ess_conformance_coverage_v1"), "{text}");
    assert!(text.contains("completed_at"), "{text}");
}
