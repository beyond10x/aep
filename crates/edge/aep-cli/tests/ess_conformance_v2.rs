//! Actual optional reader across typed transport, engine mutation/restore and CLI persistence.
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aep_domain::ess_conformance_v2::{
    EssConformanceV2Expectation, EssConformanceV2Sources, ScenarioId,
};
use aep_domain::evidence::{Evidence, EvidenceRecord, Producer, Provenance};
use aep_domain::facts::{FactPath, FactSource};
use aep_domain::requirement::{EvidenceRequirement, RecordQualification, RequirementContext};
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use aep_engine::engine::{EvidenceSubmission, ProtocolEngine};
use aep_engine::{Engine, FixedClock};
use aep_ess_evidence::{adapt_json_v2, CountStageReader};
use sha2::{Digest, Sha256};

const MODEL: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";
const SELECTED: &str = "demo.core/authored/one";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}
fn pair(time: u64) -> (String, String) {
    let suite = format!(
        r#"{{"provenance":{{"suite_version":"ess-conformance/4","system":"demo","specification_version":"v1","spec_digest":"{MODEL}","contract_digest":"{MODEL}"}},"scenarios":{{"{SELECTED}":{{"purpose":"An independently named scenario","steps":[],"source":[]}}}}}}"#
    );
    let digest =
        Sha256::digest(suite.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            });
    let report = serde_json::json!({"format":"ess-conformance-report/2","specification":"demo/v1","spec_digest":MODEL,"implementation":"fixture","producer_profile":"rust-scenario-status/1","suite":{"version":"ess-conformance/4","digest_profile":"sha256-json-bytes/1","digest":digest},"counts":{"total":1,"passed":1,"failed":0,"error":0,"unsupported":0,"skipped":0},"outcomes":{"passed":[SELECTED],"failed":[],"error":[],"unsupported":[],"skipped":[]},"execution_status":"passed","conformance_status":"inconclusive","coverage":{"knowledge":"unknown"},"policy":"complete-selection/1","completed_at":time}).to_string();
    (report, suite)
}
fn admitted(time: u64) -> Evidence {
    let (report, suite) = pair(time);
    adapt_json_v2(&report, &suite).unwrap().evidence().clone()
}
fn record(time: u64) -> EvidenceRecord {
    EvidenceRecord {
        id: "count-record".parse().unwrap(),
        observed_at: ObservedAt::new(Timestamp::from_epoch_millis(time)),
        produced_at: Timestamp::from_epoch_millis(time),
        producer: Producer::Verifier {
            verifier: Verifier::ConformanceRunner,
        },
        subject: Some("service:demo".parse().unwrap()),
        value: admitted(time),
        provenance: Provenance::default(),
    }
}
fn task() -> aep_domain::Task {
    let mut task = aep_schema::parse::task("id: COUNTS-1\nkind: feature\nobjective: admit exact diagnostics\nprotocol: adp-ess-conformance/1\nprofile: development.ess-conformance-v2\nsubject: service:demo\n", None).unwrap();
    let Evidence::EssConformanceV2(sources) = admitted(1) else {
        panic!("v2");
    };
    task.constraints.ess_conformance_v2 = Some(
        EssConformanceV2Expectation::new(
            "executable-system-specification:demo".parse().unwrap(),
            sources.reading().unwrap().data().suite.clone(),
            vec![ScenarioId::new(SELECTED).unwrap()],
        )
        .unwrap(),
    );
    task
}
fn graph() -> aep_domain::ArtifactGraph {
    aep_schema::parse::artifact_manifest(&format!("version: aep.artifacts/1\nartifacts:\n  - id: executable-system-specification:demo\n    kind: executable-system-specification\n    status: approved\n    model_digest: {MODEL}\n    location: {{path: demo.yaml}}\n"), None).unwrap()
}
fn engine(time: u64, reader: bool) -> Engine<FixedClock> {
    let engine = Engine::with_clock(
        aep_project::load_tree(&root()).unwrap(),
        FixedClock::new(time),
    );
    if reader {
        engine.with_ess_conformance_v2_reader(Arc::new(CountStageReader))
    } else {
        engine
    }
}
fn requirement() -> EvidenceRequirement {
    serde_json::from_str(r#"{"kind":"ess_conformance_v2","at_least":1,"independent":true,"verifier":"conformance-runner"}"#).unwrap()
}
fn submission(record: &EvidenceRecord) -> EvidenceSubmission {
    let mut submission = EvidenceSubmission::new(
        record.value.clone(),
        record.producer.clone(),
        record.observed_at,
    );
    submission.subject.clone_from(&record.subject);
    submission
}
fn reason(decision: RecordQualification) -> String {
    match decision {
        RecordQualification::Unknown(issue) | RecordQualification::Contradiction(issue) => {
            issue.reason.into()
        }
        RecordQualification::Qualified => panic!("count-stage never qualifies"),
    }
}

#[test]
fn actual_policy_composes_and_valid_same_record_reaches_unknown_coverage() {
    let engine = engine(2, true);
    let mut execution = engine.initialize_with_artifacts(task(), graph()).unwrap();
    let principles: Vec<_> = execution
        .plan()
        .principles
        .iter()
        .map(|p| p.id.as_str())
        .collect();
    assert!(principles.contains(&"ess-conformance-v2"));
    assert!(principles.contains(&"contract-testing"));
    assert!(!principles.contains(&"ess-conformance"));
    let record = record(1);
    assert_eq!(
        reason(requirement().qualify_record(&record, &execution)),
        "UnknownCoverage"
    );
    engine
        .submit_evidence(&mut execution, submission(&record))
        .unwrap();
    assert_eq!(
        reason(requirement().qualify_record(&execution.evidence()[0], &execution)),
        "UnknownCoverage"
    );
    assert!(!requirement().matches(&execution.evidence()[0]));
    assert!(engine
        .explain_completion(&execution)
        .outstanding()
        .any(|item| item
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("UnknownCoverage"))));
    assert_ne!(
        execution
            .facts()
            .fact(&FactPath::new("evidence.missing").unwrap()),
        Some(aep_domain::FactValue::count(0))
    );
}

#[test]
fn typed_json_yaml_full_u64_and_raw_snapshot_readback_re_admit_original_bytes() {
    for time in [
        0,
        9_007_199_254_740_993,
        i64::MAX as u64,
        i64::MAX as u64 + 1,
        u64::MAX,
    ] {
        let (report, suite) = pair(time);
        let adapted = adapt_json_v2(&report, &suite).unwrap();
        let mut entry = serde_json::to_value(adapted).unwrap();
        entry["about"] = "service:demo".into();
        for text in [
            serde_json::to_string(&vec![entry.clone()]).unwrap(),
            serde_yaml::to_string(&vec![adapt_json_v2(&report, &suite).unwrap()]).unwrap(),
        ] {
            let parsed =
                aep_schema::parse::evidence_list_with_reader(&text, None, &CountStageReader)
                    .unwrap();
            assert_eq!(parsed[0].observed_at.timestamp().epoch_millis(), time);
            let Evidence::EssConformanceV2(sources) = &parsed[0].evidence else {
                panic!("v2");
            };
            assert_eq!(sources.report_json(), report);
            assert_eq!(sources.suite_json(), suite);
            assert_eq!(
                sources
                    .reading()
                    .unwrap()
                    .data()
                    .completed_at
                    .epoch_millis(),
                time
            );
            assert!(aep_schema::parse::evidence_list(&text, None)
                .unwrap_err()
                .to_string()
                .contains("MissingReader"));
        }
        let engine = engine(time, true);
        let mut execution = engine.initialize_with_artifacts(task(), graph()).unwrap();
        engine
            .submit_evidence(&mut execution, submission(&record(time)))
            .unwrap();
        let wire = serde_json::to_string(&execution.snapshot()).unwrap();
        let raw: aep_engine::execution::Snapshot = serde_json::from_str(&wire).unwrap();
        assert!(raw.evidence[0].record.facts().is_empty());
        assert!(raw.evidence[0].record.value.spec_digest().is_none());
        assert_eq!(
            raw.evidence[0]
                .record
                .observed_at
                .timestamp()
                .epoch_millis(),
            time
        );
        let restored = engine.restore(task(), graph(), raw).unwrap();
        assert_eq!(serde_json::to_string(&restored.snapshot()).unwrap(), wire);
        assert_eq!(
            restored.evidence()[0].value.spec_digest().unwrap().as_str(),
            MODEL
        );
    }
}

#[test]
fn refused_submission_and_restore_leave_nonempty_execution_unchanged() {
    let engine = engine(10, true);
    let mut execution = engine.initialize_with_artifacts(task(), graph()).unwrap();
    engine
        .submit_evidence(&mut execution, submission(&record(1)))
        .unwrap();
    let before = serde_json::to_vec(&execution.snapshot()).unwrap();
    let before_facts = execution.fact_store().clone();
    let before_time = execution.evaluated_at();
    let mut invalid = record(1);
    invalid.observed_at = ObservedAt::new(Timestamp::from_epoch_millis(2));
    assert!(engine
        .submit_evidence(&mut execution, submission(&invalid))
        .unwrap_err()
        .to_string()
        .contains("ObservationMismatch"));
    assert_eq!(serde_json::to_vec(&execution.snapshot()).unwrap(), before);
    assert_eq!(execution.fact_store(), &before_facts);
    assert_eq!(execution.evaluated_at(), before_time);
    let mut snapshot = execution.snapshot();
    let mut bad = snapshot.evidence[0].clone();
    bad.record = invalid;
    snapshot.evidence.push(bad);
    assert!(engine
        .restore(task(), graph(), snapshot)
        .unwrap_err()
        .to_string()
        .contains("ObservationMismatch"));
    let mut future = record(11);
    assert!(engine
        .submit_evidence(&mut execution, submission(&future))
        .unwrap_err()
        .to_string()
        .contains("future"));
    future.observed_at =
        ObservedAt::on_day(aep_domain::time::CivilDate::parse("1970-01-01").unwrap());
    assert!(engine
        .submit_evidence(&mut execution, submission(&future))
        .unwrap_err()
        .to_string()
        .contains("InstantRequired"));
    assert_eq!(serde_json::to_vec(&execution.snapshot()).unwrap(), before);
}

#[test]
fn core_without_reader_and_direct_record_without_time_refuse_even_cached_admission() {
    let no_reader = engine(10, false);
    let mut execution = no_reader
        .initialize_with_artifacts(task(), graph())
        .unwrap();
    for value in [
        admitted(1),
        Evidence::EssConformanceV2(EssConformanceV2Sources::new(pair(1).0, pair(1).1)),
    ] {
        let mut r = record(1);
        r.value = value;
        assert!(no_reader
            .submit_evidence(&mut execution, submission(&r))
            .unwrap_err()
            .to_string()
            .contains("MissingReader"));
        assert!(execution
            .record_evidence(r)
            .unwrap_err()
            .to_string()
            .contains("MissingReader"));
    }
    let registry = aep_project::load_tree(&root()).unwrap();
    let plan = aep_engine::resolve(&task(), &registry).unwrap();
    let mut direct = aep_engine::Execution::new_with_ess_reader(
        "direct".parse().unwrap(),
        plan.clone(),
        graph(),
        Some(Arc::new(CountStageReader)),
    );
    assert!(direct
        .record_evidence(record(1))
        .unwrap_err()
        .to_string()
        .contains("MissingTime"));
    direct.observe_at(Timestamp::from_epoch_millis(10));
    direct.record_evidence(record(1)).unwrap();
    assert!(
        aep_engine::Execution::restore(plan, graph(), direct.snapshot())
            .unwrap_err()
            .to_string()
            .contains("MissingReader")
    );
    assert!(no_reader
        .restore(task(), graph(), direct.snapshot())
        .unwrap_err()
        .to_string()
        .contains("MissingReader"));
}

fn cli(args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(root())
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
    let directory = root()
        .join("target/ess-conformance-v2-counts/fixtures")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}
fn bytes(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn walk(path: &Path, result: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, result);
            } else {
                result.insert(path.clone(), std::fs::read(path).unwrap());
            }
        }
    }
    let mut result = std::collections::BTreeMap::new();
    walk(root, &mut result);
    result
}

// Keep the concrete command/qualification matrix and its unchanged-state assertions together.
#[allow(clippy::too_many_lines)]
#[test]
fn real_planning_pair_preserves_full_u64_diagnostics_and_refusals_do_not_open_store() {
    let directory = scratch("planning-pair");
    let store = directory.join("store");
    let store_arg = store.to_str().unwrap();
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        "story",
        "counts",
        "--title",
        "Count reader",
        "--store",
        store_arg,
    ]));
    // Actual entity-runtime recording accepts through the last millisecond of year9999.
    for time in [0, 1, 1_700_000_000_001, 253_402_300_799_999] {
        let (report, suite) = pair(time);
        let report_path = directory.join(format!("report;{time}.json"));
        let suite_path = directory.join(format!("suite;{time}.json"));
        std::fs::write(&report_path, report).unwrap();
        std::fs::write(&suite_path, suite).unwrap();
        let report_arg = report_path.to_str().unwrap();
        let suite_arg = suite_path.to_str().unwrap();
        success(&cli(&[
            "plan",
            "artifact",
            "evidence",
            "story:counts",
            "--from",
            report_arg,
            "--suite",
            suite_arg,
            "--ref",
            "external:display-only",
            "--store",
            store_arg,
        ]));
        let history = cli(&[
            "plan",
            "artifact",
            "history",
            "story:counts",
            "--store",
            store_arg,
            "--format",
            "json",
        ]);
        success(&history);
        let text = String::from_utf8(history.stdout).unwrap();
        assert!(
            text.contains(&time.to_string()),
            "exact completed_at is durable: {text}"
        );
        assert!(text.contains("ess_conformance_v2"));
        assert!(text.contains("external:display-only"));
        assert!(text.contains("suite_input") && text.contains("report_input"));
        assert!(text.contains("inconclusive") && text.contains("unknown"));
        let journal = std::fs::read_to_string(store.join("journal.jsonl")).unwrap();
        let event: serde_json::Value =
            serde_json::from_str(journal.lines().last().unwrap()).unwrap();
        assert_eq!(event["payload"]["at"].as_u64(), Some(time));
        let source: serde_json::Value =
            serde_json::from_str(event["payload"]["change"]["source"].as_str().unwrap()).unwrap();
        assert_eq!(source["completed_at"], time.to_string());
        assert_eq!(
            source["counts"],
            serde_json::json!({"total":1,"passed":1,"failed":0,"error":0,"unsupported":0,"skipped":0})
        );
        assert_eq!(source["report_input"], report_arg);
        assert_eq!(source["suite_input"], suite_arg);
        assert_eq!(source["execution_status"], "passed");
        assert_eq!(source["conformance_status"], "inconclusive");
        assert_eq!(
            source["coverage"],
            serde_json::json!({"knowledge":"unknown"})
        );
        let before = bytes(&store);
        std::fs::write(&suite_path, format!("{}\n", pair(time).1)).unwrap();
        let refused = cli(&[
            "plan",
            "artifact",
            "evidence",
            "story:counts",
            "--from",
            report_arg,
            "--suite",
            suite_arg,
            "--store",
            store_arg,
        ]);
        assert!(!refused.status.success());
        assert!(String::from_utf8_lossy(&refused.stderr).contains("SuiteDigestMismatch"));
        assert_eq!(bytes(&store), before);
        let missing = directory.join("must-not-open");
        let refused = cli(&[
            "plan",
            "artifact",
            "evidence",
            "story:counts",
            "--from",
            report_arg,
            "--store",
            missing.to_str().unwrap(),
        ]);
        assert!(String::from_utf8_lossy(&refused.stderr).contains("MissingSuite"));
        assert!(!missing.exists());
    }
}

#[test]
fn unrepresentable_planning_dates_refuse_before_opening_or_mutating_the_store() {
    let directory = scratch("planning-date-boundary");
    let store = directory.join("store");
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        "story",
        "counts",
        "--title",
        "Count reader",
        "--store",
        store.to_str().unwrap(),
    ]));
    let before = bytes(&store);
    for time in [
        253_402_300_800_000,
        9_007_199_254_740_993,
        i64::MAX as u64,
        i64::MAX as u64 + 1,
        u64::MAX,
    ] {
        let (report, suite) = pair(time);
        assert_eq!(
            adapt_json_v2(&report, &suite)
                .unwrap()
                .observed_at()
                .timestamp()
                .epoch_millis(),
            time
        );
        let report_path = directory.join(format!("{time}.report.json"));
        let suite_path = directory.join(format!("{time}.suite.json"));
        std::fs::write(&report_path, report).unwrap();
        std::fs::write(&suite_path, suite).unwrap();
        let absent = directory.join("must-not-open");
        for destination in [&store, &absent] {
            let output = cli(&[
                "plan",
                "artifact",
                "evidence",
                "story:counts",
                "--from",
                report_path.to_str().unwrap(),
                "--suite",
                suite_path.to_str().unwrap(),
                "--store",
                destination.to_str().unwrap(),
            ]);
            assert!(!output.status.success());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("PlanningTimestampUnsupported"), "{error}");
            assert!(error.contains(&time.to_string()), "{error}");
            assert!(!error.contains("panicked"), "{error}");
            assert!(!absent.exists());
            assert_eq!(bytes(&store), before);
        }
    }
}

struct Context {
    task: aep_domain::Task,
    graph: aep_domain::ArtifactGraph,
    now: Option<Timestamp>,
    facts: aep_domain::FactStore,
}
impl RequirementContext for Context {
    fn facts(&self) -> &dyn FactSource {
        &self.facts
    }
    fn artifacts(&self) -> &aep_domain::ArtifactGraph {
        &self.graph
    }
    fn evidence(&self) -> &[EvidenceRecord] {
        &[]
    }
    fn now(&self) -> Option<Timestamp> {
        self.now
    }
    fn task_subject(&self) -> Option<&aep_domain::SubjectRef> {
        self.task.subject.as_ref()
    }
    fn ess_conformance_v2_expectation(&self) -> Option<&EssConformanceV2Expectation> {
        self.task.constraints.ess_conformance_v2.as_ref()
    }
}
fn context() -> Context {
    Context {
        task: task(),
        graph: graph(),
        now: Some(Timestamp::from_epoch_millis(10)),
        facts: aep_domain::FactStore::new(),
    }
}

// Keep the concrete command/qualification matrix and its unchanged-state assertions together.
#[allow(clippy::too_many_lines)]
#[test]
fn qualifier_checks_independent_subject_model_suite_selection_producer_and_time_reasons() {
    let r = record(1);
    let required = requirement();
    assert_eq!(
        reason(required.qualify_record(&r, &context())),
        "UnknownCoverage"
    );
    let mut c = context();
    c.task.subject = None;
    assert_eq!(
        reason(required.qualify_record(&r, &c)),
        "MissingTaskSubject"
    );
    c = context();
    c.task.constraints.ess_conformance_v2 = None;
    assert_eq!(
        reason(required.qualify_record(&r, &c)),
        "MissingExpectation"
    );
    c = context();
    c.graph = aep_domain::ArtifactGraph::new();
    assert_eq!(reason(required.qualify_record(&r, &c)), "MissingModel");
    c = context();
    let reference: aep_domain::ArtifactRef =
        "executable-system-specification:demo".parse().unwrap();
    let mut model = c.graph.resolve(&reference).unwrap().clone();
    model.model_digest = None;
    c.graph.insert(model);
    assert_eq!(
        reason(required.qualify_record(&r, &c)),
        "MissingModelDigest"
    );
    c = context();
    let mut model = c.graph.resolve(&reference).unwrap().clone();
    model.kind = aep_domain::ArtifactKind::Design;
    c.graph.insert(model);
    assert_eq!(reason(required.qualify_record(&r, &c)), "ModelKindMismatch");
    c = context();
    let mut model = c.graph.resolve(&reference).unwrap().clone();
    model.model_digest = Some(aep_domain::SpecDigest::new("a".repeat(64)).unwrap());
    c.graph.insert(model);
    assert_eq!(
        reason(required.qualify_record(&r, &c)),
        "ModelDigestMismatch"
    );
    // Another model with the report's digest does not satisfy the named model expectation.
    let mut other = graph().resolve(&reference).unwrap().clone();
    other.id = "executable-system-specification:other".parse().unwrap();
    c.graph.insert(other);
    assert_eq!(
        reason(required.qualify_record(&r, &c)),
        "ModelDigestMismatch"
    );
    c = context();
    let expected = c.task.constraints.ess_conformance_v2.as_ref().unwrap();
    let suite = aep_domain::ess_conformance_v2::SuiteReference::new(
        "ess-conformance/3".into(),
        expected.suite().digest_profile().into(),
        expected.suite().digest().into(),
    )
    .unwrap();
    c.task.constraints.ess_conformance_v2 = Some(
        EssConformanceV2Expectation::new(
            reference.clone(),
            suite,
            vec![ScenarioId::new(SELECTED).unwrap()],
        )
        .unwrap(),
    );
    assert_eq!(
        reason(required.qualify_record(&r, &c)),
        "SuiteReferenceMismatch"
    );
    c = context();
    let suite = c
        .task
        .constraints
        .ess_conformance_v2
        .as_ref()
        .unwrap()
        .suite()
        .clone();
    c.task.constraints.ess_conformance_v2 = Some(
        EssConformanceV2Expectation::new(
            reference,
            suite,
            vec![ScenarioId::new("demo.core/authored/other").unwrap()],
        )
        .unwrap(),
    );
    assert_eq!(reason(required.qualify_record(&r, &c)), "SelectionMismatch");
    c = context();
    c.now = None;
    assert_eq!(reason(required.qualify_record(&r, &c)), "MissingTime");
    c = context();
    c.now = Some(Timestamp::EPOCH);
    assert_eq!(reason(required.qualify_record(&r, &c)), "FutureObservation");
    let mut wrong = r.clone();
    wrong.subject = Some("service:other".parse().unwrap());
    assert_eq!(
        reason(required.qualify_record(&wrong, &context())),
        "SubjectMismatch"
    );
    wrong = r.clone();
    wrong.producer = Producer::Verifier {
        verifier: Verifier::TestRunner,
    };
    wrong.provenance.tool = Some("conformance-runner".parse().unwrap());
    assert_eq!(
        reason(required.qualify_record(&wrong, &context())),
        "ProducerMismatch"
    );
    wrong.producer = Producer::Agent {
        id: "claims-conformance".into(),
    };
    assert_eq!(
        reason(required.qualify_record(&wrong, &context())),
        "ProducerMismatch"
    );
    wrong = r.clone();
    wrong.observed_at = ObservedAt::new(Timestamp::from_epoch_millis(2));
    assert_eq!(
        reason(required.qualify_record(&wrong, &context())),
        "ObservationMismatch"
    );
    let mut bare: EvidenceRequirement = serde_json::from_str("\"ess_conformance_v2\"").unwrap();
    assert_eq!(
        reason(bare.qualify_record(&r, &context())),
        "UnknownCoverage"
    );
    bare.at_least = 0;
    assert_eq!(
        reason(bare.qualify_record(&r, &context())),
        "InvalidRequirement"
    );
    assert!(!bare.matches(&r));
    let invalid = serde_json::from_str::<EvidenceRequirement>(
        r#"{"kind":"ess_conformance_v2","at_least":0}"#,
    )
    .unwrap_err();
    assert!(invalid.to_string().contains("at_least"));
}

#[test]
fn horizon_boundary_and_independent_constraints_survive_typed_task_readback() {
    let mut required = requirement();
    required.horizon = Some(aep_domain::time::Horizon::days(1).unwrap());
    let r = record(1);
    let mut c = context();
    c.now = Some(Timestamp::from_epoch_millis(86_400_001));
    assert_eq!(reason(required.qualify_record(&r, &c)), "UnknownCoverage");
    c.now = Some(Timestamp::from_epoch_millis(86_400_002));
    assert_eq!(reason(required.qualify_record(&r, &c)), "StaleObservation");
    let original = task();
    assert!(!original.constraints.is_empty());
    let wire = serde_json::to_string(&original).unwrap();
    let restored = aep_schema::parse::task(&wire, None).unwrap();
    assert_eq!(
        restored.constraints.ess_conformance_v2,
        original.constraints.ess_conformance_v2
    );
    let mut raw: serde_json::Value = serde_json::from_str(&wire).unwrap();
    raw["constraints"]["ess_conformance_v2"]["selected_ids"] =
        serde_json::json!([SELECTED, SELECTED]);
    assert!(aep_schema::parse::task(&raw.to_string(), None)
        .unwrap_err()
        .to_string()
        .contains("SelectionOrder"));
    raw["constraints"]["ess_conformance_v2"]["selected_ids"] = serde_json::json!(["not-an-ESS-id"]);
    assert!(aep_schema::parse::task(&raw.to_string(), None)
        .unwrap_err()
        .to_string()
        .contains("MalformedScenarioId"));
    raw["constraints"]["ess_conformance_v2"]["selected_ids"] = serde_json::json!([SELECTED]);
    raw["constraints"]["ess_conformance_v2"]["verified"] = true.into();
    assert!(aep_schema::parse::task(&raw.to_string(), None)
        .unwrap_err()
        .to_string()
        .contains("unknown field"));
    for id in ["aep", "adp-ess-conformance", "future"] {
        let error = aep_schema::parse::protocol(
            &format!("id: {id}\nversion: 2\ntitle: Future\nobservables: ['tests.**']\n"),
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("unsupported_protocol_version"));
    }
}

#[test]
fn real_driver_ingests_original_pair_and_re_admits_it_on_status_and_resume() {
    let directory = scratch("driver-pair");
    let documents = directory.join("documents");
    let put = |path: &str, contents: &str| {
        let path = documents.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    };
    put("protocols/counts/1.yaml", "id: counts\nversion: 1\ntitle: Count fixture\nphases: [verification, completion]\ncapabilities: [tests.execute, command.execute, repository.read, repository.write]\nevidence_kinds: [ess_conformance_v2]\nverifiers: [conformance-runner]\nobservables: ['ess_conformance_v2.**', 'evidence.**', 'state.**', 'workflow.**', 'task.**']\n");
    put("principles/counts.yaml", "id: counts\nversion: 1\ntitle: Counts\nrequires:\n  before_completion:\n    evidence:\n      - kind: ess_conformance_v2\n        independent: true\n        verifier: conformance-runner\nverification:\n  - verifier: conformance-runner\n");
    put("profiles/counts.yaml", "id: counts.test\nversion: 1\ntitle: Counts\nprotocol: counts/1\nworkflow: counts/default\nprinciples: [counts]\ncapabilities:\n  allow: [tests.execute, command.execute, repository.read, repository.write]\ncompletion: [evidence.missing == 0]\n");
    put("workflows/counts/default.yaml", "id: counts/default\nversion: 1\ntitle: Counts\ninitial: verify\nstates:\n  verify:\n    title: Verify\n    phases: [verification]\n  complete:\n    title: Complete\n    terminal: true\n    phases: [completion]\ntransitions:\n  - from: verify\n    to: complete\n    when: evidence.missing == 0\n");
    let task_path = directory.join("task.yaml");
    std::fs::write(&task_path, "id: COUNT-DRIVE\nkind: feature\nobjective: read an original source pair\nprotocol: counts/1\nprofile: counts.test\nsubject: service:demo\n").unwrap();
    let (report, suite) = pair(1);
    let mut entry = serde_json::to_value(adapt_json_v2(&report, &suite).unwrap()).unwrap();
    entry["about"] = "service:demo".into();
    let record_path = directory.join("input.json");
    std::fs::write(&record_path, serde_json::to_string(&vec![entry]).unwrap()).unwrap();
    let map_path = directory.join("steps.yaml");
    std::fs::write(&map_path, format!("format: aep.driver-steps/1\nid: fixture/counts\nworkflow: counts/default/1\nstates:\n  verify:\n    steps:\n      - kind: command\n        run: [/usr/bin/true]\n        evidence:\n          kind: ess_conformance_v2\n          verifier: conformance-runner\n          record: {}\n      - kind: operator\n        prompt: confirm the diagnostic input\n", record_path.display())).unwrap();
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
    let run = directory.join(".engineering/runs/COUNT-DRIVE/1");
    let read = aep_driver::run::RunDirectory::at(run.clone());
    let snapshot = read.read_snapshot().unwrap_or_else(|error| {
        panic!(
            "{error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(snapshot.evidence.len(), 1);
    assert!(
        snapshot.evidence[0].record.facts().is_empty(),
        "disk read is raw until engine restore"
    );
    let Evidence::EssConformanceV2(sources) = &snapshot.evidence[0].record.value else {
        panic!("v2");
    };
    assert_eq!(sources.report_json(), report);
    assert_eq!(sources.suite_json(), suite);
    for verb in ["status", "resume"] {
        let mut args = vec!["drive", verb, "COUNT-DRIVE/1"];
        args.extend(location);
        if verb == "resume" {
            args.push("--pause-on-approval");
        }
        let output = cli(&args);
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !said.contains("MissingReader") && !said.contains("ObservationMismatch"),
            "{said}"
        );
        assert!(said.contains("COUNT-DRIVE/1"), "{said}");
        assert_eq!(read.read_snapshot().unwrap().evidence.len(), 1);
    }
    assert_eq!(
        std::fs::read_dir(directory.join(".engineering/runs/COUNT-DRIVE"))
            .unwrap()
            .count(),
        1
    );
    let no_record = std::fs::read_to_string(&map_path)
        .unwrap()
        .lines()
        .filter(|line| !line.trim_start().starts_with("record:"))
        .collect::<Vec<_>>()
        .join("\n");
    let error = aep_schema::parse::step_map(&no_record, None).unwrap_err();
    assert!(error
        .to_string()
        .contains("cannot establish `ess_conformance_v2`"));
}

#[test]
fn envelope_forgery_alias_duplicates_and_rounded_times_cannot_admit() {
    let (report, suite) = pair(9_007_199_254_740_993);
    let adapted = adapt_json_v2(&report, &suite).unwrap();
    let entry = serde_json::to_value(adapted).unwrap();
    for key in ["verified", "admitted", "reading", "counts", "extra"] {
        let mut bad = entry.clone();
        bad[key] = true.into();
        let error = aep_schema::parse::evidence_list_with_reader(
            &serde_json::to_string(&vec![bad]).unwrap(),
            None,
            &CountStageReader,
        )
        .unwrap_err();
        assert!(error.to_string().contains("UnknownField"), "{error}");
    }
    let mut alias = entry.clone();
    alias["about"] = "service:demo".into();
    alias["envelope_subject"] = "service:demo".into();
    let error = aep_schema::parse::evidence_list_with_reader(
        &serde_json::to_string(&vec![alias]).unwrap(),
        None,
        &CountStageReader,
    )
    .unwrap_err();
    assert!(error.to_string().contains("duplicate field"), "{error}");
    let raw = serde_json::to_string(&vec![entry]).unwrap();
    let rounded = raw.replace(
        "\"observed_at\":9007199254740993",
        "\"observed_at\":9007199254740993.0",
    );
    assert_ne!(raw, rounded);
    let error = aep_schema::parse::evidence_list_with_reader(&rounded, None, &CountStageReader)
        .unwrap_err();
    assert!(error.to_string().contains("ObservationMismatch"), "{error}");
    let mut verification =
        aep_domain::verification::VerificationResult::passed(Verifier::ConformanceRunner);
    verification.evidence.push(record(1));
    let raw: aep_domain::verification::VerificationResult =
        serde_json::from_str(&serde_json::to_string(&verification).unwrap()).unwrap();
    assert!(raw.evidence[0].facts().is_empty());
    assert!(raw.evidence[0].value.spec_digest().is_none());
}

// Keep the concrete command/qualification matrix and its unchanged-state assertions together.
#[allow(clippy::too_many_lines)]
#[test]
fn cli_inspect_evaluate_aliases_and_pair_flag_refusals_use_the_actual_reader() {
    let directory = scratch("cli-reader-routes");
    let (report, suite) = pair(1);
    let report_path = directory.join("report.json");
    let suite_path = directory.join("suite.json");
    std::fs::write(&report_path, &report).unwrap();
    std::fs::write(&suite_path, &suite).unwrap();
    let mut entry = serde_json::to_value(adapt_json_v2(&report, &suite).unwrap()).unwrap();
    entry["about"] = "service:demo".into();
    let evidence = directory.join("evidence.json");
    std::fs::write(
        &evidence,
        serde_json::to_string(&vec![entry.clone()]).unwrap(),
    )
    .unwrap();
    let args = [
        "observe",
        "evidence",
        "inspect",
        evidence.to_str().unwrap(),
        "--format",
        "json",
    ];
    let observed = cli(&args);
    success(&observed);
    let alias = std::process::Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert_eq!(alias.status.code(), observed.status.code());
    assert_eq!(alias.stdout, observed.stdout);
    assert_eq!(alias.stderr, observed.stderr);
    let inspected: serde_json::Value = serde_json::from_slice(&observed.stdout).unwrap();
    assert_eq!(inspected[0]["completed_at"], "1");
    let task_path = directory.join("task.yaml");
    std::fs::write(&task_path, serde_yaml::to_string(&task()).unwrap()).unwrap();
    let output = cli(&[
        "govern",
        "evaluate",
        "--root",
        root().to_str().unwrap(),
        "--task",
        task_path.to_str().unwrap(),
        "--evidence",
        evidence.to_str().unwrap(),
    ]);
    success(&output);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("MissingReader"));
    let absent = directory.join("absent-store");
    let output = cli(&[
        "plan",
        "artifact",
        "evidence",
        "story:counts",
        "--suite",
        suite_path.to_str().unwrap(),
        "--store",
        absent.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--from"));
    assert!(!absent.exists());
    let v1 = root().join("crates/edge/aep-cli/tests/fixtures/conformance-reports/passed.json");
    let output = cli(&[
        "plan",
        "artifact",
        "evidence",
        "story:counts",
        "--from",
        v1.to_str().unwrap(),
        "--suite",
        suite_path.to_str().unwrap(),
        "--store",
        absent.to_str().unwrap(),
    ]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("UnsupportedPairing"));
    assert!(!absent.exists());
    for (flag, value) in [
        ("--kind", "ess_conformance_v2"),
        ("--source", "manual"),
        ("--at", "1"),
        ("--review", "review-result:manual"),
        ("--outcome", "fixed"),
    ] {
        let output = cli(&[
            "plan",
            "artifact",
            "evidence",
            "story:counts",
            "--from",
            report_path.to_str().unwrap(),
            "--suite",
            suite_path.to_str().unwrap(),
            flag,
            value,
            "--store",
            absent.to_str().unwrap(),
        ]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be used with"));
        assert!(!absent.exists());
    }
    entry["observed_at"] = 2.into();
    std::fs::write(&evidence, serde_json::to_string(&vec![entry]).unwrap()).unwrap();
    let output = cli(&args);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ObservationMismatch"));
}

#[test]
fn cross_record_fact_overwrites_and_event_claims_never_repair_qualification() {
    let engine = engine(10, true);
    let mut execution = engine.initialize_with_artifacts(task(), graph()).unwrap();
    let (report, suite) = pair(1);
    let mut failed: serde_json::Value = serde_json::from_str(&report).unwrap();
    failed["counts"]["passed"] = 0.into();
    failed["counts"]["failed"] = 1.into();
    failed["outcomes"]["passed"] = serde_json::json!([]);
    failed["outcomes"]["failed"] = serde_json::json!([SELECTED]);
    failed["execution_status"] = "failed".into();
    failed["conformance_status"] = "failed".into();
    let mut first = record(1);
    first.value = adapt_json_v2(&failed.to_string(), &suite)
        .unwrap()
        .evidence()
        .clone();
    engine
        .submit_evidence(&mut execution, submission(&first))
        .unwrap();
    let mut second = record(1);
    second.id = "other-record".parse().unwrap();
    second.subject = Some("service:other".parse().unwrap());
    // Direct archival recording is permitted; every requirement must still qualify each record.
    execution.record_evidence(second).unwrap();
    assert_eq!(
        execution
            .facts()
            .fact(&FactPath::new("ess_conformance_v2.execution_status").unwrap()),
        Some(aep_domain::FactValue::text("passed"))
    );
    assert_eq!(
        reason(requirement().qualify_record(&execution.evidence()[0], &execution)),
        "ExecutionFailed"
    );
    assert_eq!(
        reason(requirement().qualify_record(&execution.evidence()[1], &execution)),
        "SubjectMismatch"
    );
    assert_ne!(
        execution
            .facts()
            .fact(&FactPath::new("evidence.missing").unwrap()),
        Some(aep_domain::FactValue::count(0))
    );
    assert!(engine
        .explain_completion(&execution)
        .outstanding()
        .any(|item| item
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("ExecutionFailed"))));
    let mut event_only = execution.snapshot();
    event_only.evidence.clear();
    let restored = engine.restore(task(), graph(), event_only).unwrap();
    assert!(restored.evidence().is_empty());
    assert_eq!(
        restored
            .facts()
            .fact(&FactPath::new("ess_conformance_v2.execution_status").unwrap()),
        None
    );
    assert!(restored.events().iter().any(|event| matches!(
        event.event,
        aep_domain::event::ProtocolEvent::EvidenceProduced { .. }
    )));
}

#[derive(Debug)]
struct CurrentReader {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    refuse: bool,
}
impl aep_domain::ess_conformance_v2::EssConformanceV2Reader for CurrentReader {
    fn read(
        &self,
        sources: &EssConformanceV2Sources,
    ) -> Result<
        aep_domain::ess_conformance_v2::EssConformanceV2Reading,
        aep_domain::ess_conformance_v2::EssAdmissionError,
    > {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.refuse {
            return Err(aep_domain::ess_conformance_v2::EssAdmissionError::new(
                "CurrentReaderRefused",
                "$",
                "current integration policy refuses this pair",
            ));
        }
        aep_domain::ess_conformance_v2::EssConformanceV2Reader::read(&CountStageReader, sources)
    }
}

#[test]
fn current_reader_rechecks_cached_sources_on_submission_direct_record_and_restore() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = Arc::new(AtomicUsize::new(0));
    let reader = Arc::new(CurrentReader {
        calls: calls.clone(),
        refuse: false,
    });
    let engine = engine(10, false).with_ess_conformance_v2_reader(reader.clone());
    let mut execution = engine.initialize_with_artifacts(task(), graph()).unwrap();
    let cached = record(1);
    engine
        .submit_evidence(&mut execution, submission(&cached))
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    execution.record_evidence(cached.clone()).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let snapshot = execution.snapshot();
    let restored = engine.restore(task(), graph(), snapshot.clone()).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert_eq!(
        serde_json::to_vec(&restored.snapshot()).unwrap(),
        serde_json::to_vec(&snapshot).unwrap()
    );
    let refused_reader = Arc::new(CurrentReader {
        calls: calls.clone(),
        refuse: true,
    });
    let refused = aep_engine::execution::Execution::restore_with_ess_reader(
        execution.plan().clone(),
        graph(),
        snapshot,
        refused_reader,
        Timestamp::from_epoch_millis(10),
    )
    .unwrap_err();
    assert!(refused.to_string().contains("CurrentReaderRefused"));
    assert_eq!(calls.load(Ordering::SeqCst), 5);
    let before = serde_json::to_vec(&execution.snapshot()).unwrap();
    let refused = execution.record_evidence(record(11)).unwrap_err();
    assert!(refused.to_string().contains("FutureObservation"));
    assert_eq!(serde_json::to_vec(&execution.snapshot()).unwrap(), before);
}

#[test]
fn raw_wrong_kind_empty_and_terminal_records_keep_distinct_qualification_reasons() {
    let required = requirement();
    let mut r = record(1);
    r.value = serde_json::from_str(r#"{"kind":"source_diff"}"#).unwrap();
    assert_eq!(reason(required.qualify_record(&r, &context())), "WrongKind");
    r.value = serde_json::from_str(&serde_json::to_string(&admitted(1)).unwrap()).unwrap();
    assert_eq!(
        reason(required.qualify_record(&r, &context())),
        "MissingAdmission"
    );
    r = record(1);
    r.observed_at = ObservedAt::on_day(aep_domain::time::CivilDate::parse("1970-01-01").unwrap());
    assert_eq!(
        reason(required.qualify_record(&r, &context())),
        "InstantRequired"
    );
    for (category, execution, expected) in [
        ("failed", "failed", "ExecutionFailed"),
        ("error", "inconclusive", "InconclusiveExecution"),
        ("passed", "passed", "UnknownCoverage"),
    ] {
        let (report, suite) = pair(1);
        let mut report: serde_json::Value = serde_json::from_str(&report).unwrap();
        report["counts"]["passed"] = 0.into();
        report["outcomes"]["passed"] = serde_json::json!([]);
        report["counts"][category] = 1.into();
        report["outcomes"][category] = serde_json::json!([SELECTED]);
        report["execution_status"] = execution.into();
        report["conformance_status"] = if execution == "failed" {
            "failed"
        } else {
            "inconclusive"
        }
        .into();
        r = record(1);
        r.value = adapt_json_v2(&report.to_string(), &suite)
            .unwrap()
            .evidence()
            .clone();
        assert_eq!(reason(required.qualify_record(&r, &context())), expected);
    }
    let (report, suite) = pair(1);
    let mut report: serde_json::Value = serde_json::from_str(&report).unwrap();
    let mut suite: serde_json::Value = serde_json::from_str(&suite).unwrap();
    suite["scenarios"] = serde_json::json!({});
    let suite = suite.to_string();
    let hash =
        Sha256::digest(suite.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            });
    report["suite"]["digest"] = hash.into();
    report["counts"]["total"] = 0.into();
    report["counts"]["passed"] = 0.into();
    report["outcomes"]["passed"] = serde_json::json!([]);
    r = record(1);
    r.value = adapt_json_v2(&report.to_string(), &suite)
        .unwrap()
        .evidence()
        .clone();
    let Evidence::EssConformanceV2(sources) = &r.value else {
        panic!("v2");
    };
    let mut c = context();
    c.task.constraints.ess_conformance_v2 = Some(
        EssConformanceV2Expectation::new(
            "executable-system-specification:demo".parse().unwrap(),
            sources.reading().unwrap().data().suite.clone(),
            vec![],
        )
        .unwrap(),
    );
    assert_eq!(reason(required.qualify_record(&r, &c)), "EmptySelection");
}
