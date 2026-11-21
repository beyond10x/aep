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

/// Every roadmap rule over one opened store, whose own workspace member, if it declares one, is
/// `own`.
pub(crate) fn check(
    report: &StoreReport,
    own: Option<&aep_domain::workspace::MemberName>,
    limit: Result<usize, String>,
) -> Roadmaps {
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
        if !front.kind.is_a(&ArtifactKind::Roadmap) {
            continue;
        }
        let id = &front.id;
        let mut entries = match roadmap::recorded(&stored.document.body) {
            Ok(entries) => entries.unwrap_or_default(),
            Err(errors) => {
                problems.extend(errors.iter().map(|error| format!("{id}: {error}")));
                Vec::new()
            }
        };
        // This store's own member name is this store's artifact written the long way, so the
        // spelling decides nothing (`ArtifactId::local_to`), here as on an edge.
        for entry in &mut entries {
            entry.epic = entry.epic.local_to(own).into_owned();
        }
        for defect in roadmap::defects(&entries) {
            problems.push(format!("{id}: {defect}"));
        }
        problems.extend(listed_problems(report, id, &entries));
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

/// What the entries of roadmap `id` list, judged against the store: an artifact it holds, on the
/// epic family. Each epic is judged once, however often it is listed; listing it twice is
/// `roadmap::defects`' finding, not this one's.
fn listed_problems(report: &StoreReport, id: &ArtifactId, entries: &[roadmap::Entry]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut named = BTreeSet::new();
    for (index, entry) in entries.iter().enumerate() {
        if !named.insert(&entry.epic) {
            continue;
        }
        let position = index + 1;
        let named_kind = ArtifactKind::parse(entry.epic.namespace()).ok();
        match report.documents.get(&entry.epic) {
            // One defect, not two: an id whose kind is not an epic is wrong whether or not it
            // resolves, and saying it is missing as well would send the reader to create it.
            None if !named_kind.is_some_and(|kind| kind.is_a(&ArtifactKind::Epic)) => {
                problems.push(format!(
                    "{id} lists {} at entry {position}, which is not an epic and is not in this \
                     store; a roadmap orders epics",
                    entry.epic
                ));
            }
            None => problems.push(format!(
                "{id} lists {} at entry {position}, and there is no such artifact in this store — \
                 a roadmap orders this store's epics; create the epic or take the entry out",
                entry.epic
            )),
            // An epic is anything on the epic family, so a `platform-epic` the off-roadmap list
            // names is one a roadmap can place.
            Some(listed) if !listed.document.frontmatter.kind.is_a(&ArtifactKind::Epic) => {
                problems.push(format!(
                    "{id} lists {} at entry {position}, which is a {} and not an epic; a roadmap \
                     orders epics",
                    entry.epic, listed.document.frontmatter.kind
                ));
            }
            Some(_) => {}
        }
    }
    problems
}
