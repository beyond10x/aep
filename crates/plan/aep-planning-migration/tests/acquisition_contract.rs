//! Public acquisition regressions for physical legacy planning stores.

use std::path::Path;

use aep_contract::migration::{DigestV1, HexBytesV1, HostPathV1, StoreFieldV1};
use aep_planning_migration::{capture_sqlite, SelectorBinding};

#[cfg(unix)]
fn host_path(path: &Path) -> HostPathV1 {
    use std::os::unix::ffi::OsStrExt as _;
    HostPathV1::Unix(HexBytesV1::new(path.as_os_str().as_bytes().to_vec()))
}

#[cfg(windows)]
fn host_path(path: &Path) -> HostPathV1 {
    use std::os::windows::ffi::OsStrExt as _;
    HostPathV1::Windows(path.as_os_str().encode_wide().collect())
}

#[test]
fn sqlite_capture_refuses_matching_names_with_malformed_schema_predicates() {
    let path = std::env::temp_dir().join(format!(
        "aep-planning-malformed-catalog-{}.sqlite3",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let connection = rusqlite::Connection::open(&path).expect("fixture opens");
    connection
        .execute_batch(
            "CREATE TABLE events(entity TEXT,id TEXT,revision INTEGER,position INTEGER,document TEXT);\
             CREATE TABLE history(entity TEXT,id TEXT,position INTEGER,kind TEXT,record_id TEXT,document TEXT);\
             CREATE TABLE instances(entity TEXT,id TEXT,revision INTEGER,document TEXT);\
             CREATE TABLE legacy_origins(entity TEXT,id TEXT,revision INTEGER);",
        )
        .expect("malformed catalog is physically readable");
    drop(connection);

    let selector = SelectorBinding {
        project_root: host_path(Path::new(".")),
        project_file: host_path(Path::new(".engineering/project.yaml")),
        selector_digest: DigestV1::from_bytes([1; 32]),
        store_field: StoreFieldV1::Present,
        config_digest: DigestV1::from_bytes([2; 32]),
    };
    capture_sqlite(&path, selector).expect_err(
        "column names alone cannot qualify a catalog missing its required keys and checks",
    );
    let _ = std::fs::remove_file(path);
}

struct RefusalFixture(std::path::PathBuf);

impl RefusalFixture {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "aep-retained-catalog-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir(&path).expect("fresh fixture directory");
        let connection = rusqlite::Connection::open(path.join("source.sqlite3")).unwrap();
        connection.execute_batch(
            "CREATE TABLE events(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,position INTEGER NOT NULL,document TEXT NOT NULL,PRIMARY KEY(entity,id,revision,position));
             CREATE TABLE history(entity TEXT NOT NULL,id TEXT NOT NULL,position INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE instances(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,document TEXT NOT NULL,PRIMARY KEY(entity,id));
             CREATE TABLE legacy_origins(entity TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,PRIMARY KEY(entity,id));"
        ).unwrap();
        std::fs::create_dir_all(path.join("local/story")).unwrap();
        std::fs::write(
            path.join("local/story/retained.md"),
            b"retained before SQL failure",
        )
        .unwrap();
        Self(path)
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

    fn provider() -> Self {
        let fixture = Self::new();
        let connection = rusqlite::Connection::open(fixture.0.join("source.sqlite3")).unwrap();
        connection.execute_batch(
            "DROP TABLE history;
             CREATE TABLE history(entity TEXT NOT NULL,id TEXT NOT NULL,position INTEGER NOT NULL,
                kind TEXT NOT NULL CHECK(kind IN ('decision','observation')),record_id TEXT NOT NULL UNIQUE,
                document TEXT NOT NULL,PRIMARY KEY(entity,id,position));
             INSERT INTO instances VALUES('aep.entity','a',0,'{}');
             INSERT INTO events VALUES('aep.entity','a',0,0,'{}');
             INSERT INTO history VALUES('aep.entity','a',0,'decision','first','{}');
             INSERT INTO legacy_origins VALUES('aep.entity','a',0);"
        ).unwrap();
        fixture
    }
}

impl Drop for RefusalFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn sqlite_schema_refusal_retains_prior_table_and_exact_failing_ddl_without_reading_rows() {
    use aep_contract::migration::*;
    let fixture = RefusalFixture::new();
    let database = fixture.0.join("source.sqlite3");
    let before = std::fs::read(&database).unwrap();
    let error =
        capture_sqlite(&database, fixture.selector()).expect_err("unsupported default refuses");
    let aep_planning_migration::AcquisitionError::RefusedObservation(observation) = error else {
        panic!("catalog evidence was discarded: {error}");
    };
    observation
        .validate()
        .expect("retained failure obeys the frozen capture contract");
    let ObservationOutcomeV1::Refused(refused) = &observation.observation else {
        panic!("not refused");
    };
    let PhaseResultV1::Refused(failure) = &refused.phases[0].result else {
        panic!("no failed phase");
    };
    assert_eq!(
        failure.refusals,
        vec![CaptureRefusalV1 {
            code: CaptureRefusalCodeV1::UnsupportedSchema,
            at: PhysicalCoordinateV1::SqlCatalog(SqlCatalogCoordinateV1 {
                namespace: PresenceV1::Present("main".into()),
                family: CatalogFamilyV1::TableDefinition,
                table: PresenceV1::Present("history".into()),
                row: PresenceV1::Present(0),
            }),
        }]
    );
    let PhaseEvidenceV1::Sqlite(sql) = &failure.evidence else {
        panic!("wrong dialect");
    };
    assert_eq!(
        sql.catalog.tables.len(),
        4,
        "all discovered tables retained"
    );
    assert!(matches!(
        sql.catalog.tables[0].checks.terminal,
        EnumerationTerminalV1::Complete
    ));
    let ObservedValueV1::Complete(PresenceV1::Present(ddl)) =
        &sql.catalog.tables[1].catalog_definition
    else {
        panic!("failing DDL lost");
    };
    assert_eq!(ddl.as_bytes(), b"CREATE TABLE history(entity TEXT NOT NULL,id TEXT NOT NULL,position INTEGER NOT NULL DEFAULT 0)");
    assert!(matches!(
        sql.catalog.tables[1].columns.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert!(matches!(
        sql.catalog.tables[2].catalog_definition,
        ObservedValueV1::NotAttempted
    ));
    assert!(matches!(
        sql.instances.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert!(matches!(
        sql.events.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert!(matches!(
        sql.history.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert_eq!(
        std::fs::read(database).unwrap(),
        before,
        "capture never writes the source"
    );
}

#[test]
fn hybrid_catalog_refusal_keeps_before_observations_and_stops_the_bracket() {
    use aep_contract::migration::*;
    let fixture = RefusalFixture::new();
    let error = aep_planning_migration::capture_hybrid_sqlite(
        &fixture.0.join("local"),
        &fixture.0.join("source.sqlite3"),
        &fixture.0.join("divergences"),
        HybridPolicyWordsV1 {
            authority: "local".into(),
            read: "local-first".into(),
            on_unreachable: "refuse".into(),
            on_divergence: "refuse".into(),
        },
        fixture.selector(),
    )
    .expect_err("unsupported replica schema refuses the entire hybrid capture");
    let aep_planning_migration::AcquisitionError::RefusedObservation(observation) = error else {
        panic!("hybrid prefix lost: {error}");
    };
    observation
        .validate()
        .expect("retained hybrid obeys frozen phase ordering");
    let ObservationOutcomeV1::Refused(refused) = observation.observation else {
        panic!("not refused");
    };
    assert_eq!(refused.phases.len(), 5);
    let PhaseResultV1::Complete(first) = &refused.phases[0].result else {
        panic!("local prefix lost");
    };
    let PhaseEvidenceV1::Markdown(local) = &first.evidence else {
        panic!("wrong evidence");
    };
    assert!(local.nodes.items.iter().any(|node| matches!(node,
        MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 { node: MarkdownNodeKindV1::Regular(file), .. })
            if file.bytes.as_bytes() == b"retained before SQL failure"
    )));
    assert!(matches!(
        refused.phases[1].result,
        PhaseResultV1::Complete(_)
    ));
    assert!(matches!(
        refused.phases[2].result,
        PhaseResultV1::Refused(_)
    ));
    assert!(matches!(
        refused.phases[3].result,
        PhaseResultV1::NotAttempted
    ));
    assert!(matches!(
        refused.phases[4].result,
        PhaseResultV1::NotAttempted
    ));
}

#[test]
fn sqlite_row_refusals_retain_each_family_prefix_and_exact_cells_before_stopping() {
    use aep_contract::migration::*;
    let cases = [
        ("instances", "INSERT INTO instances VALUES('aep.entity','z',-1,'{}')", "revision", SqlCellValueV1::Integer(-1), CaptureRefusalCodeV1::NumericOutOfRange),
        ("events", "INSERT INTO events VALUES('aep.entity','z',0,-1,'{}')", "position", SqlCellValueV1::Integer(-1), CaptureRefusalCodeV1::NumericOutOfRange),
        ("history", "PRAGMA ignore_check_constraints=ON; INSERT INTO history VALUES('aep.entity','z',0,'other','bad','{}')", "kind", SqlCellValueV1::Text(HexBytesV1::new(b"other".to_vec())), CaptureRefusalCodeV1::UnknownHistoryKind),
        ("legacy_origins", "INSERT INTO legacy_origins VALUES('aep.entity','z',-1)", "revision", SqlCellValueV1::Integer(-1), CaptureRefusalCodeV1::NumericOutOfRange),
        ("instances", "INSERT INTO instances VALUES('aep.entity','z',0,CAST(X'ff' AS TEXT))", "document", SqlCellValueV1::Text(HexBytesV1::new(vec![255])), CaptureRefusalCodeV1::NonUtf8Text),
        ("instances", "INSERT INTO instances VALUES('aep.entity','z',0,X'0102')", "document", SqlCellValueV1::Blob(HexBytesV1::new(vec![1,2])), CaptureRefusalCodeV1::SqlTypeMismatch),
    ];
    for (table, mutation, column, value, code) in cases {
        let fixture = RefusalFixture::provider();
        let database = fixture.0.join("source.sqlite3");
        let connection = rusqlite::Connection::open(&database).unwrap();
        connection.execute_batch(mutation).unwrap();
        let rowid: i64 = connection
            .query_row(
                &format!("SELECT rowid FROM {table} WHERE id='z'"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        let before = std::fs::read(&database).unwrap();
        let error = capture_sqlite(&database, fixture.selector())
            .expect_err("invalid physical row must refuse");
        let aep_planning_migration::AcquisitionError::RefusedObservation(observation) = error
        else {
            panic!("row evidence lost: {error}");
        };
        observation
            .validate()
            .expect("retained row failure satisfies frozen capture contract");
        let ObservationOutcomeV1::Refused(refused) = &observation.observation else {
            panic!("not refused");
        };
        let PhaseResultV1::Refused(failure) = &refused.phases[0].result else {
            panic!("no failed phase");
        };
        let coordinate = PhysicalCoordinateV1::SqlPhysicalRow(SqlPhysicalRowCoordinateV1 {
            table: table.into(),
            locator: SqlPhysicalLocatorV1::SqliteRowId(rowid),
        });
        assert_eq!(
            failure.refusals,
            vec![CaptureRefusalV1 {
                code,
                at: coordinate.clone()
            }]
        );
        let PhaseEvidenceV1::Sqlite(sql) = &failure.evidence else {
            panic!("wrong evidence");
        };
        assert_eq!(sql.rejected_rows.len(), 1);
        assert_eq!(sql.rejected_rows[0].at, coordinate);
        assert!(
            sql.rejected_rows[0]
                .cells
                .iter()
                .any(|cell| cell.column == column && cell.value == value),
            "exact invalid {column} bytes lost"
        );
        let families = [
            (
                "instances",
                sql.instances.items.len(),
                &sql.instances.terminal,
            ),
            ("events", sql.events.items.len(), &sql.events.terminal),
            ("history", sql.history.items.len(), &sql.history.terminal),
            (
                "legacy_origins",
                sql.legacy_origins.items.len(),
                &sql.legacy_origins.terminal,
            ),
        ];
        let mut stopped = false;
        for (name, count, terminal) in families {
            if stopped {
                assert_eq!(count, 0);
                assert!(matches!(terminal, EnumerationTerminalV1::NotAttempted));
            } else if name == table {
                assert_eq!(count, 1);
                assert!(matches!(terminal, EnumerationTerminalV1::Refused(_)));
                stopped = true;
            } else {
                assert_eq!(count, 1);
                assert!(matches!(terminal, EnumerationTerminalV1::Complete));
            }
        }
        assert_eq!(
            std::fs::read(database).unwrap(),
            before,
            "refused capture changed its source"
        );
    }
}
