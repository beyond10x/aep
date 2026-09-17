use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(std::path::PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "aep-local-capture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir_all(path.join("local/story")).unwrap();
        fs::write(path.join("local/story/zz.md"), b"before short").unwrap();
        fs::write(path.join("local/story/a_long_name.md"), b"before long").unwrap();
        Self(path)
    }

    fn root(&self) -> std::path::PathBuf {
        self.0.join("local")
    }

    fn selector(&self) -> SelectorBinding {
        SelectorBinding {
            project_root: host_path(&self.0),
            project_file: host_path(&self.0.join("project.yaml")),
            selector_digest: DigestV1::from_bytes([1; 32]),
            store_field: StoreFieldV1::Present,
            config_digest: DigestV1::from_bytes([2; 32]),
        }
    }

    fn sqlite(&self) -> std::path::PathBuf {
        let path = self.0.join("source.sqlite3");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TABLE instances(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,document TEXT NOT NULL,PRIMARY KEY(entity,id));
            CREATE TABLE events(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,position INTEGER NOT NULL,document TEXT NOT NULL,PRIMARY KEY(entity,id,revision,position));
            CREATE TABLE history(entity TEXT NOT NULL,id TEXT NOT NULL,position INTEGER NOT NULL,kind TEXT NOT NULL CHECK(kind IN ('decision','observation')),record_id TEXT NOT NULL UNIQUE,document TEXT NOT NULL,PRIMARY KEY(entity,id,position));
            CREATE TABLE legacy_origins(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,PRIMARY KEY(entity,id));").unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn policy() -> HybridPolicyWordsV1 {
    HybridPolicyWordsV1 {
        authority: "local".into(),
        read: "local-first".into(),
        on_unreachable: "refuse".into(),
        on_divergence: "refuse".into(),
    }
}

fn refused(error: AcquisitionError) -> RawCaptureObservationV1 {
    let AcquisitionError::RefusedObservation(observation) = error else {
        panic!("retained observation lost: {error}");
    };
    observation
        .validate()
        .expect("refusal obeys the original capture contract");
    *observation
}

fn phases(observation: &RawCaptureObservationV1) -> &[PhaseObservationV1] {
    let ObservationOutcomeV1::Refused(value) = &observation.observation else {
        panic!("not refused");
    };
    &value.phases
}

#[test]
fn unresolved_postgres_capture_keeps_the_actual_earlier_hybrid_observations() {
    let fixture = Fixture::new();
    let observation = refused(
        capture_hybrid_postgres(
            &fixture.root(),
            "not-a-connection-string",
            &fixture.0.join("absent-divergences.json"),
            policy(),
            fixture.selector(),
        )
        .expect_err("unresolved PostgreSQL replica"),
    );
    assert!(matches!(observation.source, PresenceV1::Missing));
    let observed = phases(&observation);
    assert!(matches!(observed[0].result, PhaseResultV1::Complete(_)));
    assert!(matches!(observed[1].result, PhaseResultV1::Complete(_)));
    let PhaseResultV1::Refused(failure) = &observed[2].result else {
        panic!("replica failure must retain its own evidence");
    };
    assert_eq!(
        failure.refusals[0].code,
        CaptureRefusalCodeV1::UnresolvedSourceCoordinate
    );
    assert!(matches!(observed[3].result, PhaseResultV1::NotAttempted));
    assert!(matches!(observed[4].result, PhaseResultV1::NotAttempted));
}

#[test]
fn a_failed_first_scan_never_attempts_the_second() {
    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.root()).unwrap();
    let mut calls = 0;
    let observation = refused(
        markdown_scans(&host_path(&fixture.root()), fixture.selector(), || {
            calls += 1;
            local_capture::scan(&fixture.root(), &host_path(&fixture.root()))
        })
        .expect_err("missing root refuses"),
    );
    assert_eq!(calls, 1);
    assert!(matches!(
        phases(&observation)[0].result,
        PhaseResultV1::Refused(_)
    ));
    assert!(matches!(
        phases(&observation)[1].result,
        PhaseResultV1::NotAttempted
    ));
}

#[test]
fn a_failed_second_scan_retains_the_complete_first_scan() {
    let fixture = Fixture::new();
    let mut calls = 0;
    let observation = refused(
        markdown_scans(&host_path(&fixture.root()), fixture.selector(), || {
            calls += 1;
            if calls == 2 {
                fs::remove_dir_all(fixture.root()).unwrap();
            }
            local_capture::scan(&fixture.root(), &host_path(&fixture.root()))
        })
        .expect_err("second scan fails"),
    );
    assert_eq!(calls, 2);
    let PhaseResultV1::Complete(first) = &phases(&observation)[0].result else {
        panic!("first scan lost");
    };
    let PhaseEvidenceV1::Markdown(markdown) = &first.evidence else {
        panic!("wrong evidence");
    };
    assert!(markdown.nodes.items.iter().any(
        |node| matches!(node, MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 {
        node: MarkdownNodeKindV1::Regular(value), ..
    }) if value.bytes.as_bytes() == b"before long")
    ));
    assert!(matches!(
        phases(&observation)[1].result,
        PhaseResultV1::Refused(_)
    ));
}

#[test]
fn unstable_markdown_reports_the_exact_canonical_changed_paths() {
    let fixture = Fixture::new();
    let mut calls = 0;
    let observation = markdown_scans(&host_path(&fixture.root()), fixture.selector(), || {
        calls += 1;
        if calls == 2 {
            fs::write(fixture.root().join("story/zz.md"), b"after short").unwrap();
            fs::remove_file(fixture.root().join("story/a_long_name.md")).unwrap();
            fs::write(fixture.root().join("story/b.md"), b"added").unwrap();
        }
        local_capture::scan(&fixture.root(), &host_path(&fixture.root()))
    })
    .expect("changed source is an unstable observation, not invalid output");
    observation.validate().unwrap();
    let ObservationOutcomeV1::Unstable(value) = observation.observation else {
        panic!("changes were missed");
    };
    let mut expected = ["zz.md", "a_long_name.md", "b.md"]
        .map(|name| {
            PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                relative: host_path(&Path::new("story").join(name)),
            })
        })
        .to_vec();
    expected.sort_by_key(sort_key_v1);
    assert_eq!(value.changed, expected);
    assert_eq!(value.method, ObservationMethodV1::MarkdownDoubleScan);
}

#[test]
fn an_early_hybrid_failure_keeps_local_evidence_without_resolving_postgres() {
    let fixture = Fixture::new();
    let divergence = fixture.0.join("divergence-directory");
    fs::create_dir(&divergence).unwrap();
    let observation = refused(
        capture_hybrid(
            &fixture.root(),
            &divergence,
            policy(),
            fixture.selector(),
            &PresenceV1::Missing,
            || panic!("a failed local phase must not invoke PostgreSQL discovery"),
        )
        .expect_err("reading a directory as divergence bytes fails"),
    );
    assert_eq!(observation.source, PresenceV1::Missing);
    assert!(matches!(
        phases(&observation)[0].result,
        PhaseResultV1::Complete(_)
    ));
    assert!(matches!(
        phases(&observation)[1].result,
        PhaseResultV1::Refused(_)
    ));
    assert!(phases(&observation)[2..]
        .iter()
        .all(|phase| matches!(phase.result, PhaseResultV1::NotAttempted)));
}

#[test]
fn a_late_hybrid_failure_retains_sql_and_cannot_hide_its_resolved_source() {
    let fixture = Fixture::new();
    let database = fixture.sqlite();
    let replica = SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
        database: host_path(&database),
    });
    let mut observation = refused(
        capture_hybrid(
            &fixture.root(),
            &fixture.0.join("divergences"),
            policy(),
            fixture.selector(),
            &PresenceV1::Present(replica.clone()),
            || {
                let raw = sqlite_raw(&database)?;
                fs::remove_dir_all(fixture.root()).unwrap();
                Ok((raw, replica))
            },
        )
        .expect_err("local-after scan must fail"),
    );
    assert!(phases(&observation)[..3]
        .iter()
        .all(|phase| matches!(phase.result, PhaseResultV1::Complete(_))));
    assert!(matches!(
        phases(&observation)[3].result,
        PhaseResultV1::Refused(_)
    ));
    assert!(matches!(
        phases(&observation)[4].result,
        PhaseResultV1::NotAttempted
    ));
    observation.source = PresenceV1::Missing;
    assert!(observation
        .validate()
        .expect_err("an already observed replica cannot be omitted")
        .contains(CaptureValidationCodeV1::MissingCoordinate));
}

#[cfg(unix)]
#[test]
fn invalid_path_units_survive_refusal_and_require_the_matching_diagnostic() {
    use std::os::unix::ffi::OsStringExt as _;
    let fixture = Fixture::new();
    let filename = std::ffi::OsString::from_vec(vec![255, b'.', b'm', b'd']);
    fs::write(
        fixture.root().join("story").join(filename),
        b"invalid path content",
    )
    .unwrap();
    let mut observation = refused(
        capture_markdown(
            &fixture.root(),
            host_path(&fixture.root()),
            fixture.selector(),
        )
        .expect_err("invalid host bytes refuse"),
    );
    let ObservationOutcomeV1::Refused(value) = &mut observation.observation else {
        unreachable!();
    };
    let PhaseResultV1::Refused(failure) = &mut value.phases[0].result else {
        unreachable!();
    };
    let at = PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
        relative: HostPathV1::Unix(HexBytesV1::new(b"story/\xff.md".to_vec())),
    });
    assert!(failure
        .refusals
        .iter()
        .any(|value| value.at == at && value.code == CaptureRefusalCodeV1::InvalidRelativePath));
    failure.refusals.clear();
    assert!(
        observation.validate().is_err(),
        "invalid path bytes require an explicit refusal"
    );
}

#[cfg(unix)]
#[test]
fn symlink_refusal_retains_the_link_without_reading_its_target() {
    let fixture = Fixture::new();
    let target = fixture.0.join("outside");
    fs::write(&target, b"outside target is not capture content").unwrap();
    std::os::unix::fs::symlink(&target, fixture.root().join("story/link.md")).unwrap();
    let observation = refused(
        capture_markdown(
            &fixture.root(),
            host_path(&fixture.root()),
            fixture.selector(),
        )
        .expect_err("symlink is a foreign node"),
    );
    let PhaseResultV1::Refused(failure) = &phases(&observation)[0].result else {
        unreachable!();
    };
    let PhaseEvidenceV1::Markdown(markdown) = &failure.evidence else {
        unreachable!();
    };
    assert!(markdown.nodes.items.iter().any(|node| matches!(node, MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 { node: MarkdownNodeKindV1::Symlink(value), .. }) if value.target == host_path(&target))));
    assert!(!markdown.nodes.items.iter().any(|node| matches!(node, MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 { node: MarkdownNodeKindV1::Regular(value), .. }) if value.bytes.as_bytes() == b"outside target is not capture content")));
}
