//! Credential-free constructed exchanges shared by the official client and service.

use crate::wire::{Method, MEDIA_TYPE_V1};

/// A verifier input after credential validation, containing no credential bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Principal {
    /// On whose behalf the operation occurs.
    pub authority: &'static str,
    /// What executes it when different from the authority.
    pub executor: Option<&'static str>,
    /// Realm asserted by verified identity.
    pub realm: &'static str,
    /// Workspaces currently granted.
    pub workspace_grants: &'static [&'static str],
    /// Roles currently granted.
    pub roles: &'static [&'static str],
    /// Verified delegation identity, when an agent acts for an owner.
    pub delegation_id: Option<&'static str>,
}

/// What credential verification produces for a constructed case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifierOutcome {
    /// The credential was accepted as this principal.
    Verified(Principal),
    /// No usable credential was presented.
    Unauthenticated,
}

/// Whether the request reaches a semantic service method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dispatch {
    /// Dispatches one semantic command.
    Command,
    /// Dispatches one entity query.
    EntityQuery,
    /// Is refused before semantic dispatch.
    None,
}

/// The request half of one constructed exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedRequest {
    /// HTTP method.
    pub method: Method,
    /// Exact request path.
    pub path: &'static str,
    /// Requested response media type.
    pub accept: &'static str,
    /// Request body media type, absent on a bodyless request.
    pub content_type: Option<&'static str>,
    /// Whether an authorization credential is presented.
    pub credential_present: bool,
    /// Exact request bytes, empty for a bodyless request.
    pub body: &'static [u8],
}

/// The response half of one constructed exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedResponse {
    /// HTTP status.
    pub status: u16,
    /// Selected response media type, absent for an empty unnegotiated response.
    pub content_type: Option<&'static str>,
    /// Advertised served versions on failed negotiation.
    pub supported_versions: Option<&'static str>,
    /// Exact response bytes, possibly empty.
    pub body: &'static [u8],
}

/// One complete constructed client/service conformance case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Case {
    /// Stable case name.
    pub name: &'static str,
    /// Request expectation.
    pub request: ExpectedRequest,
    /// Credential-verifier outcome.
    pub verifier: VerifierOutcome,
    /// Expected semantic dispatch or non-dispatch.
    pub dispatch: Dispatch,
    /// Response expectation.
    pub response: ExpectedResponse,
    /// Whether retrying unchanged intent may succeed later.
    pub retryable: bool,
}

const COMMAND_PATH: &str = "/aep/v1/realms/company/workspaces/repo/commands";
const ENTITY_QUERY_PATH: &str = "/aep/v1/realms/company/workspaces/repo/entities/query";
const COMMAND_REQUEST: &[u8] = include_bytes!("../fixtures/v1/command-request.json");
const MALFORMED_COMMAND_REQUEST: &[u8] =
    include_bytes!("../fixtures/v1/malformed-command-request.json");
const QUERY_REQUEST: &[u8] = include_bytes!("../fixtures/v1/entity-query-request.json");

const HUMAN: Principal = Principal {
    authority: "human:alice",
    executor: None,
    realm: "company",
    workspace_grants: &["repo"],
    roles: &["engineer"],
    delegation_id: None,
};

const DELEGATED: Principal = Principal {
    authority: "human:alice",
    executor: Some("agent:planner"),
    realm: "company",
    workspace_grants: &["repo"],
    roles: &["engineer"],
    delegation_id: Some("delegation-1"),
};

const OUTSIDE_WORKSPACE: Principal = Principal {
    authority: "human:bob",
    executor: None,
    realm: "company",
    workspace_grants: &["other-repo"],
    roles: &["engineer"],
    delegation_id: None,
};

const fn post(
    path: &'static str,
    body: &'static [u8],
    credential_present: bool,
) -> ExpectedRequest {
    ExpectedRequest {
        method: Method::Post,
        path,
        accept: MEDIA_TYPE_V1,
        content_type: Some(MEDIA_TYPE_V1),
        credential_present,
        body,
    }
}

const fn answered(status: u16, body: &'static [u8]) -> ExpectedResponse {
    ExpectedResponse {
        status,
        content_type: Some(MEDIA_TYPE_V1),
        supported_versions: None,
        body,
    }
}

/// Every constructed version-1 exchange, in stable corpus order.
pub const CASES: &[Case] = &[
    Case {
        name: "accepted-human-command",
        request: post(COMMAND_PATH, COMMAND_REQUEST, true),
        verifier: VerifierOutcome::Verified(HUMAN),
        dispatch: Dispatch::Command,
        response: answered(
            200,
            include_bytes!("../fixtures/v1/accepted-command-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "replayed-delegated-command",
        request: post(COMMAND_PATH, COMMAND_REQUEST, true),
        verifier: VerifierOutcome::Verified(DELEGATED),
        dispatch: Dispatch::Command,
        response: answered(
            200,
            include_bytes!("../fixtures/v1/replayed-command-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "semantic-command-refusal",
        request: post(COMMAND_PATH, COMMAND_REQUEST, true),
        verifier: VerifierOutcome::Verified(HUMAN),
        dispatch: Dispatch::Command,
        response: answered(
            409,
            include_bytes!("../fixtures/v1/semantic-refusal-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "revision-conflict",
        request: post(COMMAND_PATH, COMMAND_REQUEST, true),
        verifier: VerifierOutcome::Verified(HUMAN),
        dispatch: Dispatch::Command,
        response: answered(
            409,
            include_bytes!("../fixtures/v1/revision-conflict-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "malformed-command",
        request: post(COMMAND_PATH, MALFORMED_COMMAND_REQUEST, true),
        verifier: VerifierOutcome::Verified(HUMAN),
        dispatch: Dispatch::None,
        response: answered(
            400,
            include_bytes!("../fixtures/v1/malformed-command-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "service-unavailable",
        request: post(COMMAND_PATH, COMMAND_REQUEST, true),
        verifier: VerifierOutcome::Verified(HUMAN),
        dispatch: Dispatch::Command,
        response: answered(
            503,
            include_bytes!("../fixtures/v1/unavailable-response.json"),
        ),
        retryable: true,
    },
    Case {
        name: "unauthenticated-command",
        request: post(COMMAND_PATH, COMMAND_REQUEST, false),
        verifier: VerifierOutcome::Unauthenticated,
        dispatch: Dispatch::None,
        response: answered(
            401,
            include_bytes!("../fixtures/v1/unauthenticated-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "workspace-unauthorized-command",
        request: post(COMMAND_PATH, COMMAND_REQUEST, true),
        verifier: VerifierOutcome::Verified(OUTSIDE_WORKSPACE),
        dispatch: Dispatch::None,
        response: answered(
            403,
            include_bytes!("../fixtures/v1/unauthorized-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "authorized-entity-query",
        request: post(ENTITY_QUERY_PATH, QUERY_REQUEST, true),
        verifier: VerifierOutcome::Verified(HUMAN),
        dispatch: Dispatch::EntityQuery,
        response: answered(
            200,
            include_bytes!("../fixtures/v1/entity-query-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "workspace-unauthorized-query",
        request: post(ENTITY_QUERY_PATH, QUERY_REQUEST, true),
        verifier: VerifierOutcome::Verified(OUTSIDE_WORKSPACE),
        dispatch: Dispatch::None,
        response: answered(
            403,
            include_bytes!("../fixtures/v1/query-unauthorized-response.json"),
        ),
        retryable: false,
    },
    Case {
        name: "unsupported-wire-version",
        request: ExpectedRequest {
            method: Method::Post,
            path: COMMAND_PATH,
            accept: "application/vnd.aep.service+json;version=2",
            content_type: Some("application/vnd.aep.service+json;version=2"),
            credential_present: true,
            body: COMMAND_REQUEST,
        },
        verifier: VerifierOutcome::Verified(HUMAN),
        dispatch: Dispatch::None,
        response: ExpectedResponse {
            status: 406,
            content_type: None,
            supported_versions: Some("1"),
            body: b"",
        },
        retryable: false,
    },
];
