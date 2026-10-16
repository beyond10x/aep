//! `aep plan artifact spend [--kind <k>] [--since <date>] [--baseline <n>]`: spend per task kind
//! and figure against the baseline and target the project file declares (`aep.evidence.ReportSpend`
//! and `aep.plan.Project.spend` in `ess/`, story:spend-against-baseline).
//!
//! A read: it writes nothing, the project file included, and reads no clock. The fold the
//! specification cannot express is here, in three functions: [`artifact_figures`] (one record per
//! `source_input`, the latest by `at`, summed per figure), [`lower_median`] and [`report`] (the
//! rows, the filters and the standing).

use std::collections::BTreeMap;
use std::process::ExitCode;

use aep_backend_markdown::journal::{evidence_records_git, Change};
use aep_domain::artifact::ArtifactId;
use aep_domain::evidence::{EvidenceKind, TaskSpend};
use aep_domain::project::{SpendBound, SpendFigure};
use aep_domain::time::CivilDate;
use anyhow::{bail, Result};

use crate::Format;

use super::{open, Plan, StoreArgs};

/// What an unknown median prints as, and a standing whose median is unknown. Never `0`.
const UNKNOWN: &str = "unknown";

/// What a baseline or target the project does not declare prints as.
const NONE: &str = "none";

/// Where a median stands against its target: `aep.evidence.SpendStanding`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Standing {
    /// The median is below the target.
    Under,
    /// The median equals the target.
    At,
    /// The median is above the target.
    Over,
    /// No artifact states the figure, whether or not a target is declared.
    Unknown,
    /// The median is known and no target is declared.
    Undeclared,
}

impl Standing {
    /// The standing of `median` against `bound`.
    fn of(median: Option<u64>, bound: Option<SpendBound>) -> Self {
        match (median, bound) {
            // An unknown median stands nowhere, declared or not; only a known one can be
            // undeclared.
            (None, _) => Self::Unknown,
            (Some(_), None) => Self::Undeclared,
            (Some(median), Some(bound)) => match median.cmp(&bound.target) {
                std::cmp::Ordering::Less => Self::Under,
                std::cmp::Ordering::Equal => Self::At,
                std::cmp::Ordering::Greater => Self::Over,
            },
        }
    }

    /// The standing as the table prints it.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Under => "under",
            Self::At => "at",
            Self::Over => "over",
            Self::Unknown => UNKNOWN,
            Self::Undeclared => "undeclared",
        }
    }
}

/// One row: `aep.evidence.SpendRow`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct SpendRow {
    /// The task kind.
    kind: String,
    /// The figure.
    figure: SpendFigure,
    /// Artifacts of the kind with a `task_spend` record.
    tasks: usize,
    /// Of those, the ones whose figure is unknown.
    skipped: usize,
    /// The lower median over the others; `None` when none states the figure.
    median: Option<u64>,
    /// The project's baseline, `None` when it declares none.
    baseline: Option<u64>,
    /// The project's target, `None` when it declares none.
    target: Option<u64>,
    /// Where the median stands against the target.
    standing: Standing,
}

/// The report `spend` prints.
#[derive(Debug, serde::Serialize)]
struct SpendReport {
    /// `--since`, as given.
    since: Option<String>,
    /// `--baseline`, as given.
    baseline_tasks: Option<usize>,
    /// `task_spend` records that did not read and are not counted.
    unreadable: usize,
    /// The rows, kinds in name order and figures in [`SpendFigure::ALL`] order.
    rows: Vec<SpendRow>,
}

/// One `task_spend` record, as the fold reads it.
#[derive(Debug, Clone)]
pub(crate) struct SpendEntry {
    /// The evidence file's stem, which breaks a tie of `at`.
    pub(crate) id: String,
    /// The record's `at`, in milliseconds since the epoch; `None` when it does not read.
    pub(crate) at: Option<u64>,
    /// What the record says.
    pub(crate) spend: TaskSpend,
}

/// What one artifact spent: its earliest record and its summed figures.
#[derive(Debug)]
struct ArtifactSpend {
    /// The artifact's id.
    id: ArtifactId,
    /// Its earliest record's `at`.
    earliest: Option<u64>,
    /// Each figure, summed over the records that count; `None` when unknown.
    figures: BTreeMap<SpendFigure, Option<u64>>,
}

/// Each figure of one artifact, from its `task_spend` records.
///
/// Records sharing a `source_input` are snapshots of one cumulative source, so only the latest by
/// `at` counts, and at equal `at` the one whose evidence id sorts last. A record whose `at` does
/// not read is never later than one whose `at` does. Each figure is the sum over the records that
/// count, one per `source_input`, and unknown when any of them leaves it absent or the sum
/// overflows: a sum over some of the sources understates the task by the ones left out.
pub(crate) fn artifact_figures(records: &[SpendEntry]) -> BTreeMap<SpendFigure, Option<u64>> {
    let mut latest: BTreeMap<&str, &SpendEntry> = BTreeMap::new();
    for record in records {
        let key = |entry: &SpendEntry| (entry.at, entry.id.clone());
        latest
            .entry(record.spend.source_input.as_str())
            .and_modify(|held| {
                if key(record) > key(held) {
                    *held = record;
                }
            })
            .or_insert(record);
    }
    SpendFigure::ALL
        .into_iter()
        .map(|figure| {
            let sum = latest
                .values()
                .try_fold(0_u64, |sum, record| {
                    sum.checked_add(figure.of(&record.spend.figures)?)
                });
            (figure, sum)
        })
        .collect()
}

/// The lower median of `values`: the middle value of an odd count, the lower of the two middle
/// values of an even one, so always a value the store holds. `None` for no values.
pub(crate) fn lower_median(mut values: Vec<u64>) -> Option<u64> {
    values.sort_unstable();
    let middle = values.len().checked_sub(1)? / 2;
    values.get(middle).copied()
}

/// Every `task_spend` record of a Git-native plan, by artifact, with its kind, and how many did
/// not read. A SQLite or Postgres plan holds no evidence files and answers none.
fn records(
    opened: &super::Opened,
) -> (BTreeMap<ArtifactId, (String, Vec<SpendEntry>)>, usize) {
    let Plan::Git { root, evidence } = &opened.plan else {
        return (BTreeMap::new(), 0);
    };
    let mut found = BTreeMap::new();
    let mut unreadable = 0;
    for id in opened.report.documents.keys() {
        let (entries, _) = evidence_records_git(root, evidence, id);
        for (path, entry) in entries {
            let Change::Evidence { kind, source, .. } = &entry.change else {
                continue;
            };
            if *kind != EvidenceKind::TaskSpend {
                continue;
            }
            let Ok(spend) = serde_json::from_str::<TaskSpend>(source) else {
                unreadable += 1;
                continue;
            };
            let record = SpendEntry {
                id: path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                at: super::instant(&entry.at).ok().map(aep_domain::time::Timestamp::epoch_millis),
                spend,
            };
            found
                .entry(entry.artifact.clone())
                .or_insert_with(|| (entry.kind.to_string(), Vec::new()))
                .1
                .push(record);
        }
    }
    (found, unreadable)
}

/// `aep plan artifact spend`
pub(crate) fn report(
    args: &StoreArgs,
    kind: Option<&str>,
    since: Option<&str>,
    baseline: Option<usize>,
) -> Result<ExitCode> {
    // Parsed before the store is opened, so a day nobody can read is refused as a day, naming
    // it, rather than cutting the records somewhere else and printing that table.
    let cutoff = match since {
        Some(written) => match CivilDate::parse(written) {
            Ok(day) => Some(day.to_timestamp().epoch_millis()),
            Err(error) => bail!(
                "`--since {written}` is not a calendar date written YYYY-MM-DD ({error}); no table \
                 is printed"
            ),
        },
        None => None,
    };
    // A median over no task is unknown for every row, and the advice to copy the medians into
    // the project file would then hand a person nothing to copy.
    if baseline == Some(0) {
        bail!(
            "`--baseline 0` takes the median over no task of any kind; give the number of tasks \
             of each kind to measure, at least 1; no table is printed"
        );
    }
    let opened = open(&args.location, false)?;
    let config = args.location.project_config()?;
    let (found, unreadable) = records(&opened);

    let mut kinds: BTreeMap<String, Vec<ArtifactSpend>> = BTreeMap::new();
    for (id, (artifact_kind, mut entries)) in found {
        if let Some(cutoff) = cutoff {
            // An undated record cannot be shown to lie inside the window.
            entries.retain(|entry| entry.at.is_some_and(|at| at >= cutoff));
        }
        if entries.is_empty() {
            continue;
        }
        let earliest = entries.iter().filter_map(|entry| entry.at).min();
        kinds
            .entry(artifact_kind)
            .or_default()
            .push(ArtifactSpend {
                id,
                earliest,
                figures: artifact_figures(&entries),
            });
    }

    let mut rows = Vec::new();
    for (task_kind, mut artifacts) in kinds {
        if kind.is_some_and(|wanted| wanted != task_kind) {
            continue;
        }
        if let Some(first) = baseline {
            // The first `n` by their earliest record, an undated artifact after every dated one,
            // then by id, so the cut is the same on every run.
            artifacts.sort_by(|left, right| {
                (left.earliest.is_none(), left.earliest, &left.id).cmp(&(
                    right.earliest.is_none(),
                    right.earliest,
                    &right.id,
                ))
            });
            artifacts.truncate(first);
        }
        for figure in SpendFigure::ALL {
            let known: Vec<u64> = artifacts
                .iter()
                .filter_map(|artifact| artifact.figures.get(&figure).copied().flatten())
                .collect();
            let bound = config
                .as_ref()
                .and_then(|config| config.spend_bound(&task_kind, figure));
            let skipped = artifacts.len() - known.len();
            let median = lower_median(known);
            rows.push(SpendRow {
                kind: task_kind.clone(),
                figure,
                tasks: artifacts.len(),
                skipped,
                median,
                baseline: bound.map(|bound| bound.baseline),
                target: bound.map(|bound| bound.target),
                standing: Standing::of(median, bound),
            });
        }
    }

    let report = SpendReport {
        since: since.map(ToOwned::to_owned),
        baseline_tasks: baseline,
        unreadable,
        rows,
    };
    match args.format {
        Format::Text => print_report(&report),
        Format::Yaml | Format::Json => crate::print_serialised(&report, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// The table, for a person.
fn print_report(report: &SpendReport) {
    let mut rows = vec![[
        "kind", "figure", "tasks", "skipped", "median", "baseline", "target", "standing",
    ]
    .map(ToOwned::to_owned)
    .to_vec()];
    let shown = |value: Option<u64>, absent: &str| value.map_or_else(|| absent.to_owned(), |v| v.to_string());
    for row in &report.rows {
        rows.push(vec![
            row.kind.clone(),
            row.figure.as_str().to_owned(),
            row.tasks.to_string(),
            row.skipped.to_string(),
            shown(row.median, UNKNOWN),
            shown(row.baseline, NONE),
            shown(row.target, NONE),
            row.standing.as_str().to_owned(),
        ]);
    }
    if report.rows.is_empty() {
        outln!(
            "no task_spend records{}",
            report
                .since
                .as_ref()
                .map_or_else(String::new, |since| format!(" since {since}"))
        );
    } else {
        crate::print_table(&rows);
    }
    if let Some(first) = report.baseline_tasks {
        outln!(
            "medians over the first {first} task(s) of each kind; copy them into `spend` in \
             .engineering/project.yaml as baselines"
        );
    }
    if report.unreadable > 0 {
        outln!(
            "{} task_spend record(s) did not read and are not counted",
            report.unreadable
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_domain::evidence::SpendFigures;

    fn entry(id: &str, at: u64, source_input: &str, model_calls: Option<u64>) -> SpendEntry {
        SpendEntry {
            id: id.to_owned(),
            at: Some(at),
            spend: TaskSpend {
                source: aep_domain::evidence::SpendSource::Transcript,
                source_input: source_input.to_owned(),
                source_digest: "sha256:00".to_owned(),
                figures: SpendFigures {
                    model_calls,
                    ..SpendFigures::default()
                },
            },
        }
    }

    #[test]
    fn the_lower_median_is_a_value_the_store_holds() {
        assert_eq!(lower_median(vec![]), None);
        assert_eq!(lower_median(vec![7]), Some(7));
        assert_eq!(lower_median(vec![12, 8]), Some(8));
        assert_eq!(lower_median(vec![20, 8, 12]), Some(12));
        assert_eq!(lower_median(vec![20, 8, 12, 3]), Some(8));
    }

    #[test]
    fn one_record_per_source_input_counts_the_latest_then_the_last_id() {
        let figures = artifact_figures(&[
            entry("a", 1, "runs/a.jsonl", Some(3)),
            entry("b", 2, "runs/b.jsonl", Some(4)),
            entry("c1", 4, "runs/c.jsonl", Some(5)),
            entry("c2", 5, "runs/c.jsonl", Some(9)),
        ]);
        assert_eq!(figures[&SpendFigure::ModelCalls], Some(16));

        let tied = artifact_figures(&[
            entry("002", 5, "runs/c.jsonl", Some(9)),
            entry("001", 5, "runs/c.jsonl", Some(5)),
        ]);
        assert_eq!(
            tied[&SpendFigure::ModelCalls],
            Some(9),
            "at equal `at`, the id that sorts last counts"
        );
    }

    #[test]
    fn a_figure_one_counted_record_leaves_absent_is_unknown_never_zero() {
        let figures = artifact_figures(&[
            entry("a", 1, "runs/a.jsonl", Some(3)),
            entry("b", 2, "runs/b.jsonl", None),
        ]);
        assert_eq!(figures[&SpendFigure::ModelCalls], None);
        assert_eq!(figures[&SpendFigure::TotalTokens], None);
    }

    #[test]
    fn standing_compares_the_median_with_the_target() {
        let bound = Some(SpendBound {
            baseline: 25,
            target: 12,
        });
        assert_eq!(Standing::of(Some(11), bound), Standing::Under);
        assert_eq!(Standing::of(Some(12), bound), Standing::At);
        assert_eq!(Standing::of(Some(13), bound), Standing::Over);
        assert_eq!(Standing::of(None, bound), Standing::Unknown);
        assert_eq!(Standing::of(Some(12), None), Standing::Undeclared);
        assert_eq!(
            Standing::of(None, None),
            Standing::Unknown,
            "an unknown median is unknown before it is undeclared"
        );
    }
}
