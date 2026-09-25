//! The fenced `findings` block a `review-result` body may carry, and the ledger over two of them.
//!
//! # Why a block in the body and not a frontmatter key
//!
//! A `review-result` is immutable: its body arrives with the record at `new` and nothing edits it
//! afterwards (`artifacts/lifecycles/review-result.yaml`). Its findings are part of what the review
//! *said*, so they belong in the bytes that are frozen with it rather than in a field a later write
//! could grow. A fenced block also stays readable as prose — the same document is a report somebody
//! reads and a record something computes over, which is the whole point of the review being an
//! artifact rather than a `docs/reviews/` file.
//!
//! ````text
//! ```findings
//! - file: crates/govern/aep-domain/src/artifact.rs
//!   line: 1462
//!   category: correctness
//!   severity: blocker
//!   verdict: CONFIRMED
//!   origin: introduced
//!   message: The loop never advances its index.
//! ```
//! ````
//!
//! # What is required, and what is not
//!
//! `file`, `category`, `severity` and `message` are required; `line`, `verdict` and `origin` are
//! not. That split is not a preference about completeness — it is what the signature needs against
//! what a given reviewer can honestly produce. The signature is `(file, category, normalised
//! message)`, so those three cannot be absent, and `severity` is what makes a finding actionable at
//! all. A finding about a whole file has no line; a critic returning `approve | needs-revision` at
//! the review level has no per-finding verdict; and `origin` has an explicit value for *not
//! decided*, which is what an unwritten one means. Writing `origin: undecided` and leaving `origin`
//! out are therefore the same claim, and this reads them as one.
//!
//! # Line drift
//!
//! Two findings are the same finding when their signatures match and their lines are within
//! [`LINE_TOLERANCE`]. A finding that moved because somebody inserted an import above it is not a
//! new finding, and a ledger that said it was would report a loop as diverging on every commit.
//! Where either side wrote no line, the line is not compared: *unknown* is not *far away*.

use std::fmt;
use std::fmt::Write as _;

/// The info string that marks the block, as it is written after the opening fence.
pub const FENCE_INFO: &str = "findings";

/// How far a finding may move and still be the same finding, in lines.
///
/// Three, as `finding_signature.py` uses at `dev-team-v13.0.0`, and the number is a judgement
/// rather than a discovery: it is wide enough to survive an added import or a reformatted
/// signature and narrow enough that two findings in one function do not merge.
pub const LINE_TOLERANCE: u32 = 3;

/// How much a finding is worth reacting to.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The work cannot ship with it.
    Blocker,
    /// It should be dealt with, and does not stop the work.
    Warning,
    /// Worth writing down, and nothing is owed.
    Note,
}

impl Severity {
    /// Every severity, in the order a report reads them.
    pub const ALL: &'static [Self] = &[Self::Blocker, Self::Warning, Self::Note];

    /// The severity as written in a block.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocker => "blocker",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }

    /// The severity a block spells, or nothing when the word is outside the vocabulary.
    #[must_use]
    pub fn parse(written: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|value| value.as_str() == written)
            .copied()
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// What the reviewer concluded about one finding.
///
/// Two vocabularies in one list, deliberately. The adversary decides per finding — `CONFIRMED`,
/// `NEEDS-CHANGE`, `INFEASIBLE` — and a plan-time critic decides per review — `approve`,
/// `needs-revision`. A block written by either is read by the same parser, and a reader that had to
/// know which agent wrote a document before it could read it would be two formats wearing one name.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Verdict {
    /// The adversary reproduced it.
    #[serde(rename = "CONFIRMED")]
    Confirmed,
    /// The adversary wants it changed.
    #[serde(rename = "NEEDS-CHANGE")]
    NeedsChange,
    /// The adversary could not reach it.
    #[serde(rename = "INFEASIBLE")]
    Infeasible,
    /// A critic accepted the work.
    #[serde(rename = "approve")]
    Approve,
    /// A critic wants the work revised.
    #[serde(rename = "needs-revision")]
    NeedsRevision,
}

impl Verdict {
    /// Every verdict, adversary's first.
    pub const ALL: &'static [Self] = &[
        Self::Confirmed,
        Self::NeedsChange,
        Self::Infeasible,
        Self::Approve,
        Self::NeedsRevision,
    ];

    /// The verdict as written in a block.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "CONFIRMED",
            Self::NeedsChange => "NEEDS-CHANGE",
            Self::Infeasible => "INFEASIBLE",
            Self::Approve => "approve",
            Self::NeedsRevision => "needs-revision",
        }
    }

    /// The verdict a block spells, or nothing when the word is outside the vocabulary.
    #[must_use]
    pub fn parse(written: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|value| value.as_str() == written)
            .copied()
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Whether the work under review put the defect there.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    /// This change introduced it.
    Introduced,
    /// It was already there.
    PreExisting,
    /// Nobody decided which, which is what an unwritten `origin` means.
    Undecided,
}

impl Origin {
    /// Every origin.
    pub const ALL: &'static [Self] = &[Self::Introduced, Self::PreExisting, Self::Undecided];

    /// The origin as written in a block.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Introduced => "introduced",
            Self::PreExisting => "pre-existing",
            Self::Undecided => "undecided",
        }
    }

    /// The origin a block spells, or nothing when the word is outside the vocabulary.
    #[must_use]
    pub fn parse(written: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|value| value.as_str() == written)
            .copied()
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One finding, as a block entry states it.
///
/// Every field is written on the way out, `line` and `verdict` as `null` where they were not
/// written on the way in: a key somebody left out and a key with no value are the same fact here,
/// and a consumer that had to branch on a missing key would be reading two shapes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Finding {
    /// The file it is about, as the reviewer spelled the path.
    pub file: String,
    /// The line, where the reviewer named one.
    pub line: Option<u32>,
    /// What kind of defect it is — the reviewer's own word, not a closed list.
    pub category: String,
    /// How much it is worth reacting to.
    pub severity: Severity,
    /// What the reviewer concluded, where the reviewer concludes per finding.
    pub verdict: Option<Verdict>,
    /// Whether the work under review put it there.
    pub origin: Origin,
    /// What is wrong, in the reviewer's words.
    pub message: String,
}

impl Finding {
    /// What makes two findings the same finding, before the line is considered.
    ///
    /// Deliberately **not** the reviewer. Two reviewers finding the same defect have found one
    /// defect, and a ledger keyed by who said it would report every second opinion as new work.
    /// The reviewer is printed beside the row instead, which is where a reader can use it.
    #[must_use]
    pub fn signature(&self) -> Signature {
        Signature {
            file: self.file.clone(),
            category: self.category.clone(),
            message: normalise(&self.message),
        }
    }

    /// Whether `other` is this finding, allowing for line drift.
    #[must_use]
    pub fn is_the_same_as(&self, other: &Self) -> bool {
        self.signature() == other.signature() && self.within_tolerance(other)
    }

    /// Whether the two lines are close enough to be one place.
    ///
    /// A line neither side wrote is not compared. Unknown is not far away — invariant 5 in the one
    /// place a ledger would otherwise quietly turn a missing observation into a difference.
    fn within_tolerance(&self, other: &Self) -> bool {
        match (self.line, other.line) {
            (Some(here), Some(there)) => here.abs_diff(there) <= LINE_TOLERANCE,
            _ => true,
        }
    }
}

/// What two findings are compared by: the file, the category and the message, normalised.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct Signature {
    /// The file, as written.
    pub file: String,
    /// The category, as written.
    pub category: String,
    /// The message, lowercased with its whitespace collapsed.
    pub message: String,
}

impl fmt::Display for Signature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            self.file, self.category, self.message
        )
    }
}

/// A message as it is compared: lowercased, with every run of whitespace collapsed to one space.
///
/// Two reviewers writing the same sentence with a different wrap are not two findings, and a
/// re-wrapped report is exactly what a second round of the same reviewer produces.
#[must_use]
pub fn normalise(message: &str) -> String {
    message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// What the second review found that the first did not, and the other way round.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    /// In both reviews: the earlier finding and the later one, paired.
    pub carried: Vec<(Finding, Finding)>,
    /// In the later review only.
    pub new: Vec<Finding>,
    /// In the earlier review only.
    pub resolved: Vec<Finding>,
}

/// Classifies the later review's findings against the earlier review's.
///
/// One earlier finding answers for at most one later finding, matched in order. Without that, three
/// identical findings in the earlier review and one in the later would report one carried and two
/// resolved *and* leave the earlier three all matched, which is a count that does not add up.
#[must_use]
pub fn compare(from: &[Finding], to: &[Finding]) -> Ledger {
    let mut spoken_for = vec![false; from.len()];
    let mut ledger = Ledger::default();
    for later in to {
        let matched = from
            .iter()
            .enumerate()
            .find(|(index, earlier)| !spoken_for[*index] && earlier.is_the_same_as(later));
        match matched {
            Some((index, earlier)) => {
                spoken_for[index] = true;
                ledger.carried.push((earlier.clone(), later.clone()));
            }
            None => ledger.new.push(later.clone()),
        }
    }
    for (index, earlier) in from.iter().enumerate() {
        if !spoken_for[index] {
            ledger.resolved.push(earlier.clone());
        }
    }
    ledger
}

/// What a set of findings was read from, which is what a refusal's line number counts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingsInput {
    /// The fenced block in an artifact's body; the line counts the body's lines.
    Body,
    /// A JSON array given on its own, as `new --findings` takes one; the line counts its lines.
    Json,
}

/// Why findings could not be read, **where**, and what the place says.
///
/// A refusal with no position sends the writer back to a document to find a defect this code had
/// already located, so the line is part of the type rather than part of one message's wording. It
/// is the **one** coordinate a refusal carries: the parser's own position inside the block is
/// translated into it rather than printed beside it, because a number counted from the fence next
/// to one counted from the top of the body is two answers to one question (beyond10x/aep#38).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingsError {
    /// What the findings were read from, which says what [`FindingsError::line`] counts.
    pub input: FindingsInput,
    /// The line the defect is on, counting the input's first line as 1.
    pub line: usize,
    /// That line as it is written, where the input has one — a parser that ran off the end of the
    /// input positions a defect on no line, and that is shown as nothing rather than as a blank.
    pub quoted: Option<String>,
    /// What is wrong with it.
    pub detail: String,
}

impl FindingsError {
    /// What to do about it, which is the same advice for every defect in the input.
    ///
    /// The advice names JSON because the defect this most often answers is prose breaking a YAML
    /// scalar — `": "` in a message, then an apostrophe once the message is single-quoted — and a
    /// serializer's JSON quotes every value, which is also what JSON is for a tool that writes one.
    #[must_use]
    pub fn hint(&self) -> &'static str {
        match self.input {
            FindingsInput::Body => {
                "Entries are `{file, line, category, severity, verdict, origin, message}`; `file`, \
                 `category`, `severity` and `message` are required. A value holding `: ` or a \
                 quote breaks unquoted YAML: quote the value with a serializer, or write the block \
                 as JSON — `[{\"file\": \"…\", \"category\": \"…\", \"severity\": \"…\", \
                 \"message\": \"…\"}]` — which is the machine-written form"
            }
            FindingsInput::Json => {
                "`--findings` takes a JSON array of entries `{file, line, category, severity, \
                 verdict, origin, message}`; `file`, `category`, `severity` and `message` are \
                 required"
            }
        }
    }
}

impl fmt::Display for FindingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.input {
            FindingsInput::Body => "the findings block is not readable at line {} of the body",
            FindingsInput::Json => "the findings input is not readable at line {}",
        };
        write!(
            formatter,
            "{}: {}",
            what.replace("{}", &self.line.to_string()),
            self.detail
        )?;
        if let Some(quoted) = &self.quoted {
            write!(formatter, "; the line reads `{quoted}`")?;
        }
        write!(formatter, ". {}", self.hint())
    }
}

impl std::error::Error for FindingsError {}

/// A defect before it is told what it was read from and what its line says.
struct Defect {
    line: usize,
    /// Whether [`Defect::line`] is a place in the input at all. A parser that reports a defect with
    /// no position leaves nothing to quote, and quoting the first line instead would point at a
    /// line that is not wrong.
    positioned: bool,
    detail: String,
}

impl Defect {
    /// A defect on `line`.
    fn at(line: usize, detail: String) -> Self {
        Self {
            line,
            positioned: true,
            detail,
        }
    }

    /// The refusal, with the line quoted out of `text`, which [`Defect::line`] counts in.
    fn within(self, input: FindingsInput, text: &str) -> FindingsError {
        FindingsError {
            input,
            quoted: self
                .line
                .checked_sub(1)
                .filter(|_| self.positioned)
                .and_then(|index| text.lines().nth(index))
                .map(ToOwned::to_owned),
            line: self.line,
            detail: self.detail,
        }
    }
}

/// A parser's message with every position it prints taken out.
///
/// `serde_yaml` and `serde_json` both append their own position to the message, counted from the
/// start of what *they* were given — the block, not the body — and `serde_yaml` appends a second one
/// for the construct it was inside. The line is reported once, translated, by [`FindingsError`];
/// leaving these in is the two coordinate systems beyond10x/aep#38 reports.
///
/// Every form the two parsers print is here, read from their `Display` implementations
/// (`serde_yaml` 0.9 `libyaml/error.rs` and `error.rs`, `serde_json` 1 `error.rs`): ` at line N
/// column M`, from both, for the problem and for its context; and ` at position N`, from
/// `serde_yaml`, for a defect libyaml's *reader* finds — a character YAML does not allow — which it
/// positions by byte offset because the reader runs before lines are counted.
fn without_positions(message: &str) -> String {
    let mut kept = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(character) = rest.chars().next() {
        if let Some(width) = position_width(rest) {
            rest = &rest[width..];
        } else {
            kept.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    kept
}

/// How many bytes the position `text` starts with spans, when it starts with one.
fn position_width(text: &str) -> Option<usize> {
    let digits = |text: &str| text.bytes().take_while(u8::is_ascii_digit).count();
    if let Some(after) = text.strip_prefix(" at line ") {
        let line = digits(after);
        let tail = after[line..].strip_prefix(" column ")?;
        let column = digits(tail);
        (line > 0 && column > 0).then(|| text.len() - tail.len() + column)
    } else if let Some(after) = text.strip_prefix(" at position ") {
        let offset = digits(after);
        (offset > 0).then(|| text.len() - after.len() + offset)
    } else {
        None
    }
}

/// The byte offset a libyaml reader error names, where the message names one.
fn reader_offset(message: &str) -> Option<usize> {
    let after = &message[message.find(" at position ")? + " at position ".len()..];
    let digits = after.bytes().take_while(u8::is_ascii_digit).count();
    after[..digits].parse().ok()
}

/// A fenced `findings` block found in a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The block's YAML, without the fences.
    pub text: String,
    /// The body line the first line of [`Block::text`] is on, counting from 1.
    pub first_line: usize,
}

/// The first fenced `findings` block in `body`, if it has one.
///
/// The opening fence is three or more backticks followed by the info string and nothing else, which
/// is what a markdown renderer reads as one too — a block this finds and a reader's renderer does
/// not would be a document that says two things. For the same reason a `findings` fence *inside*
/// another fenced block is not one: a review quoting an example block in a ` ````markdown ` fence
/// is prose a renderer shows as code, and reading the example as the review's findings would record
/// what it quoted instead of what it found.
#[must_use]
pub fn block(body: &str) -> Option<Block> {
    let (opened, closed) = findings_fence(body)?;
    // An unterminated fence is deliberately not a block: the rest of the document would be read as
    // YAML, and a missing closing fence is a defect [`parse`] reports rather than one this guesses
    // its way around.
    let closed = closed?;
    let lines: Vec<&str> = body.lines().collect();
    Some(Block {
        text: lines[opened + 1..closed].join("\n"),
        first_line: opened + 2,
    })
}

/// A fence line: its character, its length, and its info string.
///
/// Backticks or tildes, three or more, as `CommonMark` has them. A backtick run whose info string
/// holds a backtick is inline code, not a fence.
fn fence(line: &str) -> Option<(char, usize, &str)> {
    let character = line.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let length = line.chars().take_while(|c| *c == character).count();
    if length < 3 {
        return None;
    }
    let info = line[length..].trim();
    (character == '~' || !info.contains('`')).then_some((character, length, info))
}

/// The first `findings` fence at the top level of `body`: the index of its opening line, and of
/// its closing line where it has one.
///
/// Every fence is followed, whatever its info string, because what is inside one is code to a
/// renderer — a `findings` fence nested in it included — until a run of the same character at
/// least as long closes it.
fn findings_fence(body: &str) -> Option<(usize, Option<usize>)> {
    let mut open: Option<(char, usize, bool, usize)> = None;
    for (index, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        match open {
            None => {
                if let Some((character, length, info)) = fence(trimmed) {
                    let findings = character == '`' && info == FENCE_INFO;
                    open = Some((character, length, findings, index));
                }
            }
            Some((character, length, findings, opened)) => {
                let closes = trimmed.chars().count() >= length
                    && trimmed.chars().all(|found| found == character);
                if closes {
                    if findings {
                        return Some((opened, Some(index)));
                    }
                    open = None;
                }
            }
        }
    }
    match open {
        Some((_, _, true, opened)) => Some((opened, None)),
        _ => None,
    }
}

/// Whether `body` opens a `findings` block at all, terminated or not.
///
/// Asked by `validate`, which reports a review that recorded no findings: a review whose block is
/// there and broken has a *different* defect, and reporting it as *absent* would name the wrong
/// repair. Asked by `new --findings` too, which refuses a body that already states findings of its
/// own — broken or not, it is a second answer to the question `--findings` answers. A `findings`
/// fence quoted inside another fence states nothing, as [`block`] says.
#[must_use]
pub fn opens_a_block(body: &str) -> bool {
    findings_fence(body).is_some()
}

/// The findings a body states, or the first defect in the block, positioned.
///
/// A body with no block has no findings and is not an error — most artifacts are not reviews, and
/// `validate` is where a review that should have had one is reported.
///
/// The block is read as YAML, and JSON is the spelling of it a tool should write: every JSON array
/// of entries is a YAML one, and a serializer quotes every message, which is the one thing prose in
/// an unquoted YAML scalar cannot survive.
///
/// # Errors
///
/// A block that is not a YAML sequence of entries, an entry missing a required key, an entry
/// carrying a key this format does not have, or a value outside the vocabulary — positioned at one
/// body line, which the error quotes.
pub fn parse(body: &str) -> Result<Vec<Finding>, FindingsError> {
    let Some(block) = block(body) else {
        if let Some((opened, _)) = findings_fence(body) {
            return Err(Defect::at(
                opened + 1,
                "the block is opened and never closed".to_owned(),
            )
            .within(FindingsInput::Body, body));
        }
        return Ok(Vec::new());
    };
    parse_block(&block).map_err(|defect| defect.within(FindingsInput::Body, body))
}

/// Findings given as a JSON array on their own, read against the entry schema a block is read
/// against — what `new --findings` takes, so a tool need not embed its findings in markdown.
///
/// # Errors
///
/// Text that is not JSON, JSON that is not an array of entries, or anything [`parse`] refuses in a
/// block — positioned at one line of `text`, which the error quotes.
pub fn parse_json(text: &str) -> Result<Vec<Finding>, FindingsError> {
    let raw: Vec<RawFinding> = serde_json::from_str(text).map_err(|error| {
        Defect {
            // `serde_json` counts from 1 and reports 0 only for a defect with no position at all.
            line: error.line().max(1),
            positioned: error.line() > 0,
            detail: without_positions(&error.to_string()),
        }
        .within(FindingsInput::Json, text)
    })?;
    let starts = json_entry_starts(text);
    let lines = text.lines().count();
    validate(&raw, |index, key, value| {
        let (start, end) = entry_span(&starts, index, 1, lines + 1);
        locate(text, 1, start, end, key, value)
    })
    .map_err(|defect| defect.within(FindingsInput::Json, text))
}

/// `findings` as a fenced block, written as JSON, ready to append to a body.
///
/// JSON because it is the form a tool writes and the form every message survives in: the block is
/// read back through [`parse`], and a serializer's quoting is what keeps a `": "` or an apostrophe
/// in a message from being read as structure. One entry per line, so a refusal's line — should a
/// later reader ever raise one — points at one finding.
///
/// Every character YAML does not take as written is escaped, because the block is read as YAML and
/// `serde_json` writes most characters raw: a NEXT LINE would be folded to a space, a DELETE or a C1
/// control refused by the reader, and whatever `parse_json` accepted would not be what `parse` read
/// back. The set kept raw is YAML 1.1's printable characters less NEXT LINE, LINE SEPARATOR,
/// PARAGRAPH SEPARATOR and BYTE ORDER MARK; everything outside it is written `\uXXXX`, which JSON
/// and a YAML double-quoted scalar both decode to the same character.
#[must_use]
pub fn render_block(findings: &[Finding]) -> String {
    let entries: Vec<String> = findings
        .iter()
        .map(|finding| yaml_safe(&serde_json::to_string(finding).unwrap_or_default()))
        .collect();
    let array = if entries.is_empty() {
        "[]".to_owned()
    } else {
        format!("[\n{}\n]", entries.join(",\n"))
    };
    format!("```{FENCE_INFO}\n{array}\n```\n")
}

/// Compact JSON with every character outside [`yaml_printable`] written as a `\u` escape.
///
/// Only inside a string can such a character occur: compact `serde_json` output is otherwise ASCII
/// punctuation, digits and the literals.
fn yaml_safe(json: &str) -> String {
    let mut safe = String::with_capacity(json.len());
    for character in json.chars() {
        if yaml_printable(character) {
            safe.push(character);
        } else {
            // Every character outside the set is in the Basic Multilingual Plane, so four digits.
            let _ = write!(safe, "\\u{:04x}", u32::from(character));
        }
    }
    safe
}

/// Whether libyaml reads `character` in a double-quoted scalar as itself.
///
/// YAML 1.1's printable set (§5.1), which is what libyaml's reader checks, less the three it gives
/// a meaning of its own: NEXT LINE, LINE SEPARATOR and PARAGRAPH SEPARATOR are line breaks, folded
/// inside a quoted scalar, and a BYTE ORDER MARK is dropped. A tab and a line feed are outside it
/// too — `serde_json` already escapes both, and escaping them again would change nothing.
fn yaml_printable(character: char) -> bool {
    matches!(u32::from(character),
        0x20..=0x7E | 0xA0..=0xD7FF | 0xE000..=0xFFFD | 0x1_0000..=0x10_FFFF)
        && !matches!(character, '\u{2028}' | '\u{2029}' | '\u{FEFF}')
}

/// [`parse`], with the block already located.
fn parse_block(block: &Block) -> Result<Vec<Finding>, Defect> {
    if block.text.trim().is_empty() {
        return Err(Defect::at(
            block.first_line,
            // `new` accepts a body with no block, but `validate` then reports the review as prose
            // only, so `[]` is the spelling to send the writer to.
            "the block is empty; a review with no findings writes `[]` in it".to_owned(),
        ));
    }
    let raw: Vec<RawFinding> = serde_yaml::from_str(&block.text).map_err(|error| {
        let message = error.to_string();
        // A reader error carries a byte offset and a zero mark; the offset is the place.
        let line = match reader_offset(&message) {
            Some(offset) => {
                let before = &block.text.as_bytes()[..offset.min(block.text.len())];
                block.first_line + before.split(|byte| *byte == b'\n').count() - 1
            }
            None => error
                .location()
                .map_or(block.first_line, |at| block.first_line + at.line() - 1),
        };
        Defect::at(line, without_positions(&message))
    })?;

    let starts = entry_starts(block);
    let end = block.first_line + block.text.lines().count();
    validate(&raw, |index, key, value| {
        let (start, end) = entry_span(&starts, index, block.first_line, end);
        locate(&block.text, block.first_line, start, end, key, value)
    })
}

/// The lines entry `index` spans, `start` inclusive and `end` exclusive, from where every entry
/// starts; `first` and `last` where there is no start to read.
///
/// Two entries written on one line share it, so an entry's span is never empty.
fn entry_span(starts: &[usize], index: usize, first: usize, last: usize) -> (usize, usize) {
    let start = starts.get(index).copied().unwrap_or(first);
    let end = starts.get(index + 1).copied().unwrap_or(last);
    (start, end.max(start + 1))
}

/// Where a value `validate` refuses is written: its line, and whether that line writes the value
/// or is only the first line of the entry that holds it.
struct Place {
    line: usize,
    exact: bool,
}

/// The vocabulary and blank checks every entry passes, wherever it was read from.
///
/// `at` answers where entry `index` writes `key` with `value`, in the lines the caller counts.
fn validate(
    raw: &[RawFinding],
    at: impl Fn(usize, &str, &str) -> Place,
) -> Result<Vec<Finding>, Defect> {
    let mut findings = Vec::with_capacity(raw.len());
    for (index, entry) in raw.iter().enumerate() {
        let refuse = |key: &str, value: &str, detail: String| -> Defect {
            let place = at(index, key, value);
            if place.exact {
                Defect::at(place.line, detail)
            } else {
                Defect::at(
                    place.line,
                    format!(
                        "{detail} (entry {} starts on this line; the line writing `{key}` was not \
                         found)",
                        index + 1
                    ),
                )
            }
        };
        let severity = Severity::parse(&entry.severity).ok_or_else(|| {
            refuse(
                "severity",
                &entry.severity,
                format!(
                    "`{}` is not a severity; write one of {}",
                    entry.severity,
                    spelled(Severity::ALL.iter().map(|value| value.as_str()))
                ),
            )
        })?;
        let verdict = match &entry.verdict {
            None => None,
            Some(written) => Some(Verdict::parse(written).ok_or_else(|| {
                refuse(
                    "verdict",
                    written,
                    format!(
                        "`{written}` is not a verdict; write one of {}",
                        spelled(Verdict::ALL.iter().map(|value| value.as_str()))
                    ),
                )
            })?),
        };
        let origin = match &entry.origin {
            None => Origin::Undecided,
            Some(written) => Origin::parse(written).ok_or_else(|| {
                refuse(
                    "origin",
                    written,
                    format!(
                        "`{written}` is not an origin; write one of {}",
                        spelled(Origin::ALL.iter().map(|value| value.as_str()))
                    ),
                )
            })?,
        };
        if entry.file.trim().is_empty() || entry.category.trim().is_empty() {
            let (key, value) = if entry.file.trim().is_empty() {
                ("file", &entry.file)
            } else {
                ("category", &entry.category)
            };
            return Err(refuse(
                key,
                value,
                "`file` and `category` are what a finding is compared by, so neither may be blank"
                    .to_owned(),
            ));
        }
        if entry.message.trim().is_empty() {
            return Err(refuse(
                "message",
                &entry.message,
                "`message` is what the finding says, and a blank one says nothing".to_owned(),
            ));
        }
        findings.push(Finding {
            file: entry.file.clone(),
            line: entry.line,
            category: entry.category.clone(),
            severity,
            verdict,
            origin,
            message: entry.message.clone(),
        });
    }
    Ok(findings)
}

/// A vocabulary, as a refusal lists it.
fn spelled<'a>(values: impl Iterator<Item = &'a str>) -> String {
    values.collect::<Vec<_>>().join(", ")
}

/// The body line each top-level entry of the block starts on.
///
/// Read from the text rather than from the parser, because `serde_yaml` reports the position of the
/// *sequence*, not of the value inside it, once the entry has already deserialized — and the value
/// inside it is exactly what a vocabulary refusal is about. A block written as JSON — or as any
/// flow sequence — has no `- ` lines, and its entries are the objects the array holds.
fn entry_starts(block: &Block) -> Vec<usize> {
    if block.text.trim_start().starts_with('[') {
        return json_entry_starts(&block.text)
            .into_iter()
            .map(|line| block.first_line + line - 1)
            .collect();
    }
    block
        .text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with("- ") || line.trim_end() == "-")
        .map(|(index, _)| block.first_line + index)
        .collect()
}

/// The line, counting `text`'s first as 1, each object directly inside the top-level array opens
/// on. Strings are skipped, so a brace inside a message is not an entry.
fn json_entry_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let (mut line, mut depth) = (1, 0_usize);
    let (mut in_string, mut escaped) = (false, false);
    for character in text.chars() {
        if in_string {
            match (escaped, character) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
        } else {
            match character {
                '"' => in_string = true,
                '{' | '[' => {
                    if character == '{' && depth == 1 {
                        starts.push(line);
                    }
                    depth += 1;
                }
                '}' | ']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if character == '\n' {
            line += 1;
        }
    }
    starts
}

/// Where entry lines `start..end` of `text` write `key` with `value` — lines numbered from `first`.
///
/// The YAML spelling `key: value` first; then, since the block may be written as JSON, a line
/// whose JSON strings **decode** to the key and to the value, so a serializer that escaped the value
/// — `"blöcker"` — is found as surely as one that did not. Where neither is found the place is
/// the entry's own first line, and [`Place::exact`] says so rather than letting it pass for the
/// line that is wrong.
fn locate(text: &str, first: usize, start: usize, end: usize, key: &str, value: &str) -> Place {
    let wanted = format!("{key}: {value}");
    let lines = || {
        text.lines()
            .enumerate()
            .map(|(index, line)| (first + index, line))
            .filter(|(line_number, _)| *line_number >= start && *line_number < end)
    };
    lines()
        .find(|(_, line)| line.trim().trim_start_matches("- ").trim() == wanted)
        .or_else(|| lines().find(|(_, line)| writes_as_json(line, key, value)))
        .map_or(
            Place {
                line: start,
                exact: false,
            },
            |(line, _)| Place { line, exact: true },
        )
}

/// Whether `line` holds a JSON string decoding to `key` and one decoding to `value`.
fn writes_as_json(line: &str, key: &str, value: &str) -> bool {
    let strings = json_strings(line);
    strings.iter().any(|string| string == key) && strings.iter().any(|string| string == value)
}

/// Every complete JSON string on `line`, decoded. One that does not decode is left out.
fn json_strings(line: &str) -> Vec<String> {
    let mut strings = Vec::new();
    let mut opened: Option<usize> = None;
    let mut escaped = false;
    for (at, character) in line.char_indices() {
        match opened {
            None => {
                if character == '"' {
                    opened = Some(at);
                }
            }
            Some(from) => match (escaped, character) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => {
                    if let Ok(decoded) = serde_json::from_str::<String>(&line[from..=at]) {
                        strings.push(decoded);
                    }
                    opened = None;
                }
                _ => {}
            },
        }
    }
    strings
}

/// One entry as written, before its vocabulary is checked.
///
/// Parse, then validate: this deserializes and [`Finding`] is what a caller may hold, so a block
/// wrong about a key and wrong about a severity is reported about the key first, at the key's line,
/// rather than about whichever `serde` reached.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFinding {
    file: String,
    #[serde(default)]
    line: Option<u32>,
    category: String,
    severity: String,
    #[serde(default)]
    verdict: Option<String>,
    #[serde(default)]
    origin: Option<String>,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::{
        compare, normalise, parse, parse_json, render_block, Finding, Origin, Severity, Verdict,
    };

    /// A body with `entries` as its block.
    fn body(entries: &str) -> String {
        format!("# A review\n\nProse first.\n\n```findings\n{entries}```\n")
    }

    /// The one-line spelling of a finding.
    fn one(file: &str, line: &str, message: &str) -> String {
        format!(
            "- file: {file}\n  line: {line}\n  category: correctness\n  severity: blocker\n  \
             verdict: CONFIRMED\n  origin: introduced\n  message: {message}\n"
        )
    }

    #[test]
    fn a_body_with_no_block_has_no_findings_and_is_not_an_error() {
        assert_eq!(
            parse("# A story\n\nNothing fenced here.\n").expect("no block is not a defect"),
            Vec::new()
        );
    }

    #[test]
    fn an_empty_block_is_refused_with_the_accepted_spelling_of_no_findings() {
        let error = parse(&body("")).expect_err("an empty block is a defect");
        assert!(
            error.detail.contains("`[]`"),
            "the refusal names `[]` as the way to record no findings: {error}"
        );
        assert!(
            !error.detail.contains("writes no block at all"),
            "the refusal does not tell the writer to drop the block: {error}"
        );
    }

    #[test]
    fn an_empty_sequence_is_a_review_with_no_findings() {
        assert_eq!(
            parse(&body("[]\n")).expect("`[]` is the accepted spelling of no findings"),
            Vec::new()
        );
    }

    #[test]
    fn every_key_of_an_entry_is_read_and_an_unwritten_origin_is_undecided() {
        let findings = parse(&body(
            "- file: src/a.rs\n  line: 12\n  category: correctness\n  severity: warning\n  \
             message: it drifts\n",
        ))
        .expect("a well-formed block");
        assert_eq!(
            findings,
            vec![Finding {
                file: "src/a.rs".to_owned(),
                line: Some(12),
                category: "correctness".to_owned(),
                severity: Severity::Warning,
                verdict: None,
                origin: Origin::Undecided,
                message: "it drifts".to_owned(),
            }]
        );
    }

    #[test]
    fn each_verdict_spelling_of_both_vocabularies_is_read() {
        for verdict in Verdict::ALL {
            let findings = parse(&body(&format!(
                "- file: a.rs\n  category: c\n  severity: note\n  verdict: {}\n  message: m\n",
                verdict.as_str()
            )))
            .expect("a verdict in the vocabulary");
            assert_eq!(findings[0].verdict, Some(*verdict));
        }
    }

    #[test]
    fn a_value_outside_the_vocabulary_is_refused_at_the_line_it_is_written_on() {
        // Body line 5 opens the fence, so the first entry starts on line 6 and `severity` is on 9.
        let error = parse(&body(
            "- file: a.rs\n  line: 1\n  category: correctness\n  severity: catastrophic\n  \
             message: boom\n",
        ))
        .expect_err("a severity outside the vocabulary");
        assert_eq!(error.line, 9, "{error}");
        assert!(error.detail.contains("catastrophic"), "{error}");
    }

    #[test]
    fn a_key_this_format_does_not_have_is_refused() {
        let error = parse(&body(
            "- file: a.rs\n  category: c\n  severity: note\n  message: m\n  confidence: 0.9\n",
        ))
        .expect_err("an unknown key");
        assert!(error.detail.contains("confidence"), "{error}");
    }

    #[test]
    fn an_unterminated_block_is_refused_rather_than_read_to_the_end_of_the_body() {
        let error =
            parse("# A review\n\n```findings\n- file: a.rs\n").expect_err("an unterminated fence");
        assert_eq!(error.line, 3, "{error}");
        assert!(error.detail.contains("never closed"), "{error}");
    }

    #[test]
    fn a_message_is_compared_without_its_case_or_its_wrapping() {
        assert_eq!(
            normalise("The  loop\n never   advances"),
            "the loop never advances"
        );
    }

    #[test]
    fn a_finding_that_moved_two_lines_is_carried_and_one_that_moved_four_is_not() {
        let first =
            parse(&body(&one("a.rs", "40", "the loop never advances"))).expect("the first review");
        let moved_two =
            parse(&body(&one("a.rs", "42", "The loop  never advances"))).expect("the second");
        let moved_four = parse(&body(&one("a.rs", "44", "the loop never advances")))
            .expect("the second, further");

        let ledger = compare(&first, &moved_two);
        assert_eq!(ledger.carried.len(), 1, "{ledger:?}");
        assert!(
            ledger.new.is_empty() && ledger.resolved.is_empty(),
            "{ledger:?}"
        );

        let ledger = compare(&first, &moved_four);
        assert_eq!(ledger.new.len(), 1, "{ledger:?}");
        assert_eq!(ledger.resolved.len(), 1, "{ledger:?}");
    }

    #[test]
    fn one_earlier_finding_answers_for_at_most_one_later_finding() {
        let first = parse(&body(&format!(
            "{}{}",
            one("a.rs", "10", "same words"),
            one("a.rs", "10", "same words")
        )))
        .expect("two identical findings");
        let second = parse(&body(&one("a.rs", "10", "same words"))).expect("one of them");
        let ledger = compare(&first, &second);
        assert_eq!(ledger.carried.len(), 1, "{ledger:?}");
        assert_eq!(ledger.resolved.len(), 1, "{ledger:?}");
        assert!(ledger.new.is_empty(), "{ledger:?}");
    }

    /// The reproduction from beyond10x/aep#38: prose a reviewer writes, unquoted.
    const PROSE: &str = "the judge reports \"(platform): X\" for an unmeasured step";

    #[test]
    fn a_block_written_as_json_reads_messages_carrying_colons_quotes_and_apostrophes() {
        let findings = parse(&body(
            "[\n  {\"file\": \"src/lib.rs\", \"line\": 12, \"category\": \"acceptance\", \
             \"severity\": \"warning\", \"verdict\": \"CONFIRMED\", \"origin\": \"introduced\", \
             \"message\": \"the judge reports \\\"(platform): X\\\" for an unmeasured step\"},\n  \
             {\"file\": \"src/lib.rs\", \"category\": \"acceptance\", \"severity\": \"note\", \
             \"message\": \"the caller's wait: never bounded\"}\n]\n",
        ))
        .expect("JSON is the machine-written form of the block");
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert_eq!(findings[0].message, PROSE);
        assert_eq!(findings[1].message, "the caller's wait: never bounded");
    }

    #[test]
    fn a_block_that_does_not_parse_is_refused_at_one_body_line_quoting_it_with_a_json_hint() {
        // The fence opens on body line 5; `one` writes `message` as the seventh key, on line 12.
        let error = parse(&body(&one("src/lib.rs", "12", PROSE))).expect_err("unquoted prose");
        assert_eq!(error.line, 12, "{error}");
        assert_eq!(
            error.quoted.as_deref(),
            Some(format!("  message: {PROSE}").as_str()),
            "{error}"
        );
        let said = error.to_string();
        assert!(said.contains("at line 12 of the body"), "{said}");
        assert_eq!(
            said.matches("at line").count(),
            1,
            "a second coordinate system is reported: {said}"
        );
        assert!(!said.contains("column"), "a block-relative column: {said}");
        assert!(
            said.contains(PROSE),
            "the offending line is not quoted: {said}"
        );
        assert!(said.contains("JSON"), "the hint does not name JSON: {said}");
    }

    #[test]
    fn a_value_outside_the_vocabulary_is_refused_with_its_own_line_quoted() {
        let error = parse(&body(
            "- file: a.rs\n  category: correctness\n  severity: catastrophic\n  message: boom\n",
        ))
        .expect_err("a severity outside the vocabulary");
        assert_eq!(error.line, 8, "{error}");
        assert_eq!(error.quoted.as_deref(), Some("  severity: catastrophic"));
    }

    #[test]
    fn findings_given_as_json_are_read_against_the_same_entry_schema() {
        let findings = parse_json(
            "[{\"file\": \"a.rs\", \"category\": \"c\", \"severity\": \"note\", \
             \"message\": \"the caller's wait: never bounded\"}]",
        )
        .expect("a JSON array of entries");
        assert_eq!(findings[0].origin, Origin::Undecided);
        assert_eq!(findings[0].message, "the caller's wait: never bounded");

        let unknown = parse_json(
            "[{\"file\": \"a.rs\", \"category\": \"c\", \"severity\": \"note\", \
             \"message\": \"m\", \"confidence\": 0.9}]",
        )
        .expect_err("a key this format does not have");
        assert!(unknown.detail.contains("confidence"), "{unknown}");
        assert!(unknown.to_string().contains("findings input"), "{unknown}");

        let vocabulary = parse_json(
            "[\n  {\n    \"file\": \"a.rs\",\n    \"category\": \"c\",\n    \
             \"severity\": \"catastrophic\",\n    \"message\": \"m\"\n  }\n]\n",
        )
        .expect_err("a severity outside the vocabulary");
        assert_eq!(vocabulary.line, 5, "{vocabulary}");
        assert_eq!(
            vocabulary.quoted.as_deref(),
            Some("    \"severity\": \"catastrophic\",")
        );
    }

    #[test]
    fn findings_input_that_is_not_json_is_refused_at_its_line_with_the_text_quoted() {
        let error = parse_json("[\n  {\"file\": \"a.rs\",}\n]\n").expect_err("a trailing comma");
        assert_eq!(error.line, 2, "{error}");
        assert_eq!(error.quoted.as_deref(), Some("  {\"file\": \"a.rs\",}"));
        assert!(!error.to_string().contains("column"), "{error}");
    }

    #[test]
    fn rendered_findings_read_back_from_a_body_as_the_same_findings() {
        let given = parse_json(
            "[{\"file\": \"a.rs\", \"line\": 3, \"category\": \"c\", \"severity\": \"blocker\", \
             \"verdict\": \"NEEDS-CHANGE\", \"origin\": \"pre-existing\", \
             \"message\": \"the judge reports \\\"(platform): X\\\" for the caller's step\\n```\"},\
             {\"file\": \"b.rs\", \"category\": \"c\", \"severity\": \"note\", \"message\": \"m\"}]",
        )
        .expect("a JSON array of entries");
        let stored = format!("# A review\n\nProse first.\n\n{}", render_block(&given));
        assert_eq!(parse(&stored).expect("the rendered block reads"), given);
        assert_eq!(
            parse(&format!("# A review\n\n{}", render_block(&[])))
                .expect("no findings renders as `[]`"),
            Vec::new()
        );
    }

    /// The class behind correction round 1, finding 1, checked whole rather than per character: every
    /// Unicode scalar value a message can hold — which is every one, since JSON can escape any — reads
    /// back from the block [`render_block`] writes as the character it was.
    #[test]
    fn every_character_a_message_can_hold_reads_back_unchanged_from_a_rendered_block() {
        let every: Vec<char> = (0..=0x10_FFFF).filter_map(char::from_u32).collect();
        let mut failures = Vec::new();
        for chunk in every.chunks(4096) {
            let message: String = chunk.iter().collect();
            let given = vec![Finding {
                file: "a.rs".to_owned(),
                line: None,
                category: "c".to_owned(),
                severity: Severity::Note,
                verdict: None,
                origin: Origin::Undecided,
                message,
            }];
            let body = format!("# A review\n\n{}", render_block(&given));
            match parse(&body) {
                Ok(read) if read == given => {}
                Ok(read) => {
                    let wrote: Vec<char> = given[0].message.chars().collect();
                    let got: Vec<char> = read[0].message.chars().collect();
                    let first = wrote
                        .iter()
                        .zip(&got)
                        .position(|(a, b)| a != b)
                        .unwrap_or(wrote.len().min(got.len()));
                    failures.push(format!(
                        "U+{:04X}..: changed from character {first}",
                        u32::from(chunk[0])
                    ));
                }
                Err(error) => {
                    failures.push(format!("U+{:04X}..: refused: {error}", u32::from(chunk[0])));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn a_position_either_parser_prints_is_taken_out_of_the_detail() {
        assert_eq!(
            super::without_positions(
                "did not find expected key at line 14 column 47, while parsing a block mapping \
                 at line 9 column 3"
            ),
            "did not find expected key, while parsing a block mapping"
        );
        assert_eq!(
            super::without_positions("control characters are not allowed at position 71"),
            "control characters are not allowed"
        );
        assert_eq!(
            super::without_positions("a message about line 3 at line 2"),
            "a message about line 3 at line 2",
            "text that is not a whole position is kept"
        );
    }

    #[test]
    fn a_reader_error_is_positioned_at_the_body_line_holding_its_offset() {
        let error = parse(
            "# A review\n\n```findings\n[\n{\"file\": \"a.rs\", \"category\": \"c\", \"severity\": \
             \"note\",\n \"message\": \"del\u{7f}char\"}\n]\n```\n",
        )
        .expect_err("libyaml refuses a DEL");
        assert_eq!(error.line, 6, "{error}");
        assert_eq!(
            error.quoted.as_deref(),
            Some(" \"message\": \"del\u{7f}char\"}")
        );
        assert!(!error.to_string().contains("position"), "{error}");
    }

    #[test]
    fn a_value_not_found_on_any_line_is_reported_at_its_entry_and_says_so() {
        // The severity is split from its key across two lines, so no one line writes the pair.
        let error = parse_json(
            "[\n  {\"file\": \"a.rs\", \"category\": \"c\", \"message\": \"m\",\n   \
             \"severity\":\n   \"catastrophic\"}\n]\n",
        )
        .expect_err("a severity outside the vocabulary");
        assert_eq!(error.line, 2, "{error}");
        assert!(
            error.detail.contains("entry 1 starts on this line"),
            "{error}"
        );
    }

    #[test]
    fn an_escaped_value_in_a_json_block_is_located_by_what_it_decodes_to() {
        let error = parse(&body(
            "[\n{\"file\": \"a.rs\", \"category\": \"c\", \"severity\": \"note\", \"message\": \"m\"},\n\
             {\"file\": \"b.rs\", \"category\": \"c\", \"severity\": \"bl\\u00f6cker\", \
             \"message\": \"m\"}\n]\n",
        ))
        .expect_err("`blöcker` is not a severity");
        // The fence opens on body line 5: `[` is 6, the first entry 7, the second 8.
        assert_eq!(error.line, 8, "{error}");
        assert!(!error.detail.contains("was not found"), "{error}");

        // Written the way `json.dumps(indent=2)` writes it, where the entry's first line is `{`.
        let error = parse_json(
            "[\n  {\n    \"file\": \"a.rs\",\n    \"category\": \"c\",\n    \
             \"severity\": \"bl\\u00f6cker\",\n    \"message\": \"m\"\n  }\n]\n",
        )
        .expect_err("`blöcker` is not a severity");
        assert_eq!(error.line, 5, "{error}");
        assert_eq!(
            error.quoted.as_deref(),
            Some("    \"severity\": \"bl\\u00f6cker\",")
        );
    }

    #[test]
    fn a_fence_of_either_character_hides_the_findings_fence_it_quotes() {
        for outer in ["````", "~~~", "```markdown"] {
            let close = if outer.starts_with('~') {
                "~~~"
            } else if outer == "```markdown" {
                "```"
            } else {
                "````"
            };
            let quoted = format!("# R\n\n{outer}\n```findings\n[]\n```\n{close}\n");
            // A ```markdown fence is closed by the example's own ``` line — CommonMark reads it so,
            // and the `findings` fence then sits inside it.
            assert!(!super::opens_a_block(&quoted), "{outer}: {quoted}");
            assert_eq!(super::block(&quoted), None, "{outer}");
        }
        assert!(
            super::opens_a_block("# R\n\n```rust\nlet x = 1;\n```\n\n```findings\n[]\n```\n"),
            "a closed fence before the block does not hide it"
        );
        assert_eq!(
            parse("# R\n\n````\n```findings\n- nonsense: [\n```\n````\n").expect("prose only"),
            Vec::new()
        );
    }

    #[test]
    fn a_line_neither_side_wrote_is_not_a_difference() {
        let without = parse(&body(
            "- file: a.rs\n  category: c\n  severity: note\n  message: m\n",
        ))
        .expect("no line");
        let with = parse(&body(
            "- file: a.rs\n  line: 900\n  category: c\n  severity: note\n  message: m\n",
        ))
        .expect("a line");
        assert_eq!(compare(&without, &with).carried.len(), 1);
    }
}
