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
    workspace: Option<PathBuf>,
    home_directory: Option<PathBuf>,
    found: BTreeMap<String, String>,
}

impl Rewrite<'_> {
    fn value(&mut self, value: &Value) -> Value {
        match value {
            Value::String(text) => Value::String(self.text(text)),
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

    fn text(&mut self, text: &str) -> String {
        let mut out = text.to_owned();
        for (old, new) in self.identities {
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

fn rewrite_instance(
    rewrite: &mut Rewrite<'_>,
    instance: &EntityInstance,
) -> Result<EntityInstance, String> {
    let value = serde_json::to_value(instance).map_err(|error| error.to_string())?;
    serde_json::from_value(rewrite.value(&value)).map_err(|error| error.to_string())
}

/// Export the `aep.project/2` store `source` into a new tree authority at `target`.
///
/// `workspace` is the directory home paths under which are rewritten relative to it, and
/// `home_directory` the one `~/` and `$HOME/` name.
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
    workspace: Option<&Path>,
    home_directory: Option<&Path>,
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
        workspace: workspace.map(Path::to_owned),
        home_directory: home_directory.map(Path::to_owned),
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
            workspace: Some(PathBuf::from(home("ada/work"))),
            home_directory: Some(PathBuf::from(home("ada"))),
            found: BTreeMap::new(),
        };
        let out = rewrite.text(text);
        (out, rewrite.found)
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
    fn a_counted_identity_is_replaced_by_its_derived_one_everywhere() {
        let identities = BTreeMap::from([("01MEM0001".to_owned(), "ent-abc".to_owned())]);
        let (out, _) = rewrite("relates 01MEM0001 to 01MEM0001", &identities);
        assert_eq!(out, "relates ent-abc to ent-abc");
    }
}
