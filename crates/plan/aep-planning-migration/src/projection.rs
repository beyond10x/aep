//! Staged tracked-Markdown projection from selected Eventlog authority.

#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

#[allow(clippy::wildcard_imports)]
// This publisher implements the complete closed projection vocabulary.
use aep_contract::migration::*;
use aep_contract::query::{EntityQuery, QueryService, RelationQuery};
use aep_contract::testing::block_on;
use aep_contract::QueryConsistency;
use aep_domain::artifact::{ArtifactId, ArtifactRef, ArtifactRelation};
use aep_domain::entity::EntityRef;

use crate::durable::{ProjectionError, ProjectionPublication, ProjectionPublisher};
use entity_eventlog::{Authority, EventlogOperationContext};
use time::OffsetDateTime;

const PROJECTION_OWNERSHIP_FILE: &str = ".aep-projection-ownership.json";

pub type ProjectionInventoryEntry = (String, Vec<u8>, u32);

#[cfg(unix)]
const OWNED_FILE_MODE: u32 = 0o644;

#[cfg(windows)]
const OWNED_FILE_MODE: u32 = 0;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionOwnershipV1 {
    format: String,
    authority_snapshot: AuthoritySnapshotIdV1,
    owned: Vec<ProjectionOwnedFileV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionOwnedFileV1 {
    path: String,
    digest: DigestV1,
    mode: u32,
}

pub struct FileProjectionPublisher {
    authority_path: PathBuf,
    authority: AuthorityCoordinateV1,
    projection_root: PathBuf,
    /// A complete capture of this authority the caller already holds, if it holds one.
    ///
    /// Publishing reads the authority three times — to open the store it renders from, to learn
    /// which files the authority itself captured, and to see whether this watermark is already
    /// committed — and a migration's caller has just captured that same authority to verify what
    /// it imported, with nothing written to it since. Each of those reads costs a full capture:
    /// on a 448-artifact store, 7,815 blobs and 174 MB re-read and re-hashed to learn what the
    /// verification capture already said.
    held: Option<aep_backend_eventlog::HeldCapture>,
    /// The open authority every read and write goes through, when the caller holds one.
    ///
    /// Without it each read and each watermark write opens the file authority afresh, and an
    /// open re-verifies the authority's whole history before it answers — 7–10 s apiece on the
    /// ESS store, several times per publication.
    session: Option<aep_backend_eventlog::AuthoritySession>,
}

pub struct StagedProjection {
    stage: PathBuf,
    authority_snapshot: AuthoritySnapshotIdV1,
    inventory_digest: ProjectionInventoryDigestV1,
    watermark: ProjectionWatermarkV1,
    identity: String,
    replaced_owned_paths: u64,
    preserved_foreign_paths: u64,
}

impl StagedProjection {
    pub fn inventory_digest(&self) -> ProjectionInventoryDigestV1 {
        self.inventory_digest
    }
}

impl FileProjectionPublisher {
    pub fn new(
        authority_path: PathBuf,
        authority: AuthorityCoordinateV1,
        projection_root: PathBuf,
    ) -> Self {
        Self {
            authority_path,
            authority,
            projection_root,
            held: None,
            session: None,
        }
    }

    /// [`Self::new`], reading and writing the authority through a session the caller holds.
    ///
    /// Every call still reads the authority as it is when it is made; the session saves the
    /// open, not the capture. A session of another authority than `authority` is refused by the
    /// first call that would use it.
    pub fn over_session(
        session: aep_backend_eventlog::AuthoritySession,
        authority: AuthorityCoordinateV1,
        projection_root: PathBuf,
    ) -> Self {
        Self {
            authority_path: session.path().to_path_buf(),
            authority,
            projection_root,
            held: None,
            session: Some(session),
        }
    }

    fn adapter_authority(&self) -> Authority {
        Authority {
            logical_scope: self.authority.logical_scope.as_str().to_owned(),
            tenant: self.authority.tenant.as_str().to_owned(),
            stream_identity: self.authority.stream_identity.as_str().to_owned(),
        }
    }

    /// The session, refused when it opened another authority than this publisher names.
    fn session(&self) -> Result<Option<&aep_backend_eventlog::AuthoritySession>, String> {
        match &self.session {
            Some(session) if *session.authority() != self.adapter_authority() => Err(format!(
                "the session is of authority {:?}, not this publisher's {:?}",
                session.authority(),
                self.authority
            )),
            session => Ok(session.as_ref()),
        }
    }

    /// One fresh complete capture of the authority, with its provenance.
    fn capture_held(&self) -> Result<aep_backend_eventlog::HeldCapture, String> {
        match self.session()? {
            Some(session) => session.capture_held(),
            None => {
                aep_backend_eventlog::capture_held(&self.authority_path, self.adapter_authority())
            }
        }
    }

    /// The plan to render from: seeded from `seed` when there is one, otherwise read afresh.
    fn open_plan(
        &self,
        seed: Option<aep_backend_eventlog::HeldCapture>,
    ) -> Result<aep_backend_eventlog::EventlogBackend, String> {
        match (self.session()?, seed) {
            (Some(session), Some(seed)) => session.open_backend_with_snapshot(seed),
            (Some(session), None) => session.open_backend(),
            (None, Some(seed)) => aep_backend_eventlog::open_with_snapshot(
                self.authority_path.clone(),
                self.authority.logical_scope.as_str().to_owned(),
                self.authority.tenant.as_str().to_owned(),
                self.authority.stream_identity.as_str().to_owned(),
                seed,
            ),
            (None, None) => aep_backend_eventlog::open(
                self.authority_path.clone(),
                self.authority.logical_scope.as_str().to_owned(),
                self.authority.tenant.as_str().to_owned(),
                self.authority.stream_identity.as_str().to_owned(),
            ),
        }
    }

    /// Whether the projection-metadata subject `identity` already exists, read afresh.
    fn watermark_exists(&self, identity: &str) -> Result<bool, String> {
        Ok(match self.session()? {
            Some(session) => session
                .read_control(aep_backend_eventlog::PROJECTION_METADATA_AS, identity)?
                .is_some(),
            None => aep_backend_eventlog::read_file_control(
                self.authority_path.clone(),
                self.adapter_authority(),
                aep_backend_eventlog::PROJECTION_METADATA_AS,
                identity,
            )?
            .is_some(),
        })
    }

    fn write_watermark(
        &self,
        identity: String,
        document: serde_json::Value,
        context: EventlogOperationContext,
    ) -> Result<(), String> {
        match self.session()? {
            Some(session) => session.write_control(
                aep_backend_eventlog::PROJECTION_METADATA_AS,
                identity.clone(),
                identity,
                document,
                None,
                context,
            ),
            None => aep_backend_eventlog::write_file_control(
                self.authority_path.clone(),
                self.adapter_authority(),
                aep_backend_eventlog::PROJECTION_METADATA_AS,
                identity.clone(),
                identity,
                document,
                None,
                context,
            ),
        }
        .map(|_| ())
    }

    /// [`Self::new`], publishing from a capture the caller already took of this authority.
    ///
    /// The caller asserts that nothing has been written to the authority since it took `held`.
    /// A migration can: it captures to verify its import, and between that capture and this call
    /// it renames the authority directory, writes its own phase journal and replaces the
    /// selector — none of which is a write to the authority.
    pub fn with_snapshot(
        authority_path: PathBuf,
        authority: AuthorityCoordinateV1,
        projection_root: PathBuf,
        held: aep_backend_eventlog::HeldCapture,
    ) -> Self {
        Self {
            authority_path,
            authority,
            projection_root,
            held: Some(held),
            session: None,
        }
    }

    pub fn stage(
        &self,
        authority_snapshot: AuthoritySnapshotIdV1,
    ) -> Result<StagedProjection, ProjectionError> {
        self.stage_from(authority_snapshot, self.held.clone())
    }

    /// [`Self::stage`], rendering from `seed` when it is given and from a fresh read otherwise.
    ///
    /// Staging reads the authority twice: the plan it renders, and the complete capture whose
    /// watermarks and captured files decide which existing files it may replace. A seed answers
    /// both, so a seeded stage reads nothing. What the seed says about watermarks and captured
    /// files is taken before the seed is handed to the plan, so it is not copied to be read twice.
    #[allow(clippy::too_many_lines)] // Staging keeps rendering, authority evidence, preservation, ownership, and the watermark in one ordered operation.
    fn stage_from(
        &self,
        authority_snapshot: AuthoritySnapshotIdV1,
        seed: Option<aep_backend_eventlog::HeldCapture>,
    ) -> Result<StagedProjection, ProjectionError> {
        let seeded = seed.as_ref().map(|seed| {
            (
                projection_watermarks(seed.snapshot(), &self.authority),
                captured_markdown_files(seed.snapshot()),
            )
        });
        let backend = self
            .open_plan(seed)
            .map_err(|_| ProjectionError::NotPublished)?;
        let page = block_on(backend.query(&EntityQuery {
            organisation: Some(aep_backend_markdown::backend::ORGANISATION.to_owned()),
            space: Some(aep_backend_markdown::backend::SPACE.to_owned()),
            ..EntityQuery::default()
        }))
        .map_err(|_| ProjectionError::NotPublished)?;
        let stage = self.projection_root.with_extension(format!(
            "aep-stage-{}",
            authority_snapshot.0.as_wire().trim_start_matches("sha256:")
        ));
        if stage.exists() {
            fs::remove_dir_all(&stage).map_err(|_| ProjectionError::Uncertain)?;
        }
        fs::create_dir_all(&stage).map_err(|_| ProjectionError::NotPublished)?;
        let store = aep_backend_markdown::MarkdownStore::open(&stage);
        let mut rendered = Vec::new();
        for envelope in &page.items {
            let locator = &envelope.metadata.locator;
            let Ok(id) = ArtifactId::new(format!("{}:{}", locator.kind(), locator.key())) else {
                continue;
            };
            let edges = block_on(backend.relations(&RelationQuery {
                source: Some(EntityRef::new(envelope.metadata.id.clone())),
                ..RelationQuery::default()
            }))
            .map_err(|_| ProjectionError::NotPublished)?;
            let mut relations = Vec::new();
            for relation in &edges.items {
                let target = block_on(backend.get(&relation.target, QueryConsistency::Current))
                    .map_err(|_| ProjectionError::NotPublished)?;
                let locator = &target.metadata.locator;
                if let Ok(target) = ArtifactId::new(format!("{}:{}", locator.kind(), locator.key()))
                {
                    relations.push(ArtifactRelation::new(
                        relation.kind,
                        ArtifactRef::new(target, None),
                    ));
                }
            }
            let Some(document) = aep_backend_markdown::projection::document_from_entity(
                id.clone(),
                &envelope.data,
                envelope.metadata.revision.get(),
                &relations,
            ) else {
                continue;
            };
            rendered.push((store.relative_path_for(&id), document));
        }
        write_staged_documents(&store, &stage, &rendered)?;
        let mut owned = rendered
            .into_iter()
            .map(|(relative, document)| (relative, document.render().into_bytes(), OWNED_FILE_MODE))
            .collect::<Vec<_>>();
        owned.sort_by(|left, right| left.0.cmp(&right.0));
        let (known_watermarks, captured_owned) = if let Some((known, captured)) = seeded {
            (known, captured?)
        } else {
            let complete = self
                .capture_held()
                .map_err(|_| ProjectionError::NotPublished)?;
            (
                projection_watermarks(complete.snapshot(), &self.authority),
                captured_markdown_files(complete.snapshot())?,
            )
        };
        let preserved_foreign_paths = preserve_foreign(
            &self.projection_root,
            &stage,
            &owned,
            &known_watermarks,
            &captured_owned,
        )?;
        let inventory_digest =
            projection_inventory_digest(&owned).map_err(|_| ProjectionError::NotPublished)?;
        write_projection_ownership(&stage, authority_snapshot, &owned)?;
        let watermark = ProjectionWatermarkV1 {
            format: ProjectionWatermarkFormatV1,
            authority: self.authority.clone(),
            authority_snapshot,
            projection_inventory_digest: inventory_digest,
            watermark_digest: projection_watermark_digest(authority_snapshot, inventory_digest),
        };
        let identity = format!(
            "projection-watermark-{}",
            authority_snapshot.0.as_wire().trim_start_matches("sha256:")
        );
        Ok(StagedProjection {
            stage,
            authority_snapshot,
            inventory_digest,
            watermark,
            identity,
            replaced_owned_paths: owned.len() as u64,
            preserved_foreign_paths,
        })
    }

    pub fn commit(
        &self,
        staged: StagedProjection,
    ) -> Result<ProjectionPublication, ProjectionError> {
        self.commit_knowing(
            staged,
            self.held
                .as_ref()
                .map(aep_backend_eventlog::HeldCapture::snapshot),
        )
    }

    /// [`Self::commit`], answering whether the watermark exists from `current` when given one.
    ///
    /// `current` must name every subject the authority holds now — a held capture nothing has
    /// been written after, or `publish_current`'s verifying capture, taken after staging.
    fn commit_knowing(
        &self,
        staged: StagedProjection,
        current: Option<&entity_store::asynchronous::CompleteStoreSnapshot>,
    ) -> Result<ProjectionPublication, ProjectionError> {
        // Whether this exact watermark is already committed. `read_file_control` answers it with
        // a fresh capture of the whole authority plus the subject's named reservation batch; only
        // the existence of the subject is read here, and a capture of the authority as it is now
        // names every subject the authority has.
        let recovering = match current {
            Some(current) => current.histories.iter().any(|subject| {
                subject.history.subject.entity == aep_backend_eventlog::PROJECTION_METADATA_AS
                    && subject.history.subject.id == staged.identity
            }),
            None => self
                .watermark_exists(&staged.identity)
                .map_err(|_| ProjectionError::Uncertain)?,
        };
        self.write_watermark(
            staged.identity,
            serde_json::to_value(&staged.watermark).map_err(|_| ProjectionError::NotPublished)?,
            EventlogOperationContext {
                subject: "aep-planning-projection".to_owned(),
                actor: "aep-planning-projection".to_owned(),
                request_id: format!("projection:{}", staged.authority_snapshot.0.as_wire()),
                trace_id: staged.authority_snapshot.0.as_wire(),
                causation_id: None,
                causation_depth: 0,
                occurred_at: OffsetDateTime::UNIX_EPOCH,
            },
        )
        .map_err(|_| ProjectionError::Uncertain)?;
        publish_stage(&staged.stage, &self.projection_root, recovering)?;
        Ok(ProjectionPublication {
            inventory_digest: staged.inventory_digest,
            replaced_owned_paths: staged.replaced_owned_paths,
            preserved_foreign_paths: staged.preserved_foreign_paths,
        })
    }

    /// Publishes the exact current complete authority once, or recovers the already committed
    /// watermark for it. No invocation-ledger write may follow this call.
    ///
    /// **Refused on a publisher built by [`Self::with_snapshot`].** The refusal is internal: no
    /// command reaches it, so it mints no wire vocabulary. `apply` builds its publisher with
    /// `with_snapshot` and calls [`Self::publish`]; the one production caller of this method
    /// builds its publisher with [`Self::over_session`] and rewrites any projection-failure code
    /// it gets. This publishes *the current
    /// authority* and establishes that by capturing before staging and again after and comparing
    /// the two identities; a seeded publisher stages from its held capture, between two captures
    /// that never read it, so the comparison agrees while the documents published are the ones
    /// the seed named. The result would be a watermark asserting a projection inventory for an
    /// authority snapshot whose content was not projected, which
    /// `before_projection_watermark` and `projection_watermarks` then trust. A seeded publisher
    /// publishes through [`Self::publish`] only.
    ///
    /// **This method stages from its own first capture**, which is not that hazard: the
    /// documents rendered are the first capture's, and the second capture proving the first
    /// current is exactly the proof that they are the current authority's. For the same reason
    /// the second capture answers whether the watermark already exists.
    #[allow(clippy::result_large_err, clippy::too_many_lines)] // The refusal carries its exact snapshot and typed projection diagnostic.
    pub fn publish_current(
        &self,
    ) -> Result<ProjectionPublishedV1, (AuthoritySnapshotIdV1, ProjectionFailureV1)> {
        if self.held.is_some() {
            return Err((
                zero_snapshot(),
                projection_failure(
                    CommandRefusalCodeV1::VerificationMismatch,
                    ProjectionFailureReasonV1::InventoryMismatch,
                    &self.projection_root,
                ),
            ));
        }
        let first = self.capture_held().map_err(|_| {
            (
                zero_snapshot(),
                projection_failure(
                    CommandRefusalCodeV1::SourceUnreadable,
                    ProjectionFailureReasonV1::Io,
                    &self.projection_root,
                ),
            )
        })?;
        let (current, _) =
            crate::durable::authority_snapshot_identity(&self.authority, first.snapshot())
                .map_err(|_| {
                    (
                        zero_snapshot(),
                        projection_failure(
                            CommandRefusalCodeV1::VerificationMismatch,
                            ProjectionFailureReasonV1::InventoryMismatch,
                            &self.projection_root,
                        ),
                    )
                })?;
        let mut candidates = first
            .snapshot()
            .histories
            .iter()
            .filter_map(|subject| {
                if subject.history.subject.entity != aep_backend_eventlog::PROJECTION_METADATA_AS
                    || subject.history.records.len() != 1
                {
                    return None;
                }
                let value = subject.terminal.fields.get("document")?.clone();
                let watermark = serde_json::from_value::<ProjectionWatermarkV1>(value).ok()?;
                (watermark.authority == self.authority)
                    .then_some((subject.history.records[0].receipt.position.store, watermark))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(position, _)| *position);
        // Only the newest watermark can be the one a crash left unpublished, and only while
        // nothing has been recorded after it. A watermark W names the authority as it was before
        // W was written; the authority less W still holds every record appended after W — a
        // newer watermark, or any later write — which that authority could not have held, so
        // its identity cannot match. Checking anyway cost a copy and a full transcript digest of
        // the whole authority per watermark the store had ever recorded, two more per write.
        let last_recorded = first
            .snapshot()
            .histories
            .iter()
            .flat_map(|subject| &subject.history.records)
            .map(|record| record.position.store)
            .max();
        for (position, watermark) in candidates.into_iter().rev().take(1) {
            if last_recorded != Some(position) {
                continue;
            }
            let Ok(prior) = crate::durable::before_projection_watermark(
                first.snapshot(),
                watermark.authority_snapshot,
            ) else {
                continue;
            };
            let Ok((prior_id, _)) =
                crate::durable::authority_snapshot_identity(&self.authority, &prior)
            else {
                continue;
            };
            if prior_id != watermark.authority_snapshot {
                continue;
            }
            // W may have committed just before a crash, so the filesystem can still contain the
            // prior projection. Recreate the deterministic stage from current business subjects,
            // require the exact inventory W names, and replay only W's original command.
            let staged = self
                .stage(prior_id)
                .map_err(|error| (prior_id, projection_error(error, &self.projection_root)))?;
            if staged.inventory_digest != watermark.projection_inventory_digest
                || staged.watermark.watermark_digest != watermark.watermark_digest
            {
                return Err((
                    prior_id,
                    projection_failure(
                        CommandRefusalCodeV1::ProjectionDrift,
                        ProjectionFailureReasonV1::InventoryMismatch,
                        &self.projection_root,
                    ),
                ));
            }
            self.commit(staged)
                .map_err(|error| (prior_id, projection_error(error, &self.projection_root)))?;
            return Ok(ProjectionPublishedV1 {
                authority_snapshot: prior_id,
                inventory_digest: watermark.projection_inventory_digest,
                watermark_digest: watermark.watermark_digest,
            });
        }
        let staged = self
            .stage_from(current, Some(first))
            .map_err(|error| (current, projection_error(error, &self.projection_root)))?;
        let second = self.capture_held().map_err(|_| {
            (
                current,
                projection_failure(
                    CommandRefusalCodeV1::SourceUnreadable,
                    ProjectionFailureReasonV1::Io,
                    &self.projection_root,
                ),
            )
        })?;
        let (second_id, _) =
            crate::durable::authority_snapshot_identity(&self.authority, second.snapshot())
                .map_err(|_| {
                    (
                        current,
                        projection_failure(
                            CommandRefusalCodeV1::VerificationMismatch,
                            ProjectionFailureReasonV1::InventoryMismatch,
                            &self.projection_root,
                        ),
                    )
                })?;
        if second_id != current {
            return Err((
                second_id,
                projection_failure(
                    CommandRefusalCodeV1::AuthoritySnapshotChanged,
                    ProjectionFailureReasonV1::InventoryMismatch,
                    &self.projection_root,
                ),
            ));
        }
        let inventory_digest = staged.inventory_digest;
        let watermark_digest = staged.watermark.watermark_digest;
        self.commit_knowing(staged, Some(second.snapshot()))
            .map_err(|error| (current, projection_error(error, &self.projection_root)))?;
        Ok(ProjectionPublishedV1 {
            authority_snapshot: current,
            inventory_digest,
            watermark_digest,
        })
    }
}

fn projection_error(error: ProjectionError, root: &Path) -> ProjectionFailureV1 {
    match error {
        ProjectionError::ForeignConflict => projection_failure(
            CommandRefusalCodeV1::ProjectionConflict,
            ProjectionFailureReasonV1::ForeignPath,
            root,
        ),
        ProjectionError::NotPublished => projection_failure(
            CommandRefusalCodeV1::IncompletePublication,
            ProjectionFailureReasonV1::Io,
            root,
        ),
        ProjectionError::Uncertain => projection_failure(
            CommandRefusalCodeV1::PublishUncertain,
            ProjectionFailureReasonV1::WatermarkFailure,
            root,
        ),
    }
}

fn projection_failure(
    code: CommandRefusalCodeV1,
    reason: ProjectionFailureReasonV1,
    root: &Path,
) -> ProjectionFailureV1 {
    ProjectionFailureV1 {
        code,
        at: DiagnosticCoordinateV1::Projection(PathDiagnosticV1 {
            path: host_path(root),
        }),
        reason,
    }
}

fn zero_snapshot() -> AuthoritySnapshotIdV1 {
    AuthoritySnapshotIdV1(DigestV1::from_bytes([0; 32]))
}

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

impl ProjectionPublisher for FileProjectionPublisher {
    fn publish(
        &self,
        authority_snapshot: AuthoritySnapshotIdV1,
    ) -> Result<ProjectionPublication, ProjectionError> {
        let staged = self.stage(authority_snapshot)?;
        self.commit(staged)
    }
}

/// Writes every staged document through [`aep_backend_markdown::MarkdownStore::create`], several
/// at once.
///
/// Each file is written exactly as before — to a temporary, synced, renamed, then given its owned
/// mode — and the call fails if any one of them fails. What runs concurrently is the waiting for
/// those syncs: one after another they were about 5 s of every publication of the 448 ESS
/// documents, and a write publishes twice. Two documents at one path are refused before anything
/// is written, because `create`'s own refusal of an existing file is a check-then-rename that
/// two concurrent writers of one path could both pass.
fn write_staged_documents(
    store: &aep_backend_markdown::MarkdownStore,
    stage: &Path,
    documents: &[(String, aep_backend_markdown::PlanningDocument)],
) -> Result<(), ProjectionError> {
    let mut paths = BTreeSet::new();
    if !documents
        .iter()
        .all(|(relative, _)| paths.insert(relative.as_str()))
    {
        return Err(ProjectionError::NotPublished);
    }
    let workers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .clamp(1, 8);
    let chunk = documents.len().div_ceil(workers).max(1);
    std::thread::scope(|scope| {
        let writers = documents
            .chunks(chunk)
            .map(|chunk| {
                scope.spawn(move || {
                    for (relative, document) in chunk {
                        store
                            .create(document)
                            .map_err(|_| ProjectionError::NotPublished)?;
                        set_owned_file_mode(&stage.join(relative))
                            .map_err(|_| ProjectionError::NotPublished)?;
                    }
                    Ok(())
                })
            })
            .collect::<Vec<_>>();
        writers.into_iter().try_for_each(|writer| {
            writer
                .join()
                .map_err(|_| ProjectionError::NotPublished)
                .and_then(|written| written)
        })
    })
}

fn preserve_foreign(
    current: &Path,
    stage: &Path,
    new_owned: &[(String, Vec<u8>, u32)],
    known_watermarks: &[(AuthoritySnapshotIdV1, ProjectionInventoryDigestV1)],
    captured_owned: &BTreeMap<String, Vec<u8>>,
) -> Result<u64, ProjectionError> {
    if !current.exists() {
        return Ok(0);
    }
    let old_owned =
        match read_projection_ownership_for_rebuild(current, known_watermarks, new_owned)? {
            Some(owned) => owned
                .into_iter()
                .map(|(path, _, _)| path)
                .collect::<BTreeSet<_>>(),
            None => captured_owned
                .iter()
                .filter(|(path, bytes)| {
                    fs::read(current.join(path)).ok().as_deref() == Some(bytes.as_slice())
                })
                .map(|(path, _)| path.clone())
                .collect(),
        };
    let new_owned = new_owned
        .iter()
        .map(|(path, _, _)| path.clone())
        .collect::<BTreeSet<_>>();
    let mut files = Vec::new();
    collect_files(current, current, &mut files)?;
    let mut preserved = 0;
    for (relative, source) in files {
        if relative == PROJECTION_OWNERSHIP_FILE || old_owned.contains(&relative) {
            continue;
        }
        if new_owned.contains(&relative) {
            return Err(ProjectionError::ForeignConflict);
        }
        let destination = stage.join(&relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|_| ProjectionError::NotPublished)?;
        }
        fs::copy(source, destination).map_err(|_| ProjectionError::NotPublished)?;
        preserved += 1;
    }
    let report = aep_backend_markdown::MarkdownStore::open(stage).load();
    if !report.failures.is_empty() {
        return Err(ProjectionError::ForeignConflict);
    }
    Ok(preserved)
}

fn projection_watermarks(
    snapshot: &entity_store::asynchronous::CompleteStoreSnapshot,
    authority: &AuthorityCoordinateV1,
) -> Vec<(AuthoritySnapshotIdV1, ProjectionInventoryDigestV1)> {
    snapshot
        .histories
        .iter()
        .filter_map(|subject| {
            if subject.history.subject.entity != aep_backend_eventlog::PROJECTION_METADATA_AS
                || subject.history.records.len() != 1
            {
                return None;
            }
            let watermark = serde_json::from_value::<ProjectionWatermarkV1>(
                subject.terminal.fields.get("document")?.clone(),
            )
            .ok()?;
            (watermark.authority == *authority
                && watermark.watermark_digest
                    == projection_watermark_digest(
                        watermark.authority_snapshot,
                        watermark.projection_inventory_digest,
                    ))
            .then_some((
                watermark.authority_snapshot,
                watermark.projection_inventory_digest,
            ))
        })
        .collect()
}

fn captured_markdown_files(
    snapshot: &entity_store::asynchronous::CompleteStoreSnapshot,
) -> Result<BTreeMap<String, Vec<u8>>, ProjectionError> {
    let mut owned = BTreeMap::new();
    for subject in &snapshot.histories {
        if subject.history.subject.entity != "aep.planning-import-boundary" {
            continue;
        }
        let raw = subject
            .terminal
            .fields
            .get("raw_capture")
            .cloned()
            .ok_or(ProjectionError::ForeignConflict)
            .and_then(|value| {
                serde_json::from_value::<LegacyRawCaptureV1>(value)
                    .map_err(|_| ProjectionError::ForeignConflict)
            })?;
        let markdown = match &raw {
            LegacyRawCaptureV1::Markdown(value) => Some(value),
            LegacyRawCaptureV1::Hybrid(value) => Some(&value.local),
            LegacyRawCaptureV1::Sqlite(_) | LegacyRawCaptureV1::Postgres(_) => None,
        };
        let Some(markdown) = markdown else {
            continue;
        };
        for node in &markdown.nodes {
            let MarkdownNodeKindV1::Regular(file) = &node.node else {
                continue;
            };
            let relative = relative_projection_path(&node.relative)?;
            if Path::new(&relative)
                .extension()
                .and_then(|value| value.to_str())
                != Some("md")
            {
                continue;
            }
            if owned
                .insert(relative, file.bytes.as_bytes().to_vec())
                .is_some()
            {
                return Err(ProjectionError::ForeignConflict);
            }
        }
    }
    Ok(owned)
}

fn relative_projection_path(path: &HostPathV1) -> Result<String, ProjectionError> {
    let value = match path {
        HostPathV1::Unix(bytes) => std::str::from_utf8(bytes.as_bytes())
            .map_err(|_| ProjectionError::ForeignConflict)?
            .to_owned(),
        HostPathV1::Windows(units) => {
            String::from_utf16(units).map_err(|_| ProjectionError::ForeignConflict)?
        }
    };
    let path = Path::new(&value);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ProjectionError::ForeignConflict);
    }
    Ok(value.replace('\\', "/"))
}

fn owned_file_digest(bytes: &[u8]) -> Result<DigestV1, ProjectionError> {
    digest_parts_v1("aep.planning-projection-owned-file/1", &[bytes.to_vec()])
        .map_err(|_| ProjectionError::NotPublished)
}

fn write_projection_ownership(
    stage: &Path,
    authority_snapshot: AuthoritySnapshotIdV1,
    owned: &[(String, Vec<u8>, u32)],
) -> Result<(), ProjectionError> {
    let marker = ProjectionOwnershipV1 {
        format: "aep.planning-projection-ownership/1".to_owned(),
        authority_snapshot,
        owned: owned
            .iter()
            .map(|(path, bytes, mode)| {
                Ok(ProjectionOwnedFileV1 {
                    path: path.clone(),
                    digest: owned_file_digest(bytes)?,
                    mode: *mode,
                })
            })
            .collect::<Result<Vec<_>, ProjectionError>>()?,
    };
    let mut bytes = serde_json::to_vec(&marker).map_err(|_| ProjectionError::NotPublished)?;
    bytes.push(b'\n');
    let path = stage.join(PROJECTION_OWNERSHIP_FILE);
    fs::write(&path, bytes).map_err(|_| ProjectionError::NotPublished)?;
    set_owned_file_mode(&path).map_err(|_| ProjectionError::NotPublished)
}

fn read_projection_ownership(
    current: &Path,
    known_watermarks: &[(AuthoritySnapshotIdV1, ProjectionInventoryDigestV1)],
) -> Result<Option<Vec<ProjectionInventoryEntry>>, ProjectionError> {
    let Some(marker) = read_projection_ownership_marker(current)? else {
        return Ok(None);
    };
    let (mut inventory, missing) = read_owned_files(current, &marker, false)?;
    debug_assert!(!missing, "strict ownership read refuses missing files");
    inventory.sort_by(|left, right| left.0.cmp(&right.0));
    validate_owned_inventory(marker.authority_snapshot, &inventory, known_watermarks)?;
    Ok(Some(inventory))
}

fn read_projection_ownership_for_rebuild(
    current: &Path,
    known_watermarks: &[(AuthoritySnapshotIdV1, ProjectionInventoryDigestV1)],
    staged_owned: &[ProjectionInventoryEntry],
) -> Result<Option<Vec<ProjectionInventoryEntry>>, ProjectionError> {
    let Some(mut marker) = read_projection_ownership_marker(current)? else {
        return Ok(None);
    };
    let (mut present, missing) = read_owned_files(current, &marker, true)?;
    if !missing {
        present.sort_by(|left, right| left.0.cmp(&right.0));
        validate_owned_inventory(marker.authority_snapshot, &present, known_watermarks)?;
        return Ok(Some(present));
    }

    marker
        .owned
        .sort_by(|left, right| left.path.cmp(&right.path));
    let mut staged_marker = staged_owned
        .iter()
        .map(|(path, bytes, mode)| {
            Ok(ProjectionOwnedFileV1 {
                path: path.clone(),
                digest: owned_file_digest(bytes)?,
                mode: *mode,
            })
        })
        .collect::<Result<Vec<_>, ProjectionError>>()?;
    staged_marker.sort_by(|left, right| left.path.cmp(&right.path));
    if marker.owned != staged_marker {
        return Err(ProjectionError::ForeignConflict);
    }
    let mut staged_owned = staged_owned.to_vec();
    staged_owned.sort_by(|left, right| left.0.cmp(&right.0));
    validate_owned_inventory(marker.authority_snapshot, &staged_owned, known_watermarks)?;
    Ok(Some(staged_owned))
}

fn read_projection_ownership_marker(
    current: &Path,
) -> Result<Option<ProjectionOwnershipV1>, ProjectionError> {
    let marker_path = current.join(PROJECTION_OWNERSHIP_FILE);
    let metadata = match fs::symlink_metadata(&marker_path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ProjectionError::ForeignConflict),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ProjectionError::ForeignConflict);
    }
    let marker: ProjectionOwnershipV1 = serde_json::from_slice(
        &fs::read(marker_path).map_err(|_| ProjectionError::ForeignConflict)?,
    )
    .map_err(|_| ProjectionError::ForeignConflict)?;
    if marker.format != "aep.planning-projection-ownership/1" {
        return Err(ProjectionError::ForeignConflict);
    }
    Ok(Some(marker))
}

fn read_owned_files(
    current: &Path,
    marker: &ProjectionOwnershipV1,
    allow_missing: bool,
) -> Result<(Vec<ProjectionInventoryEntry>, bool), ProjectionError> {
    let mut paths = BTreeSet::new();
    let mut inventory = Vec::with_capacity(marker.owned.len());
    let mut missing = false;
    for entry in &marker.owned {
        if relative_projection_path(&HostPathV1::Unix(HexBytesV1::new(
            entry.path.as_bytes().to_vec(),
        )))? != entry.path
            || entry.path == PROJECTION_OWNERSHIP_FILE
            || !paths.insert(entry.path.clone())
        {
            return Err(ProjectionError::ForeignConflict);
        }
        let path = current.join(&entry.path);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {
                missing = true;
                continue;
            }
            Err(_) => return Err(ProjectionError::ForeignConflict),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ProjectionError::ForeignConflict);
        }
        let bytes = fs::read(&path).map_err(|_| ProjectionError::ForeignConflict)?;
        if owned_file_digest(&bytes)? != entry.digest
            || projection_file_mode(&path).map_err(|_| ProjectionError::ForeignConflict)?
                != entry.mode
        {
            return Err(ProjectionError::ForeignConflict);
        }
        inventory.push((entry.path.clone(), bytes, entry.mode));
    }
    Ok((inventory, missing))
}

fn validate_owned_inventory(
    authority_snapshot: AuthoritySnapshotIdV1,
    inventory: &[ProjectionInventoryEntry],
    known_watermarks: &[(AuthoritySnapshotIdV1, ProjectionInventoryDigestV1)],
) -> Result<(), ProjectionError> {
    let digest =
        projection_inventory_digest(inventory).map_err(|_| ProjectionError::ForeignConflict)?;
    if !known_watermarks.contains(&(authority_snapshot, digest)) {
        return Err(ProjectionError::ForeignConflict);
    }
    Ok(())
}

pub fn projection_owned_inventory(
    root: &Path,
    snapshot: &entity_store::asynchronous::CompleteStoreSnapshot,
) -> Result<Vec<ProjectionInventoryEntry>, ProjectionError> {
    let watermarks = snapshot
        .histories
        .iter()
        .filter_map(|subject| {
            if subject.history.subject.entity != aep_backend_eventlog::PROJECTION_METADATA_AS
                || subject.history.records.len() != 1
            {
                return None;
            }
            let watermark = serde_json::from_value::<ProjectionWatermarkV1>(
                subject.terminal.fields.get("document")?.clone(),
            )
            .ok()?;
            (watermark.watermark_digest
                == projection_watermark_digest(
                    watermark.authority_snapshot,
                    watermark.projection_inventory_digest,
                ))
            .then_some((
                watermark.authority_snapshot,
                watermark.projection_inventory_digest,
            ))
        })
        .collect::<Vec<_>>();
    read_projection_ownership(root, &watermarks)?.ok_or(ProjectionError::ForeignConflict)
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), ProjectionError> {
    for entry in fs::read_dir(directory).map_err(|_| ProjectionError::ForeignConflict)? {
        let entry = entry.map_err(|_| ProjectionError::ForeignConflict)?;
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| ProjectionError::ForeignConflict)?;
        if metadata.file_type().is_symlink() {
            return Err(ProjectionError::ForeignConflict);
        }
        if metadata.is_dir() {
            collect_files(root, &entry.path(), files)?;
        } else if metadata.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| ProjectionError::ForeignConflict)?
                .to_string_lossy()
                .replace('\\', "/");
            files.push((relative, entry.path()));
        } else {
            return Err(ProjectionError::ForeignConflict);
        }
    }
    Ok(())
}

fn publish_stage(
    stage: &Path,
    destination: &Path,
    recovering: bool,
) -> Result<(), ProjectionError> {
    let backup = destination.with_extension("aep-projection-backup");
    // A committed watermark makes the filesystem operation a projection retry, never another
    // authority command. Reconcile only the exact swap states this function can create.
    if recovering && destination.exists() && trees_equal(stage, destination)? {
        fs::remove_dir_all(stage).map_err(|_| ProjectionError::Uncertain)?;
        if backup.exists() {
            fs::remove_dir_all(&backup).map_err(|_| ProjectionError::Uncertain)?;
        }
        return Ok(());
    }
    if backup.exists() {
        if !recovering || destination.exists() {
            return Err(ProjectionError::Uncertain);
        }
        fs::rename(stage, destination).map_err(|_| ProjectionError::Uncertain)?;
        sync_parent(destination)?;
        fs::remove_dir_all(backup).map_err(|_| ProjectionError::Uncertain)?;
        return Ok(());
    }
    if destination.exists() {
        fs::rename(destination, &backup).map_err(|_| ProjectionError::Uncertain)?;
    }
    if fs::rename(stage, destination).is_err() {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(ProjectionError::Uncertain);
    }
    sync_parent(destination)?;
    if backup.exists() {
        fs::remove_dir_all(backup).map_err(|_| ProjectionError::Uncertain)?;
    }
    Ok(())
}

fn sync_parent(path: &Path) -> Result<(), ProjectionError> {
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|_| ProjectionError::Uncertain)?;
    }
    Ok(())
}

fn trees_equal(left: &Path, right: &Path) -> Result<bool, ProjectionError> {
    let mut left_files = Vec::new();
    let mut right_files = Vec::new();
    collect_files(left, left, &mut left_files)?;
    collect_files(right, right, &mut right_files)?;
    left_files.sort_by(|a, b| a.0.cmp(&b.0));
    right_files.sort_by(|a, b| a.0.cmp(&b.0));
    if left_files
        .iter()
        .map(|item| &item.0)
        .ne(right_files.iter().map(|item| &item.0))
    {
        return Ok(false);
    }
    for ((_, left), (_, right)) in left_files.iter().zip(&right_files) {
        if projection_file_mode(left).map_err(|_| ProjectionError::Uncertain)?
            != projection_file_mode(right).map_err(|_| ProjectionError::Uncertain)?
        {
            return Ok(false);
        }
        let left = fs::read(left).map_err(|_| ProjectionError::Uncertain)?;
        let right = fs::read(right).map_err(|_| ProjectionError::Uncertain)?;
        if left != right {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn projection_inventory_digest(
    owned: &[ProjectionInventoryEntry],
) -> Result<ProjectionInventoryDigestV1, FrameError> {
    let parts = owned
        .iter()
        .flat_map(|(path, bytes, mode)| {
            [
                path.as_bytes().to_vec(),
                bytes.clone(),
                mode.to_be_bytes().to_vec(),
            ]
        })
        .collect::<Vec<_>>();
    Ok(ProjectionInventoryDigestV1(digest_parts_v1(
        "aep.planning-projection-inventory/1",
        &parts,
    )?))
}

#[cfg(unix)]
pub fn projection_file_mode(path: &Path) -> std::io::Result<u32> {
    use std::os::unix::fs::PermissionsExt as _;

    Ok(fs::symlink_metadata(path)?.permissions().mode() & 0o7777)
}

#[cfg(windows)]
pub fn projection_file_mode(_path: &Path) -> std::io::Result<u32> {
    Ok(OWNED_FILE_MODE)
}

#[cfg(unix)]
fn set_owned_file_mode(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    fs::set_permissions(path, fs::Permissions::from_mode(OWNED_FILE_MODE))
}

#[cfg(windows)]
fn set_owned_file_mode(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[must_use]
pub fn projection_watermark_digest(
    authority_snapshot: AuthoritySnapshotIdV1,
    inventory: ProjectionInventoryDigestV1,
) -> DigestV1 {
    digest_parts_v1(
        "aep.planning-projection-watermark/1",
        &[
            authority_snapshot.0.as_bytes().to_vec(),
            inventory.0.as_bytes().to_vec(),
        ],
    )
    .expect("two fixed digests fit canonical framing")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    use aep_backend_eventlog::counting::sites;
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity, UpdateEntity};
    use aep_domain::entity::{ActorRef, EntityLocator, EntityType};
    use aep_domain::node::Node;
    use aep_domain::time::Timestamp;

    use super::*;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        authority_path: PathBuf,
        projection_root: PathBuf,
        authority: AuthorityCoordinateV1,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "aep-projection-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            let authority_path = root.join("authority");
            let projection_root = root.join("planning");
            let tenant = "tenant-projection";
            let stream_identity = aep_backend_eventlog::prepare_file(&authority_path, tenant)
                .expect("prepare disposable file authority");
            let authority = AuthorityCoordinateV1 {
                logical_scope: AuthorityValueV1::new("planning-projection").expect("scope"),
                tenant: AuthorityValueV1::new(tenant).expect("tenant"),
                stream_identity: AuthorityValueV1::new(stream_identity).expect("stream"),
            };
            aep_backend_eventlog::provision_file(
                &authority_path,
                authority.logical_scope.as_str().to_owned(),
                authority.tenant.as_str().to_owned(),
                authority.stream_identity.as_str().to_owned(),
                EventlogOperationContext {
                    subject: "projection-test".to_owned(),
                    actor: "projection-test".to_owned(),
                    request_id: "projection-test:binding".to_owned(),
                    trace_id: "projection-test".to_owned(),
                    causation_id: None,
                    causation_depth: 0,
                    occurred_at: OffsetDateTime::UNIX_EPOCH,
                },
            )
            .expect("provision disposable authority binding");
            Self {
                root,
                authority_path,
                projection_root,
                authority,
            }
        }

        fn adapter_authority(&self) -> Authority {
            Authority {
                logical_scope: self.authority.logical_scope.as_str().to_owned(),
                tenant: self.authority.tenant.as_str().to_owned(),
                stream_identity: self.authority.stream_identity.as_str().to_owned(),
            }
        }

        fn create_story(&self) {
            let backend = aep_backend_eventlog::open(
                self.authority_path.clone(),
                self.authority.logical_scope.as_str().to_owned(),
                self.authority.tenant.as_str().to_owned(),
                self.authority.stream_identity.as_str().to_owned(),
            )
            .expect("open disposable authority");
            let command = Command::CreateEntity(CreateEntity {
                entity_type: EntityType::parse("aep.story/v1").expect("entity type"),
                locator: EntityLocator::parse("ep://planning/store/story/projected")
                    .expect("locator"),
                data: Node::Map(BTreeMap::from([
                    ("status".to_owned(), Node::from("draft")),
                    ("title".to_owned(), Node::from("Projected")),
                    ("body".to_owned(), Node::from("# Projected\n")),
                ])),
            });
            let context = CommandContext::new(
                "req-projected".parse().expect("request"),
                "key-projected".parse().expect("idempotency key"),
                ActorRef::parse("human:projection-test").expect("actor"),
                "corr-projected".parse().expect("correlation"),
                Timestamp::from_epoch_millis(1_700_000_000_000),
            );
            let envelope = CommandEnvelope::new(
                "cmd-projected".parse().expect("command"),
                command.kind().as_str(),
                command,
                context,
            );
            block_on(backend.execute(envelope)).expect("create projected story");
        }

        fn update_story_title(&self, title: &str) {
            let suffix = title.to_ascii_lowercase().replace(' ', "-");
            let backend = aep_backend_eventlog::open(
                self.authority_path.clone(),
                self.authority.logical_scope.as_str().to_owned(),
                self.authority.tenant.as_str().to_owned(),
                self.authority.stream_identity.as_str().to_owned(),
            )
            .expect("open disposable authority");
            let target = block_on(backend.resolve(
                &EntityLocator::parse("ep://planning/store/story/projected").expect("locator"),
            ))
            .expect("resolve projected story");
            let command = Command::UpdateEntity(UpdateEntity {
                target: EntityRef::new(target),
                changes: BTreeMap::from([("title".to_owned(), Node::from(title))]),
            });
            let context = CommandContext::new(
                format!("req-update-projected-{suffix}")
                    .parse()
                    .expect("request"),
                format!("key-update-projected-{suffix}")
                    .parse()
                    .expect("idempotency key"),
                ActorRef::parse("human:projection-test").expect("actor"),
                "corr-update-projected".parse().expect("correlation"),
                Timestamp::from_epoch_millis(1_700_000_000_001),
            );
            let envelope = CommandEnvelope::new(
                format!("cmd-update-projected-{suffix}")
                    .parse()
                    .expect("command"),
                command.kind().as_str(),
                command,
                context,
            );
            block_on(backend.execute(envelope)).expect("update projected story");
        }

        fn snapshot(&self) -> entity_store::asynchronous::CompleteStoreSnapshot {
            self.held().into_snapshot()
        }

        /// The same capture, with the authority it was taken from recorded alongside it.
        fn held(&self) -> aep_backend_eventlog::HeldCapture {
            aep_backend_eventlog::capture_held(&self.authority_path, self.adapter_authority())
                .expect("capture complete disposable authority")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn committed_watermark_recovers_deleted_projection_without_an_authority_write() {
        let fixture = Fixture::new();
        fixture.create_story();
        let before = fixture.snapshot();
        let (covered, _) = crate::durable::authority_snapshot_identity(&fixture.authority, &before)
            .expect("covered snapshot identity");
        let publisher = FileProjectionPublisher::new(
            fixture.authority_path.clone(),
            fixture.authority.clone(),
            fixture.projection_root.clone(),
        );

        let first = publisher.publish_current().expect("initial publication");
        assert_eq!(first.authority_snapshot, covered);
        assert!(fixture.projection_root.join("story/projected.md").is_file());

        let with_watermark = fixture.snapshot();
        let (current, _) =
            crate::durable::authority_snapshot_identity(&fixture.authority, &with_watermark)
                .expect("complete post-watermark identity");
        assert_ne!(current, covered, "the complete transcript includes W");
        let reconstructed = crate::durable::before_projection_watermark(&with_watermark, covered)
            .expect("remove only the exact newly-created W");
        let (reconstructed_id, _) =
            crate::durable::authority_snapshot_identity(&fixture.authority, &reconstructed)
                .expect("reconstructed S identity");
        assert_eq!(reconstructed_id, covered);

        fs::remove_dir_all(&fixture.projection_root).expect("simulate loss after W committed");
        let recovered = publisher
            .publish_current()
            .expect("same W recovers the filesystem publication");
        assert_eq!(recovered, first);
        assert!(fixture.projection_root.join("story/projected.md").is_file());
        let after_recovery = fixture.snapshot();
        assert_eq!(
            after_recovery, with_watermark,
            "projection recovery must not append or replace any authority record"
        );

        aep_backend_eventlog::write_file_control(
            fixture.authority_path.clone(),
            fixture.adapter_authority(),
            aep_backend_eventlog::INVOCATION_AS,
            "later-control".to_owned(),
            "later-control".to_owned(),
            serde_json::json!({"after":"covered-snapshot"}),
            None,
            EventlogOperationContext {
                subject: "projection-test".to_owned(),
                actor: "projection-test".to_owned(),
                request_id: "projection-test:later-control".to_owned(),
                trace_id: "projection-test".to_owned(),
                causation_id: None,
                causation_depth: 0,
                occurred_at: OffsetDateTime::UNIX_EPOCH,
            },
        )
        .expect("append an ordinary later authority write");
        let later = fixture.snapshot();
        let without_old_watermark = crate::durable::before_projection_watermark(&later, covered)
            .expect("the exact old W remains identifiable");
        let (stale_reconstruction, _) =
            crate::durable::authority_snapshot_identity(&fixture.authority, &without_old_watermark)
                .expect("hash the complete reconstruction containing the later write");
        assert_ne!(
            stale_reconstruction, covered,
            "removing W cannot hide any later business or control subject"
        );
    }

    /// Staging from a held capture takes no capture of the authority; staging without one takes one.
    ///
    /// This is the `complete` site, and nothing else in the suite sees it. The staged bytes come
    /// from `query` on the seeded handle, so a revert of this one site alone leaves every
    /// identity case and every byte comparison green — and the review-1 measurement puts
    /// 12.17 s → 0.27 s of a 27.3 s saving behind it. It is not a store read, so the seeded
    /// handle's own counter cannot see it either. What sees it is a counter charged where this
    /// crate opens the authority, inside `complete_file_snapshot`'s own path.
    #[test]
    fn staging_from_a_held_capture_takes_no_capture_of_the_authority() {
        let fixture = Fixture::new();
        fixture.create_story();
        let held = fixture.held();
        let (identity, _) =
            crate::durable::authority_snapshot_identity(&fixture.authority, held.snapshot())
                .expect("identity of the held authority");

        let seeded = FileProjectionPublisher::with_snapshot(
            fixture.authority_path.clone(),
            fixture.authority.clone(),
            fixture.root.join("planning-seeded"),
            held,
        );
        let _ = sites::take_authority_opens();
        seeded.stage(identity).expect("the seeded stage");
        let seeded_opens = sites::take_authority_opens();

        let fresh = FileProjectionPublisher::new(
            fixture.authority_path.clone(),
            fixture.authority.clone(),
            fixture.root.join("planning-fresh"),
        );
        let _ = sites::take_authority_opens();
        fresh.stage(identity).expect("the capture-taking stage");
        let fresh_opens = sites::take_authority_opens();

        assert_eq!(
            fresh_opens.captures, 1,
            "staging without a held capture takes exactly one capture of the authority, and this \
             number is here so that the zero below is a saving rather than an absence: {fresh_opens:?}"
        );
        assert_eq!(
            seeded_opens.captures, 0,
            "staging from a held capture takes none: {seeded_opens:?}"
        );
    }

    /// Committing from a held capture opens no bridge to ask what is already published.
    ///
    /// The `recovering` site. `read_file_control` captures the whole authority to learn whether
    /// one control subject exists, and a held capture already names every subject there is. Like
    /// the `complete` site it is invisible to every other case here: it changes no published
    /// byte, and a revert of it alone leaves the suite green. Two fixtures, because `commit`
    /// writes its watermark and the second commit would no longer be reading an unchanged one.
    #[test]
    fn committing_from_a_held_capture_opens_no_bridge_to_ask_what_is_already_published() {
        let seeded_fixture = Fixture::new();
        seeded_fixture.create_story();
        let held = seeded_fixture.held();
        let (seeded_identity, _) =
            crate::durable::authority_snapshot_identity(&seeded_fixture.authority, held.snapshot())
                .expect("identity of the held authority");
        let seeded = FileProjectionPublisher::with_snapshot(
            seeded_fixture.authority_path.clone(),
            seeded_fixture.authority.clone(),
            seeded_fixture.projection_root.clone(),
            held,
        );
        let staged = seeded.stage(seeded_identity).expect("the seeded stage");
        let _ = sites::take_authority_opens();
        seeded.commit(staged).expect("the seeded commit");
        let seeded_opens = sites::take_authority_opens();

        let fresh_fixture = Fixture::new();
        fresh_fixture.create_story();
        let (fresh_identity, _) = crate::durable::authority_snapshot_identity(
            &fresh_fixture.authority,
            &fresh_fixture.snapshot(),
        )
        .expect("identity of the fresh authority");
        let fresh = FileProjectionPublisher::new(
            fresh_fixture.authority_path.clone(),
            fresh_fixture.authority.clone(),
            fresh_fixture.projection_root.clone(),
        );
        let staged = fresh
            .stage(fresh_identity)
            .expect("the capture-taking stage");
        let _ = sites::take_authority_opens();
        fresh.commit(staged).expect("the capture-taking commit");
        let fresh_opens = sites::take_authority_opens();

        assert_eq!(
            fresh_opens.control_bridges, 2,
            "committing without a held capture opens two control bridges — one to ask whether \
             this watermark is already committed, one to write it: {fresh_opens:?}"
        );
        assert_eq!(
            seeded_opens.control_bridges, 1,
            "committing from a held capture opens only the one that writes: {seeded_opens:?}"
        );
    }

    /// Retitles the fixture's story through `backend`, as a CLI child command does.
    fn retitle(backend: &aep_backend_eventlog::EventlogBackend, identity: &str, title: &str) {
        let target = block_on(backend.resolve(
            &EntityLocator::parse("ep://planning/store/story/projected").expect("locator"),
        ))
        .expect("resolve projected story");
        let command = Command::UpdateEntity(UpdateEntity {
            target: EntityRef::new(target),
            changes: BTreeMap::from([("title".to_owned(), Node::from(title))]),
        });
        let context = CommandContext::new(
            format!("req-{identity}").parse().expect("request"),
            format!("key-{identity}").parse().expect("idempotency key"),
            ActorRef::parse("human:projection-test").expect("actor"),
            "corr-retitle".parse().expect("correlation"),
            Timestamp::from_epoch_millis(1_700_000_000_002),
        );
        let envelope = CommandEnvelope::new(
            format!("cmd-{identity}").parse().expect("command"),
            command.kind().as_str(),
            command,
            context,
        );
        block_on(backend.execute(envelope)).expect("retitle projected story");
    }

    /// One ordinary invocation opens its authority once, however often it reads and writes it.
    ///
    /// A `plan artifact move` on the 448-artifact ESS store spent about 160 s of 356 s opening
    /// the same file authority eighteen times: a control-row bridge for every ledger read and
    /// write and every watermark, a planning bridge for the plan and for each stage, and a fresh
    /// provider for every complete capture — each open re-verifying the whole history before
    /// answering anything. The command now opens one session with its plan and hands it to the
    /// ledger and the publisher. No byte a reader can see says whether they used it, so the count
    /// is the observation: after the session exists, the whole invocation — reservation, child,
    /// post-commit capture, both publications and both ledger advances — opens nothing.
    #[test]
    #[allow(clippy::result_large_err)] // The publish callback returns the publisher's typed refusal.
    fn an_ordinary_invocation_opens_its_authority_once_for_every_read_and_write() {
        let fixture = Fixture::new();
        fixture.create_story();
        let session = aep_backend_eventlog::AuthoritySession::open(
            fixture.authority_path.clone(),
            fixture.adapter_authority(),
        )
        .expect("open one session");
        let backend = session.open_backend().expect("the plan over the session");
        let publisher = FileProjectionPublisher::over_session(
            session.clone(),
            fixture.authority.clone(),
            fixture.projection_root.clone(),
        );
        let reservation = crate::mutation::reservation(
            MigrationIdV1::new("one-session").expect("identity"),
            fixture.authority.clone(),
            br#"{"verb":"set","title":"Retitled"}"#,
            &[br#"{"title":"Retitled"}"#.to_vec()],
        )
        .expect("a valid reservation");

        let _ = sites::take_authority_opens();
        let envelope = crate::mutation::execute_invocation(
            &session,
            reservation,
            |planned| {
                retitle(&backend, &planned.child_identity, "Retitled");
                let commit_receipt = backend
                    .last_commit_receipt()
                    .expect("the child's commit receipt");
                let snapshot = session
                    .complete_snapshot()
                    .expect("the post-commit capture");
                let (authority_snapshot, _) =
                    crate::durable::authority_snapshot_identity(&fixture.authority, &snapshot)
                        .expect("the committed authority's identity");
                Ok(crate::mutation::ExecutedChild {
                    commit_receipt,
                    result: PlanningMutationResultV1::FieldsSet(FieldsSetResultV1 {
                        id: "story:projected".to_owned(),
                        revision: 2,
                        fields: vec!["title".to_owned()],
                    }),
                    authority_snapshot,
                })
            },
            || publisher.publish_current(),
        )
        .expect("the invocation completes");
        let opens = sites::take_authority_opens();

        assert!(envelope.success(), "the invocation succeeds: {envelope:?}");
        assert_eq!(
            (opens.sessions, opens.captures, opens.control_bridges),
            (0, 0, 0),
            "an invocation over an open session opens no further session, fresh capture or \
             control bridge: {opens:?}"
        );
        let projected = fs::read_to_string(fixture.projection_root.join("story/projected.md"))
            .expect("the retitled story is projected");
        assert!(
            projected.contains("Retitled"),
            "the projection shows the committed title:\n{projected}"
        );
    }

    /// Publishing the current authority captures it twice: once to stage from, once to verify.
    ///
    /// `publish_current` proves what it published is the current authority by capturing before
    /// and after staging and comparing the two identities. Staging used to take two more captures
    /// of its own in between — one to open the plan it renders, one to learn which files the
    /// authority captured and which watermarks it holds — and committing a third, to ask whether
    /// the watermark already exists. Each of those is answered by the capture beside it: the
    /// stage renders from the first, which the unchanged second then proves current, and the
    /// second names every subject, the watermark included, that exists when it is written.
    #[test]
    fn publishing_through_a_session_captures_the_authority_twice_and_opens_it_never() {
        let fixture = Fixture::new();
        fixture.create_story();
        let session = aep_backend_eventlog::AuthoritySession::open(
            fixture.authority_path.clone(),
            fixture.adapter_authority(),
        )
        .expect("open one session");
        let publisher = FileProjectionPublisher::over_session(
            session,
            fixture.authority.clone(),
            fixture.projection_root.clone(),
        );

        let _ = sites::take_authority_opens();
        let publication = publisher.publish_current().expect("the publication");
        let opens = sites::take_authority_opens();

        assert_eq!(
            opens.session_reads, 2,
            "one capture to stage from and one to prove it current: {opens:?}"
        );
        assert_eq!(
            (opens.sessions, opens.captures, opens.control_bridges),
            (0, 0, 0),
            "and no open of the authority at all: {opens:?}"
        );
        let (covered, _) = crate::durable::before_projection_watermark(
            &fixture.snapshot(),
            publication.authority_snapshot,
        )
        .and_then(|prior| crate::durable::authority_snapshot_identity(&fixture.authority, &prior))
        .expect("the watermark covers the authority it was staged from");
        assert_eq!(covered, publication.authority_snapshot);
        assert!(fixture.projection_root.join("story/projected.md").is_file());
    }

    /// `publish_current` on a seeded publisher is refused, and the refusal names the seeded path.
    ///
    /// `publish_current` publishes *the current authority* and proves it did by capturing before
    /// staging and again after and comparing the two identities. Both captures are fresh; a seeded
    /// publisher stages from its held capture in between. So the comparison agrees, no refusal
    /// fires, and the watermark is written asserting a projection inventory for an authority
    /// snapshot whose content was not what was published — which `before_projection_watermark`
    /// and `projection_watermarks` then trust downstream. The types permit the combination and
    /// nothing in the repository makes it, so the refusal is what keeps it from being made.
    #[test]
    fn a_seeded_publisher_refuses_to_publish_the_current_authority() {
        let fixture = Fixture::new();
        fixture.create_story();

        let seeded = FileProjectionPublisher::with_snapshot(
            fixture.authority_path.clone(),
            fixture.authority.clone(),
            fixture.projection_root.clone(),
            fixture.held(),
        );

        let (snapshot, failure) = seeded
            .publish_current()
            .expect_err("a publisher holding a capture cannot answer for the current authority");
        assert_eq!(
            failure.code,
            CommandRefusalCodeV1::VerificationMismatch,
            "the publisher cannot verify that what it would publish is the current authority, \
             which is what this call promises; no new wire code is minted for it, because no \
             command can reach this refusal"
        );
        assert_eq!(
            failure.at,
            DiagnosticCoordinateV1::Projection(PathDiagnosticV1 {
                path: host_path(&fixture.projection_root),
            }),
            "coordinated at the projection root, as every projection failure is"
        );
        assert_eq!(
            snapshot,
            zero_snapshot(),
            "and names no authority snapshot, because it captured none"
        );
        assert!(
            !fixture.projection_root.exists(),
            "a refused publication writes nothing"
        );

        // The control: the same call on the same authority, from a publisher that holds no
        // capture, publishes. The refusal is about the seed and about nothing else.
        let fresh = FileProjectionPublisher::new(
            fixture.authority_path.clone(),
            fixture.authority.clone(),
            fixture.projection_root.clone(),
        );
        fresh
            .publish_current()
            .expect("an unseeded publisher still publishes the current authority");
    }

    /// Publishing from a capture the caller holds writes what publishing from a fresh one writes.
    ///
    /// Speed is the cheap half. `with_snapshot` reads the authority through a capture it was
    /// handed instead of three it takes, and the only thing that makes that worth doing is that
    /// the documents on disk afterwards are the same documents. Two fixtures rather than one,
    /// because the first publication writes its watermark to the authority and a second
    /// publication would no longer be reading an unchanged one.
    #[test]
    fn publishing_from_a_held_capture_writes_what_publishing_from_a_fresh_one_writes() {
        let capturing = Fixture::new();
        capturing.create_story();
        let fresh = FileProjectionPublisher::new(
            capturing.authority_path.clone(),
            capturing.authority.clone(),
            capturing.projection_root.clone(),
        );
        let (capturing_id, _) = crate::durable::authority_snapshot_identity(
            &capturing.authority,
            &capturing.snapshot(),
        )
        .expect("identity of the captured authority");
        let from_fresh = fresh.publish(capturing_id).expect("fresh publication");

        let seeded = Fixture::new();
        seeded.create_story();
        let held = seeded.held();
        let (seeded_id, _) =
            crate::durable::authority_snapshot_identity(&seeded.authority, held.snapshot())
                .expect("identity of the held authority");
        let reused = FileProjectionPublisher::with_snapshot(
            seeded.authority_path.clone(),
            seeded.authority.clone(),
            seeded.projection_root.clone(),
            held,
        );
        let from_held = reused
            .publish(seeded_id)
            .expect("publication from a held capture");

        assert_eq!(
            from_fresh.inventory_digest, from_held.inventory_digest,
            "the inventory a publication describes does not depend on where its capture came from"
        );
        assert_eq!(
            from_fresh.replaced_owned_paths, from_held.replaced_owned_paths,
            "the same owned paths are published either way"
        );
        assert_eq!(
            from_fresh.preserved_foreign_paths, from_held.preserved_foreign_paths,
            "the same foreign paths are preserved either way"
        );

        let documents = |root: &Path| {
            let mut found = BTreeMap::new();
            let mut pending = vec![root.to_path_buf()];
            while let Some(directory) = pending.pop() {
                for entry in fs::read_dir(&directory).expect("published directory") {
                    let path = entry.expect("published entry").path();
                    if path.is_dir() {
                        pending.push(path);
                    } else if path.extension().is_some_and(|kind| kind == "md") {
                        let relative = path
                            .strip_prefix(root)
                            .expect("published under the root")
                            .to_string_lossy()
                            .into_owned();
                        found.insert(relative, fs::read(&path).expect("published document"));
                    }
                }
            }
            found
        };
        let published = documents(&capturing.projection_root);
        assert!(
            !published.is_empty(),
            "the fixture publishes at least one document to compare"
        );
        assert_eq!(
            published,
            documents(&seeded.projection_root),
            "every published document is byte-identical whichever capture it was rendered from"
        );
    }

    #[test]
    fn valid_foreign_document_survives_owned_replacement_and_owned_collision_refuses() {
        let fixture = Fixture::new();
        fixture.create_story();
        let publisher = FileProjectionPublisher::new(
            fixture.authority_path.clone(),
            fixture.authority.clone(),
            fixture.projection_root.clone(),
        );
        publisher.publish_current().expect("initial publication");
        let foreign = fixture.projection_root.join("task/foreign.md");
        fs::create_dir_all(foreign.parent().expect("foreign parent")).expect("foreign parent");
        let foreign_bytes = b"---\nformat: aep.planning-md/1\nid: task:foreign\nkind: task\nstatus: draft\ntitle: Foreign\nrelations: []\nrevision: 1\n---\n";
        fs::write(&foreign, foreign_bytes).expect("valid unowned document");
        let current = aep_backend_markdown::MarkdownStore::open(&fixture.projection_root).load();
        assert!(
            current.is_clean(),
            "foreign fixture must be valid: {current:?}"
        );
        assert!(
            current
                .documents
                .contains_key(&ArtifactId::new("task:foreign").expect("foreign id")),
            "the valid foreign document must participate in the destructive parseability branch"
        );

        fixture.update_story_title("Replaced");
        publisher
            .publish_current()
            .expect("ordinary owned replacement preserves a valid foreign document");
        assert_eq!(
            fs::read(&foreign).expect("foreign document survives"),
            foreign_bytes
        );
        assert!(
            fs::read_to_string(fixture.projection_root.join("story/projected.md"))
                .expect("owned document")
                .contains("title: Replaced"),
            "owned projection was not replaced"
        );

        let ownership_path = fixture.projection_root.join(PROJECTION_OWNERSHIP_FILE);
        let ownership = fs::read(&ownership_path).expect("projection ownership marker");
        fs::remove_file(&ownership_path).expect("simulate lost local ownership metadata");
        fixture.update_story_title("Unproven replacement");
        let (_, failure) = publisher
            .publish_current()
            .expect_err("a missing ownership marker cannot make parseability prove ownership");
        assert_eq!(failure.code, CommandRefusalCodeV1::ProjectionConflict);
        assert_eq!(
            fs::read(&foreign).expect("foreign document survives missing-marker refusal"),
            foreign_bytes
        );
        fs::write(&ownership_path, ownership).expect("restore validated ownership marker");

        fs::write(
            fixture.projection_root.join("story/projected.md"),
            "---\nformat: aep.planning-md/1\nid: story:projected\nkind: story\nstatus: draft\ntitle: Foreign collision\nrelations: []\nrevision: 2\n---\n",
        )
        .expect("valid foreign collision");
        let (_, failure) = publisher
            .publish_current()
            .expect_err("a changed file at an owned path is an explicit conflict");
        assert_eq!(failure.code, CommandRefusalCodeV1::ProjectionConflict);
    }

    #[test]
    #[allow(clippy::too_many_lines)] // One matrix holds every conflict that a missing sibling must not hide.
    fn partial_recovery_authenticates_missing_paths_and_checks_every_remaining_file() {
        let root = std::env::temp_dir().join(format!(
            "aep-partial-projection-ownership-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("story")).expect("projection root");
        let owned = vec![
            (
                "story/one.md".to_owned(),
                b"owned one\n".to_vec(),
                OWNED_FILE_MODE,
            ),
            (
                "story/two.md".to_owned(),
                b"owned two\n".to_vec(),
                OWNED_FILE_MODE,
            ),
        ];
        for (path, bytes, _) in &owned {
            fs::write(root.join(path), bytes).expect("owned projection file");
            set_owned_file_mode(&root.join(path)).expect("canonical owned mode");
        }
        let snapshot = AuthoritySnapshotIdV1(DigestV1::from_bytes([7; 32]));
        write_projection_ownership(&root, snapshot, &owned).expect("ownership marker");
        let watermark = vec![(
            snapshot,
            projection_inventory_digest(&owned).expect("owned inventory digest"),
        )];

        fs::remove_file(root.join("story/two.md")).expect("one owned path is missing");
        assert_eq!(
            read_projection_ownership(&root, &watermark),
            Err(ProjectionError::ForeignConflict),
            "ordinary verification must not accept a partial projection"
        );
        assert_eq!(
            read_projection_ownership_for_rebuild(&root, &watermark, &owned),
            Ok(Some(owned.clone())),
            "the authority-bound inventory authenticates the missing owned path"
        );

        fs::write(root.join("story/one.md"), b"replaced\n").expect("replace remaining owned file");
        assert_eq!(
            read_projection_ownership_for_rebuild(&root, &watermark, &owned),
            Err(ProjectionError::ForeignConflict),
            "a missing sibling must not hide changed remaining content"
        );
        fs::write(root.join("story/one.md"), &owned[0].1).expect("restore remaining bytes");
        set_owned_file_mode(&root.join("story/one.md")).expect("restore remaining mode");

        let mut stale_owned = owned.clone();
        stale_owned[1].1 = b"new authority bytes\n".to_vec();
        assert_eq!(
            read_projection_ownership_for_rebuild(&root, &watermark, &stale_owned),
            Err(ProjectionError::ForeignConflict),
            "a stale marker cannot claim a changed authoritative inventory"
        );

        let marker_path = root.join(PROJECTION_OWNERSHIP_FILE);
        let marker_bytes = fs::read(&marker_path).expect("original marker bytes");
        let mut forged: ProjectionOwnershipV1 =
            serde_json::from_slice(&marker_bytes).expect("typed marker");
        forged.authority_snapshot = AuthoritySnapshotIdV1(DigestV1::from_bytes([8; 32]));
        fs::write(
            &marker_path,
            serde_json::to_vec(&forged).expect("forged marker serialises"),
        )
        .expect("write forged marker");
        assert_eq!(
            read_projection_ownership_for_rebuild(&root, &watermark, &owned),
            Err(ProjectionError::ForeignConflict),
            "an unauthenticated ownership marker cannot authorize recovery"
        );
        fs::write(&marker_path, marker_bytes).expect("restore ownership marker");

        #[cfg(unix)]
        {
            use std::os::unix::fs::{symlink, PermissionsExt as _};

            fs::set_permissions(root.join("story/one.md"), fs::Permissions::from_mode(0o600))
                .expect("change remaining owned mode");
            assert_eq!(
                read_projection_ownership_for_rebuild(&root, &watermark, &owned),
                Err(ProjectionError::ForeignConflict),
                "a missing sibling must not hide mode drift"
            );
            fs::set_permissions(
                root.join("story/one.md"),
                fs::Permissions::from_mode(OWNED_FILE_MODE),
            )
            .expect("restore remaining owned mode");
            fs::remove_file(root.join("story/one.md")).expect("remove remaining owned file");
            symlink("two.md", root.join("story/one.md")).expect("symlink conflict");
            assert_eq!(
                read_projection_ownership_for_rebuild(&root, &watermark, &owned),
                Err(ProjectionError::ForeignConflict),
                "a symlink at an owned path remains a conflict"
            );
            fs::remove_file(root.join("story/one.md")).expect("remove symlink conflict");
        }
        #[cfg(not(unix))]
        fs::remove_file(root.join("story/one.md")).expect("remove remaining owned file");
        fs::create_dir(root.join("story/one.md")).expect("type conflict");
        assert_eq!(
            read_projection_ownership_for_rebuild(&root, &watermark, &owned),
            Err(ProjectionError::ForeignConflict),
            "a directory at an owned path remains a conflict"
        );

        fs::remove_dir_all(root).expect("remove partial projection fixture");
    }

    #[cfg(unix)]
    #[test]
    fn recovery_tree_equality_includes_regular_file_modes() {
        use std::os::unix::fs::PermissionsExt as _;

        let root = std::env::temp_dir().join(format!(
            "aep-projection-mode-equality-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let left = root.join("left");
        let right = root.join("right");
        fs::create_dir_all(&left).expect("left tree");
        fs::create_dir_all(&right).expect("right tree");
        fs::write(left.join("same.md"), b"same bytes\n").expect("left file");
        fs::write(right.join("same.md"), b"same bytes\n").expect("right file");
        fs::set_permissions(left.join("same.md"), fs::Permissions::from_mode(0o600))
            .expect("left mode");
        fs::set_permissions(right.join("same.md"), fs::Permissions::from_mode(0o644))
            .expect("right mode");

        assert!(
            !trees_equal(&left, &right).expect("trees compare"),
            "mode-only drift must prevent recovery equality"
        );
        let _ = fs::remove_dir_all(root);
    }
}
