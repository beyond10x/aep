//! Each artifact kind as a real Entity Runtime entity type.
//!
//! [`crate::kernel::definition_for`] builds the ladder the kernel decides moves with: states,
//! transitions and the evidence and date rules. A planning store that records its artifacts as
//! Entity Runtime entities needs more than that. The entity must carry the artifact itself, as
//! typed fields, so the kernel validates every write and the store holds a structured object
//! instead of an opaque document. This module extends the ladder's definition with:
//!
//! | part | what it holds |
//! |---|---|
//! | `title`, `summary`, `owner`, `withholds`, `model_digest` | optional strings |
//! | `tags` | an array of strings |
//! | `relations`, `refs`, `scope` | JSON arrays in the frontmatter's own shape |
//! | `extra` | every other frontmatter key, as a JSON object |
//! | `body` | the Markdown body |
//! | operation `edit` | replaces the artifact's fields and body in any state, without moving it |
//!
//! Moving between statuses stays the ladder's operations, one per target status. A move carries no
//! field change: an edit and a move are two decisions, so each one's record says exactly what it
//! decided.

use aep_domain::artifact::{ArtifactKind, ArtifactLifecycle};
use entity_core::EntityDefinition;
use serde_json::{json, Value};

use crate::kernel::{lifecycle_document, parse_definition};

/// The operation that replaces an artifact's content without moving it.
pub const EDIT: &str = "edit";

/// The typed fields every artifact kind carries, beside the ladder's dated keys.
///
/// `title` and every other string is optional, because a store may hold artifacts written before
/// a field existed. A present value must still be the declared type.
fn content_fields() -> serde_json::Map<String, Value> {
    let text = json!({ "type": "string" });
    let list = json!({ "type": "json" });
    let mut fields = serde_json::Map::new();
    for name in [
        "title",
        "summary",
        "owner",
        "withholds",
        "model_digest",
        "body",
    ] {
        fields.insert(name.to_owned(), text.clone());
    }
    fields.insert(
        "tags".to_owned(),
        json!({ "type": "array", "items": { "type": "string" } }),
    );
    for name in ["relations", "refs", "scope"] {
        fields.insert(name.to_owned(), list.clone());
    }
    fields.insert("extra".to_owned(), json!({ "type": "json" }));
    fields
}

/// The field a stored artifact carries its contract record in, beside the typed content: AEP's
/// packed metadata and the domain events its history and audit are read from.
pub const DOCUMENT: &str = "document";

/// The definition a planning store records an artifact kind under.
///
/// [`planning_definition`], plus the [`DOCUMENT`] field, and with the ladder's evidence and
/// date rules left out. The store records a move AEP's kernel already decided with the evidence
/// on hand (`crate::kernel::decide`). The write sees the decided result, not the counts, so the
/// recorded definition holds the transitions and the typed content, and the kernel decision holds
/// the rules. Passing the counts through, so the recorded decision re-checks them, is the
/// follow-up the design names in § 4.2a.
///
/// # Errors
///
/// As for [`planning_definition`].
pub fn storage_definition(
    kind: Option<&ArtifactKind>,
    lifecycle: &ArtifactLifecycle,
) -> Result<EntityDefinition, String> {
    let definition = planning_definition(kind, lifecycle)?;
    let mut document = serde_json::to_value(definition).map_err(|error| error.to_string())?;
    document["schema"]["fields"][DOCUMENT] = json!({ "type": "json" });
    if let Some(operations) = document["operations"].as_object_mut() {
        for (name, operation) in operations.iter_mut() {
            if let Some(operation) = operation.as_object_mut() {
                operation.remove("preconditions");
                if name == EDIT {
                    operation["arguments"]["fields"][DOCUMENT] =
                        json!({ "type": "json", "default": null });
                    operation["set"][DOCUMENT] = json!(format!("$args.{DOCUMENT}"));
                }
            }
        }
    }
    parse_definition(document)
}

/// The kind's entity definition: the ladder, the typed content, and `edit`.
///
/// # Errors
///
/// The lifecycle names something the pinned kernel does not read, as [`crate::kernel::definition_for`]
/// reports it.
pub fn planning_definition(
    kind: Option<&ArtifactKind>,
    lifecycle: &ArtifactLifecycle,
) -> Result<EntityDefinition, String> {
    let mut document = lifecycle_document(kind, lifecycle)?;
    let fields = document["schema"]["fields"]
        .as_object_mut()
        .ok_or_else(|| "the ladder's definition has no field map".to_owned())?;
    let content = content_fields();
    for (name, schema) in &content {
        if fields.contains_key(name) {
            return Err(format!(
                "the lifecycle names `{name}` as a dated key, and `{name}` is an artifact field"
            ));
        }
        fields.insert(name.clone(), schema.clone());
    }
    // Edit takes every field as an argument and sets each from it. The kernel refuses a template
    // that names an absent argument, so each argument defaults to its field's empty value: an
    // edit states the artifact's whole content, as a replace did, and a field it leaves out is
    // emptied rather than kept.
    let mut edit_fields = serde_json::Map::new();
    let mut set = serde_json::Map::new();
    for (name, schema) in fields.iter() {
        let mut argument = schema.clone();
        let empty = match argument["type"].as_str() {
            Some("string") => json!(""),
            Some("array") => json!([]),
            _ => Value::Null,
        };
        argument["default"] = empty;
        edit_fields.insert(name.clone(), argument);
        set.insert(name.clone(), Value::String(format!("$args.{name}")));
    }
    let states: Vec<Value> = document["lifecycle"]["states"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let operations = document["operations"]
        .as_object_mut()
        .ok_or_else(|| "the ladder's definition has no operation map".to_owned())?;
    if operations.contains_key(EDIT) {
        return Err(format!(
            "the lifecycle declares a status named `{EDIT}`, which is the edit operation's name"
        ));
    }
    operations.insert(
        EDIT.to_owned(),
        json!({
            "arguments": { "fields": edit_fields },
            "transitions": states
                .iter()
                .map(|state| json!({ "from": state, "to": state }))
                .collect::<Vec<_>>(),
            "set": set,
        }),
    );
    parse_definition(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_domain::artifact::{ArtifactLifecycle, ArtifactStatus};
    use entity_core::{Registry, Runtime};

    fn lifecycle() -> ArtifactLifecycle {
        serde_yaml_like_lifecycle()
    }

    /// `draft → active → implemented`, the smallest ladder with a move worth refusing.
    fn serde_yaml_like_lifecycle() -> ArtifactLifecycle {
        let mut lifecycle = ArtifactLifecycle::permissive();
        lifecycle.initial = ArtifactStatus::parse("draft").unwrap();
        lifecycle.transitions = [
            ("draft", vec!["active"]),
            ("active", vec!["implemented"]),
            ("implemented", vec![]),
        ]
        .into_iter()
        .map(|(from, to)| {
            (
                ArtifactStatus::parse(from).unwrap(),
                to.into_iter()
                    .map(|status| ArtifactStatus::parse(status).unwrap())
                    .collect(),
            )
        })
        .collect();
        lifecycle
    }

    fn registry() -> (Registry, String) {
        let kind = ArtifactKind::parse("story").unwrap();
        let definition = planning_definition(Some(&kind), &lifecycle()).expect("definition");
        let entity = definition.entity.clone();
        let mut registry = Registry::new();
        registry.register(definition).expect("the kernel holds it");
        (registry, entity)
    }

    #[test]
    fn an_artifact_is_created_with_typed_fields_and_a_wrong_type_is_refused() {
        let (registry, entity) = registry();
        let runtime = Runtime::new(&registry);
        let created = runtime
            .create(
                &entity,
                1,
                "story:one".to_owned(),
                json!({ "title": "One", "tags": ["a"], "body": "text" }),
            )
            .expect("a well-typed artifact is created");
        assert_eq!(created.instance.fields["title"], json!("One"));
        assert_eq!(created.instance.lifecycle_state, "draft");
        let refused = runtime.create(&entity, 1, "story:two".to_owned(), json!({ "title": 7 }));
        assert!(
            refused.is_err(),
            "a title that is not a string was accepted"
        );
        let refused = runtime.create(&entity, 1, "story:three".to_owned(), json!({ "tags": [1] }));
        assert!(refused.is_err(), "a tag that is not a string was accepted");
    }

    #[test]
    fn an_edit_replaces_the_content_in_place_and_a_move_off_the_ladder_is_refused() {
        let (registry, entity) = registry();
        let runtime = Runtime::new(&registry);
        let created = runtime
            .create(
                &entity,
                1,
                "story:one".to_owned(),
                json!({ "title": "One", "summary": "s" }),
            )
            .expect("created");
        let edited = runtime
            .execute(
                &created.instance,
                EDIT,
                json!({ "title": "One, revised", "body": "new" }),
            )
            .expect("an edit in any state");
        assert_eq!(
            edited.instance.lifecycle_state, "draft",
            "an edit moved the artifact"
        );
        assert_eq!(edited.instance.fields["title"], json!("One, revised"));
        assert_eq!(edited.instance.fields["body"], json!("new"));
        assert_eq!(
            edited.instance.fields["summary"],
            json!(""),
            "an edit that left `summary` out kept it"
        );
        let skipped = runtime.execute(&edited.instance, "implemented", json!({}));
        assert!(
            skipped.is_err(),
            "draft → implemented skips a rung and was permitted"
        );
        let moved = runtime
            .execute(&edited.instance, "active", json!({}))
            .expect("draft → active is on the ladder");
        assert_eq!(moved.instance.lifecycle_state, "active");
        assert_eq!(
            moved.instance.fields["title"],
            json!("One, revised"),
            "a move lost a field"
        );
    }
}
