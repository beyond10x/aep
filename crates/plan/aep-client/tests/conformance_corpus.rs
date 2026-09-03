//! Structural and byte-level checks over the shared constructed corpus.

use std::collections::BTreeSet;

use aep_client::conformance::{Dispatch, CASES};
use aep_client::wire::{
    self, CommandRequestV1, CommandResultV1, EntityPageV1, EntityQueryV1, ProblemDocumentV1,
    ProblemMappingV1, SuccessV1,
};
use aep_contract::CommandError;
use aep_domain::entity::EntityRevision;

#[test]
fn every_case_has_one_name_and_canonical_document_bytes() {
    let mut names = BTreeSet::new();
    for case in CASES {
        assert!(names.insert(case.name), "duplicate case {}", case.name);
        assert!(
            !case
                .request
                .body
                .windows("Bearer ".len())
                .any(|window| window == b"Bearer "),
            "{} contains a credential",
            case.name
        );
        if !case.request.body.is_empty() {
            assert_eq!(
                case.request.body.last(),
                Some(&b'\n'),
                "{} request has no trailing line feed",
                case.name
            );
        }
        if !case.response.body.is_empty() {
            assert_eq!(
                case.response.body.last(),
                Some(&b'\n'),
                "{} response has no trailing line feed",
                case.name
            );
        }

        if case.name == "malformed-command" {
            let error = wire::decode::<CommandRequestV1>(case.request.body)
                .expect_err("the malformed control must remain malformed");
            assert!(
                error.to_string().contains("missing field `target`"),
                "{error}"
            );
        } else if case.request.path.ends_with("/commands") && case.request.accept.ends_with("=1") {
            let request: CommandRequestV1 =
                wire::decode(case.request.body).expect("command request");
            request
                .decode_command()
                .expect("command type and payload agree");
            assert_eq!(
                wire::encode(&request).expect("re-encodes"),
                case.request.body
            );
        } else if case.request.path.ends_with("/entities/query") {
            let request: EntityQueryV1 = wire::decode(case.request.body).expect("entity query");
            assert_eq!(
                wire::encode(&request).expect("re-encodes"),
                case.request.body
            );
        }

        if case.response.status == 200 && case.request.path.ends_with("/commands") {
            let response: SuccessV1<CommandResultV1> =
                wire::decode(case.response.body).expect("command response");
            assert_eq!(
                wire::encode(&response).expect("re-encodes"),
                case.response.body
            );
        } else if case.response.status == 200 {
            let response: SuccessV1<EntityPageV1> =
                wire::decode(case.response.body).expect("query response");
            assert_eq!(
                wire::encode(&response).expect("re-encodes"),
                case.response.body
            );
        } else if !case.response.body.is_empty() {
            let response: ProblemDocumentV1 =
                wire::decode(case.response.body).expect("problem response");
            assert_eq!(
                response.error.retryable,
                response.error.code == "unavailable",
                "{} retry polarity",
                case.name
            );
            assert_eq!(
                wire::encode(&response).expect("re-encodes"),
                case.response.body
            );
        }
    }
}

#[test]
fn the_corpus_covers_dispatch_refusal_attribution_and_retry_boundaries() {
    let names: BTreeSet<&str> = CASES.iter().map(|case| case.name).collect();
    for required in [
        "accepted-human-command",
        "replayed-delegated-command",
        "semantic-command-refusal",
        "revision-conflict",
        "malformed-command",
        "service-unavailable",
        "unauthenticated-command",
        "workspace-unauthorized-command",
        "authorized-entity-query",
        "workspace-unauthorized-query",
        "unsupported-wire-version",
    ] {
        assert!(names.contains(required), "missing required case {required}");
    }

    assert!(CASES.iter().any(|case| case.dispatch == Dispatch::Command));
    assert!(CASES
        .iter()
        .any(|case| case.dispatch == Dispatch::EntityQuery));
    assert!(CASES.iter().any(|case| case.dispatch == Dispatch::None));
    assert_eq!(CASES.iter().filter(|case| case.retryable).count(), 1);

    let replay = CASES
        .iter()
        .find(|case| case.name == "replayed-delegated-command")
        .expect("replay case");
    let aep_client::conformance::VerifierOutcome::Verified(principal) = replay.verifier else {
        panic!("replay must carry a verified delegated principal");
    };
    assert_eq!(principal.authority, "human:alice");
    assert_eq!(principal.executor, Some("agent:planner"));
    assert_eq!(principal.delegation_id, Some("delegation-1"));

    let malformed = CASES
        .iter()
        .find(|case| case.name == "malformed-command")
        .expect("malformed case");
    assert_eq!(malformed.dispatch, Dispatch::None);
}

#[test]
fn service_side_problem_mapping_is_the_mapping_the_corpus_pins() {
    let entity = aep_domain::entity::EntityRef::new(
        "01K2R8JD3ZJME72AJGQY67E5F8".parse().expect("entity id"),
    );
    let mapping = ProblemMappingV1::command(&CommandError::RevisionConflict {
        entity,
        expected: EntityRevision::new(7).expect("revision"),
        actual: EntityRevision::new(8).expect("revision"),
    });
    let case = CASES
        .iter()
        .find(|case| case.name == "revision-conflict")
        .expect("revision case");
    let document: ProblemDocumentV1 = wire::decode(case.response.body).expect("problem");
    assert_eq!(mapping.status, case.response.status);
    assert_eq!(mapping.problem, document.error);

    let denied = ProblemMappingV1::unauthorized("workspace is not granted");
    let case = CASES
        .iter()
        .find(|case| case.name == "workspace-unauthorized-command")
        .expect("authorization case");
    let document: ProblemDocumentV1 = wire::decode(case.response.body).expect("problem");
    assert_eq!(denied.status, case.response.status);
    assert_eq!(denied.problem, document.error);
}
