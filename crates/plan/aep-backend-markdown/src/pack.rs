//! `aep.pack/1`: closed artifacts and their evidence, held in one text file, and the **overlay**
//! that reads them as if they were still loose.
//!
//! ```text
//! aep.pack/1
//! --- evidence/story/x/20261110T000000Z-000-3f2a9c1d4e5b.json 412 sha256:<64 hex>
//! <exactly 412 bytes: the original file>
//! --- planning/story/x.md 1873 sha256:<64 hex>
//! <exactly 1873 bytes: the original file>
//! ```
//!
//! One header line, then per record a `--- <path> <byte length> sha256:<hex>` line, the original
//! file's bytes verbatim and one `\n`. The path is relative to `.engineering` and follows the
//! store's own grammar: `planning/<kind>/<name>.md` or `evidence/<kind>/<name>/<file>.json`.
//! Records are sorted by path bytes and a path appears once. A pack is a function of the set of
//! `(path, bytes)` it holds, and its **name** is a function of its bytes: the first sixteen hex
//! digits of their SHA-256, so the file is `.engineering/packs/<name>.pack`.
//!
//! # The two halves
//!
//! * [`render`], [`Pack::parse`], [`name`] and [`check_path`] are the format. They touch no disk,
//!   and every refusal carries a stable [`PackCode`].
//! * [`Overlay`] is the read abstraction: the loose files under `planning/` and `evidence/` plus
//!   every record of every pack under `packs/`, listed and read as one tree. Every reader of a
//!   Git-native store goes through it, so a verb answers identically whether a record is loose or
//!   packed. [`Overlay::holders`] is the lookup the `pack`, `unpack` and `validate` verbs use: for a
//!   path, every place that holds it.
//!
//! A pack is never edited in place, so a parsed pack is cached for the process, keyed by its file,
//! its length and its modification time; a verb that reads every artifact does not parse every pack
//! once per artifact.
//!
//! `parse` checks the structure and not the digests, but no reader of the overlay serves a record
//! whose digests were not checked: [`PackFile::verified`] is a pack that parses, whose every
//! record matches its header ([`PackRecord::digest_matches`]) and whose file name is its content
//! ([`Pack::name`] against the file's stem). A pack that fails either is refused as one that does
//! not parse is, and the check is cached with the parse. [`Overlay::packs`] still lists such a
//! pack with its parse, so `validate` reports it under its own codes.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use sha2::{Digest as _, Sha256};

/// The first line of a pack.
pub const FORMAT: &str = "aep.pack/1";

/// The directory, beside `planning` and `evidence`, packs are kept in.
pub const PACKS_DIRECTORY: &str = "packs";

/// The extension of a pack file.
pub const EXTENSION: &str = "pack";

/// How many hex digits of the content digest name a pack.
pub const NAME_LENGTH: usize = 16;

/// Why a pack was refused. Stable: a caller matches on it, and [`Self::as_str`] is what a report
/// prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackCode {
    /// The first line is not `aep.pack/1`.
    HeaderInvalid,
    /// A record line is not `--- <path> <length> sha256:<hex>`.
    RecordLineMalformed,
    /// The digest on a record line is not `sha256:` and 64 lower-case hex digits.
    DigestMalformed,
    /// The file ends inside a record line, or before the `\n` that closes a record.
    RecordTruncated,
    /// A record declares more bytes than the file has left.
    LengthPastEnd,
    /// A record's bytes are not followed by `\n`.
    RecordUnterminated,
    /// A path sorts before the path above it.
    PathUnsorted,
    /// A path appears twice.
    PathRepeated,
    /// A path is outside the grammar.
    PathInvalid,
    /// The pack file could not be read.
    Unreadable,
    /// A record's bytes are not the ones its header digest is of.
    RecordDigestDiffers,
    /// The file is not named for the digest of its content.
    NameNotDigest,
}

impl PackCode {
    /// The code as reports spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HeaderInvalid => "pack_header_invalid",
            Self::RecordLineMalformed => "pack_record_line_malformed",
            Self::DigestMalformed => "pack_digest_malformed",
            Self::RecordTruncated => "pack_record_truncated",
            Self::LengthPastEnd => "pack_length_past_end",
            Self::RecordUnterminated => "pack_record_unterminated",
            Self::PathUnsorted => "pack_path_unsorted",
            Self::PathRepeated => "pack_path_repeated",
            Self::PathInvalid => "pack_path_invalid",
            Self::Unreadable => "pack_unreadable",
            Self::RecordDigestDiffers => "pack_record_digest_differs",
            Self::NameNotDigest => "pack_name_not_digest",
        }
    }
}

impl fmt::Display for PackCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A refusal of a pack, or of a path or record that would not fit one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackError {
    code: PackCode,
    detail: String,
}

impl PackError {
    fn new(code: PackCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    /// The stable code.
    #[must_use]
    pub fn code(&self) -> PackCode {
        self.code
    }

    /// What is wrong, for a person.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.detail)
    }
}

impl std::error::Error for PackError {}

/// Lower-case hexadecimal of `bytes`.
fn hex(bytes: &[u8]) -> String {
    use fmt::Write as _;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
        output
    })
}

/// The SHA-256 of `bytes`, as 64 lower-case hex digits.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// The name of the pack whose bytes are `bytes`: the first [`NAME_LENGTH`] hex digits of their
/// SHA-256.
#[must_use]
pub fn name(bytes: &[u8]) -> String {
    sha256_hex(bytes)[..NAME_LENGTH].to_owned()
}

/// The file a pack named `name` is kept in, relative to the packs directory.
#[must_use]
pub fn file_name(name: &str) -> String {
    format!("{name}.{EXTENSION}")
}

/// Checks `path` against the store's grammar, relative to `.engineering`.
///
/// `planning/<kind>/<name>.md` or `evidence/<kind>/<name>/<file>.json`, and nothing else: no `..`,
/// no absolute path, no backslash, no empty or `.` segment, no NUL or other control character.
/// `unpack` writes only paths that pass, inside `.engineering`.
///
/// # Errors
///
/// [`PackCode::PathInvalid`], with the reason in the detail.
pub fn check_path(path: &str) -> Result<(), PackError> {
    let refuse = |reason: &str| {
        Err(PackError::new(
            PackCode::PathInvalid,
            format!("`{}` {reason}", path.escape_debug()),
        ))
    };
    if path.is_empty() {
        return refuse("is empty");
    }
    if path.chars().any(char::is_control) {
        return refuse("holds a control character");
    }
    if path.contains('\\') {
        return refuse("holds a backslash");
    }
    if path.starts_with('/') {
        return refuse("is absolute");
    }
    let segments: Vec<&str> = path.split('/').collect();
    for segment in &segments {
        match *segment {
            "" => return refuse("has an empty segment"),
            "." => return refuse("has a `.` segment"),
            ".." => return refuse("has a `..` segment"),
            // Readers skip dot-names (a write's temporary file is one), so a record the parser
            // admitted under one would be held and never listed.
            dotted if dotted.starts_with('.') => return refuse("has a segment starting with `.`"),
            _ => {}
        }
    }
    let file_stem_ok = |file: &str, extension: &str| {
        file.strip_suffix(extension)
            .is_some_and(|stem| !stem.is_empty())
    };
    match segments.as_slice() {
        ["planning", _kind, file] if file_stem_ok(file, ".md") => Ok(()),
        ["evidence", _kind, _name, file] if file_stem_ok(file, ".json") => Ok(()),
        _ => refuse(
            "is neither `planning/<kind>/<name>.md` nor `evidence/<kind>/<name>/<file>.json`",
        ),
    }
}

/// One record of a pack: a path and the original file's bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRecord {
    path: String,
    bytes: Vec<u8>,
    sha256: String,
}

impl PackRecord {
    /// The path, relative to `.engineering`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The original file's bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The digest the record line declares: 64 lower-case hex digits.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Whether the bytes are the ones the declared digest is of.
    #[must_use]
    pub fn digest_matches(&self) -> bool {
        sha256_hex(&self.bytes) == self.sha256
    }
}

/// A parsed pack: its records, sorted by path, and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    name: String,
    records: Vec<PackRecord>,
}

impl Pack {
    /// Parses `bytes` as `aep.pack/1`.
    ///
    /// # Errors
    ///
    /// The first [`PackCode`] the bytes earn: a wrong header, a malformed record line or digest, a
    /// truncated record, a length past the end, bytes not closed by `\n`, an unsorted or repeated
    /// path, or a path outside the grammar.
    pub fn parse(bytes: &[u8]) -> Result<Self, PackError> {
        let header = format!("{FORMAT}\n");
        if !bytes.starts_with(header.as_bytes()) {
            return Err(PackError::new(
                PackCode::HeaderInvalid,
                format!("the first line is not `{FORMAT}`"),
            ));
        }
        let mut position = header.len();
        let mut records: Vec<PackRecord> = Vec::new();
        while position < bytes.len() {
            let Some(line_length) = bytes[position..].iter().position(|byte| *byte == b'\n') else {
                return Err(PackError::new(
                    PackCode::RecordTruncated,
                    format!("the file ends inside the record line at byte {position}"),
                ));
            };
            let line = &bytes[position..position + line_length];
            let (path, length, sha256) = record_line(line, position)?;
            check_path(path)?;
            if let Some(previous) = records.last() {
                if previous.path == path {
                    return Err(PackError::new(
                        PackCode::PathRepeated,
                        format!("`{path}` appears twice"),
                    ));
                }
                if previous.path.as_str() > path {
                    return Err(PackError::new(
                        PackCode::PathUnsorted,
                        format!("`{path}` sorts before `{}` above it", previous.path),
                    ));
                }
            }
            let start = position + line_length + 1;
            if length > bytes.len() - start {
                return Err(PackError::new(
                    PackCode::LengthPastEnd,
                    format!(
                        "`{path}` declares {length} bytes and the file has {} left",
                        bytes.len() - start
                    ),
                ));
            }
            let end = start + length;
            match bytes.get(end) {
                None => {
                    return Err(PackError::new(
                        PackCode::RecordTruncated,
                        format!("the file ends before the line end that closes `{path}`"),
                    ))
                }
                Some(b'\n') => {}
                Some(_) => {
                    return Err(PackError::new(
                        PackCode::RecordUnterminated,
                        format!("the {length} bytes of `{path}` are not followed by a line end"),
                    ))
                }
            }
            records.push(PackRecord {
                path: path.to_owned(),
                bytes: bytes[start..end].to_vec(),
                sha256: sha256.to_owned(),
            });
            position = end + 1;
        }
        Ok(Self {
            name: name(bytes),
            records,
        })
    }

    /// The name of the pack: the first sixteen hex digits of the SHA-256 of the bytes it was parsed
    /// from.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The records, sorted by path.
    #[must_use]
    pub fn records(&self) -> &[PackRecord] {
        &self.records
    }

    /// The record at `path`, if the pack holds one.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&PackRecord> {
        self.records
            .binary_search_by(|record| record.path.as_str().cmp(path))
            .ok()
            .map(|index| &self.records[index])
    }
}

/// Splits `--- <path> <length> sha256:<hex>`.
fn record_line(line: &[u8], position: usize) -> Result<(&str, usize, &str), PackError> {
    let malformed = |why: &str| {
        PackError::new(
            PackCode::RecordLineMalformed,
            format!("the record line at byte {position} {why}"),
        )
    };
    let text = std::str::from_utf8(line).map_err(|_| malformed("is not UTF-8"))?;
    let rest = text
        .strip_prefix("--- ")
        .ok_or_else(|| malformed("does not start with `--- `"))?;
    let mut fields = rest.rsplitn(3, ' ');
    let (Some(digest), Some(length), Some(path)) = (fields.next(), fields.next(), fields.next())
    else {
        return Err(malformed("is not `--- <path> <length> sha256:<hex>`"));
    };
    if length.is_empty() || !length.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(malformed("has a length that is not a number"));
    }
    // One spelling per length, so a pack that parses is the bytes `render` writes for its records.
    if length.len() > 1 && length.starts_with('0') {
        return Err(malformed("has a length with a leading zero"));
    }
    let length: usize = length
        .parse()
        .map_err(|_| malformed("has a length that is not a number"))?;
    let hex = digest.strip_prefix("sha256:").filter(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    });
    let Some(hex) = hex else {
        return Err(PackError::new(
            PackCode::DigestMalformed,
            format!("the digest on the record line at byte {position} is not `sha256:` and 64 lower-case hex digits"),
        ));
    };
    Ok((path, length, hex))
}

/// Renders the pack that holds `records`, each a path relative to `.engineering` and the original
/// file's bytes.
///
/// Sorted by path, so the bytes are a function of the *set*: the order the records are given in
/// does not show.
///
/// # Errors
///
/// [`PackCode::PathInvalid`] for a path outside the grammar and [`PackCode::PathRepeated`] for one
/// given twice.
pub fn render<P: AsRef<str>, B: AsRef<[u8]>>(records: &[(P, B)]) -> Result<Vec<u8>, PackError> {
    let mut sorted: Vec<(&str, &[u8])> = Vec::with_capacity(records.len());
    for (path, bytes) in records {
        check_path(path.as_ref())?;
        sorted.push((path.as_ref(), bytes.as_ref()));
    }
    sorted.sort_by(|left, right| left.0.cmp(right.0));
    if let Some(pair) = sorted.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(PackError::new(
            PackCode::PathRepeated,
            format!("`{}` is given twice", pair[0].0),
        ));
    }
    let mut out = format!("{FORMAT}\n").into_bytes();
    for (path, bytes) in sorted {
        out.extend_from_slice(
            format!("--- {path} {} sha256:{}\n", bytes.len(), sha256_hex(bytes)).as_bytes(),
        );
        out.extend_from_slice(bytes);
        out.push(b'\n');
    }
    Ok(out)
}

/// The bytes of `pack` without the records at `dropped`, or `None` when nothing is left.
///
/// The records that stay keep their bytes, so the result is what [`render`] makes of the remaining
/// set: its name is the name the rewritten pack is written under. A pack is never edited in place.
///
/// # Errors
///
/// As [`render`]; a pack that parsed holds paths that pass, so a refusal here is a bug.
pub fn render_without(
    pack: &Pack,
    dropped: &std::collections::BTreeSet<String>,
) -> Result<Option<Vec<u8>>, PackError> {
    let kept: Vec<(&str, &[u8])> = pack
        .records()
        .iter()
        .filter(|record| !dropped.contains(record.path()))
        .map(|record| (record.path(), record.bytes()))
        .collect();
    if kept.is_empty() {
        return Ok(None);
    }
    render(&kept).map(Some)
}

/// One pack file in the packs directory, parsed or refused.
#[derive(Debug, Clone)]
pub struct PackFile {
    name: String,
    file: PathBuf,
    pack: Result<Arc<Pack>, PackError>,
    integrity: Option<PackError>,
}

impl PackFile {
    /// The file's stem: the name the pack claims. [`Pack::name`] is the one its bytes earn.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The pack file.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// The parsed pack, or why the file is not one.
    #[must_use]
    pub fn pack(&self) -> &Result<Arc<Pack>, PackError> {
        &self.pack
    }

    /// The parsed pack when it can be read from: it parsed, every record matches its header
    /// digest and its file name is the digest of its content. Otherwise why not.
    ///
    /// # Errors
    ///
    /// The parse refusal, or the first record whose bytes fail their digest, or the name that is
    /// not the content's.
    pub fn verified(&self) -> Result<&Arc<Pack>, PackError> {
        let pack = self.pack.as_ref().map_err(Clone::clone)?;
        match &self.integrity {
            Some(error) => Err(error.clone()),
            None => Ok(pack),
        }
    }
}

/// The evidence directory of a Git-native store whose documents are in `planning`, and so, beside
/// it, where its packs are: the one derivation every reader of the store uses.
///
/// Inside a project directory (`project_directory`, `.engineering` by default) it is the sibling
/// `evidence`. Anywhere else it is `evidence` inside `planning` itself, so two stores that share a
/// parent never share, and count, each other's records.
#[must_use]
pub fn evidence_beside(planning: &Path, project_directory: &str) -> PathBuf {
    let evidence = aep_domain::project::GIT_EVIDENCE_DIRECTORY;
    let absolute = std::path::absolute(planning).unwrap_or_else(|_| planning.to_path_buf());
    match absolute.parent() {
        Some(parent)
            if parent
                .file_name()
                .is_some_and(|name| name == project_directory) =>
        {
            planning
                .parent()
                .map_or_else(|| parent.join(evidence), |sibling| sibling.join(evidence))
        }
        _ => planning.join(evidence),
    }
}

/// Where a copy of a store file is kept.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    /// A loose file under `planning/` or `evidence/`.
    Loose,
    /// A record of the pack with this name.
    Packed(String),
}

/// One copy of a store file: where it is held and the bytes held there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    place: Place,
    bytes: Vec<u8>,
}

impl Holder {
    /// A holder of `bytes` at `place`.
    #[must_use]
    pub fn new(place: Place, bytes: Vec<u8>) -> Self {
        Self { place, bytes }
    }

    /// Where the copy is.
    #[must_use]
    pub fn place(&self) -> &Place {
        &self.place
    }

    /// The bytes it holds.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

type Cache = BTreeMap<PathBuf, (u64, Option<u128>, Arc<Pack>, Option<PackError>)>;

/// Parsed packs of this process, by file. A pack is never edited in place, so length and
/// modification time say whether the entry is still the file.
static PARSED: Mutex<Cache> = Mutex::new(BTreeMap::new());

/// The first way a parsed pack at `file` is not what its header and its name say.
fn integrity(file: &Path, pack: &Pack) -> Option<PackError> {
    if let Some(record) = pack
        .records()
        .iter()
        .find(|record| !record.digest_matches())
    {
        return Some(PackError::new(
            PackCode::RecordDigestDiffers,
            format!(
                "`{}` is not the bytes its header digest is of",
                record.path().escape_debug()
            ),
        ));
    }
    let stem = file.file_stem().and_then(|stem| stem.to_str());
    (stem != Some(pack.name())).then(|| {
        PackError::new(
            PackCode::NameNotDigest,
            format!(
                "the file is named `{}` and its content is `{}`",
                stem.unwrap_or_default(),
                pack.name()
            ),
        )
    })
}

fn load(file: &Path) -> Result<(Arc<Pack>, Option<PackError>), PackError> {
    let unreadable = |error: &io::Error| {
        PackError::new(
            PackCode::Unreadable,
            format!("{} cannot be read: {error}", file.display()),
        )
    };
    let metadata = std::fs::metadata(file).map_err(|error| unreadable(&error))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_nanos());
    let length = metadata.len();
    if let Some((held_length, held_modified, pack, flaw)) = PARSED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(file)
    {
        if *held_length == length && *held_modified == modified {
            return Ok((Arc::clone(pack), flaw.clone()));
        }
    }
    let bytes = std::fs::read(file).map_err(|error| unreadable(&error))?;
    let pack = Arc::new(Pack::parse(&bytes)?);
    let flaw = integrity(file, &pack);
    PARSED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(
            file.to_path_buf(),
            (length, modified, Arc::clone(&pack), flaw.clone()),
        );
    Ok((pack, flaw))
}

/// The loose files of a store and every record of every pack in it, read as one tree.
///
/// Paths are the ones a pack record carries: relative to `.engineering`, `/`-separated, under
/// `planning/` or `evidence/`. Holding an overlay costs nothing and touches no disk; each call reads
/// what is there at that moment.
///
/// **Which copy answers.** A path held in two places (loose and in a pack, or in two packs) with
/// identical bytes is read as that one copy. With different bytes it is not read at all:
/// [`Self::read`] fails with an [`io::ErrorKind::InvalidData`] error naming both places, as
/// `validate` and `unpack` refuse it. [`Self::holders`] is what shows every copy.
///
/// **Errors.** A pack that does not parse, a record that fails its digest and a file name that is
/// not its content's digest are not skipped: the read that reaches the pack fails with an
/// [`io::ErrorKind::InvalidData`] error naming the pack file and its [`PackCode`]. [`Self::packs`]
/// is the call that reports every pack, parsed or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlay {
    planning: PathBuf,
    evidence: PathBuf,
    packs: PathBuf,
}

impl Overlay {
    /// The overlay over loose files in `planning` and `evidence`, and the packs in the `packs`
    /// directory beside `evidence`.
    pub fn new(planning: impl Into<PathBuf>, evidence: impl Into<PathBuf>) -> Self {
        let evidence = evidence.into();
        let packs = evidence.parent().map_or_else(
            || PathBuf::from(PACKS_DIRECTORY),
            |parent| parent.join(PACKS_DIRECTORY),
        );
        Self {
            planning: planning.into(),
            evidence,
            packs,
        }
    }

    /// The overlay for a caller that reads evidence and packs only: the planning directory is
    /// the `planning` beside `evidence`.
    pub fn for_evidence(evidence: impl Into<PathBuf>) -> Self {
        let evidence = evidence.into();
        let planning = evidence.with_file_name("planning");
        Self::new(planning, evidence)
    }

    /// The overlay for a store known only by its planning directory: the evidence directory is
    /// [`evidence_beside`], with the default project directory.
    ///
    /// A caller that knows the evidence directory passes it to [`Self::new`] instead.
    pub fn beside_planning(planning: impl Into<PathBuf>) -> Self {
        let planning = planning.into();
        let evidence = evidence_beside(&planning, aep_domain::project::PROJECT_DIRECTORY);
        Self::new(planning, evidence)
    }

    /// The directory the packs are in.
    #[must_use]
    pub fn packs_directory(&self) -> &Path {
        &self.packs
    }

    /// Where a loose copy of the store file `path` is (or would be) on disk.
    ///
    /// `planning/…` is under the planning directory and `evidence/…` under the evidence directory,
    /// whatever those directories are called.
    #[must_use]
    pub fn loose_path(&self, path: &str) -> PathBuf {
        let (root, rest) = match path
            .split_once('/')
            .or_else(|| matches!(path, "planning" | "evidence").then_some((path, "")))
        {
            Some(("planning", rest)) => (&self.planning, rest),
            Some(("evidence", rest)) => (&self.evidence, rest),
            _ => (&self.planning, path),
        };
        let mut found = root.clone();
        for segment in rest.split('/').filter(|segment| !segment.is_empty()) {
            found.push(segment);
        }
        found
    }

    /// Every pack file, parsed or refused, in name order.
    ///
    /// A packs directory that is not there is no packs. One that is there and cannot be listed, a
    /// `packs` that is not a directory, an entry that cannot be read and a pack file whose name is
    /// not UTF-8 are each a refused pack, named by its path: what they hide is records, and a
    /// reader must not take them for records there are none of.
    #[must_use]
    pub fn packs(&self) -> Vec<PackFile> {
        let refused = |file: PathBuf, detail: String| PackFile {
            name: file.to_string_lossy().into_owned(),
            pack: Err(PackError::new(PackCode::Unreadable, detail)),
            integrity: None,
            file,
        };
        let entries = match std::fs::read_dir(&self.packs) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Vec::new(),
            Err(error) => {
                let detail = format!("{} cannot be listed: {error}", self.packs.display());
                return vec![refused(self.packs.clone(), detail)];
            }
        };
        let mut found: Vec<PackFile> = Vec::new();
        for entry in entries {
            let file = match entry {
                Ok(entry) => entry.path(),
                Err(error) => {
                    let detail = format!("{} cannot be listed: {error}", self.packs.display());
                    found.push(refused(self.packs.clone(), detail));
                    continue;
                }
            };
            if file
                .extension()
                .is_none_or(|extension| extension != EXTENSION)
                || file
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with('.'))
                || !file.is_file()
            {
                continue;
            }
            let Some(name) = file.file_stem().and_then(|stem| stem.to_str()) else {
                let detail = format!("{} has a name that is not UTF-8", file.display());
                found.push(refused(file, detail));
                continue;
            };
            let name = name.to_owned();
            let (pack, integrity) = match load(&file) {
                Ok((pack, integrity)) => (Ok(pack), integrity),
                Err(error) => (Err(error), None),
            };
            found.push(PackFile {
                name,
                file,
                pack,
                integrity,
            });
        }
        found.sort_by(|left, right| left.name.cmp(&right.name));
        found
    }

    /// The verified packs, in name order, or the first refusal as an error naming the file.
    fn parsed(&self) -> io::Result<Vec<(String, Arc<Pack>)>> {
        let mut parsed = Vec::new();
        for file in self.packs() {
            match file.verified() {
                Ok(pack) => parsed.push((file.name.clone(), Arc::clone(pack))),
                Err(error) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("pack {}: {error}", file.file.display()),
                    ))
                }
            }
        }
        Ok(parsed)
    }

    /// Every place that holds the store file `path`, with the bytes each holds: the loose file
    /// first, then each pack in name order.
    ///
    /// # Errors
    ///
    /// A loose file that exists and cannot be read, and a pack that does not parse.
    pub fn holders(&self, path: &str) -> io::Result<Vec<Holder>> {
        let mut holders = Vec::new();
        match std::fs::read(self.loose_path(path)) {
            Ok(bytes) => holders.push(Holder::new(Place::Loose, bytes)),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) => {}
            // A directory where a file belongs holds no file.
            Err(_) if self.loose_path(path).is_dir() => {}
            Err(error) => return Err(error),
        }
        for (name, pack) in self.parsed()? {
            if let Some(record) = pack.get(path) {
                holders.push(Holder::new(Place::Packed(name), record.bytes.clone()));
            }
        }
        Ok(holders)
    }

    /// The bytes that answer a read of `path`, or `None` when nothing holds it.
    ///
    /// # Errors
    ///
    /// As [`Self::holders`], and a path held in two places with different bytes: the error names
    /// both places.
    pub fn read(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        let holders = self.holders(path)?;
        let Some(first) = holders.first() else {
            return Ok(None);
        };
        if let Some(other) = holders.iter().find(|holder| holder.bytes != first.bytes) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "`{path}` is held with different bytes by {} and by {}; \
                     a copy to keep has to be chosen before it can be read",
                    self.describe(path, &first.place),
                    self.describe(path, &other.place)
                ),
            ));
        }
        Ok(holders.into_iter().next().map(|holder| holder.bytes))
    }

    /// Where a copy is, as an error spells it: the loose file or the pack file.
    fn describe(&self, path: &str, place: &Place) -> String {
        match place {
            Place::Loose => self.loose_path(path).display().to_string(),
            Place::Packed(name) => self.packs.join(file_name(name)).display().to_string(),
        }
    }

    /// The names of the packs that hold `path`, in name order.
    ///
    /// # Errors
    ///
    /// A pack that does not parse.
    pub fn packed_in(&self, path: &str) -> io::Result<Vec<String>> {
        Ok(self
            .parsed()?
            .into_iter()
            .filter(|(_, pack)| pack.get(path).is_some())
            .map(|(name, _)| name)
            .collect())
    }

    /// Whether anything holds `path`.
    ///
    /// # Errors
    ///
    /// As [`Self::holders`].
    pub fn exists(&self, path: &str) -> io::Result<bool> {
        Ok(!self.holders(path)?.is_empty())
    }

    /// The entries directly under `directory` (`planning`, `evidence/story`, …): names of loose
    /// entries and of the next segment of packed paths, each with whether it is a directory.
    /// Dot-names are skipped; sorted; each name once.
    fn children(&self, directory: &str) -> io::Result<BTreeMap<String, bool>> {
        let mut found: BTreeMap<String, bool> = BTreeMap::new();
        match std::fs::read_dir(self.loose_path(directory)) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry?;
                    let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                        // Skipping it would list the directory without a file that is there.
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("{} has a name that is not UTF-8", entry.path().display()),
                        ));
                    };
                    if name.starts_with('.') {
                        continue;
                    }
                    let is_directory = entry.path().is_dir();
                    *found.entry(name).or_insert(false) |= is_directory;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) => {}
            Err(error) => return Err(error),
        }
        let prefix = format!("{directory}/");
        for (_, pack) in self.parsed()? {
            for record in pack.records() {
                let Some(rest) = record.path.strip_prefix(&prefix) else {
                    continue;
                };
                let (name, is_directory) = match rest.split_once('/') {
                    Some((name, _)) => (name, true),
                    None => (rest, false),
                };
                if name.starts_with('.') {
                    continue;
                }
                *found.entry(name.to_owned()).or_insert(false) |= is_directory;
            }
        }
        Ok(found)
    }

    /// Everything directly under `directory`, sorted, each with whether it is a directory.
    ///
    /// # Errors
    ///
    /// As [`Self::directories`].
    pub fn list(&self, directory: &str) -> io::Result<Vec<(String, bool)>> {
        Ok(self.children(directory)?.into_iter().collect())
    }

    /// The directories directly under `directory`, sorted.
    ///
    /// # Errors
    ///
    /// A directory that exists and cannot be listed, and a pack that does not parse.
    pub fn directories(&self, directory: &str) -> io::Result<Vec<String>> {
        Ok(self
            .children(directory)?
            .into_iter()
            .filter_map(|(name, is_directory)| is_directory.then_some(name))
            .collect())
    }

    /// The files directly under `directory` whose extension is `extension` (without the dot),
    /// sorted.
    ///
    /// # Errors
    ///
    /// As [`Self::directories`].
    pub fn files(&self, directory: &str, extension: &str) -> io::Result<Vec<String>> {
        let suffix = format!(".{extension}");
        Ok(self
            .children(directory)?
            .into_iter()
            .filter(|(name, is_directory)| !is_directory && name.ends_with(&suffix))
            .map(|(name, _)| name)
            .collect())
    }
}
