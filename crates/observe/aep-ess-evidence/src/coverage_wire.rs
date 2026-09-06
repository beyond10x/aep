//! Strict structural transcription before typed suite/5 inventory checks.
use crate::count_json::{Json, Result};
use aep_domain::ess_conformance_coverage::{CoverageSummary, Inventory, SourceIdentity};

pub(crate) fn typed<T: serde::de::DeserializeOwned>(value: &Json) -> Result<T> {
    serde_json::from_str(&value.raw)
        .map_err(|error| value.error("InvalidCoverageVocabulary", error.to_string()))
}

pub(crate) fn summary(value: &Json) -> Result<CoverageSummary> {
    let fields = value.closed(&["selection", "knowledge", "counts", "refused"], &[])?;
    common(fields)?;
    typed(value)
}

pub(crate) fn inventory(value: &Json) -> Result<Inventory> {
    let fields = value.closed(
        &[
            "selection",
            "knowledge",
            "counts",
            "refused",
            "generated",
            "authored",
            "outside",
            "authored_sources",
        ],
        &[],
    )?;
    common(fields)?;
    for outside in fields["outside"].array()? {
        let record = outside.closed(&["scenario", "origin", "reason", "needs"], &[])?;
        references(&record["needs"])?;
    }
    for (identity, source) in fields["authored_sources"].object()? {
        SourceIdentity::new(identity.clone()).map_err(|mut error| {
            for issue in &mut error.issues {
                issue.path.clone_from(&source.path);
            }
            error
        })?;
        source.closed(&["digest", "scenario", "disposition"], &[])?;
    }
    typed(value)
}

fn common(fields: &std::collections::BTreeMap<String, Json>) -> Result<()> {
    let counts = fields["counts"].closed(&["generated", "authored", "outside", "refused"], &[])?;
    for count in counts.values() {
        count.unsigned()?;
    }
    for refusal in fields["refused"].array()? {
        let record = refusal.closed(
            &[
                "origin", "scenario", "subject", "source", "code", "message", "effect", "retained",
                "scope", "needs",
            ],
            &[],
        )?;
        if !record["subject"].null() {
            crate::count_suite::semantic_reference(&record["subject"])?;
        }
        if !record["retained"].null() {
            record["retained"].closed(&["origin", "source"], &[])?;
        }
        references(&record["needs"])?;
    }
    Ok(())
}

fn references(value: &Json) -> Result<()> {
    for reference in value.array()? {
        crate::count_suite::semantic_reference(reference)?;
    }
    Ok(())
}
