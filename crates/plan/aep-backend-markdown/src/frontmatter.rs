//! The YAML block at the top of a planning document.
//!
//! Parse, then validate, as everywhere else in this workspace: [`RawPlanningFrontmatter`]
//! deserializes and [`PlanningFrontmatter`] is what a caller may hold. Validation accumulates, so
//! a block that is wrong about two things reports two errors rather than the first one twice.
//!
//! # The key set, and why it is this short
//!
//! ```yaml
//! format: aep.planning-md/1
//! id: story:passkey-login
//! kind: story
//! status: draft
//! title: Passkey login
//! relations:
//!   - derived_from: epic:passwordless
//! revision: 1
//! ```
//!
//! `id`, `kind` and `status` are required; `title`, `summary`, `owner`, `tags`, `refs`,
//! `relations`, `scope`, `withholds`, `model_digest` and `prose_only` are optional (the last two
//! only on the kinds that carry them); `format` and `revision` default. Everything else a document
//! carries is **kept** — see [`PlanningFrontmatter::extra`] — because a store that silently drops
//! the field somebody's own tooling writes is a store they will stop trusting after the first round
//! trip.
//!
//! What is deliberately absent is a timestamp. See the crate documentation: git carries
//! authorship, and a second answer beside it is one that goes stale.

use std::collections::{BTreeMap, BTreeSet};

use aep_domain::artifact::{
    Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactMetadata, ArtifactRelation,
    ArtifactStatus, ArtifactVersion, ExternalRef, RelationKind, ScopeEntry,
};
use aep_domain::error::{ValidationCode, ValidationError, ValidationErrors};
use aep_domain::evidence::{EvidenceKind, SpecDigest};
use aep_domain::node::Node;

/// The frontmatter format version this build reads and writes.
///
/// Versioned from the first document rather than from the first breaking change: a file with no
/// version in it is a file whose reader has to guess, and the guess is only ever wrong once the
/// format has already moved.
pub const PLANNING_FORMAT: &str = "aep.planning-md/1";

/// The format a tree store's projection is rendered in (`aep.project/3`, design § 3.2).
///
/// The keys are those of [`PLANNING_FORMAT`]. The tag is what differs, so a document says which
/// renderer produced it and `validate` holds it to that render (S5), and a later renderer change
/// is a new tag rather than a silent rewrite of every committed document.
pub const PLANNING_FORMAT_V2: &str = "aep.planning-md/2";

/// The format of a Git-native store's documents (`aep.project/5`, git-native design § 4).
///
/// The keys of [`PLANNING_FORMAT`] plus `transitions`: the document is the authority for its own
/// status history, because there is no journal beside it.
pub const PLANNING_FORMAT_V3: &str = "aep.planning-md/3";

/// The front-matter key a `review-result` records why it has no `findings` block under.
pub const PROSE_ONLY: &str = "prose_only";

/// Which planning format a document is written in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanningFormat {
    /// [`PLANNING_FORMAT`]: a Markdown store's documents, and an `aep.project/2` projection.
    #[default]
    V1,
    /// [`PLANNING_FORMAT_V2`]: an `aep.project/3` tree store's projection.
    V2,
    /// [`PLANNING_FORMAT_V3`]: a Git-native store's documents, carrying their own transitions.
    V3,
}

impl PlanningFormat {
    /// The tag written as a document's `format:`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V1 => PLANNING_FORMAT,
            Self::V2 => PLANNING_FORMAT_V2,
            Self::V3 => PLANNING_FORMAT_V3,
        }
    }

    /// The format a `format:` tag names, or nothing for a tag this build does not read.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            PLANNING_FORMAT => Some(Self::V1),
            PLANNING_FORMAT_V2 => Some(Self::V2),
            PLANNING_FORMAT_V3 => Some(Self::V3),
            _ => None,
        }
    }
}

/// Serde default for [`RawPlanningFrontmatter::format`].
fn default_format() -> String {
    PLANNING_FORMAT.to_owned()
}

/// Serde default for [`RawPlanningFrontmatter::revision`].
///
/// One, not zero: the first written revision of a document is its first revision, and a store
/// whose counter starts below the first write has an off-by-one in every comparison built on it.
fn default_revision() -> u64 {
    1
}

/// The frontmatter of a planning document, as parsed.
///
/// Deliberately permissive about *shape* and strict about nothing: every rule this format has is
/// checked by the [`TryFrom`] into [`PlanningFrontmatter`], so a document with two problems is
/// reported once with both.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct RawPlanningFrontmatter {
    /// The format version. Defaults to [`PLANNING_FORMAT`]; a value other than it or
    /// [`PLANNING_FORMAT_V2`] is refused.
    #[serde(default = "default_format")]
    pub format: String,
    /// The artifact's identifier, such as `story:passkey-login`.
    pub id: ArtifactId,
    /// What kind of artifact it is. Aliases such as `adr` are accepted.
    pub kind: ArtifactKind,
    /// Where its lifecycle has got to.
    pub status: ArtifactStatus,
    /// Its title, for a listing.
    #[serde(default)]
    pub title: Option<String>,
    /// One-line summary.
    #[serde(default)]
    pub summary: Option<String>,
    /// Who owns it.
    #[serde(default)]
    pub owner: Option<String>,
    /// Free-form labels.
    #[serde(default)]
    pub tags: BTreeSet<String>,
    /// Records of the same work elsewhere, as written.
    ///
    /// [`Node`] and not [`ExternalRef`] so a malformed entry is reported beside every other defect
    /// in the document rather than aborting the parse of the whole file — the reason `withholds` is
    /// text here too.
    #[serde(default)]
    pub refs: Vec<Node>,
    /// Its outgoing edges, each a single-entry mapping such as `{derived_from: epic:passwordless}`.
    #[serde(default)]
    pub relations: Vec<ArtifactRelation>,
    /// The surfaces this artifact lands on, as written.
    ///
    /// [`Node`] and not [`ScopeEntry`] for the reason `refs` above is: a malformed entry is
    /// reported beside every other defect in the document rather than aborting the parse of the
    /// whole file.
    #[serde(default)]
    pub scope: Vec<Node>,
    /// The evidence kind this artifact is stopping anybody from producing, as written.
    ///
    /// Text here and an [`EvidenceKind`] after validation, so a misspelling is reported beside
    /// every other defect in the document rather than aborting the parse of the whole file.
    #[serde(default)]
    pub withholds: Option<String>,
    /// The content identity of the compiled model, as text until the kind says it may carry one.
    #[serde(default)]
    pub model_digest: Option<String>,
    /// Why a `review-result` records its findings as prose only, as text until the kind says it
    /// may carry one.
    #[serde(default)]
    pub prose_only: Option<String>,
    /// Which revision of this document this is. Bumped by every mutating operation.
    #[serde(default = "default_revision")]
    pub revision: u64,
    /// The status moves this document has been through, oldest first (`aep.planning-md/3`).
    ///
    /// Left out of the published schema: the schema describes the keys every format shares, and
    /// a Git-native store's transition list is checked by [`PlanningFrontmatter`]'s validation.
    #[serde(default)]
    #[schemars(skip)]
    pub transitions: Vec<crate::journal::Transition>,
    /// Every key this format does not name.
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, Node>,
}

/// The frontmatter of a planning document, validated.
///
/// No `Deserialize`, by invariant *Parse, then validate*: the only way to obtain one is to validate
/// a [`RawPlanningFrontmatter`], so a value of this type is one whose format version is understood
/// and whose revision counts from a real write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningFrontmatter {
    /// The format it is written in: the one it was read in, or [`PlanningFormat::V1`] for a new
    /// one. A tree store's projection sets [`PlanningFormat::V2`].
    pub format: PlanningFormat,
    /// The artifact's identifier.
    pub id: ArtifactId,
    /// What kind of artifact it is.
    pub kind: ArtifactKind,
    /// Where its lifecycle has got to.
    pub status: ArtifactStatus,
    /// Its title.
    pub title: Option<String>,
    /// One-line summary.
    pub summary: Option<String>,
    /// Who owns it.
    pub owner: Option<String>,
    /// Free-form labels.
    pub tags: BTreeSet<String>,
    /// Records of the same work in systems AEP does not own, such as `jira:DEV-630`.
    ///
    /// **The join a body paragraph cannot carry.** A team adopting this already has a tracker, and
    /// a ticket id written into prose is invisible to `aep plan artifact list` and unchecked by
    /// anything. Here it is a field, so it can be filtered on and refused when it is malformed.
    pub refs: BTreeSet<ExternalRef>,
    /// Its outgoing edges.
    pub relations: Vec<ArtifactRelation>,
    /// The surfaces this artifact lands on, each `cited` or `inferred`, ordered by path.
    ///
    /// **What a wave's disjointness claim is derived from.** `aep artifact waves` reads this and
    /// nothing else to decide whether two stories may be implemented at once; a story that
    /// declares none is listed as unassessed and never placed, because an unassessed story reads
    /// exactly like a safe one and that is the defect the field exists against.
    ///
    /// Ordered by path and holding one entry per path, so two renderings of one document are the
    /// same bytes and a computation reading it never has to pick between two claims about one
    /// surface.
    pub scope: Vec<ScopeEntry>,
    /// The evidence kind this artifact is stopping anybody from producing.
    ///
    /// **The join between a blocker and an evidence gate.** A rung asks for a `test_result`; the
    /// job that would produce one cannot mint a read-scope token; so the record of *why the fact
    /// does not exist* is a `credential-blocker` that `blocks` the work and names `test_result`
    /// here. `aep plan artifact explain` reads it, which is what makes the missing record
    /// answerable out of the store instead of out of somebody's memory.
    ///
    /// Only meaningful beside a `blocks` edge, and graph validation says so.
    pub withholds: Option<EvidenceKind>,
    /// The digest of the compiled model this document is at.
    ///
    /// Only on a kind [`ArtifactKind::carries_model_digest`] says has one. It is what binds a
    /// conformance run to this revision of the specification: `ess-conformance` counts a run only
    /// when its `spec_digest` is this value, and fails closed when there is none.
    pub model_digest: Option<SpecDigest>,
    /// Why this `review-result` records its findings as prose only, with no `findings` block.
    ///
    /// Written once, by `aep plan artifact new --prose-only <reason>`, and only on a
    /// `review-result`: it is the recorded way a review was settled in a store that requires a
    /// block (`findings_required_since`), so `validate` lists it as exempt rather than counting it.
    /// Never blank — a reason of nothing is no reason.
    pub prose_only: Option<String>,
    /// Which revision of this document this is.
    pub revision: u64,
    /// Every status move the document has been through, oldest first.
    ///
    /// Written by a Git-native store (`aep.planning-md/3`), where the document is the authority for
    /// its own history. Empty — and not rendered — for every document a journal store writes.
    pub transitions: Vec<crate::journal::Transition>,
    /// Every key this format does not name, kept so a round trip loses nothing.
    ///
    /// A planning file is a file people edit. Somebody's board tool will write `sprint: 42` into
    /// one, and the first time `aep plan artifact move` rewrites the file that key has to still be
    /// there — otherwise the tool that wrote it and this one cannot both be used, and this one is
    /// the one that gets deleted.
    pub extra: BTreeMap<String, Node>,
}

impl PlanningFrontmatter {
    /// The format version this frontmatter is written in, as its tag.
    pub fn format(&self) -> &'static str {
        self.format.as_str()
    }

    /// A minimal frontmatter, at revision 1, in [`PLANNING_FORMAT`].
    pub fn new(id: ArtifactId, kind: ArtifactKind, status: ArtifactStatus) -> Self {
        Self {
            format: PlanningFormat::V1,
            id,
            kind,
            status,
            title: None,
            summary: None,
            owner: None,
            tags: BTreeSet::new(),
            refs: BTreeSet::new(),
            relations: Vec::new(),
            scope: Vec::new(),
            withholds: None,
            model_digest: None,
            prose_only: None,
            revision: default_revision(),
            transitions: Vec::new(),
            extra: BTreeMap::new(),
        }
    }

    /// The frontmatter rendered as YAML, with `transitions` one flow mapping per line.
    ///
    /// The [`serde::Serialize`] rendering writes every key in block style; this is what a
    /// document's file carries, so a move's diff is one changed `status:` line, one changed
    /// `revision:` line and one added transition line (git-native design § 4.2). Without
    /// transitions it is exactly the `Serialize` rendering, byte for byte.
    ///
    /// # Panics
    ///
    /// Never for a validated frontmatter: every value it holds serialises.
    #[must_use]
    pub fn render_yaml(&self) -> String {
        fn yaml(value: &impl serde::Serialize) -> String {
            serde_yaml::to_string(value)
                .unwrap_or_else(|error| panic!("validated frontmatter serialises: {error}"))
        }
        if self.transitions.is_empty() {
            return yaml(self);
        }
        let mut head = self.clone();
        head.transitions = Vec::new();
        head.extra = BTreeMap::new();
        let mut rendered = yaml(&head);
        rendered.push_str("transitions:\n");
        for transition in &self.transitions {
            rendered.push_str("- ");
            rendered.push_str(&transition.flow());
            rendered.push('\n');
        }
        if !self.extra.is_empty() {
            rendered.push_str(&yaml(&self.extra));
        }
        rendered
    }

    /// Sets the title, builder-style.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Adds a relation, builder-style.
    #[must_use]
    pub fn with_relation(mut self, relation: ArtifactRelation) -> Self {
        self.relations.push(relation);
        self
    }

    /// `true` when this frontmatter already declares `relation`.
    pub fn declares(&self, relation: &ArtifactRelation) -> bool {
        self.relations.contains(relation)
    }

    /// Every target of `kind`.
    pub fn targets(&self, kind: RelationKind) -> impl Iterator<Item = &ArtifactRelation> {
        self.relations
            .iter()
            .filter(move |relation| relation.kind == kind)
    }

    /// The domain artifact this document describes, located at `relative_path`.
    ///
    /// The mapping is the whole point of this crate: what the protocol reasons about is an
    /// [`Artifact`] in an [`ArtifactGraph`](aep_domain::artifact::ArtifactGraph), and the file
    /// format is how one is written down here.
    ///
    /// Two things are worth saying out loud about what comes out:
    ///
    /// * the location is a [`ArtifactLocation::RepositoryPath`] with no `repository`, because the
    ///   document is a file in *this* repository and naming one would claim it is somewhere else;
    /// * there is **no provenance**, so no `created_at`. Git holds authorship; see the crate
    ///   documentation for why a second copy of it is worse than none.
    pub fn to_artifact(&self, relative_path: &str) -> Artifact {
        let mut artifact = Artifact::new(
            self.id.clone(),
            self.kind.clone(),
            self.status.clone(),
            ArtifactLocation::RepositoryPath {
                repository: None,
                path: relative_path.to_owned(),
            },
        );
        // The document revision is the version this record describes: it moves with every write,
        // which is exactly what a version label is for here. It is not a digest and does not
        // pretend to be one — `model_digest` stays empty, because a plan item has no compiled
        // model to hash.
        artifact.version = Some(ArtifactVersion::new(self.revision.to_string()));
        artifact.relations.clone_from(&self.relations);
        artifact.withholds = self.withholds;
        // The one field here that *is* a digest, on the one kind that has a compiled model to
        // hash. `version` above stays the document revision for every kind.
        artifact.model_digest.clone_from(&self.model_digest);
        artifact.metadata = ArtifactMetadata {
            title: self.title.clone(),
            summary: self.summary.clone(),
            owner: self.owner.clone(),
            tags: self.tags.clone(),
            refs: self.refs.clone(),
            extra: self.extra.clone(),
        };
        artifact
    }
}

/// Written by hand rather than derived, for two reasons that both come down to bytes.
///
/// `format` is a constant, and a struct field holding it would be a second place for it to be
/// wrong. And the **key order is the file's key order**: a derived implementation orders by field
/// declaration, which is the same thing, but a reader of this type would have to know that to
/// trust it. Here the order is written down where the rendering is, which is what makes two
/// renderings of one document byte-identical and a round trip a comparison rather than a hope.
impl serde::Serialize for PlanningFrontmatter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;

        let length = 5
            + usize::from(self.title.is_some())
            + usize::from(self.summary.is_some())
            + usize::from(self.owner.is_some())
            + usize::from(!self.tags.is_empty())
            + usize::from(!self.refs.is_empty())
            + usize::from(!self.relations.is_empty())
            + usize::from(!self.scope.is_empty())
            + usize::from(self.withholds.is_some())
            + usize::from(self.model_digest.is_some())
            + usize::from(self.prose_only.is_some())
            + usize::from(!self.transitions.is_empty())
            + self.extra.len();

        let mut map = serializer.serialize_map(Some(length))?;
        map.serialize_entry("format", self.format.as_str())?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("kind", &self.kind)?;
        map.serialize_entry("status", &self.status)?;
        if let Some(title) = &self.title {
            map.serialize_entry("title", title)?;
        }
        if let Some(summary) = &self.summary {
            map.serialize_entry("summary", summary)?;
        }
        if let Some(owner) = &self.owner {
            map.serialize_entry("owner", owner)?;
        }
        if !self.tags.is_empty() {
            map.serialize_entry("tags", &self.tags)?;
        }
        if !self.refs.is_empty() {
            map.serialize_entry("refs", &self.refs)?;
        }
        if !self.relations.is_empty() {
            map.serialize_entry("relations", &self.relations)?;
        }
        if !self.scope.is_empty() {
            map.serialize_entry("scope", &self.scope)?;
        }
        if let Some(withholds) = &self.withholds {
            map.serialize_entry("withholds", withholds.as_str())?;
        }
        if let Some(digest) = &self.model_digest {
            map.serialize_entry("model_digest", digest.as_str())?;
        }
        if let Some(reason) = &self.prose_only {
            map.serialize_entry(PROSE_ONLY, reason)?;
        }
        map.serialize_entry("revision", &self.revision)?;
        if !self.transitions.is_empty() {
            map.serialize_entry("transitions", &self.transitions)?;
        }
        // Last, and in `BTreeMap` order: an unrecognised key keeps its value and loses only its
        // position, which is the most a reader that does not know what it means can promise.
        for (key, value) in &self.extra {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl TryFrom<RawPlanningFrontmatter> for PlanningFrontmatter {
    type Error = ValidationErrors;

    fn try_from(raw: RawPlanningFrontmatter) -> Result<Self, Self::Error> {
        let mut errors = ValidationErrors::new();

        let format = PlanningFormat::from_tag(&raw.format);
        if format.is_none() {
            errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedFormatVersion,
                    "planning.format",
                    format!(
                        "this build reads planning documents written as `{PLANNING_FORMAT}`, \
                         `{PLANNING_FORMAT_V2}` or `{PLANNING_FORMAT_V3}`, not `{}`",
                        raw.format
                    ),
                )
                .with_hint(
                    "upgrade the tooling rather than reinterpreting the document: a reader that \
                     guesses at an unknown version writes back a file it has already lost part of",
                ),
            );
        }

        if raw.revision == 0 {
            errors.push(
                ValidationError::new(
                    ValidationCode::TypeMismatch,
                    "planning.revision",
                    "`revision: 0` names a state before the document was written",
                )
                .with_hint("revisions count from 1; omit the key to get it"),
            );
        }

        // Refused by name, not defaulted and not carried through as text: a withheld kind
        // nothing recognises would read to a person as a fact the engine is tracking, and the
        // engine tracks only the kinds it knows the semantics of.
        let withholds = match raw.withholds.as_deref() {
            None => None,
            Some(value) => match EvidenceKind::parse(value) {
                Ok(kind) => Some(kind),
                Err(error) => {
                    errors.push(
                        ValidationError::new(
                            ValidationCode::UndeclaredEvidenceKind,
                            "planning.withholds",
                            format!("`withholds: {value}` names no evidence kind: {error}"),
                        )
                        .with_hint(
                            "`withholds` names the kind of proof this artifact is stopping, and \
                             the engine's evidence vocabulary is closed on purpose: an invented \
                             kind would be the sort of proof a gate is asking for, named by \
                             whoever is trying to get past it",
                        ),
                    );
                    None
                }
            },
        };

        // Every entry is attempted, so a document with two malformed references reports both.
        let mut refs = BTreeSet::new();
        for (index, node) in raw.refs.iter().enumerate() {
            match ExternalRef::from_node(node) {
                Ok(reference) => {
                    refs.insert(reference);
                }
                Err(error) => errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("planning.refs[{index}]"),
                        error.to_string(),
                    )
                    .with_hint(
                        "a reference is `{provider: jira, reference: DEV-630}`, or the shorthand \
                         `jira:DEV-630`",
                    ),
                ),
            }
        }

        let scope = validated_scope(&raw.scope, &mut errors);

        let model_digest = validated_model_digest(&raw, &mut errors);
        let prose_only = validated_prose_only(&raw, &mut errors);

        validate_transitions(&raw, &mut errors);

        errors.into_result(Self {
            format: format.unwrap_or_default(),
            id: raw.id,
            kind: raw.kind,
            status: raw.status,
            title: raw.title,
            summary: raw.summary,
            owner: raw.owner,
            tags: raw.tags,
            refs,
            relations: raw.relations,
            scope,
            withholds,
            model_digest,
            prose_only,
            revision: raw.revision,
            transitions: raw.transitions,
            extra: raw.extra,
        })
    }
}

/// Holds a transition list to the document it is in (git-native design § 4.3).
///
/// Two rules that need nothing but the document: `status` is the last transition's `to`, and
/// `revision` counts at least one write per transition beyond the first. That a transition list
/// is a legal walk of the kind's lifecycle needs the lifecycle, and is not decided here.
fn validate_transitions(raw: &RawPlanningFrontmatter, errors: &mut ValidationErrors) {
    let Some(last) = raw.transitions.last() else {
        return;
    };
    if last.to != raw.status {
        errors.push(
            ValidationError::new(
                ValidationCode::TypeMismatch,
                "planning.transitions",
                format!(
                    "`status: {}` and the last transition, which moved to `{}`, disagree",
                    raw.status, last.to
                ),
            )
            .with_hint(
                "a status is changed by a move, which appends the transition that says so; \
                 re-apply the move through the CLI rather than editing either line",
            ),
        );
    }
    let written = u64::try_from(raw.transitions.len()).unwrap_or(u64::MAX);
    if raw.revision <= written {
        errors.push(
            ValidationError::new(
                ValidationCode::TypeMismatch,
                "planning.revision",
                format!(
                    "`revision: {}` is below the {written} transition(s) plus the first write \
                     the document records",
                    raw.revision
                ),
            )
            .with_hint("every move is a write, so a revision is at least one more than the moves"),
        );
    }
}

/// The model digest a document declares, validated, with every defect accumulated into `errors`.
///
/// Lifted out of [`TryFrom`] beside [`validated_scope`], and for the same reason. A digest only
/// where there is a compiled model to be the digest of: on any other kind the key is refused
/// rather than kept as text, because a reader would take it for a binding and the engine's
/// revision check would then be looking at a value nothing computed.
fn validated_model_digest(
    raw: &RawPlanningFrontmatter,
    errors: &mut ValidationErrors,
) -> Option<SpecDigest> {
    match raw.model_digest.as_deref() {
        None => None,
        Some(value) if !raw.kind.carries_model_digest() => {
            errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedConstruct,
                    "planning.model_digest",
                    format!(
                        "`{}` has no compiled model, so `model_digest: {value}` binds nothing",
                        raw.kind.as_str()
                    ),
                )
                .with_hint(
                    "only an executable-system-specification carries a model digest; drop the \
                     key, or file the artifact as that kind",
                ),
            );
            None
        }
        Some(value) => match SpecDigest::new(value) {
            Ok(digest) => Some(digest),
            Err(error) => {
                errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        "planning.model_digest",
                        format!("`model_digest: {value}` is not a digest: {error}"),
                    )
                    .with_hint(
                        "copy the model digest `ess compile` prints, whole; a run is bound to \
                         it byte for byte",
                    ),
                );
                None
            }
        },
    }
}

/// The prose-only reason a document declares, validated, with every defect accumulated into
/// `errors`.
///
/// Gated by kind the way [`validated_model_digest`] is: only a `review-result` has findings to
/// record, so on any other kind the key excuses nothing and is refused rather than kept as text a
/// reader would take for an exemption. A reason of only whitespace is refused too: it is the
/// exemption `validate` grants, and an exemption on the strength of nothing is the defect it
/// exists against.
fn validated_prose_only(
    raw: &RawPlanningFrontmatter,
    errors: &mut ValidationErrors,
) -> Option<String> {
    let value = raw.prose_only.as_deref()?;
    if raw.kind != ArtifactKind::ReviewResult {
        errors.push(
            ValidationError::new(
                ValidationCode::UnsupportedConstruct,
                format!("planning.{PROSE_ONLY}"),
                format!(
                    "`{}` records no findings, so `{PROSE_ONLY}` excuses nothing; only a \
                     `review-result` carries one",
                    raw.kind.as_str()
                ),
            )
            .with_hint("drop the key, or file the artifact as a review-result"),
        );
        return None;
    }
    if value.trim().is_empty() {
        errors.push(
            ValidationError::new(
                ValidationCode::TypeMismatch,
                format!("planning.{PROSE_ONLY}"),
                format!("`{PROSE_ONLY}` is blank, and a reason of nothing is no reason"),
            )
            .with_hint("say why this review records its findings as prose only"),
        );
        return None;
    }
    Some(value.to_owned())
}

/// The scope entries a document declares, validated, with every defect accumulated into `errors`.
///
/// Lifted out of [`TryFrom`] so that block reads as the list of checks it is. Every entry is
/// attempted, so a document wrong about two surfaces reports both, and a path declared twice is
/// refused rather than resolved: the document would be saying two things about one surface, and a
/// computation reading it would have to pick one.
fn validated_scope(raw: &[Node], errors: &mut ValidationErrors) -> Vec<ScopeEntry> {
    let mut scope: Vec<ScopeEntry> = Vec::new();
    for (index, node) in raw.iter().enumerate() {
        match ScopeEntry::from_node(node) {
            Ok(entry) => {
                if scope.iter().any(|held| held.path == entry.path) {
                    errors.push(
                        ValidationError::new(
                            ValidationCode::TypeMismatch,
                            format!("planning.scope[{index}]"),
                            format!(
                                "`{}` is declared twice, and the two entries may disagree about \
                                 how well it is known",
                                entry.path
                            ),
                        )
                        .with_hint(
                            "one entry per path; `aep artifact scope <id> --add <path>` replaces \
                             the entry a path already has rather than adding a second",
                        ),
                    );
                } else {
                    scope.push(entry);
                }
            }
            Err(error) => errors.push(
                ValidationError::new(
                    ValidationCode::TypeMismatch,
                    format!("planning.scope[{index}]"),
                    error.to_string(),
                )
                .with_hint(
                    "a scope entry is `{path: crates/x/src/lib.rs, confidence: cited}`, or the \
                     bare path, which is `cited`",
                ),
            ),
        }
    }
    // Ordered by path, which is what makes two renderings of one document the same bytes.
    scope.sort_by(|left, right| left.path.cmp(&right.path));
    scope
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(text: &str) -> RawPlanningFrontmatter {
        serde_yaml::from_str(text).expect("the fixture parses as YAML")
    }

    const MINIMAL: &str = "id: story:passkey-login\nkind: story\nstatus: draft\n";

    #[test]
    fn a_withheld_evidence_kind_survives_a_round_trip_and_a_misspelling_does_not() {
        // The join a blocker makes to an evidence gate is only worth writing down if it comes back
        // out of the file unchanged — this key is written by a command and read by `explain`.
        let front =
            PlanningFrontmatter::try_from(raw(&format!("{MINIMAL}withholds: test_result\n")))
                .expect("a known evidence kind is accepted");
        assert_eq!(front.withholds, Some(EvidenceKind::TestResult));
        assert_eq!(
            front.to_artifact("story/passkey-login.md").withholds,
            Some(EvidenceKind::TestResult),
            "and it reaches the artifact the graph validates"
        );

        let written = serde_yaml::to_string(&front).expect("the frontmatter renders");
        assert!(written.contains("withholds: test_result"), "{written}");
        let again = PlanningFrontmatter::try_from(raw(&written)).expect("the rendering re-reads");
        assert_eq!(again.withholds, front.withholds);
        assert!(
            !again.extra.contains_key("withholds"),
            "a named key must not also land in `extra`: {:?}",
            again.extra
        );

        // A spelling outside the engine's closed vocabulary is refused by name, and refused
        // *here*, so the message names the key rather than the whole document.
        let errors =
            PlanningFrontmatter::try_from(raw(&format!("{MINIMAL}withholds: green_build\n")))
                .expect_err("an invented evidence kind is refused");
        assert_eq!(
            errors
                .as_slice()
                .iter()
                .map(|error| error.code)
                .collect::<Vec<_>>(),
            vec![ValidationCode::UndeclaredEvidenceKind],
            "{errors}"
        );
    }

    #[test]
    fn a_model_digest_is_kept_on_a_compiled_kind_and_refused_by_name_on_every_other() {
        // The key exists so a conformance report can be tied to the exact model it ran against.
        // That tie is only a tie where there is a model: `carries_model_digest` decides which
        // kinds have one, and this asserts both sides of that predicate rather than the branch
        // that happens to be exercised elsewhere.
        const DIGEST: &str = "8aee51b644a97580e2603ea3c9f57d22ca24d765643f2e0a4e0e6410dbfd1fef";
        let front = PlanningFrontmatter::try_from(raw(&format!(
            "id: executable-system-specification:acd-v3\n\
             kind: executable-system-specification\n\
             status: draft\n\
             model_digest: {DIGEST}\n"
        )))
        .expect("a compiled kind carries a digest");
        assert_eq!(
            front.model_digest.as_ref().map(SpecDigest::as_str),
            Some(DIGEST)
        );
        assert_eq!(
            front
                .to_artifact("executable-system-specification/acd-v3.md")
                .model_digest
                .as_ref()
                .map(SpecDigest::as_str),
            Some(DIGEST),
            "and it reaches the artifact the graph validates"
        );

        // Round-trip, because this key is written by `aep artifact set --model-digest` and read
        // back by the next command that touches the file.
        let written = serde_yaml::to_string(&front).expect("the frontmatter renders");
        assert!(
            written.contains(&format!("model_digest: {DIGEST}")),
            "{written}"
        );
        let again = PlanningFrontmatter::try_from(raw(&written)).expect("the rendering re-reads");
        assert_eq!(again.model_digest, front.model_digest);
        assert!(
            !again.extra.contains_key("model_digest"),
            "a named key must not also land in `extra`: {:?}",
            again.extra
        );

        // On a story it binds nothing, so it is refused here rather than kept as text a reader
        // would take for a guarantee.
        let errors =
            PlanningFrontmatter::try_from(raw(&format!("{MINIMAL}model_digest: {DIGEST}\n")))
                .expect_err("a kind with no compiled model is refused");
        assert_eq!(
            errors
                .as_slice()
                .iter()
                .map(|error| error.code)
                .collect::<Vec<_>>(),
            vec![ValidationCode::UnsupportedConstruct],
            "{errors}"
        );
        assert!(
            errors.to_string().contains("binds nothing"),
            "the refusal says why, not just that: {errors}"
        );
    }

    #[test]
    fn a_prose_only_reason_is_kept_on_a_review_result_and_refused_by_name_on_every_other_kind() {
        // `prose_only` is why a review records its findings as prose: what `validate` exempts it
        // on in a store that requires a block. It is a reason only where there are findings to
        // record — a review-result — and only when it says something.
        const REVIEW: &str = "id: review-result:attack\nkind: review-result\nstatus: active\n";
        let front = PlanningFrontmatter::try_from(raw(&format!(
            "{REVIEW}prose_only: the reviewer's tool writes no block yet\n"
        )))
        .expect("a review-result carries a prose-only reason");
        assert_eq!(
            front.prose_only.as_deref(),
            Some("the reviewer's tool writes no block yet")
        );

        // Round-trip, because the key is written once, by `new`, and read by every `validate`.
        let written = serde_yaml::to_string(&front).expect("the frontmatter renders");
        assert!(
            written.contains("prose_only: the reviewer's tool writes no block yet"),
            "{written}"
        );
        let again = PlanningFrontmatter::try_from(raw(&written)).expect("the rendering re-reads");
        assert_eq!(again.prose_only, front.prose_only);
        assert!(
            !again.extra.contains_key("prose_only"),
            "a named key must not also land in `extra`: {:?}",
            again.extra
        );

        // On a story it excuses nothing, so it is refused rather than kept as text.
        let errors =
            PlanningFrontmatter::try_from(raw(&format!("{MINIMAL}prose_only: no block\n")))
                .expect_err("a kind that carries no findings is refused");
        let refusal = errors
            .as_slice()
            .iter()
            .find(|error| error.location == "planning.prose_only")
            .unwrap_or_else(|| panic!("no prose_only refusal: {errors}"));
        assert_eq!(refusal.code, ValidationCode::UnsupportedConstruct);
        assert!(
            refusal.message.contains("review-result") && refusal.message.contains("story"),
            "the refusal names the kind that carries one and the kind that does not: {errors}"
        );

        // A reason of nothing but whitespace is no reason.
        for blank in ["\"\"", "\"  \\t \""] {
            let errors =
                PlanningFrontmatter::try_from(raw(&format!("{REVIEW}prose_only: {blank}\n")))
                    .expect_err("a blank reason is refused");
            let refusal = errors
                .as_slice()
                .iter()
                .find(|error| error.location == "planning.prose_only")
                .unwrap_or_else(|| panic!("no prose_only refusal for {blank}: {errors}"));
            assert_eq!(refusal.code, ValidationCode::TypeMismatch, "{blank}");
        }
    }

    #[test]
    fn both_written_forms_of_a_reference_read_and_only_the_mapping_is_written_back() {
        // The shorthand exists because it is what a person types and what `--ref` takes; the
        // mapping is what the file carries, so a document written by hand and one written by the
        // CLI are the same bytes after one write.
        let front = PlanningFrontmatter::try_from(raw(&format!(
            "{MINIMAL}refs:\n  - jira:DEV-630\n  - provider: zendesk\n    reference: \"8812\"\n"
        )))
        .expect("both forms are accepted");
        assert_eq!(
            front
                .refs
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["jira:DEV-630".to_owned(), "zendesk:8812".to_owned()],
            "a set, so the order is the provider's and not the file's"
        );
        assert_eq!(
            front.to_artifact("story/passkey-login.md").metadata.refs,
            front.refs,
            "and they reach the artifact the graph validates"
        );

        let written = serde_yaml::to_string(&front).expect("the frontmatter renders");
        assert!(written.contains("provider: jira"), "{written}");
        assert!(
            !written.contains("jira:DEV-630"),
            "the shorthand is an input form, not an output one: {written}"
        );
        let again = PlanningFrontmatter::try_from(raw(&written)).expect("the rendering re-reads");
        assert_eq!(again.refs, front.refs);
        assert!(
            !again.extra.contains_key("refs"),
            "a named key must not also land in `extra`: {:?}",
            again.extra
        );
    }

    #[test]
    fn every_malformed_reference_is_reported_and_not_just_the_first() {
        // Invariant *Validation accumulates* again, and the case it matters for: a migration writes
        // twenty of these at once, and a validator that stopped at the first would be run twenty
        // times.
        let errors = PlanningFrontmatter::try_from(raw(&format!(
            "{MINIMAL}refs:\n  - DEV-630\n  - jira:has a space\n  - provider: jira\n"
        )))
        .expect_err(
            "a key with no provider, a key with whitespace and a mapping with no \
                     reference are all refused",
        );
        assert_eq!(errors.len(), 3, "{errors}");
        assert!(
            errors
                .as_slice()
                .iter()
                .all(|error| error.location.starts_with("planning.refs[")),
            "each names the entry it came from: {errors}"
        );
    }

    #[test]
    fn a_document_without_a_format_key_is_read_as_the_current_format() {
        // The default exists so the first hand-written file does not need a header nobody would
        // think to write. It has to be the *current* version, not "unknown".
        let frontmatter =
            PlanningFrontmatter::try_from(raw(MINIMAL)).expect("the minimal document is valid");
        assert_eq!(frontmatter.format(), PLANNING_FORMAT);
        assert_eq!(
            frontmatter.revision, 1,
            "the revision defaults to the first"
        );
    }

    #[test]
    fn a_document_in_the_tree_projection_format_is_read_and_written_back_in_it() {
        // `aep.planning-md/2` is what a tree store's projection is rendered in (design § 3.2). A
        // reader that took it and wrote `/1` back would change every document it round-tripped.
        let front =
            PlanningFrontmatter::try_from(raw(&format!("format: aep.planning-md/2\n{MINIMAL}")))
                .expect("the tree projection format is one this build reads");
        assert_eq!(front.format(), "aep.planning-md/2");
        let written = serde_yaml::to_string(&front).expect("serializes");
        assert!(
            written.starts_with("format: aep.planning-md/2\n"),
            "a /2 document is written back as /2:\n{written}"
        );
        let first = PlanningFrontmatter::try_from(raw(MINIMAL)).expect("valid");
        assert!(
            serde_yaml::to_string(&first)
                .expect("serializes")
                .starts_with("format: aep.planning-md/1\n"),
            "a /1 document stays /1"
        );
    }

    #[test]
    fn a_format_version_this_build_does_not_read_is_refused_by_code() {
        let errors =
            PlanningFrontmatter::try_from(raw(&format!("format: aep.planning-md/99\n{MINIMAL}")))
                .expect_err("a version this build cannot read is not silently reinterpreted");
        assert_eq!(errors.len(), 1, "{errors}");
        assert!(
            errors.contains(ValidationCode::UnsupportedFormatVersion),
            "{errors}"
        );
    }

    #[test]
    fn a_document_wrong_about_two_things_reports_both() {
        // Invariant *Validation accumulates*. An exact count, because "is an error" would pass
        // with a validator that returned on the first problem.
        let errors = PlanningFrontmatter::try_from(raw(&format!(
            "format: aep.planning-md/99\nrevision: 0\n{MINIMAL}"
        )))
        .expect_err("both problems are problems");
        assert_eq!(errors.len(), 2, "{errors}");
        assert!(
            errors.contains(ValidationCode::UnsupportedFormatVersion),
            "{errors}"
        );
        assert!(errors.contains(ValidationCode::TypeMismatch), "{errors}");
    }

    #[test]
    fn revision_zero_is_refused_because_no_write_produced_it() {
        let errors = PlanningFrontmatter::try_from(raw(&format!("revision: 0\n{MINIMAL}")))
            .expect_err("a revision below the first write is not a revision");
        assert_eq!(errors.len(), 1, "{errors}");
        assert_eq!(errors.as_slice()[0].location, "planning.revision");
    }

    #[test]
    fn a_kind_alias_is_accepted_and_canonicalised() {
        let frontmatter =
            PlanningFrontmatter::try_from(raw("id: adr:passkeys\nkind: adr\nstatus: proposed\n"))
                .expect("`adr` is an accepted spelling of the kind");
        assert_eq!(frontmatter.kind, ArtifactKind::ArchitectureDecisionRecord);
    }

    #[test]
    fn an_unknown_key_survives_validation() {
        let frontmatter =
            PlanningFrontmatter::try_from(raw(&format!("{MINIMAL}sprint: 42\n"))).expect("valid");
        assert_eq!(
            frontmatter.extra.get("sprint"),
            Some(&Node::Number(42_i64.into())),
            "an unrecognised key is carried, not dropped"
        );
    }

    #[test]
    fn the_artifact_it_maps_to_is_located_at_its_file_and_carries_no_timestamp() {
        let frontmatter = PlanningFrontmatter::try_from(raw(&format!(
            "{MINIMAL}title: Passkey login\nrelations:\n  - derived_from: epic:passwordless\n"
        )))
        .expect("valid");
        let artifact = frontmatter.to_artifact("story/passkey-login.md");

        assert_eq!(
            artifact.location,
            ArtifactLocation::RepositoryPath {
                repository: None,
                path: "story/passkey-login.md".to_owned(),
            }
        );
        assert!(
            artifact.provenance.is_none(),
            "git carries authorship; a second copy of it goes stale"
        );
        assert_eq!(artifact.metadata.title.as_deref(), Some("Passkey login"));
        assert_eq!(artifact.relations.len(), 1);
        assert_eq!(
            artifact.version.as_ref().map(ArtifactVersion::as_str),
            Some("1")
        );
    }

    /// The field a wave is derived from has to come back out of the file exactly as it went in,
    /// because the computation that reads it is comparing paths for equality.
    #[test]
    fn a_scope_round_trips_and_keeps_the_confidence_each_entry_was_written_with() {
        let front = PlanningFrontmatter::try_from(raw(&format!(
            "{MINIMAL}scope:\n  - path: crates/govern/aep-domain/src/artifact.rs\n    \
             confidence: cited\n  - path: crates/edge/aep-cli/src/planning.rs\n    \
             confidence: inferred\n"
        )))
        .expect("both entries are accepted");
        assert_eq!(
            front
                .scope
                .iter()
                .map(|entry| (entry.path.as_str(), entry.confidence))
                .collect::<Vec<_>>(),
            // Path order, which is the set's and not the document's: the two entries are written
            // the other way round above.
            vec![
                (
                    "crates/edge/aep-cli/src/planning.rs",
                    aep_domain::artifact::ScopeConfidence::Inferred
                ),
                (
                    "crates/govern/aep-domain/src/artifact.rs",
                    aep_domain::artifact::ScopeConfidence::Cited
                ),
            ]
        );

        let written = serde_yaml::to_string(&front).expect("the frontmatter renders");
        let again = PlanningFrontmatter::try_from(raw(&written)).expect("the rendering re-reads");
        assert_eq!(again.scope, front.scope);
        assert!(
            !again.extra.contains_key("scope"),
            "a named key must not also land in `extra`: {:?}",
            again.extra
        );
    }

    const V3: &str = "---\nformat: aep.planning-md/3\nid: story:one\nkind: story\nstatus: blocked\n\
title: One\nrevision: 4\ntransitions:\n\
- {from: \"draft\", to: \"active\", at: \"2026-09-28T10:04:11Z\", actor: \"human:timo\", revision: 2}\n\
- {from: \"active\", to: \"blocked\", at: \"2026-09-28T11:00:00Z\", actor: \"human:timo\", revision: 3, decided_on: {\"asserted\":{\"test_result\":1}}}\n\
board: alpha\n---\n\n# One\n";

    /// A Git-native document is written by the CLI and diffed by people: reading it and writing it
    /// back must not move a byte, or every command would rewrite lines it did not change.
    #[test]
    fn a_v3_document_with_transitions_renders_back_byte_for_byte() {
        let document =
            crate::document::PlanningDocument::parse(V3, None).expect("the /3 document parses");
        assert_eq!(document.frontmatter.format, PlanningFormat::V3);
        assert_eq!(document.frontmatter.transitions.len(), 2);
        assert_eq!(
            document.frontmatter.transitions[1]
                .decided_on
                .asserted
                .get(&EvidenceKind::TestResult),
            Some(&1)
        );
        assert!(
            !document.frontmatter.extra.contains_key("transitions"),
            "a named key must not also land in `extra`"
        );
        assert_eq!(document.render(), V3);
    }

    /// A document without transitions renders exactly as the block-style `Serialize` does, which
    /// is what keeps every `/1` and `/2` document byte-identical to what it was.
    #[test]
    fn without_transitions_the_rendering_is_the_serialize_rendering() {
        let front =
            PlanningFrontmatter::try_from(raw(&format!("{MINIMAL}sprint: 42\n"))).expect("valid");
        assert_eq!(
            front.render_yaml(),
            serde_yaml::to_string(&front).expect("serializes")
        );
        assert!(!front.render_yaml().contains("transitions"));
    }

    /// `status` is the last transition's `to` (git-native design § 4.3): a hand edit to either
    /// line alone is a status nothing moved to, and is refused naming the key.
    #[test]
    fn a_status_the_last_transition_did_not_move_to_is_refused() {
        let errors = PlanningFrontmatter::try_from(raw(
            "id: story:one\nkind: story\nstatus: implemented\nrevision: 2\ntransitions:\n\
             - {from: draft, to: active, at: \"2026-09-28T10:04:11Z\", actor: x, revision: 2}\n",
        ))
        .expect_err("the status and the transition disagree");
        assert_eq!(errors.len(), 1, "{errors}");
        assert_eq!(errors.as_slice()[0].location, "planning.transitions");
        assert!(errors.to_string().contains("implemented"), "{errors}");

        let errors = PlanningFrontmatter::try_from(raw(
            "id: story:one\nkind: story\nstatus: active\nrevision: 1\ntransitions:\n\
             - {from: draft, to: active, at: \"2026-09-28T10:04:11Z\", actor: x, revision: 2}\n",
        ))
        .expect_err("a move is a write, so revision 1 cannot follow one");
        assert_eq!(errors.as_slice()[0].location, "planning.revision");
    }

    /// Two entries for one path is a document that says two things about the same surface, and a
    /// computation reading it would have to pick one. Refused where every other defect is.
    #[test]
    fn one_path_declared_twice_is_refused_naming_the_path() {
        let errors = PlanningFrontmatter::try_from(raw(&format!(
            "{MINIMAL}scope:\n  - path: crates/x.rs\n    confidence: cited\n  \
             - path: crates/x.rs\n    confidence: inferred\n"
        )))
        .expect_err("a duplicate path is refused");
        assert!(
            errors.to_string().contains("crates/x.rs"),
            "the refusal names the path: {errors}"
        );
    }
}
