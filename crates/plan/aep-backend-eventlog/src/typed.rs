//! Planning artifacts recorded as typed Entity Runtime entities, on a store version control merges.
//!
//! An artifact of a kind the project's lifecycles declare is recorded as an instance of that kind's
//! entity type ([`aep_backend_entity::definition::storage_definition`]): its content as typed
//! fields, its status as the entity's state, a content change as `edit`, a status change as the
//! ladder's operation for the target status. The contract record AEP reads back — its packed
//! metadata and the events of each decision — rides in the entity's `document` field, so history,
//! audit and hydration read it exactly as they read the one-field form.
//!
//! Every other record — a contract entity of a type no lifecycle declares, relations, audit,
//! applied commands — keeps the one-field form it had.
//!
//! The store is an `eventlog-tree` directory: one file per event and per group, so branches that
//! wrote different artifacts merge, and one both wrote is reported as forked until a merge joins it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aep_backend_entity::definition::{storage_definition, DOCUMENT, EDIT};
use aep_backend_entity::METADATA_KEY;
use aep_domain::artifact::{ArtifactKind, LifecycleRegistry};
use entity_core::{EntityInstance, Registry};
use entity_eventlog::sync::{BridgeConfig, EventlogRecordedStoreOwner, RecordedEventlogBridge};
use entity_eventlog::{
    AsyncBindingProvisioner, Authority, ErRecordedProjector, EventlogBindingProvisioner,
    EventlogOperationContext,
};
use eventlog_core::{EventStore, InlineProjectionAdmin};
use serde_json::{Map, Value};

use crate::{ProvisionedBinding, CAPTURE_LIMITS};

/// The typed content keys an artifact body carries, beside `status` and the packed metadata.
const CONTENT: [&str; 10] = [
    "title",
    "summary",
    "owner",
    "withholds",
    "model_digest",
    "body",
    "tags",
    "relations",
    "refs",
    "scope",
];

/// The kinds a store records typed, with the ladder each is held to.
#[derive(Debug, Clone)]
pub struct TypedKinds {
    pub(crate) lifecycles: BTreeMap<String, aep_domain::artifact::ArtifactLifecycle>,
}

impl TypedKinds {
    /// Every kind `lifecycles` declares.
    #[must_use]
    pub fn of(lifecycles: &LifecycleRegistry) -> Self {
        Self {
            lifecycles: lifecycles
                .iter()
                .map(|(kind, lifecycle)| (kind.as_str().to_owned(), lifecycle.clone()))
                .collect(),
        }
    }

    pub(crate) fn contains(&self, kind: &str) -> bool {
        self.lifecycles.contains_key(kind)
    }

    pub(crate) fn initial(&self, kind: &str) -> Option<String> {
        self.lifecycles
            .get(kind)
            .map(|lifecycle| lifecycle.initial.as_str().to_owned())
    }
}

/// The artifact kind a packed contract entity names in its metadata's `type`: `aep.<kind>/v1` → `<kind>`.
pub(crate) fn kind_of(fields: &Map<String, Value>) -> Option<String> {
    let entity_type = fields
        .get(METADATA_KEY)?
        .get("metadata")?
        .get("type")?
        .as_str()?;
    entity_type
        .strip_prefix("aep.")?
        .split('/')
        .next()
        .filter(|kind| !kind.is_empty())
        .map(str::to_owned)
}

/// The typed fields for a packed body: each content key as it is, every other key under `extra`,
/// and the contract record under `document`.
pub(crate) fn typed_fields(fields: &Map<String, Value>, document: Value) -> Map<String, Value> {
    let mut typed = Map::new();
    let mut extra = Map::new();
    for (key, value) in fields {
        if key == METADATA_KEY || key == "status" {
            continue;
        }
        if CONTENT.contains(&key.as_str()) {
            typed.insert(key.clone(), value.clone());
        } else {
            extra.insert(key.clone(), value.clone());
        }
    }
    if !extra.is_empty() {
        typed.insert("extra".to_owned(), Value::Object(extra));
    }
    typed.insert(DOCUMENT.to_owned(), document);
    typed
}

/// The operation that replaces an artifact's content, for callers building actions.
pub(crate) const EDIT_OPERATION: &str = EDIT;

/// A typed instance as the contract layer reads it: under the contract entity type, so `unpack`
/// takes its revision from AEP's own metadata.
pub(crate) fn as_contract(mut instance: EntityInstance) -> EntityInstance {
    aep_backend_entity::STORED_AS.clone_into(&mut instance.entity);
    instance
}

/// Every entity type a typed store holds: each kind's, and the one-field record types.
pub(crate) fn typed_registry(kinds: &TypedKinds) -> Result<Registry, String> {
    let mut registry = crate::registry()?;
    for (kind, lifecycle) in &kinds.lifecycles {
        let parsed = ArtifactKind::parse(kind).map_err(|error| error.to_string())?;
        let definition = storage_definition(Some(&parsed), lifecycle)
            .map_err(|error| format!("building the {kind} entity type: {error}"))?;
        registry
            .register(definition)
            .map_err(|error| format!("registering the {kind} entity type: {error}"))?;
    }
    Ok(registry)
}

/// The backend over a typed tree store: the same type as the file store's, so every caller of an
/// Eventlog plan opens either.
pub type TreeBackend = crate::EventlogBackend;

/// Prepares a tree store at `path` and returns the stream identity it gave `tenant`.
///
/// # Errors
/// The directory cannot hold a tree store, or the projection cannot be admitted.
pub fn prepare_tree(path: &Path, tenant: &str) -> Result<String, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| format!("constructing the tree preparation runtime: {error}"))?;
    runtime.block_on(async {
        let concrete = eventlog_tree::TreeEventStore::open(path)
            .await
            .map_err(|error| format!("opening the tree store: {error}"))?;
        concrete
            .create_projections(Arc::new(ErRecordedProjector::new()))
            .await
            .map_err(|error| format!("admitting the recorded projection: {error}"))?;
        let tenant = eventlog_core::TenantId::new(tenant.to_owned())
            .map_err(|error| format!("invalid tenant: {error}"))?;
        concrete
            .stream_identity(&tenant)
            .await
            .map_err(|error| format!("reading the stream identity: {error}"))
    })
}

/// Provisions or recovers the immutable binding of a tree store.
///
/// # Errors
/// The store cannot be opened, the identity is not the one it holds, or the binding conflicts.
pub fn provision_tree(
    path: &Path,
    logical_scope: String,
    tenant: String,
    stream_identity: String,
    context: EventlogOperationContext,
) -> Result<ProvisionedBinding, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| format!("constructing the tree provisioning runtime: {error}"))?;
    runtime.block_on(async {
        let concrete = Arc::new(
            eventlog_tree::TreeEventStore::open(path)
                .await
                .map_err(|error| format!("opening the tree store: {error}"))?,
        );
        let projector = Arc::new(ErRecordedProjector::new());
        concrete
            .create_projections(projector.clone())
            .await
            .map_err(|error| format!("admitting the recorded projection: {error}"))?;
        concrete
            .attach_inline_existing(projector)
            .await
            .map_err(|error| format!("attaching the recorded projection: {error}"))?;
        let tenant_id = eventlog_core::TenantId::new(tenant.clone())
            .map_err(|error| format!("invalid tenant: {error}"))?;
        let actual = concrete
            .stream_identity(&tenant_id)
            .await
            .map_err(|error| format!("reading the stream identity: {error}"))?;
        if actual != stream_identity {
            return Err(format!(
                "the tree store's stream identity is {actual}, not the requested {stream_identity}"
            ));
        }
        let backend: Arc<dyn entity_eventlog::EventlogBackend> = concrete;
        let outcome = EventlogBindingProvisioner::new(backend, CAPTURE_LIMITS)
            .provision_binding(
                Authority {
                    logical_scope,
                    tenant,
                    stream_identity,
                },
                context,
            )
            .await
            .map_err(|error| format!("provisioning the binding: {error:?}"))?;
        Ok(ProvisionedBinding {
            event_id: outcome.physical.event_id,
            global_seq: outcome.physical.global_seq,
            stream_id: outcome.physical.stream_id,
            stream_version: outcome.physical.stream_version,
            replayed: outcome.replayed,
        })
    })
}

/// Opens a provisioned tree store, recording `lifecycles`' kinds typed.
///
/// # Errors
/// A kind's ladder is not one the pinned kernel reads, or the store refuses to open.
pub fn open_tree(
    path: PathBuf,
    logical_scope: String,
    tenant: String,
    stream_identity: String,
    lifecycles: LifecycleRegistry,
) -> Result<TreeBackend, String> {
    let authority = Authority {
        logical_scope,
        tenant,
        stream_identity,
    };
    let kinds = TypedKinds::of(&lifecycles);
    let path_of_tree = path.clone();
    let bridge = RecordedEventlogBridge::start(
        typed_registry(&kinds)?,
        EventlogRecordedStoreOwner::Tree {
            path,
            authority: authority.clone(),
            limits: CAPTURE_LIMITS,
        },
        BridgeConfig {
            queue_capacity: std::num::NonZeroU16::new(32).expect("nonzero queue"),
        },
    )
    .map_err(|error| format!("opening the tree store: {error:?}"))?;
    crate::AuthoritySession::over_bridge(path_of_tree, authority, bridge, (kinds, lifecycles))
        .open_backend()
}

/// The operation context a provisioning command records its binding under.
#[must_use]
pub fn provisioning_context(request: &str) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "aep".into(),
        actor: "aep".into(),
        request_id: request.to_owned(),
        trace_id: request.to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::now_utc(),
    }
}
