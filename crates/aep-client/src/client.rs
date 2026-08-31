use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;

use aep_contract::command::{CommandEnvelope, CommandResult, CommandService};
use aep_contract::error::{CommandError, QueryError};
use aep_contract::query::{
    AuditQuery, EntityEnvelope, EntityQuery, Page, QueryService, Relation, RelationQuery,
    RevisionRecord,
};
use aep_contract::registry::TypeDescriptor;
use aep_contract::QueryConsistency;
use aep_domain::audit::AuditRecord;
use aep_domain::capability::Capability;
use aep_domain::command::Command;
use aep_domain::entity::{EntityId, EntityLocator, EntityRef, EntityRevision, EntityType};
use aep_domain::error::ValidationErrors;
use aep_domain::ids::{CommandId, IdempotencyKey};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::wire::{
    self, AuditPageV1, AuditQueryV1, CommandRequestV1, CommandResultV1, EntityPageV1,
    EntityQueryV1, HistoryV1, Method, ProblemDocumentV1, RelationPageV1, RelationQueryV1, Request,
    ResolveRequestV1, Response, SuccessV1, TypeDescriptionV1, CONSISTENCY_HEADER, MEDIA_TYPE_V1,
    SUPPORTED_VERSIONS_HEADER,
};

/// A bearer credential whose diagnostics never expose its bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct BearerToken(String);

impl BearerToken {
    /// Validates a token for use in an HTTP `Authorization` header.
    pub fn new(value: impl Into<String>) -> Result<Self, ClientConfigurationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ClientConfigurationError::Credential(
                "a bearer token must not be empty".to_owned(),
            ));
        }
        if value.len() > 8_192 {
            return Err(ClientConfigurationError::Credential(
                "a bearer token must be at most 8192 bytes".to_owned(),
            ));
        }
        if value.chars().any(char::is_whitespace) || value.chars().any(char::is_control) {
            return Err(ClientConfigurationError::Credential(
                "a bearer token must not contain whitespace or control characters".to_owned(),
            ));
        }
        Ok(Self(value))
    }

    fn authorization_value(&self) -> String {
        format!("Bearer {}", self.0)
    }
}

impl fmt::Debug for BearerToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BearerToken([REDACTED])")
    }
}

/// Why client configuration could not be accepted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClientConfigurationError {
    /// A realm or workspace was not a usable opaque coordinate.
    #[error("invalid {kind} coordinate: {reason}")]
    Coordinate {
        /// Which coordinate.
        kind: &'static str,
        /// Why it was refused.
        reason: String,
    },
    /// A bearer credential was unsafe to place in a header.
    #[error("invalid bearer credential: {0}")]
    Credential(String),
}

/// A credential-provider failure, classified by whether retrying may help.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("could not obtain a credential: {reason}")]
pub struct CredentialError {
    reason: String,
    retryable: bool,
}

impl CredentialError {
    /// A permanent absence or rejection of credentials.
    pub fn unauthenticated(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            retryable: false,
        }
    }

    /// A temporary credential-source outage.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            retryable: true,
        }
    }

    /// Whether retrying without changing intent may succeed later.
    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }
}

/// Supplies short-lived credentials without giving the client their source.
pub trait CredentialProvider {
    /// Obtains a credential for one exchange.
    fn credential(&self) -> impl Future<Output = Result<BearerToken, CredentialError>>;
}

/// A failure before any HTTP response was received.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the AEP service transport did not answer: {reason}")]
pub struct TransportError {
    reason: String,
}

impl TransportError {
    /// Builds a safe transport diagnostic.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// Executes an HTTP-shaped exchange without choosing an HTTP or async runtime.
pub trait Transport {
    /// Sends one request and returns an answered response or a no-response failure.
    fn send(&self, request: Request) -> impl Future<Output = Result<Response, TransportError>>;
}

/// The official AEP service client.
///
/// It implements the semantic traits directly. Trusted attribution fields present on an in-process
/// command envelope are deliberately not serialized: the service reconstructs them from the
/// credential, ingress request and server clock.
pub struct AepClient<T, C> {
    transport: T,
    credentials: C,
    base_path: String,
}

impl<T, C> fmt::Debug for AepClient<T, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AepClient")
            .field("base_path", &self.base_path)
            .finish_non_exhaustive()
    }
}

impl<T, C> AepClient<T, C> {
    /// Configures one realm and workspace.
    pub fn new(
        transport: T,
        credentials: C,
        realm: &str,
        workspace: &str,
    ) -> Result<Self, ClientConfigurationError> {
        let realm = coordinate("realm", realm)?;
        let workspace = coordinate("workspace", workspace)?;
        Ok(Self {
            transport,
            credentials,
            base_path: format!("/aep/v1/realms/{realm}/workspaces/{workspace}"),
        })
    }

    /// Returns the configured realm/workspace base path.
    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// Recovers the injected components.
    pub fn into_parts(self) -> (T, C) {
        (self.transport, self.credentials)
    }
}

fn coordinate(kind: &'static str, value: &str) -> Result<String, ClientConfigurationError> {
    if value.is_empty() {
        return Err(ClientConfigurationError::Coordinate {
            kind,
            reason: "must not be empty".to_owned(),
        });
    }
    if value.len() > 200 {
        return Err(ClientConfigurationError::Coordinate {
            kind,
            reason: "must be at most 200 bytes".to_owned(),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(ClientConfigurationError::Coordinate {
            kind,
            reason: "must not contain control characters".to_owned(),
        });
    }
    Ok(percent_encode_segment(value))
}

fn percent_encode_segment(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

#[derive(Debug)]
enum ExchangeError {
    Credential(CredentialError),
    Transport(TransportError),
    Protocol(String),
    Problem {
        status: u16,
        problem: wire::ProblemV1,
    },
    UnsupportedVersion(String),
}

impl<T: Transport, C: CredentialProvider> AepClient<T, C> {
    async fn post<B: serde::Serialize, R: DeserializeOwned>(
        &self,
        suffix: &str,
        body: &B,
    ) -> Result<R, ExchangeError> {
        let body =
            wire::encode(body).map_err(|error| ExchangeError::Protocol(error.to_string()))?;
        self.exchange(Method::Post, suffix, body, None).await
    }

    async fn get<R: DeserializeOwned>(
        &self,
        suffix: &str,
        consistency: Option<&str>,
    ) -> Result<R, ExchangeError> {
        self.exchange(Method::Get, suffix, Vec::new(), consistency)
            .await
    }

    async fn exchange<R: DeserializeOwned>(
        &self,
        method: Method,
        suffix: &str,
        body: Vec<u8>,
        consistency: Option<&str>,
    ) -> Result<R, ExchangeError> {
        let token = self
            .credentials
            .credential()
            .await
            .map_err(ExchangeError::Credential)?;
        let mut headers = BTreeMap::from([
            ("Accept".to_owned(), MEDIA_TYPE_V1.to_owned()),
            ("Authorization".to_owned(), token.authorization_value()),
        ]);
        if !body.is_empty() {
            headers.insert("Content-Type".to_owned(), MEDIA_TYPE_V1.to_owned());
        }
        if let Some(token) = consistency {
            headers.insert(CONSISTENCY_HEADER.to_owned(), token.to_owned());
        }
        let response = self
            .transport
            .send(Request {
                method,
                path: format!("{}{}", self.base_path, suffix),
                headers,
                body,
            })
            .await
            .map_err(ExchangeError::Transport)?;
        decode_response(&response)
    }
}

fn decode_response<R: DeserializeOwned>(response: &Response) -> Result<R, ExchangeError> {
    if response.status == 406 && response.body.is_empty() {
        return Err(ExchangeError::UnsupportedVersion(
            response
                .header(SUPPORTED_VERSIONS_HEADER)
                .unwrap_or("none advertised")
                .to_owned(),
        ));
    }
    if response.body.is_empty() {
        return Err(ExchangeError::Protocol(format!(
            "status {} carried no response document",
            response.status
        )));
    }
    if response.header("Content-Type") != Some(MEDIA_TYPE_V1) {
        return Err(ExchangeError::Protocol(format!(
            "status {} did not select media type {MEDIA_TYPE_V1}",
            response.status
        )));
    }
    let varies_on_accept = response.header("Vary").is_some_and(|value| {
        value
            .split(',')
            .any(|name| name.trim().eq_ignore_ascii_case("Accept"))
    });
    if !varies_on_accept {
        return Err(ExchangeError::Protocol(
            "the response does not vary on Accept".to_owned(),
        ));
    }
    if response.status == 200 {
        let success: SuccessV1<R> = wire::decode(&response.body)
            .map_err(|error| ExchangeError::Protocol(error.to_string()))?;
        return Ok(success.result);
    }
    let document: ProblemDocumentV1 =
        wire::decode(&response.body).map_err(|error| ExchangeError::Protocol(error.to_string()))?;
    if document.error.retryable != (document.error.code == "unavailable") {
        return Err(ExchangeError::Protocol(format!(
            "problem code {} carried an invalid retryable value",
            document.error.code
        )));
    }
    Err(ExchangeError::Problem {
        status: response.status,
        problem: document.error,
    })
}

fn details<T: DeserializeOwned>(problem: &wire::ProblemV1) -> Result<T, String> {
    serde_json::from_value(problem.details.clone())
        .map_err(|error| format!("problem {} carried invalid details: {error}", problem.code))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReasonDetails {
    reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandUnauthorisedDetails {
    capability: wire::Nullable<Capability>,
    reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryUnauthorisedDetails {
    #[serde(rename = "capability")]
    _capability: wire::Nullable<Capability>,
    reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionConflictDetails {
    entity: EntityRef,
    expected: EntityRevision,
    actual: EntityRevision,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandNotFoundDetails {
    entity: EntityRef,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InvalidCommandDetails {
    errors: ValidationErrors,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdempotencyMismatchDetails {
    key: IdempotencyKey,
    original: CommandId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnsupportedCommandDetails {
    command_type: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryNotFoundDetails {
    what: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsistencyTimeoutDetails {
    token: String,
}

fn protocol_command(reason: impl Into<String>) -> CommandError {
    CommandError::Unavailable {
        reason: reason.into(),
    }
}

fn protocol_query(reason: impl Into<String>) -> QueryError {
    QueryError::Unavailable {
        reason: reason.into(),
    }
}

fn command_error(error: ExchangeError) -> CommandError {
    match error {
        ExchangeError::Credential(error) if error.is_retryable() => {
            protocol_command(error.to_string())
        }
        ExchangeError::Credential(error) => CommandError::Unauthorised {
            capability: None,
            reason: error.to_string(),
        },
        ExchangeError::Transport(error) => protocol_command(error.to_string()),
        ExchangeError::Protocol(reason) => protocol_command(reason),
        ExchangeError::UnsupportedVersion(supported) => CommandError::Unsupported {
            command_type: format!("AEP service wire version 1; server supports {supported}"),
        },
        ExchangeError::Problem { status, problem } => {
            map_command_problem(status, &problem).unwrap_or_else(protocol_command)
        }
    }
}

fn map_command_problem(status: u16, problem: &wire::ProblemV1) -> Result<CommandError, String> {
    match (status, problem.code.as_str()) {
        (409, "revision_conflict") => {
            let value: RevisionConflictDetails = details(problem)?;
            Ok(CommandError::RevisionConflict {
                entity: value.entity,
                expected: value.expected,
                actual: value.actual,
            })
        }
        (404, "not_found") => {
            let value: CommandNotFoundDetails = details(problem)?;
            Ok(CommandError::NotFound {
                entity: value.entity,
            })
        }
        (401, "unauthenticated") => {
            let value: ReasonDetails = details(problem)?;
            Ok(CommandError::Unauthorised {
                capability: None,
                reason: value.reason,
            })
        }
        (403, "unauthorized" | "unauthorised") => {
            let value: CommandUnauthorisedDetails = details(problem)?;
            Ok(CommandError::Unauthorised {
                capability: value.capability.into_option(),
                reason: value.reason,
            })
        }
        (400, "invalid") => {
            let value: InvalidCommandDetails = details(problem)?;
            Ok(CommandError::Invalid {
                errors: value.errors,
            })
        }
        (409, "idempotency_mismatch") => {
            let value: IdempotencyMismatchDetails = details(problem)?;
            Ok(CommandError::IdempotencyMismatch {
                key: value.key,
                original: value.original,
            })
        }
        (409, "conflict") => {
            let value: ReasonDetails = details(problem)?;
            Ok(CommandError::Conflict {
                reason: value.reason,
            })
        }
        (422, "unsupported") => {
            let value: UnsupportedCommandDetails = details(problem)?;
            Ok(CommandError::Unsupported {
                command_type: value.command_type,
            })
        }
        (406, "unsupported_version") => Ok(CommandError::Unsupported {
            command_type: "AEP service wire version 1".to_owned(),
        }),
        (503, "unavailable") => {
            let value: ReasonDetails = details(problem)?;
            Ok(CommandError::Unavailable {
                reason: value.reason,
            })
        }
        _ => Err(format!(
            "status {status} and problem code {} are not a version-1 command failure",
            problem.code
        )),
    }
}

fn query_error(error: ExchangeError) -> QueryError {
    match error {
        ExchangeError::Credential(error) if error.is_retryable() => {
            protocol_query(error.to_string())
        }
        ExchangeError::Credential(error) => QueryError::Unauthorised {
            reason: error.to_string(),
        },
        ExchangeError::Transport(error) => protocol_query(error.to_string()),
        ExchangeError::Protocol(reason) => protocol_query(reason),
        ExchangeError::UnsupportedVersion(supported) => QueryError::Invalid {
            reason: format!(
                "server does not support AEP service wire version 1; supports {supported}"
            ),
        },
        ExchangeError::Problem { status, problem } => {
            map_query_problem(status, &problem).unwrap_or_else(protocol_query)
        }
    }
}

fn map_query_problem(status: u16, problem: &wire::ProblemV1) -> Result<QueryError, String> {
    match (status, problem.code.as_str()) {
        (404, "not_found") => {
            let value: QueryNotFoundDetails = details(problem)?;
            Ok(QueryError::NotFound { what: value.what })
        }
        (400, "invalid") => {
            let value: ReasonDetails = details(problem)?;
            Ok(QueryError::Invalid {
                reason: value.reason,
            })
        }
        (401, "unauthenticated") => {
            let value: ReasonDetails = details(problem)?;
            Ok(QueryError::Unauthorised {
                reason: value.reason,
            })
        }
        (403, "unauthorized" | "unauthorised") => {
            let value: QueryUnauthorisedDetails = details(problem)?;
            Ok(QueryError::Unauthorised {
                reason: value.reason,
            })
        }
        (504, "consistency_timeout") => {
            let value: ConsistencyTimeoutDetails = details(problem)?;
            Ok(QueryError::ConsistencyTimeout { token: value.token })
        }
        (406, "unsupported_version") => Ok(QueryError::Invalid {
            reason: "server does not support AEP service wire version 1".to_owned(),
        }),
        (503, "unavailable") => {
            let value: ReasonDetails = details(problem)?;
            Ok(QueryError::Unavailable {
                reason: value.reason,
            })
        }
        _ => Err(format!(
            "status {status} and problem code {} are not a version-1 query failure",
            problem.code
        )),
    }
}

impl<T: Transport, C: CredentialProvider> CommandService for AepClient<T, C> {
    type Command = Command;

    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        if envelope.command_type != envelope.payload.kind().as_str() {
            let mut errors = ValidationErrors::new();
            errors.push(aep_domain::error::ValidationError::new(
                aep_domain::error::ValidationCode::TypeMismatch,
                "command_type",
                format!(
                    "command type `{}` does not name payload `{}`",
                    envelope.command_type,
                    envelope.payload.kind().as_str()
                ),
            ));
            return Err(CommandError::Invalid { errors });
        }
        let request = CommandRequestV1::from_envelope(envelope)
            .map_err(|error| protocol_command(error.to_string()))?;
        let result: CommandResultV1 = self
            .post("/commands", &request)
            .await
            .map_err(command_error)?;
        Ok(result.into())
    }
}

impl<T: Transport, C: CredentialProvider> QueryService for AepClient<T, C> {
    type AuditRecord = AuditRecord;

    async fn get(
        &self,
        reference: &EntityRef,
        consistency: QueryConsistency,
    ) -> Result<EntityEnvelope, QueryError> {
        let suffix = format!(
            "/entities/{}",
            percent_encode_segment(reference.id.as_str())
        );
        self.get(
            &suffix,
            consistency.token().map(ToString::to_string).as_deref(),
        )
        .await
        .map_err(query_error)
    }

    async fn resolve(&self, locator: &EntityLocator) -> Result<EntityId, QueryError> {
        self.post(
            "/entities/resolve",
            &ResolveRequestV1 {
                locator: locator.clone(),
            },
        )
        .await
        .map_err(query_error)
    }

    async fn query(&self, query: &EntityQuery) -> Result<Page<EntityEnvelope>, QueryError> {
        let page: EntityPageV1 = self
            .post("/entities/query", &EntityQueryV1::from(query))
            .await
            .map_err(query_error)?;
        Ok(page.into())
    }

    async fn relations(&self, query: &RelationQuery) -> Result<Page<Relation>, QueryError> {
        let page: RelationPageV1 = self
            .post("/relations/query", &RelationQueryV1::from(query))
            .await
            .map_err(query_error)?;
        Ok(page.into())
    }

    async fn history(&self, reference: &EntityRef) -> Result<Vec<RevisionRecord>, QueryError> {
        let suffix = format!(
            "/entities/{}/history",
            percent_encode_segment(reference.id.as_str())
        );
        let history: HistoryV1 = self.get(&suffix, None).await.map_err(query_error)?;
        Ok(history)
    }

    async fn audit(&self, query: &AuditQuery) -> Result<Page<Self::AuditRecord>, QueryError> {
        let page: AuditPageV1 = self
            .post("/audit/query", &AuditQueryV1::from(query))
            .await
            .map_err(query_error)?;
        Ok(page.into())
    }

    async fn describe_type(&self, entity_type: &EntityType) -> Result<TypeDescriptor, QueryError> {
        let suffix = format!(
            "/types/{}",
            percent_encode_segment(&entity_type.to_string())
        );
        let description: TypeDescriptionV1 = self.get(&suffix, None).await.map_err(query_error)?;
        Ok(description)
    }
}
