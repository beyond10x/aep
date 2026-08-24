//! `protocol eval matrix` — many checked runs become one table of facts, and never a score.
//!
//! The sixth module split, on the criterion the first five took: a verb family with its own input,
//! its own vocabulary and no shared state with the rest of the binary.
//!
//! # What this assembles, and what it refuses to assemble
//!
//! A three-arm evaluation runs the same cases three ways — **raw** instructions, the shipped
//! **plugin**, and a **driven** run whose tool calls an enforcer decides — against more than one
//! harness. Each run leaves two documents: a `eval.run-manifest/1` saying what was run and under
//! which arm, and the `trace-report/1` record `protocol trace check --format json` writes about its
//! transcript. This verb reads those pairs and reports, per harness × arm × workflow and per
//! expectation, **how many facts held, how many were contradicted, and how many nobody could find
//! out**.
//!
//! It computes no score, and that is a rule rather than an omission. A scalar would have to fold
//! the third column into one of the other two — the only two ways to do it are to count an
//! unobservable expectation as a pass, which is the lie invariant 5 exists to refuse, or as a
//! failure, which blames an agent for a harness that stopped recording a field. There is no
//! percentage, no ranking and no leaderboard in the output, and nothing in this module computes
//! one.
//!
//! # The record it reads is the check report, not the evidence record
//!
//! Both are called `trace_conformance` in conversation and they are not the same document. The
//! **evidence** record `protocol trace evidence` mints carries three counts and the ids that
//! gapped, and deliberately drops the rows — their citations quote the transcript, and an evidence
//! record is a thing people paste into pull requests (`crates/trace-spec/src/evidence.rs`). A
//! per-expectation matrix cannot be built from counts, so what this verb reads is the **check
//! report**, which has one row per expectation. `--redact` on the checker is what makes such a
//! report committable, and every committed fixture here is redacted.
//!
//! # Fail-closed, stated once and applied everywhere
//!
//! A row whose verdict is missing, or written as `null`, is counted **unobservable**. It is never
//! counted as held. That is the whole polarity of this crate in one sentence, and
//! `a_row_whose_verdict_is_null_is_unobservable_and_never_held` is the test that breaks if somebody
//! reverses it.
//!
//! The manifest carries the same rule in its shape: `plugin_digest` must be *written*, as a digest
//! or as an explicit `null`, because a key somebody forgot and a run that had no plugin are
//! different facts and only one of them is a run of arm `raw`.
//!
//! # Exit code
//!
//! `0` whenever a matrix was assembled, whatever it says. A matrix is a report, not a gate — the
//! same position `protocol trace inspect` and `protocol infra simulate` take — and an exit code
//! that moved with the counts would be the scalar this verb refuses to compute. Everything refused
//! here leaves through the binary's top-level handler as `1`, with the refusals on standard error.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

/// The format claim a run manifest carries.
const MANIFEST_FORMAT: &str = "eval.run-manifest/1";

/// The format claim the assembled matrix carries.
const MATRIX_FORMAT: &str = "eval.matrix/1";

/// The format claim of the record a manifest accompanies.
const REPORT_FORMAT: &str = "trace-report/1";

/// How a run manifest is named on disk.
const MANIFEST_SUFFIX: &str = ".manifest.yaml";

/// How the record beside it is named.
const RECORD_SUFFIX: &str = ".report.json";

/// How many hex characters a content digest has.
const DIGEST_WIDTH: usize = 64;

/// Micro-dollars in a dollar.
///
/// Cost is carried in millionths of a US dollar and never as a float: a matrix is a document people
/// commit and diff, and two runs of the same assembler must produce the same bytes. `aep-render`
/// bans floats outright for this reason; here the ban is narrower — the input field is an integer,
/// so nothing downstream has to round.
const MICRO_USD: u64 = 1_000_000;

// --- the three arms ---------------------------------------------------------------------------

/// Which arm a run belongs to.
///
/// Closed, because the three arms are the design of the evaluation rather than a label somebody
/// picked: raw instructions, the shipped plugin, and a driven run whose calls an enforcer decides.
/// A fourth arm is a change to the programme, and it should stop here rather than appear as a new
/// row nobody planned.
///
/// The declaration order is the programme's order — a, b, c — and `Ord` follows it. Sorting
/// alphabetically would print `driven`, `plugin`, `raw`, which reads the experiment backwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Arm {
    /// The instructions alone: no plugin, no enforcement.
    Raw,
    /// The shipped plugin is installed, and nothing decides the agent's calls.
    Plugin,
    /// The run is driven, and every tool call is decided at a seam.
    Driven,
    /// The loop is ours, and the published toolset **is** the policy.
    ///
    /// The fourth treatment, and it is not a fourth flavour of the third. In `plugin` the policy is
    /// injected into a vendor's loop; in `driven` it is imposed on one from outside, per call, at a
    /// seam. Here there is no vendor loop: the tools a run may call are computed from what the
    /// machine can confine and published, so a tool outside the surface is not refused — it does
    /// not exist. What this arm measures is whether *that* changes what a model does.
    Native,
}

impl Arm {
    /// Reads the word a manifest wrote, or nothing when it is not one of the three.
    fn parse(written: &str) -> Option<Self> {
        match written {
            "raw" => Some(Self::Raw),
            "plugin" => Some(Self::Plugin),
            "driven" => Some(Self::Driven),
            "native" => Some(Self::Native),
            _ => None,
        }
    }

    /// The word a manifest writes.
    fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Plugin => "plugin",
            Self::Driven => "driven",
            Self::Native => "native",
        }
    }

    /// Every arm, in programme order, for a refusal that lists what was expected.
    const ALL: [Self; 3] = [Self::Raw, Self::Plugin, Self::Driven];
}

impl fmt::Display for Arm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// --- what one expectation said ----------------------------------------------------------------

/// What one expectation's row in a record says, in the matrix's vocabulary.
///
/// Three, for the reason `trace-spec`'s `Verdict` has three and `infra-spec`'s `Outcome` has three:
/// *the run did the wrong thing* and *nobody could find out* are different findings that want
/// different people to react, and a matrix that folded them together would be reporting a number
/// nobody can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The expectation held.
    Held,
    /// The run contradicted it.
    Violated,
    /// The record could not decide it — or recorded no verdict at all.
    Unobservable,
}

/// The three counts, which is all a cell of the matrix ever holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
struct Counts {
    /// How many facts held.
    held: usize,
    /// How many were contradicted.
    violated: usize,
    /// How many nobody could find out.
    unobservable: usize,
}

impl Counts {
    /// Adds one outcome.
    fn add(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Held => self.held += 1,
            Outcome::Violated => self.violated += 1,
            Outcome::Unobservable => self.unobservable += 1,
        }
    }

    /// Adds another cell's counts.
    fn absorb(&mut self, other: Self) {
        self.held += other.held;
        self.violated += other.violated;
        self.unobservable += other.unobservable;
    }
}

// --- refusals ---------------------------------------------------------------------------------

/// Every way a pair of documents is refused at this boundary, by name.
///
/// A code and a sentence, on the reasoning invariant 4 gives for `ValidationCode`: a test matching
/// on `EVAL-MANIFEST-005` still passes when the sentence is rewritten, and a test matching on the
/// sentence pins prose that nobody meant to freeze.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Refusal {
    /// The document does not claim to be a run manifest.
    NotAManifest {
        /// What it claimed instead, where it claimed anything.
        found: Option<String>,
    },
    /// The `arm` is not one of the three.
    ArmUnknown {
        /// The word the manifest wrote.
        written: String,
    },
    /// A required field is not there at all.
    FieldMissing {
        /// Which one.
        field: &'static str,
    },
    /// A required field is there and says nothing.
    FieldEmpty {
        /// Which one.
        field: &'static str,
    },
    /// Arm `raw` is the arm with no plugin in it, and this manifest names one.
    PluginDigestOnRawArm,
    /// Arm `plugin` is the arm whose subject is the plugin, and this manifest has no digest for it.
    PluginDigestAbsentOnPluginArm,
    /// A digest field is not a digest.
    DigestMalformed {
        /// Which field.
        field: &'static str,
        /// What was written there.
        written: String,
    },
    /// The record is not a check report.
    NotARecord {
        /// What it claimed instead, where it claimed anything.
        found: Option<String>,
    },
    /// A row in the record has no expectation id, so its outcome cannot be attributed.
    RowWithoutId {
        /// Which row, by position.
        position: usize,
    },
    /// A row carries a verdict word this build does not know.
    VerdictUnknown {
        /// The row.
        id: String,
        /// The word.
        written: String,
    },
    /// A manifest with no record beside it.
    RecordMissing {
        /// Where the record was looked for.
        expected: String,
    },
    /// A record with no manifest beside it.
    ManifestMissing {
        /// Where the manifest was looked for.
        expected: String,
    },
    /// The manifest and the record describe different runs.
    TranscriptMismatch {
        /// What the manifest claims.
        manifest: String,
        /// What the record was judged over.
        record: String,
    },
    /// Two manifests describe the same transcript, so one run would be counted twice.
    RunCountedTwice {
        /// The digest they share.
        transcript_digest: String,
    },
    /// One specification id arrived at two digests.
    SpecificationMoved {
        /// The id both records name.
        specification: String,
        /// The digests they name it at, sorted.
        digests: Vec<String>,
    },
    /// There is nothing to assemble.
    NoRuns,
}

impl Refusal {
    /// The stable code a test matches on.
    fn code(&self) -> &'static str {
        match self {
            Self::NotAManifest { .. } => "EVAL-MANIFEST-001",
            Self::ArmUnknown { .. } => "EVAL-MANIFEST-002",
            Self::FieldMissing { .. } => "EVAL-MANIFEST-003",
            Self::FieldEmpty { .. } => "EVAL-MANIFEST-004",
            Self::PluginDigestOnRawArm => "EVAL-MANIFEST-005",
            Self::PluginDigestAbsentOnPluginArm => "EVAL-MANIFEST-006",
            Self::DigestMalformed { .. } => "EVAL-MANIFEST-007",
            Self::NotARecord { .. } => "EVAL-RECORD-001",
            Self::RowWithoutId { .. } => "EVAL-RECORD-002",
            Self::VerdictUnknown { .. } => "EVAL-RECORD-003",
            Self::RecordMissing { .. } => "EVAL-PAIR-001",
            Self::ManifestMissing { .. } => "EVAL-PAIR-002",
            Self::TranscriptMismatch { .. } => "EVAL-PAIR-003",
            Self::RunCountedTwice { .. } => "EVAL-PAIR-004",
            Self::SpecificationMoved { .. } => "EVAL-PAIR-005",
            Self::NoRuns => "EVAL-PAIR-006",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.code())?;
        match self {
            Self::NotAManifest { found } => write!(
                f,
                "this is not a `{MANIFEST_FORMAT}` document{}",
                claimed(found.as_deref())
            ),
            Self::ArmUnknown { written } => write!(
                f,
                "`{written}` is not one of the three arms this evaluation has: {}. A fourth arm is \
                 a change to the programme, not a label",
                Arm::ALL
                    .iter()
                    .map(|arm| format!("`{arm}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::FieldMissing { field } => write!(
                f,
                "the manifest states no `{field}`, and every field of a run manifest is what a \
                 later reader joins the matrix's rows by"
            ),
            Self::FieldEmpty { field } => {
                write!(f, "the manifest's `{field}` is empty, which names nothing")
            }
            Self::PluginDigestOnRawArm => write!(
                f,
                "arm `raw` is the arm with no plugin in it, and this manifest names a \
                 `plugin_digest`. Either the run had the plugin — in which case it is arm `plugin` \
                 — or the digest belongs to another run"
            ),
            Self::PluginDigestAbsentOnPluginArm => write!(
                f,
                "arm `plugin` is the arm whose subject is the plugin, and this manifest writes \
                 `plugin_digest: null`. A matrix row that cannot say which plugin was measured \
                 measures nothing"
            ),
            Self::DigestMalformed { field, written } => write!(
                f,
                "`{field}` is `{written}`, which is not a digest: {DIGEST_WIDTH} lowercase hex \
                 characters, the form `sha256sum` prints"
            ),
            Self::NotARecord { found } => write!(
                f,
                "this is not a `{REPORT_FORMAT}` record{} — the matrix reads the report \
                 `protocol trace check --format json` writes, which has one row per expectation, \
                 and not the evidence record, which carries counts and drops the rows",
                claimed(found.as_deref())
            ),
            Self::RowWithoutId { position } => write!(
                f,
                "the record's row at position {position} states no `id`, so what it says cannot be \
                 attributed to an expectation"
            ),
            Self::VerdictUnknown { id, written } => write!(
                f,
                "the row `{id}` states the verdict `{written}`, which this build does not know. A \
                 word nobody can read is not the same finding as a row nobody could decide, so it \
                 is refused rather than counted as one"
            ),
            Self::RecordMissing { expected } => write!(
                f,
                "this manifest has no record beside it: {expected} does not exist. A manifest \
                 alone describes a run nobody checked"
            ),
            Self::ManifestMissing { expected } => write!(
                f,
                "this record has no manifest beside it: {expected} does not exist. Left alone the \
                 record's outcomes would be dropped from the matrix without a line saying so"
            ),
            Self::TranscriptMismatch { manifest, record } => write!(
                f,
                "the manifest claims the run with transcript `{manifest}` and the record was \
                 judged over `{record}`. One of the two documents belongs to another run"
            ),
            Self::RunCountedTwice { transcript_digest } => write!(
                f,
                "two manifests name the transcript `{transcript_digest}`, so one run would be \
                 counted twice. Two runs are two transcripts"
            ),
            Self::SpecificationMoved {
                specification,
                digests,
            } => write!(
                f,
                "the records name the specification `{specification}` at {} different digests \
                 ({}). The matrix joins its rows by expectation id, so rows judged by two versions \
                 of one document share a name and not a meaning — re-check the runs against one \
                 version",
                digests.len(),
                digests.join(", ")
            ),
            Self::NoRuns => write!(
                f,
                "there is nothing here to assemble: no `*{MANIFEST_SUFFIX}` was found. An empty \
                 matrix renders as a table with no failures in it, which reads exactly like a \
                 clean sheet"
            ),
        }
    }
}

/// The ` (it claims to be X)` clause, where a document claimed anything at all.
fn claimed(found: Option<&str>) -> String {
    found.map_or_else(
        || " and states no `format`".to_owned(),
        |format| format!(" — it states `format: {format}`"),
    )
}

/// Every refusal in one message, in the shape this binary already uses for a refused document.
fn refused(subject: &Path, refusals: &[Refusal]) -> anyhow::Error {
    let lines: Vec<String> = refusals
        .iter()
        .map(|refusal| format!("  {refusal}"))
        .collect();
    anyhow::anyhow!(
        "{} — {} refusal(s):\n{}",
        subject.display(),
        refusals.len(),
        lines.join("\n")
    )
}

// --- the manifest, parsed then validated ------------------------------------------------------

/// A run manifest as it is written down.
///
/// Every field is optional here and required in [`RunManifest`], so that a missing one is refused
/// **by name** with the other refusals beside it, rather than aborting the parse at the first
/// (invariant 3: validation accumulates).
///
/// `deny_unknown_fields`, unlike the record reader below, and the asymmetry is deliberate: this
/// document is ours, so `plugin_digests:` is a typo that would otherwise be dropped silently and
/// read as an omitted key; the record is another producer's, so a field it grows is not this
/// verb's business.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRunManifest {
    /// The format claim.
    format: Option<String>,
    /// Which arm.
    arm: Option<String>,
    /// Which harness ran it.
    harness: Option<String>,
    /// The workflow the case is a run of.
    workflow: Option<String>,
    /// The case or task.
    case: Option<String>,
    /// The plugin the run was given, or an explicit `null`.
    ///
    /// A [`Written`] and not an `Option`, because serde maps *the key is not there* and *the key
    /// says `null`* onto the same `None` — and those are the two facts this field exists to keep
    /// apart.
    #[serde(default, deserialize_with = "written_down")]
    plugin_digest: Written,
    /// The model, as the harness resolved it, or an explicit `null`.
    ///
    /// A [`Written`] for `plugin_digest`'s reason, and it earned it the same way — on a live run.
    /// Codex's wire states no model at session start at all, so *nobody wrote the model down* and
    /// *the harness never said which model* are different facts, and only the second is a run this
    /// verb can honestly describe. The key is still required.
    #[serde(default, deserialize_with = "written_down")]
    model: Written,
    /// The harness version the arm is pinned to.
    harness_version: Option<String>,
    /// The transcript the record beside it was judged over.
    transcript_digest: Option<String>,
    /// When the run was observed.
    observed_at: Option<String>,
    /// What it cost, in millionths of a US dollar.
    cost_micro_usd: Option<u64>,
    /// How many tokens it used.
    tokens: Option<u64>,
    /// How long it took, in milliseconds.
    wall_time_ms: Option<u64>,
}

/// What a document wrote for a key that must be written down.
///
/// Three states, because there are three: the key is not there, the key is there and says `null`,
/// the key is there and says something. Collapsing the first two is what an `Option` does, and it
/// is the collapse `plugin_digest` cannot survive — *nobody recorded which plugin* and *there was
/// no plugin* are the difference between a manifest with a hole in it and a run of arm `raw`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Written {
    /// The key is not in the document.
    #[default]
    Absent,
    /// The key is there and says `null`.
    Null,
    /// The key is there and says this.
    Value(String),
}

/// Reads a key that must be *written*, so `null` and absent stay different answers.
///
/// `#[serde(default)]` on the field is what produces [`Written::Absent`]: this function is only
/// called when the key is present, so a `None` here is the document's own `null`.
fn written_down<'de, D>(deserializer: D) -> Result<Written, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match Option::<String>::deserialize(deserializer)? {
        Some(value) => Written::Value(value),
        None => Written::Null,
    })
}

/// What a run manifest says, once it has been read through the rules.
///
/// No `Deserialize`, by invariant 2: the only way to obtain one is [`TryFrom`], so there is no path
/// into the matrix that skipped the arm vocabulary or the `plugin_digest` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunManifest {
    /// Which arm.
    arm: Arm,
    /// Which harness.
    harness: String,
    /// The workflow.
    workflow: String,
    /// The case or task.
    case: String,
    /// The plugin the run was given, where it had one.
    plugin_digest: Option<String>,
    /// The model, where the harness said which.
    model: Option<String>,
    /// The harness version pin.
    harness_version: String,
    /// The transcript this manifest claims to describe.
    transcript_digest: String,
    /// When the run was observed, as the manifest wrote it.
    observed_at: String,
    /// What it cost, in millionths of a US dollar, where it said.
    cost_micro_usd: Option<u64>,
    /// How many tokens, where it said.
    tokens: Option<u64>,
    /// How long, in milliseconds, where it said.
    wall_time_ms: Option<u64>,
}

impl TryFrom<RawRunManifest> for RunManifest {
    type Error = Vec<Refusal>;

    fn try_from(raw: RawRunManifest) -> Result<Self, Self::Error> {
        let mut refusals = Vec::new();

        if raw.format.as_deref() != Some(MANIFEST_FORMAT) {
            refusals.push(Refusal::NotAManifest {
                found: raw.format.clone(),
            });
        }

        let arm = raw.arm.as_deref().and_then(|written| {
            let arm = Arm::parse(written);
            if arm.is_none() {
                refusals.push(Refusal::ArmUnknown {
                    written: written.to_owned(),
                });
            }
            arm
        });
        if raw.arm.is_none() {
            refusals.push(Refusal::FieldMissing { field: "arm" });
        }

        let harness = required(&mut refusals, "harness", raw.harness.as_deref());
        let workflow = required(&mut refusals, "workflow", raw.workflow.as_deref());
        let case = required(&mut refusals, "case", raw.case.as_deref());
        let model = written_or_null(&mut refusals, "model", &raw.model);
        let harness_version = required(
            &mut refusals,
            "harness_version",
            raw.harness_version.as_deref(),
        );
        let observed_at = required(&mut refusals, "observed_at", raw.observed_at.as_deref());
        let transcript_digest = required(
            &mut refusals,
            "transcript_digest",
            raw.transcript_digest.as_deref(),
        );
        if let Some(digest) = transcript_digest.as_deref() {
            check_digest(&mut refusals, "transcript_digest", digest);
        }

        let plugin_digest = plugin_digest(&mut refusals, arm, &raw.plugin_digest);

        if !refusals.is_empty() {
            return Err(refusals);
        }

        // Every `expect` below is discharged by the emptiness check above: each `None` pushed a
        // refusal, so reaching here means every one of them is `Some`.
        Ok(Self {
            arm: arm.expect("an arm was read"),
            harness: harness.expect("a harness was read"),
            workflow: workflow.expect("a workflow was read"),
            case: case.expect("a case was read"),
            plugin_digest,
            model,
            harness_version: harness_version.expect("a harness version was read"),
            transcript_digest: transcript_digest.expect("a transcript digest was read"),
            observed_at: observed_at.expect("an observation time was read"),
            cost_micro_usd: raw.cost_micro_usd,
            tokens: raw.tokens,
            wall_time_ms: raw.wall_time_ms,
        })
    }
}

/// A field that must be there and must say something.
fn required(
    refusals: &mut Vec<Refusal>,
    field: &'static str,
    written: Option<&str>,
) -> Option<String> {
    match written {
        None => {
            refusals.push(Refusal::FieldMissing { field });
            None
        }
        Some(value) if value.trim().is_empty() => {
            refusals.push(Refusal::FieldEmpty { field });
            None
        }
        Some(value) => Some(value.to_owned()),
    }
}

/// A field that must be *written*, and whose written value may be `null`.
///
/// [`required`]'s sibling, and the difference between them is the whole of what a live Codex run
/// taught this reader: `null` is an answer and an absent key is not one. A key nobody wrote is
/// refused, a key written `null` is read as *the harness did not say*, and the two never collapse.
fn written_or_null(
    refusals: &mut Vec<Refusal>,
    field: &'static str,
    written: &Written,
) -> Option<String> {
    match written {
        Written::Absent => {
            refusals.push(Refusal::FieldMissing { field });
            None
        }
        Written::Null => None,
        Written::Value(value) if value.trim().is_empty() => {
            refusals.push(Refusal::FieldEmpty { field });
            None
        }
        Written::Value(value) => Some(value.clone()),
    }
}

/// A digest field must be the form `sha256sum` prints.
fn check_digest(refusals: &mut Vec<Refusal>, field: &'static str, written: &str) {
    let well_formed = written.len() == DIGEST_WIDTH
        && written
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));
    if !well_formed {
        refusals.push(Refusal::DigestMalformed {
            field,
            written: written.to_owned(),
        });
    }
}

/// The `plugin_digest` rule: written always, `null` exactly on arm `raw`.
///
/// Arm `driven` may answer either way, and that is a decision rather than an oversight. What
/// enforces a driven run is the driver at the seam; whether the plugin was *also* installed is a
/// fact about that run, so both answers describe a run that could have happened — and the key is
/// still required, so the manifest has to state which.
fn plugin_digest(
    refusals: &mut Vec<Refusal>,
    arm: Option<Arm>,
    written: &Written,
) -> Option<String> {
    match (arm, written) {
        (_, Written::Absent) => {
            refusals.push(Refusal::FieldMissing {
                field: "plugin_digest",
            });
            None
        }
        (Some(Arm::Raw), Written::Value(_)) => {
            refusals.push(Refusal::PluginDigestOnRawArm);
            None
        }
        (Some(Arm::Plugin), Written::Null) => {
            refusals.push(Refusal::PluginDigestAbsentOnPluginArm);
            None
        }
        (_, Written::Value(digest)) => {
            check_digest(refusals, "plugin_digest", digest);
            Some(digest.clone())
        }
        (_, Written::Null) => None,
    }
}

// --- the record ---------------------------------------------------------------------------------

/// A check report, read for the three things the matrix needs.
///
/// No `deny_unknown_fields`: the record is another producer's document and grows fields — the
/// reader gains one the day the seam carries it and gains no rule with it, which is the position
/// `trace-spec`'s own adapters take.
#[derive(Debug, Deserialize)]
struct RawRecord {
    /// The format claim.
    format: Option<String>,
    /// The specification's id.
    spec_id: Option<String>,
    /// The specification's digest.
    spec_digest: Option<String>,
    /// The transcript it was judged over.
    transcript_digest: Option<String>,
    /// One row per expectation.
    expectations: Option<Vec<RawRow>>,
}

/// One expectation's row.
#[derive(Debug, Deserialize)]
struct RawRow {
    /// The id the specification gave it.
    id: Option<String>,
    /// The verdict after the expectation's own `on_unknown` policy — the same value the report's
    /// summary counts and its exit code is derived from.
    verdict: Option<String>,
}

/// What a record says, once read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    /// The specification's id.
    specification: String,
    /// The specification's digest, where the record states one.
    spec_digest: Option<String>,
    /// The transcript it was judged over, where the record states one.
    transcript_digest: Option<String>,
    /// Each expectation and what it said, in the record's own order.
    rows: Vec<(String, Outcome)>,
}

impl TryFrom<RawRecord> for Record {
    type Error = Vec<Refusal>;

    fn try_from(raw: RawRecord) -> Result<Self, Self::Error> {
        let mut refusals = Vec::new();

        if raw.format.as_deref() != Some(REPORT_FORMAT) {
            refusals.push(Refusal::NotARecord {
                found: raw.format.clone(),
            });
        }

        let mut rows = Vec::new();
        for (position, row) in raw.expectations.unwrap_or_default().into_iter().enumerate() {
            let Some(id) = row.id.filter(|id| !id.trim().is_empty()) else {
                refusals.push(Refusal::RowWithoutId { position });
                continue;
            };
            match outcome_of(row.verdict.as_deref()) {
                Ok(outcome) => rows.push((id, outcome)),
                Err(written) => refusals.push(Refusal::VerdictUnknown { id, written }),
            }
        }

        if !refusals.is_empty() {
            return Err(refusals);
        }

        Ok(Self {
            specification: raw.spec_id.unwrap_or_default(),
            spec_digest: raw.spec_digest,
            transcript_digest: raw.transcript_digest,
            rows,
        })
    }
}

/// The polarity of this whole verb, in one function.
///
/// A verdict the record does not state — absent, or written `null` — is **unobservable**. Never
/// held: a checker that recorded nothing about a row has not established anything about it, and a
/// matrix that counted silence as a pass would be the one number this programme refuses to produce.
/// A word this build cannot read is refused instead of bucketed, because *the checker said
/// something new* and *nobody found out* are different facts.
fn outcome_of(verdict: Option<&str>) -> Result<Outcome, String> {
    match verdict {
        Some("ok") => Ok(Outcome::Held),
        Some("gap") => Ok(Outcome::Violated),
        // The checker's own third verdict, and the absence of any verdict at all, are the same
        // answer here: nothing was established. Written as one arm because they *are* one answer —
        // what must never appear on this line is `Outcome::Held`.
        Some("unknown") | None => Ok(Outcome::Unobservable),
        Some(other) => Err(other.to_owned()),
    }
}

// --- the matrix ---------------------------------------------------------------------------------

/// A resource column: a total, and how many runs it is a total over.
///
/// Never a bare number. A cell whose three runs include one that recorded no cost has a total over
/// two of them, and a reader who cannot see that is reading a number that means something else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct Reported {
    /// How many runs in the cell stated this quantity.
    runs: usize,
    /// Their total.
    total: u64,
}

/// The three resource columns of a cell, each absent until some run states one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
struct Resources {
    /// Cost, in millionths of a US dollar.
    cost_micro_usd: Option<Reported>,
    /// Tokens.
    tokens: Option<Reported>,
    /// Wall time, in milliseconds.
    wall_time_ms: Option<Reported>,
}

impl Resources {
    /// Folds one run's quantities in.
    fn absorb(&mut self, manifest: &RunManifest) {
        add(&mut self.cost_micro_usd, manifest.cost_micro_usd);
        add(&mut self.tokens, manifest.tokens);
        add(&mut self.wall_time_ms, manifest.wall_time_ms);
    }
}

/// Adds a run's quantity to a column, creating the column the first time one is stated.
fn add(column: &mut Option<Reported>, stated: Option<u64>) {
    let Some(value) = stated else { return };
    let reported = column.get_or_insert(Reported { runs: 0, total: 0 });
    reported.runs += 1;
    reported.total = reported.total.saturating_add(value);
}

/// One run, as the matrix reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RunRow {
    /// The workflow.
    workflow: String,
    /// The case.
    case: String,
    /// The harness.
    harness: String,
    /// The arm.
    arm: Arm,
    /// The model, where the harness said which — `null` and never an omitted key, because a wire
    /// that states no model is stating something.
    model: Option<String>,
    /// The harness version pin.
    harness_version: String,
    /// The plugin the run was given — `null` on arm `raw`, and never an omitted key.
    plugin_digest: Option<String>,
    /// The specification its record was judged by.
    specification: String,
    /// The transcript.
    transcript_digest: String,
    /// When it was observed.
    observed_at: String,
    /// What its expectations said.
    #[serde(flatten)]
    counts: Counts,
    /// What it cost, where it said — `null` and never an omitted key, for the manifest's reason.
    cost_micro_usd: Option<u64>,
    /// How many tokens, where it said.
    tokens: Option<u64>,
    /// How long, where it said.
    wall_time_ms: Option<u64>,
}

/// One harness × arm × workflow cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Cell {
    /// The workflow.
    workflow: String,
    /// The harness.
    harness: String,
    /// The arm.
    arm: Arm,
    /// How many runs went into it.
    runs: usize,
    /// What their expectations said.
    #[serde(flatten)]
    counts: Counts,
    /// What they cost, over the runs that said.
    #[serde(flatten)]
    resources: Resources,
}

/// One expectation in one cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ExpectationRow {
    /// The workflow.
    workflow: String,
    /// The expectation's id.
    expectation: String,
    /// The harness.
    harness: String,
    /// The arm.
    arm: Arm,
    /// How many runs judged it.
    runs: usize,
    /// What they said.
    #[serde(flatten)]
    counts: Counts,
}

/// The specification a set of records was judged by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SpecificationRow {
    /// Its id.
    id: String,
    /// Its digest, where the records state one.
    digest: Option<String>,
    /// How many runs it judged.
    runs: usize,
}

/// The deliverable: counts of facts, per run, per cell and per expectation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Matrix {
    /// The format claim.
    format: &'static str,
    /// The specifications the records were judged by.
    specifications: Vec<SpecificationRow>,
    /// Every run.
    runs: Vec<RunRow>,
    /// Every harness × arm × workflow cell.
    cells: Vec<Cell>,
    /// Every expectation, per cell.
    expectations: Vec<ExpectationRow>,
    /// The three counts over everything, which is as close to a summary as this document goes.
    #[serde(flatten)]
    totals: Counts,
}

/// Assembles the matrix, or every reason it cannot be assembled.
///
/// Sorted by construction: the pairs arrive sorted by path, and every aggregate is built in a
/// `BTreeMap` whose key is the tuple it is grouped by (invariant 9 — no `HashMap` anywhere near an
/// output ordering).
fn assemble(pairs: Vec<(RunManifest, Record)>) -> Result<Matrix, Vec<Refusal>> {
    let mut refusals = Vec::new();

    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (manifest, record) in &pairs {
        *seen.entry(manifest.transcript_digest.clone()).or_default() += 1;
        if let Some(judged) = record.transcript_digest.as_deref() {
            if judged != manifest.transcript_digest {
                refusals.push(Refusal::TranscriptMismatch {
                    manifest: manifest.transcript_digest.clone(),
                    record: judged.to_owned(),
                });
            }
        }
    }
    for (digest, count) in seen {
        if count > 1 {
            refusals.push(Refusal::RunCountedTwice {
                transcript_digest: digest,
            });
        }
    }

    let mut specifications: BTreeMap<String, (BTreeSet<String>, usize)> = BTreeMap::new();
    for (_, record) in &pairs {
        let entry = specifications
            .entry(record.specification.clone())
            .or_default();
        if let Some(digest) = record.spec_digest.clone() {
            entry.0.insert(digest);
        }
        entry.1 += 1;
    }
    for (specification, (digests, _)) in &specifications {
        if digests.len() > 1 {
            refusals.push(Refusal::SpecificationMoved {
                specification: specification.clone(),
                digests: digests.iter().cloned().collect(),
            });
        }
    }

    if !refusals.is_empty() {
        return Err(refusals);
    }

    Ok(fold(pairs, specifications))
}

/// Adds one run's rows to the per-expectation table, and answers with that run's own counts.
///
/// The key is `(workflow, expectation, harness, arm)`, which is the join the matrix is *for*: the
/// same expectation, asked of the same workflow, in each arm of each harness.
fn per_expectation(
    expectations: &mut BTreeMap<(String, String, String, Arm), (usize, Counts)>,
    manifest: &RunManifest,
    record: &Record,
) -> Counts {
    let mut counts = Counts::default();
    for (id, outcome) in &record.rows {
        counts.add(*outcome);
        let row = expectations
            .entry((
                manifest.workflow.clone(),
                id.clone(),
                manifest.harness.clone(),
                manifest.arm,
            ))
            .or_insert_with(|| (0, Counts::default()));
        row.0 += 1;
        row.1.add(*outcome);
    }
    counts
}

/// The folding half of [`assemble`], once the pairs are known to describe distinct runs.
fn fold(
    pairs: Vec<(RunManifest, Record)>,
    specifications: BTreeMap<String, (BTreeSet<String>, usize)>,
) -> Matrix {
    let mut runs = Vec::new();
    let mut cells: BTreeMap<(String, String, Arm), (usize, Counts, Resources)> = BTreeMap::new();
    let mut expectations: BTreeMap<(String, String, String, Arm), (usize, Counts)> =
        BTreeMap::new();
    let mut totals = Counts::default();

    for (manifest, record) in pairs {
        let counts = per_expectation(&mut expectations, &manifest, &record);
        totals.absorb(counts);

        let cell = cells
            .entry((
                manifest.workflow.clone(),
                manifest.harness.clone(),
                manifest.arm,
            ))
            .or_insert_with(|| (0, Counts::default(), Resources::default()));
        cell.0 += 1;
        cell.1.absorb(counts);
        cell.2.absorb(&manifest);

        runs.push(RunRow {
            workflow: manifest.workflow,
            case: manifest.case,
            harness: manifest.harness,
            arm: manifest.arm,
            model: manifest.model,
            harness_version: manifest.harness_version,
            plugin_digest: manifest.plugin_digest,
            specification: record.specification,
            transcript_digest: manifest.transcript_digest,
            observed_at: manifest.observed_at,
            counts,
            cost_micro_usd: manifest.cost_micro_usd,
            tokens: manifest.tokens,
            wall_time_ms: manifest.wall_time_ms,
        });
    }

    runs.sort_by(|left, right| {
        (
            &left.workflow,
            &left.harness,
            left.arm,
            &left.case,
            &left.transcript_digest,
        )
            .cmp(&(
                &right.workflow,
                &right.harness,
                right.arm,
                &right.case,
                &right.transcript_digest,
            ))
    });

    Matrix {
        format: MATRIX_FORMAT,
        specifications: specifications
            .into_iter()
            .map(|(id, (digests, runs))| SpecificationRow {
                id,
                digest: digests.into_iter().next(),
                runs,
            })
            .collect(),
        runs,
        cells: cells
            .into_iter()
            .map(
                |((workflow, harness, arm), (runs, counts, resources))| Cell {
                    workflow,
                    harness,
                    arm,
                    runs,
                    counts,
                    resources,
                },
            )
            .collect(),
        expectations: expectations
            .into_iter()
            .map(
                |((workflow, expectation, harness, arm), (runs, counts))| ExpectationRow {
                    workflow,
                    expectation,
                    harness,
                    arm,
                    runs,
                    counts,
                },
            )
            .collect(),
        totals,
    }
}

// --- reading the pairs off the disk --------------------------------------------------------------

/// Finds every manifest under the paths the caller named, in a stable order.
///
/// A directory is read one level deep for `*.manifest.yaml`, which is the convention
/// `protocol evidence scan` already uses for markdown. A record with no manifest beside it is
/// refused rather than skipped: a dropped record is a run that silently left the matrix.
fn collect(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut manifests = BTreeSet::new();
    for path in paths {
        if path.is_dir() {
            let mut records = BTreeSet::new();
            let entries = std::fs::read_dir(path)
                .with_context(|| format!("reading the run directory {}", path.display()))?;
            for entry in entries {
                let entry =
                    entry.with_context(|| format!("reading an entry of {}", path.display()))?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(MANIFEST_SUFFIX) {
                    manifests.insert(entry.path());
                } else if name.ends_with(RECORD_SUFFIX) {
                    records.insert(entry.path());
                }
            }
            for record in records {
                let manifest = sibling(&record, RECORD_SUFFIX, MANIFEST_SUFFIX);
                if !manifest.exists() {
                    return Err(refused(
                        &record,
                        &[Refusal::ManifestMissing {
                            expected: manifest.display().to_string(),
                        }],
                    ));
                }
            }
        } else if path.to_string_lossy().ends_with(MANIFEST_SUFFIX) {
            manifests.insert(path.clone());
        } else {
            bail!(
                "{} is neither a directory of runs nor a `*{MANIFEST_SUFFIX}`. A run is two \
                 documents named alike: `<run>{MANIFEST_SUFFIX}` beside `<run>{RECORD_SUFFIX}`",
                path.display()
            );
        }
    }

    if manifests.is_empty() {
        return Err(refused(
            paths.first().unwrap_or(&PathBuf::new()),
            &[Refusal::NoRuns],
        ));
    }
    Ok(manifests.into_iter().collect())
}

/// The path of the other half of a pair.
fn sibling(path: &Path, from: &str, to: &str) -> PathBuf {
    let name = path.to_string_lossy();
    PathBuf::from(format!("{}{to}", name.trim_end_matches(from)))
}

/// Reads one pair: the manifest, then the record beside it.
fn read_pair(manifest_path: &Path) -> Result<(RunManifest, Record)> {
    let text = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("reading the manifest at {}", manifest_path.display()))?;
    let raw: RawRunManifest = serde_yaml::from_str(&text).with_context(|| {
        format!(
            "{} is not a `{MANIFEST_FORMAT}` document",
            manifest_path.display()
        )
    })?;
    let manifest =
        RunManifest::try_from(raw).map_err(|refusals| refused(manifest_path, &refusals))?;

    let record_path = sibling(manifest_path, MANIFEST_SUFFIX, RECORD_SUFFIX);
    if !record_path.exists() {
        return Err(refused(
            manifest_path,
            &[Refusal::RecordMissing {
                expected: record_path.display().to_string(),
            }],
        ));
    }
    let record_text = std::fs::read_to_string(&record_path)
        .with_context(|| format!("reading the record at {}", record_path.display()))?;
    let raw: RawRecord = serde_json::from_str(&record_text).with_context(|| {
        format!(
            "{} is not a `{REPORT_FORMAT}` record",
            record_path.display()
        )
    })?;
    let record = Record::try_from(raw).map_err(|refusals| refused(&record_path, &refusals))?;

    Ok((manifest, record))
}

// --- the verb -------------------------------------------------------------------------------------

/// How to render the matrix.
///
/// Its own enum rather than the crate's shared `Format`, on the reasoning `TraceFormat` gives: the
/// matrix is either read by a person, as three tables, or parsed by a program, as JSON. A third
/// rendering would be a third thing to keep in step with the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum MatrixFormat {
    /// Human-readable tables.
    Text,
    /// JSON, for another tool to read.
    Json,
}

/// What can be done with a set of evaluation runs.
#[derive(Debug, Subcommand)]
pub(crate) enum EvalCommand {
    /// Assemble the outcome matrix from run manifests and the records they accompany.
    ///
    /// One row per run, one cell per harness × arm × workflow, one row per expectation per cell —
    /// each of them three counts: held, contradicted, and nobody found out. No score, no ranking
    /// and no percentage: see the [module documentation](self) for why a scalar cannot be produced
    /// here honestly.
    Matrix(MatrixArgs),
    /// Run one arm of one case and leave the three documents `eval matrix` reads.
    ///
    /// The runner drives `metaharness` as a **tool**, the way this repository drives `git`: the
    /// binary is looked for on `PATH`, a machine without it is told so by name and exits `2`, and
    /// no crate crosses in either direction. Nothing spawns without `METAHARNESS_LIVE=1` and a
    /// `--budget-usd` cap; `--stream` ingests a run somebody already recorded and spends nothing.
    Run(RunArgs),
}

/// The arguments of `protocol eval matrix`.
#[derive(Debug, Args)]
pub(crate) struct MatrixArgs {
    /// The runs: a directory holding `*.manifest.yaml` beside `*.report.json`, or a manifest
    /// itself. Several may be named.
    #[arg(required = true)]
    runs: Vec<PathBuf>,
    /// How to render it.
    #[arg(long, value_enum, default_value_t = MatrixFormat::Text)]
    format: MatrixFormat,
    /// Where to write it. Without it, the matrix goes to standard output.
    #[arg(long)]
    out: Option<PathBuf>,
}

/// The `eval` verb family, one arm per subcommand.
pub(crate) fn run(command: EvalCommand) -> Result<ExitCode> {
    match command {
        EvalCommand::Matrix(args) => matrix(&args),
        EvalCommand::Run(args) => run_arm(&args),
    }
}

/// `protocol eval matrix`
fn matrix(args: &MatrixArgs) -> Result<ExitCode> {
    let manifests = collect(&args.runs)?;
    let mut pairs = Vec::new();
    for manifest in &manifests {
        pairs.push(read_pair(manifest)?);
    }

    let matrix = assemble(pairs)
        .map_err(|refusals| refused(args.runs.first().unwrap_or(&PathBuf::new()), &refusals))?;

    let document = match args.format {
        MatrixFormat::Text => to_text(&matrix),
        MatrixFormat::Json => {
            let mut json =
                serde_json::to_string_pretty(&matrix).context("rendering the matrix as JSON")?;
            json.push('\n');
            json
        }
    };

    match &args.out {
        Some(file) => {
            if let Some(parent) = file
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            std::fs::write(file, &document)
                .with_context(|| format!("writing {}", file.display()))?;
            outln!("{} — {} run(s)", file.display(), matrix.runs.len());
        }
        None => out!("{document}"),
    }

    // A matrix is a report, not a gate.
    Ok(ExitCode::SUCCESS)
}

// --- the human rendering -----------------------------------------------------------------------

/// Renders the matrix as three tables and one sentence.
fn to_text(matrix: &Matrix) -> String {
    let mut lines = vec![
        format!(
            "{} — {} run(s), {} specification(s), {} cell(s)",
            MATRIX_FORMAT,
            matrix.runs.len(),
            matrix.specifications.len(),
            matrix.cells.len()
        ),
        String::new(),
    ];

    lines.push(row(&[
        ("workflow", 24),
        ("harness", 9),
        ("arm", 8),
        ("runs", 5),
        ("held", 5),
        ("violated", 9),
        ("unobservable", 12),
    ]));
    for cell in &matrix.cells {
        lines.push(row(&[
            (&cell.workflow, 24),
            (&cell.harness, 9),
            (cell.arm.as_str(), 8),
            (&cell.runs.to_string(), 5),
            (&cell.counts.held.to_string(), 5),
            (&cell.counts.violated.to_string(), 9),
            (&cell.counts.unobservable.to_string(), 12),
        ]));
    }

    lines.push(String::new());
    lines.push(row(&[
        ("expectation", 28),
        ("harness", 9),
        ("arm", 8),
        ("runs", 5),
        ("held", 5),
        ("violated", 9),
        ("unobservable", 12),
    ]));
    for expectation in &matrix.expectations {
        lines.push(row(&[
            (&expectation.expectation, 28),
            (&expectation.harness, 9),
            (expectation.arm.as_str(), 8),
            (&expectation.runs.to_string(), 5),
            (&expectation.counts.held.to_string(), 5),
            (&expectation.counts.violated.to_string(), 9),
            (&expectation.counts.unobservable.to_string(), 12),
        ]));
    }

    lines.push(String::new());
    lines.push("what the runs that recorded it cost".to_owned());
    lines.push(row(&[
        ("workflow", 24),
        ("harness", 9),
        ("arm", 8),
        ("cost", 18),
        ("tokens", 18),
        ("wall time", 18),
    ]));
    for cell in &matrix.cells {
        lines.push(row(&[
            (&cell.workflow, 24),
            (&cell.harness, 9),
            (cell.arm.as_str(), 8),
            (&cost(cell.resources.cost_micro_usd, cell.runs), 18),
            (&quantity(cell.resources.tokens, cell.runs, ""), 18),
            (&quantity(cell.resources.wall_time_ms, cell.runs, " ms"), 18),
        ]));
    }

    lines.push(String::new());
    lines.push(format!(
        "{} fact(s) held, {} contradicted, {} nobody found out, over {} run(s). No arm is ranked \
         and no score is computed: an expectation nobody could decide is not a failure, and \
         folding it into one would be the only way to get a single number out of this.",
        matrix.totals.held,
        matrix.totals.violated,
        matrix.totals.unobservable,
        matrix.runs.len()
    ));

    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// One table row, left-aligned, single-spaced by the widths given.
fn row(columns: &[(&str, usize)]) -> String {
    let rendered: Vec<String> = columns
        .iter()
        .map(|(text, width)| format!("{text:<width$}"))
        .collect();
    rendered.join("  ").trim_end().to_owned()
}

/// A cost column: dollars from micro-dollars by integer arithmetic, never a float.
fn cost(reported: Option<Reported>, runs: usize) -> String {
    reported.map_or_else(
        || "—".to_owned(),
        |reported| {
            format!(
                "${}.{:06} {}",
                reported.total / MICRO_USD,
                reported.total % MICRO_USD,
                coverage(reported.runs, runs)
            )
        },
    )
}

/// A quantity column, with the runs it covers.
fn quantity(reported: Option<Reported>, runs: usize, unit: &str) -> String {
    reported.map_or_else(
        || "—".to_owned(),
        |reported| format!("{}{unit} {}", reported.total, coverage(reported.runs, runs)),
    )
}

/// `(2/3)`, and nothing at all when every run in the cell answered.
fn coverage(reporting: usize, runs: usize) -> String {
    if reporting == runs {
        String::new()
    } else {
        format!("({reporting}/{runs})")
    }
}

// --- the runner ---------------------------------------------------------------------------------
//
// Everything below is `protocol eval run`: the verb that produces the pairs everything above reads.
//
// # The manifest is assembled runner-side, and the seam grew nothing (decision R3.2)
//
// A run manifest has two kinds of field, and the split is the whole decision. The fields that
// **describe the run** — the harness's version, the digest of the plugin that was installed, what
// the transcript is — are facts about a session this process did not conduct, so they are read out
// of the stream metaharness already emits: `session.started` states the adapter, its version, the
// model it resolved (or `null`, which is an answer) and — in `hermetic.installed_plugins`, the
// instrument's own row rather than the vendor echo beside it — what was injected into the scratch
// home; and the check this runner performs over the whole stream states the transcript's digest.
// The fields only the **runner** knows — which arm this run
// belongs to, which case it is a run of, which workflow that case is about, and when a person
// observed it — are its own, because nothing in the stream could know them: metaharness runs a
// session, and *this session is arm b of case X* is a claim about an experiment.
//
// So no event gained a field and no crossing was added. The alternative — metaharness emitting an
// `eval.run-manifest/1` fragment — would have put this repository's experiment vocabulary (`raw`,
// `plugin`, `driven`) into a repository that has no business knowing it, and would have made every
// change to the manifest a two-repository release.
//
// The rule that keeps this honest is the fail-closed one: a stream whose `session.started` does not
// state what the manifest needs is **refused by name** ([`StreamRefusal`]), and no manifest is
// written. A runner that filled a hole with a plausible value would be writing the one document the
// matrix trusts.

/// The binary the runner drives, the way this repository drives `git`.
const METAHARNESS_BINARY: &str = "metaharness";

/// Where to look for that binary when it is not on `PATH` under its own name.
const METAHARNESS_BIN_ENV: &str = "METAHARNESS_BIN";

/// The environment variable that must say `1` before anything is spawned and paid for.
const METAHARNESS_LIVE_ENV: &str = "METAHARNESS_LIVE";

/// The exit code for *the tool this verb drives is not installed*.
///
/// Distinct from `1`, which is every refusal about a document, because the two want different
/// reactions: one is *install something*, the other is *fix what you wrote*. It is the code the
/// programme's design constant 4 asks for — an absent binary is a skip, never a red gate — so a
/// caller can tell the two apart without reading prose off stderr.
const TOOL_MISSING_EXIT: u8 = 2;

/// How a run's raw event stream is named on disk.
const EVENTS_SUFFIX: &str = ".events.jsonl";

/// What a run whose stream states no cost is counted at, in US dollars.
///
/// Conservative rather than accurate, and it is a budget input rather than a measurement: a run
/// whose cost nobody recorded is counted against the cap at this rate so that an unpriced wire
/// cannot spend without limit. It never reaches a manifest — `cost_micro_usd` stays absent there,
/// because the matrix reports totals over the runs that stated one and an assumed number would
/// silently become a measurement.
const ASSUMED_USD_PER_RUN: &str = "0.25";

/// Which harness a run is of.
///
/// Closed here and open in the manifest, and the asymmetry is the difference between reading a run
/// and launching one. The matrix takes `harness` as free text because a third harness is a run and
/// not a redesign; the **runner** has to know which plugin directory arm `plugin` injects and which
/// vendor word `metaharness run` takes, and a harness it cannot name is one it cannot treat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Harness {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
    /// The b10x harness: our own agent loop, over a model API directly.
    B10x,
}

impl Harness {
    /// The word `metaharness run` takes, and the word the manifest writes.
    fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::B10x => "b10x",
        }
    }

    /// The plugin directory arm `plugin` injects for this harness.
    fn plugin_dir(self) -> &'static str {
        match self {
            Self::Claude => "integrations/claude-code",
            Self::Codex => "integrations/codex",
            // There is no plugin, and there is no arm that would inject one. `b10x` holds its own
            // loop, so `plugin` - *the shipped plugin is installed and nothing decides the agent's
            // calls* - has no meaning for it: there is no vendor surface to install into. The path
            // is named so the type stays total; a run that reached for it is refused by the arm
            // check before it gets here.
            Self::B10x => "integrations/b10x",
        }
    }
}

impl fmt::Display for Harness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// --- what the runner refuses before anything is spawned -----------------------------------------

/// Every way the runner refuses to start, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RunRefusal {
    /// The binary this verb drives is not installed.
    ToolMissing {
        /// What was looked for, and where.
        looked_for: String,
    },
    /// A live spawn was asked for and the environment does not permit one.
    NotLive,
    /// A live spawn was asked for with no cap on what it may spend.
    NoBudget,
    /// Arm `driven` is not launched from here.
    DrivenIsNotLaunchedHere,
    /// Arm `native` is not launched from here either, and for a different reason.
    NativeIsNotLaunchedHere,
    /// A live spawn was asked for with no tree for the session to work in.
    NoWorkingTree,
    /// The cap would be exceeded by the next run.
    BudgetWouldBeExceeded {
        /// What has been spent, in millionths of a US dollar.
        spent: u64,
        /// What the next run is counted at.
        next: u64,
        /// The cap.
        cap: u64,
        /// How many runs were launched before the stop.
        launched: usize,
        /// How many were not.
        skipped: usize,
    },
    /// Nothing was named to run.
    NoCase,
    /// A recorded stream describes one run, and more than one was named.
    StreamIsOneRun {
        /// How many runs were named.
        named: usize,
    },
    /// The workflow's rendered instructions — arm `raw`'s whole treatment — are not there.
    InstructionsMissing {
        /// The workflow.
        workflow: String,
        /// Where they were looked for.
        expected: String,
    },
    /// The tool ran and did not finish.
    SpawnFailed {
        /// What it exited with.
        status: String,
        /// The last of what it said.
        tail: String,
        /// Where the stream it did write is.
        stream: String,
    },
}

impl RunRefusal {
    /// The stable code a test matches on.
    fn code(&self) -> &'static str {
        match self {
            Self::ToolMissing { .. } => "EVAL-RUN-001",
            Self::NotLive => "EVAL-RUN-002",
            Self::NoBudget => "EVAL-RUN-003",
            Self::DrivenIsNotLaunchedHere => "EVAL-RUN-004",
            Self::NoWorkingTree => "EVAL-RUN-005",
            Self::BudgetWouldBeExceeded { .. } => "EVAL-RUN-006",
            Self::NoCase => "EVAL-RUN-007",
            Self::StreamIsOneRun { .. } => "EVAL-RUN-008",
            Self::InstructionsMissing { .. } => "EVAL-RUN-009",
            Self::SpawnFailed { .. } => "EVAL-RUN-010",
            Self::NativeIsNotLaunchedHere => "EVAL-RUN-011",
        }
    }
}

impl fmt::Display for RunRefusal {
    // One arm per refusal, each carrying the whole sentence a person reads when a run will not
    // start. Splitting it would put half the sentences somewhere else without making any of them
    // shorter, and this is the one place where reading them all together is the point.
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.code())?;
        match self {
            Self::ToolMissing { looked_for } => write!(
                f,
                "`{METAHARNESS_BINARY}` is not on PATH — the eval runner drives it as a tool \
                 ({looked_for}).\n\
                 \n\
                 Every arm of the evaluation is spawned by metaharness into a hermetic scratch \
                 home, because that is what makes the arms comparable: the instrument is constant \
                 and only the treatment varies. There is no second launcher to fall back to.\n\
                 \n\
                 Install it with `cargo install --path crates/metaharness-cli` from a metaharness \
                 checkout, or point `{METAHARNESS_BIN_ENV}` at the binary. `--stream FILE` needs \
                 neither: it ingests a stream somebody already recorded."
            ),
            Self::NotLive => write!(
                f,
                "a spawn costs money and `{METAHARNESS_LIVE_ENV}=1` is not in this environment. \
                 Set it deliberately, or pass `--stream FILE` to ingest a run that already \
                 happened, which spends nothing"
            ),
            Self::NoBudget => write!(
                f,
                "a spawn needs `--budget-usd`. A paid sweep with no cap is the failure this \
                 programme wrote a cap into its plan to avoid — the runner reads each run's cost \
                 out of its own stream and stops launching when the next one would exceed what you \
                 named"
            ),
            Self::NativeIsNotLaunchedHere => write!(
                f,
                "arm `native` is not launched by this verb — `b10x-harness` is its own loop and \
                 launches itself. Every other arm is a treatment applied to a vendor harness that \
                 `metaharness` drives from outside; `native` has no vendor harness in it, so there \
                 is nothing here to drive. Spawning one from this verb would mean this binary held \
                 a second launcher for a component that already has one.\n\
                 \n\
                 What this verb does with a native run is **read** it: run it with `b10x-harness`, \
                 then ingest the event stream with `protocol eval run --arm native --stream <file>`."
            ),
            Self::DrivenIsNotLaunchedHere => write!(
                f,
                "arm `driven` is not launched by this verb — `protocol drive run` launches it. A \
                 driven run is a walk of a step map whose every `llm` step is spawned through the \
                 seam with the engine deciding each call, and a second way to launch one would be a \
                 second policy to forget, which is the mistake `epic:metaharness-migration` \
                 retired.\n\
                 \n\
                 What this verb does with a driven run is **read** it: drive it with `protocol \
                 drive run`, then ingest the event stream that run wrote with `protocol eval run \
                 --arm driven --stream <the stream>`"
            ),
            Self::NoWorkingTree => write!(
                f,
                "a spawn needs `--cwd DIR`, the tree the session works in. There is no default, \
                 and the missing default is the point: arm `raw`'s agent is given a shell and no \
                 enforcement, and the checkout holding the specification it is being measured \
                 against is the last directory it should be started in"
            ),
            Self::BudgetWouldBeExceeded {
                spent,
                next,
                cap,
                launched,
                skipped,
            } => write!(
                f,
                "the cap stopped the sweep after {launched} run(s), with {skipped} not launched: \
                 {} spent and the next run counted at {} would pass the cap of {}",
                dollars(*spent),
                dollars(*next),
                dollars(*cap)
            ),
            Self::NoCase => write!(
                f,
                "name what to run: `--case DIR` (repeatable) or `--workflow ID` for every case of \
                 one workflow. Running the whole corpus because nobody said otherwise is a bill \
                 nobody asked for"
            ),
            Self::StreamIsOneRun { named } => write!(
                f,
                "`--stream` ingests one recorded run and {named} runs were named. A stream is one \
                 session; ingest them one at a time, or drop `--stream` and let the runner spawn \
                 them"
            ),
            Self::InstructionsMissing {
                workflow,
                expected,
            } => write!(
                f,
                "arm `raw` gives the agent the committed instructions for `{workflow}` and there \
                 is no document at {expected}. Arm `raw` *is* those instructions — a run of it \
                 without them is a run of no arm at all. Render them with `protocol workflow \
                 instruct --out generated/instructions`"
            ),
            Self::SpawnFailed {
                status,
                tail,
                stream,
            } => write!(
                f,
                "metaharness exited {status}; {}the stream it wrote is at {stream}. No manifest \
                 was assembled, because a run the tool says did not finish is not a run to put in \
                 a matrix — read the stream, and ingest it with `--stream` if it turns out to hold \
                 a whole session",
                if tail.is_empty() {
                    String::new()
                } else {
                    format!("it said: {tail}; ")
                }
            ),
        }
    }
}

/// Every way the case document is refused, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CaseRefusal {
    /// The document does not claim to be an eval case.
    NotACase {
        /// What it claimed instead.
        found: Option<String>,
    },
    /// A field the runner needs is not there, or says nothing.
    FieldMissing {
        /// Which one.
        field: &'static str,
    },
    /// The directory holds no `case.yaml`.
    NoManifest {
        /// Where one was looked for.
        expected: String,
    },
}

impl CaseRefusal {
    /// The stable code a test matches on.
    fn code(&self) -> &'static str {
        match self {
            Self::NotACase { .. } => "EVAL-CASE-001",
            Self::FieldMissing { .. } => "EVAL-CASE-002",
            Self::NoManifest { .. } => "EVAL-CASE-003",
        }
    }
}

impl fmt::Display for CaseRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.code())?;
        match self {
            Self::NotACase { found } => write!(
                f,
                "this is not an `{CASE_FORMAT}` document{}",
                claimed(found.as_deref())
            ),
            Self::FieldMissing { field } => write!(
                f,
                "the case states no `{field}`, and the runner writes it into the manifest of every \
                 run of this case"
            ),
            Self::NoManifest { expected } => write!(
                f,
                "there is no case here: {expected} does not exist. A case is a directory holding a \
                 `case.yaml`, its expectations and its transcript"
            ),
        }
    }
}

/// Every way a recorded stream is refused where the manifest is assembled from it.
///
/// This is the boundary decision R3.2 turns on: a manifest field that describes the run is read out
/// of the stream, so a stream that does not state one is refused **here**, by name, and no manifest
/// exists for the matrix to trust.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StreamRefusal {
    /// The bytes are not a transcript this build can read.
    Unreadable {
        /// What the reader said.
        reason: String,
    },
    /// A line of the stream is not a JSON object.
    LineNotAnObject {
        /// Which line, counting from one.
        line: usize,
    },
    /// The stream never opens a session.
    NoSessionStarted,
    /// `session.started` states no field the manifest needs.
    SessionSaysNothing {
        /// Which one.
        field: &'static str,
    },
    /// The stream is a run of another harness than the one this run claims.
    HarnessMismatch {
        /// What was asked for.
        asked: String,
        /// What the stream says.
        stream: String,
    },
    /// Arm `plugin` is the arm whose treatment is the plugin, and this stream attests none.
    PluginUnattestedOnPluginArm,
    /// Arm `raw` is the arm with no plugin in it, and this stream attests one.
    PluginAttestedOnRawArm {
        /// Where it came from.
        source: String,
    },
    /// A plugin was installed and the attestation does not say which bytes.
    PluginWithoutDigest {
        /// Its name.
        name: String,
    },
    /// More than one plugin was installed, and a manifest carries one digest.
    SeveralPluginsAttested {
        /// Their names.
        names: Vec<String>,
    },
    /// The stream stops before the session ends.
    NoTerminalEvent,
    /// The terminal event states a cost this reader cannot convert.
    CostUnreadable {
        /// Why.
        reason: String,
    },
    /// The manifest the runner assembled is one the matrix's own reader refuses.
    ManifestUnreadable {
        /// Why.
        reason: String,
    },
}

impl StreamRefusal {
    /// The stable code a test matches on.
    fn code(&self) -> &'static str {
        match self {
            Self::Unreadable { .. } => "EVAL-STREAM-001",
            Self::LineNotAnObject { .. } => "EVAL-STREAM-002",
            Self::NoSessionStarted => "EVAL-STREAM-003",
            Self::SessionSaysNothing { .. } => "EVAL-STREAM-004",
            Self::HarnessMismatch { .. } => "EVAL-STREAM-005",
            Self::PluginUnattestedOnPluginArm => "EVAL-STREAM-006",
            Self::PluginAttestedOnRawArm { .. } => "EVAL-STREAM-007",
            Self::PluginWithoutDigest { .. } => "EVAL-STREAM-008",
            Self::SeveralPluginsAttested { .. } => "EVAL-STREAM-009",
            Self::NoTerminalEvent => "EVAL-STREAM-010",
            Self::CostUnreadable { .. } => "EVAL-STREAM-011",
            Self::ManifestUnreadable { .. } => "EVAL-STREAM-012",
        }
    }
}

impl fmt::Display for StreamRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.code())?;
        match self {
            Self::Unreadable { reason } => write!(
                f,
                "this is not a transcript this build can read: {reason}"
            ),
            Self::LineNotAnObject { line } => write!(
                f,
                "line {line} of the stream is not a JSON object, so the run it describes cannot be \
                 read"
            ),
            Self::NoSessionStarted => write!(
                f,
                "the stream carries no `session.started`, which is where a run says which harness \
                 ran it, at which version, on which model and with which plugins installed. \
                 Without it there is no manifest to write and nothing to guess from"
            ),
            Self::SessionSaysNothing { field } => write!(
                f,
                "`session.started` states no `{field}`, and the manifest's own field is read out of \
                 it. A runner that filled the hole with a plausible value would be writing the one \
                 document the matrix trusts.\n\
                 \n\
                 A key written as an explicit `null` is a different finding from a key that is not \
                 there, and only `model` may answer `null` — a wire that states no model is stating \
                 something, and the manifest writes it down as `model: null`"
            ),
            Self::HarnessMismatch { asked, stream } => write!(
                f,
                "this run claims harness `{asked}` and the stream's `session.started` says \
                 `{stream}`. One of the two is about another run"
            ),
            Self::PluginUnattestedOnPluginArm => write!(
                f,
                "arm `plugin` is the arm whose treatment is the plugin, and this stream attests \
                 none: `session.started.{INSTALLED_PLUGINS}` is empty. That is the treated arm \
                 without its treatment, which would enter the matrix as a measurement of the plugin \
                 and be a measurement of nothing.\n\
                 \n\
                 The row read here is the **instrument's**, not the vendor's own `plugins` echo: \
                 metaharness writes what it injected on every adapter, and a vendor that states \
                 nothing writes `null` rather than a minted list"
            ),
            Self::PluginAttestedOnRawArm { source } => write!(
                f,
                "arm `raw` is the arm with no plugin in it, and this stream attests `{source}`. \
                 That is the control arm with the treatment applied — either the run was arm \
                 `plugin`, or the wrong stream is being ingested"
            ),
            Self::PluginWithoutDigest { name } => write!(
                f,
                "the stream attests the plugin `{name}` and states no `digest` for it. A manifest \
                 that cannot say **which bytes** were installed cannot say what was measured, and \
                 an edited plugin would be indistinguishable from the shipped one"
            ),
            Self::SeveralPluginsAttested { names } => write!(
                f,
                "the stream attests {} installed plugins ({}) and a manifest carries one \
                 `plugin_digest`. Which of them was the treatment is not this reader's guess to \
                 make",
                names.len(),
                names.join(", ")
            ),
            Self::NoTerminalEvent => write!(
                f,
                "the stream carries no `session.ended`, so it is a run that stopped rather than a \
                 run that finished. Its cost, its tokens and its wall time are in that event, and a \
                 manifest written without it would report a partial run as a whole one"
            ),
            Self::CostUnreadable { reason } => write!(
                f,
                "the run's terminal event states a cost this reader cannot convert: {reason}.\n\
                 \n\
                 It is refused rather than read as a run that priced nothing, because those are \
                 different facts and only one of them may be charged at `--assume-usd-per-run`. A \
                 stated cost that quietly became an assumption is how a sweep under-reports what it \
                 spent"
            ),
            Self::ManifestUnreadable { reason } => write!(
                f,
                "the runner assembled a manifest its own reader refuses, which is a defect here \
                 and not in anything you passed: {reason}"
            ),
        }
    }
}

/// The format claim an eval case carries.
const CASE_FORMAT: &str = "eval-case/1";

// --- the case ------------------------------------------------------------------------------------

/// An eval case as it is written down.
///
/// Only the four fields the runner needs, and deliberately **not** `deny_unknown_fields`: the
/// corpus's shape has an owner already — `crates/protocol-cli/tests/eval_corpus.rs` reads every
/// field and denies the unknown — and a second denier would be a second place to update when a case
/// grows a key, which is how two readers of one document start disagreeing.
#[derive(Debug, Deserialize)]
struct RawCase {
    /// The format claim.
    format: Option<String>,
    /// The case's id, which must be its directory's name.
    id: Option<String>,
    /// The workflow it is a run of.
    workflow: Option<String>,
    /// What the agent is asked to do.
    task: Option<String>,
    /// The `trace-spec/1` document it is judged by, relative to the case directory.
    expectations: Option<String>,
}

/// A case, once read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Case {
    /// Its id.
    id: String,
    /// The workflow.
    workflow: String,
    /// The task statement.
    task: String,
    /// The document it is judged by.
    expectations: PathBuf,
}

/// Reads the case in a directory.
fn read_case(directory: &Path) -> Result<Case> {
    let manifest = directory.join("case.yaml");
    if !manifest.exists() {
        return Err(refused_run(
            directory,
            &[CaseRefusal::NoManifest {
                expected: manifest.display().to_string(),
            }],
        ));
    }
    let text = std::fs::read_to_string(&manifest)
        .with_context(|| format!("reading the case at {}", manifest.display()))?;
    let raw: RawCase = serde_yaml::from_str(&text)
        .with_context(|| format!("{} is not an `{CASE_FORMAT}` document", manifest.display()))?;

    let mut refusals = Vec::new();
    if raw.format.as_deref() != Some(CASE_FORMAT) {
        refusals.push(CaseRefusal::NotACase { found: raw.format });
    }
    let mut want = |field: &'static str, written: Option<String>| -> Option<String> {
        match written {
            Some(value) if !value.trim().is_empty() => Some(value),
            _ => {
                refusals.push(CaseRefusal::FieldMissing { field });
                None
            }
        }
    };
    let id = want("id", raw.id);
    let workflow = want("workflow", raw.workflow);
    let task = want("task", raw.task);
    let expectations = want("expectations", raw.expectations);

    if !refusals.is_empty() {
        return Err(refused_run(&manifest, &refusals));
    }
    Ok(Case {
        id: id.expect("an id was read"),
        workflow: workflow.expect("a workflow was read"),
        task: task.expect("a task was read"),
        expectations: directory.join(expectations.expect("a document was named")),
    })
}

/// Every refusal in one message, in this binary's shape for a refused document.
fn refused_run<R: fmt::Display>(subject: &Path, refusals: &[R]) -> anyhow::Error {
    let lines: Vec<String> = refusals
        .iter()
        .map(|refusal| format!("  {refusal}"))
        .collect();
    anyhow::anyhow!(
        "{} — {} refusal(s):\n{}",
        subject.display(),
        refusals.len(),
        lines.join("\n")
    )
}

/// The cases this invocation is about, in a stable order.
fn select_cases(args: &RunArgs) -> Result<Vec<Case>> {
    let mut cases = Vec::new();
    for directory in &args.cases {
        cases.push(read_case(directory)?);
    }

    if let Some(workflow) = &args.workflow {
        let mut directories: Vec<PathBuf> = std::fs::read_dir(&args.corpus)
            .with_context(|| format!("reading the corpus at {}", args.corpus.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("reading the corpus at {}", args.corpus.display()))?
            .into_iter()
            .filter(|path| path.is_dir())
            .collect();
        directories.sort();
        for directory in directories {
            let case = read_case(&directory)?;
            if &case.workflow == workflow {
                cases.push(case);
            }
        }
    }

    if cases.is_empty() {
        return Err(refused_run(&args.corpus, &[RunRefusal::NoCase]));
    }
    Ok(cases)
}

// --- what the stream says -------------------------------------------------------------------------

/// The half of a run manifest that is read out of the stream metaharness emits.
///
/// See the section comment above for why these five fields are here and the other five are the
/// runner's own. Every one of them is required, and a stream that states none of them is refused
/// rather than filled in: this document is what a later reader joins the matrix's rows by.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Session {
    /// The harness and its version, as the manifest spells it: `claude 2.1.239`.
    harness_version: String,
    /// The model the harness resolved, where it said which.
    ///
    /// Read from the stream and **not** from what the runner asked for, which is the narrowing this
    /// story makes to the plan's field list: a runner writing down the model it requested would
    /// record a model the run may not have used, and `claude-sonnet-5` resolving to something else
    /// is exactly the fact a later reader of the matrix needs to see.
    ///
    /// [`None`] is a **stated** absence and not a hole. Codex's wire names no model at session
    /// start — the whole of a 62-event pilot run never states one — so the honest manifest says
    /// `model: null`. Inventing `gpt-5-codex` there because it is the likely answer would be
    /// writing the one document the matrix trusts.
    model: Option<String>,
    /// The plugin that was installed, where one was.
    plugin_digest: Option<String>,
    /// What the run cost, in millionths of a US dollar, totalled over every session that said.
    ///
    /// # Every session, and the run that made that matter
    ///
    /// A stream is usually one session and the total is that session's figure. A **driven** run is
    /// not: `protocol drive` starts a fresh session per workflow state, so its transcript is a
    /// concatenation carrying one terminal record per state. This reader took the *last* of them
    /// until 2026-08-23, when the first live driven run reported `$1.135363` for a walk that had
    /// cost `$15.014604` across six sessions — the sixth session's figure, presented as the run's.
    ///
    /// [`accumulate`] holds the absence rule: a session stating nothing adds nothing and never a
    /// zero, so a total is over the sessions that stated one.
    cost_micro_usd: Option<u64>,
    /// How many tokens it used, totalled over every session that said.
    tokens: Option<u64>,
    /// How long it took, totalled over every session that said.
    ///
    /// A sum and not a span: the driver runs its sessions one after another, and nothing here reads
    /// a clock to find out (invariant 9).
    wall_time_ms: Option<u64>,
}

/// The four token counts a `usage` object carries, which is what the manifest's `tokens` totals.
const TOKEN_KEYS: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_read_input_tokens",
    "cache_creation_input_tokens",
];

impl Session {
    /// Reads the manifest's stream-side fields, or every reason they cannot be read.
    ///
    /// Fail-closed throughout, and the arm is an input because two of the rules are about the
    /// experiment rather than about the document: the treated arm without its treatment and the
    /// control arm with one are both refused here, which is crossing #4 arriving on this side.
    fn read(events: &[u8], arm: Arm, harness: Harness) -> Result<Self, Vec<StreamRefusal>> {
        let text = String::from_utf8_lossy(events);
        let mut started: Option<serde_json::Value> = None;
        let mut ended: Vec<serde_json::Value> = Vec::new();
        let mut refusals = Vec::new();

        for (offset, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                refusals.push(StreamRefusal::LineNotAnObject { line: offset + 1 });
                continue;
            };
            match value.get("event").and_then(serde_json::Value::as_str) {
                Some("session.started") if started.is_none() => started = Some(value),
                Some("session.ended") => ended.push(value),
                _ => {}
            }
        }

        let Some(started) = started else {
            refusals.push(StreamRefusal::NoSessionStarted);
            return Err(refusals);
        };

        let mut word_at = |field: &'static str| -> Option<String> {
            match started.get(field).and_then(serde_json::Value::as_str) {
                Some(value) if !value.trim().is_empty() => Some(value.to_owned()),
                _ => {
                    refusals.push(StreamRefusal::SessionSaysNothing { field });
                    None
                }
            }
        };
        let adapter = word_at("adapter");
        let version = word_at("harness_version");
        // The one field of the three that may answer `null`, and it must still be *written*: an
        // adapter that dropped the key has told us nothing, and one that wrote `null` has told us
        // that its vendor does not say at session start.
        let model = match started.get("model") {
            Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(named)) if !named.trim().is_empty() => {
                Some(named.clone())
            }
            // An absent key and a key holding something that is not a model name are one answer:
            // nobody wrote a model down here. Only the explicit `null` above is the other one.
            _ => {
                refusals.push(StreamRefusal::SessionSaysNothing { field: "model" });
                None
            }
        };

        if let Some(adapter) = adapter.as_deref() {
            if adapter != harness.as_str() {
                refusals.push(StreamRefusal::HarnessMismatch {
                    asked: harness.to_string(),
                    stream: adapter.to_owned(),
                });
            }
        }

        let plugin_digest = plugin_attestation(&mut refusals, &started, arm);

        if ended.is_empty() {
            refusals.push(StreamRefusal::NoTerminalEvent);
            return Err(refusals);
        }

        // Read before the emptiness check, so an unreadable cost joins the other refusals rather
        // than being discovered after them (invariant 3: validation accumulates).
        let mut cost = None;
        let mut tokens = None;
        let mut wall_time_ms = None;
        for ended in &ended {
            match cost_of(ended) {
                Ok(figure) => accumulate(&mut cost, figure),
                Err(reason) => refusals.push(StreamRefusal::CostUnreadable { reason }),
            }
            accumulate(&mut tokens, tokens_of(ended));
            accumulate(
                &mut wall_time_ms,
                ended.get("duration_ms").and_then(serde_json::Value::as_u64),
            );
        }

        if !refusals.is_empty() {
            return Err(refusals);
        }

        Ok(Self {
            // `claude 2.1.239`, which is the spelling the committed manifests already use: the
            // version alone would not say whose it was, and two harnesses at `0.145.0` are not the
            // same pin.
            harness_version: format!(
                "{} {}",
                adapter.expect("an adapter was read"),
                version.expect("a version was read")
            ),
            model,
            plugin_digest,
            cost_micro_usd: cost,
            tokens,
            wall_time_ms,
        })
    }
}

/// Adds one session's quantity to a run's total, where the session stated one.
///
/// Absent stays absent: a stream whose sessions all say `null` totals `None` and never `0`, which
/// is the same rule [`add`] applies one level up when a cell totals over its runs.
fn accumulate(total: &mut Option<u64>, stated: Option<u64>) {
    let Some(value) = stated else { return };
    let running = total.get_or_insert(0);
    *running = running.saturating_add(value);
}

/// Where the instrument attests what it installed, as against what a vendor happened to echo.
const INSTALLED_PLUGINS: &str = "hermetic.installed_plugins";

/// Reads the installed-plugin attestation, and applies the two rules that are about the experiment.
///
/// **Crossing #4, on this side.** metaharness's `--plugin-dir` copies a plugin into the scratch home
/// and attests what it installed; this reads the digest out of that attestation byte for byte and
/// writes it into the manifest. Nothing derives it, nothing recomputes it from the directory on
/// disk, and that is the point: the digest is a claim about the bytes the **session** was given, and
/// a runner that hashed its own copy would be attesting a file the run never saw.
///
/// # It is `hermetic.installed_plugins`, and not the top-level `plugins`
///
/// `session.started` carries both, and they answer different questions. Top-level `plugins` is the
/// **vendor's own init list**, echoed: Claude Code writes one, and Codex writes `null` because its
/// vendor states nothing and metaharness will not mint a vendor field it did not receive — a9
/// discipline, the same rule that leaves `thinking_tokens` null rather than zero.
/// `hermetic.installed_plugins` is the **instrument's** record of what *it* injected, and it is
/// always written, on every adapter.
///
/// This reader read the vendor echo until the first live pilot run, where it cost two refusals on a
/// Codex arm-a run that was perfectly well-formed (`plugins: null`, `installed_plugins: []`). The
/// boundary refusing to guess was right; the field it was reading was wrong. What makes the
/// instrument's row the correct one is not that it is populated more often — it is that the question
/// this manifest asks is *what was this run given*, and only one of the two rows is an answer to it
/// from something that knows.
fn plugin_attestation(
    refusals: &mut Vec<StreamRefusal>,
    started: &serde_json::Value,
    arm: Arm,
) -> Option<String> {
    let Some(entries) = started
        .get("hermetic")
        .and_then(|hermetic| hermetic.get("installed_plugins"))
        .and_then(serde_json::Value::as_array)
    else {
        refusals.push(StreamRefusal::SessionSaysNothing {
            field: INSTALLED_PLUGINS,
        });
        return None;
    };

    if entries.len() > 1 {
        refusals.push(StreamRefusal::SeveralPluginsAttested {
            names: entries.iter().map(plugin_name).collect(),
        });
        return None;
    }

    let Some(entry) = entries.first() else {
        if arm == Arm::Plugin {
            refusals.push(StreamRefusal::PluginUnattestedOnPluginArm);
        }
        return None;
    };

    if arm == Arm::Raw {
        refusals.push(StreamRefusal::PluginAttestedOnRawArm {
            source: entry
                .get("source")
                .and_then(serde_json::Value::as_str)
                .map_or_else(|| plugin_name(entry), ToOwned::to_owned),
        });
        return None;
    }

    match entry.get("digest").and_then(serde_json::Value::as_str) {
        Some(digest) if !digest.trim().is_empty() => Some(digest.to_owned()),
        _ => {
            refusals.push(StreamRefusal::PluginWithoutDigest {
                name: plugin_name(entry),
            });
            None
        }
    }
}

/// A plugin entry's name, however the attestation spelled it.
fn plugin_name(entry: &serde_json::Value) -> String {
    entry
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("(unnamed)")
        .to_owned()
}

/// The run's cost as millionths of a dollar, where the terminal event states one.
///
/// A `null` — which is what a Codex stream writes today — reads as **unknown**, so the manifest
/// states no cost and the matrix's total says how many runs it covers. It is never read as free.
fn cost_of(ended: &serde_json::Value) -> Result<Option<u64>, String> {
    match ended.get("total_cost_usd") {
        None | Some(serde_json::Value::Null) => Ok(None),
        // The number's own decimal text, rounded to the nearest millionth by integer arithmetic.
        Some(serde_json::Value::Number(stated)) => micro_usd_stated(&stated.to_string()).map(Some),
        Some(other) => Err(format!(
            "`total_cost_usd` is {other}, which is neither a number nor `null`"
        )),
    }
}

/// A cost a **wire** stated, as millionths of a dollar, rounded to the nearest one.
///
/// [`micro_usd`]'s sibling, and the split between them is the whole of this fix. That one reads an
/// amount a **person typed** — `--budget-usd 5.00` — and refuses anything it cannot convert exactly,
/// because a human who typed `1e-7` has made a mistake worth naming. This one reads a number a
/// harness computed, and a harness computes in binary floating point: a live Claude run stated
/// `0.7977854999999999`, which is the shortest text that round-trips the `f64` sum of its per-turn
/// costs. Refusing that is refusing a perfectly good cost for having more precision than the
/// document it is going into.
///
/// So the fraction is taken to six places and the remainder decides the last one — half-up, by
/// comparing digits, never `value * 1_000_000.0`. `0.7977854999999999` becomes `797785`, and
/// `0.9999995` carries into the dollar because the addition is ordinary integer addition.
///
/// # Errors
///
/// When the text is not a plain decimal — an exponent, most likely. **Refused rather than answered
/// [`None`]**, which is the actual defect this pair of functions was split to fix: `.ok()` on the
/// strict reader turned *there is a number here I cannot convert* into *there is no number*, and a
/// run that cost eighty cents entered the ledger at the assumed rate and its manifest with no cost
/// at all. Unreadable is not unstated, on exactly invariant 5's reasoning one domain out.
fn micro_usd_stated(written: &str) -> Result<u64, String> {
    let text = written.trim();
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let readable = !whole.is_empty()
        && whole.chars().all(|character| character.is_ascii_digit())
        && fraction.chars().all(|character| character.is_ascii_digit());
    if !readable {
        return Err(format!(
            "`{written}` is not a plain decimal number of US dollars. A cost this reader cannot \
             convert is refused rather than read as a run that stated none: charging it at the \
             assumed rate would hide a real cost behind an estimate"
        ));
    }

    let padded = format!("{fraction:0<6}");
    let (head, tail) = padded.split_at(6);
    let dollars: u64 = whole
        .parse()
        .map_err(|_| format!("`{written}` states more dollars than this reader can count"))?;
    let millionths: u64 = head
        .parse()
        .map_err(|_| format!("`{written}` states a fraction this reader cannot read"))?;
    // Half-up on everything past the sixth place: a remainder beginning `5` rounds the millionth up
    // and anything below it leaves it alone. `4999999999` is below it, which is why the live run's
    // cost is `797785` and not `797786`.
    let round_up = tail.starts_with(['5', '6', '7', '8', '9']);
    dollars
        .checked_mul(MICRO_USD)
        .and_then(|total| total.checked_add(millionths))
        .and_then(|total| total.checked_add(u64::from(round_up)))
        .ok_or_else(|| format!("`{written}` is too large to count in millionths of a dollar"))
}

/// The run's tokens, totalled over the four counts a `usage` object carries.
///
/// A key the wire wrote as `null` contributes nothing and does not make the total absent: that is
/// the same reading `protocol trace check` gives it — a count nobody stated is not a zero, and a
/// total over the counts that were stated is what the matrix reports.
fn tokens_of(ended: &serde_json::Value) -> Option<u64> {
    let usage = ended.get("usage")?;
    let mut total = 0_u64;
    let mut stated = false;
    for key in TOKEN_KEYS {
        if let Some(count) = usage.get(key).and_then(serde_json::Value::as_u64) {
            total = total.saturating_add(count);
            stated = true;
        }
    }
    stated.then_some(total)
}

/// A decimal number of US dollars as millionths of one, by integer arithmetic.
///
/// Never `value * 1_000_000.0`: the wire carries a JSON number, and multiplying its `f64` by a
/// million is how `0.0714` becomes `71399`. What is parsed here is the number's own **decimal
/// text** — which is what `serde_json` prints for it — digit by digit, so the conversion is exact
/// or it is refused.
///
/// # Errors
///
/// When the text is not a plain decimal with at most six fraction digits. Scientific notation is
/// refused rather than approximated, because a cost this reader cannot convert exactly is a cost it
/// should not write into a document somebody commits.
fn micro_usd(written: &str) -> Result<u64> {
    let text = written.trim().trim_start_matches('$');
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (text, ""),
    };
    let readable = !whole.is_empty()
        && whole.chars().all(|character| character.is_ascii_digit())
        && fraction.chars().all(|character| character.is_ascii_digit())
        && fraction.len() <= 6;
    if !readable {
        bail!(
            "`{written}` is not an amount in US dollars, such as `10` or `0.25`. Six decimal \
             places at most, and no exponent: a cost that cannot be converted exactly is one this \
             runner will not write into a document"
        );
    }
    let whole: u64 = whole.parse().context("the dollars part is too large")?;
    let padded = format!("{fraction:0<6}");
    let millionths: u64 = if padded.is_empty() {
        0
    } else {
        padded.parse().context("the cents part is too large")?
    };
    whole
        .checked_mul(MICRO_USD)
        .and_then(|dollars| dollars.checked_add(millionths))
        .context("the amount is too large to count in millionths of a dollar")
}

/// Millionths of a dollar as `$0.250000`, which is how every amount is printed here.
fn dollars(micro: u64) -> String {
    format!("${}.{:06}", micro / MICRO_USD, micro % MICRO_USD)
}

// --- assembling one run's three documents ---------------------------------------------------------

/// The three documents one run leaves behind.
struct Products {
    /// The `eval.run-manifest/1` document.
    manifest: String,
    /// The `trace-report/1` record `protocol trace check --format json` writes.
    report: String,
    /// What the run cost, for the budget, where its stream stated one.
    cost_micro_usd: Option<u64>,
    /// What the check said, for the line the runner prints.
    verdict: String,
    /// The model the stream stated, for the same line — `None` where it stated none.
    model: Option<String>,
}

/// One run of one case in one arm on one harness.
struct Plan {
    /// The case.
    case: Case,
    /// The arm.
    arm: Arm,
    /// The harness.
    harness: Harness,
}

impl Plan {
    /// How this run's three documents are named, which is also how the matrix pairs them.
    fn name(&self) -> String {
        format!("{}-{}-{}", self.harness, self.arm, self.case.id)
    }
}

/// Turns a recorded stream into the pair `protocol eval matrix` reads.
///
/// **The whole ingest half of the runner, and nothing in it spawns anything.** That is what makes
/// the pipeline testable end to end for nothing: `--stream` reaches this function with bytes
/// somebody already recorded, and `task check` exercises manifest assembly, the check and the
/// matrix layout over committed fixtures without a vendor binary anywhere.
///
/// The check runs **in this process**, through the same `trace_spec::check::check` the `trace check`
/// verb calls, rather than by shelling out to it: a report produced by a second path could differ
/// from the one a reader gets, and the record beside a manifest has to be the checker's own output.
fn ingest(plan: &Plan, events: &[u8], observed_at: &str, redact: bool) -> Result<Products> {
    let ir = trace_spec::reader::read_any(events).map_err(|errors| {
        refused_run(
            &plan.case.expectations,
            &[StreamRefusal::Unreadable {
                reason: errors.to_string(),
            }],
        )
    })?;
    let session = Session::read(events, plan.arm, plan.harness)
        .map_err(|refusals| refused_run(Path::new(&plan.name()), &refusals))?;

    let spec = crate::trace::load_spec(&plan.case.expectations)?;
    let mut report = trace_spec::check::check(&spec, &ir, &[]);
    if redact {
        report = report.redact();
    }

    let manifest = manifest_text(plan, &session, &report.transcript_digest, observed_at);
    // The runner never writes a manifest its own reader refuses. Cheap, and it closes the gap
    // between *the assembler believes this is a manifest* and *the matrix will read it* — which is
    // where a quoting or key-order mistake would otherwise sit until a sweep had already been paid
    // for.
    let raw: RawRunManifest = serde_yaml::from_str(&manifest).map_err(|error| {
        refused_run(
            Path::new(&plan.name()),
            &[StreamRefusal::ManifestUnreadable {
                reason: error.to_string(),
            }],
        )
    })?;
    RunManifest::try_from(raw).map_err(|refusals| {
        refused_run(
            Path::new(&plan.name()),
            &[StreamRefusal::ManifestUnreadable {
                reason: refusals
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            }],
        )
    })?;

    let mut report_json =
        serde_json::to_string_pretty(&report).context("rendering the record as JSON")?;
    report_json.push('\n');

    Ok(Products {
        manifest,
        report: report_json,
        cost_micro_usd: session.cost_micro_usd,
        verdict: trace_spec::render::verdict_sentence(&report),
        model: session.model.clone(),
    })
}

/// The manifest, as the bytes that go on disk.
///
/// Written out rather than serialised from a struct, for two reasons that are the same reason: the
/// key order is the one the committed fixtures already use, so two waves' manifests diff against
/// each other; and `plugin_digest` has to appear as an explicit `null` on arm `raw`, which is the
/// one field whose *absence* the matrix refuses.
fn manifest_text(
    plan: &Plan,
    session: &Session,
    transcript_digest: &str,
    observed_at: &str,
) -> String {
    let mut lines = vec![
        format!("format: {MANIFEST_FORMAT}"),
        format!("arm: {}", plan.arm),
        format!("harness: {}", plan.harness),
        format!("workflow: {}", plan.case.workflow),
        format!("case: case:{}", plan.case.id),
        match &session.plugin_digest {
            Some(digest) => format!("plugin_digest: {digest}"),
            None => "plugin_digest: null".to_owned(),
        },
        // Written always, `null` where the harness did not say — the same rule as `plugin_digest`
        // one line above, and for the same reason: an omitted key is a runner that forgot.
        match &session.model {
            Some(model) => format!("model: {model}"),
            None => "model: null".to_owned(),
        },
        format!("harness_version: {}", session.harness_version),
        format!("transcript_digest: {transcript_digest}"),
        format!("observed_at: {observed_at}"),
    ];
    // Absent and never zero. A run whose stream stated no cost did not cost nothing, and the matrix
    // reports every resource total over the runs that stated one for exactly this case.
    if let Some(cost) = session.cost_micro_usd {
        lines.push(format!("cost_micro_usd: {cost}"));
    }
    if let Some(tokens) = session.tokens {
        lines.push(format!("tokens: {tokens}"));
    }
    if let Some(wall_time) = session.wall_time_ms {
        lines.push(format!("wall_time_ms: {wall_time}"));
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

// --- spawning ---------------------------------------------------------------------------------------

/// The binary, where it is installed.
///
/// A lookup and never a spawn, on `crate::drive::on_path`'s reasoning: running the tool to find out
/// whether it exists is a side effect in a pre-flight.
fn tool() -> Option<String> {
    if let Some(named) = std::env::var_os(METAHARNESS_BIN_ENV) {
        let path = PathBuf::from(&named);
        return path.is_file().then(|| path.display().to_string());
    }
    crate::drive::on_path(METAHARNESS_BINARY).then(|| METAHARNESS_BINARY.to_owned())
}

/// Where the binary was looked for, for the refusal that says it is not there.
fn looked_for() -> String {
    match std::env::var_os(METAHARNESS_BIN_ENV) {
        Some(named) => format!(
            "`{METAHARNESS_BIN_ENV}` names {}, which is not a file",
            PathBuf::from(named).display()
        ),
        None => format!("nothing on PATH is named `{METAHARNESS_BINARY}`"),
    }
}

/// Whether the environment permits spending money.
fn live() -> bool {
    std::env::var(METAHARNESS_LIVE_ENV).is_ok_and(|value| value == "1")
}

/// The prompt one arm gives one case.
///
/// **Arm `raw` gets the instructions and arm `plugin` does not**, and that is the experiment rather
/// than an omission. Arm a is *text and hope*: the workflow's committed instruction document,
/// rendered by `protocol workflow instruct`, in front of the task. Arm b's treatment **is** the
/// plugin — the skills and agents it installs are what are supposed to carry the workflow — so
/// giving it the instructions too would measure a and b at once and attribute the result to b.
fn prompt_for(plan: &Plan, instructions: &Path) -> Result<String> {
    if plan.arm != Arm::Raw {
        return Ok(plan.case.task.clone());
    }
    let document = instructions.join(format!("{}.md", plan.case.workflow));
    let rendered = std::fs::read_to_string(&document).map_err(|_| {
        refused_run(
            &plan.case.expectations,
            &[RunRefusal::InstructionsMissing {
                workflow: plan.case.workflow.clone(),
                expected: document.display().to_string(),
            }],
        )
    })?;
    Ok(format!("{rendered}\n---\n\n{}", plan.case.task))
}

/// The `metaharness run` invocation for one arm of one case.
///
/// `--decisions observe` is what makes arms a and b comparable with each other and with arm c:
/// every run is spawned by the same instrument into the same hermetic scratch home with the same
/// recording, and only the treatment varies. The mode allows everything and records everything —
/// nothing here decides a tool call, which is arm c's whole difference and `protocol drive`'s job.
fn spawn_argv(plan: &Plan, binary: &str, working_directory: &Path, prompt: &str) -> Vec<String> {
    let mut argv = vec![
        binary.to_owned(),
        "run".to_owned(),
        plan.harness.as_str().to_owned(),
        "--hermetic".to_owned(),
        "--cwd".to_owned(),
        working_directory.display().to_string(),
        "--decisions".to_owned(),
        "observe".to_owned(),
        "-p".to_owned(),
        prompt.to_owned(),
    ];
    if plan.arm == Arm::Plugin {
        argv.push("--plugin-dir".to_owned());
        argv.push(plan.harness.plugin_dir().to_owned());
    }
    argv
}

/// Spawns one run and answers with the stream it wrote.
///
/// The stream is captured whatever the tool exits with, and written down before anything is read
/// out of it: a run that was paid for and then discarded because its last event was missing is the
/// worst outcome this verb has.
fn spawn(plan: &Plan, argv: &[String], stream_path: &Path) -> Result<Vec<u8>> {
    let spawned = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(std::process::Stdio::null())
        .output();
    let output = match spawned {
        Ok(output) => output,
        Err(error) => bail!("`{}` could not be run: {error}", argv.join(" ")),
    };

    std::fs::write(stream_path, &output.stdout)
        .with_context(|| format!("writing {}", stream_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr.lines().rev().take(3).collect::<Vec<_>>().join(" | ");
        return Err(refused_run(
            Path::new(&plan.name()),
            &[RunRefusal::SpawnFailed {
                status: output
                    .status
                    .code()
                    .map_or_else(|| "on a signal".to_owned(), |code| code.to_string()),
                tail,
                stream: stream_path.display().to_string(),
            }],
        ));
    }
    Ok(output.stdout)
}

// --- the verb -------------------------------------------------------------------------------------

/// The arguments of `protocol eval run`.
#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    /// A case directory — one holding a `case.yaml`. Repeatable.
    #[arg(long = "case", value_name = "DIR")]
    cases: Vec<PathBuf>,
    /// Every case of one workflow, taken from the corpus.
    #[arg(long, value_name = "WORKFLOW_ID")]
    workflow: Option<String>,
    /// Where the corpus is, for `--workflow`.
    #[arg(long, value_name = "DIR", default_value = "conformance/eval")]
    corpus: PathBuf,
    /// Which arm this run belongs to.
    ///
    /// `driven` is refused for a spawn and accepted for `--stream`: a driven run is launched by
    /// `protocol drive run`, and this verb reads the stream it wrote.
    #[arg(long, value_enum)]
    arm: Arm,
    /// Which harness runs it.
    #[arg(long, value_enum)]
    harness: Harness,
    /// Where the three documents per run are written.
    #[arg(long, value_name = "DIR")]
    out: PathBuf,
    /// Ingest a stream that already exists instead of spawning one.
    ///
    /// The whole runner minus the spawn, and it spends nothing: no binary is looked for, no
    /// environment variable is consulted and no cap is needed. It is how a driven run enters the
    /// matrix, how a paid run is re-ingested after its manifest rules changed, and how `task check`
    /// exercises this pipeline end to end for free.
    #[arg(long, value_name = "FILE")]
    stream: Option<PathBuf>,
    /// When the run was observed, as a date or epoch milliseconds.
    ///
    /// Required, and deliberately not defaulted to now as `protocol trace evidence` does. The
    /// difference is what the document is for: an evidence record is minted by the process that
    /// performed the observation, and a manifest is a committed document that must assemble to the
    /// same bytes twice. A clock in it would make every re-ingest a diff.
    #[arg(long, value_name = "DATE")]
    observed_at: String,
    /// The tree the session works in. Required for a spawn.
    #[arg(long, value_name = "DIR")]
    cwd: Option<PathBuf>,
    /// The cap on what this invocation may spend, in US dollars. Required for a spawn.
    #[arg(long, value_name = "USD")]
    budget_usd: Option<String>,
    /// What a run whose stream states no cost is counted at, in US dollars.
    #[arg(long, value_name = "USD", default_value = ASSUMED_USD_PER_RUN)]
    assume_usd_per_run: String,
    /// The rendered instruction documents arm `raw` is given.
    #[arg(long, value_name = "DIR", default_value = "generated/instructions")]
    instructions: PathBuf,
    /// Cite event indices and digests only in the record beside each run.
    ///
    /// Opt-in, exactly as `protocol trace check --redact` is and for the same reason — a report is
    /// most useful with its evidence visible. Every record committed to this repository is written
    /// with it, because a report that quotes a transcript is not a thing to publish.
    #[arg(long)]
    redact: bool,
}

/// `protocol eval run --stream FILE`: the runner minus the spawn.
///
/// Split out of [`run_arm`] because it shares none of that function's machinery — no binary, no
/// live flag, no cap, no working tree — and because a reader looking for *what happens for free*
/// should find it in one piece.
fn ingest_recorded(args: &RunArgs, cases: Vec<Case>, stream: &Path) -> Result<ExitCode> {
    if cases.len() != 1 {
        return Err(refused_run(
            stream,
            &[RunRefusal::StreamIsOneRun { named: cases.len() }],
        ));
    }
    let plan = Plan {
        case: cases.into_iter().next().expect("exactly one case"),
        arm: args.arm,
        harness: args.harness,
    };
    let events = std::fs::read(stream)
        .with_context(|| format!("reading the stream at {}", stream.display()))?;
    let products = ingest(&plan, &events, &args.observed_at, args.redact)?;
    write_products(&args.out, &plan, None, &products)?;
    Ok(ExitCode::SUCCESS)
}

/// `protocol eval run`
fn run_arm(args: &RunArgs) -> Result<ExitCode> {
    // Validated and then written through as the caller spelled it: the manifest carries the date,
    // and a date this binary cannot read is one the next reader cannot either.
    crate::observation_time(Some(&args.observed_at))?;

    let cases = select_cases(args)?;
    std::fs::create_dir_all(&args.out)
        .with_context(|| format!("creating {}", args.out.display()))?;

    if let Some(stream) = &args.stream {
        return ingest_recorded(args, cases, stream);
    }

    // --- everything from here spends money -------------------------------------------------------

    let Some(binary) = tool() else {
        // Not an `Err`: the top-level handler renders those as `1`, and *the tool is missing* is
        // the one outcome a caller has to be able to tell from *what you passed is wrong* without
        // parsing prose. Design constant 4 — absent binary is a skip, never a red gate.
        eprintln!(
            "{}",
            RunRefusal::ToolMissing {
                looked_for: looked_for()
            }
        );
        return Ok(ExitCode::from(TOOL_MISSING_EXIT));
    };
    if !live() {
        return Err(refused_run(&args.out, &[RunRefusal::NotLive]));
    }
    if args.arm == Arm::Driven {
        return Err(refused_run(
            &args.out,
            &[RunRefusal::DrivenIsNotLaunchedHere],
        ));
    }
    if args.arm == Arm::Native {
        return Err(refused_run(
            &args.out,
            &[RunRefusal::NativeIsNotLaunchedHere],
        ));
    }
    let Some(budget) = &args.budget_usd else {
        return Err(refused_run(&args.out, &[RunRefusal::NoBudget]));
    };
    let cap = micro_usd(budget)?;
    let assumed = micro_usd(&args.assume_usd_per_run)?;
    let Some(working_directory) = &args.cwd else {
        return Err(refused_run(&args.out, &[RunRefusal::NoWorkingTree]));
    };

    let total = cases.len();
    let mut spent = 0_u64;
    let mut launched = 0_usize;

    for (position, case) in cases.into_iter().enumerate() {
        // Checked **before** the spawn and against the assumed rate, because the only number
        // available before a run is the assumed one: a cap enforced after the fact is a receipt.
        if spent.saturating_add(assumed) > cap {
            outln!(
                "{}",
                RunRefusal::BudgetWouldBeExceeded {
                    spent,
                    next: assumed,
                    cap,
                    launched,
                    skipped: total - position,
                }
            );
            break;
        }

        let plan = Plan {
            case,
            arm: args.arm,
            harness: args.harness,
        };
        let prompt = prompt_for(&plan, &args.instructions)?;
        let invocation = spawn_argv(&plan, &binary, working_directory, &prompt);
        let stream_path = args.out.join(format!("{}{EVENTS_SUFFIX}", plan.name()));

        let events = spawn(&plan, &invocation, &stream_path)?;
        let products = ingest(&plan, &events, &args.observed_at, args.redact)?;
        // The stream's own stated cost, and the assumption **only** where it stated none — a cost
        // this reader could not convert never arrives here as `None`, because `ingest` refuses it
        // (`EVAL-STREAM-011`). A wire that writes `null` must not be able to spend without limit;
        // a wire that priced the run must not be charged an estimate instead.
        let (charge, source) = products
            .cost_micro_usd
            .map_or((assumed, "assumed"), |stated| (stated, "stated"));
        spent = spent.saturating_add(charge);
        write_products(&args.out, &plan, Some(&stream_path), &products)?;
        // Printed per run rather than only as a total, because the failure this line exists to make
        // visible is silent by nature: a stated cost dropped to an assumption looks exactly like a
        // cheap run, and one live Claude run at $0.797785 was charged $0.250000 before anybody
        // could see which of the two numbers the ledger was using.
        outln!("  charged:  {} ({source})", dollars(charge));
        launched += 1;
    }

    if launched == 0 {
        return Err(refused_run(
            &args.out,
            &[RunRefusal::BudgetWouldBeExceeded {
                spent,
                next: assumed,
                cap,
                launched,
                skipped: total,
            }],
        ));
    }

    outln!(
        "{launched} run(s), {} spent against a cap of {}",
        dollars(spent),
        dollars(cap)
    );
    Ok(ExitCode::SUCCESS)
}

/// How a model nobody stated is spelled for a person.
///
/// Obviously not a model name, and deliberately not blank: a column that renders an unstated model
/// as nothing reads exactly like a column nobody looked at. The matrix's own **text** rendering has
/// no model column at all — it groups by harness × arm × workflow — so this is where a person meets
/// one; its JSON writes `"model": null`, which is the same fact in the shape a program reads.
const MODEL_UNSTATED: &str = "(unstated)";

/// Writes one run's documents and says what was left where.
fn write_products(
    out: &Path,
    plan: &Plan,
    stream: Option<&Path>,
    products: &Products,
) -> Result<()> {
    let name = plan.name();
    let manifest = out.join(format!("{name}{MANIFEST_SUFFIX}"));
    let record = out.join(format!("{name}{RECORD_SUFFIX}"));
    std::fs::write(&manifest, &products.manifest)
        .with_context(|| format!("writing {}", manifest.display()))?;
    std::fs::write(&record, &products.report)
        .with_context(|| format!("writing {}", record.display()))?;
    outln!(
        "{name} — {}{}",
        products.verdict,
        stream.map_or_else(String::new, |path| format!(
            "\n  stream:   {}",
            path.display()
        ))
    );
    outln!(
        "  model:    {}",
        products.model.as_deref().unwrap_or(MODEL_UNSTATED)
    );
    outln!("  manifest: {}", manifest.display());
    outln!("  record:   {}", record.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest that passes every rule, as a starting point for one-line mutations.
    const HONEST: &str = "\
format: eval.run-manifest/1
arm: plugin
harness: claude
workflow: adp/default
case: case:create-a-story
plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332
model: claude-sonnet-5
harness_version: claude 2.1.239
transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b
observed_at: 2026-08-23
";

    /// Reads a manifest, returning the refusals rather than a message.
    fn read(text: &str) -> Result<RunManifest, Vec<Refusal>> {
        let raw: RawRunManifest = serde_yaml::from_str(text).expect("the fixture is YAML");
        RunManifest::try_from(raw)
    }

    /// The codes a refusal set carries, which is what a test matches on.
    fn codes(refusals: &[Refusal]) -> Vec<&'static str> {
        refusals.iter().map(Refusal::code).collect()
    }

    #[test]
    fn the_honest_manifest_is_read_so_every_mutation_below_reaches_its_rule() {
        // The control. Without it a mutation test could be passing because the fixture was broken
        // for some other reason entirely.
        let manifest = read(HONEST).expect("the fixture states every field");
        assert_eq!(manifest.arm, Arm::Plugin);
        assert_eq!(manifest.harness, "claude");
        assert!(manifest.plugin_digest.is_some());
        assert_eq!(manifest.cost_micro_usd, None, "a quantity nobody stated");
    }

    #[test]
    fn an_arm_this_evaluation_does_not_have_is_refused_by_name() {
        let refusals = read(&HONEST.replace("arm: plugin", "arm: hybrid"))
            .expect_err("a fourth arm is a change to the programme");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-002"]);
        let sentence = refusals[0].to_string();
        assert!(
            sentence.contains("`raw`, `plugin`, `driven`"),
            "the refusal lists the three arms there are: {sentence}"
        );
    }

    #[test]
    fn a_missing_field_is_refused_by_its_own_name_and_every_other_refusal_is_reported_beside_it() {
        // Invariant 3: a document with four broken fields reports four refusals. A reader who has
        // to run the verb four times to find four typos stops running it.
        //
        // `model` stays in this set deliberately after it became a `Written`: an *absent* model is
        // still refused, and keeping it here proves `written_or_null` accumulates beside `required`
        // rather than short-circuiting the pass.
        let stripped = HONEST
            .lines()
            .filter(|line| {
                !line.starts_with("model:")
                    && !line.starts_with("harness_version:")
                    && !line.starts_with("case:")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let refusals = read(&stripped).expect_err("three fields are missing");
        assert_eq!(
            codes(&refusals),
            [
                "EVAL-MANIFEST-003",
                "EVAL-MANIFEST-003",
                "EVAL-MANIFEST-003"
            ]
        );
        let named: Vec<String> = refusals.iter().map(ToString::to_string).collect();
        for field in ["case", "model", "harness_version"] {
            assert!(
                named
                    .iter()
                    .any(|line| line.contains(&format!("`{field}`"))),
                "each refusal names its own field: {named:?}"
            );
        }
    }

    #[test]
    fn an_omitted_plugin_digest_is_refused_and_an_explicit_null_is_not() {
        // The rule the shape exists for. Serde maps `null` and *absent* onto the same `None`, so
        // without `written_down` a manifest that forgot the key would read as a run that had no
        // plugin — and arm `raw`'s whole claim is that it had none.
        let omitted = HONEST
            .lines()
            .filter(|line| !line.starts_with("plugin_digest:"))
            .collect::<Vec<_>>()
            .join("\n");
        let refusals = read(&omitted).expect_err("the key must be written");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-003"]);

        let raw_arm = HONEST.replace("arm: plugin", "arm: raw").replace(
            "plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332",
            "plugin_digest: null",
        );
        let manifest = read(&raw_arm).expect("an explicit null on arm raw is the honest form");
        assert_eq!(manifest.arm, Arm::Raw);
        assert_eq!(manifest.plugin_digest, None);
    }

    #[test]
    fn an_omitted_model_is_refused_and_an_explicit_null_is_not() {
        // The rule `plugin_digest` already had, extended to `model` by a live run. Codex's wire
        // names no model at session start, so *the harness did not say* is a fact a manifest must
        // be able to state — and *nobody wrote the key* must still be refused, because a runner
        // that dropped it would produce the same document.
        let omitted = HONEST
            .lines()
            .filter(|line| !line.starts_with("model:"))
            .collect::<Vec<_>>()
            .join("\n");
        let refusals = read(&omitted).expect_err("the key must be written");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-003"]);

        let unstated = read(&HONEST.replace("model: claude-sonnet-5", "model: null"))
            .expect("a wire that states no model is stating something");
        assert_eq!(unstated.model, None);
    }

    #[test]
    fn a_model_that_is_written_and_empty_is_refused_rather_than_read_as_unstated() {
        // The boundary between the two answers above. Without this an empty string would slip
        // through as `Some("")` and print as a blank model column, which reads like a rendering
        // bug rather than a fact.
        let refusals = read(&HONEST.replace("model: claude-sonnet-5", "model: \"\""))
            .expect_err("an empty model names nothing");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-004"]);
    }

    #[test]
    fn a_plugin_digest_on_arm_raw_is_refused_because_arm_raw_is_the_arm_without_one() {
        let refusals = read(&HONEST.replace("arm: plugin", "arm: raw"))
            .expect_err("arm raw carries no plugin");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-005"]);
    }

    #[test]
    fn a_null_plugin_digest_on_arm_plugin_is_refused_because_the_plugin_is_the_subject() {
        let refusals = read(&HONEST.replace(
            "plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332",
            "plugin_digest: null",
        ))
        .expect_err("arm plugin must say which plugin");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-006"]);
    }

    #[test]
    fn arm_driven_may_answer_either_way_because_the_enforcer_is_not_the_plugin() {
        // The boundary of the two rules above. Without this test they could have been written as
        // one rule — *a digest exactly when the arm is not raw* — and every other test would still
        // pass.
        let with = read(&HONEST.replace("arm: plugin", "arm: driven"))
            .expect("a driven run may also have had the plugin installed");
        assert!(with.plugin_digest.is_some());

        let without = read(&HONEST.replace("arm: plugin", "arm: driven").replace(
            "plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332",
            "plugin_digest: null",
        ))
        .expect("and it may not have");
        assert_eq!(without.plugin_digest, None);
    }

    #[test]
    fn a_document_that_does_not_claim_the_format_is_refused_before_its_fields_are_believed() {
        let refusals = read(&HONEST.replace("eval.run-manifest/1", "eval.run-manifest/2"))
            .expect_err("a version this build does not know");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-001"]);
        assert!(
            refusals[0].to_string().contains("eval.run-manifest/2"),
            "the refusal quotes what it was handed: {}",
            refusals[0]
        );
    }

    #[test]
    fn a_digest_that_is_not_a_digest_is_refused() {
        let refusals = read(&HONEST.replace(
            "transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b",
            "transcript_digest: sha256:6522e1",
        ))
        .expect_err("a digest is 64 hex characters");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-007"]);
    }

    /// A record with the verdicts given, in the shape `protocol trace check --format json` writes.
    fn record_of(verdicts: &[(&str, &str)]) -> String {
        let rows: Vec<String> = verdicts
            .iter()
            .map(|(id, verdict)| {
                format!(r#"{{"id":"{id}","kind":"tool.called","verdict":{verdict}}}"#)
            })
            .collect();
        format!(
            r#"{{"format":"trace-report/1","spec_id":"eval/development-story",
                 "spec_digest":"fd6bcd8ab28806f92f487276ffa60d21f51ebc0576b2027d9d279e3685e38466",
                 "expectations":[{}]}}"#,
            rows.join(",")
        )
    }

    /// Reads a record, returning the refusals rather than a message.
    fn read_record(text: &str) -> Result<Record, Vec<Refusal>> {
        let raw: RawRecord = serde_json::from_str(text).expect("the fixture is JSON");
        Record::try_from(raw)
    }

    #[test]
    fn the_three_verdicts_map_onto_the_three_columns() {
        let record = read_record(&record_of(&[
            ("held", "\"ok\""),
            ("violated", "\"gap\""),
            ("unobservable", "\"unknown\""),
        ]))
        .expect("the shape the checker writes");
        assert_eq!(
            record.rows,
            vec![
                ("held".to_owned(), Outcome::Held),
                ("violated".to_owned(), Outcome::Violated),
                ("unobservable".to_owned(), Outcome::Unobservable),
            ]
        );
    }

    #[test]
    fn a_row_whose_verdict_is_null_is_unobservable_and_never_held() {
        // The polarity of the whole verb, and the mutation that breaks it is one character in
        // `outcome_of`: `None => Ok(Outcome::Held)`. Both spellings of *nothing was recorded* are
        // asserted, because a checker that dropped the key and one that wrote `null` produce
        // documents that must be read the same way.
        for record in [
            record_of(&[("silent", "null")]),
            r#"{"format":"trace-report/1","spec_id":"s","expectations":[{"id":"silent"}]}"#
                .to_owned(),
        ] {
            let read = read_record(&record).expect("a silent row is read, not refused");
            assert_eq!(
                read.rows,
                vec![("silent".to_owned(), Outcome::Unobservable)],
                "a row the checker recorded no verdict for is unobservable, never held"
            );
        }
    }

    #[test]
    fn a_verdict_word_this_build_cannot_read_is_refused_rather_than_bucketed() {
        let refusals = read_record(&record_of(&[("new", "\"probably\"")]))
            .expect_err("an unreadable word is not a third answer");
        assert_eq!(codes(&refusals), ["EVAL-RECORD-003"]);
    }

    #[test]
    fn a_record_of_another_shape_is_refused_by_the_format_it_states() {
        let refusals = read_record(r#"{"format":"trace-ir/1","expectations":[]}"#)
            .expect_err("the matrix reads a check report");
        assert_eq!(codes(&refusals), ["EVAL-RECORD-001"]);
        assert!(
            refusals[0].to_string().contains("trace-ir/1"),
            "named: {}",
            refusals[0]
        );
    }

    /// The honest manifest, as a value, with the arm and transcript given.
    fn manifest_of(arm: Arm, transcript: &str) -> RunManifest {
        RunManifest {
            arm,
            harness: "claude".to_owned(),
            workflow: "adp/default".to_owned(),
            case: "case:create-a-story".to_owned(),
            plugin_digest: None,
            model: Some("claude-sonnet-5".to_owned()),
            harness_version: "claude 2.1.239".to_owned(),
            transcript_digest: transcript.to_owned(),
            observed_at: "2026-08-23".to_owned(),
            cost_micro_usd: Some(1_500_000),
            tokens: Some(10),
            wall_time_ms: Some(20),
        }
    }

    /// A record value with one row.
    fn record_value(transcript: &str, digest: &str, outcome: Outcome) -> Record {
        Record {
            specification: "eval/development-story".to_owned(),
            spec_digest: Some(digest.to_owned()),
            transcript_digest: Some(transcript.to_owned()),
            rows: vec![("only".to_owned(), outcome)],
        }
    }

    #[test]
    fn a_manifest_that_describes_another_run_than_its_record_is_refused() {
        let pairs = vec![(
            manifest_of(Arm::Raw, &"a".repeat(DIGEST_WIDTH)),
            record_value(&"b".repeat(DIGEST_WIDTH), "d", Outcome::Held),
        )];
        let refusals = assemble(pairs).expect_err("the two documents are about different runs");
        assert_eq!(codes(&refusals), ["EVAL-PAIR-003"]);
    }

    #[test]
    fn one_transcript_cannot_arrive_twice_because_one_run_would_be_counted_twice() {
        let digest = "c".repeat(DIGEST_WIDTH);
        let pairs = vec![
            (
                manifest_of(Arm::Raw, &digest),
                record_value(&digest, "d", Outcome::Held),
            ),
            (
                manifest_of(Arm::Plugin, &digest),
                record_value(&digest, "d", Outcome::Held),
            ),
        ];
        let refusals = assemble(pairs).expect_err("two runs are two transcripts");
        assert_eq!(codes(&refusals), ["EVAL-PAIR-004"]);
    }

    #[test]
    fn one_specification_at_two_digests_is_refused_because_the_rows_share_a_name_only() {
        let pairs = vec![
            (
                manifest_of(Arm::Raw, &"e".repeat(DIGEST_WIDTH)),
                record_value(&"e".repeat(DIGEST_WIDTH), "before", Outcome::Held),
            ),
            (
                manifest_of(Arm::Plugin, &"f".repeat(DIGEST_WIDTH)),
                record_value(&"f".repeat(DIGEST_WIDTH), "after", Outcome::Violated),
            ),
        ];
        let refusals = assemble(pairs).expect_err("the document moved between the two runs");
        assert_eq!(codes(&refusals), ["EVAL-PAIR-005"]);
    }

    #[test]
    fn a_cells_resource_total_says_how_many_runs_it_covers() {
        // The reason a column is a pair and not a number: a total over two of three runs read as a
        // total over three would understate the arm it describes.
        let mut quiet = manifest_of(Arm::Driven, &"1".repeat(DIGEST_WIDTH));
        quiet.cost_micro_usd = None;
        quiet.tokens = None;
        quiet.wall_time_ms = None;
        let pairs = vec![
            (
                manifest_of(Arm::Driven, &"0".repeat(DIGEST_WIDTH)),
                record_value(&"0".repeat(DIGEST_WIDTH), "d", Outcome::Held),
            ),
            (
                quiet,
                record_value(&"1".repeat(DIGEST_WIDTH), "d", Outcome::Unobservable),
            ),
        ];
        let matrix = assemble(pairs).expect("two distinct runs of one cell");
        assert_eq!(matrix.cells.len(), 1);
        let cell = &matrix.cells[0];
        assert_eq!(cell.runs, 2);
        assert_eq!(
            cell.resources.cost_micro_usd,
            Some(Reported {
                runs: 1,
                total: 1_500_000
            })
        );
        assert_eq!(cell.counts.held, 1);
        assert_eq!(cell.counts.unobservable, 1);
        assert_eq!(
            cost(cell.resources.cost_micro_usd, cell.runs),
            "$1.500000 (1/2)",
            "the rendering says what the total covers"
        );
    }

    #[test]
    fn the_arms_sort_in_the_order_the_experiment_runs_them() {
        // Alphabetically this is `driven`, `plugin`, `raw`, which reads the experiment backwards.
        let mut arms = vec![Arm::Driven, Arm::Raw, Arm::Plugin];
        arms.sort_unstable();
        assert_eq!(arms, vec![Arm::Raw, Arm::Plugin, Arm::Driven]);
    }

    #[test]
    fn no_rendering_of_a_matrix_contains_a_score() {
        // The programme's one prohibition, asserted on the bytes rather than trusted to review.
        let pairs = vec![(
            manifest_of(Arm::Raw, &"2".repeat(DIGEST_WIDTH)),
            record_value(&"2".repeat(DIGEST_WIDTH), "d", Outcome::Held),
        )];
        let matrix = assemble(pairs).expect("one run");
        let text = to_text(&matrix);
        let json = serde_json::to_string(&matrix).expect("the matrix serialises");
        for rendering in [&text, &json] {
            assert!(
                !rendering.contains('%'),
                "no percentage reaches an output: {rendering}"
            );
            assert_eq!(
                rendering.matches("score").count(),
                rendering.matches("no score is computed").count(),
                "and the only occurrence of the word is the sentence saying there is none: \
                 {rendering}"
            );
        }
        assert!(
            text.contains("No arm is ranked and no score is computed"),
            "and the text rendering says so where a reader will look for one: {text}"
        );
    }

    // --- the runner ---------------------------------------------------------------------------

    #[test]
    fn an_amount_becomes_millionths_by_integer_arithmetic_and_never_by_a_float() {
        // The one-line mutation this guards is `(value * 1_000_000.0) as u64`, which turns the
        // cost this repository's own fixtures carry — `0.0714` — into `71399`. A cent lost per run
        // is a budget that overspends, and a manifest that will not reproduce.
        assert_eq!(micro_usd("0.0714").expect("a cost off the wire"), 71_400);
        assert_eq!(micro_usd("0.4137").expect("another"), 413_700);
        assert_eq!(
            micro_usd("10").expect("a whole number of dollars"),
            10_000_000
        );
        assert_eq!(micro_usd("0.25").expect("the assumed rate"), 250_000);
        assert_eq!(
            micro_usd("$1.00").expect("a dollar sign is tolerated"),
            1_000_000
        );
        assert_eq!(micro_usd("0").expect("nothing at all"), 0);
    }

    #[test]
    fn an_amount_this_reader_cannot_convert_exactly_is_refused_rather_than_rounded() {
        // Scientific notation and a seventh decimal place are both *nearly* readable, which is
        // what makes silently approximating them tempting. A cost that cannot be converted exactly
        // does not belong in a document somebody commits.
        for written in ["1e-7", "0.1234567", "", "ten", "1.2.3", "-3"] {
            assert!(
                micro_usd(written).is_err(),
                "`{written}` is not an amount this reader will convert"
            );
        }
    }

    #[test]
    fn the_terminal_events_cost_is_read_from_its_own_decimal_text() {
        let ended = serde_json::json!({ "total_cost_usd": 0.0714 });
        assert_eq!(cost_of(&ended), Ok(Some(71_400)));
        // Written `null` and absent are the same answer, and neither is zero: the manifest states
        // no cost at all, and the matrix's total then says how many runs it covers.
        assert_eq!(
            cost_of(&serde_json::json!({ "total_cost_usd": null })),
            Ok(None)
        );
        assert_eq!(cost_of(&serde_json::json!({})), Ok(None));
    }

    #[test]
    fn a_cost_a_harness_computed_in_floating_point_is_read_and_not_refused() {
        // **The live defect.** A Claude run stated `0.7977854999999999` — the shortest text that
        // round-trips the `f64` sum of its per-turn costs — and the strict reader refused it for
        // having seventeen significant figures. Eighty cents then entered the ledger as the
        // assumed twenty-five and the manifest as no cost at all.
        assert_eq!(micro_usd_stated("0.7977854999999999"), Ok(797_785));
        assert_eq!(
            cost_of(&serde_json::json!({ "total_cost_usd": 0.797_785_499_999_999_9 })),
            Ok(Some(797_785))
        );
        // Half-up on everything past the sixth place, so both sides of the boundary are pinned
        // rather than whichever one the first fixture happened to have.
        assert_eq!(micro_usd_stated("0.1234564999"), Ok(123_456));
        assert_eq!(micro_usd_stated("0.1234565"), Ok(123_457));
        // And the carry is ordinary integer addition, so it crosses into the dollar.
        assert_eq!(micro_usd_stated("0.9999995"), Ok(1_000_000));
        // Six places or fewer still read exactly, which is every committed fixture.
        assert_eq!(micro_usd_stated("0.5216"), Ok(521_600));
        assert_eq!(micro_usd_stated("0"), Ok(0));
    }

    #[test]
    fn a_stated_cost_this_reader_cannot_convert_is_refused_rather_than_read_as_no_cost() {
        // The half of the defect that made it silent: `.ok()` collapsed *there is a number here I
        // cannot convert* into *there is no number*, and the second is the one the ledger is
        // allowed to charge an estimate for. Unreadable is not unstated.
        assert!(micro_usd_stated("1e-7").is_err(), "an exponent is refused");
        let reason = cost_of(&serde_json::json!({ "total_cost_usd": "0.80" }))
            .expect_err("a cost written as a string is not a number");
        assert!(
            reason.contains("neither a number nor `null`"),
            "and the refusal says which two answers there are: {reason}"
        );
    }

    #[test]
    fn a_person_typing_an_amount_is_still_held_to_an_exact_one() {
        // The reason there are two readers rather than one loosened one. A wire computes and may
        // hand over float noise; a person types, and `--budget-usd 1e-7` is a mistake worth naming
        // rather than a cap silently rounded to nothing.
        assert!(micro_usd("1e-7").is_err());
        assert!(micro_usd("0.1234567").is_err());
        assert_eq!(
            micro_usd_stated("0.1234567"),
            Ok(123_457),
            "while the same text off a wire is rounded to the nearest millionth"
        );
    }

    #[test]
    fn a_usage_key_written_null_contributes_nothing_and_does_not_erase_the_total() {
        let usage = serde_json::json!({
            "usage": { "input_tokens": 14, "output_tokens": 1128,
                       "cache_read_input_tokens": null, "cache_creation_input_tokens": 20168 }
        });
        assert_eq!(tokens_of(&usage), Some(14 + 1128 + 20168));
        assert_eq!(
            tokens_of(&serde_json::json!({ "usage": {} })),
            None,
            "and a usage object that states none of the four states no total"
        );
    }

    /// A plan over a case, for the argv tests.
    fn plan_of(arm: Arm, harness: Harness) -> Plan {
        Plan {
            case: Case {
                id: "development-honest".to_owned(),
                workflow: "adp/default".to_owned(),
                task: "Add a `--json` flag.".to_owned(),
                expectations: PathBuf::from(
                    "conformance/eval/development-honest/expectations.trace.yaml",
                ),
            },
            arm,
            harness,
        }
    }

    #[test]
    fn every_arm_is_spawned_by_one_instrument_and_only_the_treatment_varies() {
        // Design constant 1, as an assertion on the argv. The two invocations differ in exactly
        // two words — `--plugin-dir` and the directory — and in nothing else: same hermetic mode,
        // same decision mode, same working tree. An instrument that varied with the arm would make
        // the comparison meaningless whatever the matrix said.
        let tree = PathBuf::from("/work/scratch");
        let raw = spawn_argv(
            &plan_of(Arm::Raw, Harness::Claude),
            "metaharness",
            &tree,
            "do it",
        );
        let plugin = spawn_argv(
            &plan_of(Arm::Plugin, Harness::Claude),
            "metaharness",
            &tree,
            "do it",
        );

        assert_eq!(
            raw,
            vec![
                "metaharness",
                "run",
                "claude",
                "--hermetic",
                "--cwd",
                "/work/scratch",
                "--decisions",
                "observe",
                "-p",
                "do it",
            ]
        );
        assert_eq!(
            plugin[..raw.len()],
            raw[..],
            "arm b is arm a plus its treatment, and nothing else"
        );
        assert_eq!(
            &plugin[raw.len()..],
            &[
                "--plugin-dir".to_owned(),
                "integrations/claude-code".to_owned()
            ]
        );
        assert_eq!(
            spawn_argv(
                &plan_of(Arm::Plugin, Harness::Codex),
                "metaharness",
                &tree,
                "do it"
            )
            .last()
            .expect("a plugin directory"),
            "integrations/codex",
            "and each harness gets its own plugin"
        );
    }

    #[test]
    fn the_manifest_the_runner_assembles_is_one_the_matrixs_own_reader_reads() {
        // The round trip, at the level of the two functions rather than through the binary. The
        // failure it exists to catch is a quoting or key-order mistake that would otherwise sit
        // undetected until a sweep had been paid for.
        let session = Session {
            harness_version: "claude 2.1.239".to_owned(),
            model: Some("claude-sonnet-5".to_owned()),
            plugin_digest: Some("a".repeat(DIGEST_WIDTH)),
            cost_micro_usd: Some(521_600),
            tokens: Some(116_546),
            wall_time_ms: Some(22_320),
        };
        let text = manifest_text(
            &plan_of(Arm::Plugin, Harness::Claude),
            &session,
            &"b".repeat(DIGEST_WIDTH),
            "2026-08-23",
        );
        let manifest = read(&text).expect("the runner writes manifests its own reader reads");
        assert_eq!(manifest.arm, Arm::Plugin);
        assert_eq!(manifest.case, "case:development-honest");
        assert_eq!(manifest.plugin_digest, Some("a".repeat(DIGEST_WIDTH)));
        assert_eq!(manifest.cost_micro_usd, Some(521_600));
    }

    #[test]
    fn a_transcript_of_several_sessions_totals_them_rather_than_reporting_the_last_one() {
        // The driven shape. `protocol drive` starts a fresh session per workflow state, so a driven
        // run's transcript is a concatenation with one terminal record per state — and this reader
        // took the last of them until the first live driven run (2026-08-23) reported `$1.135363`
        // for a walk that had cost `$15.014604` across six sessions.
        //
        // Doubling a committed fixture is the whole assertion: whatever one session states, two
        // copies of it must state twice, on all three columns. A reader that takes the last record
        // answers the single figure and fails here.
        let one = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/eval-run/claude-driven-attested.jsonl"),
        )
        .expect("the committed driven fixture");
        let mut two = one.clone();
        two.extend_from_slice(&one);

        let single = Session::read(&one, Arm::Driven, Harness::Claude).expect("a readable stream");
        let doubled =
            Session::read(&two, Arm::Driven, Harness::Claude).expect("two of them, concatenated");

        assert_eq!(
            doubled.cost_micro_usd,
            single.cost_micro_usd.map(|cost| cost * 2),
            "two sessions cost what both of them cost"
        );
        assert_eq!(
            doubled.tokens,
            single.tokens.map(|tokens| tokens * 2),
            "and used what both of them used"
        );
        assert_eq!(
            doubled.wall_time_ms,
            single.wall_time_ms.map(|wall| wall * 2),
            "and took as long as both of them took: the sessions run one after another"
        );
        assert_eq!(
            (
                doubled.harness_version.clone(),
                doubled.model.clone(),
                doubled.plugin_digest.clone()
            ),
            (
                single.harness_version.clone(),
                single.model.clone(),
                single.plugin_digest.clone()
            ),
            "and nothing else moved: the opening record is still the first one"
        );
    }

    #[test]
    fn a_run_that_states_no_cost_writes_no_cost_key_rather_than_a_zero() {
        let session = Session {
            // The shape the first live pilot run recorded: codex states no model at session start.
            harness_version: "codex 0.144.0".to_owned(),
            model: None,
            plugin_digest: None,
            cost_micro_usd: None,
            tokens: None,
            wall_time_ms: None,
        };
        let text = manifest_text(
            &plan_of(Arm::Raw, Harness::Codex),
            &session,
            &"c".repeat(DIGEST_WIDTH),
            "2026-08-23",
        );
        assert!(
            !text.contains("cost_micro_usd") && !text.contains("tokens"),
            "an unpriced run states nothing, and the matrix reports its cell over the runs that \
             did: {text}"
        );
        assert!(
            text.contains("plugin_digest: null") && text.contains("model: null"),
            "and both keys that must be written even when they say nothing are written: {text}"
        );
        assert!(
            read(&text).is_ok(),
            "and a manifest with two written nulls in it is one the matrix reads: {text}"
        );
    }
}

#[cfg(test)]
mod native_arm_tests {
    use super::*;

    #[test]
    fn the_fourth_arm_is_a_word_the_manifest_reads_and_writes() {
        assert_eq!(Arm::parse("native"), Some(Arm::Native));
        assert_eq!(Arm::Native.as_str(), "native");
        assert_eq!(Harness::B10x.as_str(), "b10x");
    }

    #[test]
    fn the_arms_still_sort_in_the_order_the_experiment_runs_them() {
        // `native` last, because it is the arm that removes the vendor loop entirely and every
        // other arm is a treatment applied to one.
        let mut arms = vec![Arm::Native, Arm::Driven, Arm::Raw, Arm::Plugin];
        arms.sort();
        assert_eq!(arms, vec![Arm::Raw, Arm::Plugin, Arm::Driven, Arm::Native]);
    }

    #[test]
    fn a_native_run_is_refused_a_spawn_and_told_what_does_launch_it() {
        // Same position as `driven` and a different reason, which the message has to carry: a
        // driven run is launched by `protocol drive run` because there must be one policy; a
        // native run is launched by `b10x-harness` because it *is* the loop and there is no vendor
        // harness here to drive.
        let refusal = RunRefusal::NativeIsNotLaunchedHere.to_string();
        assert!(refusal.starts_with("EVAL-RUN-011"), "{refusal}");
        assert!(refusal.contains("b10x-harness"), "{refusal}");
        assert!(refusal.contains("--arm native --stream"), "{refusal}");
        assert!(
            refusal.contains("no vendor harness in it"),
            "and says why it differs from driven: {refusal}"
        );
    }

    #[test]
    fn every_arm_has_a_code_of_its_own_and_none_is_reused() {
        let codes = [
            RunRefusal::NotLive.code(),
            RunRefusal::NoBudget.code(),
            RunRefusal::DrivenIsNotLaunchedHere.code(),
            RunRefusal::NativeIsNotLaunchedHere.code(),
        ];
        let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();
        assert_eq!(unique.len(), codes.len(), "{codes:?}");
    }
}
