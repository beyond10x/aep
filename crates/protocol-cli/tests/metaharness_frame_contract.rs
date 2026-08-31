//! The frame this repository mints, read by the rules the other side refuses it with.
//!
//! `protocol drive` hands every `llm` step a sealed `metaharness.frame/1` document, and metaharness
//! refuses one **by name** when it is not JSON, carries no tag it knows, does not have a frame's
//! shape, or states a digest that does not describe its contents
//! (`crates/metaharness-protocol/src/frame.rs`, `Frame::parse_document` and `FrameDocError`, read
//! 2026-08-23). Until this file existed, nothing on either side of that seam compared the two: the
//! repositories share a vocabulary and no code, so a drift in the minter or in the reader was
//! silent until a driven run died at its first step with a refusal and a spent session.
//!
//! # Why the reader is transcribed and not linked
//!
//! This repository is public and metaharness is not, so no Cargo dependency may cross — that is the
//! boundary `story:metaharness-executor` was accepted on, and it is not a packaging detail. The
//! consequence is that the *only* honest test is a second implementation: the rules below are
//! written out from `frame.rs`, and a change over there that this file does not follow shows up as
//! this test disagreeing with the real consumer rather than as a green gate on both sides. The
//! transcription names its source at every rule it copies.
//!
//! # What is deliberately not covered here
//!
//! The reader's first refusal is *unreadable* — a file that does not exist, a partial write, a
//! permission error. That is an I/O condition of the caller's filesystem and not a property of any
//! document, so there is no document a minter could produce that has it. It is stated rather than
//! tested; what *is* tested is the readable-but-wrong end of the same class, a document whose bytes
//! are not JSON.
//!
//! # The golden
//!
//! [`GOLDEN`] is committed at `crates/protocol-cli/fixtures/metaharness-frame-canonical.json` and is
//! the exact document `write_frame_document` would put on disk for one deterministic step — minted
//! by the driver's own code path, not typed by hand, and reproducible because nothing in that path
//! reads a clock, a random source or anything else off the machine that minted it. It is the
//! cross-repository artifact: a later metaharness-side wave replays these bytes through the real
//! `Frame::parse_document`, and the two sides then disagree loudly or not at all. It holds nothing
//! account-level — no credential, no user, no absolute path — because this repository is public.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

/// The committed cross-repository golden, as bytes rather than as a path, so a test binary run from
/// anywhere reads the same document.
const GOLDEN: &str = include_str!("../fixtures/metaharness-frame-canonical.json");

/// The tag a frame document carries, transcribed from `frame.rs`'s `FRAME_FORMAT`.
///
/// Spelled as a literal here on purpose: comparing the golden against this crate's own
/// `METAHARNESS_FRAME_FORMAT` constant would compare the minter with itself.
const FRAME_FORMAT: &str = "metaharness.frame/1";

/// Every field the unscoped canonical `Frame` serializes, in declaration order.
///
/// Pinned as a list because the digest is computed over the *struct's* re-serialization: a field
/// the minter invented is dropped on the way in, and a frame whose digest was taken over it then
/// states a digest its surviving contents do not imply. `subjects` is the one optional field and
/// is transcribed below; the canonical deliberately leaves it absent to keep the original
/// cross-repository bytes stable.
const FRAME_FIELDS: [&str; 11] = [
    "workflow",
    "node",
    "step",
    "prior",
    "obligations",
    "reaching",
    "next",
    "handoff",
    "operations",
    "entities",
    "digest",
];

/// Every operation in the closed v0.1 vocabulary that takes no parameter, from `Operation`.
const PARAMETERLESS: [&str; 10] = [
    "file.read",
    "file.write",
    "file.edit",
    "dir.list",
    "search",
    "shell",
    "web.read",
    "skill.load",
    "subagent.spawn",
    "task.todo",
];

/// Every way a frame document fails to be read, transcribed from `FrameDocError`.
///
/// One variant per refusal the consumer has a name for, because a test that asserted only *an
/// error* would pass on the wrong refusal — a misshapen document reported as a digest mismatch is a
/// different bug report to whoever reads it at three in the morning.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// The bytes are not a JSON object.
    NotJson(String),
    /// The `format` field is missing, or names something this reader does not know.
    UnknownFormat(Option<String>),
    /// The object does not have a frame's shape. `serde` produces this over there; the shape is
    /// written out by hand here.
    Invalid(String),
    /// The stated digest does not describe the contents.
    DigestMismatch {
        /// What the document claims.
        stated: String,
        /// What its contents imply.
        computed: String,
    },
}

/// A frame document read the way the consumer reads one: tag, then shape, then digest.
///
/// The order is `Frame::parse_document`'s and it is load-bearing — a document that is untagged
/// *and* misshapen is refused as untagged, because the D2 rule is that a document says what it is
/// before anything tries to understand it.
fn parse_document(text: &str) -> Result<Value, Refusal> {
    let mut value: Value =
        serde_json::from_str(text).map_err(|error| Refusal::NotJson(error.to_string()))?;
    let Some(object) = value.as_object_mut() else {
        return Err(Refusal::NotJson(
            "the document is JSON but not an object".to_owned(),
        ));
    };
    match object.remove("format") {
        Some(Value::String(tag)) if tag == FRAME_FORMAT => {}
        Some(tag) => {
            return Err(Refusal::UnknownFormat(Some(
                tag.as_str().unwrap_or("<not a string>").to_owned(),
            )))
        }
        None => return Err(Refusal::UnknownFormat(None)),
    }
    let frame = frame_value(object)?;
    let stated = frame["digest"]
        .as_str()
        .expect("the shape check found a string digest")
        .to_owned();
    let computed = computed_digest(&frame);
    if stated != computed {
        return Err(Refusal::DigestMismatch { stated, computed });
    }
    Ok(frame)
}

/// The document projected onto a `Frame`, which is what `serde_json::from_value` does over there.
///
/// A projection and not a validation, because that difference is the whole reason an extra field is
/// caught at all: a field no variant of the struct holds is **dropped**, not refused, and the digest
/// is then taken over what survived.
fn frame_value(object: &Map<String, Value>) -> Result<Value, Refusal> {
    let workflow = object_at(field(object, "workflow")?, "workflow")?;
    let step = object_at(field(object, "step")?, "step")?;
    let mut frame = json!({
        "workflow": {
            "id": string(field(workflow, "id")?, "workflow.id")?,
            "version": string(field(workflow, "version")?, "workflow.version")?,
        },
        "node": node_ref(field(object, "node")?, "node")?,
        "step": {
            "workflow": string(field(step, "workflow")?, "step.workflow")?,
            "state": string(field(step, "state")?, "step.state")?,
            "index": number(field(step, "index")?, "step.index")?,
            "attempt": number(field(step, "attempt")?, "step.attempt")?,
        },
        "prior": lines(field(object, "prior")?, "prior", "source")?,
        "obligations": lines(field(object, "obligations")?, "obligations", "asked_by")?,
        "reaching": lines(field(object, "reaching")?, "reaching", "asked_by")?,
        "next": next(field(object, "next")?)?,
        "handoff": handoff(field(object, "handoff")?)?,
        "operations": operations(field(object, "operations")?)?,
        "entities": entities(field(object, "entities")?)?,
        "digest": string(field(object, "digest")?, "digest")?,
    });
    if let Some(value) = object.get("subjects") {
        frame["subjects"] = subject_scope(value)?;
    }
    Ok(frame)
}

/// The digest a frame's contents imply, from `Frame::computed_digest`.
///
/// SHA-256, hex, over the compact serialization of the frame with `digest` removed. `serde_json`'s
/// map is a `BTreeMap` in both workspaces — neither enables `preserve_order` — so "sorted at every
/// level" is a property of the bytes and not of either producer's field order.
fn computed_digest(frame: &Value) -> String {
    let mut value = frame.clone();
    value
        .as_object_mut()
        .expect("a frame is an object")
        .remove("digest");
    let bytes = serde_json::to_vec(&value).expect("a frame value serialises");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------- the shape, field by field

/// The field, or the refusal naming what was absent.
fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a Value, Refusal> {
    object
        .get(name)
        .ok_or_else(|| Refusal::Invalid(format!("missing field `{name}`")))
}

/// A `String` field.
fn string(value: &Value, at: &str) -> Result<Value, Refusal> {
    value
        .as_str()
        .map(|text| Value::String(text.to_owned()))
        .ok_or_else(|| Refusal::Invalid(format!("`{at}` is not a string")))
}

/// An `Option<String>` field: absent-as-null is legal and is what this repository always mints.
fn optional_string(value: &Value, at: &str) -> Result<Value, Refusal> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    string(value, at)
}

/// A `u32` field.
fn number(value: &Value, at: &str) -> Result<Value, Refusal> {
    value
        .as_u64()
        .filter(|count| u32::try_from(*count).is_ok())
        .map(Value::from)
        .ok_or_else(|| Refusal::Invalid(format!("`{at}` is not a u32")))
}

/// A nested object.
fn object_at<'a>(value: &'a Value, at: &str) -> Result<&'a Map<String, Value>, Refusal> {
    value
        .as_object()
        .ok_or_else(|| Refusal::Invalid(format!("`{at}` is not an object")))
}

/// A nested array.
fn array_at<'a>(value: &'a Value, at: &str) -> Result<&'a Vec<Value>, Refusal> {
    value
        .as_array()
        .ok_or_else(|| Refusal::Invalid(format!("`{at}` is not an array")))
}

/// A `NodeRef`.
fn node_ref(value: &Value, at: &str) -> Result<Value, Refusal> {
    let object = object_at(value, at)?;
    Ok(json!({ "id": string(field(object, "id")?, &format!("{at}.id"))? }))
}

/// A list of `EvidenceLine` or of `Line`; they differ only in what the source field is called.
///
/// Built through the map rather than through `json!` because the source field's *name* is the
/// parameter — `prior` calls it `source` and the other two call it `asked_by`, and a reader that
/// hard-coded either would accept a document the struct does not describe.
fn lines(value: &Value, at: &str, source: &str) -> Result<Value, Refusal> {
    let mut rendered = Vec::new();
    for (index, entry) in array_at(value, at)?.iter().enumerate() {
        let position = format!("{at}[{index}]");
        let object = object_at(entry, &position)?;
        let mut line = Map::new();
        line.insert(
            "text".to_owned(),
            string(field(object, "text")?, &format!("{position}.text"))?,
        );
        line.insert(
            source.to_owned(),
            optional_string(field(object, source)?, &format!("{position}.{source}"))?,
        );
        rendered.push(Value::Object(line));
    }
    Ok(Value::Array(rendered))
}

/// The nodes reachable from here.
fn next(value: &Value) -> Result<Value, Refusal> {
    let mut rendered = Vec::new();
    for (index, entry) in array_at(value, "next")?.iter().enumerate() {
        rendered.push(node_ref(entry, &format!("next[{index}]"))?);
    }
    Ok(Value::Array(rendered))
}

/// The `Handoff`, internally tagged on `handoff`.
fn handoff(value: &Value) -> Result<Value, Refusal> {
    let object = object_at(value, "handoff")?;
    let tag = field(object, "handoff")?
        .as_str()
        .ok_or_else(|| Refusal::Invalid("`handoff.handoff` is not a string".to_owned()))?;
    match tag {
        "none" => Ok(json!({ "handoff": "none" })),
        "artifact" => Ok(json!({
            "handoff": "artifact",
            "name": string(field(object, "name")?, "handoff.name")?,
            "kind": optional_string(field(object, "kind")?, "handoff.kind")?,
        })),
        "structured_answer" => Ok(json!({
            "handoff": "structured_answer",
            "schema": string(field(object, "schema")?, "handoff.schema")?,
        })),
        other => Err(Refusal::Invalid(format!("unknown handoff `{other}`"))),
    }
}

/// The `OperationSet`.
///
/// A set and not a list on the way in, keyed by `Operation::sort_key` — the wire name, then the MCP
/// coordinates — because that ordering is a **wire fact** an external producer is expected to
/// follow, not this or that enum's variant order. The first cross-repository document failed on
/// exactly that, which is why it is transcribed here rather than assumed.
fn operations(value: &Value) -> Result<Value, Refusal> {
    let mut set: BTreeMap<(String, String, String), Value> = BTreeMap::new();
    for (index, entry) in array_at(value, "operations")?.iter().enumerate() {
        let at = format!("operations[{index}]");
        let object = object_at(entry, &at)?;
        let name = field(object, "op")?
            .as_str()
            .ok_or_else(|| Refusal::Invalid(format!("`{at}.op` is not a string")))?
            .to_owned();
        if name == "mcp.call" {
            let server = string(field(object, "server")?, &format!("{at}.server"))?;
            let tool = string(field(object, "tool")?, &format!("{at}.tool"))?;
            let key = (
                name.clone(),
                server.as_str().unwrap_or_default().to_owned(),
                tool.as_str().unwrap_or_default().to_owned(),
            );
            set.insert(
                key,
                json!({ "op": "mcp.call", "server": server, "tool": tool }),
            );
        } else if PARAMETERLESS.contains(&name.as_str()) {
            set.insert(
                (name.clone(), String::new(), String::new()),
                json!({ "op": name }),
            );
        } else {
            return Err(Refusal::Invalid(format!("unknown operation `{name}`")));
        }
    }
    Ok(Value::Array(set.into_values().collect()))
}

/// The optional ordered subject scope, transcribed from `SubjectScope` and `SubjectRule`.
fn subject_scope(value: &Value) -> Result<Value, Refusal> {
    let object = object_at(value, "subjects")?;
    let mut rules = Vec::new();
    for (index, entry) in array_at(field(object, "rules")?, "subjects.rules")?
        .iter()
        .enumerate()
    {
        let at = format!("subjects.rules[{index}]");
        let rule = object_at(entry, &at)?;
        let mut patterns = Vec::new();
        for (pattern_index, pattern) in
            array_at(field(rule, "subjects")?, &format!("{at}.subjects"))?
                .iter()
                .enumerate()
        {
            patterns.push(string(pattern, &format!("{at}.subjects[{pattern_index}]"))?);
        }
        rules.push(json!({
            "subjects": patterns,
            "operations": operations(field(rule, "operations")?)?,
        }));
    }
    Ok(json!({ "rules": rules }))
}

/// The optional `EntityList`.
fn entities(value: &Value) -> Result<Value, Refusal> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let object = object_at(value, "entities")?;
    let mut members = Vec::new();
    for (index, member) in array_at(field(object, "members")?, "entities.members")?
        .iter()
        .enumerate()
    {
        members.push(string(member, &format!("entities.members[{index}]"))?);
    }
    Ok(json!({
        "source": string(field(object, "source")?, "entities.source")?,
        "members": members,
    }))
}

// ---------------------------------------------------------------- the golden, as minted

/// The golden as a mutable object, for the mutations below.
fn golden_object() -> Map<String, Value> {
    serde_json::from_str::<Value>(GOLDEN)
        .expect("the committed golden is JSON")
        .as_object()
        .expect("the committed golden is an object")
        .clone()
}

/// The golden with `mutate` applied, rendered the way the driver renders a document.
fn mutated(mutate: impl FnOnce(&mut Map<String, Value>)) -> String {
    let mut object = golden_object();
    mutate(&mut object);
    let mut text = serde_json::to_string_pretty(&Value::Object(object)).expect("serialises");
    text.push('\n');
    text
}

/// The digest the golden states, for the mutation tests to compare against.
fn stated_digest() -> String {
    golden_object()["digest"]
        .as_str()
        .expect("the golden states a digest")
        .to_owned()
}

/// The minted document is one the consumer accepts, and its digest genuinely describes it.
///
/// This is the assertion the seam did not have: not that the minter sets *a* digest, but that the
/// digest is the one a second, independent implementation of the rule arrives at from the bytes on
/// disk. A minter that sealed over its own field order, or over the document including its tag,
/// passes every test it writes about itself and fails here.
#[test]
fn the_minted_golden_is_accepted_by_the_rules_that_would_refuse_it() {
    let frame = parse_document(GOLDEN).expect("the committed golden is accepted");
    assert_eq!(
        frame["digest"].as_str().expect("a digest"),
        computed_digest(&frame),
        "the digest describes the contents"
    );
}

/// The tag is compared with a literal transcribed from `frame.rs`, never with this crate's own
/// constant: a document tagged from the same constant the minter used would agree with itself
/// whatever either says.
#[test]
fn the_minted_golden_carries_the_exact_tag_the_consumer_looks_for() {
    assert_eq!(
        golden_object()["format"].as_str(),
        Some("metaharness.frame/1"),
        "the tag is the one `frame.rs` publishes, byte for byte"
    );
}

/// A field the minter invents is not refused on the way in — it is **dropped** — so the drift shows
/// up one step later as a digest that no longer describes what survived.
///
/// Asserted because it is the most likely real drift: somebody adds a field to the frame here,
/// every test in this repository passes, and every driven run dies at its first step. The field set
/// is therefore pinned to the eleven the struct holds, plus the tag the document adds.
#[test]
fn the_minted_golden_holds_exactly_the_fields_the_frame_struct_has() {
    let object = golden_object();
    let mut present: Vec<&str> = object.keys().map(String::as_str).collect();
    present.sort_unstable();
    let mut expected: Vec<&str> = FRAME_FIELDS.into_iter().chain(["format"]).collect();
    expected.sort_unstable();
    assert_eq!(present, expected);
}

/// The consumer's additive `subjects` field is part of this independent reader even though the
/// original canonical frame deliberately keeps its byte identity by omitting the empty default.
#[test]
fn a_scoped_frame_survives_projection_and_its_scope_participates_in_the_digest() {
    let mut object = golden_object();
    object.insert(
        "subjects".to_owned(),
        json!({
            "rules": [
                {
                    "subjects": ["file:.engineering/planning/**"],
                    "operations": [{"op": "file.edit"}, {"op": "file.read"}],
                },
                {
                    "subjects": ["file:**"],
                    "operations": [{"op": "file.read"}],
                },
            ],
        }),
    );
    let mut contents = Value::Object(object.clone());
    let contents_object = contents.as_object_mut().expect("an object");
    contents_object.remove("format");
    contents_object.remove("digest");
    object.insert("digest".to_owned(), computed_digest(&contents).into());
    let mut document = serde_json::to_string_pretty(&Value::Object(object)).expect("serialises");
    document.push('\n');

    let parsed = parse_document(&document).expect("the consumer accepts its optional scope");
    assert_eq!(
        parsed["subjects"]["rules"][0]["subjects"][0],
        "file:.engineering/planning/**"
    );

    let edited = document.replace("file:.engineering/planning/**", "file:**");
    assert!(
        matches!(parse_document(&edited), Err(Refusal::DigestMismatch { .. })),
        "scope is sealed into the frame rather than advisory text"
    );
}

/// Refusal class *untagged*: the `format` field removed, and then present but naming a version this
/// build does not know. Both are `UnknownFormat`, and the reader says which — a document from a
/// future metaharness is a different conversation to a document from a producer that forgot.
#[test]
fn a_frame_document_without_a_tag_it_knows_is_refused_as_untagged() {
    let untagged = mutated(|object| {
        object.remove("format");
    });
    assert_eq!(
        parse_document(&untagged),
        Err(Refusal::UnknownFormat(None)),
        "a document that does not say what it is is refused, not guessed at"
    );

    let future = mutated(|object| {
        object.insert("format".to_owned(), json!("metaharness.frame/2"));
    });
    assert_eq!(
        parse_document(&future),
        Err(Refusal::UnknownFormat(Some(
            "metaharness.frame/2".to_owned()
        )))
    );
}

/// Refusal class *misshapen*: a field of the frame's shape removed, and a field of the wrong type.
///
/// `step.attempt` is the removal on purpose — it is a field nothing else in the document repeats,
/// so a reader that reconstructed the shape from what it could see would still be missing it.
#[test]
fn a_frame_document_that_is_not_a_frames_shape_is_refused_as_misshapen() {
    let truncated_step = mutated(|object| {
        object["step"]
            .as_object_mut()
            .expect("the golden's step is an object")
            .remove("attempt");
    });
    assert_eq!(
        parse_document(&truncated_step),
        Err(Refusal::Invalid("missing field `attempt`".to_owned()))
    );

    let wrong_type = mutated(|object| {
        object.insert("workflow".to_owned(), json!(3));
    });
    assert_eq!(
        parse_document(&wrong_type),
        Err(Refusal::Invalid("`workflow` is not an object".to_owned()))
    );

    let unknown_operation = mutated(|object| {
        object["operations"]
            .as_array_mut()
            .expect("the golden's operations are a list")
            .push(json!({ "op": "database.drop" }));
    });
    assert_eq!(
        parse_document(&unknown_operation),
        Err(Refusal::Invalid(
            "unknown operation `database.drop`".to_owned()
        )),
        "the operation vocabulary is closed; an adapter that grew one would be a weakening \
         the protocol had no way to notice"
    );
}

/// Refusal class *digest mismatch*: one byte of one obligation changed, everything else untouched.
///
/// The mutation is the smallest one that means something — `red` becomes `fed`, a word the model
/// would have been shown instead of the one the engine wrote — and the check that catches it is the
/// same re-derivation the accepting test runs. A digest that survived this edit would be
/// decoration, and the frame would pin nothing at the one boundary it is cited across.
#[test]
fn a_single_flipped_byte_in_a_minted_frame_breaks_the_digest() {
    let flipped = GOLDEN.replace("the suite is red", "the suite is fed");
    assert_ne!(flipped, GOLDEN, "the mutation reached the document");

    let error = parse_document(&flipped).expect_err("an edited document is refused");
    let Refusal::DigestMismatch { stated, computed } = error else {
        panic!("expected a digest mismatch, got {error:?}");
    };
    assert_eq!(
        stated,
        stated_digest(),
        "the document still states the digest it was sealed with"
    );
    assert_ne!(stated, computed, "and its contents no longer imply it");
}

/// The same class from the other direction: a document that was never sealed at all.
///
/// Written as a frame carrying the all-zero digest, because that is what a producer that built the
/// value and forgot to seal it emits — and it must not survive the file boundary, or the digest
/// every downstream event cites pins nothing.
#[test]
fn a_frame_document_that_was_never_sealed_is_refused_by_the_same_check() {
    let unsealed = mutated(|object| {
        object.insert("digest".to_owned(), json!("0".repeat(64)));
    });
    let error = parse_document(&unsealed).expect_err("an unsealed document is refused");
    let Refusal::DigestMismatch { stated, computed } = error else {
        panic!("expected a digest mismatch, got {error:?}");
    };
    assert_eq!(stated, "0".repeat(64));
    assert_eq!(
        computed,
        stated_digest(),
        "the contents are still the golden's"
    );
}

/// The readable end of the *unreadable* class: bytes that are not a JSON object.
///
/// The other end — a file that is missing, half-written or unreadable — is an I/O condition of the
/// caller's filesystem and is not a document any minter can produce, so it is stated in this
/// module's own documentation rather than tested here.
#[test]
fn bytes_that_are_not_a_json_object_are_refused_before_the_tag_is_looked_for() {
    assert!(matches!(
        parse_document(&GOLDEN[..GOLDEN.len() / 2]),
        Err(Refusal::NotJson(_))
    ));
    assert_eq!(
        parse_document("[]"),
        Err(Refusal::NotJson(
            "the document is JSON but not an object".to_owned()
        ))
    );
}

/// The minted operations are in the order the wire, not an enum, decides.
///
/// `OperationSet` is a set whose `Ord` is the operation's **wire name** — a rule an external
/// producer can follow without reading metaharness's enum — and the digest is taken over that
/// order. The first cross-repository frame document failed on exactly this, so it is asserted about
/// the bytes rather than trusted to the minter's `sort_unstable`.
#[test]
fn the_minted_golden_lists_its_operations_in_wire_name_order() {
    let object = golden_object();
    let names: Vec<&str> = object["operations"]
        .as_array()
        .expect("the golden lists operations")
        .iter()
        .map(|entry| entry["op"].as_str().expect("an operation names itself"))
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted, "strictly ascending by wire name, no repeats");
    for name in &names {
        assert!(
            PARAMETERLESS.contains(name),
            "`{name}` is outside the closed v0.1 vocabulary"
        );
    }
}

/// The transcription is not vacuous: it accepts a frame that is not the golden.
///
/// A reader that refused everything would pass every refusal test above and prove nothing by the
/// accepting one. This seals a second, differently shaped frame — the shape a routing step takes,
/// with an entity list, an artifact handoff and a parameterised MCP operation, none of which the
/// golden carries — by the same rule, and reads it back.
#[test]
fn the_transcribed_reader_accepts_a_frame_the_golden_does_not_cover() {
    let frame = json!({
        "workflow": { "id": "development/default", "version": "1" },
        "node": { "id": "receive" },
        "step": { "workflow": "development/default", "state": "receive", "index": 0, "attempt": 1 },
        "prior": [{ "text": "the task is stated", "source": "artifacts/task.md" }],
        "obligations": [],
        "reaching": [],
        "next": [{ "id": "specify" }],
        "handoff": { "handoff": "artifact", "name": "the classification", "kind": null },
        "operations": [
            { "op": "file.read" },
            { "op": "mcp.call", "server": "planning", "tool": "read" }
        ],
        "entities": { "source": "artifacts/registry.md", "members": ["one", "two"] }
    });
    let sealed = {
        let mut object = frame.as_object().expect("an object").clone();
        object.insert("digest".to_owned(), json!(computed_digest(&frame)));
        object.insert("format".to_owned(), json!(FRAME_FORMAT));
        Value::Object(object)
    };
    let read = parse_document(&serde_json::to_string(&sealed).expect("serialises"))
        .expect("a differently shaped frame is accepted too");
    assert_eq!(read["entities"]["members"][1], json!("two"));
    assert_eq!(read["handoff"]["name"], json!("the classification"));
    assert_eq!(read["operations"][1]["server"], json!("planning"));
}
