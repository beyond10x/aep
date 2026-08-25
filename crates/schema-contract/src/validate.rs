//! Validate JSON instances by stable schema identity.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::Value;

/// A parsed JSON document and the location a diagnostic should name.
#[derive(Debug, Clone, Copy)]
pub struct JsonDocument<'a> {
    /// A path or other human-readable origin.
    pub location: &'a str,
    /// The parsed JSON value.
    pub value: &'a Value,
}

impl<'a> JsonDocument<'a> {
    /// Pairs a parsed value with its origin.
    #[must_use]
    pub const fn new(location: &'a str, value: &'a Value) -> Self {
        Self { location, value }
    }
}

/// Stable classes of contract validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueCode {
    /// A schema has no usable absolute `$id`.
    MissingSchemaId,
    /// Two schema documents claim one identity.
    DuplicateSchemaId,
    /// A schema does not satisfy its declared meta-schema.
    InvalidSchema,
    /// A schema cannot be compiled with the supplied offline registry.
    UnresolvedSchema,
    /// An instance is not an object with a string `schema` selector.
    MissingSelector,
    /// An instance selects no supplied schema.
    UnknownSchema,
    /// An instance contradicts its selected schema.
    InvalidInstance,
}

impl IssueCode {
    /// The stable machine spelling printed in diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingSchemaId => "missing_schema_id",
            Self::DuplicateSchemaId => "duplicate_schema_id",
            Self::InvalidSchema => "invalid_schema",
            Self::UnresolvedSchema => "unresolved_schema",
            Self::MissingSelector => "missing_schema_selector",
            Self::UnknownSchema => "unknown_schema",
            Self::InvalidInstance => "invalid_instance",
        }
    }
}

/// One validation failure, carrying enough location to act on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// Its stable class.
    pub code: IssueCode,
    /// The schema or instance document responsible.
    pub document: String,
    /// A JSON Pointer inside the instance, when one exists.
    pub instance_path: String,
    /// What was refused.
    pub message: String,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let location = if self.instance_path.is_empty() {
            self.document.clone()
        } else {
            format!("{}{}", self.document, self.instance_path)
        };
        write!(
            formatter,
            "[{}] {location}: {}",
            self.code.as_str(),
            self.message
        )
    }
}

/// One instance accepted by its selected schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidInstance {
    /// Where the instance came from.
    pub instance: String,
    /// The stable identity it selected.
    pub schema_id: String,
    /// Where that schema was loaded from.
    pub schema: String,
}

/// The accumulated result of one validation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// How many distinct usable schema identities were supplied.
    pub schema_count: usize,
    /// Accepted instances, in location order.
    pub valid: Vec<ValidInstance>,
    /// Every observed failure, in document and pointer order.
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Whether every supplied instance was accepted and every schema was usable.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Validates `instances` against the schemas they select with their `schema` property.
///
/// Schema compilation uses a registry built only from `schemas`; the `jsonschema` dependency has
/// all resolver features disabled, so an unprovided reference is a refusal rather than I/O.
#[must_use]
pub fn validate(schemas: &[JsonDocument<'_>], instances: &[JsonDocument<'_>]) -> ValidationReport {
    let mut issues = Vec::new();
    let mut indexed: BTreeMap<String, JsonDocument<'_>> = BTreeMap::new();

    let mut ordered_schemas = schemas.to_vec();
    ordered_schemas.sort_by_key(|document| document.location);
    for document in ordered_schemas {
        let Some(identifier) = schema_identifier(document.value) else {
            issues.push(issue(
                IssueCode::MissingSchemaId,
                document.location,
                "",
                "expected a non-empty absolute `$id`",
            ));
            continue;
        };
        if let Some(first) = indexed.get(identifier) {
            issues.push(issue(
                IssueCode::DuplicateSchemaId,
                document.location,
                "/$id",
                format!("`{identifier}` is already declared by {}", first.location),
            ));
            continue;
        }
        if let Err(error) = jsonschema::meta::validate(document.value) {
            issues.push(issue(
                IssueCode::InvalidSchema,
                document.location,
                error.instance_path().to_string(),
                error.to_string(),
            ));
            continue;
        }
        indexed.insert(identifier.to_owned(), document);
    }

    if !issues.is_empty() {
        sort_issues(&mut issues);
        return ValidationReport {
            schema_count: indexed.len(),
            valid: Vec::new(),
            issues,
        };
    }

    let registry = match registry(&indexed) {
        Ok(registry) => registry,
        Err(message) => {
            issues.push(issue(
                IssueCode::UnresolvedSchema,
                "schema registry",
                "",
                message,
            ));
            return ValidationReport {
                schema_count: indexed.len(),
                valid: Vec::new(),
                issues,
            };
        }
    };

    for (identifier, document) in &indexed {
        if let Err(error) = validator(document.value, &registry) {
            issues.push(issue(
                IssueCode::UnresolvedSchema,
                document.location,
                "",
                format!("schema `{identifier}` cannot be compiled offline: {error}"),
            ));
        }
    }
    if !issues.is_empty() {
        sort_issues(&mut issues);
        return ValidationReport {
            schema_count: indexed.len(),
            valid: Vec::new(),
            issues,
        };
    }

    let (valid, mut instance_issues) = validate_instance_documents(&indexed, &registry, instances);
    issues.append(&mut instance_issues);

    ValidationReport {
        schema_count: indexed.len(),
        valid,
        issues,
    }
}

fn validate_instance_documents<'a>(
    indexed: &BTreeMap<String, JsonDocument<'a>>,
    registry: &jsonschema::Registry<'a>,
    instances: &[JsonDocument<'a>],
) -> (Vec<ValidInstance>, Vec<ValidationIssue>) {
    let mut valid = Vec::new();
    let mut issues = Vec::new();
    let mut ordered_instances = instances.to_vec();
    ordered_instances.sort_by_key(|document| document.location);
    for instance in ordered_instances {
        let Some(identifier) = instance
            .value
            .as_object()
            .and_then(|object| object.get("schema"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            issues.push(issue(
                IssueCode::MissingSelector,
                instance.location,
                "/schema",
                "expected a non-empty string `schema` property",
            ));
            continue;
        };
        let Some(schema) = indexed.get(identifier) else {
            issues.push(issue(
                IssueCode::UnknownSchema,
                instance.location,
                "/schema",
                format!("no supplied schema declares `$id: {identifier}`"),
            ));
            continue;
        };
        let compiled = match validator(schema.value, registry) {
            Ok(compiled) => compiled,
            Err(error) => {
                issues.push(issue(
                    IssueCode::UnresolvedSchema,
                    schema.location,
                    "",
                    error.to_string(),
                ));
                continue;
            }
        };
        let mut instance_issues = compiled
            .iter_errors(instance.value)
            .map(|error| {
                issue(
                    IssueCode::InvalidInstance,
                    instance.location,
                    error.instance_path().to_string(),
                    error.to_string(),
                )
            })
            .collect::<Vec<_>>();
        if instance_issues.is_empty() {
            valid.push(ValidInstance {
                instance: instance.location.to_owned(),
                schema_id: identifier.to_owned(),
                schema: schema.location.to_owned(),
            });
        } else {
            issues.append(&mut instance_issues);
        }
    }

    valid.sort_by(|left, right| left.instance.cmp(&right.instance));
    sort_issues(&mut issues);
    (valid, issues)
}

fn validator<'a>(
    schema: &'a Value,
    registry: &'a jsonschema::Registry<'a>,
) -> Result<jsonschema::Validator, jsonschema::ValidationError<'static>> {
    jsonschema::options()
        .with_registry(registry)
        .should_validate_formats(true)
        .build(schema)
}

fn registry<'a>(
    schemas: &'a BTreeMap<String, JsonDocument<'a>>,
) -> Result<jsonschema::Registry<'a>, String> {
    let mut registry = jsonschema::Registry::new();
    for (identifier, document) in schemas {
        registry = registry
            .add(identifier.as_str(), document.value)
            .map_err(|error| error.to_string())?;
    }
    registry.prepare().map_err(|error| error.to_string())
}

fn schema_identifier(schema: &Value) -> Option<&str> {
    schema
        .as_object()?
        .get("$id")?
        .as_str()
        .filter(|value| is_absolute_identifier(value))
}

fn is_absolute_identifier(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };
    !rest.is_empty()
        && scheme.bytes().enumerate().all(|(index, byte)| {
            matches!(
                (index, byte),
                (0, b'a'..=b'z' | b'A'..=b'Z')
                    | (_, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'+' | b'-' | b'.')
            )
        })
}

fn issue(
    code: IssueCode,
    document: impl Into<String>,
    instance_path: impl Into<String>,
    message: impl Into<String>,
) -> ValidationIssue {
    ValidationIssue {
        code,
        document: document.into(),
        instance_path: instance_path.into(),
        message: message.into(),
    }
}

fn sort_issues(issues: &mut [ValidationIssue]) {
    issues.sort_by(|left, right| {
        (&left.document, &left.instance_path, left.code).cmp(&(
            &right.document,
            &right.instance_path,
            right.code,
        ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document<'a>(location: &'a str, value: &'a Value) -> JsonDocument<'a> {
        JsonDocument::new(location, value)
    }

    #[test]
    fn a_valid_instance_selects_its_schema_by_identity() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "urn:example:record:1",
            "type": "object",
            "required": ["schema", "name"],
            "properties": {
                "schema": {"const": "urn:example:record:1"},
                "name": {"type": "string"}
            },
            "additionalProperties": false
        });
        let instance = json!({"schema": "urn:example:record:1", "name": "Ada"});
        let report = validate(
            &[document("record.schema.json", &schema)],
            &[document("record.json", &instance)],
        );
        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.valid[0].schema, "record.schema.json");
    }

    #[test]
    fn failures_accumulate_across_instances_and_fields() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "urn:example:record:1",
            "type": "object",
            "required": ["schema", "name"],
            "properties": {
                "schema": {"const": "urn:example:record:1"},
                "name": {"type": "string"}
            },
            "additionalProperties": false
        });
        let malformed = json!({"schema": "urn:example:record:1", "extra": true});
        let unknown = json!({"schema": "urn:example:missing:1"});
        let report = validate(
            &[document("record.schema.json", &schema)],
            &[document("a.json", &malformed), document("b.json", &unknown)],
        );
        assert_eq!(report.issues.len(), 3, "{:?}", report.issues);
        assert_eq!(report.issues[2].code, IssueCode::UnknownSchema);
    }

    #[test]
    fn duplicate_schema_identities_are_refused_before_instances() {
        let first = json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "$id": "urn:example:x:1"});
        let second = first.clone();
        let instance = json!({"schema": "urn:example:x:1"});
        let report = validate(
            &[
                document("a.schema.json", &first),
                document("b.schema.json", &second),
            ],
            &[document("instance.json", &instance)],
        );
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].code, IssueCode::DuplicateSchemaId);
        assert!(report.valid.is_empty());
    }

    #[test]
    fn an_unprovided_reference_is_refused_offline() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "urn:example:x:1",
            "$ref": "https://schemas.example.test/unprovided.json"
        });
        let report = validate(&[document("x.schema.json", &schema)], &[]);
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].code, IssueCode::UnresolvedSchema);
    }
}
