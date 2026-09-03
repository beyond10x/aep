//! The `relations:` block a drafted `ess/1` type carries, and the only signals an `OpenAPI`
//! document gives for one.
//!
//! # One place to rename
//!
//! ESS owns the relation vocabulary and that vocabulary is younger than this projection. Every key
//! and every value written into a draft — `relations`, `name`, `kind`, `target`, `cardinality`,
//! `via`, `references`, `one`, `many` — is a constant declared at the top of this file, and
//! [`write_block`] is the only code in this repository that writes them. A rename in ESS is an edit
//! to those constants and to nothing else here; a projection that spelt the keys inline would spread
//! that rename across one edit per call site, and the call sites would have to be found by grepping
//! for an English word rather than by following a constant.
//!
//! # Two signals, and nothing else
//!
//! A relation is emitted only where the document states one:
//!
//! - a property whose schema is a `$ref` to a schema that becomes an entity — cardinality `one`,
//!   or `many` when the property is an array of that `$ref`;
//! - a property named `<x>_id` or `<x>Id` whose type is the type of entity `X`'s own identity
//!   property.
//!
//! Everything else is a guess, and a guess is what the `UNMAPPED:` convention exists to refuse. In
//! particular `owns` is never inferred: an `OpenAPI` document says a payload carries a reference,
//! and says nothing at all about whether the referent's life is bounded by the referrer's. Every
//! relation read from an id field therefore carries an `UNMAPPED: ownership` line, and every
//! relation whose shape the document states two ways carries `UNMAPPED: cardinality` over the
//! placeholder `one`.
//!
//! A relation is named for the property it was read from — the property itself for a `$ref`, and the
//! property without its `_id`/`Id` suffix for an identity field — and `via:` carries that property
//! verbatim, so the draft can always be read back against the document it came from.

use std::fmt::Write as _;

use serde_yaml::{Mapping, Value as Yaml};

use super::{ess_type, kebab, pascal};

/// The key the block itself is written under.
const KEY_BLOCK: &str = "relations";

/// The key carrying a relation's own name.
const KEY_NAME: &str = "name";

/// The key carrying what kind of relation it is.
const KEY_KIND: &str = "kind";

/// The key carrying the qualified entity the relation points at.
const KEY_TARGET: &str = "target";

/// The key carrying how many of the target one source holds.
const KEY_CARDINALITY: &str = "cardinality";

/// The key carrying the property the reference travels on.
const KEY_VIA: &str = "via";

/// The one kind an `OpenAPI` document can support. `owns` is a lifetime claim and is never inferred.
const KIND_REFERENCES: &str = "references";

/// Cardinality `one`, and the placeholder written where the document states two shapes.
const CARDINALITY_ONE: &str = "one";

/// Cardinality `many`.
const CARDINALITY_MANY: &str = "many";

/// How many of the target one source holds, as the document states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Cardinality {
    /// The property carries a single reference.
    One,
    /// The property carries a collection of them.
    Many,
    /// The document states the property two ways and the draft will not choose between them.
    Unreadable,
}

/// One relation read out of one property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Relation {
    /// The relation's own name, derived from the property that carries it.
    name: String,
    /// The qualified entity name the relation points at.
    target: String,
    /// How many of the target one source holds.
    cardinality: Cardinality,
    /// The property on the source schema the reference travels on.
    via: String,
    /// Whether the signal was an id field, which cannot say who owns whom.
    ownership_unmapped: bool,
}

/// Every relation the properties of `schema` state, in the document's own property order.
///
/// `owner` is the qualified name of the type `schema` projects to, used only in the ownership
/// marker; `schemas` is the document's `components.schemas`, which is where a `$ref` is resolved and
/// where an entity's identity type is read.
pub(super) fn relations_for(
    owner: &str,
    schema: &Yaml,
    schemas: &Mapping,
    domain: &str,
) -> Vec<Relation> {
    let Some(properties) = schema.get("properties").and_then(Yaml::as_mapping) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    for (field, definition) in properties {
        let Some(field) = field.as_str() else { continue };
        let read = from_reference(field, definition, schemas, domain)
            .or_else(|| from_identity(owner, field, definition, schemas, domain));
        if let Some(relation) = read {
            edges.push(relation);
        }
    }
    edges
}

/// The relation a property states by referring to a schema directly.
fn from_reference(
    field: &str,
    definition: &Yaml,
    schemas: &Mapping,
    domain: &str,
) -> Option<Relation> {
    let (target, cardinality) = shape(definition, schemas, domain)?;
    Some(Relation {
        name: kebab(field),
        target,
        cardinality,
        via: field.to_owned(),
        ownership_unmapped: false,
    })
}

/// What a property refers to and how many of it, where the document states both.
fn shape(definition: &Yaml, schemas: &Mapping, domain: &str) -> Option<(String, Cardinality)> {
    if let Some(target) = referenced_entity(definition, schemas, domain) {
        return Some((target, Cardinality::One));
    }
    if definition.get("type").and_then(Yaml::as_str) == Some("array") {
        let target = definition
            .get("items")
            .and_then(|items| referenced_entity(items, schemas, domain))?;
        return Some((target, Cardinality::Many));
    }
    composed(definition, schemas, domain)
}

/// What a `allOf`/`oneOf`/`anyOf` property refers to.
///
/// One target across every branch is a reference the document states; two are a choice this draft
/// will not take, so nothing is emitted. Branches that disagree about the *shape* are the case the
/// `UNMAPPED: cardinality` marker exists for — `oneOf: [Carrier, [Carrier]]` says which entity and
/// does not say how many.
fn composed(definition: &Yaml, schemas: &Mapping, domain: &str) -> Option<(String, Cardinality)> {
    /// The keywords that compose a property out of other schemas.
    const COMPOSITIONS: &[&str] = &["allOf", "oneOf", "anyOf"];

    let mut branches: Vec<(String, Cardinality)> = Vec::new();
    for keyword in COMPOSITIONS {
        let Some(members) = definition.get(*keyword).and_then(Yaml::as_sequence) else {
            continue;
        };
        for member in members {
            if let Some(target) = referenced_entity(member, schemas, domain) {
                branches.push((target, Cardinality::One));
            } else if member.get("type").and_then(Yaml::as_str) == Some("array") {
                if let Some(target) = member
                    .get("items")
                    .and_then(|items| referenced_entity(items, schemas, domain))
                {
                    branches.push((target, Cardinality::Many));
                }
            }
        }
    }

    let (target, first) = branches.first()?.clone();
    if branches.iter().any(|(other, _)| *other != target) {
        return None;
    }
    let cardinality = if branches.iter().all(|(_, shape)| *shape == first) {
        first
    } else {
        Cardinality::Unreadable
    };
    Some((target, cardinality))
}

/// The relation a property states by carrying another entity's identity.
///
/// The owner's own identity field is not one: `Invoice.invoice_id` is what an `Invoice` *is*, not a
/// reference to a second one, and reading it as a relation would give every entity an edge to
/// itself.
fn from_identity(
    owner: &str,
    field: &str,
    definition: &Yaml,
    schemas: &Mapping,
    domain: &str,
) -> Option<Relation> {
    let base = field
        .strip_suffix("_id")
        .or_else(|| field.strip_suffix("Id"))?;
    if base.is_empty() {
        return None;
    }
    let wanted = pascal(base);
    let (name, schema) = schemas
        .iter()
        .find(|(name, _)| name.as_str().is_some_and(|name| pascal(name) == wanted))?;
    let name = name.as_str()?;
    if !becomes_entity(schema) {
        return None;
    }
    let target = format!("{domain}.{}", pascal(name));
    if target == owner {
        return None;
    }
    if ess_type(definition, domain) != identity_type(name, schema, domain)? {
        return None;
    }
    Some(Relation {
        name: kebab(base),
        target,
        cardinality: Cardinality::One,
        via: field.to_owned(),
        ownership_unmapped: true,
    })
}

/// The type of the property a schema uses as its identity, where it declares one by name.
///
/// `id`, or the entity's own name with an `_id`/`Id` suffix — the two conventions a document written
/// by hand actually uses. A schema that names its identity anything else is a schema this draft
/// cannot match an id field against, and it emits nothing rather than matching on type alone.
fn identity_type(name: &str, schema: &Yaml, domain: &str) -> Option<String> {
    let properties = schema.get("properties").and_then(Yaml::as_mapping)?;
    let snake = kebab(name).replace('-', "_");
    let mut camel = pascal(name);
    if let Some(first) = camel.get(0..1) {
        let lowered = first.to_lowercase();
        camel.replace_range(0..1, &lowered);
    }
    for candidate in ["id".to_owned(), format!("{snake}_id"), format!("{camel}Id")] {
        if let Some(definition) = properties.get(Yaml::from(candidate.as_str())) {
            return Some(ess_type(definition, domain));
        }
    }
    None
}

/// The qualified entity a `$ref` names, when it names one that becomes an entity.
fn referenced_entity(value: &Yaml, schemas: &Mapping, domain: &str) -> Option<String> {
    let reference = value.get("$ref").and_then(Yaml::as_str)?;
    let name = reference.rsplit('/').next()?;
    let schema = schemas.get(Yaml::from(name))?;
    if !becomes_entity(schema) {
        return None;
    }
    Some(format!("{domain}.{}", pascal(name)))
}

/// Whether a component schema is one that could become an entity.
///
/// An object, declared or defaulted. An enum is a value and a newtype over a scalar is a value; a
/// reference to either is a field, and calling it a relation would be the guess this refuses.
fn becomes_entity(schema: &Yaml) -> bool {
    schema.get("enum").is_none()
        && matches!(
            schema.get("type").and_then(Yaml::as_str),
            None | Some("object")
        )
}

/// Writes the `relations:` block for one type, or nothing at all when there is no signal.
pub(super) fn write_block(text: &mut String, owner: &str, relations: &[Relation]) {
    if relations.is_empty() {
        return;
    }
    let _ = writeln!(
        text,
        "    # Relations read from this schema's own properties. They belong to the entity this"
    );
    let _ = writeln!(
        text,
        "    # type becomes; which type that is is the first UNMAPPED question at the foot."
    );
    let _ = writeln!(text, "    {KEY_BLOCK}:");
    for relation in relations {
        if relation.ownership_unmapped {
            let _ = writeln!(
                text,
                "      # UNMAPPED: ownership — the document cannot say whether {owner} owns {}",
                relation.target
            );
        }
        if relation.cardinality == Cardinality::Unreadable {
            let _ = writeln!(text, "      # UNMAPPED: {KEY_CARDINALITY}");
        }
        let cardinality = match relation.cardinality {
            Cardinality::Many => CARDINALITY_MANY,
            Cardinality::One | Cardinality::Unreadable => CARDINALITY_ONE,
        };
        let _ = writeln!(text, "      - {KEY_NAME}: {}", relation.name);
        let _ = writeln!(text, "        {KEY_KIND}: {KIND_REFERENCES}");
        let _ = writeln!(text, "        {KEY_TARGET}: {}", relation.target);
        let _ = writeln!(text, "        {KEY_CARDINALITY}: {cardinality}");
        let _ = writeln!(text, "        {KEY_VIA}: {}", relation.via);
    }
}

#[cfg(test)]
mod tests {
    use super::{relations_for, Cardinality, Relation};
    use serde_yaml::{Mapping, Value as Yaml};

    /// The fixture document's `components.schemas`, parsed once per test.
    fn schemas() -> Mapping {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/reverse-openapi/order-service.yaml"),
        )
        .expect("the fixture document is readable");
        let document: Yaml = serde_yaml::from_str(&text).expect("the fixture document parses");
        document
            .get("components")
            .and_then(|components| components.get("schemas"))
            .and_then(Yaml::as_mapping)
            .expect("the fixture declares schemas")
            .clone()
    }

    /// The relations read from one named schema of the fixture.
    fn read(name: &str) -> Vec<Relation> {
        let schemas = schemas();
        let schema = schemas
            .get(Yaml::from(name))
            .expect("the fixture declares the schema")
            .clone();
        relations_for(
            &format!("acme.order.{name}"),
            &schema,
            &schemas,
            "acme.order",
        )
    }

    /// The one relation travelling on `via`, or a failure naming what was read instead.
    fn on(relations: &[Relation], via: &str) -> Relation {
        relations
            .iter()
            .find(|relation| relation.via == via)
            .unwrap_or_else(|| {
                panic!(
                    "no relation travels on `{via}`; read {:?}",
                    relations.iter().map(|r| &r.via).collect::<Vec<_>>()
                )
            })
            .clone()
    }

    #[test]
    fn a_ref_to_a_schema_that_becomes_an_entity_references_one_of_it() {
        let relation = on(&read("Order"), "customer");
        assert_eq!(relation.name, "customer");
        assert_eq!(relation.target, "acme.order.Customer");
        assert_eq!(relation.cardinality, Cardinality::One);
        assert!(!relation.ownership_unmapped);
    }

    #[test]
    fn an_array_of_that_ref_references_many_of_it() {
        let relation = on(&read("Order"), "lines");
        assert_eq!(relation.name, "lines");
        assert_eq!(relation.target, "acme.order.OrderLine");
        assert_eq!(relation.cardinality, Cardinality::Many);
    }

    #[test]
    fn an_id_field_matching_an_entitys_identity_leaves_ownership_unmapped() {
        // The document says the payload carries the key. It does not say whose life bounds whose,
        // and `owns` is the answer to that question rather than to this one.
        let relation = on(&read("Order"), "warehouse_id");
        assert_eq!(relation.name, "warehouse");
        assert_eq!(relation.target, "acme.order.Warehouse");
        assert_eq!(relation.cardinality, Cardinality::One);
        assert!(
            relation.ownership_unmapped,
            "an id field cannot say who owns whom"
        );
    }

    #[test]
    fn a_camel_case_id_field_is_read_the_same_way() {
        // `invoiceId` against `Invoice.invoice_id`: both spellings of the suffix, and an identity
        // property named for its own entity rather than `id`.
        let relation = on(&read("Order"), "invoiceId");
        assert_eq!(relation.name, "invoice");
        assert_eq!(relation.target, "acme.order.Invoice");
        assert!(relation.ownership_unmapped);
    }

    #[test]
    fn a_composition_that_states_one_shape_is_read_as_that_shape() {
        let relation = on(&read("Order"), "broker");
        assert_eq!(relation.target, "acme.order.Broker");
        assert_eq!(relation.cardinality, Cardinality::One);
    }

    #[test]
    fn a_composition_that_states_two_shapes_leaves_cardinality_unmapped() {
        // `oneOf: [Carrier, [Carrier]]`. The target is unambiguous and the shape is not, so the
        // relation is emitted and the half the document did not state is marked.
        let relation = on(&read("Order"), "shipper");
        assert_eq!(relation.target, "acme.order.Carrier");
        assert_eq!(relation.cardinality, Cardinality::Unreadable);
    }

    #[test]
    fn no_signal_is_no_relation() {
        let relations = read("Order");
        let carried: Vec<&str> = relations
            .iter()
            .map(|relation| relation.via.as_str())
            .collect();
        for silent in [
            // A scalar says nothing.
            "note",
            // The entity's own identity is not a relation to itself.
            "id",
            // A `$ref` to an enum: a value, not a thing with a life.
            "status",
            // A `$ref` to a newtype over a scalar, for the same reason.
            "reference",
            // `<x>_id` whose type is not `Courier`'s identity type.
            "courier_id",
            // `<x>_id` naming a schema the document does not declare.
            "region_id",
        ] {
            assert!(
                !carried.contains(&silent),
                "`{silent}` carries no signal and produced a relation; read {carried:?}"
            );
        }
    }

    #[test]
    fn a_schema_whose_properties_are_all_scalars_has_no_relations() {
        assert!(read("Customer").is_empty());
        assert!(read("Warehouse").is_empty());
    }
}
