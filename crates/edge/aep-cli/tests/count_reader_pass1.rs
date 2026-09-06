//! First bounded reader review through typed envelopes, execution and actual CLI.
#[path = "../../../observe/aep-ess-evidence/tests/support/count_reader_pass1.rs"]
mod support;

use aep_domain::ess_conformance_v2::{
    EssConformanceV2Expectation, EssConformanceV2Sources, ScenarioId,
};
use aep_domain::evidence::{Evidence, EvidenceRecord, Producer, Provenance};
use aep_domain::requirement::{EvidenceRequirement, RecordQualification, RequirementContext};
use aep_domain::time::{Horizon, ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use aep_engine::{Engine, FixedClock};
use aep_ess_evidence::{adapt_json_v2, CountStageReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn sources(time: u64) -> Evidence {
    let suite = support::suite();
    adapt_json_v2(&support::report(&suite, time), &suite)
        .unwrap()
        .evidence()
        .clone()
}

fn record(time: u64) -> EvidenceRecord {
    EvidenceRecord {
        id: "review-count-record".parse().unwrap(),
        observed_at: ObservedAt::new(Timestamp::from_epoch_millis(time)),
        produced_at: Timestamp::from_epoch_millis(time),
        producer: Producer::Verifier {
            verifier: Verifier::ConformanceRunner,
        },
        subject: Some("service:review".parse().unwrap()),
        value: sources(time),
        provenance: Provenance::default(),
    }
}

fn task() -> aep_domain::Task {
    let mut task = aep_schema::parse::task("id: REVIEW-1\nkind: feature\nobjective: review original count pairs\nprotocol: adp-ess-conformance/1\nprofile: development.ess-conformance-v2\nsubject: service:review\n", None).unwrap();
    let Evidence::EssConformanceV2(sources) = sources(1) else {
        panic!("v2");
    };
    task.constraints.ess_conformance_v2 = Some(
        EssConformanceV2Expectation::new(
            "executable-system-specification:review".parse().unwrap(),
            sources.reading().unwrap().data().suite.clone(),
            vec![ScenarioId::new(support::SELECTED).unwrap()],
        )
        .unwrap(),
    );
    task
}

fn graph() -> aep_domain::ArtifactGraph {
    aep_schema::parse::artifact_manifest(&format!("version: aep.artifacts/1\nartifacts:\n  - id: executable-system-specification:review\n    kind: executable-system-specification\n    status: approved\n    model_digest: {}\n    location: {{path: review.yaml}}\n", support::MODEL), None).unwrap()
}

fn engine(time: u64) -> Engine<FixedClock> {
    Engine::with_clock(
        aep_project::load_tree(&root()).unwrap(),
        FixedClock::new(time),
    )
    .with_ess_conformance_v2_reader(Arc::new(CountStageReader))
}

fn reason(result: RecordQualification) -> String {
    match result {
        RecordQualification::Unknown(issue) | RecordQualification::Contradiction(issue) => {
            issue.reason.into()
        }
        RecordQualification::Qualified => panic!("count-stage coverage is unknown"),
    }
}

#[test]
fn typed_batch_preserves_crlf_and_unicode_bytes_and_refuses_either_bad_position() {
    let suite = serde_json::to_string_pretty(
        &serde_json::from_str::<serde_json::Value>(&support::suite()).unwrap(),
    )
    .unwrap()
    .replace('\n', "\r\n")
    .replace("café", "caf\\u00e9");
    let suite = format!("\r\n{suite}\r\n");
    let report = support::report(&suite, u64::MAX);
    let entry = serde_json::to_value(adapt_json_v2(&report, &suite).unwrap()).unwrap();
    for text in [
        serde_json::to_string(&vec![entry.clone()]).unwrap(),
        serde_yaml::to_string(&vec![entry.clone()]).unwrap(),
    ] {
        let parsed =
            aep_schema::parse::evidence_list_with_reader(&text, None, &CountStageReader).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].observed_at.timestamp().epoch_millis(), u64::MAX);
        let Evidence::EssConformanceV2(sources) = &parsed[0].evidence else {
            panic!("v2");
        };
        assert_eq!(sources.report_json(), report);
        assert_eq!(sources.suite_json(), suite);
        assert!(aep_schema::parse::evidence_list(&text, None)
            .unwrap_err()
            .to_string()
            .contains("MissingReader"));
    }
    let mut changed = entry.clone();
    changed["suite_json"] = suite.replace("\r\n", "\n").into();
    for entries in [vec![entry.clone(), changed.clone()], vec![changed, entry]] {
        for text in [
            serde_json::to_string(&entries).unwrap(),
            serde_yaml::to_string(&entries).unwrap(),
        ] {
            let error =
                aep_schema::parse::evidence_list_with_reader(&text, None, &CountStageReader)
                    .unwrap_err();
            assert!(error.to_string().contains("SuiteDigestMismatch"), "{error}");
        }
    }
}

#[test]
fn exact_high_time_horizon_restore_and_changed_source_refusals_preserve_execution() {
    let engine = engine(u64::MAX);
    let mut execution = engine.initialize_with_artifacts(task(), graph()).unwrap();
    let mut requirement: EvidenceRequirement =
        serde_json::from_str("\"ess_conformance_v2\"").unwrap();
    let horizon = Horizon::days(1).unwrap();
    requirement.horizon = Some(horizon);
    for (time, expected) in [
        (u64::MAX, "UnknownCoverage"),
        (u64::MAX - horizon.as_millis(), "UnknownCoverage"),
        (u64::MAX - horizon.as_millis() - 1, "StaleObservation"),
    ] {
        assert_eq!(
            reason(requirement.qualify_record(&record(time), &execution)),
            expected
        );
    }
    execution.record_evidence(record(u64::MAX)).unwrap();
    let before = serde_json::to_vec(&execution.snapshot()).unwrap();
    let before_facts = execution.fact_store().clone();
    let before_time = execution.evaluated_at();
    let mut changed = record(u64::MAX);
    let suite = support::suite();
    changed.value = Evidence::EssConformanceV2(EssConformanceV2Sources::new(
        support::report(&suite, u64::MAX),
        format!("{suite}\n"),
    ));
    let error = execution.record_evidence(changed).unwrap_err();
    assert!(error.to_string().contains("SuiteDigestMismatch"), "{error}");
    assert_eq!(serde_json::to_vec(&execution.snapshot()).unwrap(), before);
    assert_eq!(execution.fact_store(), &before_facts);
    assert_eq!(execution.evaluated_at(), before_time);

    let raw = serde_json::from_slice(&before).unwrap();
    let restored = engine.restore(task(), graph(), raw).unwrap();
    assert_eq!(
        reason(requirement.qualify_record(&restored.evidence()[0], &restored)),
        "UnknownCoverage"
    );
    assert_eq!(serde_json::to_vec(&restored.snapshot()).unwrap(), before);
    let earlier = self::engine(u64::MAX - 1);
    let error = earlier
        .restore(task(), graph(), serde_json::from_slice(&before).unwrap())
        .unwrap_err();
    assert!(error.to_string().contains("FutureObservation"), "{error}");
    assert_eq!(serde_json::to_vec(&execution.snapshot()).unwrap(), before);
}

#[test]
fn actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output() {
    let scratch = root()
        .join("target/ess-conformance-v2-counts/adversary-pass-1")
        .join(format!("inspect-fixture-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let file = scratch.join("evidence.json");
    let suite = support::suite();
    let entry =
        serde_json::to_value(adapt_json_v2(&support::report(&suite, u64::MAX), &suite).unwrap())
            .unwrap();
    std::fs::write(&file, serde_json::to_vec(&vec![entry.clone()]).unwrap()).unwrap();
    let args = [
        "observe",
        "evidence",
        "inspect",
        file.to_str().unwrap(),
        "--format",
        "json",
        "--at",
        "2026-09-06",
    ];
    let run = |binary| {
        std::process::Command::new(binary)
            .args(args)
            .current_dir(root())
            .output()
            .unwrap()
    };
    let output = run(env!("CARGO_BIN_EXE_aep"));
    let alias = run(env!("CARGO_BIN_EXE_protocol"));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(alias.status.code(), output.status.code());
    assert_eq!(alias.stdout, output.stdout);
    assert_eq!(alias.stderr, output.stderr);
    let inspected: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(inspected[0]["completed_at"], u64::MAX.to_string());
    let refusal = String::from_utf8_lossy(&output.stderr);
    assert!(refusal.contains("has not happened yet"), "{refusal}");
    assert!(refusal.contains(&u64::MAX.to_string()), "{refusal}");
    assert!(refusal.contains("record 1"), "{refusal}");
    let mut bad = entry.clone();
    bad["report_json"] = support::report(&suite, u64::MAX)
        .replace("\"total\":1", r#""total":1,"\u0074otal":1"#)
        .into();
    std::fs::write(&file, serde_json::to_vec(&vec![entry, bad]).unwrap()).unwrap();
    let output = run(env!("CARGO_BIN_EXE_aep"));
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "a malformed batch must not print an admitted prefix"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("DuplicateKey"));
}
