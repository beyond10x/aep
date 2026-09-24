//! The one-time export of an `aep.project/2` store into a tree authority.
//!
//! Every subject the source holds is written to the tree as an imported anchor: its current state
//! as the anchor, and everything its history recorded as the anchor's evidence, so `history`
//! answers over the tree what it answered over the source. Nothing is decided again and nothing is
//! invented; the export is a copy under three rewrites.
//!
//! | rewrite | why |
//! |---|---|
//! | an artifact of a declared kind becomes that kind's typed entity | the tree records planning as typed entities |
//! | every counted entity identity becomes the derived one for its locator | a counted identity names a different entity on every branch |
//! | every absolute home path becomes a workspace path or a digest | a committed store carries no one's home directory (S9) |
//!
//! Operational records — invocation reservations and projection watermarks — are not history and
//! are left behind; the tree keeps neither.
//!
//! The provenance of the migration into `aep.project/2` — record coordinates, the exact legacy
//! bytes, the id reservation roster and the raw capture of the markdown store — holds source text
//! hex-encoded, and `history` reads the oldest events from it. The rewrites reach inside every
//! hex value that is UTF-8 text, and the digests that bind those bytes are derived again from the
//! rewritten bytes (`rebind_legacy`), so the graph still validates and names no home path.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aep_backend_entity::{METADATA_KEY, STORED_AS};
use aep_domain::artifact::LifecycleRegistry;
use entity_core::EntityInstance;
use entity_eventlog::ErRecordedProjector;
use entity_eventlog::{AsyncImportedAnchorWriter, Authority, EventlogRecordedStore};
use entity_store::asynchronous::{
    HistoryOrigin, LegacyAnchor, LegacyCompleteness, LegacyEvidence, LegacyOrderDeclaration,
    RecordedEntry, Subject, SubjectHistory,
};
use eventlog_core::InlineProjectionAdmin;
use serde_json::Value;

use crate::typed::{kind_of, typed_fields, TypedKinds};
use crate::{
    decision_document, AuthoritySession, CAPTURE_LIMITS, INVOCATION_AS, PROJECTION_METADATA_AS,
};

/// What an export wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportReport {
    /// Artifacts written as typed entities.
    pub artifacts: usize,
    /// Other subjects copied as they were: relations, audit, applied commands, legacy evidence.
    pub records: usize,
    /// Legacy evidence blobs whose rewritten bytes were bound to a new digest.
    pub provenance: usize,
    /// Operational subjects left behind.
    pub skipped: usize,
    /// The artifact kinds copied as the generic contract entity because no typed entity exists
    /// for them, such as a `<type>-blocker`.
    pub untyped: Vec<String>,
    /// Each counted identity and the derived one it became.
    pub identities: BTreeMap<String, String>,
    /// Each home path found and what it was written as. Kept by the operator, never committed.
    pub home_paths: BTreeMap<String, String>,
    /// The target's stream identity.
    pub stream_identity: String,
}

/// Rewrites every string in a value: identities first, then home paths.
struct Rewrite<'a> {
    identities: &'a BTreeMap<String, String>,
    /// Literal text the operator replaces on the way, such as a name that may not enter public
    /// history. Applied after the identities and before the home paths.
    fixups: &'a BTreeMap<String, String>,
    workspace: Option<PathBuf>,
    home_directory: Option<PathBuf>,
    found: BTreeMap<String, String>,
}

impl Rewrite<'_> {
    fn value(&mut self, value: &Value) -> Value {
        match value {
            Value::String(text) => Value::String(self.string(text)),
            Value::Array(items) => {
                Value::Array(items.iter().map(|item| self.value(item)).collect())
            }
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, item)| (self.text(key), self.value(item)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// A string value: text, or legacy bytes as `hex:` whose UTF-8 text is rewritten the same way.
    fn string(&mut self, text: &str) -> String {
        if text.starts_with("hex:") {
            let decoded = aep_contract::migration::HexBytesV1::parse(text)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes.into_bytes()).ok());
            if let Some(decoded) = decoded {
                let out = self.text(&decoded);
                if out != decoded {
                    return aep_contract::migration::HexBytesV1::new(out.into_bytes()).as_wire();
                }
            }
            return text.to_owned();
        }
        self.text(text)
    }

    fn text(&mut self, text: &str) -> String {
        let mut out = text.to_owned();
        for (old, new) in self.identities.iter().chain(self.fixups) {
            if out.contains(old.as_str()) {
                out = out.replace(old.as_str(), new);
            }
        }
        self.home(&out)
    }

    /// Replace each home path S9 refuses: one under the workspace by `workspace:<repo>/<path>`,
    /// any other by `home-path:sha256:<digest>` of it (design § 9.3). `~/` and `$HOME/` are
    /// read as the home directory they name, so a path under the workspace keeps its place.
    fn home(&mut self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some((start, prefix)) = home_prefix(rest) {
            out.push_str(&rest[..start]);
            let tail = &rest[start..];
            let end = tail
                .find(|c: char| {
                    c.is_whitespace() || matches!(c, '"' | '\'' | '`' | ')' | ']' | '>' | ',' | ';')
                })
                .unwrap_or(tail.len())
                .max(prefix);
            let path = &tail[..end];
            let absolute = match (&self.home_directory, path) {
                (Some(home), path) if path.starts_with("~/") => home.join(&path[2..]),
                (Some(home), path) if path.starts_with("$HOME/") => home.join(&path[6..]),
                (_, path) => PathBuf::from(path),
            };
            let mapped = match &self.workspace {
                Some(workspace) if absolute.starts_with(workspace) && absolute != *workspace => {
                    format!(
                        "workspace:{}",
                        absolute
                            .strip_prefix(workspace)
                            .unwrap_or(&absolute)
                            .display()
                    )
                }
                _ => {
                    use sha2::Digest as _;
                    let digest = sha2::Sha256::digest(path.as_bytes());
                    let hex = digest.iter().fold(String::new(), |mut hex, byte| {
                        let _ = write!(hex, "{byte:02x}");
                        hex
                    });
                    format!("home-path:sha256:{hex}")
                }
            };
            self.found.insert(path.to_owned(), mapped.clone());
            out.push_str(&mapped);
            rest = &tail[end..];
        }
        out.push_str(rest);
        out
    }
}

/// Where the first home path in `text` starts, and how long its fixed prefix is.
///
/// A home path is what S9 refuses: `/root/`, `~/`, `$HOME/`, or `/home/`, `/Users/` or
/// `C:\Users\` followed by a user name and a separator. A placeholder such as
/// `/home/<user>/` names nobody and is left alone, as S9 leaves it.
fn home_prefix(text: &str) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    let named = ["/home/", "/Users/", "c:\\users\\", "C:\\Users\\"];
    for prefix in named.iter().chain(&["/root/", "~/", "$HOME/"]) {
        let found = text
            .match_indices(prefix)
            .map(|(start, _)| start)
            .find(|&start| {
                if !named.contains(prefix) {
                    return true;
                }
                let rest = &text[start + prefix.len()..];
                let user = rest
                    .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.')))
                    .unwrap_or(rest.len());
                user > 0 && rest[user..].starts_with(['/', '\\'])
            });
        if let Some(start) = found {
            if best.is_none_or(|(at, _)| start < at) {
                best = Some((start, prefix.len()));
            }
        }
    }
    best
}

/// Everything a source history recorded, as anchor evidence: the evidence its own anchor held and
/// the events each later decision carried.
fn evidence_of(history: &SubjectHistory) -> Vec<LegacyEvidence> {
    let mut evidence = Vec::new();
    if let HistoryOrigin::Imported(anchor) = &history.origin {
        evidence.extend(anchor.evidence.iter().cloned());
    }
    for record in &history.records {
        if let RecordedEntry::Decision(commit) = &record.entry {
            if let Some(document) = decision_document(commit) {
                evidence.extend(document.events.into_iter().map(LegacyEvidence::Event));
            }
        }
    }
    evidence
}

fn rewrite_evidence(
    rewrite: &mut Rewrite<'_>,
    evidence: Vec<LegacyEvidence>,
) -> Result<Vec<LegacyEvidence>, String> {
    evidence
        .into_iter()
        .map(|item| {
            let value = serde_json::to_value(&item).map_err(|error| error.to_string())?;
            serde_json::from_value(rewrite.value(&value)).map_err(|error| error.to_string())
        })
        .collect()
}

/// Name the typed entity in every evidence object that names the artifact: the source recorded
/// its events and decisions under the contract entity type, and the tree checks that an imported
/// event names the subject it is imported into.
fn retype_evidence(
    evidence: Vec<LegacyEvidence>,
    id: &str,
    kind: &str,
) -> Result<Vec<LegacyEvidence>, String> {
    fn walk(value: &mut Value, id: &str, kind: &str) {
        match value {
            Value::Object(map) => {
                let names_it = map.get("entity").and_then(Value::as_str) == Some(STORED_AS)
                    && map.get("id").and_then(Value::as_str) == Some(id);
                if names_it {
                    map.insert("entity".to_owned(), Value::String(kind.to_owned()));
                }
                for item in map.values_mut() {
                    walk(item, id, kind);
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, id, kind);
                }
            }
            _ => {}
        }
    }
    evidence
        .into_iter()
        .map(|item| {
            let mut value = serde_json::to_value(&item).map_err(|error| error.to_string())?;
            walk(&mut value, id, kind);
            serde_json::from_value(value).map_err(|error| error.to_string())
        })
        .collect()
}

fn rewrite_instance(
    rewrite: &mut Rewrite<'_>,
    instance: &EntityInstance,
) -> Result<EntityInstance, String> {
    let value = serde_json::to_value(instance).map_err(|error| error.to_string())?;
    serde_json::from_value(rewrite.value(&value)).map_err(|error| error.to_string())
}

/// What an export rewrites in every string it carries over, beside the derived identities.
#[derive(Debug, Clone, Copy)]
pub struct ExportRewrites<'a> {
    /// The directory home paths under which are rewritten relative to it.
    pub workspace: Option<&'a Path>,
    /// The directory `~/` and `$HOME/` name.
    pub home_directory: Option<&'a Path>,
    /// Literal text replaced on the way, such as a name that may not enter public history.
    pub fixups: &'a BTreeMap<String, String>,
}

/// Export the `aep.project/2` store `source` into a new tree authority at `target`.
///
/// # Errors
/// The source cannot be captured, a subject cannot be rewritten, or the tree refuses the import.
#[allow(clippy::too_many_lines)] // The export reads, rewrites and writes in one visible order.
pub fn export_to_tree(
    source: &AuthoritySession,
    target: &Path,
    logical_scope: &str,
    tenant: &str,
    lifecycles: &LifecycleRegistry,
    rewrites: ExportRewrites<'_>,
) -> Result<ExportReport, String> {
    let snapshot = source.complete_snapshot()?;
    let kinds = TypedKinds::of(lifecycles);
    let mut report = ExportReport::default();

    // Identities first: every artifact's counted identity and its derived one.
    let mut typed: BTreeMap<String, String> = BTreeMap::new();
    let mut untyped = BTreeSet::new();
    for held in &snapshot.histories {
        if held.history.subject.entity != STORED_AS {
            continue;
        }
        let fields = document_fields(&held.terminal)?;
        let Some(kind) = kind_of(&fields).filter(|kind| kinds.contains(kind)) else {
            // Kept as the generic contract entity, and named in the report.
            untyped.insert(kind_of(&fields).unwrap_or_else(|| "(no type)".to_owned()));
            continue;
        };
        let locator = fields
            .get(METADATA_KEY)
            .and_then(|packed| packed.get("metadata"))
            .and_then(|metadata| metadata.get("locator"))
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{} carries no locator", held.history.subject.id))?;
        let locator: aep_domain::entity::EntityLocator = locator
            .parse()
            .map_err(|error| format!("{locator}: {error}"))?;
        let natural = aep_backend_memory::store::natural_entity_id(&locator).to_string();
        report
            .identities
            .insert(held.history.subject.id.clone(), natural);
        typed.insert(held.history.subject.id.clone(), kind);
    }

    report.untyped = untyped.into_iter().collect();

    let mut rewrite = Rewrite {
        identities: &report.identities.clone(),
        fixups: rewrites.fixups,
        workspace: rewrites.workspace.map(Path::to_owned),
        home_directory: rewrites.home_directory.map(Path::to_owned),
        found: BTreeMap::new(),
    };
    let mut histories = Vec::new();
    for held in &snapshot.histories {
        let subject = &held.history.subject;
        if subject.entity == INVOCATION_AS || subject.entity == PROJECTION_METADATA_AS {
            report.skipped += 1;
            continue;
        }
        let evidence = rewrite_evidence(&mut rewrite, evidence_of(&held.history))?;
        let history = if let Some(kind) = typed.get(&subject.id) {
            let packed = rewrite.value(&Value::Object(document_fields(&held.terminal)?));
            let packed = packed.as_object().cloned().unwrap_or_default();
            let status = document_state(&held.terminal)?;
            let id = report.identities[&subject.id].clone();
            let evidence = retype_evidence(evidence, &id, kind)?;
            let document = serde_json::json!({
                "lifecycle_state": status,
                "fields": packed,
                "events": [],
            });
            let revision = packed
                .get(METADATA_KEY)
                .and_then(|p| p.get("metadata"))
                .and_then(|m| m.get("revision"))
                .and_then(Value::as_u64)
                .unwrap_or(1);
            let instance = EntityInstance {
                entity: kind.clone(),
                version: 1,
                id: id.clone(),
                lifecycle_state: status,
                revision,
                fields: typed_fields(&packed, document),
            };
            report.artifacts += 1;
            anchored(
                Subject::new(kind, &id).map_err(|error| error.to_string())?,
                instance,
                evidence,
            )
        } else {
            let instance = rewrite_instance(&mut rewrite, &held.terminal)?;
            report.records += 1;
            anchored(
                Subject::new(&instance.entity, &instance.id).map_err(|error| error.to_string())?,
                instance,
                evidence,
            )
        };
        histories.push(history);
    }
    report.home_paths = rewrite.found;
    report.provenance = rebind_legacy(&mut histories)?;

    let identity = crate::typed::prepare_tree(target, tenant)?;
    crate::typed::provision_tree(
        target,
        logical_scope.to_owned(),
        tenant.to_owned(),
        identity.clone(),
        crate::typed::provisioning_context("aep-plan-store-export"),
    )?;
    let authority = Authority {
        logical_scope: logical_scope.to_owned(),
        tenant: tenant.to_owned(),
        stream_identity: identity.clone(),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| format!("constructing the export runtime: {error}"))?;
    runtime.block_on(async {
        let concrete = Arc::new(
            eventlog_tree::TreeEventStore::open(target)
                .await
                .map_err(|error| format!("opening the tree store: {error}"))?,
        );
        concrete
            .attach_inline_existing(Arc::new(ErRecordedProjector::new()))
            .await
            .map_err(|error| format!("attaching the recorded projection: {error}"))?;
        let backend: Arc<dyn entity_eventlog::EventlogBackend> = concrete;
        let store = EventlogRecordedStore::open(backend, authority, CAPTURE_LIMITS)
            .await
            .map_err(|error| format!("opening the recorded tree store: {error:?}"))?;
        store
            .operation(crate::typed::provisioning_context("aep-plan-store-export"))
            .import_anchors(histories)
            .await
            .map_err(|error| format!("importing into the tree store: {error:?}"))?;
        Ok::<(), String>(())
    })?;
    report.stream_identity = identity;
    Ok(report)
}

/// Replace each string equal to a rebound digest by its new digest.
fn replace_digests(value: &mut Value, rebound: &BTreeMap<String, String>) {
    match value {
        Value::String(text) => {
            if let Some(new) = rebound.get(text.as_str()) {
                new.clone_into(text);
            }
        }
        Value::Array(items) => items
            .iter_mut()
            .for_each(|item| replace_digests(item, rebound)),
        Value::Object(map) => map
            .values_mut()
            .for_each(|item| replace_digests(item, rebound)),
        _ => {}
    }
}

/// A legacy roster's `roster_digest`, over its entries, its authority and its boundary.
fn roster_digest(fields: &Value) -> Result<aep_contract::migration::DigestV1, String> {
    use aep_contract::migration::{digest_parts_v1, DigestV1};
    let text = |value: &Value, key: &str| -> Result<Vec<u8>, String> {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(|text| text.as_bytes().to_vec())
            .ok_or_else(|| format!("legacy roster has no {key}"))
    };
    let mut parts = Vec::new();
    for entry in fields
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| "legacy roster has no entries".to_owned())?
    {
        parts.push(text(entry, "record_id")?);
        parts.push(text(entry, "coordinate_subject_id")?);
        parts.push(text(entry, "evidence_blob_subject_id")?);
        let digest = DigestV1::parse(
            entry
                .get("envelope_digest")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )
        .map_err(|error| format!("legacy roster entry digest: {error:?}"))?;
        parts.push(digest.as_bytes().to_vec());
    }
    let authority = fields
        .get("authority")
        .ok_or_else(|| "legacy roster has no authority".to_owned())?;
    parts.push(text(authority, "logical_scope")?);
    parts.push(text(authority, "tenant")?);
    parts.push(text(authority, "stream_identity")?);
    parts.push(text(fields, "boundary_id")?);
    digest_parts_v1("aep.migration.legacy-id-reservations/1", &parts)
        .map_err(|error| format!("digesting legacy roster: {error:?}"))
}

/// Derive again every digest that binds rewritten legacy bytes, and return how many changed.
///
/// A legacy evidence blob's `envelope_digest` is the digest of its `exact_bytes`, and its
/// coordinate and roster entry repeat it; a roster's `roster_digest` covers its entries. The
/// formulas are the ones `validated_legacy_boundary_snapshot` checks.
fn rebind_legacy(histories: &mut [SubjectHistory]) -> Result<usize, String> {
    use aep_contract::migration::{digest_parts_v1, HexBytesV1};
    let mut rebound: BTreeMap<String, String> = BTreeMap::new();
    for history in histories.iter_mut() {
        if history.subject.entity != crate::LEGACY_EVIDENCE_AS {
            continue;
        }
        let HistoryOrigin::Imported(anchor) = &mut history.origin else {
            continue;
        };
        let fields = &mut anchor.instance.fields;
        let bytes = fields
            .get("exact_bytes")
            .and_then(Value::as_str)
            .map(HexBytesV1::parse)
            .transpose()
            .map_err(|error| format!("legacy evidence bytes: {error:?}"))?
            .ok_or_else(|| "legacy evidence has no exact bytes".to_owned())?
            .into_bytes();
        let digest = digest_parts_v1(
            "aep.migration.legacy-evidence/1",
            std::slice::from_ref(&bytes),
        )
        .map_err(|error| format!("digesting legacy evidence: {error:?}"))?
        .as_wire();
        let held = fields
            .get("envelope_digest")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if held != digest {
            rebound.insert(held, digest.clone());
            fields.insert("envelope_digest".to_owned(), Value::String(digest));
            fields.insert("byte_length".to_owned(), Value::from(bytes.len() as u64));
        }
    }
    if rebound.is_empty() {
        return Ok(0);
    }
    for history in histories.iter_mut() {
        if !history.subject.entity.starts_with("aep.migration.") {
            continue;
        }
        let HistoryOrigin::Imported(anchor) = &mut history.origin else {
            continue;
        };
        let mut fields = Value::Object(std::mem::take(&mut anchor.instance.fields));
        replace_digests(&mut fields, &rebound);
        if history.subject.entity == crate::LEGACY_ROSTER_AS {
            let digest = roster_digest(&fields)?;
            if let Value::Object(map) = &mut fields {
                map.insert("roster_digest".to_owned(), Value::String(digest.as_wire()));
            }
        }
        if let Value::Object(map) = fields {
            anchor.instance.fields = map;
        }
        let mut evidence =
            serde_json::to_value(&anchor.evidence).map_err(|error| error.to_string())?;
        replace_digests(&mut evidence, &rebound);
        anchor.evidence = serde_json::from_value(evidence).map_err(|error| error.to_string())?;
    }
    Ok(rebound.len())
}

fn anchored(
    subject: Subject,
    instance: EntityInstance,
    evidence: Vec<LegacyEvidence>,
) -> SubjectHistory {
    SubjectHistory {
        subject,
        origin: HistoryOrigin::Imported(LegacyAnchor {
            instance,
            completeness: LegacyCompleteness::AvailableEvidenceOnly,
            order: LegacyOrderDeclaration::PerKindOnly,
            evidence,
        }),
        records: Vec::new(),
    }
}

/// The contract's packed fields a one-field instance carries in its document.
fn document_fields(instance: &EntityInstance) -> Result<serde_json::Map<String, Value>, String> {
    instance
        .fields
        .get("document")
        .and_then(|document| document.get("fields"))
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| format!("{}:{} has no document", instance.entity, instance.id))
}

fn document_state(instance: &EntityInstance) -> Result<String, String> {
    instance
        .fields
        .get("document")
        .and_then(|document| document.get("lifecycle_state"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{}:{} has no state", instance.entity, instance.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The fixture paths are assembled here rather than written out, so this file carries no
    // literal home path for a personal-path scan to find.
    fn under(root: &str, rest: &str) -> String {
        format!("/{root}/{rest}")
    }

    fn home(rest: &str) -> String {
        under("home", rest)
    }

    fn rewrite(
        text: &str,
        identities: &BTreeMap<String, String>,
    ) -> (String, BTreeMap<String, String>) {
        let mut rewrite = Rewrite {
            identities,
            fixups: &BTreeMap::new(),
            workspace: Some(PathBuf::from(home("ada/work"))),
            home_directory: Some(PathBuf::from(home("ada"))),
            found: BTreeMap::new(),
        };
        let out = rewrite.text(text);
        (out, rewrite.found)
    }

    #[test]
    fn a_home_path_and_a_fixup_inside_hex_legacy_bytes_are_rewritten_and_stay_hex() {
        use aep_contract::migration::HexBytesV1;
        let fixups = BTreeMap::from([("acme/x".to_owned(), "an adopting project".to_owned())]);
        let mut rewrite = Rewrite {
            identities: &BTreeMap::new(),
            fixups: &fixups,
            workspace: None,
            home_directory: Some(PathBuf::from(home("ada"))),
            found: BTreeMap::new(),
        };
        let raw = format!(
            "{{\"ref\":\"{}\",\"in\":\"acme/x\"}}",
            home("ada/.cache/notes.md")
        );
        let out = rewrite.string(&HexBytesV1::new(raw.into_bytes()).as_wire());
        let decoded = String::from_utf8(
            HexBytesV1::parse(&out)
                .expect("the rewritten value is still hex")
                .into_bytes(),
        )
        .expect("the rewritten bytes are still UTF-8");
        assert!(!decoded.contains(&home("ada")), "{decoded}");
        assert!(decoded.contains("\"ref\":\"home-path:sha256:"), "{decoded}");
        assert!(
            decoded.contains("\"in\":\"an adopting project\""),
            "{decoded}"
        );
    }

    #[test]
    fn hex_bytes_that_are_not_text_or_need_no_rewrite_are_kept_as_they_were() {
        use aep_contract::migration::HexBytesV1;
        let mut rewrite = Rewrite {
            identities: &BTreeMap::new(),
            fixups: &BTreeMap::new(),
            workspace: None,
            home_directory: None,
            found: BTreeMap::new(),
        };
        let binary = HexBytesV1::new(vec![0xff, 0xfe, b'/']).as_wire();
        assert_eq!(rewrite.string(&binary), binary);
        let clean = HexBytesV1::new(b"nothing to rewrite".to_vec()).as_wire();
        assert_eq!(rewrite.string(&clean), clean);
    }

    #[test]
    fn rewritten_legacy_bytes_are_bound_to_new_digests_through_coordinate_and_roster() {
        use aep_contract::migration::{digest_parts_v1, DigestV1, HexBytesV1};
        let digest = |bytes: &[u8]| {
            digest_parts_v1("aep.migration.legacy-evidence/1", &[bytes.to_vec()])
                .expect("digest")
                .as_wire()
        };
        let old = digest(b"the original line");
        let new_bytes = b"the rewritten line, longer".to_vec();
        let instance = |entity: &str, id: &str, fields: Value| EntityInstance {
            entity: entity.to_owned(),
            version: 1,
            id: id.to_owned(),
            lifecycle_state: "recorded".to_owned(),
            revision: 1,
            fields: fields.as_object().cloned().expect("fields are an object"),
        };
        let history = |entity: &str, id: &str, fields: Value| {
            anchored(
                Subject::new(entity, id).expect("subject"),
                instance(entity, id, fields),
                Vec::new(),
            )
        };
        let mut histories = vec![
            history(
                crate::LEGACY_COORDINATE_AS,
                "coordinate-1",
                serde_json::json!({ "envelope_digest": old }),
            ),
            history(
                crate::LEGACY_EVIDENCE_AS,
                "blob-1",
                serde_json::json!({
                    "exact_bytes": HexBytesV1::new(new_bytes.clone()).as_wire(),
                    "byte_length": 17,
                    "envelope_digest": old,
                }),
            ),
            history(
                crate::LEGACY_ROSTER_AS,
                "roster-1",
                serde_json::json!({
                    "boundary_id": "boundary-1",
                    "authority": { "logical_scope": "s", "tenant": "t", "stream_identity": "i" },
                    "entries": [{
                        "record_id": "record-1",
                        "coordinate_subject_id": "coordinate-1",
                        "evidence_blob_subject_id": "blob-1",
                        "envelope_digest": old,
                    }],
                    "roster_digest": "stale",
                }),
            ),
        ];
        assert_eq!(rebind_legacy(&mut histories), Ok(1));
        let fields = |index: usize| match &histories[index].origin {
            HistoryOrigin::Imported(anchor) => Value::Object(anchor.instance.fields.clone()),
            HistoryOrigin::Genesis => panic!("an export writes anchors"),
        };
        let new = digest(&new_bytes);
        assert_eq!(fields(0)["envelope_digest"], new);
        assert_eq!(fields(1)["envelope_digest"], new);
        assert_eq!(fields(1)["byte_length"], new_bytes.len() as u64);
        assert_eq!(fields(2)["entries"][0]["envelope_digest"], new);
        let expected = digest_parts_v1(
            "aep.migration.legacy-id-reservations/1",
            &[
                b"record-1".to_vec(),
                b"coordinate-1".to_vec(),
                b"blob-1".to_vec(),
                DigestV1::parse(&new).expect("digest").as_bytes().to_vec(),
                b"s".to_vec(),
                b"t".to_vec(),
                b"i".to_vec(),
                b"boundary-1".to_vec(),
            ],
        )
        .expect("digest")
        .as_wire();
        assert_eq!(fields(2)["roster_digest"], expected);
    }

    #[test]
    fn a_workspace_path_becomes_relative_and_another_home_path_a_digest() {
        let other = under("Users", "bob/notes.md");
        let text = format!("see {} and {other}", home("ada/work/aep/src/lib.rs"));
        let (out, found) = rewrite(&text, &BTreeMap::new());
        assert!(out.contains("workspace:aep/src/lib.rs"), "{out}");
        assert!(out.contains("home-path:"), "{out}");
        assert!(!out.contains(&under("Users", "bob")), "{out}");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn a_placeholder_home_path_names_nobody_and_is_left_alone() {
        let placeholder = format!("refuse `{}` and `$HOME`,", home("<user>/"));
        let text = format!("{placeholder} then {}", home("ada/x/"));
        let (out, found) = rewrite(&text, &BTreeMap::new());
        assert!(out.starts_with(&placeholder), "{out}");
        assert!(!out.contains(&home("ada")), "{out}");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_tilde_or_home_variable_path_is_read_as_the_home_directory_it_names() {
        let tilde = format!("{}/work/aep/x.sh", '~');
        let variable = format!("{}/.cache/y", "$HOME");
        let windows = format!("C:\\{}\\bob\\z", "Users");
        let text = format!("run {tilde}, then {variable} and {windows}");
        let (out, found) = rewrite(&text, &BTreeMap::new());
        assert!(
            out.starts_with("run workspace:aep/x.sh, then home-path:sha256:"),
            "{out}"
        );
        assert!(!out.contains(&format!("{}/", '~')), "{out}");
        assert!(!out.contains(&format!("{}/", "$HOME")), "{out}");
        assert!(!out.contains("Users"), "{out}");
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn evidence_naming_the_artifact_takes_its_typed_entity_and_nothing_else_does() {
        let event = |id: &str| {
            LegacyEvidence::Event(
                serde_json::from_value(serde_json::json!({
                    "entity": STORED_AS, "version": 1, "id": id, "revision": 1, "type": "e", "from_state": null, "to_state": "draft", "changed": {}, "removed": [], "args": {}, "payload": null
                }))
                .expect("a domain event"),
            )
        };
        let retyped = retype_evidence(vec![event("ent-a"), event("ent-b")], "ent-a", "story")
            .expect("retyped");
        let entities: Vec<String> = retyped
            .iter()
            .map(|item| serde_json::to_value(item).expect("json").to_string())
            .collect();
        assert!(
            entities[0].contains("\"entity\":\"story\""),
            "{}",
            entities[0]
        );
        assert!(
            entities[1].contains(STORED_AS),
            "another artifact's event keeps its type: {}",
            entities[1]
        );
    }

    #[test]
    fn a_fixup_replaces_its_literal_everywhere_and_nothing_else() {
        let fixups = BTreeMap::from([("acme/x".to_owned(), "an adopting project".to_owned())]);
        let mut rewrite = Rewrite {
            identities: &BTreeMap::new(),
            fixups: &fixups,
            workspace: None,
            home_directory: None,
            found: BTreeMap::new(),
        };
        assert_eq!(
            rewrite.text("in `acme/x` twice: acme/x; acme/y stays"),
            "in `an adopting project` twice: an adopting project; acme/y stays"
        );
    }

    #[test]
    fn a_counted_identity_is_replaced_by_its_derived_one_everywhere() {
        let identities = BTreeMap::from([("01MEM0001".to_owned(), "ent-abc".to_owned())]);
        let (out, _) = rewrite("relates 01MEM0001 to 01MEM0001", &identities);
        assert_eq!(out, "relates ent-abc to ent-abc");
    }
}
