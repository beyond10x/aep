//! What `aep plan artifact evidence <id> --kind task_spend --from <file>` reads out of a source:
//! the figures of one `task_spend` record (`aep.evidence.SpendRecord` and
//! `aep.evidence.RecordSpend` in `ess/domains/evidence.yaml`).
//!
//! Three sources, told apart by what the file says it is rather than by a flag:
//!
//! * an `eval.run-manifest/1` (its `format`), read through [`RawRunManifest`], the one parser of
//!   that document: its `tokens`, `cost_micro_usd` and `wall_time_ms`, and no split figure, since
//!   it states none;
//! * a driven run's cursor (a document [`DriverCursor`] reads): the llm steps its map declares
//!   and the ones it ran;
//! * anything else is a transcript, read by [`read_any`] in whichever wire it is written: the
//!   distinct API requests ([`Census::api_requests`](trace_domain::ir::Census::api_requests)) and
//!   the terminal records' usage, cost and duration, folded per session by
//!   [`per_session`] exactly as a run manifest's are.
//!
//! The source is named before it is read, by its path from the repository root; one outside the
//! root is refused, naming the path, so that no absolute local path reaches the record.

use std::path::Path;

use aep_domain::evidence::{SpendFigures, SpendSource, TaskSpend};
use aep_driver_spec::cursor::DriverCursor;
use anyhow::{bail, Context, Result};
use trace_domain::ir::{EventKind, RunOutcome, TraceIr};
use trace_spec::reader::read_any;

use crate::eval::{micro_usd_stated, per_session, RawRunManifest, MANIFEST_FORMAT};

use super::recorded_input;

/// Reads the spend record `path` states, for a record of the repository at `root`.
///
/// # Errors
///
/// `SpendSourceOutsideRepository` when `path` is not inside `root`; the file cannot be read; a
/// manifest that is not a readable `eval.run-manifest/1`; a transcript neither reader reads; a
/// cost no reader can convert, which is refused rather than recorded as unknown.
pub(crate) fn read_spend(root: &Path, path: &Path) -> Result<TaskSpend> {
    let Some(source_input) = recorded_input::repository_relative(root, path)? else {
        bail!(
            "SpendSourceOutsideRepository: {} is not inside the repository root {}. A spend \
             record names its source by its path from the root, and this one has none, so \
             nothing was recorded; copy the file into the repository and record that",
            path.display(),
            root.display()
        );
    };
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let (source, figures) = figures_of(&bytes)
        .with_context(|| format!("reading the spend {} states", path.display()))?;
    Ok(TaskSpend {
        source,
        source_input,
        source_digest: recorded_input::sha256(&bytes),
        figures: figures.with_split_total(),
    })
}

/// Which source `bytes` are, and the figures they state.
fn figures_of(bytes: &[u8]) -> Result<(SpendSource, SpendFigures)> {
    let text = String::from_utf8_lossy(bytes);
    if says_it_is_a_manifest(&text) {
        let raw: RawRunManifest = serde_yaml::from_str(&text)
            .with_context(|| format!("this is not a readable `{MANIFEST_FORMAT}` document"))?;
        return Ok((
            SpendSource::RunManifest,
            SpendFigures {
                total_tokens: raw.tokens,
                cost_micro_usd: raw.cost_micro_usd,
                wall_time_ms: raw.wall_time_ms,
                ..SpendFigures::default()
            },
        ));
    }
    if let Ok(cursor) = serde_json::from_slice::<DriverCursor>(bytes) {
        return Ok((
            SpendSource::DrivenRun,
            SpendFigures {
                llm_steps_declared: cursor.llm_steps_declared.map(u64::from),
                llm_steps_run: cursor.llm_steps_run.map(u64::from),
                ..SpendFigures::default()
            },
        ));
    }
    let ir = read_any(bytes).map_err(|errors| anyhow::anyhow!("{errors}"))?;
    Ok((SpendSource::Transcript, transcript_figures(&ir)?))
}

/// `true` when the document's own `format` names a run manifest. A YAML mapping is asked, and a
/// transcript, which is not one document, is not.
fn says_it_is_a_manifest(text: &str) -> bool {
    serde_yaml::from_str::<serde_yaml::Value>(text)
        .ok()
        .and_then(|document| {
            document
                .get("format")
                .and_then(serde_yaml::Value::as_str)
                .map(|format| format == MANIFEST_FORMAT)
        })
        .unwrap_or(false)
}

/// A transcript's figures: its API requests, and its terminal records folded per session.
fn transcript_figures(ir: &TraceIr) -> Result<SpendFigures> {
    let sessions = sessions_of(ir);
    let usage = |read: fn(&trace_domain::ir::RunUsage) -> Option<u64>| {
        per_session(&sessions, |outcome: &&RunOutcome| {
            outcome.usage.as_ref().and_then(read)
        })
    };
    let mut unreadable = None;
    let cost_micro_usd = per_session(&sessions, |outcome: &&RunOutcome| {
        let stated = outcome.total_cost_usd?;
        match micro_usd_stated(&stated.to_string()) {
            Ok(micro) => Some(micro),
            Err(reason) => {
                unreadable.get_or_insert(reason);
                None
            }
        }
    });
    if let Some(reason) = unreadable {
        bail!("a terminal record's `total_cost_usd`: {reason}");
    }
    Ok(SpendFigures {
        // No request at all is a transcript that records none, not one that made none.
        model_calls: (!ir.requests.is_empty())
            .then(|| u64::try_from(ir.census().api_requests).ok())
            .flatten(),
        input_tokens: usage(|usage| usage.input_tokens),
        output_tokens: usage(|usage| usage.output_tokens),
        cache_read_tokens: usage(|usage| usage.cache_read_input_tokens),
        cache_write_tokens: usage(|usage| usage.cache_creation_input_tokens),
        thinking_tokens: usage(|usage| usage.thinking_tokens),
        cost_micro_usd,
        wall_time_ms: per_session(&sessions, |outcome: &&RunOutcome| outcome.duration_ms),
        ..SpendFigures::default()
    })
}

/// The transcript's terminal records, one bucket per session in stream order.
///
/// A session opens at each session-start record; terminal records before the first one get the
/// leading bucket, so none is dropped. Several terminal records in one bucket restate one
/// session's running counters, which [`per_session`] charges once.
fn sessions_of(ir: &TraceIr) -> Vec<Vec<&RunOutcome>> {
    let mut sessions: Vec<Vec<&RunOutcome>> = vec![Vec::new()];
    for event in &ir.events {
        match &event.kind {
            EventKind::SessionStart(_) => sessions.push(Vec::new()),
            EventKind::RunOutcome(outcome) => sessions
                .last_mut()
                .expect("the leading bucket is never popped")
                .push(outcome),
            _ => {}
        }
    }
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The record's fields are the specification's `aep.evidence.SpendRecord`, less the three the
    /// store holds (the artifact, the record's name and its lifecycle state), by name.
    #[test]
    fn a_task_spend_record_carries_the_generated_spend_record_fields() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../generated/ess/schema/schema/entities/aep.evidence.SpendRecord.schema.json",
        );
        let schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("the generated schema is committed"),
        )
        .expect("the generated schema is JSON");
        let generated: Vec<&str> = schema["properties"]
            .as_object()
            .expect("an entity's properties")
            .keys()
            .map(String::as_str)
            .filter(|name| !["record", "artifact", "state"].contains(name))
            .collect();
        let mut ours = vec!["source", "source_digest", "source_input"];
        ours.extend(SpendFigures::NAMES);
        let mut sorted_generated = generated.clone();
        sorted_generated.sort_unstable();
        let mut sorted_ours = ours.clone();
        sorted_ours.sort_unstable();
        assert_eq!(sorted_ours, sorted_generated, "{generated:?}");
    }

    /// The sources are the specification's `aep.evidence.SpendSource`, by name and in order.
    #[test]
    fn spend_sources_are_the_generated_spend_source_variants() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../generated/ess/schema/schema/types/aep.evidence.SpendSource.schema.json");
        let schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("the generated schema is committed"),
        )
        .expect("the generated schema is JSON");
        let generated: Vec<&str> = schema["$defs"]["aep.evidence.SpendSource"]["enum"]
            .as_array()
            .expect("an enum")
            .iter()
            .map(|value| value.as_str().expect("a name"))
            .collect();
        let ours: Vec<&str> = SpendSource::ALL.iter().map(|source| source.as_str()).collect();
        assert_eq!(ours, generated);
    }
}
