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
//! - file: crates/aep-domain/src/artifact.rs
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

/// Why a block could not be read, and **where**.
///
/// A refusal with no position sends the writer back to a document to find a defect this code had
/// already located, so the line is part of the type rather than part of one message's wording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingsError {
    /// The line of the **body** the defect is on, counting the body's first line as 1.
    pub line: usize,
    /// What is wrong with it.
    pub detail: String,
}

impl fmt::Display for FindingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "the findings block is not readable at line {} of the body: {}",
            self.line, self.detail
        )
    }
}

impl std::error::Error for FindingsError {}

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
/// not would be a document that says two things.
#[must_use]
pub fn block(body: &str) -> Option<Block> {
    let lines: Vec<&str> = body.lines().collect();
    let mut opened: Option<(usize, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        match opened {
            None => {
                if let Some(fence) = opening_fence(trimmed) {
                    opened = Some((index + 1, fence));
                }
            }
            Some((first, fence)) => {
                if trimmed.starts_with(&"`".repeat(fence))
                    && trimmed.chars().all(|character| character == '`')
                {
                    return Some(Block {
                        text: lines[first..index].join("\n"),
                        first_line: first + 1,
                    });
                }
            }
        }
    }
    // An unterminated fence is deliberately not a block: the rest of the document would be read as
    // YAML, and a missing closing fence is a defect [`parse`] reports rather than one this guesses
    // its way around.
    None
}

/// The length of the opening fence, when `line` opens a `findings` block.
fn opening_fence(line: &str) -> Option<usize> {
    let backticks = line
        .chars()
        .take_while(|character| *character == '`')
        .count();
    if backticks < 3 {
        return None;
    }
    (line[backticks..].trim() == FENCE_INFO).then_some(backticks)
}

/// Whether `body` opens a `findings` block at all, terminated or not.
///
/// Asked by `validate`, which reports a review that recorded no findings: a review whose block is
/// there and broken has a *different* defect, and reporting it as *absent* would name the wrong
/// repair.
#[must_use]
pub fn opens_a_block(body: &str) -> bool {
    body.lines()
        .any(|line| opening_fence(line.trim()).is_some())
}

/// The findings a body states, or the first defect in the block, positioned.
///
/// A body with no block has no findings and is not an error — most artifacts are not reviews, and
/// `validate` is where a review that should have had one is reported.
///
/// # Errors
///
/// A block that is not a YAML sequence of entries, an entry missing a required key, an entry
/// carrying a key this format does not have, or a value outside the vocabulary.
pub fn parse(body: &str) -> Result<Vec<Finding>, FindingsError> {
    let Some(block) = block(body) else {
        if opens_a_block(body) {
            return Err(FindingsError {
                line: body
                    .lines()
                    .position(|line| opening_fence(line.trim()).is_some())
                    .map_or(1, |index| index + 1),
                detail: "the block is opened and never closed".to_owned(),
            });
        }
        return Ok(Vec::new());
    };
    parse_block(&block)
}

/// [`parse`], with the block already located.
fn parse_block(block: &Block) -> Result<Vec<Finding>, FindingsError> {
    if block.text.trim().is_empty() {
        return Err(FindingsError {
            line: block.first_line,
            detail: "the block is empty; a review with nothing to report writes no block at all"
                .to_owned(),
        });
    }
    let raw: Vec<RawFinding> =
        serde_yaml::from_str(&block.text).map_err(|error| FindingsError {
            line: error
                .location()
                .map_or(block.first_line, |at| block.first_line + at.line() - 1),
            detail: error.to_string(),
        })?;

    let starts = entry_starts(block);
    let mut findings = Vec::with_capacity(raw.len());
    for (index, entry) in raw.iter().enumerate() {
        let start = starts.get(index).copied().unwrap_or(block.first_line);
        let end = starts
            .get(index + 1)
            .copied()
            .unwrap_or(block.first_line + block.text.lines().count());
        let at = |key: &str, value: &str| -> usize { locate(block, start, end, key, value) };

        let severity = Severity::parse(&entry.severity).ok_or_else(|| FindingsError {
            line: at("severity", &entry.severity),
            detail: format!(
                "`{}` is not a severity; write one of {}",
                entry.severity,
                spelled(Severity::ALL.iter().map(|value| value.as_str()))
            ),
        })?;
        let verdict = match &entry.verdict {
            None => None,
            Some(written) => Some(Verdict::parse(written).ok_or_else(|| FindingsError {
                line: at("verdict", written),
                detail: format!(
                    "`{written}` is not a verdict; write one of {}",
                    spelled(Verdict::ALL.iter().map(|value| value.as_str()))
                ),
            })?),
        };
        let origin = match &entry.origin {
            None => Origin::Undecided,
            Some(written) => Origin::parse(written).ok_or_else(|| FindingsError {
                line: at("origin", written),
                detail: format!(
                    "`{written}` is not an origin; write one of {}",
                    spelled(Origin::ALL.iter().map(|value| value.as_str()))
                ),
            })?,
        };
        if entry.file.trim().is_empty() || entry.category.trim().is_empty() {
            return Err(FindingsError {
                line: start,
                detail: "`file` and `category` are what a finding is compared by, so neither may \
                         be blank"
                    .to_owned(),
            });
        }
        if entry.message.trim().is_empty() {
            return Err(FindingsError {
                line: start,
                detail: "`message` is what the finding says, and a blank one says nothing"
                    .to_owned(),
            });
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
/// inside it is exactly what a vocabulary refusal is about.
fn entry_starts(block: &Block) -> Vec<usize> {
    block
        .text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with("- ") || line.trim_end() == "-")
        .map(|(index, _)| block.first_line + index)
        .collect()
}

/// The body line `<key>: <value>` is written on inside one entry, or the entry's own first line.
fn locate(block: &Block, start: usize, end: usize, key: &str, value: &str) -> usize {
    let wanted = format!("{key}: {value}");
    block
        .text
        .lines()
        .enumerate()
        .map(|(index, line)| (block.first_line + index, line))
        .find(|(line_number, line)| {
            *line_number >= start
                && *line_number < end
                && line.trim().trim_start_matches("- ").trim() == wanted
        })
        .map_or(start, |(line_number, _)| line_number)
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
    use super::{compare, normalise, parse, Finding, Origin, Severity, Verdict};

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
