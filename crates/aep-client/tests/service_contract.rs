//! Contract projections stay derived from the strict service wire.

use std::collections::BTreeSet;

use aep_client::conformance::CASES;
use aep_client::wire::{
    AuditPageV1, AuditQueryV1, CommandRequestV1, CommandResultV1, EntityPageV1, EntityQueryV1,
    HistoryQueryV2, Method, PageV2, ProblemDocumentV1, RelationPageV1, RelationQueryV1,
    ResolveRequestV1, SuccessV1, TypeDescriptionV1, ROUTES, SERVICE_PATH_PREFIX,
};
use aep_contract::query::{EntityEnvelope, RevisionRecord};
use schemars::schema_for;
use serde_json::Value;

#[test]
fn every_service_operation_has_one_unique_method_path_and_operation_id() {
    let mut method_paths = BTreeSet::new();
    let mut operation_ids = BTreeSet::new();

    for route in ROUTES {
        assert!(route.path().starts_with(SERVICE_PATH_PREFIX));
        assert!(method_paths.insert((route.method.as_str(), route.path())));
        assert!(operation_ids.insert(route.operation.id()));
        assert!(matches!(route.method, Method::Get | Method::Post));
    }

    assert_eq!(method_paths.len(), ROUTES.len());
    assert_eq!(operation_ids.len(), ROUTES.len());
}

#[test]
fn every_service_document_has_a_derived_json_schema() {
    let schemas = [
        serde_json::to_value(schema_for!(CommandRequestV1)).unwrap(),
        serde_json::to_value(schema_for!(CommandResultV1)).unwrap(),
        serde_json::to_value(schema_for!(ResolveRequestV1)).unwrap(),
        serde_json::to_value(schema_for!(HistoryQueryV2)).unwrap(),
        serde_json::to_value(schema_for!(EntityQueryV1)).unwrap(),
        serde_json::to_value(schema_for!(RelationQueryV1)).unwrap(),
        serde_json::to_value(schema_for!(AuditQueryV1)).unwrap(),
        serde_json::to_value(schema_for!(EntityPageV1)).unwrap(),
        serde_json::to_value(schema_for!(RelationPageV1)).unwrap(),
        serde_json::to_value(schema_for!(AuditPageV1)).unwrap(),
        serde_json::to_value(schema_for!(PageV2<RevisionRecord>)).unwrap(),
        serde_json::to_value(schema_for!(SuccessV1<EntityEnvelope>)).unwrap(),
        serde_json::to_value(schema_for!(SuccessV1<TypeDescriptionV1>)).unwrap(),
        serde_json::to_value(schema_for!(ProblemDocumentV1)).unwrap(),
    ];

    for schema in schemas {
        assert!(schema.as_object().is_some_and(|object| !object.is_empty()));
    }
}

#[test]
fn derived_schemas_accept_every_well_formed_constructed_exchange() {
    let command_request = validator::<CommandRequestV1>();
    let entity_query = validator::<EntityQueryV1>();
    let command_success = validator::<SuccessV1<CommandResultV1>>();
    let entity_success = validator::<SuccessV1<EntityPageV1>>();
    let problem = validator::<ProblemDocumentV1>();

    for case in CASES {
        if !case.request.body.is_empty() {
            let document: Value = serde_json::from_slice(case.request.body).unwrap();
            let selected = if case.request.path.ends_with("/commands") {
                &command_request
            } else {
                &entity_query
            };
            if case.name == "malformed-command" {
                assert!(!selected.is_valid(&document), "{}", case.name);
            } else {
                assert_valid(selected, &document, case.name);
            }
        }

        if !case.response.body.is_empty() {
            let document: Value = serde_json::from_slice(case.response.body).unwrap();
            let selected = if case.response.status != 200 {
                &problem
            } else if case.request.path.ends_with("/commands") {
                &command_success
            } else {
                &entity_success
            };
            assert_valid(selected, &document, case.name);
        }
    }
}

fn validator<T: schemars::JsonSchema>() -> jsonschema::Validator {
    let schema = serde_json::to_value(schema_for!(T)).unwrap();
    jsonschema::validator_for(&schema).unwrap()
}

fn assert_valid(validator: &jsonschema::Validator, document: &Value, case: &str) {
    let errors: Vec<String> = validator
        .iter_errors(document)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{case}: {errors:?}");
}
