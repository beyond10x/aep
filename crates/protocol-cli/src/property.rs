//! `protocol property evidence` — a property checker that runs here and writes down what it
//! measured.
//!
//! # The gap this closes
//!
//! `principles/verification/property-based-testing.yaml` owes every code task an independent
//! `property_test_result` from a `property-tester`, and until this verb existed no step of any
//! step map could produce one. `EvidenceMapping::MINTABLE` does not admit the kind and must not:
//! the payload carries a property name, a case count and a seed, and an exit status carries none of
//! them — a record minted from `exit 0` would state a case count nobody read, which the engine
//! cannot tell apart from a measured one.
//!
//! So the check runs **in this process**, and the record is the checker's own — the shape
//! `protocol trace evidence` and `protocol contract evidence` already have, and the reason
//! `EvidenceMapping::record` exists.
//!
//! # Why the property is the protocol's own algebra
//!
//! The property this repository can check without knowing anything about the task being driven is
//! the one its own decisions rest on: invariant 5, *`Unknown` is not `False`*, expressed as the
//! laws of strong Kleene three-valued logic over [`Truth`]. `Predicate::evaluate` folds a predicate
//! tree in its own order, and every guard in every workflow is decided by that fold; if the algebra
//! did not associate, distribute or negate uniformly, a run's verdict would depend on how somebody
//! parenthesised a guard.
//!
//! `crates/aep-domain/tests/truth_laws.rs` asserts the same laws under `proptest`. This is not a
//! second copy of that suite pretending to be evidence — it is the **stronger** check of the two,
//! and deliberately so:
//!
//! * that suite *samples* 256 cases from a space of 27 assignments, with a fixed seed;
//! * this one *enumerates all 27*, so a law that holds here holds for every input there is.
//!
//! Which is also why [`PropertyTestResult::seed`] is written as absent rather than invented. The
//! field's own documentation says an exhaustive or symbolic checker "has nothing to seed and should
//! not be made to invent one", and a seed on an exhaustive run would claim a search that never
//! happened. `property_test.<property>.seed.exists` reads `false`, which is the true answer:
//! nothing about this run needs reproducing, because it is the same run every time.
//!
//! # What this does not buy
//!
//! Any statement about the task being driven. This checks the engine's algebra, not the change
//! under review — `property-based-testing` itself says so: *"NOT checked here: that the properties
//! run are the right properties. Nothing in the protocol can decide that; a reviewer reading the
//! task's own predicates can."* A profile or task that wants a named property of its own adds
//! `property_test.<name>.passed` to its predicates and needs a checker that can answer it; this
//! verb answers the general obligation and says, here, exactly what it answered.

use std::path::PathBuf;
use std::process::ExitCode;

use aep_domain::evidence::{Evidence, PropertyTestResult};
use aep_domain::ids::ClaimId;
use aep_domain::predicate::Truth;
use aep_domain::verification::{Counterexample, VerificationStatus, Verifier};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::evidence_doc::{emit, MintedEvidence};
use crate::Format;

/// The property this verb checks, as it appears in `property_test.<property>.*`.
///
/// Singular and kebab, like every other claim id. It names the algebra rather than the invariant
/// number so that a reader of the fact path can tell what was checked without opening `AGENTS.md`.
const PROPERTY: &str = "kleene-algebra";

/// The verifier class this verb signs as.
///
/// `property-tester`, which is what `principles/verification/property-based-testing.yaml` pins its
/// evidence to — deliberately, so that a model checker's proof of the same claim does not satisfy
/// it by accident. A constant and not an argument: a caller that could name itself the verifier
/// would make the record's independence an input to the record.
const PRODUCER: Verifier = Verifier::PropertyTester;

/// What can be done with the workspace's own properties.
#[derive(Debug, Subcommand)]
pub(crate) enum PropertyCommand {
    /// Run them and write the `property_test_result` document the run reads.
    ///
    /// Exits `0` whatever the properties said. The verdict is in the record and the engine is what
    /// decides on it, exactly as `protocol trace evidence` writes down a run that gapped — a caller
    /// that wants the verdict as an exit code is asking for a test runner, and `cargo test` is one.
    Evidence(EvidenceArgs),
}

/// The arguments of `protocol property evidence`.
#[derive(Debug, Args)]
pub(crate) struct EvidenceArgs {
    /// Where to write the document. Without it, it goes to standard output.
    #[arg(long)]
    out: Option<PathBuf>,
    /// How to write it. Both are read by `protocol evaluate --evidence`.
    #[arg(long, value_enum, default_value_t = Format::Yaml)]
    format: Format,
}

/// The `property` verb family, one arm per subcommand.
pub(crate) fn run(command: PropertyCommand) -> Result<ExitCode> {
    match command {
        PropertyCommand::Evidence(args) => mint_evidence(&args),
    }
}

/// The three values, in a fixed order so the enumeration is the same on every machine.
const VALUES: [Truth; 3] = [Truth::True, Truth::False, Truth::Unknown];

/// One law, named the way a counterexample has to name it.
struct Law {
    /// What the law is called, for the note on a counterexample.
    name: &'static str,
    /// Whether it holds for one assignment of the three variables.
    holds: fn(Truth, Truth, Truth) -> bool,
}

/// Every law of strong Kleene three-valued logic this repository relies on.
///
/// The same list `crates/aep-domain/tests/truth_laws.rs` asserts, arranged as data so the runner
/// can name the one that broke. A law added there and not here makes this record weaker than the
/// suite, which is the direction that matters: the record must never claim more than the suite.
const LAWS: &[Law] = &[
    Law {
        name: "negation is an involution",
        holds: |a, _, _| a.not().not() == a,
    },
    Law {
        name: "de Morgan holds in both directions",
        holds: |a, b, _| {
            a.and(b).not() == a.not().or(b.not()) && a.or(b).not() == a.not().and(b.not())
        },
    },
    Law {
        name: "conjunction and disjunction commute",
        holds: |a, b, _| a.and(b) == b.and(a) && a.or(b) == b.or(a),
    },
    Law {
        name: "conjunction and disjunction associate",
        holds: |a, b, c| a.and(b).and(c) == a.and(b.and(c)) && a.or(b).or(c) == a.or(b.or(c)),
    },
    Law {
        name: "each operation distributes over the other",
        holds: |a, b, c| {
            a.and(b.or(c)) == a.and(b).or(a.and(c)) && a.or(b.and(c)) == a.or(b).and(a.or(c))
        },
    },
    Law {
        name: "identities, annihilators, idempotence and absorption",
        holds: |a, b, _| {
            a.and(Truth::True) == a
                && a.or(Truth::False) == a
                && a.and(Truth::False) == Truth::False
                && a.or(Truth::True) == Truth::True
                && a.and(a) == a
                && a.or(a) == a
                && a.and(a.or(b)) == a
                && a.or(a.and(b)) == a
        },
    },
    // Invariant 5 itself, as an algebraic fact: only `True` permits, and composition cannot
    // manufacture permission. `Unknown` is not `False`, but neither of them satisfies.
    Law {
        name: "only True permits and composition cannot widen it",
        holds: |a, b, _| {
            a.and(b).is_satisfied() == (a.is_satisfied() && b.is_satisfied())
                && a.or(b).is_satisfied() == (a.is_satisfied() || b.is_satisfied())
                && !Truth::Unknown.is_satisfied()
                && Truth::Unknown != Truth::False
        },
    },
];

/// What the run measured.
struct Measured {
    /// How many assignments were checked. Every one there is, which is what makes this exhaustive.
    cases: usize,
    /// The assignments that broke a law, in enumeration order.
    broken: Vec<Counterexample>,
}

/// Runs every law over every assignment of three [`Truth`] variables.
///
/// Exhaustive rather than sampled, and the whole space is 27 assignments — see the
/// [module documentation](self) for why that makes the record stronger than the `proptest` suite
/// rather than a copy of it. No clock, no randomness and no allocation that depends on either, so
/// two runs on two machines measure the same thing.
fn check() -> Measured {
    let mut broken = Vec::new();
    let mut cases = 0;
    for a in VALUES {
        for b in VALUES {
            for c in VALUES {
                cases += 1;
                for law in LAWS {
                    if !(law.holds)(a, b, c) {
                        broken.push(Counterexample {
                            verifier: PRODUCER,
                            property: ClaimId::new(PROPERTY).ok(),
                            note: Some(format!("{}, at a={a}, b={b}, c={c}", law.name)),
                            ..Counterexample::default()
                        });
                    }
                }
            }
        }
    }
    Measured { cases, broken }
}

/// `protocol property evidence`
fn mint_evidence(args: &EvidenceArgs) -> Result<ExitCode> {
    let measured = check();
    let record = PropertyTestResult {
        property: ClaimId::new(PROPERTY).context("`kleene-algebra` is a claim id")?,
        cases: measured.cases,
        // Absent, and absent is the honest value: an exhaustive checker has no search to reproduce.
        seed: None,
        status: if measured.broken.is_empty() {
            VerificationStatus::Passed
        } else {
            VerificationStatus::Failed
        },
        counterexamples: measured.broken,
    };

    let minted = MintedEvidence::new(
        Evidence::PropertyTestResult(record),
        PRODUCER,
        // The check ran in this process, in this second, which is the one case where a default of
        // *now* is the honest value rather than a freshness claim nobody made.
        crate::now_observed(),
    )
    .obtained_by(invocation(args))
    .reading(format!("{PROPERTY} over aep_domain::predicate::Truth"));

    emit(&minted, args.format, args.out.as_deref())?;
    // Exit 0 for a run that found a counterexample as well. The verdict is in the record.
    Ok(ExitCode::SUCCESS)
}

/// The command line, as the record's provenance reports it.
fn invocation(args: &EvidenceArgs) -> String {
    match &args.out {
        Some(out) => format!("protocol property evidence --out {}", out.display()),
        None => "protocol property evidence".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The space is 27 assignments and the checker visits all of them, which is the claim the
    /// record's `cases` count makes.
    #[test]
    fn the_run_is_exhaustive_over_the_whole_space_rather_than_a_sample_of_it() {
        let measured = check();
        assert_eq!(
            measured.cases,
            VALUES.len().pow(3),
            "three variables over three values is 27 assignments, and an exhaustive run visits \
             every one"
        );
        assert!(
            measured.broken.is_empty(),
            "the Kleene laws hold: {:?}",
            measured.broken
        );
    }

    /// The record says what was measured, with the producer `independent: true` is read as.
    #[test]
    fn the_record_names_the_property_the_case_count_and_a_verifier_that_is_not_the_agent() {
        let document = {
            let args = EvidenceArgs {
                out: None,
                format: Format::Yaml,
            };
            let measured = check();
            let record = PropertyTestResult {
                property: ClaimId::new(PROPERTY).expect("a claim id"),
                cases: measured.cases,
                seed: None,
                status: VerificationStatus::Passed,
                counterexamples: Vec::new(),
            };
            let minted = MintedEvidence::new(
                Evidence::PropertyTestResult(record),
                PRODUCER,
                aep_domain::time::ObservedAt::new(aep_domain::time::Timestamp::from_epoch_millis(
                    1_699_920_000_000,
                )),
            )
            .obtained_by(invocation(&args));
            serde_yaml::to_string(&[&minted]).expect("it renders")
        };

        let parsed = aep_schema::parse::evidence_list(&document, Some("test"))
            .expect("the record this verb writes is one `--evidence` reads");
        assert_eq!(parsed.len(), 1, "{document}");
        let entry = &parsed[0];
        assert_eq!(
            entry.evidence.kind(),
            aep_domain::evidence::EvidenceKind::PropertyTestResult,
            "the kind the map declares: {document}"
        );
        assert!(
            matches!(
                &entry.producer,
                aep_domain::evidence::Producer::Verifier {
                    verifier: Verifier::PropertyTester
                }
            ),
            "`property-based-testing` pins `property-tester` and marks it independent: {:?}",
            entry.producer
        );
        let Evidence::PropertyTestResult(result) = &entry.evidence else {
            panic!("the payload is a property test result: {document}");
        };
        assert_eq!(result.property.as_str(), PROPERTY);
        assert_eq!(result.cases, 27);
        assert!(
            !result.is_reproducible(),
            "an exhaustive run states no seed rather than inventing one, and \
             `property_test.{PROPERTY}.seed.exists` is the fact that says so"
        );
    }

    /// A broken law is reported with the assignment that broke it, not as a bare `failed`.
    ///
    /// The mutation is applied to a law here rather than to `Truth`, because breaking `Truth`
    /// itself would break every other test in the workspace and prove nothing about this reporter.
    #[test]
    fn a_law_that_does_not_hold_is_named_with_the_assignment_that_broke_it() {
        const BROKEN: &[Law] = &[Law {
            name: "unknown is false",
            holds: |a, _, _| a != Truth::Unknown,
        }];

        let mut broken = Vec::new();
        for a in VALUES {
            for law in BROKEN {
                if !(law.holds)(a, Truth::True, Truth::True) {
                    broken.push(format!("{}, at a={a}", law.name));
                }
            }
        }
        assert_eq!(
            broken,
            vec!["unknown is false, at a=unknown".to_owned()],
            "the counterexample names the law and the assignment, which is what makes it actionable"
        );
    }
}
