//! Exact legacy source values mapped into Entity Runtime import boundaries.

use std::collections::{BTreeMap, BTreeSet};

use aep_backend_markdown::{StoreReport, StoredDocument};
use aep_contract::migration::{
    digest_parts_v1, AuthorityCoordinateV1, DigestV1, HexBytesV1, HistoryKindV1, HistoryRowV1,
    HostPathV1, LegacyRawCaptureV1, MarkdownNodeKindV1, MarkdownRawV1, PresenceV1, SqlRawV1,
};
use aep_domain::artifact::ArtifactKind;
use aep_domain::entity::{
    ActorRef, EntityId, EntityLocator, EntityMetadata, EntityProvenance, EntityRef, EntityRevision,
};
use aep_domain::ids::RelationId;
use aep_domain::time::Timestamp;
use aep_domain::workspace::MemberName;
use entity_core::{DomainEvent, EntityInstance};
use entity_store::asynchronous::{
    HistoryOrigin, ImportedRecordEvidence, KnownLegacyOrder, LegacyAnchor, LegacyCompleteness,
    LegacyEvidence, LegacyOrderDeclaration, RecordedEntry, Subject, SubjectHistory,
};
use entity_store::{RecordedCommit, RecordedObservation};

const RECORD_COORDINATE_AS: &str = "aep.migration.LegacyRecordCoordinate";
const EVIDENCE_BLOB_AS: &str = "aep.migration.LegacyEvidenceBlob";
const RESERVATION_ROSTER_AS: &str = "aep.migration.LegacyIdReservationRoster";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRecordCoordinate {
    format: String,
    source_snapshot: DigestV1,
    source_locator: String,
    destination_entity: String,
    destination_id: String,
    evidence_kind: String,
    original_record_id: PresenceV1<String>,
    order: String,
    ordinal: PresenceV1<u64>,
    evidence_blob_id: String,
    envelope_digest: DigestV1,
    reservation_roster_id: PresenceV1<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEvidenceBlob {
    format: String,
    source_snapshot: DigestV1,
    coordinate_subject_id: String,
    original_record_id: PresenceV1<String>,
    exact_bytes: HexBytesV1,
    byte_length: u64,
    envelope_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyReservationEntry {
    record_id: String,
    coordinate_subject_id: String,
    evidence_blob_subject_id: String,
    envelope_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyIdReservationRoster {
    format: String,
    boundary_id: String,
    authority: AuthorityCoordinateV1,
    entries: Vec<LegacyReservationEntry>,
    roster_digest: DigestV1,
}

struct RetainedBoundaryItem {
    source_locator: String,
    destination_entity: String,
    destination_id: String,
    evidence_kind: String,
    original_record_id: PresenceV1<String>,
    order: String,
    ordinal: PresenceV1<u64>,
    exact_bytes: Vec<u8>,
}

/// A deterministic mapping refusal that never drops the exact source coordinate.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MappingError {
    /// The captured source omitted a value required to preserve a subject boundary.
    #[error("legacy source is incomplete at {coordinate}")]
    Incomplete {
        /// Exact retained legacy coordinate at which the value is absent.
        coordinate: String,
    },
    /// The captured source contains bytes that do not satisfy their declared legacy shape.
    #[error("legacy value is invalid at {coordinate}")]
    Invalid {
        /// Exact retained legacy coordinate at which decoding or validation failed.
        coordinate: String,
    },
    /// A retained history cannot establish the subject's final state.
    #[error("legacy subject has no terminal state at {coordinate}")]
    MissingTerminal {
        /// Exact retained history coordinate missing a terminal instance.
        coordinate: String,
    },
}

impl MappingError {
    /// The exact source coordinate the refusal is about.
    ///
    /// A caller reporting a mapping refusal has to be able to name where it happened. Collapsing
    /// every variant to one code at the caller's own coordinate is what made the workspace defect
    /// below take a day to find: the receipt named the selector, which was not what failed.
    pub fn coordinate(&self) -> &str {
        match self {
            Self::Incomplete { coordinate }
            | Self::Invalid { coordinate }
            | Self::MissingTerminal { coordinate } => coordinate,
        }
    }
}

/// Maps a complete Markdown read into explicit unrecorded subject boundaries.
///
/// Markdown has complete current state but no provider-complete mixed record order. Each boundary
/// therefore says `available_evidence_only` and `per_kind_only`; it does not fabricate history.
///
/// `members` is the workspace declaration beside the store, exactly as every ordinary read command
/// supplies it. See [`markdown_boundaries_raw`] for why it is a parameter rather than a read.
pub fn markdown_boundaries(
    report: &StoreReport,
    members: &[MemberName],
) -> Result<Vec<SubjectHistory>, MappingError> {
    if let Some(failure) = report.failures.first() {
        return Err(MappingError::Incomplete {
            coordinate: failure.path.display().to_string(),
        });
    }
    markdown_runtime_boundaries(&report.documents, members)
}

/// Maps the exact bytes retained by a complete raw capture, without reopening the source tree.
///
/// `members` is what the store's `.engineering/workspace.yaml` declares. It is supplied by the
/// caller and never read here: this crate reopens nothing, and the declaration lives beside the
/// store rather than inside it, so it is not in the capture. It is admission input only — the
/// graph it gates is discarded, and no produced record depends on it — so a store whose
/// declaration changed between capture and mapping still maps to the same bytes.
///
/// Passing an empty slice is what a store with no workspace file declares, and it is the defect
/// this parameter exists to close: the mapper used to pass empty unconditionally, so a relation
/// into a declared member read as a dangling edge and the whole migration was refused, while every
/// ordinary read command on the same store answered that it was valid.
pub fn markdown_boundaries_raw(
    raw: &MarkdownRawV1,
    members: &[MemberName],
) -> Result<Vec<SubjectHistory>, MappingError> {
    let mut documents = BTreeMap::new();
    for node in &raw.nodes {
        let MarkdownNodeKindV1::Regular(file) = &node.node else {
            continue;
        };
        let relative = relative_text(&node.relative)?;
        if relative == "journal.jsonl" {
            continue;
        }
        let text =
            std::str::from_utf8(file.bytes.as_bytes()).map_err(|_| MappingError::Invalid {
                coordinate: relative.clone(),
            })?;
        let document = aep_backend_markdown::PlanningDocument::parse(text, Some(&relative))
            .map_err(|_| MappingError::Invalid {
                coordinate: relative.clone(),
            })?;
        let stem = relative
            .strip_suffix(".md")
            .ok_or_else(|| MappingError::Invalid {
                coordinate: relative.clone(),
            })?;
        let (directory, name) = stem.split_once('/').ok_or_else(|| MappingError::Invalid {
            coordinate: relative.clone(),
        })?;
        if ArtifactKind::parse(directory).ok().as_ref() != Some(&document.frontmatter.kind)
            || name != document.frontmatter.id.name()
            || documents
                .insert(
                    document.frontmatter.id.clone(),
                    StoredDocument {
                        relative_path: relative.clone(),
                        document,
                    },
                )
                .is_some()
        {
            return Err(MappingError::Invalid {
                coordinate: relative,
            });
        }
    }
    markdown_runtime_boundaries(&documents, members)
}

/// The document a graph defect belongs to, so a refusal names a file somebody can open.
///
/// `ValidationError::location` is dotted document form — `artifacts.<id>.relations[2]` — and an
/// artifact id may itself contain `.`, so the id is recovered by matching the documents that were
/// read rather than by splitting on the first separator. A defect that names no document read here
/// falls back to the whole graph, which is still more than the selector.
fn graph_coordinate(
    errors: &aep_domain::ValidationErrors,
    documents: &BTreeMap<aep_domain::artifact::ArtifactId, StoredDocument>,
) -> String {
    errors
        .as_slice()
        .iter()
        .find_map(|error| {
            let location = error.location.strip_prefix("artifacts.")?;
            documents
                .iter()
                .filter(|(artifact, _)| {
                    let name = artifact.to_string();
                    location
                        .strip_prefix(&name)
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '[']))
                })
                .max_by_key(|(artifact, _)| artifact.to_string().len())
                .map(|(_, stored)| stored.relative_path.clone())
        })
        .unwrap_or_else(|| "markdown/graph".to_owned())
}

/// Maps Markdown's document-shaped current state into the four-kind identity representation used
/// by every ordinary Eventlog planning command. The imported anchor stays honest about Markdown's
/// absent history, while its terminal value must still be one the selected backend can hydrate.
#[allow(clippy::too_many_lines)] // One pass keeps document, body and relation mapping adjacent.
fn markdown_runtime_boundaries(
    documents: &BTreeMap<aep_domain::artifact::ArtifactId, StoredDocument>,
    members: &[MemberName],
) -> Result<Vec<SubjectHistory>, MappingError> {
    let report = StoreReport {
        documents: documents.clone(),
        files_read: documents.len(),
        failures: Vec::new(),
    };
    // `graph_in_workspace`, not `graph`: the same call every ordinary read command makes
    // (`planning.rs` `artifact validate`, `graph`, `board`, `explain`, …). A relation into a
    // member this store declares is a crossing an assembly resolves, not a dangling edge; a
    // relation into a member it does *not* declare stays a dangling edge and is still refused,
    // because a store migrated with edges that dangle for real is worse than a refusal.
    report
        .graph_in_workspace(members.iter().cloned())
        .map_err(|errors| MappingError::Invalid {
            coordinate: graph_coordinate(&errors, documents),
        })?;

    let mut identities = BTreeMap::new();
    for (artifact, stored) in documents {
        identities.insert(
            artifact.clone(),
            markdown_entity_id(&stored.relative_path, &artifact.to_string())?,
        );
    }

    let mut histories = Vec::new();
    for (artifact, stored) in documents {
        let document = &stored.document;
        let entity_id = identities
            .get(artifact)
            .expect("every retained document received one identity")
            .clone();
        let locator = EntityLocator::new(
            aep_backend_markdown::backend::ORGANISATION,
            aep_backend_markdown::backend::SPACE,
            document.frontmatter.kind.as_str(),
            locator_key(artifact.name()),
        )
        .map_err(|_| MappingError::Invalid {
            coordinate: stored.relative_path.clone(),
        })?;
        let revision = EntityRevision::new(document.frontmatter.revision).map_err(|_| {
            MappingError::Invalid {
                coordinate: stored.relative_path.clone(),
            }
        })?;
        let metadata = EntityMetadata {
            id: entity_id.clone(),
            locator,
            entity_type: document.frontmatter.kind.entity_type(),
            revision,
            created_at: Timestamp::EPOCH,
            updated_at: Timestamp::EPOCH,
            provenance: EntityProvenance::created_by(ActorRef::System),
        };
        let direct = aep_backend_markdown::provider::instance_of(
            document.frontmatter.kind.as_str(),
            artifact.name(),
            document,
        );
        let mut fields = direct.fields;
        fields.insert(
            "status".to_owned(),
            serde_json::Value::String(direct.lifecycle_state.clone()),
        );
        fields.insert(
            "$aep".to_owned(),
            serde_json::json!({ "metadata": metadata, "archived": false }),
        );
        histories.push(boundary(
            EntityInstance {
                entity: "aep.entity".to_owned(),
                version: 1,
                id: entity_id.to_string(),
                lifecycle_state: direct.lifecycle_state,
                revision: document.frontmatter.revision,
                fields,
            },
            Vec::new(),
            LegacyOrderDeclaration::PerKindOnly,
        )?);

        for (ordinal, declared) in document.frontmatter.relations.iter().enumerate() {
            let Some(target) = identities.get(declared.target.id()) else {
                // Workspace crossings have no destination entity in this authority. The exact
                // frontmatter relation remains in the entity body and is restored by projection.
                continue;
            };
            let relation_id = markdown_relation_id(&stored.relative_path, ordinal)?;
            let relation = aep_contract::query::Relation {
                id: relation_id.clone(),
                kind: declared.kind,
                source: EntityRef::new(entity_id.clone()),
                target: EntityRef::new(target.clone()),
                created_at: Timestamp::EPOCH,
                created_by: ActorRef::System,
            };
            histories.push(boundary(
                EntityInstance {
                    entity: "aep.relation".to_owned(),
                    version: 1,
                    id: relation_id.to_string(),
                    lifecycle_state: "recorded".to_owned(),
                    revision: 1,
                    fields: serde_json::from_value(serde_json::json!({
                        "relation": relation,
                        "removed": false,
                    }))
                    .expect("a relation record is a JSON object"),
                },
                Vec::new(),
                LegacyOrderDeclaration::PerKindOnly,
            )?);
        }
    }
    histories.sort_by(|left, right| left.subject.cmp(&right.subject));
    Ok(histories)
}

fn markdown_entity_id(relative: &str, artifact: &str) -> Result<EntityId, MappingError> {
    let digest = digest_parts_v1(
        "aep.migration.markdown-entity/1",
        &[relative.as_bytes().to_vec(), artifact.as_bytes().to_vec()],
    )
    .map_err(|_| MappingError::Invalid {
        coordinate: relative.to_owned(),
    })?;
    EntityId::new(format!(
        "MIG{}",
        digest.as_wire().trim_start_matches("sha256:")
    ))
    .map_err(|_| MappingError::Invalid {
        coordinate: relative.to_owned(),
    })
}

fn markdown_relation_id(relative: &str, ordinal: usize) -> Result<RelationId, MappingError> {
    let digest = digest_parts_v1(
        "aep.migration.markdown-relation/1",
        &[
            relative.as_bytes().to_vec(),
            u64::try_from(ordinal)
                .map_err(|_| MappingError::Invalid {
                    coordinate: relative.to_owned(),
                })?
                .to_be_bytes()
                .to_vec(),
        ],
    )
    .map_err(|_| MappingError::Invalid {
        coordinate: relative.to_owned(),
    })?;
    RelationId::new(format!(
        "mig-rel-{}",
        digest.as_wire().trim_start_matches("sha256:")
    ))
    .map_err(|_| MappingError::Invalid {
        coordinate: relative.to_owned(),
    })
}

fn locator_key(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn relative_text(path: &HostPathV1) -> Result<String, MappingError> {
    match path {
        HostPathV1::Unix(bytes) => std::str::from_utf8(bytes.as_bytes())
            .map(ToOwned::to_owned)
            .map_err(|_| MappingError::Invalid {
                coordinate: "markdown/path".to_owned(),
            }),
        HostPathV1::Windows(units) => {
            String::from_utf16(units).map_err(|_| MappingError::Invalid {
                coordinate: "markdown/path".to_owned(),
            })
        }
    }
}

/// Adds the closed raw boundary value and immutable original identity roster as an ordinary
/// imported authority subject. This is the authoritative recovery value after external copies are
/// removed; it is covered by the provider-complete snapshot like every business subject.
pub fn boundaries_with_authoritative_evidence(
    mut histories: Vec<SubjectHistory>,
    capture: &LegacyRawCaptureV1,
    snapshot: DigestV1,
) -> Result<Vec<SubjectHistory>, MappingError> {
    let mut ids = histories
        .iter()
        .map(|history| format!("{}\u{0}{}", history.subject.entity, history.subject.id))
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    let raw = serde_json::to_value(capture).map_err(|_| MappingError::Invalid {
        coordinate: "boundary/raw_capture".to_owned(),
    })?;
    let mut fields = serde_json::Map::new();
    fields.insert(
        "format".to_owned(),
        serde_json::Value::String("aep.planning-import-boundary/1".to_owned()),
    );
    fields.insert(
        "source_snapshot".to_owned(),
        serde_json::Value::String(snapshot.as_wire()),
    );
    fields.insert("raw_capture".to_owned(), raw);
    fields.insert(
        "reserved_subjects".to_owned(),
        serde_json::to_value(ids).expect("string vector serialises"),
    );
    fields.insert(
        "mixed_order".to_owned(),
        serde_json::Value::String("available_evidence_only".to_owned()),
    );
    for history in &mut histories {
        let HistoryOrigin::Imported(anchor) = &mut history.origin else {
            continue;
        };
        if matches!(
            anchor.instance.entity.as_str(),
            "aep.entity" | "aep.relation" | "aep.audit" | "aep.applied"
        ) {
            anchor.instance = eventlog_planning_terminal(&anchor.instance);
        }
    }
    let instance = EntityInstance {
        entity: "aep.planning-import-boundary".to_owned(),
        version: 1,
        id: snapshot.as_wire(),
        lifecycle_state: "recorded".to_owned(),
        revision: 1,
        fields,
    };
    histories.push(boundary(
        instance,
        Vec::new(),
        LegacyOrderDeclaration::PerKindOnly,
    )?);
    histories.sort_by(|left, right| left.subject.cmp(&right.subject));
    Ok(histories)
}

/// The Eventlog planning adapter stores each AEP provider row as a closed document on the same
/// subject. Ordinary commands unwrap this exact shape on open. Legacy import therefore anchors the
/// provider row in that durable representation rather than leaving a raw row that no ordinary
/// reader can hydrate. Historical evidence remains attached to the same subject unchanged.
fn eventlog_planning_terminal(instance: &EntityInstance) -> EntityInstance {
    EntityInstance {
        entity: instance.entity.clone(),
        version: instance.version,
        id: instance.id.clone(),
        lifecycle_state: "recorded".to_owned(),
        revision: instance.revision,
        fields: serde_json::from_value(serde_json::json!({
            "document": {
                "lifecycle_state": instance.lifecycle_state,
                "fields": instance.fields,
                "events": [],
            }
        }))
        .expect("the Eventlog planning document is a JSON object"),
    }
}

/// Adds the provider-complete exact evidence blobs and immutable original-record-id roster once
/// the provider-assigned authority coordinate is known. These are ordinary imported subjects, so
/// deleting the external recovery copy cannot remove either the bytes or the collision evidence.
pub fn bind_authoritative_evidence(
    histories: Vec<SubjectHistory>,
    authority: &AuthorityCoordinateV1,
) -> Result<Vec<SubjectHistory>, MappingError> {
    bind_authoritative_evidence_with_unavailable(histories, authority, &[])
}

/// One complete legacy envelope whose exact source does not establish subject or per-kind order.
/// It remains an ordinary coordinate/blob/roster join and is deliberately not coerced into the
/// Entity Runtime imported-envelope type, whose public contract requires a known order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableLegacyEnvelope {
    /// Exact coordinate within the captured source.
    pub source_locator: String,
    /// Destination provider subject type.
    pub destination_entity: String,
    /// Destination provider subject identity.
    pub destination_id: String,
    /// Concrete complete envelope kind.
    pub evidence_kind: HistoryKindV1,
    /// Original immutable global record identity.
    pub original_record_id: String,
    /// Exact complete envelope bytes.
    pub exact_bytes: HexBytesV1,
}

/// Adds raw-capture evidence plus explicit complete envelopes whose order is unavailable.
#[allow(clippy::too_many_lines)] // One audit path builds the closed coordinate/blob/roster join.
pub fn bind_authoritative_evidence_with_unavailable(
    mut histories: Vec<SubjectHistory>,
    authority: &AuthorityCoordinateV1,
    unavailable: &[UnavailableLegacyEnvelope],
) -> Result<Vec<SubjectHistory>, MappingError> {
    if histories.is_empty() && unavailable.is_empty() {
        return Ok(histories);
    }
    let (boundary_id, source_snapshot, capture) = histories
        .iter()
        .find_map(|history| {
            let HistoryOrigin::Imported(anchor) = &history.origin else {
                return None;
            };
            (history.subject.entity == "aep.planning-import-boundary").then(|| {
                let source = anchor.instance.fields.get("source_snapshot")?.as_str()?;
                let snapshot = DigestV1::parse(source).ok()?;
                let capture =
                    serde_json::from_value(anchor.instance.fields.get("raw_capture")?.clone())
                        .ok()?;
                Some((history.subject.id.clone(), snapshot, capture))
            })?
        })
        .ok_or_else(|| MappingError::Incomplete {
            coordinate: "boundary/raw_capture".to_owned(),
        })?;

    let mut items = retained_boundary_items(&capture)?;
    for envelope in unavailable {
        let entry = match envelope.evidence_kind {
            HistoryKindV1::Decision => {
                serde_json::from_slice::<RecordedCommit>(envelope.exact_bytes.as_bytes())
                    .map(RecordedEntry::Decision)
            }
            HistoryKindV1::Observation => {
                serde_json::from_slice::<RecordedObservation>(envelope.exact_bytes.as_bytes())
                    .map(RecordedEntry::Observation)
            }
        }
        .map_err(|_| MappingError::Invalid {
            coordinate: envelope.source_locator.clone(),
        })?;
        if entry.record_id() != envelope.original_record_id
            || entry.subject().entity != envelope.destination_entity
            || entry.subject().id != envelope.destination_id
        {
            return Err(MappingError::Invalid {
                coordinate: envelope.source_locator.clone(),
            });
        }
        items.push(RetainedBoundaryItem {
            source_locator: envelope.source_locator.clone(),
            destination_entity: envelope.destination_entity.clone(),
            destination_id: envelope.destination_id.clone(),
            evidence_kind: match envelope.evidence_kind {
                HistoryKindV1::Decision => "decision",
                HistoryKindV1::Observation => "observation",
            }
            .to_owned(),
            original_record_id: PresenceV1::Present(envelope.original_record_id.clone()),
            order: "unavailable".to_owned(),
            ordinal: PresenceV1::Missing,
            exact_bytes: envelope.exact_bytes.as_bytes().to_vec(),
        });
    }
    items.sort_by(|left, right| left.source_locator.cmp(&right.source_locator));
    if items.is_empty() {
        return Ok(histories);
    }

    let authority_id = digest_parts_v1(
        "aep.migration.authority-boundary/1",
        &[
            authority.logical_scope.as_str().as_bytes().to_vec(),
            authority.tenant.as_str().as_bytes().to_vec(),
            authority.stream_identity.as_str().as_bytes().to_vec(),
            boundary_id.as_bytes().to_vec(),
        ],
    )
    .map_err(|_| MappingError::Invalid {
        coordinate: "boundary/authority".to_owned(),
    })?;
    let roster_id = format!(
        "legacy-roster-{}",
        authority_id.as_wire().trim_start_matches("sha256:")
    );
    let mut reservations = Vec::with_capacity(items.len());
    let mut additions = Vec::with_capacity(items.len() * 2 + 1);
    let mut seen = BTreeMap::<String, DigestV1>::new();
    for item in items {
        let locator = item.source_locator.clone();
        let envelope_digest = digest_parts_v1(
            "aep.migration.legacy-evidence/1",
            std::slice::from_ref(&item.exact_bytes),
        )
        .map_err(|_| MappingError::Invalid {
            coordinate: locator.clone(),
        })?;
        if let PresenceV1::Present(record_id) = &item.original_record_id {
            if record_id.is_empty()
                || seen
                    .insert(record_id.clone(), envelope_digest)
                    .is_some_and(|held| held != envelope_digest)
            {
                return Err(MappingError::Invalid {
                    coordinate: locator,
                });
            }
        }
        let coordinate_digest = digest_parts_v1(
            "aep.migration.legacy-coordinate/1",
            &[
                source_snapshot.as_bytes().to_vec(),
                item.destination_entity.as_bytes().to_vec(),
                item.destination_id.as_bytes().to_vec(),
                locator.as_bytes().to_vec(),
                item.exact_bytes.clone(),
            ],
        )
        .map_err(|_| MappingError::Invalid {
            coordinate: locator.clone(),
        })?;
        let coordinate_id = format!(
            "legacy-coordinate-{}",
            coordinate_digest.as_wire().trim_start_matches("sha256:")
        );
        let blob_id = format!(
            "legacy-evidence-{}",
            envelope_digest.as_wire().trim_start_matches("sha256:")
        );
        let coordinate = LegacyRecordCoordinate {
            format: "aep.legacy-record-coordinate/1".to_owned(),
            source_snapshot,
            source_locator: locator,
            destination_entity: item.destination_entity,
            destination_id: item.destination_id,
            evidence_kind: item.evidence_kind,
            original_record_id: item.original_record_id.clone(),
            order: item.order,
            ordinal: item.ordinal,
            evidence_blob_id: blob_id.clone(),
            envelope_digest,
            reservation_roster_id: match &item.original_record_id {
                PresenceV1::Present(_) => PresenceV1::Present(roster_id.clone()),
                PresenceV1::Missing => PresenceV1::Missing,
            },
        };
        let blob = LegacyEvidenceBlob {
            format: "aep.legacy-evidence-blob/1".to_owned(),
            source_snapshot,
            coordinate_subject_id: coordinate_id.clone(),
            original_record_id: item.original_record_id.clone(),
            exact_bytes: HexBytesV1::new(item.exact_bytes.clone()),
            byte_length: u64::try_from(item.exact_bytes.len()).map_err(|_| {
                MappingError::Invalid {
                    coordinate: "boundary/evidence-length".to_owned(),
                }
            })?,
            envelope_digest,
        };
        additions.push(boundary_value(
            RECORD_COORDINATE_AS,
            &coordinate_id,
            &coordinate,
        )?);
        additions.push(boundary_value(EVIDENCE_BLOB_AS, &blob_id, &blob)?);
        if let PresenceV1::Present(record_id) = item.original_record_id {
            reservations.push(LegacyReservationEntry {
                record_id,
                coordinate_subject_id: coordinate_id,
                evidence_blob_subject_id: blob_id,
                envelope_digest,
            });
        }
    }
    reservations.sort();
    reservations.dedup();
    let roster_parts = reservations
        .iter()
        .flat_map(|entry| {
            [
                entry.record_id.as_bytes().to_vec(),
                entry.coordinate_subject_id.as_bytes().to_vec(),
                entry.evidence_blob_subject_id.as_bytes().to_vec(),
                entry.envelope_digest.as_bytes().to_vec(),
            ]
        })
        .chain([
            authority.logical_scope.as_str().as_bytes().to_vec(),
            authority.tenant.as_str().as_bytes().to_vec(),
            authority.stream_identity.as_str().as_bytes().to_vec(),
            boundary_id.as_bytes().to_vec(),
        ])
        .collect::<Vec<_>>();
    if !reservations.is_empty() {
        let roster = LegacyIdReservationRoster {
            format: "aep.legacy-id-reservations/1".to_owned(),
            boundary_id,
            authority: authority.clone(),
            entries: reservations,
            roster_digest: digest_parts_v1("aep.migration.legacy-id-reservations/1", &roster_parts)
                .map_err(|_| MappingError::Invalid {
                    coordinate: "boundary/roster".to_owned(),
                })?,
        };
        additions.push(boundary_value(RESERVATION_ROSTER_AS, &roster_id, &roster)?);
    }
    histories.extend(additions);
    histories.sort_by(|left, right| left.subject.cmp(&right.subject));
    Ok(histories)
}

fn retained_boundary_items(
    capture: &LegacyRawCaptureV1,
) -> Result<Vec<RetainedBoundaryItem>, MappingError> {
    let mut items = match capture {
        LegacyRawCaptureV1::Markdown(raw) => markdown_journal_items(raw)?,
        LegacyRawCaptureV1::Sqlite(raw) | LegacyRawCaptureV1::Postgres(raw) => {
            sql_history_items(raw)?
        }
        LegacyRawCaptureV1::Hybrid(raw) => {
            let mut items = markdown_journal_items(&raw.local)?;
            items.extend(sql_history_items(&raw.replica)?);
            items
        }
    };
    items.sort_by(|left, right| left.source_locator.cmp(&right.source_locator));
    Ok(items)
}

fn sql_history_items(raw: &SqlRawV1) -> Result<Vec<RetainedBoundaryItem>, MappingError> {
    raw.history
        .iter()
        .map(|row| {
            let ordinal = nonnegative(row.position, "history.position")?;
            Ok(RetainedBoundaryItem {
                source_locator: format!("history/{}/{}/{}", row.entity, row.id, row.position),
                destination_entity: row.entity.clone(),
                destination_id: row.id.clone(),
                evidence_kind: match row.kind {
                    HistoryKindV1::Decision => "decision",
                    HistoryKindV1::Observation => "observation",
                }
                .to_owned(),
                original_record_id: PresenceV1::Present(row.record_id.clone()),
                order: "subject".to_owned(),
                ordinal: PresenceV1::Present(ordinal),
                exact_bytes: row.document.as_bytes().to_vec(),
            })
        })
        .collect()
}

fn markdown_journal_items(raw: &MarkdownRawV1) -> Result<Vec<RetainedBoundaryItem>, MappingError> {
    let mut destinations = BTreeMap::new();
    let mut journal = None;
    for node in &raw.nodes {
        let MarkdownNodeKindV1::Regular(file) = &node.node else {
            continue;
        };
        let relative = relative_text(&node.relative)?;
        if relative == "journal.jsonl" {
            journal = Some(file.bytes.as_bytes());
            continue;
        }
        let text =
            std::str::from_utf8(file.bytes.as_bytes()).map_err(|_| MappingError::Invalid {
                coordinate: relative.clone(),
            })?;
        let document = aep_backend_markdown::PlanningDocument::parse(text, Some(&relative))
            .map_err(|_| MappingError::Invalid {
                coordinate: relative.clone(),
            })?;
        let destination = markdown_entity_id(&relative, &document.frontmatter.id.to_string())?;
        destinations.insert(
            document.frontmatter.id,
            (destination.to_string(), document.frontmatter.revision),
        );
    }
    let Some(journal) = journal else {
        return Ok(Vec::new());
    };
    let mut items = Vec::new();
    for (index, raw_line) in journal.split(|byte| *byte == b'\n').enumerate() {
        let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let ordinal = u64::try_from(index).map_err(|_| MappingError::Invalid {
            coordinate: "journal.jsonl".to_owned(),
        })?;
        let (artifact, revision, evidence_kind) =
            if let Ok(event) = serde_json::from_slice::<DomainEvent>(line) {
                let artifact =
                    aep_domain::artifact::ArtifactId::new(format!("{}:{}", event.entity, event.id))
                        .map_err(|_| MappingError::Invalid {
                            coordinate: format!("journal.jsonl/{ordinal}"),
                        })?;
                (artifact, event.revision, "event")
            } else if let Ok(entry) =
                serde_json::from_slice::<aep_backend_markdown::journal::Entry>(line)
            {
                (entry.artifact, entry.revision, "change")
            } else {
                return Err(MappingError::Invalid {
                    coordinate: format!("journal.jsonl/{ordinal}"),
                });
            };
        let Some((destination_id, terminal_revision)) = destinations.get(&artifact) else {
            return Err(MappingError::MissingTerminal {
                coordinate: format!("journal.jsonl/{ordinal}/{artifact}"),
            });
        };
        if revision == 0 || revision > *terminal_revision {
            return Err(MappingError::Invalid {
                coordinate: format!("journal.jsonl/{ordinal}/revision"),
            });
        }
        items.push(RetainedBoundaryItem {
            source_locator: format!("journal.jsonl/{ordinal}"),
            destination_entity: "aep.entity".to_owned(),
            destination_id: destination_id.clone(),
            evidence_kind: evidence_kind.to_owned(),
            original_record_id: PresenceV1::Missing,
            order: "store".to_owned(),
            ordinal: PresenceV1::Present(ordinal),
            exact_bytes: line.to_vec(),
        });
    }
    Ok(items)
}

fn boundary_value(
    entity: &str,
    id: &str,
    value: &impl serde::Serialize,
) -> Result<SubjectHistory, MappingError> {
    let fields = serde_json::to_value(value)
        .map_err(|_| MappingError::Invalid {
            coordinate: format!("{entity}/{id}"),
        })?
        .as_object()
        .cloned()
        .ok_or_else(|| MappingError::Invalid {
            coordinate: format!("{entity}/{id}"),
        })?;
    boundary(
        EntityInstance {
            entity: entity.to_owned(),
            version: 1,
            id: id.to_owned(),
            lifecycle_state: "recorded".to_owned(),
            revision: 1,
            fields,
        },
        Vec::new(),
        LegacyOrderDeclaration::PerKindOnly,
    )
}

/// Maps every retained SQL row, including complete envelopes and otherwise bare events.
#[allow(clippy::too_many_lines)] // One ordered pass makes every SQL row family's coverage explicit.
pub fn sql_boundaries(
    raw: &SqlRawV1,
    source_id: &str,
) -> Result<Vec<SubjectHistory>, MappingError> {
    let mut terminals = BTreeMap::new();
    for row in &raw.instances {
        let coordinate = format!("instances/{}/{}", row.entity, row.id);
        let instance: EntityInstance =
            serde_json::from_slice(row.document.as_bytes()).map_err(|_| MappingError::Invalid {
                coordinate: coordinate.clone(),
            })?;
        if instance.entity != row.entity
            || instance.id != row.id
            || i64::try_from(instance.revision).ok() != Some(row.revision)
        {
            return Err(MappingError::Invalid { coordinate });
        }
        terminals.insert((row.entity.clone(), row.id.clone()), instance);
    }

    let mut evidence: BTreeMap<(String, String), Vec<(u64, LegacyEvidence)>> = BTreeMap::new();
    let mut covered_events = BTreeSet::<(String, String, i64)>::new();
    for row in &raw.history {
        let entry = recorded_entry(row)?;
        if entry.record_id() != row.record_id {
            return Err(MappingError::Invalid {
                coordinate: format!("history/{}/{}/{}", row.entity, row.id, row.position),
            });
        }
        if let RecordedEntry::Decision(commit) = &entry {
            for event in &commit.envelope.record.events {
                covered_events.insert((
                    event.entity.clone(),
                    event.id.clone(),
                    i64::try_from(event.revision).unwrap_or(-1),
                ));
            }
        }
        let imported = ImportedRecordEvidence::new(
            entry,
            source_id,
            format!("history/{}/{}/{}", row.entity, row.id, row.position),
            KnownLegacyOrder::Subject(nonnegative(row.position, "history.position")?),
        )
        .map_err(|_| MappingError::Invalid {
            coordinate: format!("history/{}/{}/{}", row.entity, row.id, row.position),
        })?;
        evidence
            .entry((row.entity.clone(), row.id.clone()))
            .or_default()
            .push((
                nonnegative(row.position, "history.position")?,
                LegacyEvidence::Envelope(imported),
            ));
    }
    for row in &raw.events {
        if covered_events.contains(&(row.entity.clone(), row.id.clone(), row.revision)) {
            continue;
        }
        let coordinate = format!(
            "events/{}/{}/{}/{}",
            row.entity, row.id, row.revision, row.position
        );
        let event: DomainEvent =
            serde_json::from_slice(row.document.as_bytes()).map_err(|_| MappingError::Invalid {
                coordinate: coordinate.clone(),
            })?;
        if event.entity != row.entity
            || event.id != row.id
            || i64::try_from(event.revision).ok() != Some(row.revision)
        {
            return Err(MappingError::Invalid { coordinate });
        }
        evidence
            .entry((row.entity.clone(), row.id.clone()))
            .or_default()
            .push((
                nonnegative(row.position, "events.position")?,
                LegacyEvidence::Event(event),
            ));
    }

    let mut histories = Vec::with_capacity(terminals.len());
    for (key, terminal) in terminals {
        let mut items = evidence.remove(&key).unwrap_or_default();
        items.sort_by_key(|(position, _)| *position);
        let order = if items
            .iter()
            .all(|(_, item)| matches!(item, LegacyEvidence::Envelope(_)))
        {
            LegacyOrderDeclaration::Subject
        } else {
            LegacyOrderDeclaration::PerKindOnly
        };
        histories.push(boundary(
            terminal,
            items.into_iter().map(|(_, item)| item).collect(),
            order,
        )?);
    }
    if let Some(((entity, id), _)) = evidence.into_iter().next() {
        return Err(MappingError::MissingTerminal {
            coordinate: format!("{entity}/{id}"),
        });
    }
    histories.sort_by(|left, right| left.subject.cmp(&right.subject));
    Ok(histories)
}

fn recorded_entry(row: &HistoryRowV1) -> Result<RecordedEntry, MappingError> {
    let coordinate = format!("history/{}/{}/{}", row.entity, row.id, row.position);
    match row.kind {
        HistoryKindV1::Decision => {
            serde_json::from_slice::<RecordedCommit>(row.document.as_bytes())
                .map(RecordedEntry::Decision)
                .map_err(|_| MappingError::Invalid { coordinate })
        }
        HistoryKindV1::Observation => {
            serde_json::from_slice::<RecordedObservation>(row.document.as_bytes())
                .map(RecordedEntry::Observation)
                .map_err(|_| MappingError::Invalid { coordinate })
        }
    }
}

fn boundary(
    instance: EntityInstance,
    evidence: Vec<LegacyEvidence>,
    order: LegacyOrderDeclaration,
) -> Result<SubjectHistory, MappingError> {
    let subject =
        Subject::new(&instance.entity, &instance.id).map_err(|_| MappingError::Invalid {
            coordinate: format!("{}/{}", instance.entity, instance.id),
        })?;
    Ok(SubjectHistory {
        subject,
        origin: HistoryOrigin::Imported(LegacyAnchor {
            instance,
            completeness: LegacyCompleteness::AvailableEvidenceOnly,
            order,
            evidence,
        }),
        records: Vec::new(),
    })
}

fn nonnegative(value: i64, coordinate: &str) -> Result<u64, MappingError> {
    u64::try_from(value).map_err(|_| MappingError::Invalid {
        coordinate: coordinate.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_contract::migration::{HexBytesV1, MarkdownNodeV1, RegularMarkdownNodeV1};

    fn member(name: &str) -> MemberName {
        MemberName::parse(name).expect("a member name")
    }

    fn node(relative: &str, text: &str) -> MarkdownNodeV1 {
        MarkdownNodeV1 {
            relative: HostPathV1::Unix(HexBytesV1::new(relative.as_bytes().to_vec())),
            node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(text.as_bytes().to_vec()),
            }),
        }
    }

    /// One local story and one that points into `other`, which only a workspace can declare.
    fn crossing_capture() -> MarkdownRawV1 {
        MarkdownRawV1 {
            nodes: vec![
                node(
                    "story/local.md",
                    "---\nformat: aep.planning-md/1\nid: story:local\nkind: story\nstatus: draft\n\
                     title: Local\nrelations: []\nrevision: 1\n---\n",
                ),
                node(
                    "story/crossing.md",
                    "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\n\
                     status: draft\ntitle: Crossing\nrelations:\n\
                     - informed_by: other/story:theirs\nrevision: 1\n---\n",
                ),
            ],
        }
    }

    #[test]
    fn a_relation_into_a_declared_member_is_a_crossing_and_maps() {
        let histories = markdown_boundaries_raw(&crossing_capture(), &[member("other")])
            .expect("the declaration every ordinary read command reads admits the crossing");
        assert_eq!(histories.len(), 2);
    }

    #[test]
    fn a_relation_into_a_member_nobody_declared_is_a_dangling_edge_and_is_refused() {
        // The bound this fix must not cross: a store whose relations genuinely dangle is worse
        // migrated than refused. With no declaration, and with a *different* member declared, the
        // same edge dangles and the refusal stands.
        for members in [Vec::new(), vec![member("mistyped")]] {
            let error = markdown_boundaries_raw(&crossing_capture(), &members)
                .expect_err("an undeclared member leaves the edge pointing at nothing");
            assert_eq!(
                error,
                MappingError::Invalid {
                    coordinate: "story/crossing.md".to_owned(),
                },
                "the refusal names the document that carries the edge, for members {members:?}"
            );
        }
    }

    #[test]
    fn a_graph_defect_that_names_no_document_read_falls_back_to_the_graph() {
        let orphan = MarkdownRawV1 {
            nodes: vec![node(
                "story/orphan.md",
                "---\nformat: aep.planning-md/1\nid: story:orphan\nkind: story\nstatus: draft\n\
                 title: Orphan\nrelations:\n- derived_from: epic:absent\nrevision: 1\n---\n",
            )],
        };
        // The defect is located at `artifacts.story:orphan.relations[0]`, which *is* a document
        // that was read, so it resolves; `epic:absent` is the target and never becomes the
        // coordinate.
        assert_eq!(
            markdown_boundaries_raw(&orphan, &[]).expect_err("an edge to nothing is not a graph"),
            MappingError::Invalid {
                coordinate: "story/orphan.md".to_owned(),
            }
        );
        assert_eq!(
            graph_coordinate(
                &aep_domain::ValidationErrors::new().with(aep_domain::ValidationError::new(
                    aep_domain::ValidationCode::UndeclaredReference,
                    "members.other",
                    "nothing that was read",
                )),
                &BTreeMap::new(),
            ),
            "markdown/graph",
            "a defect naming no document read still names the graph, never the selector"
        );
    }

    #[test]
    fn every_refusal_variant_can_name_its_coordinate() {
        // The accessor is exhaustive by construction — the compiler refuses a new variant that
        // does not answer — so this asserts the answer, not the enumeration.
        for error in [
            MappingError::Incomplete {
                coordinate: "story/one.md".to_owned(),
            },
            MappingError::Invalid {
                coordinate: "story/one.md".to_owned(),
            },
            MappingError::MissingTerminal {
                coordinate: "story/one.md".to_owned(),
            },
        ] {
            assert_eq!(error.coordinate(), "story/one.md", "{error}");
        }
    }
}
