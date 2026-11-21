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
//! This module reads one block and judges what one block can judge: its shape, its vocabulary, that
//! each entry names an epic, that no epic is named twice, and that the horizon never goes back.
//! Whether the epic *exists* is a question about the store, and `aep plan artifact validate` asks it.

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
    /// The epic it places.
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

/// Why a block could not be read, and the body line it is on.
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

/// The entries a body's block states, `None` for a body with no block, or why the block is not
/// readable.
///
/// An empty list (`[]`) is a roadmap that orders nothing yet, which is what the template seeds.
///
/// # Errors
///
/// A block opened and never closed, YAML that is not a sequence of entries, an entry with a key
/// this format does not have or without a required one, an `epic` that is not an artifact id, or a
/// `horizon` outside the vocabulary.
pub fn recorded(body: &str) -> Result<Option<Vec<Entry>>, RoadmapError> {
    let Some((opened, closed)) = crate::findings::named_fence(body, FENCE_INFO) else {
        return Ok(None);
    };
    let Some(closed) = closed else {
        return Err(RoadmapError {
            line: opened + 1,
            detail: "the block is opened and never closed".to_owned(),
        });
    };
    let lines: Vec<&str> = body.lines().collect();
    let text = lines[opened + 1..closed].join("\n");
    let first_line = opened + 2;
    if text.trim().is_empty() {
        return Err(RoadmapError {
            line: first_line,
            detail: "the block is empty; a roadmap that orders nothing yet writes `[]`".to_owned(),
        });
    }
    let raw: Vec<RawEntry> = serde_yaml::from_str(&text).map_err(|error| RoadmapError {
        line: error
            .location()
            .map_or(first_line, |at| first_line + at.line().saturating_sub(1)),
        detail: error.to_string(),
    })?;
    let mut entries = Vec::with_capacity(raw.len());
    for (index, entry) in raw.into_iter().enumerate() {
        let position = index + 1;
        let epic = ArtifactId::new(&entry.epic).map_err(|error| RoadmapError {
            line: first_line,
            detail: format!(
                "entry {position}: `{}` is not an artifact id: {error}",
                entry.epic
            ),
        })?;
        let horizon = Horizon::parse(&entry.horizon).ok_or_else(|| RoadmapError {
            line: first_line,
            detail: format!(
                "entry {position}: horizon `{}` is not one of now, next or later",
                entry.horizon
            ),
        })?;
        entries.push(Entry {
            epic,
            horizon,
            rationale: entry.rationale,
        });
    }
    Ok(Some(entries))
}

/// What one block's entries get wrong on their own, every defect and not only the first.
///
/// An entry naming something other than an epic, an epic named twice, and a horizon nearer than
/// one above it. Whether each epic exists is not decided here: that needs the store.
#[must_use]
pub fn defects(entries: &[Entry]) -> Vec<String> {
    let mut defects = Vec::new();
    let mut seen: BTreeMap<&ArtifactId, usize> = BTreeMap::new();
    let mut furthest: Option<(Horizon, &ArtifactId)> = None;
    for (index, entry) in entries.iter().enumerate() {
        let position = index + 1;
        if entry.epic.namespace() != "epic" {
            defects.push(format!(
                "entry {position} names {}, which is not an epic; a roadmap orders epics",
                entry.epic
            ));
        }
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
        assert!(defects(&entries).is_empty());
    }

    #[test]
    fn a_horizon_outside_the_vocabulary_is_refused_and_quoted() {
        let error = recorded(&body("- epic: epic:a\n  horizon: soon")).expect_err("refused");
        assert!(error.detail.contains("`soon`"), "{error}");
        assert_eq!(error.line, 4);
    }

    #[test]
    fn a_key_the_format_does_not_have_is_refused() {
        let error =
            recorded(&body("- epic: epic:a\n  horizon: now\n  rank: 1")).expect_err("refused");
        assert!(error.detail.contains("rank"), "{error}");
    }

    #[test]
    fn an_unclosed_block_is_refused_at_its_opening_line() {
        let error = recorded("intro\n```roadmap\n- epic: epic:a\n").expect_err("refused");
        assert_eq!(error.line, 2);
        assert!(error.detail.contains("never closed"));
    }

    #[test]
    fn a_block_quoted_inside_another_fence_is_not_the_roadmap() {
        let quoted = "````markdown\n```roadmap\n- epic: epic:a\n  horizon: now\n```\n````\n";
        assert_eq!(recorded(quoted), Ok(None));
    }

    #[test]
    fn every_defect_of_a_list_is_reported_together() {
        let entries = recorded(&body(
            "- {epic: 'epic:a', horizon: next}\n- {epic: 'epic:a', horizon: next}\n\
             - {epic: 'story:b', horizon: next}\n- {epic: 'epic:c', horizon: now}",
        ))
        .expect("readable")
        .expect("a block");
        let found = defects(&entries);
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[0].contains("twice"));
        assert!(found[1].contains("not an epic"));
        assert!(found[2].contains("goes back"));
    }
}
