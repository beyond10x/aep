//! Source-bound transcriptions of immutable reviews, without manufacturing another review round.

use aep_backend_markdown::{findings, StoreReport, StoredDocument};
use aep_domain::artifact::{ArtifactId, ArtifactKind, RelationKind};
use anyhow::{bail, Context, Result};
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;

const TAG: &str = "review-findings-supplement";
const SOURCE: &str = "review-body-sha256:";

pub(super) struct Resolved {
    pub source: ArtifactId,
    pub findings: Vec<findings::Finding>,
}

fn tagged(stored: &StoredDocument) -> bool {
    stored.document.frontmatter.tags.contains(TAG)
}

/// Check the transcription itself, including the immutable bytes it names.
fn target<'a>(report: &'a StoreReport, supplement: &StoredDocument) -> Result<&'a StoredDocument> {
    let front = &supplement.document.frontmatter;
    if front.kind != ArtifactKind::VerificationReport {
        bail!("review-findings-supplement requires verification-report");
    }
    let targets: Vec<_> = front
        .relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::Verifies)
        .collect();
    if targets.len() != 1 {
        bail!("review-findings-supplement requires exactly one verifies relation");
    }
    let original = report
        .documents
        .get(targets[0].target.id())
        .context("review-findings-supplement target is missing")?;
    if original.document.frontmatter.kind != ArtifactKind::ReviewResult {
        bail!("review-findings-supplement must verify a review-result");
    }
    if findings::opens_a_block(&original.document.body) {
        bail!("review-findings-supplement cannot replace an original findings block");
    }
    let sources: Vec<_> = front
        .refs
        .iter()
        .map(ToString::to_string)
        .filter(|reference| reference.starts_with(SOURCE))
        .collect();
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(original.document.body.as_bytes()) {
        write!(digest, "{byte:02x}")?;
    }
    let expected = format!("{SOURCE}{digest}");
    if sources.as_slice() != [expected] {
        bail!("review-findings-supplement requires the exact review-body-sha256 reference");
    }
    if !findings::opens_a_block(&supplement.document.body) {
        bail!(
            "review-findings-supplement requires a findings block, including [] for zero findings"
        );
    }
    findings::parse(&supplement.document.body)?;
    Ok(original)
}

/// Invalid declarations must be reported even when they name no existing review.
pub(super) fn problems(report: &StoreReport) -> Vec<String> {
    report
        .documents
        .values()
        .filter(|stored| tagged(stored))
        .filter_map(|stored| {
            target(report, stored)
                .err()
                .map(|error| format!("{}: {error}", stored.document.frontmatter.id))
        })
        .collect()
}

/// Resolve once for every reader; transcriptions never become additional reviews.
pub(super) fn resolve(report: &StoreReport, stored: &StoredDocument) -> Result<Option<Resolved>> {
    let id = &stored.document.frontmatter.id;
    let candidates: Vec<_> = report
        .documents
        .values()
        .filter(|candidate| {
            stored.document.frontmatter.kind == ArtifactKind::ReviewResult
                && tagged(candidate)
                && candidate
                    .document
                    .frontmatter
                    .relations
                    .iter()
                    .any(|relation| {
                        relation.kind == RelationKind::Verifies && relation.target.id() == id
                    })
        })
        .collect();
    if candidates.len() > 1 {
        bail!("multiple review-findings-supplement records verify {id}");
    }
    let Some(supplement) = candidates.first() else {
        if findings::opens_a_block(&stored.document.body) {
            return Ok(Some(Resolved { source: id.clone(), findings: findings::parse(&stored.document.body)? }));
        }
        return Ok(None);
    };
    target(report, supplement)?;
    Ok(Some(Resolved {
        source: supplement.document.frontmatter.id.clone(),
        findings: findings::parse(&supplement.document.body)?,
    }))
}
