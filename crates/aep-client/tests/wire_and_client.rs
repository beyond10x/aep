//! Version-1 wire and official-client boundary tests.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use aep_client::wire::{self, CommandRequestV1, Method, Nullable, Response, MEDIA_TYPE_V1};
use aep_client::{
    AepClient, BearerToken, CredentialError, CredentialProvider, Transport, TransportError,
};
use aep_contract::command::{CommandContext, CommandEnvelope, CommandResult, CommandService};
use aep_contract::query::{AuditQuery, EntityQuery, QueryService, RelationQuery};
use aep_contract::testing::block_on;
use aep_contract::{CommandError, ConsistencyToken, QueryConsistency, QueryError};
use aep_domain::command::{Command, MoveStatus};
use aep_domain::entity::{EntityRef, EntityRevision};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use serde_json::json;

fn entity() -> EntityRef {
    EntityRef::new("01K2R8JD3ZJME72AJGQY67E5F8".parse().expect("entity id"))
}

fn command() -> Command {
    Command::MoveStatus(MoveStatus {
        target: entity(),
        to: "proposed".to_owned(),
        expected_revision: Some(EntityRevision::new(7).expect("revision")),
        decided_on: Some(Node::Text("reviewed".to_owned())),
    })
}

fn envelope() -> CommandEnvelope<Command> {
    let context = CommandContext::new(
        "request-secret-context".parse().expect("request id"),
        "retry-1".parse().expect("idempotency key"),
        "human:alice".parse().expect("actor"),
        "correlation-1".parse().expect("correlation id"),
        Timestamp::from_epoch_millis(1_700_000_000_000),
    )
    .executed_by("agent:runner".parse().expect("executor"));
    CommandEnvelope::new(
        "command-1".parse().expect("command id"),
        command().kind().as_str(),
        command(),
        context,
    )
    .targeting(entity())
    .expecting(EntityRevision::new(7).expect("revision"))
}

fn request_document() -> CommandRequestV1 {
    CommandRequestV1 {
        command_id: "command-1".parse().expect("command id"),
        idempotency_key: "retry-1".parse().expect("idempotency key"),
        command_type: command().kind().as_str().to_owned(),
        target: Nullable::new(Some(entity())),
        expected_revision: Nullable::new(Some(EntityRevision::new(7).expect("revision"))),
        correlation_id: "correlation-1".parse().expect("correlation id"),
        causation: Nullable::new(None),
        execution_id: Nullable::new(None),
        task: Nullable::new(None),
        payload: serde_json::to_value(command()).expect("command serialises"),
    }
}

#[derive(Clone, Default)]
struct RecordingTransport {
    requests: Arc<Mutex<Vec<wire::Request>>>,
    responses: Arc<Mutex<VecDeque<Result<Response, TransportError>>>>,
}

impl RecordingTransport {
    fn answering(response: Response) -> Self {
        Self {
            requests: Arc::default(),
            responses: Arc::new(Mutex::new(VecDeque::from([Ok(response)]))),
        }
    }

    fn failing(reason: &str) -> Self {
        Self {
            requests: Arc::default(),
            responses: Arc::new(Mutex::new(VecDeque::from([Err(TransportError::new(
                reason,
            ))]))),
        }
    }

    fn answering_many(responses: impl IntoIterator<Item = Response>) -> Self {
        Self {
            requests: Arc::default(),
            responses: Arc::new(Mutex::new(
                responses.into_iter().map(Ok).collect::<VecDeque<_>>(),
            )),
        }
    }

    fn requests(&self) -> Vec<wire::Request> {
        self.requests.lock().expect("request lock").clone()
    }
}

impl Transport for RecordingTransport {
    fn send(
        &self,
        request: wire::Request,
    ) -> impl std::future::Future<Output = Result<Response, TransportError>> {
        self.requests.lock().expect("request lock").push(request);
        std::future::ready(
            self.responses
                .lock()
                .expect("response lock")
                .pop_front()
                .expect("one planted response"),
        )
    }
}

#[derive(Clone)]
struct StaticCredential(BearerToken);

impl CredentialProvider for StaticCredential {
    fn credential(
        &self,
    ) -> impl std::future::Future<Output = Result<BearerToken, CredentialError>> {
        std::future::ready(Ok(self.0.clone()))
    }
}

fn credential() -> StaticCredential {
    StaticCredential(BearerToken::new("token-secret-value").expect("token"))
}

fn response(status: u16, body: Vec<u8>) -> Response {
    Response {
        status,
        headers: BTreeMap::from([
            ("Content-Type".to_owned(), MEDIA_TYPE_V1.to_owned()),
            ("Vary".to_owned(), "Accept".to_owned()),
        ]),
        body,
    }
}

fn accepted_response() -> Response {
    let result = wire::CommandResultV1::from(CommandResult::accepted(
        "command-1".parse().expect("command id"),
        Vec::new(),
        ConsistencyToken::new("seq:1").expect("token"),
    ));
    response(
        200,
        wire::encode(&wire::SuccessV1 {
            request_id: "server-request-1".parse().expect("request id"),
            result,
        })
        .expect("response encodes"),
    )
}

fn unavailable_response() -> Response {
    response(
        503,
        wire::encode(&wire::ProblemDocumentV1 {
            request_id: "server-request-1".parse().expect("request id"),
            error: wire::ProblemV1 {
                code: "unavailable".to_owned(),
                message: "try later".to_owned(),
                retryable: true,
                details: json!({"reason": "maintenance"}),
            },
        })
        .expect("problem encodes"),
    )
}

#[test]
fn nullable_request_members_are_present_even_when_their_value_is_null() {
    let mut document = request_document();
    document.target = Nullable::new(None);
    document.expected_revision = Nullable::new(None);
    let encoded = wire::encode(&document).expect("encodes");
    let value: serde_json::Value = serde_json::from_slice(&encoded).expect("json");

    assert_eq!(value["target"], serde_json::Value::Null);
    assert_eq!(value["expected_revision"], serde_json::Value::Null);
    assert_eq!(value["causation"], serde_json::Value::Null);
    assert_eq!(value["execution_id"], serde_json::Value::Null);
    assert_eq!(value["task"], serde_json::Value::Null);
    assert_eq!(encoded.last(), Some(&b'\n'));
}

#[test]
fn a_missing_nullable_member_and_an_unknown_member_are_each_refused() {
    let mut missing = serde_json::to_value(request_document()).expect("value");
    missing.as_object_mut().expect("object").remove("target");
    let error =
        wire::decode::<CommandRequestV1>(&serde_json::to_vec(&missing).expect("serialises"))
            .expect_err("target is mandatory even though nullable");
    assert!(
        error.to_string().contains("missing field `target`"),
        "{error}"
    );

    let mut unknown = serde_json::to_value(request_document()).expect("value");
    unknown
        .as_object_mut()
        .expect("object")
        .insert("actor".to_owned(), json!("human:mallory"));
    let error =
        wire::decode::<CommandRequestV1>(&serde_json::to_vec(&unknown).expect("serialises"))
            .expect_err("trusted field is not part of the request");
    assert!(
        error.to_string().contains("unknown field `actor`"),
        "{error}"
    );
}

#[test]
fn the_semantic_client_sends_intent_and_never_trusted_context() {
    let transport = RecordingTransport::answering(accepted_response());
    let client = AepClient::new(transport.clone(), credential(), "company/acme", "repo one")
        .expect("client");

    let result = block_on(client.execute(envelope())).expect("accepted");
    assert_eq!(result.consistency.to_string(), "seq:1");

    let requests = transport.requests();
    let [request] = requests.as_slice() else {
        panic!("expected one request, got {}", requests.len());
    };
    assert_eq!(request.method, Method::Post);
    assert_eq!(
        request.path,
        "/aep/v1/realms/company%2Facme/workspaces/repo%20one/commands"
    );
    assert_eq!(request.headers["Accept"], MEDIA_TYPE_V1);
    assert_eq!(request.headers["Content-Type"], MEDIA_TYPE_V1);
    assert_eq!(
        request.headers["Authorization"],
        "Bearer token-secret-value"
    );

    let body: serde_json::Value = serde_json::from_slice(&request.body).expect("json body");
    for trusted in ["actor", "executor", "request_id", "issued_at", "roles"] {
        assert!(
            body.get(trusted).is_none(),
            "{trusted} crossed the trust boundary"
        );
    }
    assert_eq!(body["idempotency_key"], "retry-1");
    assert_eq!(body["correlation_id"], "correlation-1");
}

#[test]
fn a_no_response_failure_is_unavailable_without_a_forged_problem() {
    let transport = RecordingTransport::failing("connection reset");
    let client = AepClient::new(transport, credential(), "company", "repo").expect("client");

    let error = block_on(client.execute(envelope())).expect_err("transport did not answer");
    assert!(matches!(error, CommandError::Unavailable { .. }));
    assert!(error.to_string().contains("connection reset"), "{error}");
    assert!(error.is_retryable());
}

#[test]
fn a_revision_problem_maps_back_to_the_semantic_error() {
    let body = wire::encode(&wire::ProblemDocumentV1 {
        request_id: "server-request-1".parse().expect("request id"),
        error: wire::ProblemV1 {
            code: "revision_conflict".to_owned(),
            message: "the entity changed".to_owned(),
            retryable: false,
            details: json!({"entity": entity(), "expected": 7, "actual": 8}),
        },
    })
    .expect("problem encodes");
    let client = AepClient::new(
        RecordingTransport::answering(response(409, body)),
        credential(),
        "company",
        "repo",
    )
    .expect("client");

    let error = block_on(client.execute(envelope())).expect_err("conflict");
    assert!(matches!(
        error,
        CommandError::RevisionConflict {
            expected,
            actual,
            ..
        } if expected.get() == 7 && actual.get() == 8
    ));
}

#[test]
fn token_and_client_debug_output_do_not_expose_credentials() {
    let token = BearerToken::new("do-not-print-this").expect("token");
    assert_eq!(format!("{token:?}"), "BearerToken([REDACTED])");

    let client = AepClient::new(
        RecordingTransport::default(),
        StaticCredential(token),
        "company",
        "repo",
    )
    .expect("client");
    let debug = format!("{client:?}");
    assert!(!debug.contains("do-not-print-this"), "{debug}");
    assert!(debug.contains("/aep/v1/realms/company/workspaces/repo"));
}

#[test]
fn every_semantic_query_has_one_versioned_route() {
    let transport =
        RecordingTransport::answering_many(std::iter::repeat_with(unavailable_response).take(7));
    let client =
        AepClient::new(transport.clone(), credential(), "company", "repo").expect("client");

    let demanded = QueryConsistency::at_least(ConsistencyToken::new("seq:9").expect("token"));
    assert!(matches!(
        block_on(client.get(&entity(), demanded)),
        Err(QueryError::Unavailable { .. })
    ));
    assert!(matches!(
        block_on(
            client.resolve(
                &"ep://acme/payments/story/AUTH-142"
                    .parse()
                    .expect("locator")
            )
        ),
        Err(QueryError::Unavailable { .. })
    ));
    assert!(matches!(
        block_on(client.query(&EntityQuery::default())),
        Err(QueryError::Unavailable { .. })
    ));
    assert!(matches!(
        block_on(client.relations(&RelationQuery::default())),
        Err(QueryError::Unavailable { .. })
    ));
    assert!(matches!(
        block_on(client.history(&entity())),
        Err(QueryError::Unavailable { .. })
    ));
    assert!(matches!(
        block_on(client.audit(&AuditQuery::default())),
        Err(QueryError::Unavailable { .. })
    ));
    assert!(matches!(
        block_on(client.describe_type(&"aep.story/v1".parse().expect("entity type"))),
        Err(QueryError::Unavailable { .. })
    ));

    let requests = transport.requests();
    let paths: Vec<&str> = requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "/aep/v1/realms/company/workspaces/repo/entities/01K2R8JD3ZJME72AJGQY67E5F8",
            "/aep/v1/realms/company/workspaces/repo/entities/resolve",
            "/aep/v1/realms/company/workspaces/repo/entities/query",
            "/aep/v1/realms/company/workspaces/repo/relations/query",
            "/aep/v1/realms/company/workspaces/repo/entities/01K2R8JD3ZJME72AJGQY67E5F8/history",
            "/aep/v1/realms/company/workspaces/repo/audit/query",
            "/aep/v1/realms/company/workspaces/repo/types/aep.story%2Fv1",
        ]
    );
    assert_eq!(requests[0].headers["AEP-Consistency"], "seq:9");
    assert_eq!(requests[0].method, Method::Get);
    assert_eq!(requests[1].method, Method::Post);

    let entity_query: serde_json::Value =
        serde_json::from_slice(&requests[2].body).expect("entity query JSON");
    for nullable in [
        "entity_type",
        "organisation",
        "space",
        "related_to",
        "relation",
        "limit",
        "after",
    ] {
        assert_eq!(
            entity_query[nullable],
            serde_json::Value::Null,
            "{nullable}"
        );
    }
}

#[test]
fn the_official_client_maps_every_constructed_command_answer() {
    let expectations = [
        ("accepted-human-command", "accepted"),
        ("replayed-delegated-command", "replayed"),
        ("semantic-command-refusal", "conflict"),
        ("revision-conflict", "revision_conflict"),
        ("malformed-command", "invalid"),
        ("service-unavailable", "unavailable"),
        ("unauthenticated-command", "unauthorised"),
        ("workspace-unauthorized-command", "unauthorised"),
        ("unsupported-wire-version", "unsupported"),
    ];

    for (name, expected) in expectations {
        let case = aep_client::conformance::CASES
            .iter()
            .find(|case| case.name == name)
            .expect("case");
        let headers = match case.response.content_type {
            Some(content_type) => BTreeMap::from([
                ("Content-Type".to_owned(), content_type.to_owned()),
                ("Vary".to_owned(), "Accept".to_owned()),
            ]),
            None => BTreeMap::from([(
                "AEP-Supported-Versions".to_owned(),
                case.response
                    .supported_versions
                    .unwrap_or_default()
                    .to_owned(),
            )]),
        };
        let client = AepClient::new(
            RecordingTransport::answering(Response {
                status: case.response.status,
                headers,
                body: case.response.body.to_vec(),
            }),
            credential(),
            "company",
            "repo",
        )
        .expect("client");

        match block_on(client.execute(envelope())) {
            Ok(result) => assert_eq!(
                serde_json::to_value(result.outcome).expect("outcome"),
                json!(expected),
                "{name}"
            ),
            Err(error) => assert_eq!(error.code(), expected, "{name}: {error}"),
        }
    }
}
