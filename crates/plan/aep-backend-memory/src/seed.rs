//! Turning an artifact manifest into entities and relations.
//!
//! A manifest is the human-facing spelling of the graph the interaction contract stores:
//! `design:passkeys-auth` with `{designs: spec:passkeys-auth}` is an entity of type
//! `aep.design/v1` with a `designs` relation to another entity. This module converts one into the
//! other **through [`CommandService`]**, never by writing to the store, and that is the whole point
//! of it:
//!
//! * a manifest that seeds is a manifest that is expressible through the contract, which is a
//!   claim worth checking rather than assuming;
//! * everything it creates gets history, events and audit records exactly like anything else, so
//!   `aep plan entity history` and `aep plan audit` have something real to show.
//!
//! Command ids and idempotency keys are derived from the artifact id, so seeding the same manifest
//! into the same backend twice is a replay rather than a second set of entities — and derived
//! injectively, so two artifacts are never one command (see `token`).
//!
//! # Two passes
//!
//! Entities first, relations second. A manifest may declare an edge to an artifact that appears
//! later in the file — `design:passkeys-auth` sorts before `spec:passkeys-auth` — and a
//! single-pass seeder would have to either drop that edge or reorder the manifest behind the
//! reader's back.

use std::collections::BTreeMap;

use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::error::CommandError;
use aep_contract::testing::block_on;
use aep_domain::artifact::{Artifact, ArtifactGraph, ArtifactId};
use aep_domain::command::{Command, CreateEntity, CreateRelation};
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef};
use aep_domain::error::ParseError;
use aep_domain::ids::{CommandId, CorrelationId, IdempotencyKey, RequestId};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;

use crate::command::STATUS_KEY;

/// The activity every seeding command belongs to.
///
/// Fixed rather than generated: `aep plan audit --correlation seed-manifest` is how a reader asks
/// for "everything this run did", and a correlation id nobody can predict cannot be typed.
pub const SEED_CORRELATION: &str = "seed-manifest";

/// What one seeding run produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedReport {
    /// How many entities the manifest maps to.
    pub entities: usize,
    /// How many relations were issued between them.
    pub relations: usize,
    /// The identity each artifact was stored under, so a caller can map its manifest back.
    pub by_id: BTreeMap<ArtifactId, EntityId>,
}

/// Seeds `backend` with the artifacts in `graph`, addressed under `organisation`/`space`.
///
/// Every artifact becomes an entity of its kind's entity type, at
/// `ep://<organisation>/<space>/<kind>/<name>`; every artifact relation becomes a relation command
/// once all the entities exist.
///
/// Fails — rather than dropping an edge — when a relation points at an artifact the manifest does
/// not declare. A dangling edge means the manifest is describing a graph it does not contain, and
/// seeding a partial graph would make the missing artifact look like a deliberate absence.
pub fn from_manifest<B: CommandService<Command = Command>>(
    backend: &B,
    graph: &ArtifactGraph,
    organisation: &str,
    space: &str,
    at: Timestamp,
    actor: &ActorRef,
) -> Result<SeedReport, CommandError> {
    // Before the first command, so a refusal writes nothing: two artifacts that would be one
    // entity used to seed as one, or seed the first and refuse the second.
    let shared = collisions(graph);
    if !shared.is_empty() {
        return Err(CommandError::Conflict {
            reason: shared.join("; "),
        });
    }

    let mut report = SeedReport::default();

    for artifact in graph.artifacts() {
        let id = create_entity(backend, artifact, organisation, space, at, actor)?;
        report.by_id.insert(artifact.id.clone(), id);
        report.entities += 1;
    }

    for artifact in graph.artifacts() {
        let source = report
            .by_id
            .get(&artifact.id)
            .expect("the first pass stored every artifact in the manifest")
            .clone();
        for relation in &artifact.relations {
            // A target naming **this store's own** member is this store's artifact written the
            // long way, and it seeds like any local edge — otherwise the spelling an author chose
            // would decide whether the relation surface holds the edge.
            let target_id = relation.target.id().local_to(graph.membership().own());
            let target_id = target_id.as_ref();
            // **A crossing is not a dangling edge.** A target naming another member lives in
            // another store by construction; seeding one repository cannot resolve it, and failing
            // here made every verb that opens a backend refuse on a store holding a legitimate
            // cross-repository relation — which is what a workspace is for. The edge stays in the
            // document, where `aep plan workspace crossings` resolves it against the assembly.
            if target_id.member().is_some() {
                continue;
            }
            let target =
                report
                    .by_id
                    .get(target_id)
                    .cloned()
                    .ok_or_else(|| CommandError::Conflict {
                        reason: format!(
                            "`{}` declares `{} {target_id}`, but the manifest does not declare \
                             `{target_id}`",
                            artifact.id, relation.kind
                        ),
                    })?;
            let payload = Command::CreateRelation(CreateRelation {
                kind: relation.kind,
                source: EntityRef::new(source.clone()),
                target: EntityRef::new(target),
            });
            let name = relation_command_name(&artifact.id, relation.kind.as_str(), target_id);
            block_on(backend.execute(envelope(&name, payload, at, actor)?))?;
            report.relations += 1;
        }
    }

    Ok(report)
}

/// Every pair of artifacts in `graph` that seeding would store as one entity, one sentence each.
///
/// Two things identify a seeded artifact, and both are derived from it: its address — its kind and
/// its name, with each `/` as `-` because a locator key cannot hold one — and its command
/// identifier. `token` gives each artifact id its own identifier, so what this finds in practice
/// is two artifacts at one address: `story:a/b` beside `story:a-b`, or `doc:x` of kind `design`
/// beside `design:x`. The second identifier check costs nothing and keeps the guarantee checked
/// rather than assumed — an identifier over the length cap is shortened with a digest.
///
/// [`from_manifest`] refuses a graph this reports on before it issues a command, and
/// `aep govern validate --artifacts` reports the same sentences as problems, so a manifest that
/// validates is one that seeds.
#[must_use]
pub fn collisions(graph: &ArtifactGraph) -> Vec<String> {
    let mut problems = Vec::new();
    let mut addresses: BTreeMap<String, &ArtifactId> = BTreeMap::new();
    let mut commands: BTreeMap<String, &ArtifactId> = BTreeMap::new();
    for artifact in graph.artifacts() {
        let address = format!(
            "{}/{}",
            artifact.kind.as_str(),
            locator_key(artifact.id.name())
        );
        if let Some(earlier) = addresses.get(&address) {
            problems.push(format!(
                "`{earlier}` and `{}` would be one entity at one address, `{address}` (an \
                 address is the kind and the name with each `/` as `-`); rename one of them",
                artifact.id
            ));
        } else {
            addresses.insert(address, &artifact.id);
        }

        let name = bounded(&entity_command_name(&artifact.id));
        if let Some(earlier) = commands.get(&name) {
            problems.push(format!(
                "`{earlier}` and `{}` would be seeded by one command, `{name}`; rename one of \
                 them",
                artifact.id
            ));
        } else {
            commands.insert(name, &artifact.id);
        }
    }
    problems
}

/// Issues the `CreateEntity` command for one artifact and returns the identity it was stored under.
fn create_entity<B: CommandService<Command = Command>>(
    backend: &B,
    artifact: &Artifact,
    organisation: &str,
    space: &str,
    at: Timestamp,
    actor: &ActorRef,
) -> Result<EntityId, CommandError> {
    let locator = EntityLocator::new(
        organisation,
        space,
        artifact.kind.as_str(),
        locator_key(artifact.id.name()),
    )
    .map_err(|error| CommandError::Conflict {
        reason: format!("`{}` cannot be given a locator: {error}", artifact.id),
    })?;

    let payload = Command::CreateEntity(CreateEntity {
        entity_type: artifact.kind.entity_type(),
        locator,
        data: body(artifact),
    });
    let name = entity_command_name(&artifact.id);
    let result = block_on(backend.execute(envelope(&name, payload, at, actor)?))?;

    result
        .affected
        .first()
        .map(|reference| reference.id.clone())
        .ok_or_else(|| CommandError::Conflict {
            reason: format!("creating `{}` reported no entity", artifact.id),
        })
}

/// The body an artifact is stored with.
///
/// Only what the manifest actually states: a title key holding an empty string would be a claim
/// the manifest never made.
fn body(artifact: &Artifact) -> Node {
    let mut fields: BTreeMap<String, Node> = BTreeMap::new();
    fields.insert(STATUS_KEY.to_owned(), Node::from(artifact.status.as_str()));
    for (key, value) in [
        ("title", artifact.metadata.title.as_deref()),
        ("summary", artifact.metadata.summary.as_deref()),
        ("owner", artifact.metadata.owner.as_deref()),
    ] {
        if let Some(value) = value {
            fields.insert(key.to_owned(), Node::from(value));
        }
    }
    if !artifact.metadata.tags.is_empty() {
        fields.insert(
            "tags".to_owned(),
            Node::Seq(
                artifact
                    .metadata
                    .tags
                    .iter()
                    .map(|tag| Node::from(tag.as_str()))
                    .collect(),
            ),
        );
    }
    if let Some(version) = &artifact.version {
        fields.insert("version".to_owned(), Node::from(version.as_str()));
    }
    Node::Map(fields)
}

/// Wraps a seeding command, deriving its identifiers from `name` so a replay is recognisable.
fn envelope(
    name: &str,
    payload: Command,
    at: Timestamp,
    actor: &ActorRef,
) -> Result<CommandEnvelope<Command>, CommandError> {
    let name = &bounded(name);
    let command_id = CommandId::new(name).map_err(|error| unusable(name, &error))?;
    let idempotency_key = IdempotencyKey::new(name).map_err(|error| unusable(name, &error))?;
    let request_id =
        RequestId::new(format!("req-{name}")).map_err(|error| unusable(name, &error))?;
    let context = CommandContext::new(
        request_id,
        idempotency_key,
        actor.clone(),
        CorrelationId::new(SEED_CORRELATION).expect("a built-in correlation id is well formed"),
        at,
    );

    let command_type = payload.kind().as_str().to_owned();
    let target = payload.target();
    let expected_revision = payload.expected_revision();
    let mut envelope = CommandEnvelope::new(command_id, command_type, payload, context);
    envelope.target = target;
    envelope.expected_revision = expected_revision;
    Ok(envelope)
}

/// Reports an artifact id that cannot be turned into a command identifier.
fn unusable(name: &str, error: &ParseError) -> CommandError {
    CommandError::Conflict {
        reason: format!("`{name}` is not a usable command identifier: {error}"),
    }
}

/// `name` cut to the identifier cap with room for the `req-` a request id puts in front of it.
fn bounded(name: &str) -> String {
    aep_domain::ids::bounded_identifier(name, "req-".len())
}

/// The name of the command that creates the entity of `id`, before [`bounded`].
fn entity_command_name(id: &ArtifactId) -> String {
    format!("seed-{}", token(id))
}

/// The name of the command that seeds the edge `source kind target`, before [`bounded`].
///
/// `/` between the three parts, because no [`token`] holds one and no relation kind does: with
/// `-` there, `story:a decomposes story:b-specifies-story-c` and
/// `story:a-decomposes-story-b specifies story:c` were one command, and the second edge was
/// refused as a reuse of the first's idempotency key.
fn relation_command_name(source: &ArtifactId, kind: &str, target: &ArtifactId) -> String {
    format!("seed-rel-{}/{kind}/{}", token(source), token(target))
}

/// A command-identifier fragment derived from an artifact id, different for every artifact id.
///
/// Deterministic, because that is what makes a second seeding run a replay: the same artifact id
/// must produce the same command id every time, in any process. **Injective**, because two
/// artifacts given one fragment are one command: the second is refused as a reuse of the first's
/// idempotency key — every verb that opens the store then refuses — or, when the two would create
/// identical entities, silently replays the first.
///
/// An id is letter-or-digit runs with separator runs (`:`, `-`, `_`, `.`, `/`) between, before and
/// after them. An identifier allows a single separator between two runs only, so:
///
/// * an id of one namespace word, `:`, and a name of runs joined by single `-` — `design:passkeys-auth`,
///   `story:AUTH-141` — is its runs joined by `-`: `design-passkeys-auth`, as it always was. The
///   first `-` is the `:`, so no two such ids share a fragment;
/// * every other id — `story:render_a`, `review-result:initial`, `docs/design:a//b` — is its runs
///   joined by `-`, then `.`, then one code per separator run, leading and trailing runs included,
///   joined by `-`: each separator as a letter (`:` `c`, `-` `h`, `.` `d`, `_` `u`, `/` `s`), an
///   empty run as `x`. `story:render_a` is `story-render-a.x-c-u-x`. The `.` is absent from the
///   first form, and the codes give back every separator, so no two ids share a fragment here
///   either. An id with no letter or digit is `artifact.` and its one code.
///
/// The fragment is a stored identity only for the length of one process — a Git-native store is
/// hydrated into memory on each open, and no file holds it — so the second form changing the
/// fragment some ids had before is invisible on disk.
fn token(id: &ArtifactId) -> String {
    let text = id.to_string();
    let mut runs: Vec<String> = Vec::new();
    // `gaps[0]` precedes the first run, `gaps[i]` follows run `i - 1`: one more than the runs.
    let mut gaps: Vec<String> = vec![String::new()];
    let mut run = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            run.push(character);
        } else {
            if !run.is_empty() {
                runs.push(std::mem::take(&mut run));
                gaps.push(String::new());
            }
            gaps.last_mut()
                .expect("there is always a gap")
                .push(character);
        }
    }
    if !run.is_empty() {
        runs.push(run);
        gaps.push(String::new());
    }

    let last = gaps.len() - 1;
    let plain = runs.len() >= 2
        && gaps[0].is_empty()
        && gaps[last].is_empty()
        && gaps[1] == ":"
        && gaps[2..last].iter().all(|gap| gap == "-");
    if plain {
        return runs.join("-");
    }

    let head = if runs.is_empty() {
        "artifact".to_owned()
    } else {
        runs.join("-")
    };
    let codes: Vec<String> = gaps
        .iter()
        .map(|gap| {
            if gap.is_empty() {
                return "x".to_owned();
            }
            gap.chars()
                .map(|separator| match separator {
                    ':' => 'c',
                    '-' => 'h',
                    '.' => 'd',
                    '_' => 'u',
                    // `ArtifactId` admits no other character outside letters and digits.
                    _ => 's',
                })
                .collect()
        })
        .collect();
    format!("{head}.{}", codes.join("-"))
}

/// The locator key an artifact name maps to.
///
/// Artifact names allow `/`, locator segments do not, so anything a locator cannot carry becomes
/// `-`. That is the one lossy step left in seeding, and the only one a locator key's alphabet
/// forces: [`collisions`] names two artifacts it would put at one address, and [`from_manifest`]
/// refuses them before it writes anything.
fn locator_key(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::MemoryBackend;
    use aep_contract::consistency::QueryConsistency;
    use aep_contract::query::{QueryService, RelationQuery};
    use aep_domain::artifact::{
        ArtifactKind, ArtifactLocation, ArtifactMetadata, ArtifactRef, ArtifactStatus,
        ArtifactVersion, RelationKind,
    };

    use super::*;

    /// The actor every test seeds as.
    fn actor() -> ActorRef {
        "service:aep-cli".parse().expect("actor")
    }

    /// An artifact at an inline location, which keeps the fixtures about the graph.
    fn artifact(id: &str, kind: ArtifactKind, status: ArtifactStatus) -> Artifact {
        Artifact::new(
            id.parse().expect("artifact id"),
            kind,
            status,
            ArtifactLocation::Inline,
        )
    }

    /// The shape of `examples/development-passkeys/artifacts.yaml`: six artifacts, five edges.
    fn example() -> ArtifactGraph {
        let mut prd = artifact(
            "prd:passkeys",
            ArtifactKind::ProductRequirements,
            ArtifactStatus::Active,
        );
        prd.metadata = ArtifactMetadata {
            title: Some("Passwordless sign-in".to_owned()),
            ..ArtifactMetadata::default()
        };

        let story = artifact(
            "story:AUTH-141",
            ArtifactKind::Story,
            ArtifactStatus::Active,
        )
        .with_relation(
            RelationKind::DerivedFrom,
            ArtifactRef::unpinned("prd:passkeys".parse().expect("artifact id")),
        );

        let specification = artifact(
            "spec:passkeys-auth",
            ArtifactKind::Specification,
            ArtifactStatus::Approved,
        )
        .with_relation(
            RelationKind::Specifies,
            ArtifactRef::unpinned("story:AUTH-141".parse().expect("artifact id")),
        );

        let mut design = artifact(
            "design:passkeys-auth",
            ArtifactKind::Design,
            ArtifactStatus::Approved,
        )
        .with_relation(
            RelationKind::Designs,
            ArtifactRef::unpinned("spec:passkeys-auth".parse().expect("artifact id")),
        );
        design.version = Some(ArtifactVersion::new("7"));

        let adr = artifact(
            "adr:0042",
            ArtifactKind::ArchitectureDecisionRecord,
            ArtifactStatus::Accepted,
        )
        .with_relation(
            RelationKind::Decides,
            ArtifactRef::unpinned("design:passkeys-auth".parse().expect("artifact id")),
        );

        let review = artifact(
            "review:design-passkeys-auth",
            ArtifactKind::ReviewResult,
            ArtifactStatus::Active,
        )
        .with_relation(
            RelationKind::Reviews,
            ArtifactRef::unpinned("design:passkeys-auth".parse().expect("artifact id")),
        );

        ArtifactGraph::build([prd, story, specification, design, adr, review])
            .expect("the example graph is valid")
    }

    /// Seeds `graph` into a fresh backend.
    fn seed(graph: &ArtifactGraph) -> (MemoryBackend, SeedReport) {
        let backend = MemoryBackend::new();
        let report = from_manifest(
            &backend,
            graph,
            "acme",
            "payments",
            Timestamp::from_epoch_millis(1_000),
            &actor(),
        )
        .expect("the manifest seeds");
        (backend, report)
    }

    #[test]
    fn every_artifact_becomes_an_entity_of_its_kinds_type() {
        let (backend, report) = seed(&example());

        assert_eq!(report.entities, 6, "one entity per artifact");
        assert_eq!(backend.len(), 6);
        assert_eq!(report.by_id.len(), 6);

        let design = block_on(
            backend.resolve(
                &"ep://acme/payments/design/passkeys-auth"
                    .parse()
                    .expect("locator"),
            ),
        )
        .expect("the design is addressable by its locator");
        assert_eq!(
            Some(&design),
            report
                .by_id
                .get(&"design:passkeys-auth".parse().expect("artifact id")),
            "the report maps the manifest back to what was stored"
        );

        let entity = block_on(backend.get(&EntityRef::new(design), QueryConsistency::Current))
            .expect("the design is readable");
        assert_eq!(entity.metadata.entity_type.to_string(), "aep.design/v1");
        let Node::Map(fields) = &entity.data else {
            panic!("the body is a mapping");
        };
        assert_eq!(fields.get(STATUS_KEY), Some(&Node::from("approved")));
        assert_eq!(fields.get("version"), Some(&Node::from("7")));
        assert!(
            !fields.contains_key("title"),
            "the manifest states no title for the design, so the entity claims none"
        );
        assert!(
            !fields.contains_key("tags"),
            "the manifest states no tags for the design, so the entity claims none"
        );
    }

    #[test]
    fn tags_travel_with_the_artifact() {
        // A planning document's tags are what `list --kind` and a board group by; a seed that
        // dropped them would hand a SQLite plan seven untagged artifacts and call it the same plan
        // (`story:store-selection-in-project-yaml`).
        let mut graph = example();
        let id: aep_domain::artifact::ArtifactId =
            "design:passkeys-auth".parse().expect("artifact id");
        let mut tagged = graph.get(&id).expect("the design is declared").clone();
        tagged.metadata.tags = ["identity", "security"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        graph.insert(tagged);

        let backend = MemoryBackend::new();
        let report = from_manifest(
            &backend,
            &graph,
            "acme",
            "platform",
            aep_domain::time::Timestamp::from_epoch_millis(1_700_000_000_000),
            &actor(),
        )
        .expect("seeding succeeds");
        let design = report.by_id.get(&id).expect("seeded").clone();
        let entity = block_on(backend.get(&EntityRef::new(design), QueryConsistency::Current))
            .expect("the design is readable");
        let Node::Map(fields) = &entity.data else {
            panic!("the body is a mapping");
        };
        assert_eq!(
            fields.get("tags"),
            Some(&Node::Seq(vec![
                Node::from("identity"),
                Node::from("security")
            ]))
        );
    }

    #[test]
    fn a_relation_to_an_artifact_declared_later_still_resolves() {
        // `design:passkeys-auth` is seeded before `spec:passkeys-auth`, so this edge only works
        // because relations are issued in a second pass.
        let (backend, report) = seed(&example());
        assert_eq!(report.relations, 5);

        let design = report
            .by_id
            .get(&"design:passkeys-auth".parse().expect("artifact id"))
            .expect("the design was seeded")
            .clone();
        let specification = report
            .by_id
            .get(&"spec:passkeys-auth".parse().expect("artifact id"))
            .expect("the specification was seeded")
            .clone();

        let outgoing =
            block_on(backend.relations(&RelationQuery::from(EntityRef::new(design.clone()))))
                .expect("relations are queryable");
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing.items[0].kind, RelationKind::Designs);
        assert_eq!(outgoing.items[0].target.id, specification);

        let incoming = block_on(backend.relations(&RelationQuery::to(EntityRef::new(design))))
            .expect("the inverse question is answerable");
        assert_eq!(
            incoming.len(),
            2,
            "the ADR decides it and the review reviews it"
        );
    }

    #[test]
    fn seeding_twice_replays_rather_than_duplicating() {
        let graph = example();
        let (backend, first) = seed(&graph);

        let second = from_manifest(
            &backend,
            &graph,
            "acme",
            "payments",
            Timestamp::from_epoch_millis(2_000),
            &actor(),
        )
        .expect("the second run is a replay, not a conflict");

        assert_eq!(first, second, "the same manifest maps to the same entities");
        assert_eq!(backend.len(), 6, "no second set of entities");
        assert_eq!(
            backend.with_store(|store| store.relations().count()),
            5,
            "no second set of relations"
        );

        let design = EntityRef::new(
            first
                .by_id
                .get(&"design:passkeys-auth".parse().expect("artifact id"))
                .expect("the design was seeded")
                .clone(),
        );
        let history = block_on(backend.history(&design)).expect("history is kept");
        assert_eq!(
            history.len(),
            1,
            "a replay is not a change, so it adds no revision"
        );
    }

    #[test]
    fn a_relation_target_the_manifest_does_not_declare_is_reported() {
        // Built by hand: `ArtifactGraph::build` rejects a dangling edge, and the point here is what
        // seeding does when one reaches it anyway.
        let mut graph = ArtifactGraph::new();
        graph.insert(
            artifact(
                "design:passkeys-auth",
                ArtifactKind::Design,
                ArtifactStatus::Draft,
            )
            .with_relation(
                RelationKind::Designs,
                ArtifactRef::unpinned("spec:missing".parse().expect("artifact id")),
            ),
        );

        let backend = MemoryBackend::new();
        let error = from_manifest(
            &backend,
            &graph,
            "acme",
            "payments",
            Timestamp::from_epoch_millis(1_000),
            &actor(),
        )
        .expect_err("the edge points at nothing");

        assert_eq!(error.code(), "conflict");
        assert!(
            error.to_string().contains("spec:missing"),
            "the report names the artifact that is missing: {error}"
        );
        assert_eq!(
            backend.with_store(|store| store.relations().count()),
            0,
            "the edge is refused rather than dropped in silence"
        );
    }

    #[test]
    fn an_identifier_is_derived_from_the_artifact_id_and_stays_addressable() {
        assert_eq!(
            token(&"design:passkeys-auth".parse().expect("artifact id")),
            "design-passkeys-auth"
        );
        assert_eq!(token(&"adr:0042".parse().expect("artifact id")), "adr-0042");
        assert_eq!(
            token(&"docs/design:a//b".parse().expect("artifact id")),
            "docs-design-a-b.x-s-c-ss-x",
            "identifiers reject repeated separators, so a run of them is spelled as a code"
        );
        assert_eq!(
            token(&"story:render_a".parse().expect("artifact id")),
            "story-render-a.x-c-u-x"
        );
        assert!(CommandId::new(format!(
            "seed-{}",
            token(&"docs/design:a//b".parse().expect("artifact id"))
        ))
        .is_ok());

        assert_eq!(locator_key("passkeys/auth"), "passkeys-auth");
        assert_eq!(locator_key("AUTH-141"), "AUTH-141");
    }

    /// A graph of draft stories with the given ids, so a test is about the ids alone.
    fn stories(ids: &[&str]) -> ArtifactGraph {
        ArtifactGraph::build(
            ids.iter()
                .map(|id| artifact(id, ArtifactKind::Story, ArtifactStatus::Draft)),
        )
        .expect("the graph is valid")
    }

    #[test]
    fn ids_that_differ_only_in_a_separator_seed_as_distinct_entities() {
        // `story:render-a` and `story:render_a` are two artifacts `validate` accepts; both used to
        // seed under the command `seed-story-render-a`, and the second was refused as a reuse of
        // the first's idempotency key, so every verb that opens the store refused.
        let ids = [
            "story:render-a",
            "story:render_a",
            "story:render.a",
            "story:render--a",
            "review-result:initial",
            "review:result-initial",
            "story-x:y",
        ];
        let backend = MemoryBackend::new();
        let report = from_manifest(
            &backend,
            &stories(&ids),
            "acme",
            "payments",
            Timestamp::from_epoch_millis(1_000),
            &actor(),
        )
        .unwrap_or_else(|error| panic!("every accepted id seeds: {error}"));

        assert_eq!(report.entities, ids.len());
        assert_eq!(backend.len(), ids.len(), "one entity per artifact");
        let identities: std::collections::BTreeSet<&EntityId> = report.by_id.values().collect();
        assert_eq!(identities.len(), ids.len(), "no two artifacts share one");
    }

    #[test]
    fn every_artifact_id_gets_its_own_command_identifier() {
        // Every id over an alphabet of one letter and each separator an id allows, up to three
        // characters a part: separators leading, trailing, repeated and mixed. A digit or a capital
        // is a letter to `token`, and three more letters only multiply the run time.
        let alphabet = ['a', '-', '_', '.', '/'];
        let mut parts = Vec::new();
        for length in 1..=3_u32 {
            for index in 0..alphabet.len().pow(length) {
                let mut rest = index;
                let mut part = String::new();
                for _ in 0..length {
                    part.push(alphabet[rest % alphabet.len()]);
                    rest /= alphabet.len();
                }
                parts.push(part);
            }
        }

        let mut seen: BTreeMap<String, ArtifactId> = BTreeMap::new();
        for namespace in &parts {
            for name in &parts {
                let id: ArtifactId = format!("{namespace}:{name}").parse().expect("artifact id");
                let name = format!("seed-{}", token(&id));
                assert!(CommandId::new(&name).is_ok(), "{id} gives `{name}`");
                if let Some(earlier) = seen.insert(name.clone(), id.clone()) {
                    panic!("{earlier} and {id} both give `{name}`");
                }
            }
        }
    }

    #[test]
    fn an_id_of_one_namespace_word_and_a_kebab_name_keeps_the_identifier_it_always_had() {
        for (id, expected) in [
            ("design:passkeys-auth", "design-passkeys-auth"),
            ("adr:0042", "adr-0042"),
            ("story:AUTH-141", "story-AUTH-141"),
        ] {
            assert_eq!(token(&id.parse().expect("artifact id")), expected, "{id}");
        }
    }

    #[test]
    fn relations_whose_parts_used_to_run_together_seed_as_two_edges() {
        // Both edges were `seed-rel-story-a-decomposes-story-b-specifies-story-c`.
        let graph = ArtifactGraph::build([
            artifact("story:a", ArtifactKind::Story, ArtifactStatus::Draft).with_relation(
                RelationKind::Decomposes,
                ArtifactRef::unpinned("story:b-specifies-story-c".parse().expect("id")),
            ),
            artifact(
                "story:a-decomposes-story-b",
                ArtifactKind::Story,
                ArtifactStatus::Draft,
            )
            .with_relation(
                RelationKind::Specifies,
                ArtifactRef::unpinned("story:c".parse().expect("id")),
            ),
            artifact(
                "story:b-specifies-story-c",
                ArtifactKind::Story,
                ArtifactStatus::Draft,
            ),
            artifact("story:c", ArtifactKind::Story, ArtifactStatus::Draft),
        ])
        .expect("the graph is valid");
        let backend = MemoryBackend::new();
        let report = from_manifest(
            &backend,
            &graph,
            "acme",
            "payments",
            Timestamp::from_epoch_millis(1_000),
            &actor(),
        )
        .unwrap_or_else(|error| panic!("both edges seed: {error}"));

        assert_eq!(report.relations, 2);
        assert_eq!(backend.with_store(|store| store.relations().count()), 2);
    }

    #[test]
    fn two_artifacts_at_one_address_are_refused_before_anything_is_written() {
        // A locator key cannot hold `/`, so `story:a/b` is addressed as `story/a-b`, where
        // `story:a-b` is too; and the address carries the kind, not the namespace, so `doc:x` of
        // kind `design` is where `design:x` is. Seeding used to create the first of each pair, then
        // refuse the second naming only the address.
        let mut graph = stories(&["story:a-b", "story:a/b"]);
        for id in ["design:x", "doc:x"] {
            graph.insert(artifact(id, ArtifactKind::Design, ArtifactStatus::Draft));
        }

        let backend = MemoryBackend::new();
        let error = from_manifest(
            &backend,
            &graph,
            "acme",
            "payments",
            Timestamp::from_epoch_millis(1_000),
            &actor(),
        )
        .expect_err("two artifacts cannot share one entity");
        assert_eq!(error.code(), "conflict");
        assert!(
            ["story:a-b", "story:a/b", "design:x", "doc:x"]
                .iter()
                .all(|id| error.to_string().contains(id)),
            "the refusal names every artifact involved: {error}"
        );
        assert_eq!(backend.len(), 0, "a refusal writes nothing");

        let found = collisions(&graph);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(
            found
                .iter()
                .any(|problem| problem.contains("`story:a-b` and `story:a/b`")
                    && problem.contains("one address, `story/a-b`")),
            "{found:?}"
        );
    }

    #[test]
    fn a_graph_whose_artifacts_have_their_own_addresses_has_no_collisions() {
        assert_eq!(collisions(&example()), Vec::<String>::new());
        assert_eq!(
            collisions(&stories(&[
                "story:render-a",
                "story:render_a",
                "story:render.a"
            ])),
            Vec::<String>::new()
        );
    }
}

#[cfg(test)]
mod linearity {
    use aep_domain::artifact::{ArtifactKind, ArtifactLocation, ArtifactRef, RelationKind};

    use super::*;
    use crate::MemoryBackend;

    /// Seeding copies the whole store zero times, however many artifacts there are. One copy per
    /// command is what made hydrating a plan quadratic in its size.
    #[test]
    fn seeding_does_not_copy_the_store_per_command() {
        let artifacts = (0..200).map(|index| {
            let artifact = Artifact::new(
                format!("task:t{index}").parse().expect("artifact id"),
                ArtifactKind::Task,
                aep_domain::artifact::ArtifactStatus::Draft,
                ArtifactLocation::Inline,
            );
            if index == 0 {
                artifact
            } else {
                artifact.with_relation(
                    RelationKind::DependsOn,
                    ArtifactRef::unpinned("task:t0".parse().expect("artifact id")),
                )
            }
        });
        let graph = ArtifactGraph::build(artifacts).expect("the graph is valid");
        let backend = MemoryBackend::new();
        let before = crate::store::store_clones();
        let report = from_manifest(
            &backend,
            &graph,
            "acme",
            "payments",
            Timestamp::from_epoch_millis(1_000),
            &"service:aep-cli".parse().expect("actor"),
        )
        .expect("the manifest seeds");
        assert_eq!(report.entities, 200);
        assert_eq!(report.relations, 199);
        assert_eq!(crate::store::store_clones(), before, "no whole-store copy");
    }
}
