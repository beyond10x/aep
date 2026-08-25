//! The status ladder, decided by `entity-core` instead of by a hand-written lookup.
//!
//! # What this is for
//!
//! `ArtifactStatus` is a ten-variant Rust enum, so every rung this protocol can express is fixed
//! at compile time in this repository. An adopter who needed `correction-owed` could not have it
//! without a release of ours — which is the meta-defect `story:open-vocabulary-audit` opened, read
//! from the far side. `entity-core` is an IO-free kernel that takes an entity type as **data** —
//! states, transitions, rules — and answers `definition + instance + operation → Decision`. Once
//! the ladder is data, a rung is a line in a YAML file.
//!
//! Phase 2 of `entity-runtime/docs/design/engineering-protocols-adoption-v0.1.md`, and it
//! deliberately changes **no verdict**: [`permits_transition`] answers exactly what
//! [`ArtifactLifecycle::permits_transition`] answers, for every kind in the store and every pair
//! of statuses. `tests/kernel_equivalence.rs` is that claim, over this repository's own planning
//! store. Nothing about the ladder is opened here; opening it is a later step, and it is a
//! different decision.
//!
//! # Operations are named for the target status, not for a verb
//!
//! `entity-core` moves an instance by *operation*, and our lifecycle documents declare target
//! states only — `protocol artifact move --to <TO> <ID>` never names a verb. So the definition
//! built here carries one operation per reachable status, named for that status. A verb vocabulary
//! is a published surface and would need its own decision; `story:entity-runtime-mapping` asks for
//! one, and until it has an answer this bridge introduces no name that is not already ours.
//!
//! # Why the translation is total
//!
//! Every refusal `entity-core` can produce for a move maps onto *the same* refusal this store
//! already gives, because the reasons line up one to one:
//!
//! | the store's reading | the kernel's refusal |
//! |---|---|
//! | `from` has no entry in `transitions` | `InvalidTransition` — no operation leaves that state |
//! | `to` is not among `from`'s targets | `InvalidTransition` — that operation does not leave this state |
//! | `to` is a status the ladder never reaches | `OperationNotFound` — no operation is named for it |
//! | the document claims a status the ladder does not declare | `UnknownState` |
//!
//! All four are *refused*, which is the only distinction a move makes, so the verdicts agree
//! whatever the reason. The kernel's richer error is not thrown away — it is simply not what
//! [`crate::document::MoveRefusal`] reports, because that type's job is to tell an author where they may go
//! instead.

use std::collections::BTreeMap;

use aep_domain::artifact::{ArtifactKind, ArtifactLifecycle, ArtifactStatus};
use aep_domain::evidence::EvidenceKind;
use entity_core::{CoreError, EntityDefinition, EntityInstance, Registry, Runtime};
use serde_json::json;

/// How many records of each kind are on hand for the artifact being moved.
///
/// The planning store holds no evidence — it holds markdown files with frontmatter — so this comes
/// from the caller, which is the same shape the kernel already demands of a clock: a value the
/// outside world knows enters as an argument (`entity-runtime` R-62) rather than being fetched
/// from inside a decision.
pub type EvidenceOnHand = BTreeMap<EvidenceKind, usize>;

/// What the kernel said about a move.
///
/// Three outcomes and not two, which is the whole of gap-register `:39`. *Nobody presented any
/// evidence* and *evidence was presented and there is not enough of it* send a person to different
/// places, and a store that reported both as "refused" would be the prose rule the register
/// complains about, wearing a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The move is permitted.
    Permitted,
    /// The ladder declares no such move from here.
    NotOnTheLadder,
    /// The ladder requires evidence and none was presented for it. Carries the addresses nobody
    /// looked at, verbatim from the kernel.
    EvidenceUnobserved {
        /// What the rule needed and could not read, such as `$args.evidence.test_result`.
        unobserved: Vec<String>,
        /// What the requirement said, for a person.
        message: String,
    },
    /// Evidence was presented and the requirement is not met.
    EvidenceInsufficient {
        /// What the requirement said, for a person.
        message: String,
    },
}

/// The identity the kernel is handed. It never reads it, and a refusal changes nothing, so one
/// fixed name for every decision is honest and allocation-free.
const SUBJECT: &str = "artifact";

/// The entity name for a kind that declares no lifecycle of its own.
const FALLBACK_ENTITY: &str = "artifact";

/// The lifecycle as an entity definition: states, initial status, one operation per reachable
/// status.
///
/// Built as **data** and parsed, rather than assembled from `entity-core`'s structs. That is the
/// interface an adopter actually has — a YAML or JSON document — so building it any other way
/// would test a path nobody uses, and it keeps this bridge insulated from the struct's field list.
///
/// The kind's name is the entity name, so a refusal names the ladder that refused. A kind that
/// declares no lifecycle is handed [`ArtifactLifecycle::permissive`] by the caller, exactly as
/// before, and permissive translates to every status reaching every status.
#[must_use]
pub fn definition_for(
    kind: Option<&ArtifactKind>,
    lifecycle: &ArtifactLifecycle,
) -> EntityDefinition {
    let states: Vec<String> = lifecycle
        .statuses()
        .iter()
        .map(|status| status.as_str().to_owned())
        .collect();

    // One operation per status something can move *to*, carrying every status it can be moved to
    // from. A status that is only ever a source gets no operation, which is what "nothing moves
    // there" means when the mover names its destination rather than a verb.
    // Keyed by the status itself, not by its name: the requirement lookup needs the status, and
    // parsing a name back into one would be a round trip that can fail for no reason.
    let mut sources: BTreeMap<&ArtifactStatus, Vec<&str>> = BTreeMap::new();
    for (from, targets) in &lifecycle.transitions {
        for to in targets {
            sources.entry(to).or_default().push(from.as_str());
        }
    }

    let operations: serde_json::Map<String, serde_json::Value> = sources
        .into_iter()
        .map(|(status, from)| {
            let to = status.as_str();
            // `evidence` is declared `json` so a path into it — `$args.evidence.test_result` — is
            // a run-time question rather than a registration-time one, which is what lets a
            // requirement name a kind without this definition enumerating every kind that exists.
            let mut operation = json!({
                "arguments": { "fields": { "evidence": { "type": "json" } } },
                "transitions": [ { "from": from, "to": to } ],
            });
            let requirements: Vec<serde_json::Value> = lifecycle
                .requirements_for(status)
                .iter()
                .map(|requirement| {
                    let kind = requirement.evidence.as_str();
                    let at_least = requirement.at_least;
                    json!({
                        "name": format!("evidence: {kind}"),
                        "message": format!("reaching {to} needs at least {at_least} {kind} record(s)"),
                        "assert": { "gte": [format!("$args.evidence.{kind}"), at_least] },
                    })
                })
                .collect();
            if !requirements.is_empty() {
                operation["preconditions"] = serde_json::Value::Array(requirements);
            }
            (to.to_owned(), operation)
        })
        .collect();

    // A blank name would be refused at registration, and refusing every move for a kind whose
    // name is odd is a verdict change; the fallback carries those.
    let entity = kind
        .map(ArtifactKind::as_str)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(FALLBACK_ENTITY);

    let document = json!({
        "entity": entity,
        "version": 1,
        "schema": {},
        "lifecycle": { "initial": lifecycle.initial.as_str(), "states": states },
        "operations": operations,
    });

    serde_json::from_value(document)
        .unwrap_or_else(|error| panic!("a lifecycle is a well-formed entity definition: {error}"))
}

/// Whether the kernel permits the move.
///
/// The same answer as [`ArtifactLifecycle::permits_transition`], reached by executing the move
/// against a definition built from the same lifecycle. `entity-core` is pure and a refusal changes
/// nothing, so *attempting* the move is the safe way to ask whether it is permitted — there is no
/// dry-run verb to need, and no state to undo when the answer is no.
///
/// A lifecycle that produces a definition the kernel refuses to register answers `false`. That is
/// unreachable for a lifecycle the document tree parsed — the equivalence test registers every one
/// of them — and "not permitted" is the only safe reading of a ladder nobody can evaluate.
#[must_use]
pub fn permits_transition(
    kind: Option<&ArtifactKind>,
    lifecycle: &ArtifactLifecycle,
    from: &ArtifactStatus,
    to: &ArtifactStatus,
) -> bool {
    decide(kind, lifecycle, from, to, &EvidenceOnHand::new()) == Verdict::Permitted
}

/// Whether the kernel permits the move, and if not, which of the three reasons.
///
/// `entity-core` is pure and a refusal changes nothing, so *attempting* the move is the safe way to
/// ask — there is no dry-run verb to need, and no state to undo when the answer is no.
///
/// A ladder that declares no `requires` cannot reach the two evidence verdicts, whatever is passed
/// here: with no precondition to evaluate there is nothing for evidence to satisfy. That is what
/// keeps [`permits_transition`] above a faithful shorthand and what keeps
/// `tests/kernel_equivalence.rs` meaningful.
#[must_use]
pub fn decide(
    kind: Option<&ArtifactKind>,
    lifecycle: &ArtifactLifecycle,
    from: &ArtifactStatus,
    to: &ArtifactStatus,
    evidence: &EvidenceOnHand,
) -> Verdict {
    let definition = definition_for(kind, lifecycle);
    let entity = definition.entity.clone();
    let mut registry = Registry::new();
    if registry.register(definition).is_err() {
        return Verdict::NotOnTheLadder;
    }

    let instance = EntityInstance {
        entity,
        version: 1,
        id: SUBJECT.to_owned(),
        lifecycle_state: from.as_str().to_owned(),
        revision: 1,
        fields: serde_json::Map::new(),
    };
    let counted: serde_json::Map<String, serde_json::Value> = evidence
        .iter()
        .map(|(kind, count)| (kind.as_str().to_owned(), json!(count)))
        .collect();

    match Runtime::new(&registry).execute(
        &instance,
        to.as_str(),
        json!({ "evidence": serde_json::Value::Object(counted) }),
    ) {
        Ok(_) => Verdict::Permitted,
        // The two the whole of gap-register :39 turns on. `Unobservable` means the rule read an
        // address nothing supplied — nobody presented evidence of that kind — and `Failed` means
        // somebody did and there is not enough of it.
        Err(CoreError::PreconditionUnobservable {
            message,
            unresolved,
            ..
        }) => Verdict::EvidenceUnobserved {
            unobserved: unresolved,
            message,
        },
        Err(CoreError::PreconditionFailed { message, .. }) => {
            Verdict::EvidenceInsufficient { message }
        }
        Err(_) => Verdict::NotOnTheLadder,
    }
}
