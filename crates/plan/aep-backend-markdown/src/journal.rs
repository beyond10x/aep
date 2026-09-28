//! What happened to the plan, in the order it happened, in one vocabulary.
//!
//! A Git-native store (`aep.project/5`) keeps no log beside its documents: a status move is a
//! [`Transition`] appended to the document it moved, and a recorded observation is one immutable
//! file under `evidence/<kind>/<name>/`. [`read_git`] and [`history_git`] read both back as
//! [`Entry`]s, so `aep plan artifact history`, `explain` and the evidence gate have one type to read
//! whatever produced the record.
//!
//! The store-wide `journal.jsonl` of `aep.project/1` is gone: nothing here writes one, and the only
//! reader left is the migration that rewrites such a store (`aep plan store migrate git`). Its lines
//! were an [`Entry`] or an `entity_core::DomainEvent` carrying one under `payload.change`;
//! [`entry_of`] is the reading of the second shape, public for that migration.
//!
//! # Append-only
//!
//! Evidence files are written once and never again; a transition list only grows. A mistake is
//! corrected by a later record rather than by editing an earlier one, which is invariant 16 —
//! *nothing is physically deleted* — applied to the record of what was done. `aep plan artifact
//! validate` compares each committed evidence file with its Git blob to hold that.

use std::fmt;
use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use aep_domain::artifact::{ArtifactId, ArtifactKind, ArtifactStatus, RelationKind};
use aep_domain::evidence::EvidenceKind;
use aep_domain::review::ReviewOutcome;

/// The store-wide log an `aep.project/1` store kept beside its documents.
///
/// Nothing in this crate reads or writes it. It is named so an open can refuse a planning directory
/// that still holds one, rather than treating a journal store as a Git-native one.
pub const LEGACY_JOURNAL: &str = "journal.jsonl";

/// One thing that happened to one artifact.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    /// When, ISO-8601, as the caller observed it. The store has no clock: this is read at the edge
    /// and handed over, exactly as a dated rung's instant is.
    pub at: String,
    /// Who. Free text, because the store cannot verify an identity and a field that looks verified
    /// and is not is worse than one that plainly is not.
    pub actor: String,
    /// Which artifact.
    pub artifact: ArtifactId,
    /// Its kind, recorded here so a reader of the journal alone can group without loading files
    /// that may since have been archived.
    pub kind: ArtifactKind,
    /// The revision the artifact was at **after** the change.
    pub revision: u64,
    /// What happened.
    pub change: Change,
}

/// What kind of thing happened.
///
/// Deliberately closed, and this is the one place in this repository where closing a vocabulary
/// needs no argument: a journal entry is written by this crate and read by this crate, so an
/// unknown variant would mean code that wrote a change no code can read.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "change", rename_all = "snake_case")]
pub enum Change {
    /// The artifact was written for the first time.
    Created {
        /// Where it started.
        status: ArtifactStatus,
    },
    /// Its status moved.
    Moved {
        /// Where it was.
        from: ArtifactStatus,
        /// Where it went.
        to: ArtifactStatus,
        /// What the decision rested on, split by where it came from.
        ///
        /// `#[serde(default)]` because entries written before provenance existed have no such
        /// account, and an empty one is the honest reading of them: *nothing was recorded about how
        /// this was decided*. Rewriting them to claim otherwise is exactly what append-only forbids.
        #[serde(default)]
        decided_on: Provenance,
        /// What actually performed the move, when that was not the actor.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        executor: Option<String>,
        /// The wider activity the command that made the move named.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        correlation: Option<String>,
    },
    /// An edge was added.
    Related {
        /// What the edge means.
        relation: RelationKind,
        /// Where it points.
        target: String,
    },
    /// An edge was taken back.
    ///
    /// A separate variant rather than a `Related` with a flag, for the reason the whole enum is
    /// closed: a reader of the history asks *what happened*, and "related, negated" is a sentence
    /// that has to be decoded before it can be read. The pair is written down as append-only means
    /// it — the `related` entry stays exactly where it was, and this one says it was undone.
    Unrelated {
        /// What the edge meant.
        relation: RelationKind,
        /// Where it pointed.
        target: String,
    },
    /// Its markdown body was replaced.
    BodyReplaced,
    /// Evidence was recorded **about** this artifact.
    ///
    /// The subject is `Entry::artifact`, not a field here, and that is the whole point: evidence
    /// that does not name what it is about cannot be counted for anything, and a count with no
    /// subject is the gap this closes. Because the subject is the entry's own artifact,
    /// [`history_git`] already shows it and already filters it.
    Evidence {
        /// What kind of observation it is.
        kind: EvidenceKind,
        /// Where it came from, as the recorder is willing to say — `task check`, a CI run, a
        /// person's name. Free text for the same reason `actor` is: the store cannot verify it, and
        /// a field that looks verified and is not is worse than one that plainly is not.
        source: String,
        /// Where to go and look — a URL, a run id, a file path. Optional, because evidence with no
        /// retrievable address is still better attributed than a bare number.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
        /// The review this record answers, on a `review_outcome`.
        ///
        /// `#[serde(default)]` for the reason `decided_on` has one: entries written before the
        /// kind existed name no review, and an empty one is the honest reading of them.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        review: Option<ArtifactId>,
        /// What became of that review.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcome: Option<ReviewOutcome>,
    },
}

/// What a decision rested on, and — the point of the type — **where each part came from**.
///
/// # The gap this closes
///
/// `docs/plan/gap-register.md:39` says a story's `implemented` is a claim nothing checks. The
/// mechanism half closed when a rung could declare `requires:` and the move began refusing without
/// evidence. That left the trust root exactly where it was: `--evidence test_result=1` is a number
/// somebody typed, naming no test, about no artifact, from no run.
///
/// Recorded evidence names its subject, its source and its instant, and cannot be edited afterwards.
/// Asserted evidence is still accepted — a CI run nobody recorded is real, and refusing it would
/// only push people to record a fiction — but the two are **counted separately and both written
/// down**, so a reader of the history can always tell which kind of claim a move rested on. That is
/// provenance: not that every move is proven, but that no move can be *mistaken* for proven.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Provenance {
    /// Evidence the store holds, naming this artifact.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub recorded: BTreeMap<EvidenceKind, usize>,
    /// Evidence the caller asserted at the command line and nothing checks.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub asserted: BTreeMap<EvidenceKind, usize>,
}

impl Provenance {
    /// Everything on hand, whatever its origin — what the rung's `requires:` is decided against.
    ///
    /// Summed rather than preferring one side: two recorded test results and one asserted are three
    /// test results for the purpose of *did anybody look*, and the account of which is which is kept
    /// in the fields rather than smuggled into the total.
    #[must_use]
    pub fn total(&self) -> BTreeMap<EvidenceKind, usize> {
        let mut total = self.recorded.clone();
        for (kind, count) in &self.asserted {
            *total.entry(*kind).or_default() += *count;
        }
        total
    }

    /// Whether any part of this rested on a number nobody can go and check.
    #[must_use]
    pub fn leans_on_an_assertion(&self) -> bool {
        !self.asserted.is_empty()
    }

    /// `true` when nothing at all was recorded about how the decision was taken.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.recorded.is_empty() && self.asserted.is_empty()
    }
}

/// One status move, as a Git-native document (`aep.planning-md/3`) carries it in `transitions`.
///
/// The document is the authority in that layout — there is no journal — so the move is written
/// into the file it moved, and a diff of a move is one added line. Closed: a key this type does not
/// name would be dropped on the next write, so it is refused on read instead.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    /// When, ISO-8601, as the command carried it.
    pub at: String,
    /// Who, as the command carried it.
    pub actor: String,
    /// The document's revision **after** the move.
    pub revision: u64,
    /// Where it was.
    pub from: ArtifactStatus,
    /// Where it went.
    pub to: ArtifactStatus,
    /// What the decision rested on; absent when nothing was recorded.
    #[serde(default, skip_serializing_if = "Provenance::is_empty")]
    pub decided_on: Provenance,
    /// Brought over from an event-log store by `aep plan store migrate git`, rather than made by a
    /// move in this layout. Such a move was decided under the lifecycle of its day, which may since
    /// have changed, and the store it came from did not record the status an artifact was created
    /// in — so `validate` holds an imported move to continuity only, never to today's ladder.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub imported: bool,
    /// What actually performed the move, written only when it differs from `actor`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
    /// The wider activity the command named, written when it named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation: Option<String>,
}

/// The correlation a move carries when its caller named no wider activity.
///
/// The command context requires a correlation, so an ordinary `aep plan artifact move` supplies
/// this placeholder. It is not written into a transition: every move would otherwise carry the
/// same line, which names nothing.
pub const UNCORRELATED_MOVE: &str = "protocol-artifact-move";

/// The executor and correlation an event's sealed payload carries, as a transition records them.
///
/// The executor is kept only when it differs from `actor`; the correlation only when it is not
/// [`UNCORRELATED_MOVE`].
#[must_use]
pub fn attribution(
    payload: &serde_json::Map<String, serde_json::Value>,
    actor: &str,
) -> (Option<String>, Option<String>) {
    let text = |key: &str| payload.get(key).and_then(serde_json::Value::as_str);
    let executor = text("executor")
        .filter(|executor| *executor != actor)
        .map(str::to_owned);
    let correlation = text("correlation")
        .filter(|correlation| !correlation.is_empty() && *correlation != UNCORRELATED_MOVE)
        .map(str::to_owned);
    (executor, correlation)
}

impl Transition {
    /// The transition as one YAML flow mapping on one line.
    ///
    /// Every value is written as JSON, which is valid YAML flow syntax, so no value needs a
    /// quoting rule of its own and the line reads back as exactly this transition.
    #[must_use]
    pub fn flow(&self) -> String {
        fn json(value: &impl serde::Serialize) -> String {
            serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned())
        }
        let mut line = format!(
            "{{from: {}, to: {}, at: {}, actor: {}, revision: {}",
            json(&self.from),
            json(&self.to),
            json(&self.at),
            json(&self.actor),
            self.revision
        );
        if !self.decided_on.is_empty() {
            line.push_str(", decided_on: ");
            line.push_str(&json(&self.decided_on));
        }
        if self.imported {
            line.push_str(", imported: true");
        }
        if let Some(executor) = &self.executor {
            line.push_str(", executor: ");
            line.push_str(&json(executor));
        }
        if let Some(correlation) = &self.correlation {
            line.push_str(", correlation: ");
            line.push_str(&json(correlation));
        }
        line.push('}');
        line
    }

    /// The journal entry this transition stands for, about `artifact`.
    #[must_use]
    pub fn entry(&self, artifact: &ArtifactId, kind: &ArtifactKind) -> Entry {
        Entry {
            at: self.at.clone(),
            actor: self.actor.clone(),
            artifact: artifact.clone(),
            kind: kind.clone(),
            revision: self.revision,
            change: Change::Moved {
                from: self.from.clone(),
                to: self.to.clone(),
                decided_on: self.decided_on.clone(),
                executor: self.executor.clone(),
                correlation: self.correlation.clone(),
            },
        }
    }
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created { status } => write!(f, "created as {status}"),
            Self::Moved {
                from,
                to,
                decided_on,
                executor,
                correlation,
            } => {
                write!(f, "moved {from} -> {to}")?;
                if decided_on.leans_on_an_assertion() {
                    f.write_str(" (on asserted evidence)")?;
                }
                if let Some(executor) = executor {
                    write!(f, ", executed by {executor}")?;
                }
                if let Some(correlation) = correlation {
                    write!(f, ", correlation {correlation}")?;
                }
                Ok(())
            }
            Self::Related { relation, target } => write!(f, "{relation} {target}"),
            Self::Unrelated { relation, target } => write!(f, "no longer {relation} {target}"),
            Self::BodyReplaced => f.write_str("body replaced"),
            Self::Evidence {
                kind,
                source,
                reference,
                review,
                outcome,
            } => {
                write!(f, "{} recorded from {source}", kind.as_str())?;
                if let (Some(review), Some(outcome)) = (review, outcome) {
                    write!(f, ": {review} was {outcome}")?;
                }
                if let Some(reference) = reference {
                    write!(f, " ({reference})")?;
                }
                Ok(())
            }
        }
    }
}

/// The entry an event line stands for, when it was written by the plan's own projection.
///
/// The seal in the payload says who and when; `payload.change` says what, in this module's own
/// vocabulary; the event's coordinates say which artifact. An event without those — a provider's
/// event about something that is not a plan document — is not an entry, and reads as unreadable
/// rather than as a guess.
pub fn entry_of(event: &entity_core::DomainEvent) -> Option<Entry> {
    let payload = event.payload.as_object()?;
    let at = payload.get("recorded_at")?.as_str()?.to_owned();
    let actor = payload.get("actor")?.as_str()?.to_owned();
    let change: Change = serde_json::from_value(payload.get("change")?.clone()).ok()?;
    let artifact = ArtifactId::new(format!("{}:{}", event.entity, event.id)).ok()?;
    let kind: ArtifactKind = event.entity.parse().ok()?;
    Some(Entry {
        at,
        actor,
        artifact,
        kind,
        revision: event.revision,
        change,
    })
}

/// Every entry a Git-native store holds, in the order they happened.
///
/// There is no journal in that layout: a move is a transition in the document it moved, and an
/// observation is one file under `evidence`. So this reads one [`Change::Moved`] per transition of
/// every document under `root`, and one entry per evidence file under
/// `evidence/<kind>/<name>/`, sorted by `at`, then artifact, then revision — the same [`Entry`]
/// vocabulary every history reader answers, so a caller needs no second type.
///
/// A document or evidence file that does not parse is skipped and counted: one bad file must not
/// make the whole history unreadable, and a caller can say so.
#[must_use]
pub fn read_git(root: &Path, evidence: &Path) -> (Vec<Entry>, usize) {
    let mut entries = Vec::new();
    let mut unreadable = 0;
    for kind in subdirectories(root) {
        for path in files_with(&kind, "md") {
            match document_entries(&path) {
                Some(found) => entries.extend(found),
                None => unreadable += 1,
            }
        }
    }
    for kind in subdirectories(evidence) {
        for name in subdirectories(&kind) {
            let (found, skipped) = evidence_in(&name);
            entries.extend(found);
            unreadable += skipped;
        }
    }
    sort_entries(&mut entries);
    (entries, unreadable)
}

/// Every entry about one artifact in a Git-native store, oldest first.
///
/// Reads the artifact's own document and its own evidence directory and nothing else. A document
/// filed somewhere other than `<namespace>/<name>.md` — under an accepted kind alias — is found by
/// falling back to [`read_git`].
#[must_use]
pub fn history_git(root: &Path, evidence: &Path, artifact: &ArtifactId) -> (Vec<Entry>, usize) {
    let document = root
        .join(artifact.namespace())
        .join(format!("{}.md", artifact.name()));
    if !document.is_file() {
        let (entries, unreadable) = read_git(root, evidence);
        return (
            entries
                .into_iter()
                .filter(|entry| &entry.artifact == artifact)
                .collect(),
            unreadable,
        );
    }
    let mut unreadable = 0;
    let mut entries = document_entries(&document).unwrap_or_else(|| {
        unreadable += 1;
        Vec::new()
    });
    let (found, skipped) = evidence_in(&evidence_directory(evidence, artifact));
    entries.extend(found);
    unreadable += skipped;
    entries.retain(|entry| &entry.artifact == artifact);
    sort_entries(&mut entries);
    (entries, unreadable)
}

/// How much evidence a Git-native store holds about one artifact, by kind.
///
/// One evidence file is one piece of evidence: no deduplication by source, no expiry, no weighting.
/// Each of those is a judgement that belongs in a rung's `requires:`, not buried in a counter.
#[must_use]
pub fn evidence_on_hand_git(
    root: &Path,
    evidence: &Path,
    artifact: &ArtifactId,
) -> BTreeMap<EvidenceKind, usize> {
    let (entries, _) = history_git(root, evidence, artifact);
    let mut counted = BTreeMap::new();
    for entry in entries {
        if let Change::Evidence { kind, .. } = entry.change {
            *counted.entry(kind).or_default() += 1;
        }
    }
    counted
}

/// Writes one evidence record into a Git-native store and answers where it went.
///
/// `<evidence>/<kind>/<name>/<compact at>-<first 12 hex of the SHA-256 of the file>.json`, where
/// the file is the entry as pretty JSON with a trailing newline. Written through a temporary file
/// in the same directory and then linked into place, so a reader never sees half a record.
///
/// A record is never rewritten: writing the same entry again is a no-op answering the same path,
/// and a different file already at that path is refused.
///
/// # Errors
///
/// Whatever the filesystem said, and [`std::io::ErrorKind::AlreadyExists`] for a different file at
/// the record's path.
pub fn write_evidence(evidence: &Path, entry: &Entry) -> std::io::Result<PathBuf> {
    write_evidence_occurrence(evidence, entry, 0)
}

/// [`write_evidence`] for the `occurrence`-th identical copy of one record.
///
/// A store may hold the same record more than once — two identical `review_outcome` lines in an
/// imported history — and each counts as one piece of evidence, so a migration must keep every
/// copy rather than collapse them into one file. Occurrence 0 is [`write_evidence`]'s own path;
/// occurrence `n` appends `-<n>` to the file stem.
///
/// # Errors
///
/// As [`write_evidence`].
pub fn write_evidence_occurrence(
    evidence: &Path,
    entry: &Entry,
    occurrence: usize,
) -> std::io::Result<PathBuf> {
    use sha2::{Digest as _, Sha256};
    use std::io::Write as _;

    let mut text = serde_json::to_string_pretty(entry).map_err(std::io::Error::other)?;
    text.push('\n');
    let digest = hex(&Sha256::digest(text.as_bytes()));
    let mut compact: String = entry
        .at
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    if compact.is_empty() {
        compact.push_str("undated");
    }
    let directory = evidence_directory(evidence, &entry.artifact);
    let stem = if occurrence == 0 {
        format!("{compact}-{}", &digest[..12])
    } else {
        format!("{compact}-{}-{occurrence}", &digest[..12])
    };
    let path = directory.join(format!("{stem}.json"));

    let identical = |path: &Path| -> std::io::Result<PathBuf> {
        if std::fs::read(path)? == text.as_bytes() {
            Ok(path.to_path_buf())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "{} already holds a different evidence record, and a record is never rewritten",
                    path.display()
                ),
            ))
        }
    };
    if path.exists() {
        return identical(&path);
    }

    std::fs::create_dir_all(&directory)?;
    let temporary = directory.join(format!(".{stem}.{}.tmp", std::process::id()));
    let mut file = std::fs::File::create(&temporary)?;
    let written = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = written {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    // A hard link refuses an existing destination, which a rename would silently replace: two
    // writers racing on one record both land, and the loser compares instead of overwriting.
    let linked = std::fs::hard_link(&temporary, &path);
    let _ = std::fs::remove_file(&temporary);
    match linked {
        Ok(()) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => identical(&path),
        Err(error) => Err(error),
    }
}

/// Where the evidence about `artifact` is kept.
fn evidence_directory(evidence: &Path, artifact: &ArtifactId) -> PathBuf {
    evidence.join(artifact.namespace()).join(artifact.name())
}

/// The entries one document's transitions stand for, or `None` when it does not parse.
fn document_entries(path: &Path) -> Option<Vec<Entry>> {
    let text = std::fs::read_to_string(path).ok()?;
    let document = crate::document::PlanningDocument::parse(&text, path.to_str()).ok()?;
    let frontmatter = &document.frontmatter;
    Some(
        frontmatter
            .transitions
            .iter()
            .map(|transition| transition.entry(&frontmatter.id, &frontmatter.kind))
            .collect(),
    )
}

/// The evidence records in one artifact's directory, and how many did not parse.
fn evidence_in(directory: &Path) -> (Vec<Entry>, usize) {
    let mut entries = Vec::new();
    let mut unreadable = 0;
    for path in files_with(directory, "json") {
        match std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Entry>(&text).ok())
        {
            Some(entry) if matches!(entry.change, Change::Evidence { .. }) => entries.push(entry),
            _ => unreadable += 1,
        }
    }
    (entries, unreadable)
}

/// The directories directly under `directory` whose names do not start with a dot, sorted.
fn subdirectories(directory: &Path) -> Vec<PathBuf> {
    listed(directory, Path::is_dir)
}

/// The files directly under `directory` with `extension`, not starting with a dot, sorted.
fn files_with(directory: &Path, extension: &str) -> Vec<PathBuf> {
    listed(directory, |path| {
        path.is_file() && path.extension().is_some_and(|found| found == extension)
    })
}

/// The entries under `directory` that `keep` accepts, skipping dot-names, sorted.
fn listed(directory: &Path, keep: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .map(|entry| entry.path())
        .filter(|path| keep(path))
        .collect();
    found.sort();
    found
}

/// The order a Git-native history is answered in: `at`, then artifact, then revision.
fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|left, right| {
        left.at
            .cmp(&right.at)
            .then_with(|| left.artifact.cmp(&right.artifact))
            .then_with(|| left.revision.cmp(&right.revision))
    });
}

/// Lower-case hexadecimal of `bytes`, as an evidence file's name carries its digest.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
        output
    })
}
