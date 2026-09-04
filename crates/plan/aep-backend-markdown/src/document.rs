//! One planning file: a frontmatter block, and the markdown nobody interprets.
//!
//! ```markdown
//! ---
//! format: aep.planning-md/1
//! id: story:passkey-login
//! kind: story
//! status: draft
//! title: Passkey login
//! relations:
//!   - derived_from: epic:passwordless
//! revision: 1
//! ---
//! # Passkey login
//!
//! Anything at all. The tooling never reads past the closing fence.
//! ```
//!
//! # The body is bytes
//!
//! [`PlanningDocument::render`] writes the body back exactly as it was read — no reflow, no
//! trailing-newline policy, no heading rewrite. A store that reformats prose is a store whose
//! every status move produces a diff nobody can review, and the review is the reason the plan
//! lives in the repository in the first place.
//!
//! # The fence split is hand-rolled
//!
//! Deliberately, and it is four lines: find `---` on the first line, find the next line that is
//! `---`, hand the middle to `serde_yaml` and keep the rest. The crates that do this
//! (`gray_matter`, `matter`, …) bring a second YAML implementation, a second markdown opinion and
//! a dependency the workspace would have to justify — see `AGENTS.md` § Dependencies, which asks
//! for the refusal to be recorded where the refusal happens. This is that record.

use std::collections::BTreeSet;
use std::fmt;

use aep_domain::artifact::{
    ArtifactId, ArtifactKind, ArtifactLifecycle, ArtifactRef, ArtifactRelation, ArtifactStatus,
    LifecycleRegistry, RelationKind,
};
use aep_domain::error::ValidationErrors;

use crate::frontmatter::{PlanningFrontmatter, RawPlanningFrontmatter};

/// The line that opens and closes a frontmatter block.
const FENCE: &str = "---";

/// A planning document: validated frontmatter, plus the body as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningDocument {
    /// What the tooling reads.
    pub frontmatter: PlanningFrontmatter,
    /// What it does not: everything after the closing fence, byte for byte.
    pub body: String,
}

impl PlanningDocument {
    /// Builds a document from validated frontmatter and a body.
    pub fn new(frontmatter: PlanningFrontmatter, body: impl Into<String>) -> Self {
        Self {
            frontmatter,
            body: body.into(),
        }
    }

    /// Reads one document.
    ///
    /// `origin` appears in error messages only; pass the file path when there is one.
    pub fn parse(text: &str, origin: Option<&str>) -> Result<Self, PlanningDocumentError> {
        let (block, body) =
            split_fences(text).ok_or_else(|| PlanningDocumentError::NoFrontmatter {
                origin: origin.map(ToOwned::to_owned),
            })?;

        let raw: RawPlanningFrontmatter =
            serde_yaml::from_str(block).map_err(|source| PlanningDocumentError::Syntax {
                origin: origin.map(ToOwned::to_owned),
                source,
            })?;
        let frontmatter = PlanningFrontmatter::try_from(raw).map_err(|errors| {
            PlanningDocumentError::Invalid {
                origin: origin.map(ToOwned::to_owned),
                errors,
            }
        })?;

        Ok(Self {
            frontmatter,
            body: body.to_owned(),
        })
    }

    /// Writes the document back out.
    ///
    /// Deterministic: the same document renders to the same bytes every time, and
    /// `parse(&render(&d)) == d`. Both are asserted rather than asserted-to-be-obvious, because a
    /// store whose round trip is lossy corrupts a file on the first status move and nobody notices
    /// until the second.
    pub fn render(&self) -> String {
        let block = serde_yaml::to_string(&self.frontmatter)
            .unwrap_or_else(|error| panic!("validated frontmatter serialises: {error}"));
        format!("{FENCE}\n{block}{FENCE}\n{}", self.body)
    }

    /// Moves the artifact to `to`, or says what it could have moved to instead.
    ///
    /// The lifecycle comes from the document tree, through
    /// [`LifecycleRegistry::for_kind`] — so a kind that declares none inherits its parent's, and a
    /// kind with no lifecycle anywhere in its lineage gets
    /// [`ArtifactLifecycle::permissive`], which permits every move. Permissive is the honest
    /// default: refusing a transition because nobody wrote a ladder for `runbook` would make the
    /// store unusable for the kinds it has no opinion about.
    ///
    /// On success the revision is bumped, because the file on disk is about to change.
    ///
    /// The verdict itself comes from [`crate::kernel`], which evaluates the ladder as data through
    /// `entity-core` rather than by a lookup written here. The answer is the same one
    /// [`ArtifactLifecycle::permits_transition`] gives — `tests/kernel_equivalence.rs` holds that
    /// over every kind in the store and every pair of statuses — and the refusal below is
    /// unchanged, because what an author needs is where they may go instead.
    pub fn move_status(
        &mut self,
        to: ArtifactStatus,
        lifecycles: &LifecycleRegistry,
        evidence: &crate::kernel::EvidenceOnHand,
        now: Option<&str>,
    ) -> Result<(), Box<MoveRefusal>> {
        let permissive = ArtifactLifecycle::permissive();
        let lifecycle = lifecycles
            .for_kind(&self.frontmatter.kind)
            .unwrap_or(&permissive);
        let from = self.frontmatter.status.clone();

        // The dated keys come from this document's own frontmatter, not from the caller: an
        // artifact's `expires_at` is a fact it records, and letting a caller supply one would make
        // the guard something the mover chooses.
        let dates = lifecycle
            .when
            .values()
            .flat_map(aep_domain::artifact::TimeGuard::keys)
            .filter_map(|key| {
                let value = self.frontmatter.extra.get(key)?.as_text()?;
                Some((key.to_owned(), value.to_owned()))
            })
            .collect();
        let on_hand = crate::kernel::OnHand {
            evidence: evidence.clone(),
            now: now.map(str::to_owned),
            dates,
        };

        let reason = match crate::kernel::decide(
            Some(&self.frontmatter.kind),
            lifecycle,
            &from,
            &to,
            &on_hand,
        ) {
            crate::kernel::Verdict::Permitted => None,
            crate::kernel::Verdict::NotOnTheLadder => Some(RefusalReason::NotOnTheLadder),
            crate::kernel::Verdict::Unobservable {
                unobserved,
                message,
            } => Some(RefusalReason::Unobservable {
                unobserved,
                message,
            }),
            crate::kernel::Verdict::NotEarned { message } => {
                Some(RefusalReason::NotEarned { message })
            }
            crate::kernel::Verdict::Undecidable { message } => {
                Some(RefusalReason::LadderUnreadable { message })
            }
        };

        if let Some(reason) = reason {
            let legal = lifecycle
                .transitions
                .get(&from)
                .cloned()
                .unwrap_or_default();
            return Err(Box::new(MoveRefusal {
                kind: self.frontmatter.kind.clone(),
                from,
                to,
                legal,
                reason,
            }));
        }

        self.frontmatter.status = to;
        self.bump();
        Ok(())
    }

    /// Adds an outgoing edge, and says whether anything changed.
    ///
    /// `false` means the document already declared exactly this edge: adding it again would
    /// produce a revision nobody can explain and a diff with nothing in it, so the revision is
    /// left alone. The graph itself is not checked here — whether the target exists and whether
    /// the edge closes a cycle are questions about the *store*, and
    /// [`StoreReport::graph`](crate::store::StoreReport::graph) is what answers them.
    pub fn add_relation(&mut self, kind: RelationKind, target: ArtifactRef) -> bool {
        let relation = ArtifactRelation::new(kind, target);
        if self.frontmatter.declares(&relation) {
            return false;
        }
        self.frontmatter.relations.push(relation);
        self.bump();
        true
    }

    /// Takes one outgoing edge back, and says whether anything changed.
    ///
    /// `false` means the document does not declare this edge, which is a refusal for the caller to
    /// make rather than a no-op to write: a revision spent removing nothing is a revision nobody
    /// can explain, exactly as [`Self::add_relation`] says of adding one twice. Exactly the one
    /// `(kind, target)` pair goes; every other edge, including one written by hand into a document
    /// this crate never authored, stays where it was.
    ///
    /// The pair is `(kind, target id)`, and the pinned version is deliberately not part of it: the
    /// projection's own removal matches on the id alone, and a caller made to type `@v2` to undo an
    /// edge would be typing a second spelling for one edge.
    pub fn remove_relation(&mut self, kind: RelationKind, target: &ArtifactId) -> bool {
        let declares =
            |declared: &ArtifactRelation| declared.kind == kind && declared.target.id() == target;
        if !self.frontmatter.relations.iter().any(declares) {
            return false;
        }
        self.frontmatter.relations.retain(|held| !declares(held));
        self.bump();
        true
    }

    /// Replaces the markdown body, and says whether anything changed.
    ///
    /// The body is opaque UTF-8: no markdown parsing, reflow, heading policy or newline policy is
    /// applied. `false` means the supplied bytes already are the body, so writing them would create
    /// a revision with no corresponding change.
    pub fn replace_body(&mut self, body: impl Into<String>) -> bool {
        let body = body.into();
        if self.body == body {
            return false;
        }
        self.body = body;
        self.bump();
        true
    }

    /// Records that this document has been written again.
    fn bump(&mut self) {
        self.frontmatter.revision = self.frontmatter.revision.saturating_add(1);
    }
}

/// Splits `---` fences off the front of a document.
///
/// Returns the frontmatter block and the body, or `None` when the text does not open with a fence
/// or never closes one. Line endings are tolerated on both sides; the body is whatever follows the
/// newline after the closing fence, unaltered.
fn split_fences(text: &str) -> Option<(&str, &str)> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;

    let mut offset = 0_usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == FENCE {
            return Some((&rest[..offset], &rest[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

/// Why a planning document could not be read.
///
/// Three variants rather than one string, on the split `aep-schema::parse` already draws and for
/// the same reason: a missing fence is a defect a person fixes in one keystroke, a YAML error is a
/// typo on a line the message names, and a validation failure is a document that parses and means
/// something this build refuses. They call for three different reactions.
#[derive(Debug, thiserror::Error)]
pub enum PlanningDocumentError {
    /// The text does not open with a `---` fence, or never closes one.
    #[error(
        "planning document{} has no frontmatter: it must open with a `---` line and close the \
         block with another",
        context(origin.as_deref())
    )]
    NoFrontmatter {
        /// Where the text came from, when known.
        origin: Option<String>,
    },

    /// The frontmatter block is not well-formed YAML, or does not match the document's shape.
    #[error("planning document{}: {source}", context(origin.as_deref()))]
    Syntax {
        /// Where the text came from, when known.
        origin: Option<String>,
        /// The underlying parse error, which carries the line and column.
        source: serde_yaml::Error,
    },

    /// The frontmatter parses but is not valid.
    #[error("planning document{} is not valid: {errors}", context(origin.as_deref()))]
    Invalid {
        /// Where the text came from, when known.
        origin: Option<String>,
        /// Every problem found, not the first.
        errors: ValidationErrors,
    },
}

impl PlanningDocumentError {
    /// The validation errors, when this is a semantic failure.
    pub fn validation_errors(&self) -> Option<&ValidationErrors> {
        match self {
            Self::Invalid { errors, .. } => Some(errors),
            Self::NoFrontmatter { .. } | Self::Syntax { .. } => None,
        }
    }

    /// Where the document came from, when known.
    pub fn origin(&self) -> Option<&str> {
        match self {
            Self::NoFrontmatter { origin }
            | Self::Syntax { origin, .. }
            | Self::Invalid { origin, .. } => origin.as_deref(),
        }
    }
}

/// Renders an optional origin as ` (path)`.
fn context(origin: Option<&str>) -> String {
    match origin {
        Some(origin) => format!(" ({origin})"),
        None => String::new(),
    }
}

/// A status move the kind's lifecycle does not permit, with every move it does.
///
/// The legal set is carried rather than left for the caller to look up, because the refusal a
/// person reads has to answer the question the refusal creates. "`active` is not a legal move" is
/// a dead end; "a story may move to: proposed, archived" is the next thing to type.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MoveRefusal {
    /// The kind whose lifecycle refused.
    pub kind: ArtifactKind,
    /// Where the artifact is.
    pub from: ArtifactStatus,
    /// Where the move would have taken it.
    pub to: ArtifactStatus,
    /// Every status it may move to from here. Empty when the status is terminal.
    pub legal: BTreeSet<ArtifactStatus>,
    /// Why. A rung the ladder does not declare, or a rung whose cost is not met.
    ///
    /// Flattened when serialised, so a reader finds one `reason` key at the top level beside
    /// whatever that reason carries, rather than a `reason` object holding a second `reason`.
    #[serde(flatten)]
    pub reason: RefusalReason,
}

/// Why a move was refused.
///
/// Three, not two, and the split between the last two is gap-register `:39`. *Nobody presented
/// evidence of that kind* and *evidence was presented and there is not enough of it* send an author
/// to different places — one to produce a record, the other to argue about the one that exists.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum RefusalReason {
    /// The ladder declares no such move from here. What every refusal was before requirements
    /// existed, and still the only one a ladder without `requires` can produce.
    NotOnTheLadder,
    /// The rung costs something the rule could not read: no evidence of that kind was presented, or
    /// no instant was supplied to judge a date against.
    Unobservable {
        /// What the requirement needed and could not read.
        unobserved: Vec<String>,
        /// What the requirement said.
        message: String,
    },
    /// Everything the rule reads was there, and the rung is not earned.
    NotEarned {
        /// What the requirement said.
        message: String,
    },
    /// The kernel this build pins cannot read the ladder, so nothing about the move is decidable.
    /// Not a fourth answer about the move — an answer about the *instrument*, which is why it says
    /// what to change (the pin, or the document) rather than which moves are legal.
    LadderUnreadable {
        /// The kernel's own words.
        message: String,
    },
}

impl MoveRefusal {
    /// The legal targets, comma-separated, in status order.
    pub fn legal_targets(&self) -> String {
        self.legal
            .iter()
            .map(aep_domain::ArtifactStatus::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl std::error::Error for MoveRefusal {}

/// `a` or `an`, by first letter.
///
/// A rule about English rather than about lifecycles, and it is here because the kind vocabulary is
/// open: "a obligation" was the first sentence a custom kind produced, and a refusal a person reads
/// should not be the place they notice the vocabulary was widened. Sound, not spelling — this is
/// wrong for `an hour` and for `a unicorn`, and no artifact kind is either.
fn article_for(kind: &str) -> &'static str {
    match kind.chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U') => "an",
        _ => "a",
    }
}

impl fmt::Display for MoveRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.reason {
            RefusalReason::NotOnTheLadder => {
                let article = article_for(self.kind.as_str());
                if self.legal.is_empty() {
                    write!(
                        f,
                        "{article} {} in {} is at the end of its lifecycle and may not move",
                        self.kind, self.from
                    )
                } else {
                    write!(
                        f,
                        "{article} {} may move to: {}",
                        self.kind,
                        self.legal_targets()
                    )
                }
            }
            // Names what to go and produce, rather than reporting a requirement as failed when
            // nobody has been asked for it yet.
            RefusalReason::Unobservable {
                unobserved,
                message,
            } => {
                write!(
                    f,
                    "{} is on the ladder and not yet earned: {message}. ",
                    self.to
                )?;
                // What to type, not where the rule looked. `$args.evidence.test_result` is the
                // rule's address for the count it could not read; the person reading this has a
                // verb, and until 2026-08-30 the address leaked into their terminal verbatim
                // (`ed007513#1138`, `e70b8018 s1#1748`).
                let hints: Vec<String> = unobserved.iter().map(|path| what_to_do(path)).collect();
                f.write_str(&hints.join("; "))
            }
            RefusalReason::NotEarned { message } => {
                write!(
                    f,
                    "{} is on the ladder and not yet earned: {message}",
                    self.to
                )
            }
            RefusalReason::LadderUnreadable { message } => {
                write!(f, "the move to {} cannot be decided: {message}", self.to)
            }
        }
    }
}

/// The next thing to type for one address a rule could not read.
///
/// The addresses are the kernel's (`kernel::definition_for` writes them), so this is the one place
/// that knows both spellings and translates. An address this function has not heard of is printed
/// as it came, because inventing advice for it would be worse than the leak it replaces.
fn what_to_do(unobserved: &str) -> String {
    if let Some(kind) = unobserved.strip_prefix("$args.evidence.") {
        format!(
            "no {kind} record is held for this artifact — `protocol artifact evidence <id> --kind \
             {kind} --source <where it came from>` records one"
        )
    } else if unobserved == "$args.now" {
        "no instant was supplied to judge the dated rung against — pass `--at <iso8601>`".to_owned()
    } else {
        format!("nothing was presented at {unobserved}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_domain::artifact::ArtifactId;

    const STORY: &str = "---\nformat: aep.planning-md/1\nid: story:passkey-login\nkind: story\n\
                         status: draft\ntitle: Passkey login\nrevision: 1\n---\n# Passkey login\n\n\
                         Body text.\n";

    fn document(text: &str) -> PlanningDocument {
        PlanningDocument::parse(text, Some("fixture.md")).expect("the fixture is a valid document")
    }

    fn story_lifecycle() -> LifecycleRegistry {
        let mut registry = LifecycleRegistry::new();
        registry.insert(
            ArtifactKind::Story,
            serde_yaml::from_str(
                "kind: story\ninitial: draft\ntransitions:\n  draft: [proposed, archived]\n  \
                 proposed: [draft, active, rejected]\n  active: [implemented, archived]\n  \
                 implemented: [archived]\n  rejected: [archived]\n  archived: []\n",
            )
            .expect("the fixture lifecycle parses"),
        );
        registry
    }

    #[test]
    fn an_unobservable_rung_names_the_verb_not_the_rules_address() {
        let refusal = MoveRefusal {
            kind: ArtifactKind::Story,
            from: ArtifactStatus::Active,
            to: ArtifactStatus::Implemented,
            legal: BTreeSet::from([ArtifactStatus::Implemented, ArtifactStatus::Archived]),
            reason: RefusalReason::Unobservable {
                unobserved: vec!["$args.evidence.test_result".to_owned()],
                message: "reaching implemented needs at least 1 test_result record(s)".to_owned(),
            },
        };
        let text = refusal.to_string();
        assert!(
            text.contains("`protocol artifact evidence <id> --kind test_result"),
            "the refusal says what to type: {text}"
        );
        assert!(
            !text.contains("$args"),
            "the rule's own address stays inside the kernel: {text}"
        );
    }

    #[test]
    fn an_undated_rung_names_the_flag() {
        assert!(what_to_do("$args.now").contains("--at <iso8601>"));
        // An address this translator has not heard of is printed verbatim, never guessed at.
        assert_eq!(
            what_to_do("$args.something.else"),
            "nothing was presented at $args.something.else"
        );
    }

    #[test]
    fn an_unreadable_ladder_is_a_refusal_about_the_kernel() {
        let refusal = MoveRefusal {
            kind: ArtifactKind::Story,
            from: ArtifactStatus::Draft,
            to: ArtifactStatus::Proposed,
            legal: BTreeSet::new(),
            reason: RefusalReason::LadderUnreadable {
                message: "unknown condition operator 'after'".to_owned(),
            },
        };
        let text = refusal.to_string();
        assert!(
            text.starts_with("the move to proposed cannot be decided"),
            "{text}"
        );
        assert!(
            text.contains("unknown condition operator 'after'"),
            "{text}"
        );
    }

    #[test]
    fn the_body_survives_a_round_trip_byte_for_byte() {
        let parsed = document(STORY);
        assert_eq!(parsed.body, "# Passkey login\n\nBody text.\n");
        assert_eq!(parsed.render(), STORY, "rendering restores the input");
    }

    #[test]
    fn rendering_twice_produces_the_same_bytes() {
        // Determinism, invariant 9. A second rendering that differs would make every `git diff`
        // over the plan noise, which is the thing keeping the plan in the repository buys.
        let parsed = document(STORY);
        let once = parsed.render();
        let twice = PlanningDocument::parse(&once, None)
            .expect("what render writes, parse reads")
            .render();
        assert_eq!(once, twice);
    }

    #[test]
    fn an_unknown_key_is_still_there_after_a_status_move() {
        let text = STORY.replace("revision: 1\n", "revision: 1\nsprint: 42\n");
        let mut parsed = document(&text);
        parsed
            .move_status(
                ArtifactStatus::Proposed,
                &story_lifecycle(),
                &std::collections::BTreeMap::default(),
                None,
            )
            .expect("draft to proposed is a legal story move");
        let rendered = parsed.render();
        assert!(rendered.contains("sprint: 42"), "{rendered}");
        assert!(rendered.contains("status: proposed"), "{rendered}");
        assert!(rendered.contains("revision: 2"), "{rendered}");
    }

    #[test]
    fn replacing_the_body_preserves_frontmatter_and_bumps_one_revision() {
        let mut parsed = document(STORY);
        let before = parsed.frontmatter.clone();

        assert!(parsed.replace_body("# Reframed\n\nNew evidence.\n"));
        assert_eq!(parsed.body, "# Reframed\n\nNew evidence.\n");
        assert_eq!(parsed.frontmatter.id, before.id);
        assert_eq!(parsed.frontmatter.kind, before.kind);
        assert_eq!(parsed.frontmatter.status, before.status);
        assert_eq!(parsed.frontmatter.relations, before.relations);
        assert_eq!(parsed.frontmatter.revision, before.revision + 1);
    }

    #[test]
    fn replacing_a_body_with_the_same_bytes_changes_nothing() {
        let mut parsed = document(STORY);
        let before = parsed.clone();
        assert!(!parsed.replace_body(before.body.clone()));
        assert_eq!(parsed, before);
    }

    #[test]
    fn a_document_with_no_fence_is_refused_by_variant() {
        let error = PlanningDocument::parse("# Just markdown\n", Some("loose.md"))
            .expect_err("a file with no frontmatter is not a planning document");
        assert!(
            matches!(error, PlanningDocumentError::NoFrontmatter { .. }),
            "{error}"
        );
        assert!(error.to_string().contains("loose.md"), "{error}");
    }

    #[test]
    fn a_fence_that_never_closes_is_refused_rather_than_read_to_the_end() {
        // Otherwise the whole file becomes the frontmatter and the failure is a YAML error about
        // prose, which sends the reader to the wrong line.
        let error = PlanningDocument::parse("---\nid: story:x\nkind: story\nstatus: draft\n", None)
            .expect_err("an unterminated block is not a block");
        assert!(
            matches!(error, PlanningDocumentError::NoFrontmatter { .. }),
            "{error}"
        );
    }

    #[test]
    fn an_invalid_document_reports_validation_errors_not_a_syntax_error() {
        let error = PlanningDocument::parse(
            "---\nformat: aep.planning-md/9\nid: story:x\nkind: story\nstatus: draft\n---\n",
            None,
        )
        .expect_err("a version this build cannot read is refused");
        let errors = error
            .validation_errors()
            .expect("a document that parses and is refused fails semantically");
        assert_eq!(errors.len(), 1, "{errors}");
    }

    #[test]
    fn an_illegal_move_names_every_legal_target() {
        let mut parsed = document(STORY);
        let refusal = parsed
            .move_status(
                ArtifactStatus::Implemented,
                &story_lifecycle(),
                &std::collections::BTreeMap::default(),
                None,
            )
            .expect_err("a draft story cannot jump to implemented");

        assert_eq!(refusal.from, ArtifactStatus::Draft);
        assert_eq!(refusal.to, ArtifactStatus::Implemented);
        assert_eq!(
            refusal.legal,
            [ArtifactStatus::Proposed, ArtifactStatus::Archived]
                .into_iter()
                .collect::<BTreeSet<_>>()
        );
        assert_eq!(
            refusal.to_string(),
            "a story may move to: proposed, archived"
        );
        assert_eq!(
            parsed.frontmatter.status,
            ArtifactStatus::Draft,
            "a refused move changes nothing"
        );
        assert_eq!(
            parsed.frontmatter.revision, 1,
            "and does not bump the revision"
        );
    }

    #[test]
    fn a_terminal_status_says_so_rather_than_listing_nothing() {
        let text = STORY.replace("status: draft", "status: archived");
        let mut parsed = document(&text);
        let refusal = parsed
            .move_status(
                ArtifactStatus::Draft,
                &story_lifecycle(),
                &std::collections::BTreeMap::default(),
                None,
            )
            .expect_err("archived is the end of the story ladder");
        assert!(refusal.legal.is_empty());
        assert_eq!(
            refusal.to_string(),
            "a story in archived is at the end of its lifecycle and may not move"
        );
    }

    #[test]
    fn a_kind_with_no_lifecycle_anywhere_may_move_freely() {
        // Permissive is the fallback, and it has to be reached through an empty registry rather
        // than through the story ladder — otherwise this test passes on a lookup that never runs.
        let text = STORY
            .replace("kind: story", "kind: runbook")
            .replace("id: story:passkey-login", "id: runbook:passkey-rollout");
        let mut parsed = document(&text);
        assert!(
            LifecycleRegistry::new()
                .for_kind(&ArtifactKind::Runbook)
                .is_none(),
            "the fixture registry has to hold no runbook lifecycle for this to test the fallback"
        );
        parsed
            .move_status(
                ArtifactStatus::Implemented,
                &LifecycleRegistry::new(),
                &std::collections::BTreeMap::default(),
                None,
            )
            .expect("a kind nobody wrote a ladder for is not blocked by one");
        assert_eq!(parsed.frontmatter.revision, 2);
    }

    #[test]
    fn a_relation_that_is_already_declared_does_not_bump_the_revision() {
        let mut parsed = document(STORY);
        let target = ArtifactRef::unpinned(ArtifactId::new("epic:passwordless").expect("an id"));

        assert!(
            parsed.add_relation(RelationKind::DerivedFrom, target.clone()),
            "the first edge is new"
        );
        assert_eq!(parsed.frontmatter.revision, 2);
        assert!(
            !parsed.add_relation(RelationKind::DerivedFrom, target),
            "the second is the same edge"
        );
        assert_eq!(
            parsed.frontmatter.revision, 2,
            "a write that changes nothing is not a revision"
        );
        assert_eq!(parsed.frontmatter.relations.len(), 1);
    }

    #[test]
    fn a_crlf_document_is_read_and_its_body_kept() {
        let text = "---\r\nid: story:x\r\nkind: story\r\nstatus: draft\r\n---\r\n# Title\r\n";
        let parsed = PlanningDocument::parse(text, None).expect("CRLF is a line ending too");
        assert_eq!(parsed.body, "# Title\r\n");
    }

    /// A refusal that crosses a wire keeps its field names and its reason spellings.
    ///
    /// `MoveRefusal` is `Serialize` so a caller that is not a terminal can render the refusal itself
    /// — grey out the rungs the ladder would not take, offer the ones in `legal`. That makes the
    /// field names and the four `reason` spellings a **published shape**: renaming one is a silent
    /// break in every reader, and the compiler has nothing to say about it. This is what makes the
    /// rename fail here instead.
    #[test]
    fn a_refusal_serialises_with_its_reason_and_every_legal_target() {
        let refusal = MoveRefusal {
            kind: ArtifactKind::Story,
            from: ArtifactStatus::Proposed,
            to: ArtifactStatus::Implemented,
            legal: [ArtifactStatus::Draft, ArtifactStatus::Active]
                .into_iter()
                .collect(),
            reason: RefusalReason::NotOnTheLadder,
        };

        let json = serde_json::to_value(&refusal).expect("a refusal serialises");
        assert_eq!(json["kind"], "story");
        assert_eq!(json["from"], "proposed");
        assert_eq!(json["to"], "implemented");
        assert_eq!(
            json["legal"],
            serde_json::json!(["draft", "active"]),
            "the legal set answers the question the refusal creates, and it arrives in the order \
             the vocabulary declares — `draft` before `active` — rather than alphabetically, so a \
             reader can render the rungs in the order a ladder walks them"
        );
        assert_eq!(
            json["reason"], "not_on_the_ladder",
            "the reason is a flat tag a reader branches on, not a nested object"
        );
    }

    /// Every reason a refusal can give is one a reader can branch on by name.
    #[test]
    fn each_refusal_reason_carries_its_own_tag_and_what_it_read() {
        let unobservable = serde_json::to_value(RefusalReason::Unobservable {
            unobserved: vec!["test_result".to_owned()],
            message: "needs 1 test_result".to_owned(),
        })
        .expect("a reason serialises");
        assert_eq!(unobservable["reason"], "unobservable");
        assert_eq!(
            unobservable["unobserved"],
            serde_json::json!(["test_result"])
        );

        let not_earned = serde_json::to_value(RefusalReason::NotEarned {
            message: "1 of 2 test_result".to_owned(),
        })
        .expect("a reason serialises");
        assert_eq!(not_earned["reason"], "not_earned");
        assert_eq!(not_earned["message"], "1 of 2 test_result");
    }
}
