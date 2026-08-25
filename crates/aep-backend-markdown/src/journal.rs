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

use std::collections::BTreeMap;

use aep_domain::artifact::{ArtifactId, ArtifactKind, ArtifactStatus, RelationKind};
use aep_domain::evidence::EvidenceKind;

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
        /// What the decision rested on, split by where it came from.
        ///
        /// `#[serde(default)]` because entries written before provenance existed have no such
        /// account, and an empty one is the honest reading of them: *nothing was recorded about how
        /// this was decided*. Rewriting them to claim otherwise is exactly what append-only forbids.
        #[serde(default)]
        decided_on: Provenance,
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
    /// Evidence was recorded **about** this artifact.
    ///
    /// The subject is `Entry::artifact`, not a field here, and that is the whole point: evidence
    /// that does not name what it is about cannot be counted for anything, and a count with no
    /// subject is the gap this closes. Because the subject is the entry's own artifact,
    /// [`history`] already shows it and already filters it.
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
    /// Evidence found in this journal, naming this artifact.
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
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created { status } => write!(f, "created as {status}"),
            Self::Moved {
                from,
                to,
                decided_on,
            } => {
                write!(f, "moved {from} -> {to}")?;
                if decided_on.leans_on_an_assertion() {
                    f.write_str(" (on asserted evidence)")?;
                }
                Ok(())
            }
            Self::Related { relation, target } => write!(f, "{relation} {target}"),
            Self::BodyReplaced => f.write_str("body replaced"),
            Self::Evidence {
                kind,
                source,
                reference,
            } => {
                write!(f, "{} recorded from {source}", kind.as_str())?;
                if let Some(reference) = reference {
                    write!(f, " ({reference})")?;
                }
                Ok(())
            }
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

/// How much evidence this journal holds **about one artifact**, by kind.
///
/// The counting rule is deliberately the dullest one available: one recorded entry is one piece of
/// evidence. No deduplication by source, no expiry, no weighting. Each of those is a judgement about
/// what makes evidence good, and this function's job is only to say what is there — a judgement
/// belongs in a rung's `requires:`, where it is written down and can be argued with, not buried in a
/// counter.
///
/// Evidence is **not** invalidated by a later move. A test result recorded before a story went to
/// `implemented` still counts if it is moved back and forward again, and that is the append-only
/// reading: the observation happened, and nothing that happened afterwards un-happens it. A rung
/// that needs *fresh* evidence should say so with a time guard, which is a thing the ladder can
/// already express.
#[must_use]
pub fn evidence_on_hand(root: &Path, artifact: &ArtifactId) -> BTreeMap<EvidenceKind, usize> {
    let (entries, _) = history(root, artifact);
    let mut counted = BTreeMap::new();
    for entry in entries {
        if let Change::Evidence { kind, .. } = entry.change {
            *counted.entry(kind).or_default() += 1;
        }
    }
    counted
}
