//! Where `aep plan artifact validate` places a `review-result` by its `findings` block.
//!
//! The specification is `aep.review.ValidateFindings` (`ess/domains/review.yaml`): one standing per
//! review, decided by the first of its outcomes that holds. A review carrying a block is done with.
//! One without is listed, and how it is listed depends on whether the store opted in with
//! `findings_required_since` (`project.yaml`):
//!
//! | store | review with no block | standing | counted |
//! |---|---|---|---|
//! | no `findings_required_since` | any | `not_required` | no; `--strict` refuses it |
//! | `findings_required_since` set | recorded `--prose-only <reason>` | `exempt_prose_only` | no |
//! | | a review-result carrying a block `supersedes` it | `exempt_superseded` | no |
//! | | recorded before midnight UTC of the date | `exempt_before_opt_in` | no |
//! | | none of those | `missing` | a problem, exit 1 |
//!
//! The order of the exemptions is the most explicit first: a reason somebody wrote, then a review
//! that replaced it, then its age. A review that more than one fits is listed under the first.
//!
//! Pure: every fact arrives as an argument, including the instant the opt-in starts at, so the
//! decision reads no clock and no file (invariant *Decisions are deterministic*).

use aep_domain::time::Timestamp;

/// Where `validate` places one review-result, as far as its findings go.
///
/// The variants are the specification's `aep.review.FindingsStanding`, in its order and by its
/// names; a test holds them to the generated schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FindingsStanding {
    /// The body carries a `findings` block, an empty `[]` one included.
    CarriesBlock,
    /// No block, in a store without `findings_required_since`: listed, not counted.
    NotRequired,
    /// No block, recorded with `--prose-only <reason>`: listed with the reason.
    ExemptProseOnly,
    /// No block, and a review-result carrying a block `supersedes` it: listed.
    ExemptSuperseded,
    /// No block, created before the start of `findings_required_since` in UTC: listed.
    ExemptBeforeOptIn,
    /// No block, the store requires one, and no exemption applies: a problem.
    Missing,
}

impl FindingsStanding {
    /// Every standing, in the specification's order.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 6] = [
        Self::CarriesBlock,
        Self::NotRequired,
        Self::ExemptProseOnly,
        Self::ExemptSuperseded,
        Self::ExemptBeforeOptIn,
        Self::Missing,
    ];

    /// The name a report prints, which is the specification's.
    pub(crate) fn wire(self) -> &'static str {
        match self {
            Self::CarriesBlock => "carries_block",
            Self::NotRequired => "not_required",
            Self::ExemptProseOnly => "exempt_prose_only",
            Self::ExemptSuperseded => "exempt_superseded",
            Self::ExemptBeforeOptIn => "exempt_before_opt_in",
            Self::Missing => "missing",
        }
    }
}

/// Why a Git-native store cannot date a review's recording, and so leaves it undated rather than
/// guessing (invariant *Unknown differs from false*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Undated {
    /// The repository is a shallow clone and the commit that added the file is its boundary: the
    /// history that would date it was not fetched.
    ShallowBoundary,
    /// The store is not in a Git work tree that versions it — a `git archive` export, a copied or
    /// ignored directory — so no commit records when anything in it was added.
    OutsideGit,
}

impl Undated {
    /// Why the review is undated, as the listing prints it.
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::ShallowBoundary => {
                "the repository is a shallow clone and the commit that added it is the clone's \
                 boundary, so the history that dates it is not here"
            }
            Self::OutsideGit => {
                "the store is not in a Git work tree that versions it, so no commit records when \
                 it was added"
            }
        }
    }

    /// What dates it, as the problem names it.
    pub(crate) fn remedy(self) -> &'static str {
        match self {
            Self::ShallowBoundary => {
                "fetch the full history (`git fetch --unshallow`, or `fetch-depth: 0` for \
                 actions/checkout) and validate again"
            }
            Self::OutsideGit => {
                "run `aep plan artifact validate` in the repository that versions the store"
            }
        }
    }
}

/// What one review-result is placed on: the facts `validate` reads about it from the store.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReviewFacts {
    /// The body carries a `findings` block (`findings::recorded` answers `Some`).
    pub(crate) findings_block: bool,
    /// The front matter records a `prose_only` reason.
    pub(crate) prose_only: bool,
    /// Another review-result whose body carries a block names this one with `supersedes`.
    pub(crate) superseded_by_block: bool,
    /// When the store recorded the review (`reviews_recorded_at` in `planning.rs`), when it can tell.
    ///
    /// `None` is *not known*, never *long ago*: a review with no recorded creation, in a shallow
    /// clone's boundary commit, or in a store outside Git is not dated before anything (invariant
    /// *Unknown differs from false*).
    pub(crate) created_at: Option<Timestamp>,
}

/// Places one review: the first outcome of `aep.review.ValidateFindings` that holds.
///
/// `required_from` is the instant `findings_required_since` starts at (midnight UTC of its date),
/// or `None` when the store has not opted in. `--strict` is not an input: it refuses every
/// `not_required` review, which the caller counts from the standings.
pub(crate) fn standing(facts: &ReviewFacts, required_from: Option<Timestamp>) -> FindingsStanding {
    if facts.findings_block {
        return FindingsStanding::CarriesBlock;
    }
    let Some(from) = required_from else {
        return FindingsStanding::NotRequired;
    };
    if facts.prose_only {
        FindingsStanding::ExemptProseOnly
    } else if facts.superseded_by_block {
        FindingsStanding::ExemptSuperseded
    } else if facts.created_at.is_some_and(|at| at < from) {
        FindingsStanding::ExemptBeforeOptIn
    } else {
        FindingsStanding::Missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-01T00:00:00Z.
    const MIDNIGHT: u64 = 1_790_812_800_000;

    fn at(millis: u64) -> Timestamp {
        Timestamp::from_epoch_millis(millis)
    }

    #[test]
    fn a_review_carrying_a_block_carries_it_whatever_else_is_true() {
        let facts = ReviewFacts {
            findings_block: true,
            superseded_by_block: true,
            created_at: Some(at(0)),
            ..ReviewFacts::default()
        };
        assert_eq!(standing(&facts, None), FindingsStanding::CarriesBlock);
        assert_eq!(standing(&facts, Some(at(MIDNIGHT))), FindingsStanding::CarriesBlock);
    }

    #[test]
    fn without_an_opt_in_every_review_without_a_block_is_not_required_whatever_its_exemptions() {
        for facts in [
            ReviewFacts::default(),
            ReviewFacts {
                prose_only: true,
                superseded_by_block: true,
                created_at: Some(at(0)),
                ..ReviewFacts::default()
            },
        ] {
            assert_eq!(standing(&facts, None), FindingsStanding::NotRequired, "{facts:?}");
        }
    }

    #[test]
    fn the_most_explicit_exemption_that_holds_places_the_review() {
        let all = ReviewFacts {
            prose_only: true,
            superseded_by_block: true,
            created_at: Some(at(0)),
            ..ReviewFacts::default()
        };
        assert_eq!(standing(&all, Some(at(MIDNIGHT))), FindingsStanding::ExemptProseOnly);
        let superseded_and_old = ReviewFacts {
            prose_only: false,
            ..all.clone()
        };
        assert_eq!(
            standing(&superseded_and_old, Some(at(MIDNIGHT))),
            FindingsStanding::ExemptSuperseded
        );
        let old = ReviewFacts {
            superseded_by_block: false,
            ..superseded_and_old
        };
        assert_eq!(standing(&old, Some(at(MIDNIGHT))), FindingsStanding::ExemptBeforeOptIn);
        assert_eq!(
            standing(&ReviewFacts::default(), Some(at(MIDNIGHT))),
            FindingsStanding::Missing
        );
    }

    #[test]
    fn a_review_created_a_millisecond_before_midnight_utc_is_exempt_and_one_at_midnight_is_not() {
        let before = ReviewFacts {
            created_at: Some(at(MIDNIGHT - 1)),
            ..ReviewFacts::default()
        };
        assert_eq!(
            standing(&before, Some(at(MIDNIGHT))),
            FindingsStanding::ExemptBeforeOptIn
        );
        let on = ReviewFacts {
            created_at: Some(at(MIDNIGHT)),
            ..ReviewFacts::default()
        };
        assert_eq!(standing(&on, Some(at(MIDNIGHT))), FindingsStanding::Missing);
    }

    /// A review the store cannot date — an SQLite or Postgres store whose history holds no
    /// `Created` entry for it — is undated, and undated is not "before" the opt-in.
    #[test]
    fn a_review_with_no_recorded_creation_is_not_dated_before_the_opt_in() {
        let undated = ReviewFacts {
            created_at: None,
            ..ReviewFacts::default()
        };
        assert_eq!(standing(&undated, Some(at(MIDNIGHT))), FindingsStanding::Missing);
    }

    /// The standings are the specification's `aep.review.FindingsStanding`, in its order, by its
    /// names.
    #[test]
    fn findings_standings_are_the_generated_findings_standing_variants_in_order() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../generated/ess/schema/schema/types/aep.review.FindingsStanding.schema.json",
        );
        let schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("the generated schema is committed"),
        )
        .expect("the generated schema is JSON");
        let generated: Vec<&str> = schema["$defs"]["aep.review.FindingsStanding"]["enum"]
            .as_array()
            .expect("an enum")
            .iter()
            .map(|value| value.as_str().expect("a name"))
            .collect();
        let ours: Vec<&str> = FindingsStanding::ALL
            .iter()
            .copied()
            .map(FindingsStanding::wire)
            .collect();
        assert_eq!(ours, generated);
        for standing in FindingsStanding::ALL {
            assert_eq!(
                serde_json::to_value(standing).expect("a standing serialises"),
                serde_json::Value::from(standing.wire()),
                "the serialised name is the printed one"
            );
        }
    }
}
