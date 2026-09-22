//! Retain physical refusal evidence in the existing raw-capture envelope.

use super::{selector_coordinate, AcquisitionError, SelectorBinding};
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;

pub(super) struct CatalogFailure {
    pub catalog: SqlCatalogEvidenceV1,
    pub refusals: Vec<CaptureRefusalV1>,
}

pub(super) struct CatalogProgress {
    pub(super) catalog: SqlCatalogEvidenceV1,
    pub(super) at: SqlCatalogCoordinateV1,
    pub(super) pending: Vec<RejectedCatalogRowV1>,
}

impl CatalogProgress {
    pub(super) fn next(&mut self, family: CatalogFamilyV1, table: PresenceV1<String>) {
        self.at = SqlCatalogCoordinateV1 {
            namespace: self.catalog.namespace.clone(),
            family,
            table,
            row: PresenceV1::Missing,
        };
        self.pending.clear();
    }

    pub(super) fn refused(mut self, error: &AcquisitionError) -> Box<CatalogFailure> {
        let code = if matches!(error, AcquisitionError::InvalidCapture) {
            CaptureRefusalCodeV1::UnsupportedSchema
        } else {
            CaptureRefusalCodeV1::ReadFailure
        };
        let at = PhysicalCoordinateV1::SqlCatalog(self.at.clone());
        let mut refusals = self
            .pending
            .iter()
            .map(|row| CaptureRefusalV1 {
                code: code.clone(),
                at: row.at.clone(),
            })
            .collect::<Vec<_>>();
        refusals.push(CaptureRefusalV1 {
            code: code.clone(),
            at: at.clone(),
        });
        refusals.sort_by_key(|value| (sort_key_v1(&value.at), sort_key_v1(&value.code)));
        refusals.dedup();
        self.catalog.rejected_rows.append(&mut self.pending);
        self.catalog
            .rejected_rows
            .sort_by_key(|row| sort_key_v1(&row.at));
        let terminal = EnumerationTerminalV1::Refused(EnumerationRefusalV1 { at, code });
        let table = self
            .catalog
            .tables
            .iter_mut()
            .find(|table| self.at.table == PresenceV1::Present(table.name.clone()));
        let list = match (&self.at.family, table) {
            (CatalogFamilyV1::Objects, _) => Some(&mut self.catalog.objects.terminal),
            (CatalogFamilyV1::Columns, Some(table)) => Some(&mut table.columns.terminal),
            (CatalogFamilyV1::UniqueKeys, Some(table)) => Some(&mut table.unique_keys.terminal),
            (CatalogFamilyV1::Checks, Some(table)) => Some(&mut table.checks.terminal),
            (CatalogFamilyV1::Indexes, _) => Some(&mut self.catalog.indexes.terminal),
            (CatalogFamilyV1::ForeignObjects, _) => {
                Some(&mut self.catalog.foreign_objects.terminal)
            }
            _ => None,
        };
        if let Some(list) = list {
            // A completed read followed by admission failure remains completed evidence.
            if !matches!(list, EnumerationTerminalV1::Complete) {
                *list = terminal;
            }
        }
        Box::new(CatalogFailure {
            catalog: self.catalog,
            refusals,
        })
    }
}

pub(super) fn unattempted<T>() -> ObservedListV1<T> {
    ObservedListV1 {
        items: Vec::new(),
        terminal: EnumerationTerminalV1::NotAttempted,
    }
}

pub(super) fn sql_catalog(
    source: SqlReplicaCoordinateV1,
    failure: CatalogFailure,
) -> AcquisitionError {
    let mut sql = empty_sql(source);
    sql.catalog = failure.catalog;
    sql_phase(sql, failure.refusals)
}

pub(super) fn empty_sql(source: SqlReplicaCoordinateV1) -> SqlEvidenceV1 {
    let sqlite = matches!(source, SqlReplicaCoordinateV1::Sqlite(_));
    let mut evidence = unresolved_postgres();
    evidence.source = PresenceV1::Present(source);
    if sqlite {
        evidence.provider_sequences = ObservedSequenceRowsV1::NotApplicable;
    }
    evidence
}

pub(super) fn unresolved_postgres() -> SqlEvidenceV1 {
    SqlEvidenceV1 {
        source: PresenceV1::Missing,
        catalog: SqlCatalogEvidenceV1 {
            namespace: PresenceV1::Missing,
            objects: unattempted(),
            tables: Vec::new(),
            indexes: unattempted(),
            foreign_objects: unattempted(),
            rejected_rows: Vec::new(),
        },
        instances: unattempted(),
        events: unattempted(),
        history: unattempted(),
        legacy_origins: unattempted(),
        provider_sequences: ObservedSequenceRowsV1::Observed(unattempted()),
        rejected_rows: Vec::new(),
    }
}

pub(super) fn sql_phase(sql: SqlEvidenceV1, refusals: Vec<CaptureRefusalV1>) -> AcquisitionError {
    let evidence = if matches!(
        sql.source,
        PresenceV1::Present(SqlReplicaCoordinateV1::Sqlite(_))
    ) {
        PhaseEvidenceV1::Sqlite(sql)
    } else {
        PhaseEvidenceV1::Postgres(sql)
    };
    let evidence_digest = evidence.evidence_digest();
    AcquisitionError::SqlPhase(Box::new(RefusedPhaseResultV1 {
        evidence,
        evidence_digest,
        refusals,
    }))
}

pub(super) fn observation(
    source: SourceCoordinateV1,
    selector: SelectorBinding,
    method: ObservationMethodV1,
    phases: Vec<PhaseObservationV1>,
) -> AcquisitionError {
    partial_observation(PresenceV1::Present(source), selector, method, phases)
}

pub(super) fn partial_observation(
    source: PresenceV1<SourceCoordinateV1>,
    selector: SelectorBinding,
    method: ObservationMethodV1,
    phases: Vec<PhaseObservationV1>,
) -> AcquisitionError {
    let config_digest = selector.config_digest;
    AcquisitionError::RefusedObservation(Box::new(RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source,
        selector: PresenceV1::Present(selector_coordinate(selector)),
        config_digest: PresenceV1::Present(config_digest),
        observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
            method: PresenceV1::Present(method),
            phases,
            preflight_refusals: Vec::new(),
        }),
    }))
}
