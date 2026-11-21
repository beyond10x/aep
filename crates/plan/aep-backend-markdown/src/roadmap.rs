//! The fenced `roadmap` block a `roadmap` body carries: the epics it orders, first to last.
//!
//! ````text
//! ```roadmap
//! - epic: epic:brownfield-entry
//!   horizon: now
//!   rationale: Every adopted store is blocked until it lands.
//! - epic: epic:requester-side
//!   horizon: next
//! ```
//! ````
//!
//! # Why a block in the body
//!
//! For the reason a `review-result`'s `findings` are one ([`crate::findings`]): the order is what
//! the roadmap *says*, so it belongs in the prose a person reads and approves, and `aep plan artifact
//! body` is already the verb that writes prose. A front-matter key would be a second place a roadmap
//! could be edited and a second verb to keep honest.
//!
//! # How order is written
//!
//! **Position is the order, and `horizon` is a coarse band over it.** The list is read first to
//! last, the way `transitions` and a wave plan are, and `horizon` is an ordered vocabulary — `now`,
//! `next`, `later` — in the way the protocol's `scales` are. No entry carries a sequence number:
//! inserting one would renumber every entry below it, and a diff of a roadmap would be noise. The
//! horizon never goes back down the list, so the two cannot disagree.
//!
//! # One block, either fence
//!
//! A body states its order once. A second top-level `roadmap` block is refused rather than ignored,
//! because a reader sees both and a check that read only the first would approve an order nobody
//! checked. A block fenced with tildes is read like one fenced with backticks, since a renderer shows
//! them alike.
//!
//! This module reads one body and judges what one body can judge: its shape, its vocabulary, that no
//! epic is named twice, and that the horizon never goes back. Whether each entry is an epic the
//! store holds is a question about the store, and `aep plan artifact validate` asks it.

use std::collections::BTreeMap;
use std::fmt;

use aep_domain::artifact::ArtifactId;

/// The info string that marks the block.
pub const FENCE_INFO: &str = "roadmap";

/// How far off an epic is, as an ordered band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Horizon {
    /// Being worked on, or the next thing picked up.
    Now,
    /// After what is `now`.
    Next,
    /// Agreed to come, not yet sequenced closer.
    Later,
}

impl Horizon {
    /// Every horizon, nearest first.
    pub const ALL: &'static [Self] = &[Self::Now, Self::Next, Self::Later];

    /// The horizon as written in a block.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Now => "now",
            Self::Next => "next",
            Self::Later => "later",
        }
    }

    /// The horizon a block spells, or nothing when the word is outside the vocabulary.
    #[must_use]
    pub fn parse(written: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|value| value.as_str() == written)
            .copied()
    }
}

impl fmt::Display for Horizon {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One entry of a roadmap, as read.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Entry {
    /// The epic it places, as written.
    pub epic: ArtifactId,
    /// How far off it is.
    pub horizon: Horizon,
    /// Why it sits there, when the roadmap says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// An entry as written, before its values are read against the vocabulary.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    epic: String,
    horizon: String,
    #[serde(default)]
    rationale: Option<String>,
}

/// Why a block, or one entry of it, could not be read, and the body line it is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapError {
    /// The body line, counting from 1.
    pub line: usize,
    /// What is wrong.
    pub detail: String,
}

impl fmt::Display for RoadmapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "the roadmap block is not readable at line {} of the body: {}. Entries are `{{epic, \
             horizon, rationale}}`; `epic` and `horizon` are required, and `horizon` is one of now, \
             next or later",
            self.line, self.detail
        )
    }
}

impl std::error::Error for RoadmapError {}

/// A top-level `roadmap` fence: the index of its opening line, and of its closing line if any.
type Fence = (usize, Option<usize>);

/// Every top-level fence in `body` whose info string is [`FENCE_INFO`], backticks or tildes.
///
/// Every fence is followed, whatever its info string, so a `roadmap` fence quoted inside another
/// fence is code a renderer shows, not the roadmap.
fn roadmap_fences(body: &str) -> Vec<Fence> {
    let mut found = Vec::new();
    let mut open: Option<(char, usize, bool, usize)> = None;
    for (index, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        match open {
            None => {
                if let Some((character, length, info)) = crate::findings::fence(trimmed) {
                    open = Some((character, length, info == FENCE_INFO, index));
                }
            }
            Some((character, length, named, opened)) => {
                let closes = trimmed.chars().count() >= length
                    && trimmed.chars().all(|found| found == character);
                if closes {
                    if named {
                        found.push((opened, Some(index)));
                    }
                    open = None;
                }
            }
        }
    }
    if let Some((_, _, true, opened)) = open {
        found.push((opened, None));
    }
    found
}

/// The body line each entry of a block starts on, when the block's own layout says so.
///
/// An entry starts on a line that opens a sequence item (`- `) or a flow mapping (`{`) at the
/// block's smallest such indentation. When that count is not the number of entries — a layout this
/// does not recognise — every entry is placed at the block's first line, which is still a line in
/// the block rather than a guess outside it.
fn entry_lines(text: &str, first_line: usize, entries: usize) -> Vec<usize> {
    let starts: Vec<(usize, usize)> = text
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let content = line.trim_start();
            let opens = content == "-" || content.starts_with("- ") || content.starts_with('{');
            opens.then(|| (line.len() - content.len(), index))
        })
        .collect();
    let indent = starts.iter().map(|(indent, _)| *indent).min();
    let lines: Vec<usize> = starts
        .iter()
        .filter(|(at, _)| Some(*at) == indent)
        .map(|(_, index)| first_line + index)
        .collect();
    if lines.len() == entries {
        lines
    } else {
        vec![first_line; entries]
    }
}

/// The entries a body's block states, `None` for a body with no block, or every reason the block
/// is not readable.
///
/// An empty list (`[]`) is a roadmap that orders nothing yet, which is what the template seeds.
///
/// # Errors
///
/// A block opened and never closed, a second block, YAML that is not a sequence, and — every one of
/// them, each at its entry's line — an entry with a key this format does not have or without a
/// required one, an `epic` that is not an artifact id, or a `horizon` outside the vocabulary.
pub fn recorded(body: &str) -> Result<Option<Vec<Entry>>, Vec<RoadmapError>> {
    let fences = roadmap_fences(body);
    let Some(&(opened, closed)) = fences.first() else {
        return Ok(None);
    };
    let mut errors: Vec<RoadmapError> = fences
        .iter()
        .skip(1)
        .map(|(second, _)| RoadmapError {
            line: second + 1,
            detail: format!(
                "a second `roadmap` block (the first opens at line {}); a roadmap states its order \
                 once, in one block",
                opened + 1
            ),
        })
        .collect();
    let Some(closed) = closed else {
        errors.push(RoadmapError {
            line: opened + 1,
            detail: "the block is opened and never closed".to_owned(),
        });
        return Err(errors);
    };
    let lines: Vec<&str> = body.lines().collect();
    let text = lines[opened + 1..closed].join("\n");
    let first_line = opened + 2;
    if text.trim().is_empty() {
        errors.push(RoadmapError {
            line: first_line,
            detail: "the block is empty; a roadmap that orders nothing yet writes `[]`".to_owned(),
        });
        return Err(errors);
    }
    let values: Vec<serde_yaml::Value> = match serde_yaml::from_str(&text) {
        Ok(values) => values,
        Err(error) => {
            errors.push(RoadmapError {
                line: error
                    .location()
                    .map_or(first_line, |at| first_line + at.line().saturating_sub(1)),
                detail: error.to_string(),
            });
            return Err(errors);
        }
    };
    let at = entry_lines(&text, first_line, values.len());
    let mut entries = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let position = index + 1;
        let line = at[index];
        let entry: RawEntry = match serde_yaml::from_value(value) {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(RoadmapError {
                    line,
                    detail: format!("entry {position}: {error}"),
                });
                continue;
            }
        };
        let epic = ArtifactId::new(&entry.epic).map_err(|error| RoadmapError {
            line,
            detail: format!(
                "entry {position}: `{}` is not an artifact id: {error}",
                entry.epic
            ),
        });
        let horizon = Horizon::parse(&entry.horizon).ok_or_else(|| RoadmapError {
            line,
            detail: format!(
                "entry {position}: horizon `{}` is not one of now, next or later",
                entry.horizon
            ),
        });
        match (epic, horizon) {
            (Ok(epic), Ok(horizon)) => entries.push(Entry {
                epic,
                horizon,
                rationale: entry.rationale,
            }),
            (epic, horizon) => errors.extend(epic.err().into_iter().chain(horizon.err())),
        }
    }
    if errors.is_empty() {
        Ok(Some(entries))
    } else {
        Err(errors)
    }
}

/// What one block's entries get wrong among themselves, every defect and not only the first.
///
/// An epic named twice, and a horizon nearer than one above it. Entries are compared as given, so
/// a caller that knows two spellings name one artifact passes them in one spelling.
#[must_use]
pub fn defects(entries: &[Entry]) -> Vec<String> {
    let mut defects = Vec::new();
    let mut seen: BTreeMap<&ArtifactId, usize> = BTreeMap::new();
    let mut furthest: Option<(Horizon, &ArtifactId)> = None;
    for (index, entry) in entries.iter().enumerate() {
        let position = index + 1;
        if let Some(first) = seen.get(&entry.epic) {
            defects.push(format!(
                "entry {position} lists {} twice (first at entry {first}); an epic has one place \
                 in the order",
                entry.epic
            ));
        } else {
            seen.insert(&entry.epic, position);
        }
        match furthest {
            Some((horizon, above)) if entry.horizon < horizon => defects.push(format!(
                "entry {position} places {} at `{}` after {above} at `{horizon}`; the horizon never \
                 goes back down the list, because position is the order",
                entry.epic, entry.horizon
            )),
            Some((horizon, _)) if entry.horizon == horizon => {}
            _ => furthest = Some((entry.horizon, &entry.epic)),
        }
    }
    defects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(block: &str) -> String {
        format!("# Roadmap\n\n```roadmap\n{block}\n```\n")
    }

    #[test]
    fn a_body_with_no_block_states_no_entries() {
        assert_eq!(recorded("# Roadmap\n\nprose only\n"), Ok(None));
    }

    #[test]
    fn an_empty_list_is_a_roadmap_that_orders_nothing_yet() {
        assert_eq!(recorded(&body("[]")), Ok(Some(Vec::new())));
    }

    #[test]
    fn entries_are_read_in_order_with_their_horizon_and_rationale() {
        let entries = recorded(&body(
            "- epic: epic:a\n  horizon: now\n  rationale: first\n- epic: epic:b\n  horizon: later",
        ))
        .expect("readable")
        .expect("a block");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].epic.to_string(), "epic:a");
        assert_eq!(entries[0].rationale.as_deref(), Some("first"));
        assert_eq!(entries[1].horizon, Horizon::Later);
        assert_eq!(defects(&entries), Vec::<String>::new());
    }

    #[test]
    fn a_horizon_outside_the_vocabulary_is_refused_and_quoted_at_its_entry() {
        let errors = recorded(&body(
            "- epic: epic:a\n  horizon: now\n- epic: epic:b\n  horizon: soon",
        ))
        .expect_err("refused");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].detail.contains("`soon`"), "{}", errors[0]);
        assert_eq!(errors[0].line, 6, "entry 2 starts on body line 6");
    }

    #[test]
    fn every_unreadable_entry_is_reported() {
        let errors = recorded(&body(
            "- epic: epic:a\n  horizon: soon\n- epic: not an id\n  horizon: now\n\
             - epic: epic:c\n  horizon: later\n  rank: 1",
        ))
        .expect_err("refused");
        let lines: Vec<usize> = errors.iter().map(|error| error.line).collect();
        assert_eq!(lines, [4, 6, 8], "{errors:?}");
        assert!(errors[2].detail.contains("rank"), "{}", errors[2]);
    }

    #[test]
    fn an_unclosed_block_is_refused_at_its_opening_line() {
        let errors = recorded("intro\n```roadmap\n- epic: epic:a\n").expect_err("refused");
        assert_eq!(errors[0].line, 2);
        assert!(errors[0].detail.contains("never closed"));
    }

    #[test]
    fn a_block_quoted_inside_another_fence_is_not_the_roadmap() {
        let quoted = "````markdown\n```roadmap\n- epic: epic:a\n  horizon: now\n```\n````\n";
        assert_eq!(recorded(quoted), Ok(None));
    }

    #[test]
    fn a_tilde_fence_is_read_like_a_backtick_one() {
        let entries = recorded("~~~roadmap\n- {epic: 'epic:a', horizon: now}\n~~~\n")
            .expect("readable")
            .expect("a block");
        assert_eq!(entries[0].epic.to_string(), "epic:a");
    }

    #[test]
    fn a_second_block_is_refused_at_its_opening_line() {
        let text = format!(
            "{}\n```roadmap\n- {{epic: 'epic:b', horizon: next}}\n```\n",
            body("- {epic: 'epic:a', horizon: now}")
        );
        let errors = recorded(&text).expect_err("refused");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 7);
        assert!(errors[0].detail.contains("second"), "{}", errors[0]);
    }

    #[test]
    fn every_defect_of_a_list_is_reported_together() {
        let entries = recorded(&body(
            "- {epic: 'epic:a', horizon: next}\n- {epic: 'epic:a', horizon: next}\n\
             - {epic: 'epic:c', horizon: now}",
        ))
        .expect("readable")
        .expect("a block");
        let found = defects(&entries);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains("twice"));
        assert!(found[1].contains("goes back"));
    }
}
