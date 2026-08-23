//! `protocol contract evidence` — a contract runner's own record, given the envelope the engine
//! reads.
//!
//! The fifth module split, on the criterion the first four took: a verb family with its own input,
//! its own vocabulary and no shared state with the rest of the binary.
//!
//! # The seam this closes
//!
//! metaharness — private, and no dependency of this workspace — contract-tests each
//! `metaharness ⇄ vendor` adapter and prints the outcome as **one JSON object in the
//! `contract_result` shape this repository defines** (`metaharness conformance <kind> --contract`;
//! its `docs/design/adapter-contract-v0.1.md` § *Reuse of engineering-protocols' tooling*, read
//! 2026-08-23). The intent stated there is that *"an EP-driven eval, or any consumer, reads an
//! adapter's conformance as a `contract_result` without knowing anything about metaharness's
//! internals"*.
//!
//! Until this verb existed, nothing here read one. The vocabulary crossed the boundary and no bytes
//! ever did — which is the same failure mode the frame document had before
//! `tests/metaharness_frame_contract.rs`, and the mirror image of it: that seam is *what this
//! repository mints, read by their rules*, and this one is *what they mint, read by ours*.
//!
//! # Why there is a verb at all, when the record already parses
//!
//! It parses. `Evidence` is tagged by `kind`, [`ContractResult`] is the payload, and
//! `{"kind":"contract_result","checked":20,…}` deserialises into `Evidence::ContractResult` with no
//! adapter, no second type and no new evidence kind. `protocols/aep/1.yaml` has declared
//! `contract_result`, the `contract-runner` verifier and `contracts.**` since the base protocol was
//! written, so no document changes either. That is the whole of what the shared vocabulary bought,
//! and it is most of the work.
//!
//! What a record on standard output does **not** carry is the *envelope* an evidence document
//! needs: `observed_at` and `producer`. `protocol evaluate --evidence` reads a list of records that
//! each state when somebody looked and what produced them, and the runner's object states neither.
//! So this verb supplies exactly those two fields and nothing else — it does not compute a verdict,
//! does not touch a count, and does not add a fact.
//!
//! # The producer is a constant, and the observation time is required
//!
//! The producer is [`PRODUCER`], the only class `EvidenceKind::ContractResult` names, for the same
//! reason `TraceEvidence::PRODUCER` is a constant: a caller that could name itself the verifier
//! would make the record's independence an input to the record.
//!
//! `--observed-at` is **required**, and that is the one place this verb is stricter than
//! `protocol trace evidence`. That verb defaults to now because the check runs in that process, in
//! that second. Here the check ran in another process, on another machine, possibly last week, and
//! the record carries no time of its own — so a default of *now* would be this binary stamping a
//! freshness it did not witness, and evidence horizons exist precisely to catch that (invariant 7:
//! a caller who has to write down when they looked cannot back-date by omission).
//!
//! # Two refusals, and why they are here rather than left to the engine
//!
//! `principles/development/contract-testing.yaml` states the discipline: *a run that checked
//! nothing also has zero failures, so the number of checks is part of the obligation.* It spells it
//! as `contracts.checked > 0`, and one might therefore expect a zero-checked record to be minted and
//! left for the engine to fail. Measured, that is not what happens — a `contract_result` with
//! `checked: 0` submitted against `examples/billing-conformance` reads:
//!
//! ```text
//! ✗ contracts.checked > 0                                     [principle contract-testing]
//! ✓ contracts.failed == 0                                     [principle contract-testing]
//! ✓ contracts.breaking_changes == 0                           [principle contract-testing]
//! ✓ evidence contract_result from contract-runner (independent)
//! ✓ contract-runner must run
//! ```
//!
//! Two of the three predicates pass *vacuously*, and — the part that matters — the **evidence
//! obligation is discharged**: the task now has its independent contract record and `contract-runner`
//! has run, on the strength of a run that checked nothing. So the refusal belongs at the boundary
//! where the record enters, not one layer later.
//!
//! The second refusal is `breaking_changes > failed`. A breaking change is a *subset* of the
//! failures on the producer's own definition, so a record claiming more of the subset than of the
//! set describes no run that could have happened; left alone it would let `contracts.failed == 0`
//! and `contracts.breaking_changes == 0` contradict each other inside one record.
//!
//! What is **not** refused is bad news. A record with `failed: 3` is minted without complaint and
//! exits `0`, exactly as `protocol trace evidence` writes down a run that gapped: the verdict belongs
//! in the record, and the engine is what decides on it. The two refusals are about a record that
//! *asserts nothing* and a record that *cannot describe any run* — never about one that reports a
//! failure.
//!
//! # What this does not buy
//!
//! Attestation. The record is YAML by the time the engine reads it, a person can type one, and the
//! digest in its provenance is a digest of bytes this process was handed rather than of bytes it
//! watched being produced. The same limit `trace-spec` and `ess-conformance` state about their own
//! records, and the same gap `docs/VISION.md` names.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_domain::evidence::{ContractResult, Evidence, Producer, Provenance};
use aep_domain::time::ObservedAt;
use aep_domain::verification::Verifier;
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use sha2::{Digest as _, Sha256};

use crate::Format;

/// The only producer this verb ever stamps: a contract runner, as a verifier.
///
/// A constant rather than an argument, for the reason the [module documentation](self) gives, and
/// the class is not a free choice — `EvidenceKind::ContractResult::default_verifiers` names
/// `contract-runner` and nothing else, and `principles/development/contract-testing.yaml` asks for
/// evidence of that kind from that verifier, marked `independent: true`.
const PRODUCER: Producer = Producer::Verifier {
    verifier: Verifier::ContractRunner,
};

/// What can be done with a contract runner's record.
///
/// One subcommand today, and a family rather than a bare verb because the shape this repository
/// keeps arriving at is `<domain> evidence` — `ess conform evidence`, `trace evidence`, and now
/// this — each minting the one record its domain can establish.
///
/// **This `contract` is the consumer/provider kind**, the one
/// `principles/development/contract-testing.yaml` governs: *does the published interface still
/// behave as its consumers were told?* It is not `aep-contract`, the storage and interaction
/// contract a backend implements — that question is `protocol conformance`, and the only thing the
/// two share is the word.
#[derive(Debug, Subcommand)]
pub(crate) enum ContractCommand {
    /// Read a contract runner's `contract_result` record and write the AEP evidence document it
    /// implies.
    ///
    /// The join the metaharness adapter contract exists for. The runner prints its record;
    /// redirect it to a file and hand the file here, and what comes back is a document
    /// `protocol evaluate --evidence` reads directly, carrying the counts the runner measured,
    /// `producer: verifier / contract-runner`, and the digest of the bytes it was given.
    ///
    /// Nothing about the record is computed here. This verb adds an envelope and refuses two kinds
    /// of record that would assert something nobody measured; every number in the document is the
    /// runner's own.
    Evidence(EvidenceArgs),
}

/// The arguments of `protocol contract evidence`.
#[derive(Debug, Args)]
pub(crate) struct EvidenceArgs {
    /// The record the contract runner emitted: one JSON object in the `contract_result` shape.
    ///
    /// A path rather than standard input, so that the bytes the evidence document's provenance
    /// digests are bytes that exist somewhere a later reader can go and check. Callers redirect:
    /// `metaharness conformance claude --contract > claude.json`.
    #[arg(long)]
    record: PathBuf,
    /// Where to write the document. Without it, it goes to standard output.
    #[arg(long)]
    out: Option<PathBuf>,
    /// How to write it. Both are read by `protocol evaluate --evidence`.
    #[arg(long, value_enum, default_value_t = Format::Yaml)]
    format: Format,
    /// When the contract run was made, as a date or epoch milliseconds.
    ///
    /// Required, unlike `protocol trace evidence`'s. See the module documentation: this process did
    /// not watch the run, the record carries no time of its own, and a default of *now* would be a
    /// freshness claim nobody made.
    #[arg(long, value_name = "DATE")]
    observed_at: String,
}

/// An evidence record, with the envelope this verb supplies.
///
/// Serialises as one entry of the document `protocol evaluate --evidence` reads: the payload's own
/// fields under `kind: contract_result`, beside `observed_at`, `producer` and `provenance`. The
/// producer is not a field the caller can reach — it is [`PRODUCER`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct ContractEvidence {
    /// The observation, tagged `kind: contract_result`.
    #[serde(flatten)]
    evidence: Evidence,
    /// When the contract run was made. The caller's, because this process did not watch it.
    observed_at: ObservedAt,
    /// What produced it: always [`PRODUCER`].
    producer: Producer,
    /// How it was obtained: the command, the file, and the digest of the bytes in it.
    provenance: Provenance,
}

/// The `contract` verb family, one arm per subcommand.
pub(crate) fn run(command: ContractCommand) -> Result<ExitCode> {
    match command {
        ContractCommand::Evidence(args) => mint_evidence(&args),
    }
}

/// The record, read through the rules that decide whether it says anything.
///
/// Three refusals, first one wins, each naming what is wrong rather than reporting *an error*: a
/// document that is not this shape, a run that checked nothing, and counts that describe no run.
fn read_record(text: &str) -> Result<ContractResult> {
    let evidence: Evidence = serde_json::from_str(text).context(
        "expected one JSON object in the `contract_result` shape — \
         {kind, checked, failed, breaking_changes, provider, consumer}",
    )?;
    let Evidence::ContractResult(result) = evidence else {
        bail!(
            "this is a `{}` record and this verb reads a `contract_result`. The kind is the \
             document's own `kind` field, so a runner that emitted the wrong one says so here \
             rather than having its counts read as contracts",
            evidence.kind()
        );
    };

    if result.checked == 0 {
        bail!(
            "the record states `checked: 0`, so it asserts nothing: a run that checked nothing \
             also has zero failures and zero breaking changes. Minting it would discharge the \
             evidence obligation `principles/development/contract-testing.yaml` places on a task \
             — an independent `contract_result` from a `contract-runner` — while two of that \
             principle's three predicates passed vacuously. Run the contract vectors, or record \
             nothing"
        );
    }

    if result.breaking_changes > result.failed {
        bail!(
            "the record states {} breaking change(s) out of {} failure(s), and a breaking change \
             is a failure: no run produced these counts. Left alone the record would report \
             `contracts.failed == 0` and `contracts.breaking_changes == 0` contradicting each \
             other about the same run",
            result.breaking_changes,
            result.failed
        );
    }

    Ok(result)
}

/// `protocol contract evidence`
fn mint_evidence(args: &EvidenceArgs) -> Result<ExitCode> {
    let text = std::fs::read_to_string(&args.record)
        .with_context(|| format!("reading the record at {}", args.record.display()))?;
    let result = read_record(&text)
        .with_context(|| format!("{} is not a usable contract record", args.record.display()))?;

    let record = ContractEvidence {
        provenance: Provenance {
            command: Some(invocation(args)),
            digest: Some(digest(text.as_bytes())),
            inputs: vec![args.record.display().to_string()],
            ..Provenance::default()
        },
        evidence: Evidence::ContractResult(result),
        observed_at: crate::observation_time(Some(&args.observed_at))?,
        producer: PRODUCER,
    };

    // A list of one, because that is the shape `--evidence` reads: a file holding several records
    // and a file holding one are the same document, and a bare record would be a second shape to
    // support.
    let document = match args.format {
        // There is no second rendering of an evidence record. `text` gets the document too, rather
        // than a summary a person might paste into a file and find the engine will not read.
        Format::Json => serde_json::to_string_pretty(&[&record])
            .map(|mut json| {
                json.push('\n');
                json
            })
            .context("rendering the evidence record")?,
        Format::Text | Format::Yaml => {
            serde_yaml::to_string(&[&record]).context("rendering the evidence record")?
        }
    };

    match &args.out {
        Some(file) => {
            write_beside(file, &document)?;
            let summary = record.evidence.summary();
            outln!("{} — {summary}", file.display());
        }
        None => out!("{document}"),
    }

    Ok(ExitCode::SUCCESS)
}

/// Writes the document, creating the directory it was asked for.
fn write_beside(file: &Path, document: &str) -> Result<()> {
    if let Some(parent) = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(file, document).with_context(|| format!("writing {}", file.display()))
}

/// SHA-256 of the bytes as read, hex, for [`Provenance::digest`].
///
/// Of the raw record and not of the evidence document: what a later reader wants to check is that
/// the counts in the document are the counts the runner printed, and the only artifact that can
/// answer is the runner's own output.
fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// The command line, as the record's provenance reports it.
///
/// Reconstructed rather than read from `std::env::args`, so the record says what was *asked for* in
/// the vocabulary of this verb rather than however the caller's shell spelled it.
fn invocation(args: &EvidenceArgs) -> String {
    format!(
        "protocol contract evidence --record {} --observed-at {}",
        args.record.display(),
        args.observed_at
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The provider's own bytes, captured 2026-08-23 from `metaharness conformance claude
    /// --contract`. The committed fixture, read as bytes so a test binary run from anywhere reads
    /// the same document.
    const CLAUDE: &str = include_str!("../fixtures/metaharness-contract-result-claude.json");

    /// The same, for `metaharness conformance codex --contract`.
    const CODEX: &str = include_str!("../fixtures/metaharness-contract-result-codex.json");

    /// The claim the shared vocabulary was for: the runner's record *is* the payload, with no
    /// adapter in between.
    ///
    /// Asserted on the committed bytes rather than on a value built here, because a test that
    /// constructed a [`ContractResult`] and serialised it would be comparing this crate with
    /// itself.
    #[test]
    fn the_providers_own_bytes_are_the_payload_this_repository_defines() {
        let claude = read_record(CLAUDE).expect("the captured claude record is read");
        assert_eq!(claude.checked, 20);
        assert_eq!(claude.failed, 0);
        assert_eq!(claude.breaking_changes, 0);
        assert_eq!(claude.provider.as_deref(), Some("claude 2.1.239"));
        assert_eq!(claude.consumer.as_deref(), Some("metaharness.event/1"));

        let codex = read_record(CODEX).expect("the captured codex record is read");
        assert_eq!(codex.checked, 10);
        assert_eq!(codex.provider.as_deref(), Some("codex 0.145.0"));
    }

    /// `checked: 0` is refused, and the refusal says why rather than reporting a parse failure.
    ///
    /// The mutation is the smallest one that reaches the rule: the captured record with its one
    /// count zeroed, everything else the provider's.
    #[test]
    fn a_record_that_checked_nothing_is_refused_with_the_reason_named() {
        let empty = CLAUDE.replace("\"checked\":20", "\"checked\":0");
        assert_ne!(empty, CLAUDE, "the mutation reached the document");

        let refusal = read_record(&empty)
            .expect_err("a run that checked nothing must not become evidence")
            .to_string();
        assert!(
            refusal.contains("asserts nothing"),
            "the refusal names the reason: {refusal}"
        );
        assert!(
            refusal.contains("contract-testing"),
            "and cites the principle that states the discipline: {refusal}"
        );
    }

    /// `breaking_changes > failed` is refused: a breaking change is one of the failures.
    #[test]
    fn a_record_claiming_more_breaking_changes_than_failures_is_refused() {
        let impossible = CODEX
            .replace("\"breaking_changes\":0", "\"breaking_changes\":2")
            .replace("\"failed\":0", "\"failed\":1");

        let refusal = read_record(&impossible)
            .expect_err("counts that describe no run are refused")
            .to_string();
        assert!(
            refusal.contains("2 breaking change(s) out of 1 failure(s)"),
            "the refusal quotes both counts, so the record can be fixed: {refusal}"
        );
    }

    /// The boundary of the second rule: equal counts are legal, because every failure may be the
    /// vendor's.
    ///
    /// Without this the rule could have been written `>=` and every test above would still pass.
    #[test]
    fn a_record_whose_failures_are_all_breaking_is_accepted() {
        let all_vendor = CODEX
            .replace("\"breaking_changes\":0", "\"breaking_changes\":3")
            .replace("\"failed\":0", "\"failed\":3");
        let result = read_record(&all_vendor).expect("breaking == failed is a run that happened");
        assert_eq!(result.breaking_changes, result.failed);
    }

    /// Bad news is written down, not refused. A red contract run is the case evidence exists for.
    #[test]
    fn a_failing_contract_run_is_read_rather_than_refused() {
        let red = CLAUDE.replace("\"failed\":0", "\"failed\":4");
        let result = read_record(&red).expect("a failing run still produced a record");
        assert_eq!(result.failed, 4);
        assert_eq!(
            result.breaking_changes, 0,
            "four failures, none of them the vendor's — the C3 case the provider's design names"
        );
    }

    /// A record of another kind is refused by name, not read for whatever counts it happens to
    /// carry.
    #[test]
    fn a_record_of_another_kind_is_refused_by_the_kind_it_states() {
        let refusal = read_record(r#"{"kind":"static_analysis","errors":0,"warnings":1}"#)
            .expect_err("a static analysis record is not a contract record")
            .to_string();
        assert!(
            refusal.contains("`static_analysis`"),
            "the refusal names what it was handed: {refusal}"
        );
    }

    /// The digest is over the bytes as read, so the provenance names the runner's output and not a
    /// re-rendering of it.
    #[test]
    fn the_provenance_digest_describes_the_bytes_the_runner_printed() {
        let expected = digest(CLAUDE.as_bytes());
        assert_eq!(expected.len(), 64, "sha-256, hex: {expected}");
        assert_ne!(
            expected,
            digest(CODEX.as_bytes()),
            "two different records do not share a digest"
        );
    }
}
