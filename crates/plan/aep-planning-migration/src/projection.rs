//! Staged tracked-Markdown projection from selected Eventlog authority.

#![allow(missing_docs)]

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

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

pub struct FileProjectionPublisher {
    authority_path: PathBuf,
    authority: AuthorityCoordinateV1,
    projection_root: PathBuf,
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
        }
    }

    pub fn stage(
        &self,
        authority_snapshot: AuthoritySnapshotIdV1,
    ) -> Result<StagedProjection, ProjectionError> {
        let backend = aep_backend_eventlog::open(
            self.authority_path.clone(),
            self.authority.logical_scope.as_str().to_owned(),
            self.authority.tenant.as_str().to_owned(),
            self.authority.stream_identity.as_str().to_owned(),
        )
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
        let mut owned = Vec::new();
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
            let relative = store.relative_path_for(&id);
            let bytes = document.render().into_bytes();
            store
                .create(&document)
                .map_err(|_| ProjectionError::NotPublished)?;
            owned.push((relative, bytes));
        }
        owned.sort_by(|left, right| left.0.cmp(&right.0));
        let preserved_foreign_paths = preserve_foreign(&self.projection_root, &stage, &owned)?;
        let parts = owned
            .iter()
            .flat_map(|(path, bytes)| [path.as_bytes().to_vec(), bytes.clone()])
            .collect::<Vec<_>>();
        let inventory_digest = ProjectionInventoryDigestV1(
            digest_parts_v1("aep.planning-projection-inventory/1", &parts)
                .map_err(|_| ProjectionError::NotPublished)?,
        );
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
        let authority = Authority {
            logical_scope: self.authority.logical_scope.as_str().to_owned(),
            tenant: self.authority.tenant.as_str().to_owned(),
            stream_identity: self.authority.stream_identity.as_str().to_owned(),
        };
        let recovering = aep_backend_eventlog::read_file_control(
            self.authority_path.clone(),
            authority.clone(),
            aep_backend_eventlog::PROJECTION_METADATA_AS,
            &staged.identity,
        )
        .map_err(|_| ProjectionError::Uncertain)?
        .is_some();
        aep_backend_eventlog::write_file_control(
            self.authority_path.clone(),
            authority,
            aep_backend_eventlog::PROJECTION_METADATA_AS,
            staged.identity.clone(),
            staged.identity,
            serde_json::to_value(&staged.watermark).map_err(|_| ProjectionError::NotPublished)?,
            None,
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
    #[allow(clippy::result_large_err, clippy::too_many_lines)] // The refusal carries its exact snapshot and typed projection diagnostic.
    pub fn publish_current(
        &self,
    ) -> Result<ProjectionPublishedV1, (AuthoritySnapshotIdV1, ProjectionFailureV1)> {
        let adapter = || Authority {
            logical_scope: self.authority.logical_scope.as_str().to_owned(),
            tenant: self.authority.tenant.as_str().to_owned(),
            stream_identity: self.authority.stream_identity.as_str().to_owned(),
        };
        let first = aep_backend_eventlog::complete_file_snapshot(&self.authority_path, adapter())
            .map_err(|_| {
            (
                zero_snapshot(),
                projection_failure(
                    CommandRefusalCodeV1::SourceUnreadable,
                    ProjectionFailureReasonV1::Io,
                    &self.projection_root,
                ),
            )
        })?;
        let (current, _) = crate::durable::authority_snapshot_identity(&self.authority, &first)
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
        for (_, watermark) in candidates.into_iter().rev() {
            let Ok(prior) =
                crate::durable::before_projection_watermark(&first, watermark.authority_snapshot)
            else {
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
            .stage(current)
            .map_err(|error| (current, projection_error(error, &self.projection_root)))?;
        let second = aep_backend_eventlog::complete_file_snapshot(&self.authority_path, adapter())
            .map_err(|_| {
                (
                    current,
                    projection_failure(
                        CommandRefusalCodeV1::SourceUnreadable,
                        ProjectionFailureReasonV1::Io,
                        &self.projection_root,
                    ),
                )
            })?;
        let (second_id, _) = crate::durable::authority_snapshot_identity(&self.authority, &second)
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
        self.commit(staged)
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

fn preserve_foreign(
    current: &Path,
    stage: &Path,
    new_owned: &[(String, Vec<u8>)],
) -> Result<u64, ProjectionError> {
    if !current.exists() {
        return Ok(0);
    }
    let report = aep_backend_markdown::MarkdownStore::open(current).load();
    if !report.failures.is_empty() {
        return Err(ProjectionError::ForeignConflict);
    }
    let old_owned = report
        .documents
        .values()
        .map(|stored| stored.relative_path.clone())
        .collect::<BTreeSet<_>>();
    let new_owned = new_owned
        .iter()
        .map(|(path, _)| path.clone())
        .collect::<BTreeSet<_>>();
    let mut files = Vec::new();
    collect_files(current, current, &mut files)?;
    let mut preserved = 0;
    for (relative, source) in files {
        if old_owned.contains(&relative) || new_owned.contains(&relative) {
            continue;
        }
        let destination = stage.join(&relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|_| ProjectionError::NotPublished)?;
        }
        fs::copy(source, destination).map_err(|_| ProjectionError::NotPublished)?;
        preserved += 1;
    }
    Ok(preserved)
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
        let left = fs::read(left).map_err(|_| ProjectionError::Uncertain)?;
        let right = fs::read(right).map_err(|_| ProjectionError::Uncertain)?;
        if left != right {
            return Ok(false);
        }
    }
    Ok(true)
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

    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity};
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

        fn snapshot(&self) -> entity_store::asynchronous::CompleteStoreSnapshot {
            aep_backend_eventlog::complete_file_snapshot(
                &self.authority_path,
                self.adapter_authority(),
            )
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
}
