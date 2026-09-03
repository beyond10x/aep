//! The envelope a verifier's own record needs, and the one way this binary writes one down.
//!
//! # Why this is a module and not a fourth copy
//!
//! Three verbs already mint an evidence document — `protocol ess conform evidence`,
//! `protocol trace evidence` and `protocol contract evidence` — and each carries its own private
//! envelope struct and its own `match format { … }`. That was two copies of a small thing and is
//! defensible; `story:evidence-producers-for-the-driven-map` adds three more producers, and six
//! copies of *what an evidence document looks like* is how two of them come to disagree about
//! whether the list wrapper is optional or whether `text` prints a summary the engine cannot read.
//!
//! So the shape is stated once here and the new producers use it. The three older verbs are
//! deliberately **not** migrated: their records are a published surface, and rewriting the byte
//! path that produces committed conformance evidence is a change to what is committed, not a
//! refactor. What this module buys is that every producer added from now on writes the same
//! document.
//!
//! # What the envelope is, and what it is not
//!
//! A record on its own states a payload. What `protocol evaluate --evidence` reads is a payload
//! **plus** when somebody looked, what produced it, and how — invariant 7's split, where the
//! payload is the verifier's and the envelope is the caller's. This module supplies the envelope
//! and refuses to compute any part of the payload.
//!
//! The producer is always a [`Producer::Verifier`]. There is no argument for it and there will not
//! be one: a producer a caller could choose is a producer an agent could claim, and
//! `independent: true` — which `provenance-tracking`, `property-based-testing` and `test-driven`
//! all ask for — is exactly the assertion that the record's producer was not the party that wanted
//! the claim to hold.

use std::path::Path;

use aep_domain::evidence::{Evidence, Producer, Provenance};
use aep_domain::time::ObservedAt;
use aep_domain::verification::Verifier;
use anyhow::{Context, Result};

use crate::Format;

/// One evidence record, with the envelope an evidence document needs.
///
/// Serialises as one entry of the list `protocol evaluate --evidence` reads: the payload's own
/// fields under its `kind`, beside `observed_at`, `producer` and `provenance`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct MintedEvidence {
    /// The observation, tagged by its kind.
    #[serde(flatten)]
    evidence: Evidence,
    /// When the check that produced it ran.
    observed_at: ObservedAt,
    /// What produced it. Always a verifier — see the [module documentation](self).
    producer: Producer,
    /// How it was obtained: the command, and the files it read.
    provenance: Provenance,
}

impl MintedEvidence {
    /// Wraps a payload a verifier of class `verifier` produced at `observed_at`.
    pub(crate) fn new(evidence: Evidence, verifier: Verifier, observed_at: ObservedAt) -> Self {
        Self {
            evidence,
            observed_at,
            producer: Producer::Verifier { verifier },
            provenance: Provenance::default(),
        }
    }

    /// Records the command line that produced it, as the verb reports it.
    ///
    /// Reconstructed by the caller rather than read from `std::env::args`, so the record says what
    /// was *asked for* in the vocabulary of the verb rather than however a shell spelled it.
    pub(crate) fn obtained_by(mut self, command: impl Into<String>) -> Self {
        self.provenance.command = Some(command.into());
        self
    }

    /// Records one file the check read.
    ///
    /// `reading` and not `from_input`: the same builder shape `protocol trace evidence` uses spells
    /// it the second way, and clippy's `wrong_self_convention` refuses a `from_*` that takes `self`
    /// — a rule worth keeping, because `Foo::from_x` reading as a constructor everywhere else is
    /// what makes a builder method with that name misread at every call site.
    pub(crate) fn reading(mut self, input: impl Into<String>) -> Self {
        self.provenance.inputs.push(input.into());
        self
    }

    /// The one-line summary a person sees when the document went to a file.
    pub(crate) fn summary(&self) -> String {
        self.evidence.summary()
    }
}

/// Renders the document `protocol evaluate --evidence` reads.
///
/// Always a list, even of one: a file holding several records and a file holding one are the same
/// document, and a bare record would be a second shape to support. `text` gets the document too
/// rather than a summary, because a summary is something a person pastes into a file and then
/// discovers the engine will not read.
fn render(record: &MintedEvidence, format: Format) -> Result<String> {
    match format {
        Format::Json => serde_json::to_string_pretty(&[record])
            .map(|mut json| {
                json.push('\n');
                json
            })
            .context("rendering the evidence record"),
        Format::Text | Format::Yaml => {
            serde_yaml::to_string(&[record]).context("rendering the evidence record")
        }
    }
}

/// Writes the document to `out`, or to standard output when there is none.
///
/// The directory above `out` is created, because a step map names a path inside a run directory
/// that the driver allocated and a producer that refused to make one would fail on its first run.
pub(crate) fn emit(record: &MintedEvidence, format: Format, out: Option<&Path>) -> Result<()> {
    let document = render(record, format)?;
    let Some(file) = out else {
        out!("{document}");
        return Ok(());
    };
    if let Some(parent) = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(file, &document).with_context(|| format!("writing {}", file.display()))?;
    outln!("{} — {}", file.display(), record.summary());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_domain::evidence::VerificationRecord;
    use aep_domain::ids::ClaimId;
    use aep_domain::time::Timestamp;
    use aep_domain::verification::VerificationStatus;

    /// A record to wrap, with nothing interesting in it.
    fn record() -> MintedEvidence {
        MintedEvidence::new(
            Evidence::Verification(VerificationRecord {
                claim: ClaimId::new("document-tree-valid").expect("a claim id"),
                verifier: Verifier::ArtifactValidator,
                status: VerificationStatus::Passed,
                subject: None,
                counterexamples: Vec::new(),
            }),
            Verifier::ArtifactValidator,
            ObservedAt::new(Timestamp::from_epoch_millis(1_699_920_000_000)),
        )
    }

    /// The document is the list `--evidence` reads, and it round-trips through the parser that
    /// reads one — which is the only property that matters about the shape.
    #[test]
    fn what_is_written_is_what_the_evidence_reader_parses() {
        let document = render(&record(), Format::Yaml).expect("it renders");
        let parsed = aep_schema::parse::evidence_list(&document, Some("test"))
            .expect("the document this module writes is the document `--evidence` reads");
        assert_eq!(parsed.len(), 1, "a list of one: {document}");
        assert_eq!(
            parsed[0].evidence.kind(),
            aep_domain::evidence::EvidenceKind::Verification
        );
    }

    /// `text` is the document and not a summary of it, so a person who redirected `text` output
    /// into a file has a file the engine reads.
    #[test]
    fn text_renders_the_document_rather_than_a_summary_of_it() {
        let text = render(&record(), Format::Text).expect("it renders");
        assert_eq!(
            text,
            render(&record(), Format::Yaml).expect("it renders"),
            "`text` and `yaml` are the same document"
        );
    }

    /// Invariant 7 at the layer that stamps the envelope: there is no way to ask for any producer
    /// other than a verifier, so a record this module writes can never claim a person's or an
    /// agent's authority.
    #[test]
    fn the_producer_is_a_verifier_and_no_argument_can_change_it() {
        let record = record();
        assert!(
            !record.producer.is_human(),
            "a driven step must not be able to sign as a person"
        );
        assert!(
            matches!(record.producer, Producer::Verifier { .. }),
            "`independent: true` is what this stamp is read as: {:?}",
            record.producer
        );
    }
}
