//! Strict version-1 service documents and the injected transport exchange shape.

use std::collections::BTreeMap;

use aep_contract::command::{CausationRef, CommandEnvelope, CommandOutcome, CommandResult};
use aep_contract::error::{CommandError, QueryError};
use aep_contract::query::{AuditQuery, Cursor, EntityQuery, Page, RelationQuery, RevisionRecord};
use aep_contract::{ConsistencyToken, QueryConsistency, TypeDescriptor};
use aep_domain::artifact::RelationKind;
use aep_domain::audit::AuditRecord;
use aep_domain::command::Command;
use aep_domain::entity::{
    ActorRef, EntityId, EntityLocator, EntityRef, EntityRevision, EntityType, VersionedEntityRef,
};
use aep_domain::ids::{
    AuditId, CommandId, CorrelationId, EventId, ExecutionId, IdempotencyKey, RequestId, TaskId,
};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec, SubschemaValidation};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The one media type for version-1 request and response documents.
pub const MEDIA_TYPE_V1: &str = "application/vnd.aep.service+json;version=1";
/// The media type for bounded-history version-2 documents.
pub const MEDIA_TYPE_V2: &str = "application/vnd.aep.service+json;version=2";
/// Header advertising the response versions a server currently serves.
pub const SUPPORTED_VERSIONS_HEADER: &str = "AEP-Supported-Versions";
/// Header carrying a consistency token on a single-entity read.
pub const CONSISTENCY_HEADER: &str = "AEP-Consistency";

/// The route prefix shared by every semantic AEP service operation.
pub const SERVICE_PATH_PREFIX: &str = "/aep/v1/realms/{realm}/workspaces/{workspace}";

/// A semantic operation projected onto the versioned HTTP wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operation {
    /// Execute one command.
    Command,
    /// Read one entity by identity.
    GetEntity,
    /// Resolve a logical locator.
    ResolveEntity,
    /// Query entities.
    QueryEntities,
    /// Query relations.
    QueryRelations,
    /// Read complete legacy history.
    GetHistory,
    /// Query bounded history.
    QueryHistory,
    /// Query audit records.
    QueryAudit,
    /// Describe one entity type.
    DescribeType,
}

impl Operation {
    /// Returns the stable `OpenAPI` operation identifier.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Command => "executeCommand",
            Self::GetEntity => "getEntity",
            Self::ResolveEntity => "resolveEntity",
            Self::QueryEntities => "queryEntities",
            Self::QueryRelations => "queryRelations",
            Self::GetHistory => "getEntityHistory",
            Self::QueryHistory => "queryEntityHistory",
            Self::QueryAudit => "queryAudit",
            Self::DescribeType => "describeEntityType",
        }
    }
}

/// One operation's method, path template and exact document media type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSpec {
    /// The semantic operation.
    pub operation: Operation,
    /// The HTTP method.
    pub method: Method,
    /// The suffix appended to [`SERVICE_PATH_PREFIX`].
    pub suffix: &'static str,
    /// The media type accepted and returned by this operation.
    pub media_type: &'static str,
}

impl RouteSpec {
    /// Returns the complete templated path used by contract projections.
    pub fn path(self) -> String {
        format!("{SERVICE_PATH_PREFIX}{}", self.suffix)
    }
}

/// Every semantic route published by the current service wire.
pub const ROUTES: &[RouteSpec] = &[
    RouteSpec {
        operation: Operation::Command,
        method: Method::Post,
        suffix: "/commands",
        media_type: MEDIA_TYPE_V1,
    },
    RouteSpec {
        operation: Operation::GetEntity,
        method: Method::Get,
        suffix: "/entities/{entity}",
        media_type: MEDIA_TYPE_V1,
    },
    RouteSpec {
        operation: Operation::ResolveEntity,
        method: Method::Post,
        suffix: "/entities/resolve",
        media_type: MEDIA_TYPE_V1,
    },
    RouteSpec {
        operation: Operation::QueryEntities,
        method: Method::Post,
        suffix: "/entities/query",
        media_type: MEDIA_TYPE_V1,
    },
    RouteSpec {
        operation: Operation::QueryRelations,
        method: Method::Post,
        suffix: "/relations/query",
        media_type: MEDIA_TYPE_V1,
    },
    RouteSpec {
        operation: Operation::GetHistory,
        method: Method::Get,
        suffix: "/entities/{entity}/history",
        media_type: MEDIA_TYPE_V1,
    },
    RouteSpec {
        operation: Operation::QueryHistory,
        method: Method::Post,
        suffix: "/history/query",
        media_type: MEDIA_TYPE_V2,
    },
    RouteSpec {
        operation: Operation::QueryAudit,
        method: Method::Post,
        suffix: "/audit/query",
        media_type: MEDIA_TYPE_V1,
    },
    RouteSpec {
        operation: Operation::DescribeType,
        method: Method::Get,
        suffix: "/types/{entity_type}",
        media_type: MEDIA_TYPE_V1,
    },
];

/// Finds the contract entry for one semantic operation.
pub fn route(operation: Operation) -> &'static RouteSpec {
    ROUTES
        .iter()
        .find(|route| route.operation == operation)
        .expect("every operation has exactly one route")
}

/// An HTTP method the AEP wire uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// A read whose address contains its complete question.
    Get,
    /// A command or structured query carrying a JSON document.
    Post,
}

impl Method {
    /// Returns the uppercase HTTP token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// One transport request, independent of an HTTP implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The method.
    pub method: Method,
    /// The absolute path, beginning with `/`.
    pub path: String,
    /// Request headers by name.
    pub headers: BTreeMap<String, String>,
    /// Canonical JSON bytes, or empty for a bodyless request.
    pub body: Vec<u8>,
}

/// One transport response, independent of an HTTP implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status code.
    pub status: u16,
    /// Response headers by name.
    pub headers: BTreeMap<String, String>,
    /// Response body bytes.
    pub body: Vec<u8>,
}

impl Response {
    /// Returns a header without treating its HTTP name as case-sensitive.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// A required member whose written value may be JSON `null`.
///
/// A field of this type is itself non-optional to Serde, so omitting the member is malformed while
/// writing `null` produces `Nullable(None)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Nullable<T>(pub Option<T>);

impl<T: JsonSchema> JsonSchema for Nullable<T> {
    fn schema_name() -> String {
        format!("Nullable_{}", T::schema_name())
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let null = SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Null))),
            ..SchemaObject::default()
        };
        SchemaObject {
            subschemas: Some(Box::new(SubschemaValidation {
                any_of: Some(vec![T::json_schema(generator), Schema::Object(null)]),
                ..SubschemaValidation::default()
            })),
            ..SchemaObject::default()
        }
        .into()
    }

    fn is_referenceable() -> bool {
        false
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if value.is_null() {
            return Ok(Self(None));
        }
        serde_json::from_value(value)
            .map(|value| Self(Some(value)))
            .map_err(serde::de::Error::custom)
    }
}

impl<T> Nullable<T> {
    /// Wraps an optional value while retaining mandatory-member wire semantics.
    pub const fn new(value: Option<T>) -> Self {
        Self(value)
    }

    /// Returns the optional value.
    pub fn into_option(self) -> Option<T> {
        self.0
    }

    /// Borrows the optional value.
    pub const fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }
}

impl<T> From<Option<T>> for Nullable<T> {
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

/// A version-1 command request before the server adds trusted context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandRequestV1 {
    /// The logical command identity.
    pub command_id: CommandId,
    /// The authority-scoped idempotency identity.
    pub idempotency_key: IdempotencyKey,
    /// The versioned semantic command name.
    pub command_type: String,
    /// The command target, explicitly null when absent.
    #[schemars(required)]
    pub target: Nullable<EntityRef>,
    /// The asserted revision, explicitly null when absent.
    #[schemars(required)]
    pub expected_revision: Nullable<EntityRevision>,
    /// The wider activity.
    pub correlation_id: CorrelationId,
    /// The direct cause, explicitly null when absent.
    #[schemars(required)]
    pub causation: Nullable<CausationRef>,
    /// The protocol execution, explicitly null when absent.
    #[schemars(required)]
    pub execution_id: Nullable<ExecutionId>,
    /// The governed task, explicitly null when absent.
    #[schemars(required)]
    pub task: Nullable<TaskId>,
    /// The raw semantic command document.
    #[schemars(with = "Command")]
    pub payload: Value,
}

impl CommandRequestV1 {
    /// Removes trusted context from an in-process envelope and produces its raw request document.
    pub fn from_envelope(envelope: CommandEnvelope<Command>) -> Result<Self, DocumentError> {
        Ok(Self {
            command_id: envelope.command_id,
            idempotency_key: envelope.context.idempotency_key,
            command_type: envelope.command_type,
            target: envelope.target.into(),
            expected_revision: envelope.expected_revision.into(),
            correlation_id: envelope.context.correlation_id,
            causation: envelope.context.causation.into(),
            execution_id: envelope.context.execution_id.into(),
            task: envelope.context.task.into(),
            payload: serde_json::to_value(envelope.payload)
                .map_err(|error| DocumentError::Encode(error.to_string()))?,
        })
    }

    /// Decodes the semantic payload and verifies that `command_type` names it.
    pub fn decode_command(&self) -> Result<Command, DocumentError> {
        let command: Command = serde_json::from_value(self.payload.clone())
            .map_err(|error| DocumentError::Decode(error.to_string()))?;
        if self.command_type != command.kind().as_str() {
            return Err(DocumentError::Decode(format!(
                "command type `{}` does not name payload `{}`",
                self.command_type,
                command.kind().as_str()
            )));
        }
        Ok(command)
    }
}

/// A command result whose empty collections are still written on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandResultV1 {
    /// The logical command identity.
    pub command_id: CommandId,
    /// Whether this was the first application, a replay or a no-op.
    pub outcome: CommandOutcome,
    /// Entities affected by the command.
    pub affected: Vec<VersionedEntityRef>,
    /// Events emitted by the command.
    pub events: Vec<EventId>,
    /// Audit records emitted by the command.
    pub audit: Vec<AuditId>,
    /// Token a subsequent read may demand.
    pub consistency: ConsistencyToken,
}

impl From<CommandResultV1> for CommandResult {
    fn from(value: CommandResultV1) -> Self {
        Self {
            command_id: value.command_id,
            outcome: value.outcome,
            affected: value.affected,
            events: value.events,
            audit: value.audit,
            consistency: value.consistency,
        }
    }
}

impl From<CommandResult> for CommandResultV1 {
    fn from(value: CommandResult) -> Self {
        Self {
            command_id: value.command_id,
            outcome: value.outcome,
            affected: value.affected,
            events: value.events,
            audit: value.audit,
            consistency: value.consistency,
        }
    }
}

/// A logical-address resolution request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveRequestV1 {
    /// The logical entity address.
    pub locator: EntityLocator,
}

/// A strict version-2 bounded history request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HistoryQueryV2 {
    /// Entity whose revisions are requested.
    pub entity: EntityRef,
    /// Maximum records, explicitly null for the server default.
    #[schemars(required)]
    pub limit: Nullable<usize>,
    /// Continuation, explicitly null for the first page.
    #[schemars(required)]
    pub after: Nullable<Cursor>,
    /// Required consistency demand.
    pub consistency: QueryConsistency,
}

impl From<&aep_contract::query::HistoryQuery> for HistoryQueryV2 {
    fn from(value: &aep_contract::query::HistoryQuery) -> Self {
        Self {
            entity: value.entity.clone(),
            limit: value.limit.into(),
            after: value.after.clone().into(),
            consistency: value.consistency.clone(),
        }
    }
}

impl From<HistoryQueryV2> for aep_contract::query::HistoryQuery {
    fn from(value: HistoryQueryV2) -> Self {
        Self {
            entity: value.entity,
            limit: value.limit.into_option(),
            after: value.after.into_option(),
            consistency: value.consistency,
        }
    }
}

/// A strict version-1 entity query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntityQueryV1 {
    /// Optional type filter.
    #[schemars(required)]
    pub entity_type: Nullable<EntityType>,
    /// Optional organisation filter.
    #[schemars(required)]
    pub organisation: Nullable<String>,
    /// Optional space filter.
    #[schemars(required)]
    pub space: Nullable<String>,
    /// Exact body-field filters.
    pub matching: BTreeMap<String, Node>,
    /// Optional related entity.
    #[schemars(required)]
    pub related_to: Nullable<EntityRef>,
    /// Optional relation kind.
    #[schemars(required)]
    pub relation: Nullable<RelationKind>,
    /// Optional page size.
    #[schemars(required)]
    pub limit: Nullable<usize>,
    /// Optional page cursor.
    #[schemars(required)]
    pub after: Nullable<Cursor>,
    /// Required consistency requirement.
    pub consistency: QueryConsistency,
}

impl From<&EntityQuery> for EntityQueryV1 {
    fn from(value: &EntityQuery) -> Self {
        Self {
            entity_type: value.entity_type.clone().into(),
            organisation: value.organisation.clone().into(),
            space: value.space.clone().into(),
            matching: value.matching.clone(),
            related_to: value.related_to.clone().into(),
            relation: value.relation.into(),
            limit: value.limit.into(),
            after: value.after.clone().into(),
            consistency: value.consistency.clone(),
        }
    }
}

impl From<EntityQueryV1> for EntityQuery {
    fn from(value: EntityQueryV1) -> Self {
        Self {
            entity_type: value.entity_type.into_option(),
            organisation: value.organisation.into_option(),
            space: value.space.into_option(),
            matching: value.matching,
            related_to: value.related_to.into_option(),
            relation: value.relation.into_option(),
            limit: value.limit.into_option(),
            after: value.after.into_option(),
            consistency: value.consistency,
        }
    }
}

/// A strict version-1 relation query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelationQueryV1 {
    /// Optional source filter.
    #[schemars(required)]
    pub source: Nullable<EntityRef>,
    /// Optional target filter.
    #[schemars(required)]
    pub target: Nullable<EntityRef>,
    /// Optional relation-kind filter.
    #[schemars(required)]
    pub kind: Nullable<RelationKind>,
    /// Optional page size.
    #[schemars(required)]
    pub limit: Nullable<usize>,
    /// Optional page cursor.
    #[schemars(required)]
    pub after: Nullable<Cursor>,
    /// Required consistency requirement.
    pub consistency: QueryConsistency,
}

impl From<&RelationQuery> for RelationQueryV1 {
    fn from(value: &RelationQuery) -> Self {
        Self {
            source: value.source.clone().into(),
            target: value.target.clone().into(),
            kind: value.kind.into(),
            limit: value.limit.into(),
            after: value.after.clone().into(),
            consistency: value.consistency.clone(),
        }
    }
}

impl From<RelationQueryV1> for RelationQuery {
    fn from(value: RelationQueryV1) -> Self {
        Self {
            source: value.source.into_option(),
            target: value.target.into_option(),
            kind: value.kind.into_option(),
            limit: value.limit.into_option(),
            after: value.after.into_option(),
            consistency: value.consistency,
        }
    }
}

/// A strict version-1 audit query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuditQueryV1 {
    /// Optional subject filter.
    #[schemars(required)]
    pub entity: Nullable<EntityRef>,
    /// Optional correlation filter.
    #[schemars(required)]
    pub correlation_id: Nullable<CorrelationId>,
    /// Optional command filter.
    #[schemars(required)]
    pub command_id: Nullable<CommandId>,
    /// Optional authority filter.
    #[schemars(required)]
    pub actor: Nullable<ActorRef>,
    /// Optional audit-kind filter.
    #[schemars(required)]
    pub kind: Nullable<String>,
    /// Optional lower time bound.
    #[schemars(required)]
    pub since: Nullable<Timestamp>,
    /// Optional upper time bound.
    #[schemars(required)]
    pub until: Nullable<Timestamp>,
    /// Whether only refusals are returned.
    pub rejected_only: bool,
    /// Optional page size.
    #[schemars(required)]
    pub limit: Nullable<usize>,
    /// Optional page cursor.
    #[schemars(required)]
    pub after: Nullable<Cursor>,
}

impl From<&AuditQuery> for AuditQueryV1 {
    fn from(value: &AuditQuery) -> Self {
        Self {
            entity: value.entity.clone().into(),
            correlation_id: value.correlation_id.clone().into(),
            command_id: value.command_id.clone().into(),
            actor: value.actor.clone().into(),
            kind: value.kind.clone().into(),
            since: value.since.into(),
            until: value.until.into(),
            rejected_only: value.rejected_only,
            limit: value.limit.into(),
            after: value.after.clone().into(),
        }
    }
}

impl From<AuditQueryV1> for AuditQuery {
    fn from(value: AuditQueryV1) -> Self {
        Self {
            entity: value.entity.into_option(),
            correlation_id: value.correlation_id.into_option(),
            command_id: value.command_id.into_option(),
            actor: value.actor.into_option(),
            kind: value.kind.into_option(),
            since: value.since.into_option(),
            until: value.until.into_option(),
            rejected_only: value.rejected_only,
            limit: value.limit.into_option(),
            after: value.after.into_option(),
        }
    }
}

/// A page whose nullable continuation member is always present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageV1<T> {
    /// Page items.
    pub items: Vec<T>,
    /// Continuation cursor, explicitly null on the last page.
    #[schemars(required)]
    pub next: Nullable<Cursor>,
}

/// A strict version-2 page. The shape is named separately so later versions never reinterpret v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageV2<T> {
    /// Page items.
    pub items: Vec<T>,
    /// Continuation, explicitly null when complete.
    #[schemars(required)]
    pub next: Nullable<Cursor>,
}

impl<T> From<Page<T>> for PageV2<T> {
    fn from(value: Page<T>) -> Self {
        Self {
            items: value.items,
            next: value.next.into(),
        }
    }
}

impl<T> From<PageV2<T>> for Page<T> {
    fn from(value: PageV2<T>) -> Self {
        Self {
            items: value.items,
            next: value.next.into_option(),
        }
    }
}

impl<T> From<PageV1<T>> for Page<T> {
    fn from(value: PageV1<T>) -> Self {
        Self {
            items: value.items,
            next: value.next.into_option(),
        }
    }
}

impl<T> From<Page<T>> for PageV1<T> {
    fn from(value: Page<T>) -> Self {
        Self {
            items: value.items,
            next: value.next.into(),
        }
    }
}

/// A successful answered request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SuccessV1<T> {
    /// Server-derived transport attempt identity.
    pub request_id: RequestId,
    /// The semantic result.
    pub result: T,
}

/// A stable failure document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProblemDocumentV1 {
    /// Server-derived transport attempt identity.
    pub request_id: RequestId,
    /// The typed problem.
    pub error: ProblemV1,
}

/// A stable failure and safe structured details selected by its code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProblemV1 {
    /// Stable machine-readable code.
    pub code: String,
    /// Diagnostic text, not a machine contract.
    pub message: String,
    /// Whether retrying unchanged intent may succeed later.
    pub retryable: bool,
    /// Code-specific safe details, validated when mapped to the semantic error.
    pub details: Value,
}

/// A semantic or trust-boundary problem paired with its version-1 HTTP status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProblemMappingV1 {
    /// HTTP status to return.
    pub status: u16,
    /// Version-1 problem body.
    pub problem: ProblemV1,
}

impl ProblemMappingV1 {
    /// Maps a command failure onto the version-1 status and safe details.
    pub fn command(error: &CommandError) -> Self {
        match error {
            CommandError::RevisionConflict {
                entity,
                expected,
                actual,
            } => Self::new(
                409,
                "revision_conflict",
                "the entity changed since the expected revision",
                false,
                serde_json::json!({
                    "actual": actual,
                    "entity": entity,
                    "expected": expected,
                }),
            ),
            CommandError::NotFound { entity } => Self::new(
                404,
                "not_found",
                "the entity was not found",
                false,
                serde_json::json!({ "entity": entity }),
            ),
            CommandError::Unauthorised { capability, reason } => Self::new(
                403,
                "unauthorized",
                "the operation is not permitted",
                false,
                serde_json::json!({ "capability": capability, "reason": reason }),
            ),
            CommandError::Invalid { errors } => Self::new(
                400,
                "invalid",
                "the command is invalid",
                false,
                serde_json::json!({ "errors": errors }),
            ),
            CommandError::IdempotencyMismatch { key, original } => Self::new(
                409,
                "idempotency_mismatch",
                "the idempotency key names different intent",
                false,
                serde_json::json!({ "key": key, "original": original }),
            ),
            CommandError::Conflict { reason } => Self::new(
                409,
                "conflict",
                "the command conflicts with current state",
                false,
                serde_json::json!({ "reason": reason }),
            ),
            CommandError::Unsupported { command_type } => Self::new(
                422,
                "unsupported",
                "the command type is not supported",
                false,
                serde_json::json!({ "command_type": command_type }),
            ),
            CommandError::Unavailable { reason } => Self::new(
                503,
                "unavailable",
                "the service cannot answer now",
                true,
                serde_json::json!({ "reason": reason }),
            ),
            _ => Self::new(
                503,
                "unavailable",
                "this wire version cannot represent the command failure",
                true,
                serde_json::json!({ "reason": "unmapped command failure" }),
            ),
        }
    }

    /// Maps a query failure onto the version-1 status and safe details.
    pub fn query(error: &QueryError) -> Self {
        match error {
            QueryError::NotFound { what } => Self::new(
                404,
                "not_found",
                "the requested entity was not found",
                false,
                serde_json::json!({ "what": what }),
            ),
            QueryError::Invalid { reason } => Self::new(
                400,
                "invalid",
                "the query is invalid",
                false,
                serde_json::json!({ "reason": reason }),
            ),
            QueryError::Unauthorised { reason } => Self::new(
                403,
                "unauthorized",
                "the query is not permitted",
                false,
                serde_json::json!({ "reason": reason }),
            ),
            QueryError::ConsistencyTimeout { token } => Self::new(
                504,
                "consistency_timeout",
                "the requested consistency was not reached in time",
                false,
                serde_json::json!({ "token": token }),
            ),
            QueryError::Unavailable { reason } => Self::new(
                503,
                "unavailable",
                "the service cannot answer now",
                true,
                serde_json::json!({ "reason": reason }),
            ),
            _ => Self::new(
                503,
                "unavailable",
                "this wire version cannot represent the query failure",
                true,
                serde_json::json!({ "reason": "unmapped query failure" }),
            ),
        }
    }

    /// A missing or invalid credential, before semantic dispatch.
    pub fn unauthenticated(reason: impl Into<String>) -> Self {
        Self::new(
            401,
            "unauthenticated",
            "a valid credential is required",
            false,
            serde_json::json!({ "reason": reason.into() }),
        )
    }

    /// An insufficient realm or workspace grant, before semantic dispatch.
    pub fn unauthorized(reason: impl Into<String>) -> Self {
        Self::new(
            403,
            "unauthorized",
            "workspace access is required",
            false,
            serde_json::json!({ "capability": null, "reason": reason.into() }),
        )
    }

    fn new(
        status: u16,
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
        details: Value,
    ) -> Self {
        Self {
            status,
            problem: ProblemV1 {
                code: code.into(),
                message: message.into(),
                retryable,
                details,
            },
        }
    }
}

/// A document the wire could not encode or decode.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DocumentError {
    /// A local semantic value could not be represented as JSON.
    #[error("could not encode the AEP wire document: {0}")]
    Encode(String),
    /// Received bytes were not the strict expected document.
    #[error("could not decode the AEP wire document: {0}")]
    Decode(String),
}

/// Serializes a canonical compact JSON document with one trailing line feed.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, DocumentError> {
    let mut bytes =
        serde_json::to_vec(value).map_err(|error| DocumentError::Encode(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Deserializes one strict JSON document.
pub fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, DocumentError> {
    serde_json::from_slice(bytes).map_err(|error| DocumentError::Decode(error.to_string()))
}

// Keep these result types reachable from rustdoc's module index: the aliases are the concrete
// response payloads used by the client and service, rather than a second model.
/// Version-1 entity-query page payload.
pub type EntityPageV1 = PageV1<aep_contract::query::EntityEnvelope>;
/// Version-1 relation-query page payload.
pub type RelationPageV1 = PageV1<aep_contract::query::Relation>;
/// Version-1 audit-query page payload.
pub type AuditPageV1 = PageV1<AuditRecord>;
/// Version-1 entity-history payload.
pub type HistoryV1 = Vec<RevisionRecord>;
/// Version-1 type-description payload.
pub type TypeDescriptionV1 = TypeDescriptor;
/// Version-1 resolved identity payload.
pub type ResolvedEntityV1 = EntityId;
