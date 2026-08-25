//! What happened to the plan, in the order it happened.
//!
//! # The gap this closes, and the one it does not
//!
//! `docs/plan/gap-register.md:37` records three things the markdown store lacks: a journal, an
//! audit join, and a history. This is the journal, and with it the history — *what happened to
//! `story:x`, and when, and who said so* is answerable without reading a repository's whole log.
//!
//! It is **not** the audit join and it is not `CommandService`. Those need command envelopes,
//! idempotent replay and revision conflicts, which is a larger change with an architectural
//! question inside it: the contract's `execute` is async and this store is synchronous file IO.
//! That question is worth answering deliberately rather than in passing, so the row stays open and
//! says which third is closed.
//!
//! # Why not git
//!
//! The crate's own description says *git as the log*, and git is a fine log for a human reading
//! diffs. It is a poor one for a tool: a rename is a guess, a squash loses the moves, a rebase
//! rewrites the times, and none of it answers *which of these was a status move* without parsing
//! markdown out of a patch. The journal records the change the store actually made, in the shape
//! the protocol reasons about.
//!
//! # Append-only, and what that costs
//!
//! Entries are appended and never rewritten. That is invariant 16 — *nothing is physically
//! deleted* — applied to the record of what was done, and it means a mistake is corrected by a
//! later entry rather than by editing an earlier one. The file grows; a plan that produces a
//! thousand moves a year produces a file measured in tens of kilobytes, which is the right trade
//! for a record nobody can quietly amend.

use std::fmt;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use aep_domain::artifact::{ArtifactId, ArtifactKind, ArtifactStatus, RelationKind};

/// Where the journal lives, relative to the store root.
pub const JOURNAL: &str = "journal.jsonl";

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
    },
    /// An edge was added.
    Related {
        /// What the edge means.
        relation: RelationKind,
        /// Where it points.
        target: String,
    },
    /// Its markdown body was replaced.
    BodyReplaced,
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created { status } => write!(f, "created as {status}"),
            Self::Moved { from, to } => write!(f, "moved {from} -> {to}"),
            Self::Related { relation, target } => write!(f, "{relation} {target}"),
            Self::BodyReplaced => f.write_str("body replaced"),
        }
    }
}

/// Appends an entry, creating the journal if this is the first thing that ever happened.
///
/// # Errors
///
/// Whatever the filesystem said. A write that failed is **not** swallowed: a journal that silently
/// stops recording is worse than one that is not there, because the first looks like a plan where
/// nothing happened.
pub fn append(root: &Path, entry: &Entry) -> std::io::Result<()> {
    let path = root.join(JOURNAL);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(entry).map_err(std::io::Error::other)?;
    line.push('\n');
    // Append, never rewrite: two processes writing at once interleave whole lines rather than
    // corrupting each other's, which is the property JSONL is chosen for.
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(line.as_bytes())
}

/// Every entry, oldest first.
///
/// A line that does not parse is **skipped rather than fatal**, and that is deliberate: a journal is
/// append-only and long-lived, so a single corrupt line — a half-written entry from a killed
/// process — must not make the whole history unreadable. The count of skipped lines is returned so
/// a caller can say so rather than quietly reporting a shorter history.
#[must_use]
pub fn read(root: &Path) -> (Vec<Entry>, usize) {
    let path: PathBuf = root.join(JOURNAL);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return (Vec::new(), 0);
    };
    let mut entries = Vec::new();
    let mut unreadable = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str(line) {
            Ok(entry) => entries.push(entry),
            Err(_) => unreadable += 1,
        }
    }
    (entries, unreadable)
}

/// Every entry about one artifact, oldest first.
#[must_use]
pub fn history(root: &Path, artifact: &ArtifactId) -> (Vec<Entry>, usize) {
    let (entries, unreadable) = read(root);
    (
        entries
            .into_iter()
            .filter(|entry| &entry.artifact == artifact)
            .collect(),
        unreadable,
    )
}
