//! Several planning stores, read as one graph.
//!
//! A [`MarkdownStore`] answers for one repository. An [`Assembly`] answers for the members of a
//! workspace: the same artifacts, each remembering which member it came from, and one index so a
//! reference can be resolved across all of them.
//!
//! # The consequence for a person
//!
//! One `board` instead of three, and a story that is blocked by a story in another repository can
//! say so. Today that dependency lives in somebody's head and the first anybody hears of it is when
//! the blocked work is picked up.
//!
//! # What is deliberately not merged
//!
//! Nothing is renamed and no id is rewritten. Each member's documents stay exactly as that member
//! wrote them, and membership is carried **beside** the id rather than folded into it — so reading
//! a store through an assembly and reading it on its own give the same artifacts, and a member can
//! always be dropped from the workspace without touching a file.
//!
//! # A member that failed to load is reported, never skipped
//!
//! [`Assembly::failures`] carries every member's failures with the member's name attached. A store
//! that could not be read produces an empty member rather than an absent one, because an assembly
//! that quietly answered from two members when it was asked about three would give a smaller answer
//! that looks exactly like a complete one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use aep_domain::artifact::ArtifactId;
use aep_domain::workspace::{MemberName, Resolution, WorkspaceRef};

use crate::store::{MarkdownStore, StoreFailure, StoreReport, StoredDocument};

/// One member's store, as read.
#[derive(Debug, Clone)]
pub struct MemberStore {
    /// The member's name, which is the namespace its artifacts are addressed under.
    pub name: MemberName,
    /// Where the store was read from.
    pub root: PathBuf,
    /// What that read produced.
    pub report: StoreReport,
}

/// The members of a workspace, read as one graph.
#[derive(Debug, Clone, Default)]
pub struct Assembly {
    members: Vec<MemberStore>,
    index: BTreeMap<ArtifactId, BTreeSet<MemberName>>,
}

impl Assembly {
    /// Reads every member named, in the order given.
    ///
    /// A member whose store directory does not exist reads as empty rather than failing, which is
    /// [`MarkdownStore::load`]'s existing behaviour and the right one here: a member nobody has
    /// checked out is a normal condition on a machine that checked out a different subset.
    #[must_use]
    pub fn read<'a>(members: impl IntoIterator<Item = (MemberName, &'a Path)>) -> Self {
        let mut assembly = Self::default();
        for (name, root) in members {
            let report = MarkdownStore::open(root).load();
            for id in report.documents.keys() {
                assembly
                    .index
                    .entry(id.clone())
                    .or_default()
                    .insert(name.clone());
            }
            assembly.members.push(MemberStore {
                name,
                root: root.to_path_buf(),
                report,
            });
        }
        assembly
    }

    /// Every member, in the order the workspace named them.
    #[must_use]
    pub fn members(&self) -> &[MemberStore] {
        &self.members
    }

    /// Which members hold an artifact of this id.
    #[must_use]
    pub fn holders(&self, id: &ArtifactId) -> BTreeSet<MemberName> {
        self.index.get(id).cloned().unwrap_or_default()
    }

    /// Where a reference points, or why it does not point anywhere.
    #[must_use]
    pub fn resolve(&self, reference: &WorkspaceRef) -> Resolution {
        reference.resolve(&self.holders(&reference.artifact))
    }

    /// The document a reference names, when exactly one member holds it.
    ///
    /// `None` for both [`Resolution::Absent`] and [`Resolution::Ambiguous`] — deliberately, because
    /// returning *a* document for an ambiguous reference is the guess this whole path refuses.
    /// Callers that need to tell the two apart ask [`Self::resolve`], which says which it was.
    #[must_use]
    pub fn get(&self, reference: &WorkspaceRef) -> Option<(&MemberName, &StoredDocument)> {
        let Resolution::Unique(member) = self.resolve(reference) else {
            return None;
        };
        let store = self.members.iter().find(|m| m.name == member)?;
        let document = store.report.documents.get(&reference.artifact)?;
        Some((&store.name, document))
    }

    /// Every artifact in the workspace, member by member, in the order the workspace named them.
    pub fn documents(&self) -> impl Iterator<Item = (&MemberName, &ArtifactId, &StoredDocument)> {
        self.members.iter().flat_map(|store| {
            store
                .report
                .documents
                .iter()
                .map(move |(id, document)| (&store.name, id, document))
        })
    }

    /// Every id that more than one member holds.
    ///
    /// Not a defect: two repositories may each have a `story:passkey-login` and mean different
    /// work, which is exactly why an unqualified reference to one is refused rather than resolved.
    /// It is reported so a person can see which names now need qualifying.
    #[must_use]
    pub fn shared_ids(&self) -> BTreeMap<ArtifactId, BTreeSet<MemberName>> {
        self.index
            .iter()
            .filter(|(_, members)| members.len() > 1)
            .map(|(id, members)| (id.clone(), members.clone()))
            .collect()
    }

    /// Every failure any member reported, each carrying the member it came from.
    #[must_use]
    pub fn failures(&self) -> Vec<(&MemberName, &StoreFailure)> {
        self.members
            .iter()
            .flat_map(|store| {
                store
                    .report
                    .failures
                    .iter()
                    .map(move |failure| (&store.name, failure))
            })
            .collect()
    }

    /// Every relation whose target names another member, with what resolving it found.
    ///
    /// This is the edge a workspace exists to carry: a story here blocked by a story there. A
    /// single store cannot check one — the target is outside it by construction — so
    /// [`aep_domain::artifact::ArtifactGraph`] leaves a member-qualified target alone and this is
    /// where it is answered.
    ///
    /// [`Resolution::Absent`] is reported, not refused. A member nobody has checked out holds
    /// nothing, and a workspace read on a machine with a different subset would otherwise fail for
    /// a reason that has nothing to do with the plan.
    #[must_use]
    pub fn crossing_relations(&self) -> Vec<CrossingRelation> {
        let mut crossings = Vec::new();
        for (member, id, document) in self.documents() {
            for relation in &document.document.frontmatter.relations {
                let target = relation.target.id();
                if target.member().is_none() {
                    continue;
                }
                // An id that parsed as an artifact but not as a workspace reference names more
                // than one member, which the reference type refuses; there is nothing to resolve.
                let Ok(reference) = WorkspaceRef::parse(target.to_string()) else {
                    continue;
                };
                let resolution = self.resolve(&reference);
                crossings.push(CrossingRelation {
                    from_member: member.clone(),
                    from: id.clone(),
                    kind: relation.kind.to_string(),
                    to: reference,
                    resolution,
                });
            }
        }
        crossings
    }

    /// How many artifacts the workspace holds in total.
    #[must_use]
    pub fn len(&self) -> usize {
        self.members
            .iter()
            .map(|store| store.report.documents.len())
            .sum()
    }

    /// `true` when no member holds an artifact.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// One relation whose target lives in another member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossingRelation {
    /// The member the relation was written in.
    pub from_member: MemberName,
    /// The artifact that carries it.
    pub from: ArtifactId,
    /// Which relation it is: `blocks`, `depends_on`, and the rest of the vocabulary.
    pub kind: String,
    /// Where it points.
    pub to: WorkspaceRef,
    /// What looking for the target found.
    pub resolution: Resolution,
}

impl CrossingRelation {
    /// `true` when the target is exactly one artifact in exactly one member.
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        matches!(self.resolution, Resolution::Unique(_))
    }
}
