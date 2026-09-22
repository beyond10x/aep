//! Physical PostgreSQL acceptance cases. These run only against an explicitly selected fixture.
use super::*;
use postgres::NoTls;

struct Fixture {
    client: postgres::Client,
    config: postgres::Config,
    schema: String,
}

impl Fixture {
    fn selected() -> Option<Self> {
        let Ok(url) = std::env::var("ENTITY_POSTGRES_URL") else {
            eprintln!("PostgreSQL retained acquisition NOT EXECUTED: ENTITY_POSTGRES_URL unset");
            return None;
        };
        let mut config = postgres::Config::from_str(&url).unwrap();
        let client = config
            .connect(NoTls)
            .expect("explicit disposable PostgreSQL fixture");
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let schema = format!("aep_retained_{}_{nonce}", std::process::id());
        config.options(&format!("-csearch_path={schema}"));
        let mut fixture = Self {
            client,
            config,
            schema,
        };
        fixture.reset();
        Some(fixture)
    }

    fn reset(&mut self) {
        self.client
            .batch_execute(&format!(
                "DROP SCHEMA IF EXISTS {} CASCADE; CREATE SCHEMA {}; SET search_path TO {}; {}",
                self.schema, self.schema, self.schema, PROVIDER_DDL,
            ))
            .unwrap();
        self.client
            .batch_execute(
                "INSERT INTO instances VALUES('aep.entity','a',0,'{}');
             INSERT INTO events VALUES('aep.entity','a',0,0,'{}');
             INSERT INTO history VALUES('aep.entity','a',0,'decision','first','{}');
             INSERT INTO legacy_origins VALUES('aep.entity','a',0);
             INSERT INTO provider_sequences VALUES('known',0);",
            )
            .unwrap();
    }

    fn failed_capture(&self) -> RefusedPhaseResultV1 {
        let AcquisitionError::SqlPhase(failure) =
            postgres_snapshot::read_config(&self.config).expect_err("physical source must refuse")
        else {
            panic!("failure lost its obtained evidence");
        };
        let PhaseEvidenceV1::Postgres(sql) = &failure.evidence else {
            panic!("wrong dialect");
        };
        let PresenceV1::Present(SqlReplicaCoordinateV1::Postgres(source)) = &sql.source else {
            panic!("an already resolved endpoint must be retained");
        };
        let source = SourceCoordinateV1::Postgres(PostgresSourceCoordinateV1 {
            endpoint: source.endpoint.clone(),
            endpoint_id: source.endpoint_id,
        });
        let AcquisitionError::RefusedObservation(observation) = retained_failure::observation(
            source,
            selector(),
            ObservationMethodV1::PostgresRepeatableReadOnly,
            vec![PhaseObservationV1 {
                phase: CapturePhaseV1::SqlSnapshot,
                result: PhaseResultV1::Refused((*failure).clone()),
            }],
        ) else {
            unreachable!()
        };
        observation
            .validate()
            .expect("retained physical failure obeys the capture contract");
        *failure
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self
            .client
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE", self.schema));
    }
}

fn selector() -> SelectorBinding {
    SelectorBinding {
        project_root: host_path(Path::new("fixture")),
        project_file: host_path(Path::new("fixture/project.yaml")),
        selector_digest: DigestV1::from_bytes([1; 32]),
        store_field: StoreFieldV1::Present,
        config_digest: DigestV1::from_bytes([2; 32]),
    }
}

#[test]
fn postgres_catalog_failure_retains_prior_keys_and_the_actual_rejected_cells() {
    let Some(mut fixture) = Fixture::selected() else {
        return;
    };
    postgres_snapshot::read_config(&fixture.config).expect("original catalog and rows admitted");
    fixture
        .client
        .batch_execute(
            "ALTER TABLE history DROP CONSTRAINT history_record_id_key;
         ALTER TABLE history ADD CONSTRAINT history_record_id_key UNIQUE(record_id) DEFERRABLE",
        )
        .unwrap();
    let failure = fixture.failed_capture();
    let PhaseEvidenceV1::Postgres(sql) = failure.evidence else {
        unreachable!()
    };
    let events = sql
        .catalog
        .tables
        .iter()
        .find(|table| table.name == "events")
        .unwrap();
    assert!(matches!(
        events.checks.terminal,
        EnumerationTerminalV1::Complete
    ));
    let history = sql
        .catalog
        .tables
        .iter()
        .find(|table| table.name == "history")
        .unwrap();
    assert!(matches!(
        history.primary_key,
        ObservedValueV1::Complete(PresenceV1::Present(_))
    ));
    assert!(matches!(
        history.unique_keys.terminal,
        EnumerationTerminalV1::Refused(_)
    ));
    assert!(matches!(
        history.checks.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    for table in sql
        .catalog
        .tables
        .iter()
        .filter(|table| table.name.as_str() > "history")
    {
        assert!(matches!(
            table.catalog_definition,
            ObservedValueV1::NotAttempted
        ));
    }
    assert!(matches!(
        sql.catalog.indexes.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert!(matches!(
        sql.instances.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert_eq!(sql.catalog.rejected_rows.len(), 1);
    let rejected = &sql.catalog.rejected_rows[0];
    assert!(matches!(&rejected.at,PhysicalCoordinateV1::SqlCatalog(at)
        if at.family==CatalogFamilyV1::UniqueKeys && at.table==PresenceV1::Present("history".into())
            && at.row==PresenceV1::Present(0)));
    assert!(rejected.cells.iter().any(|cell| cell.column == "conname"
        && cell.value == SqlCellValueV1::Text(HexBytesV1::new(b"history_record_id_key".to_vec()))));
    assert!(rejected
        .cells
        .iter()
        .any(|cell| cell.column == "condeferrable"
            && cell.value
                == SqlCellValueV1::PostgresBinary(PostgresBinaryCellV1 {
                    type_id: "16".into(),
                    bytes: HexBytesV1::new(vec![1]),
                })));
    assert!(failure
        .refusals
        .iter()
        .any(|refusal| refusal.at == rejected.at
            && refusal.code == CaptureRefusalCodeV1::UnsupportedSchema));
}

#[test]
// Keep the four family positions and each retained prefix assertion in one test run.
#[allow(clippy::too_many_lines)]
fn postgres_row_failures_retain_the_physical_prefix_and_stop_before_later_families() {
    let Some(mut fixture) = Fixture::selected() else {
        return;
    };
    for (family, table, column, insert) in [
        (
            0,
            "instances",
            "revision",
            "INSERT INTO instances VALUES('aep.entity','z',-1,'{}')",
        ),
        (
            1,
            "events",
            "revision",
            "INSERT INTO events VALUES('aep.entity','z',-1,0,'{}')",
        ),
        (
            2,
            "history",
            "position",
            "INSERT INTO history VALUES('aep.entity','z',-1,'decision','rejected','{}')",
        ),
        (
            3,
            "legacy_origins",
            "revision",
            "INSERT INTO legacy_origins VALUES('aep.entity','z',-1)",
        ),
    ] {
        fixture.reset();
        fixture.client.batch_execute(insert).unwrap();
        let query = format!("SELECT tableoid,ctid::text FROM {table} WHERE id='z'");
        let row = fixture.client.query_one(&query, &[]).unwrap();
        let expected_oid: u32 = row.get(0);
        let expected_tuple_id: String = row.get(1);
        let failure = fixture.failed_capture();
        let PhaseEvidenceV1::Postgres(sql) = failure.evidence else {
            unreachable!()
        };
        assert!(matches!(
            sql.catalog.foreign_objects.terminal,
            EnumerationTerminalV1::Complete
        ));
        let families = [
            (sql.instances.items.len(), &sql.instances.terminal),
            (sql.events.items.len(), &sql.events.terminal),
            (sql.history.items.len(), &sql.history.terminal),
            (sql.legacy_origins.items.len(), &sql.legacy_origins.terminal),
        ];
        for (index, (length, terminal)) in families.iter().enumerate() {
            match index.cmp(&family) {
                std::cmp::Ordering::Less => {
                    assert_eq!(*length, 1);
                    assert!(matches!(terminal, EnumerationTerminalV1::Complete));
                }
                std::cmp::Ordering::Equal => {
                    assert_eq!(*length, 1);
                    assert!(matches!(terminal, EnumerationTerminalV1::Refused(_)));
                }
                std::cmp::Ordering::Greater => {
                    assert_eq!(*length, 0);
                    assert!(matches!(terminal, EnumerationTerminalV1::NotAttempted));
                }
            }
        }
        assert!(matches!(
            sql.provider_sequences,
            ObservedSequenceRowsV1::Observed(ObservedListV1 {
                terminal: EnumerationTerminalV1::NotAttempted,
                ..
            })
        ));
        assert_eq!(sql.rejected_rows.len(), 1);
        let rejected = &sql.rejected_rows[0];
        let PhysicalCoordinateV1::SqlPhysicalRow(at) = &rejected.at else {
            panic!("lost actual row locator");
        };
        assert_eq!(at.table, table);
        let SqlPhysicalLocatorV1::PostgresTuple(locator) = &at.locator else {
            panic!("wrong locator");
        };
        assert_eq!(locator.table_oid, expected_oid);
        assert_eq!(
            format!("({},{})", locator.block, locator.offset),
            expected_tuple_id
        );
        assert!(rejected
            .cells
            .iter()
            .any(|cell| cell.column == column && cell.value == SqlCellValueV1::Integer(-1)));
        assert_eq!(
            failure.refusals,
            vec![CaptureRefusalV1 {
                code: CaptureRefusalCodeV1::NumericOutOfRange,
                at: rejected.at.clone(),
            }]
        );
        let after = fixture.client.query_one(&query, &[]).unwrap();
        assert_eq!(after.get::<_, u32>(0), expected_oid);
        assert_eq!(after.get::<_, String>(1), expected_tuple_id);
    }
}

#[test]
fn postgres_foreign_catalog_objects_keep_their_definitions_before_admission_refuses() {
    let Some(mut fixture) = Fixture::selected() else {
        return;
    };
    fixture
        .client
        .batch_execute("CREATE VIEW extra_view AS SELECT id FROM instances")
        .unwrap();
    let failure = fixture.failed_capture();
    let PhaseEvidenceV1::Postgres(sql) = failure.evidence else {
        unreachable!()
    };
    assert!(matches!(
        sql.catalog.foreign_objects.terminal,
        EnumerationTerminalV1::Complete
    ));
    assert!(sql.catalog.foreign_objects.items.iter().any(|object|
        object.kind==ForeignObjectKindV1::View && object.name=="extra_view"
            && matches!(&object.catalog_definition,PresenceV1::Present(bytes) if !bytes.as_bytes().is_empty())));
    assert!(matches!(
        sql.instances.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert!(failure
        .refusals
        .iter()
        .all(|refusal| refusal.code == CaptureRefusalCodeV1::UnsupportedSchema));
}

#[test]
fn an_unresolved_postgres_endpoint_returns_a_valid_refusal_without_the_connection_string() {
    let input = "not-a-connection-string-with-sensitive-marker";
    let AcquisitionError::RefusedObservation(observation) =
        capture_postgres(input, selector()).expect_err("malformed config")
    else {
        panic!("must retain the known method and unattempted reads");
    };
    observation.validate().unwrap();
    assert!(matches!(observation.source, PresenceV1::Missing));
    let serialized = serde_json::to_string(&observation).unwrap();
    assert!(!serialized.contains(input));
    let ObservationOutcomeV1::Refused(refused) = observation.observation else {
        unreachable!()
    };
    let PhaseResultV1::Refused(phase) = &refused.phases[0].result else {
        unreachable!()
    };
    let PhaseEvidenceV1::Postgres(sql) = &phase.evidence else {
        unreachable!()
    };
    assert!(matches!(
        sql.catalog.objects.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert!(matches!(
        sql.instances.terminal,
        EnumerationTerminalV1::NotAttempted
    ));
    assert_eq!(
        phase.refusals[0].code,
        CaptureRefusalCodeV1::UnresolvedSourceCoordinate
    );
}

const PROVIDER_DDL: &str = "
        CREATE TABLE instances (entity TEXT NOT NULL, id TEXT NOT NULL, revision BIGINT NOT NULL,
            document TEXT NOT NULL, PRIMARY KEY(entity,id));
        CREATE TABLE events (entity TEXT NOT NULL, id TEXT NOT NULL, revision BIGINT NOT NULL,
            position BIGINT NOT NULL, document TEXT NOT NULL, PRIMARY KEY(entity,id,revision,position));
        CREATE TABLE history (entity TEXT NOT NULL, id TEXT NOT NULL, position BIGINT NOT NULL,
            kind TEXT NOT NULL CHECK(kind IN ('decision','observation')), record_id TEXT NOT NULL UNIQUE,
            document TEXT NOT NULL, PRIMARY KEY(entity,id,position));
        CREATE TABLE legacy_origins (entity TEXT NOT NULL, id TEXT NOT NULL, revision BIGINT NOT NULL,
            PRIMARY KEY(entity,id));
        CREATE TABLE provider_sequences (namespace TEXT PRIMARY KEY, next_value BIGINT NOT NULL CHECK(next_value>=0));
        CREATE INDEX instances_document_query ON instances USING gin ((document::jsonb) jsonb_path_ops);
    ";
