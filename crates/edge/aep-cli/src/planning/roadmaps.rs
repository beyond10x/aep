//! What `aep plan artifact validate` says about the store's roadmaps (`story:roadmap-kind`).
//!
//! **Problems**, which fail `validate`: a `roadmap` block that does not read; an entry that names
//! something other than an epic, names an epic twice, goes back down the horizons, or names an
//! artifact the store does not hold; an approved roadmap that no `approval-record` reviews or
//! decides; and more approved roadmaps than the project's profile allows.
//!
//! **Reported, and not a problem**: a `proposed` or `active` epic that no approved roadmap lists,
//! once the store holds an approved roadmap at all. An epic is proposed *before* the roadmap that
//! would place it is approved, so refusing would break every store between a proposal and the next
//! approval; listing it keeps it visible, and `--strict` is how a gate that wants the stop refuses
//! it. A store with no approved roadmap has declared no order, and the rule waits for the first one,
//! as grounding waits for the first `vision`.

use std::collections::BTreeSet;
use std::path::Path;

use aep_backend_markdown::{roadmap, StoreReport};
use aep_domain::artifact::{ArtifactId, ArtifactKind, ArtifactStatus, RelationKind};
use aep_domain::facts::{FactPath, FactValue};
use aep_project::project::project_directory;

/// The profile fact that raises how many roadmaps may be approved at once.
pub(crate) const LIMIT_FACT: &str = "roadmap.approved.limit";

/// What `validate` found about the store's roadmaps.
pub(crate) struct Roadmaps {
    /// Defects, each failing `validate`.
    pub(crate) problems: Vec<String>,
    /// Live epics no approved roadmap lists, one sentence each.
    pub(crate) off_roadmap: Vec<String>,
}

/// How many roadmaps the project's profile allows approved at once, or why the profile's answer is
/// not a number this can use.
///
/// One unless the resolved profile declares [`LIMIT_FACT`]. The profile is the project's own
/// (`profile:` in `project.yaml`), resolved over the protocol tree's registry with the project's
/// own profiles merged in, as `aep govern` resolves it. A store no project file governs, or a
/// profile that does not resolve, gets the default: what is wrong with the project file is
/// reported by the verbs that read it, and a missing profile is not a reason to allow more.
pub(crate) fn approved_limit(
    registry: &aep_engine::Registry,
    repository_root: &Path,
) -> Result<usize, String> {
    let engineering = repository_root.join(project_directory());
    let Ok(Some(config)) = super::project_config(&engineering) else {
        return Ok(1);
    };
    let mut registry = registry.clone();
    // A local profile that does not load leaves the reference unresolved below, which is the
    // default; `aep govern validate` is where its own defect is reported.
    let _ = aep_project::project::merge_local_profiles(&mut registry, &engineering, &config);
    let Ok(profile) = registry.resolved_profile(&config.profile) else {
        return Ok(1);
    };
    let path = FactPath::new(LIMIT_FACT).expect("the limit fact is a valid path");
    match profile.facts.get(&path) {
        None => Ok(1),
        Some(FactValue::Number(number)) if number.is_integral() && number.get() >= 1.0 => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Ok(number.get() as usize)
        }
        Some(other) => Err(format!(
            "profile {} declares `{LIMIT_FACT}` as {other}, which is not a whole number of at least \
             1; it says how many roadmaps may be approved at once",
            config.profile
        )),
    }
}

/// Every roadmap rule over one opened store.
pub(crate) fn check(report: &StoreReport, limit: Result<usize, String>) -> Roadmaps {
    let mut problems = Vec::new();
    let mut approved: Vec<&ArtifactId> = Vec::new();
    let mut placed: BTreeSet<ArtifactId> = BTreeSet::new();

    let approving: BTreeSet<&ArtifactId> = report
        .documents
        .values()
        .map(|stored| &stored.document.frontmatter)
        .filter(|front| front.kind == ArtifactKind::ApprovalRecord)
        .flat_map(|front| {
            front
                .targets(RelationKind::Reviews)
                .chain(front.targets(RelationKind::Decides))
                .map(|relation| relation.target.id())
        })
        .collect();

    for stored in report.documents.values() {
        let front = &stored.document.frontmatter;
        if front.kind != ArtifactKind::Roadmap {
            continue;
        }
        let id = &front.id;
        let entries = match roadmap::recorded(&stored.document.body) {
            Ok(entries) => entries.unwrap_or_default(),
            Err(error) => {
                problems.push(format!("{id}: {error}"));
                Vec::new()
            }
        };
        for defect in roadmap::defects(&entries) {
            problems.push(format!("{id}: {defect}"));
        }
        let mut named = BTreeSet::new();
        for entry in &entries {
            if entry.epic.namespace() == "epic"
                && named.insert(&entry.epic)
                && !report.documents.contains_key(&entry.epic)
            {
                problems.push(format!(
                    "{id} lists {}, and there is no such artifact in this store — create the epic \
                     or take the entry out",
                    entry.epic
                ));
            }
        }
        if front.status == ArtifactStatus::Approved {
            approved.push(id);
            placed.extend(entries.into_iter().map(|entry| entry.epic));
            if !approving.contains(id) {
                problems.push(format!(
                    "{id} is approved and no approval-record reviews or decides it — `aep plan \
                     artifact new approval-record <name> --relate reviews:{id}` records who \
                     approved it and on what"
                ));
            }
        }
    }

    match limit {
        Err(problem) => problems.push(problem),
        Ok(limit) if approved.len() > limit => problems.push(format!(
            "{} roadmaps are approved ({}) and this project allows {limit} — move the one a \
             successor replaced to `superseded`, or declare `{LIMIT_FACT}` in the project's profile",
            approved.len(),
            approved
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Ok(_) => {}
    }

    let mut off_roadmap = Vec::new();
    if !approved.is_empty() {
        for stored in report.documents.values() {
            let front = &stored.document.frontmatter;
            let live = matches!(front.status, ArtifactStatus::Proposed | ArtifactStatus::Active);
            if front.kind.is_a(&ArtifactKind::Epic) && live && !placed.contains(&front.id) {
                off_roadmap.push(format!(
                    "{} is {} and on no approved roadmap — a successor roadmap that lists it, once \
                     approved, places it",
                    front.id, front.status
                ));
            }
        }
    }

    Roadmaps {
        problems,
        off_roadmap,
    }
}
