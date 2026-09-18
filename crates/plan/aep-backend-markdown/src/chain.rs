//! The journal, linked to itself, so a line cannot be changed without saying so.
//!
//! # The gap this closes
//!
//! `docs/plan/gap-register.md:108` records the measurement: a driven run wrote `revision: 99`
//! straight into a planning document with an ordinary file write, and `protocol artifact validate`
//! exited **0** on the well-formed result. [`crate::drift`] closed half of that — a document
//! claiming a revision higher than any event records is a [`crate::drift::ForgedRevision`] and
//! fails the gate. It closed only half, because it reconciles the document
//! against the journal and **nothing protected the journal**. An actor who writes `revision: 99`
//! into `story/forge.md` *and* appends a matching event line to `journal.jsonl` is consistent with
//! himself, and consistency is all that check can measure. Reproduced here on 2026-09-17 against a
//! scratch store: the two-file edit printed `valid`, exit 0, and `--strict` printed `valid` too.
//!
//! Each record this store appends now carries the digest of the record before it
//! ([`PARENT_HASH`]) and a digest of its own bytes ([`ENTRY_HASH`]). Changing a record changes its
//! digest; leaving the digest alone makes it disagree with the bytes. [`verify`] walks the chain
//! and stops at the exact record where it stops holding, naming that record.
//!
//! # What this detects, and what it plainly does not
//!
//! It detects **tampering inside a log**: an edited record, an inserted record, a deleted record
//! in the middle, a reordering, and a record appended by something that does not seal. It detects
//! an edit to the *unsealed* lines that predate the chain, because the first sealed record seals
//! them as a block (see *Where the chain starts*).
//!
//! It does **not** detect an operator who replaces the whole log. A chain is computed from the
//! bytes it protects, so anybody who can rewrite every byte can rewrite a consistent chain over
//! them. For the same reason it does **not** detect truncation of the tail: lopping the last ten
//! records off a hundred-record journal leaves ninety records that link perfectly. Detecting
//! either needs the head of the chain recorded somewhere the log does not control — a signature, a
//! notary, a second store — and that is gap register **D-3** (*attested evidence*), which is
//! proposed and not accepted. This module deliberately implements none of it: no signature, no
//! key, no attestation. What it buys is that tampering is no longer *free* and no longer *silent*;
//! it is not that tampering is impossible.
//!
//! It also does not **prevent** anything, for the reason [`crate::drift`] gives at greater length:
//! prevention needs to know who wrote a file, which is D-3 again. This detects, after the fact,
//! in the one place that cannot be routed around — the gate.
//!
//! # Where the chain starts on a journal that already exists
//!
//! Every store in this workspace has a journal with no digests in it; refusing those, or reporting
//! them as tampered, would make this change a lie about six honest repositories. So:
//!
//! * A line carrying no [`ENTRY_HASH`] **before any sealed line** is *uncovered*. Not a finding,
//!   not a problem, not even a `--strict` class. The chain says nothing about it, and says so.
//! * The **first** record this store seals names, as its [`PARENT_HASH`], the [`anchor`] of every
//!   line before it — one digest over the whole legacy prefix. From that append onward the prefix
//!   is frozen: editing any of those 1,782 lines in this repository's own journal makes the first
//!   sealed record's parent disagree, and [`verify`] reports it.
//! * A record that carries no seal **after** the chain has begun is a break ([`Reason::Unsealed`]).
//!   Nothing in this crate writes one, so it means either an older `aep` binary appending to a
//!   chained store or a hand-edit — and if it were tolerated, stripping the digests off the tail
//!   would be a silent way to un-chain a log.
//! * A line that is not JSON at all is **debris**, not a record — a half-written line from a killed
//!   process, which [`crate::journal::read`] already skips and counts. The chain steps over it and
//!   the next sealed record re-anchors across it, so a crash does not read as an attack.
//!
//! # Why the append takes a lock
//!
//! Sealing turns *read the head, then append* into one operation that two writers must not
//! interleave: both would read the same head, both would name it as their parent, and the second
//! record would look exactly like an inserted one. A chain that cries tampering at an ordinary
//! concurrent write would be worse than no chain. [`append_sealed`] therefore holds [`LOCK`] for
//! the read and the write, and `sync_all`s before releasing it, so a record that is in the file is
//! a record that survived the power going out.
//!
//! The lock is one `create_new` and a bounded wait, which is the position `aep-cli`'s run lock
//! already took (`crates/edge/aep-cli/src/drive.rs:1438-1440`): atomic on every filesystem that
//! matters, needing no advisory locking, whose semantics do not change underneath us per
//! filesystem. No clock is read and no stale lock is stolen — this crate's `lib.rs` claims it
//! reads neither a clock nor an RNG, and a timeout that expired would be both a clock and a
//! decision. A lock left behind by a killed process is reported by name, with the file to remove.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value};
use sha2::{Digest as _, Sha256};

use crate::journal::JOURNAL;

/// The key a sealed record carries its own digest under.
pub const ENTRY_HASH: &str = "entry_hash";

/// The key a sealed record carries the previous record's digest under.
pub const PARENT_HASH: &str = "parent_hash";

/// The lock a journal append is made under, relative to the store root.
pub const LOCK: &str = "journal.lock";

/// The domain separator the per-record digest is taken over.
///
/// A version in the preimage rather than beside it: two schemes that hash the same bytes to the
/// same value are one scheme, and the day this one is replaced the old records must fail to verify
/// under the new rule rather than accidentally pass.
const ENTRY_TAG: &str = "aep.journal-entry/1";

/// The domain separator the prefix [`anchor`] is taken over.
const ANCHOR_TAG: &str = "aep.journal-anchor/1";

/// How many times [`append_sealed`] retries a held lock before giving up.
const LOCK_ATTEMPTS: u32 = 64;

/// How long it waits between those attempts.
const LOCK_WAIT: Duration = Duration::from_millis(16);

/// Why the chain stops holding at one record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Reason {
    /// The record's bytes are not the bytes that were sealed.
    Tampered {
        /// The digest the record carries.
        sealed: String,
        /// The digest its bytes produce now.
        recomputed: String,
    },
    /// The record does not name the record before it.
    ///
    /// An insertion, a deletion in the middle, or a reordering: the content of every record may be
    /// intact and the order they were written in is still not the order they are in.
    Unlinked {
        /// The previous record's own digest.
        expected: String,
        /// What this record names as its parent, when it names anything.
        named: Option<String>,
    },
    /// The first sealed record does not seal the lines before it.
    ///
    /// The lines this record anchors over — a legacy prefix, or the debris of a killed process —
    /// are not the lines it anchored over when it was written.
    PrefixEdited {
        /// The anchor the lines before this record produce now.
        expected: String,
        /// What this record names as its parent, when it names anything.
        named: Option<String>,
    },
    /// A record appended after the chain began, carrying no seal at all.
    Unsealed,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tampered { sealed, recomputed } => write!(
                f,
                "it was sealed as {sealed} and its bytes now hash to {recomputed} — the record was \
                 edited after it was written"
            ),
            Self::Unlinked { expected, named } => write!(
                f,
                "it names {} as the record before it, and the record before it is {expected} — an \
                 entry was inserted, removed or reordered",
                named.as_deref().unwrap_or("nothing")
            ),
            Self::PrefixEdited { expected, named } => write!(
                f,
                "it seals the lines before it as {}, and those lines now seal as {expected} — the \
                 journal's older lines were edited after this record froze them",
                named.as_deref().unwrap_or("nothing")
            ),
            Self::Unsealed => f.write_str(
                "it carries no seal, in a journal that seals — nothing in this store writes an \
                 unsealed record once the chain has begun, so it came from an older `aep` binary \
                 or from a hand-edit",
            ),
        }
    }
}

/// The one record where the chain stops holding.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Broken {
    /// Which line of `journal.jsonl`, counting non-empty lines from 1.
    pub line: usize,
    /// The record, in the words its own fields give it — an event id, or an artifact and an
    /// instant. Named so the reader can go to it, which is the whole point of failing at the exact
    /// entry rather than reporting that something somewhere is wrong.
    pub entry: String,
    /// What is wrong with it.
    pub reason: Reason,
    /// Records after it, which the chain therefore says nothing about.
    pub unverified: usize,
}

impl fmt::Display for Broken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the journal's hash chain breaks at line {} ({}): {}. {} later record(s) are \
             unverified — the log is append-only and nothing in this store rewrites it, so a \
             record that does not match its own seal was changed by something that is not a command",
            self.line, self.entry, self.reason, self.unverified
        )
    }
}

/// What one walk of the chain found.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Verified {
    /// Records sealed, checked, and linked to the record before them.
    pub covered: usize,
    /// Lines written before this store chained its journal. Not a defect: see the module doc.
    pub uncovered: usize,
    /// The first place the chain stops holding, if it does.
    pub broken: Option<Broken>,
}

impl Verified {
    /// Whether the journal is chained at all.
    ///
    /// False for every store written before this change, and for a store nothing has written to
    /// since. *Not covered* and *broken* are different answers and neither is the other.
    #[must_use]
    pub const fn is_chained(&self) -> bool {
        self.covered > 0 || self.broken.is_some()
    }
}

/// Walks the journal's chain, stopping at the first record it does not hold for.
///
/// **It stops rather than accumulating**, which is the one place this crate departs from invariant
/// 3's *validation accumulates*. The invariant is about *independent* defects, and records after a
/// break are not independent of it: one edited record makes every later link mismatch, so
/// accumulating would report ninety findings for one edit and bury the only one that names where
/// the record stops being trustworthy. The count of what is left unchecked is reported instead, so
/// nothing is silently dropped.
///
/// A journal that is not there, or cannot be read, is an empty [`Verified`] — the same answer
/// `journal::read` gives, for the same reason: a store with no history is a normal store.
#[must_use]
pub fn verify(root: &Path) -> Verified {
    let Ok(text) = fs::read_to_string(root.join(JOURNAL)) else {
        return Verified::default();
    };
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    let mut verified = Verified::default();
    // The digest of the previous *sealed* record, or `None` when the record before this one was a
    // legacy line or debris — in which case this one anchors over the prefix instead of linking.
    let mut previous: Option<String> = None;
    let mut chained = false;

    for (index, line) in lines.iter().enumerate() {
        let Ok(Value::Object(record)) = serde_json::from_str::<Value>(line) else {
            // Debris, not a record. Skipped exactly as `journal::read` skips it, and the next
            // sealed record re-anchors across it.
            previous = None;
            continue;
        };
        let Some(sealed) = record.get(ENTRY_HASH).and_then(Value::as_str) else {
            if chained {
                return broken(verified, &lines, index, &record, Reason::Unsealed);
            }
            verified.uncovered += 1;
            previous = None;
            continue;
        };
        let sealed = sealed.to_owned();
        chained = true;

        let recomputed = digest(&record);
        if recomputed != sealed {
            let reason = Reason::Tampered { sealed, recomputed };
            return broken(verified, &lines, index, &record, reason);
        }

        let named = record
            .get(PARENT_HASH)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        // A record after another sealed record **links** to it; one after a legacy line, debris, or
        // nothing at all **anchors** over everything before it. The two failures are different
        // findings because they send the reader to different places: one says an entry moved, the
        // other says the older lines this entry froze were edited.
        let expected = previous.clone().unwrap_or_else(|| anchor(&lines[..index]));
        if named.as_deref() != Some(expected.as_str()) {
            let reason = if previous.is_some() {
                Reason::Unlinked { expected, named }
            } else {
                Reason::PrefixEdited { expected, named }
            };
            return broken(verified, &lines, index, &record, reason);
        }

        verified.covered += 1;
        previous = Some(sealed);
    }

    verified
}

/// Appends records to the journal, each sealed to the one before it, under the lock.
///
/// The records are JSON objects — a [`DomainEvent`](entity_core::DomainEvent) the provider wrote,
/// or a [`journal::Entry`](crate::journal::Entry) — and this adds [`PARENT_HASH`] and
/// [`ENTRY_HASH`] to each. Neither key is declared by either shape and neither refuses unknown
/// fields, so a reader that has never heard of the chain parses a sealed record exactly as it
/// parsed an unsealed one, and two `DomainEvent`s compare equal across a seal. That is what lets
/// this land without touching the runtime's type or the recovery path that compares events.
///
/// # Errors
///
/// Whatever the filesystem said, and one error of its own: a journal whose lock is held by another
/// writer after a bounded number of tries. Both are returned rather than swallowed, for the reason
/// `journal::append` gives — a log that silently stops recording looks like a plan where nothing
/// happened.
pub fn append_sealed(root: &Path, records: &[Value]) -> std::io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    let path = root.join(JOURNAL);
    fs::create_dir_all(root)?;

    let _lock = Lock::acquire(root)?;

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let mut parent = head(&lines);

    let mut appended = String::new();
    for record in records {
        let Value::Object(record) = record else {
            return Err(std::io::Error::other(
                "a journal record is a JSON object; this crate does not append anything else",
            ));
        };
        let mut record = record.clone();
        record.insert(PARENT_HASH.to_owned(), Value::String(parent));
        let entry = digest(&record);
        record.insert(ENTRY_HASH.to_owned(), Value::String(entry.clone()));
        appended.push_str(
            &serde_json::to_string(&Value::Object(record))
                .map_err(|error| -> std::io::Error { std::io::Error::other(error) })?,
        );
        appended.push('\n');
        parent = entry;
    }

    // Append, never rewrite — the property JSONL is chosen for, and the property invariant 8 is.
    // `sync_all` before the lock is dropped, so the next writer reads a head that is on the disk
    // and not merely in a buffer this process might not live to flush.
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(appended.as_bytes())?;
    file.sync_all()
}

/// The digest the next record appended to these lines must name as its parent.
///
/// The last record's own digest when the journal ends in a sealed record; the [`anchor`] over
/// every line otherwise — which is how the chain begins on a legacy journal, and how it resumes
/// across a half-written line.
fn head(lines: &[&str]) -> String {
    lines
        .last()
        .and_then(|line| serde_json::from_str::<Value>(line).ok())
        .and_then(|value| {
            value
                .get(ENTRY_HASH)
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| anchor(lines))
}

/// One digest over a block of lines the chain does not otherwise cover.
///
/// Taken over the raw lines as they stand in the file, framed by the newline they cannot contain,
/// with the count in the preimage so a prefix of `n` lines and a prefix of `n + 1` cannot collide
/// by framing. An empty prefix has an anchor too: it is what the very first record of a journal
/// that was chained from its first line names as its parent.
#[must_use]
pub fn anchor(lines: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ANCHOR_TAG.as_bytes());
    hasher.update(b"\n");
    hasher.update(lines.len().to_string().as_bytes());
    hasher.update(b"\n");
    for line in lines {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    hex(&hasher.finalize())
}

/// One record's own digest: every key it carries except [`ENTRY_HASH`], in name order.
///
/// **Sorted keys, re-serialised, rather than the bytes of the line.** A digest over the raw line
/// would depend on the order `serde_json` happened to emit the object's keys in, which the
/// `preserve_order` feature can be turned on anywhere in a dependency graph and change for
/// everybody. A `BTreeMap` is one answer on every build, and a verifier that re-derives the same
/// bytes from a parsed record is a verifier that cannot be defeated by a whitespace change either.
///
/// [`PARENT_HASH`] **is** in the preimage. That is what makes this a chain rather than a row of
/// unrelated checksums: change which record a record follows and its own digest changes with it.
fn digest(record: &Map<String, Value>) -> String {
    let canonical: BTreeMap<&str, &Value> = record
        .iter()
        .filter(|(key, _)| key.as_str() != ENTRY_HASH)
        .map(|(key, value)| (key.as_str(), value))
        .collect();
    let mut hasher = Sha256::new();
    hasher.update(ENTRY_TAG.as_bytes());
    hasher.update(b"\n");
    hasher.update(
        serde_json::to_vec(&canonical)
            .expect("a map of parsed JSON values re-serialises")
            .as_slice(),
    );
    hex(&hasher.finalize())
}

/// The record in the words its own fields give it, so the finding names something a reader can go
/// and find.
fn describe(record: &Map<String, Value>) -> String {
    if let Some(id) = record
        .get("payload")
        .and_then(|payload| payload.get("event_id"))
        .and_then(Value::as_str)
    {
        return id.to_owned();
    }
    if let (Some(entity), Some(id)) = (
        record.get("entity").and_then(Value::as_str),
        record.get("id").and_then(Value::as_str),
    ) {
        let revision = record
            .get("revision")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        return format!("{entity}:{id}@{revision}");
    }
    if let Some(artifact) = record.get("artifact").and_then(Value::as_str) {
        return match record.get("at").and_then(Value::as_str) {
            Some(at) => format!("{artifact} at {at}"),
            None => artifact.to_owned(),
        };
    }
    "a record carrying no identity this crate recognises".to_owned()
}

/// Fills in the break and returns, which is what *fails closed at the exact entry* means here.
fn broken(
    mut verified: Verified,
    lines: &[&str],
    index: usize,
    record: &Map<String, Value>,
    reason: Reason,
) -> Verified {
    verified.broken = Some(Broken {
        line: index + 1,
        entry: describe(record),
        reason,
        unverified: lines.len() - index - 1,
    });
    verified
}

/// Lowercase hex, the spelling every other digest in this workspace is written in.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
        output
    })
}

/// The exclusive claim on the journal, held for the read of the head and the write that extends it.
struct Lock {
    path: PathBuf,
}

impl Lock {
    fn acquire(root: &Path) -> std::io::Result<Self> {
        let path = root.join(LOCK);
        for _ in 0..LOCK_ATTEMPTS {
            match fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)
            {
                Ok(mut file) => {
                    // The pid, so a lock left behind names the process that left it. Best effort:
                    // the lock is the `create_new`, and a body that failed to land must not make a
                    // held lock read as an unheld one.
                    let _ = writeln!(file, "{}", std::process::id());
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    std::thread::sleep(LOCK_WAIT);
                }
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            format!(
                "another writer holds {}. If no `aep` is running, that file is residue from a \
                 process that was killed mid-append; remove it and run the command again",
                path.display()
            ),
        ))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // Nothing to do about a failure here that is better than leaving the file: the error path
        // is a lock somebody removes by hand, and the message `acquire` prints says so.
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch tree this process alone owns — the reason the pid is in the name is written out
    /// on `store.rs`'s own `scratch`: `temp_dir()` is one directory for every session and every
    /// worktree on a machine, and the first thing that function does is delete it.
    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir()
            .join("aep-markdown-chain")
            .join(format!("{}-{}", std::process::id(), name));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).expect("the scratch tree is writable");
        root
    }

    fn record(id: &str, revision: u64) -> Value {
        serde_json::json!({
            "entity": "story",
            "version": 1,
            "id": id,
            "revision": revision,
            "type": "aep.entity.update/v1",
            "from_state": "draft",
            "to_state": "draft",
            "changed": {},
            "args": {},
            "payload": { "event_id": format!("story:{id}@{revision}") },
        })
    }

    fn lines(root: &Path) -> Vec<String> {
        std::fs::read_to_string(root.join(JOURNAL))
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }

    fn rewrite(root: &Path, lines: &[String]) {
        std::fs::write(root.join(JOURNAL), format!("{}\n", lines.join("\n"))).expect("writable");
    }

    #[test]
    fn a_journal_this_store_wrote_verifies_and_reports_every_record_as_covered() {
        let root = scratch("clean");
        for revision in 1..=4 {
            append_sealed(&root, &[record("one", revision)]).expect("appendable");
        }

        let verified = verify(&root);
        assert_eq!(verified.broken, None, "{verified:?}");
        assert_eq!(verified.covered, 4);
        assert_eq!(verified.uncovered, 0);
        assert!(verified.is_chained());
    }

    #[test]
    fn a_middle_record_that_was_edited_is_named_by_its_own_event_id() {
        let root = scratch("middle");
        for revision in 1..=5 {
            append_sealed(&root, &[record("one", revision)]).expect("appendable");
        }

        // The third record, edited in place the way an editor edits: the seal is left alone
        // because the editor has no idea it is a seal.
        let mut held = lines(&root);
        held[2] = held[2].replace("\"to_state\":\"draft\"", "\"to_state\":\"implemented\"");
        rewrite(&root, &held);

        let verified = verify(&root);
        let broken = verified.broken.expect("the chain does not hold");
        assert_eq!(
            broken.line, 3,
            "the exact record, not the first or the last"
        );
        assert_eq!(broken.entry, "story:one@3");
        assert!(
            matches!(broken.reason, Reason::Tampered { .. }),
            "{:?}",
            broken.reason
        );
        assert_eq!(broken.unverified, 2, "the two records after it");
        assert_eq!(verified.covered, 2, "the two before it did hold");
    }

    #[test]
    fn a_record_removed_from_the_middle_leaves_the_next_one_naming_a_parent_that_is_not_there() {
        let root = scratch("removed");
        for revision in 1..=4 {
            append_sealed(&root, &[record("one", revision)]).expect("appendable");
        }
        let mut held = lines(&root);
        held.remove(1);
        rewrite(&root, &held);

        let broken = verify(&root).broken.expect("the chain does not hold");
        assert_eq!(broken.line, 2);
        assert_eq!(broken.entry, "story:one@3");
        assert!(
            matches!(broken.reason, Reason::Unlinked { .. }),
            "{:?}",
            broken.reason
        );
    }

    #[test]
    fn two_records_swapped_break_the_chain_even_though_neither_was_edited() {
        let root = scratch("swapped");
        for revision in 1..=4 {
            append_sealed(&root, &[record("one", revision)]).expect("appendable");
        }
        let mut held = lines(&root);
        held.swap(1, 2);
        rewrite(&root, &held);

        let broken = verify(&root).broken.expect("the chain does not hold");
        assert_eq!(broken.line, 2);
        assert!(
            matches!(broken.reason, Reason::Unlinked { .. }),
            "order is part of what a log records: {:?}",
            broken.reason
        );
    }

    #[test]
    fn a_journal_with_no_seals_at_all_is_uncovered_and_is_not_reported_as_broken() {
        // Every store in this workspace on the day this landed. *Not covered* and *tampered* are
        // different answers, and reporting the first as the second would call six honest
        // repositories forged.
        let root = scratch("legacy");
        let legacy = [
            r#"{"at":"2026-08-01T09:00:00Z","actor":"operator","artifact":"story:legacy","kind":"story","revision":1,"change":{"change":"created","status":"draft"}}"#,
            r#"{"entity":"story","version":1,"id":"legacy","revision":2,"type":"aep.entity.update/v1","from_state":"draft","to_state":"proposed","changed":{},"args":{},"payload":{"event_id":"story:legacy@2"}}"#,
        ];
        std::fs::write(root.join(JOURNAL), format!("{}\n", legacy.join("\n"))).expect("writable");

        let verified = verify(&root);
        assert_eq!(verified.broken, None, "{verified:?}");
        assert_eq!(verified.uncovered, 2);
        assert_eq!(verified.covered, 0);
        assert!(!verified.is_chained(), "the chain has not started here");
    }

    #[test]
    fn the_first_sealed_record_freezes_the_legacy_lines_before_it() {
        let root = scratch("anchored");
        let legacy = r#"{"at":"2026-08-01T09:00:00Z","actor":"operator","artifact":"story:legacy","kind":"story","revision":1,"change":{"change":"created","status":"draft"}}"#;
        std::fs::write(root.join(JOURNAL), format!("{legacy}\n")).expect("writable");
        append_sealed(&root, &[record("one", 2)]).expect("appendable");

        let verified = verify(&root);
        assert_eq!(verified.broken, None, "{verified:?}");
        assert_eq!(verified.uncovered, 1);
        assert_eq!(verified.covered, 1);

        // Now edit the legacy line the sealed record anchored over. Nothing seals that line
        // itself; what seals it is being counted in the anchor the record after it names.
        let mut held = lines(&root);
        held[0] = held[0].replace("\"actor\":\"operator\"", "\"actor\":\"somebody-else\"");
        rewrite(&root, &held);

        let broken = verify(&root).broken.expect("the prefix is sealed too");
        assert_eq!(broken.line, 2, "the record that anchors it is what reports");
        assert!(
            matches!(broken.reason, Reason::PrefixEdited { .. }),
            "and it says the older lines changed, not that this record did: {:?}",
            broken.reason
        );
    }

    #[test]
    fn stripping_the_seals_off_the_tail_is_a_break_and_not_a_downgrade_to_legacy() {
        // The obvious way around a chain: delete the digests instead of recomputing them. If an
        // unsealed record after a sealed one were tolerated, this would un-chain a log silently.
        let root = scratch("stripped");
        for revision in 1..=3 {
            append_sealed(&root, &[record("one", revision)]).expect("appendable");
        }
        let mut held = lines(&root);
        let mut last: Map<String, Value> = serde_json::from_str(&held[2]).expect("a record");
        last.remove(ENTRY_HASH);
        last.remove(PARENT_HASH);
        held[2] = serde_json::to_string(&Value::Object(last)).expect("serialisable");
        rewrite(&root, &held);

        let broken = verify(&root).broken.expect("the chain does not hold");
        assert_eq!(broken.line, 3);
        assert_eq!(broken.reason, Reason::Unsealed);
    }

    #[test]
    fn a_half_written_line_is_debris_and_the_next_append_re_anchors_across_it() {
        // A killed process, not an attacker. `journal::read` skips and counts such a line; the
        // chain steps over it, and the record after it anchors over everything before it instead
        // of linking to a line that is not a record.
        let root = scratch("debris");
        append_sealed(&root, &[record("one", 1)]).expect("appendable");
        let mut text = std::fs::read_to_string(root.join(JOURNAL)).expect("readable");
        text.push_str("half a line, written by a process that died\n");
        std::fs::write(root.join(JOURNAL), text).expect("writable");
        append_sealed(&root, &[record("one", 2)]).expect("appendable");

        let verified = verify(&root);
        assert_eq!(
            verified.broken, None,
            "a crash is not an attack: {verified:?}"
        );
        assert_eq!(verified.covered, 2);
    }

    #[test]
    fn an_appended_record_that_forges_a_revision_cannot_be_sealed_by_copying_a_neighbour() {
        // The gap register's own case, one layer down. Appending a `revision: 99` line by hand is
        // the half that made a forged document validate; copying a real record's seal onto it does
        // not help, because the seal is over the record's own bytes.
        let root = scratch("forged-line");
        append_sealed(&root, &[record("forge", 1)]).expect("appendable");
        let held = lines(&root);
        let mut forged: Map<String, Value> = serde_json::from_str(&held[0]).expect("a record");
        forged.insert("revision".to_owned(), Value::from(99_u64));
        let mut text = std::fs::read_to_string(root.join(JOURNAL)).expect("readable");
        text.push_str(&serde_json::to_string(&Value::Object(forged)).expect("serialisable"));
        text.push('\n');
        std::fs::write(root.join(JOURNAL), text).expect("writable");

        let broken = verify(&root).broken.expect("the chain does not hold");
        assert_eq!(broken.line, 2);
        assert!(
            matches!(broken.reason, Reason::Tampered { .. }),
            "{:?}",
            broken.reason
        );
    }

    #[test]
    fn a_sealed_record_still_parses_as_the_runtime_event_it_was() {
        // The compatibility claim `append_sealed` rests on, asserted rather than assumed: neither
        // shape refuses unknown fields, so the two keys are invisible to every existing reader and
        // two events compare equal across a seal.
        let root = scratch("parses");
        let original: entity_core::DomainEvent =
            serde_json::from_value(record("one", 1)).expect("a well-formed event");
        append_sealed(&root, &[record("one", 1)]).expect("appendable");

        let held = lines(&root);
        assert!(held[0].contains(ENTRY_HASH), "it really was sealed");
        let read: entity_core::DomainEvent =
            serde_json::from_str(&held[0]).expect("a sealed line is still an event");
        assert_eq!(read, original, "the seal is not part of the event");
    }

    #[test]
    fn the_lock_is_released_when_the_append_returns() {
        let root = scratch("lock");
        append_sealed(&root, &[record("one", 1)]).expect("appendable");
        assert!(
            !root.join(LOCK).exists(),
            "a lock nobody releases wedges the store on the next write"
        );
    }

    #[test]
    fn an_append_refuses_while_another_writer_holds_the_lock() {
        let root = scratch("held");
        let _held = Lock::acquire(&root).expect("the lock is free");
        let refused = append_sealed(&root, &[record("one", 1)]).expect_err("the lock is held");
        assert_eq!(refused.kind(), std::io::ErrorKind::WouldBlock);
        assert!(
            refused.to_string().contains("journal.lock"),
            "the refusal names the file to remove: {refused}"
        );
    }
}
