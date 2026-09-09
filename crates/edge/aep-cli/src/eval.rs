//! `protocol eval matrix` — many checked runs become one table of facts, and never a score.
//!
//! The sixth module split, on the criterion the first five took: a verb family with its own input,
//! its own vocabulary and no shared state with the rest of the binary.
//!
//! # What this assembles, and what it refuses to assemble
//!
//! The evaluation runs the same cases under each arm — **raw** instructions, the shipped
//! **plugin**, a **driven** run whose tool calls an enforcer decides, and a **native** run whose
//! published toolset is the policy — against more than one harness. Each run leaves two documents:
//! a `eval.run-manifest/1` saying what was run and under which arm, and the `trace-report/1` record
//! `protocol trace check --format json` writes about its transcript. This verb reads those pairs
//! and reports, per harness × arm × workflow and per expectation, **how many facts held, how many
//! were contradicted, and how many nobody could find out**.
//!
//! The arms are not four ways of saying the same thing, and the difference decides what a clean
//! cell means: on `driven` a call was answered at a seam, on `raw` and `native` nothing adjudicated
//! anything. `Arm` carries the model per variant and the rendering repeats it under the table
//! whenever a `native` cell is printed.
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
//! record is a thing people paste into pull requests (`crates/observe/trace-spec/src/evidence.rs`). A
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

include!("eval_tests.rs");
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::money::{dollars, MICRO_USD};

/// The format claim a run manifest carries.
pub const MANIFEST_FORMAT: &str = "eval.run-manifest/1";

/// The format claim the assembled matrix carries.
pub const MATRIX_FORMAT: &str = "eval.matrix/1";

/// The format claim of the record a manifest accompanies.
pub const REPORT_FORMAT: &str = "trace-report/1";

/// How a run manifest is named on disk.
pub const MANIFEST_SUFFIX: &str = ".manifest.yaml";

/// How the record beside it is named.
pub const RECORD_SUFFIX: &str = ".report.json";

/// How many hex characters a content digest has.
pub const DIGEST_WIDTH: usize = 64;

// --- the arms ------------------------------------------------------------------------------------

/// Which arm a run belongs to.
///
/// Closed, because the arms are the design of the evaluation rather than a label somebody picked:
/// raw instructions, the shipped plugin, a driven run whose calls an enforcer decides, and our own
/// loop where the published toolset is the policy. Another arm is a change to the programme, and it
/// should stop here rather than appear as a new row nobody planned.
///
/// The declaration order is the programme's order — a, b, c, d — and `Ord` follows it. Sorting
/// alphabetically would print `driven`, `native`, `plugin`, `raw`, which reads the experiment
/// backwards.
///
/// # The word is also the enforcement model, and that is the only label a cell gets
///
/// Nothing in the matrix says *enforced* beside *complied*, so the arm has to carry it. Each
/// variant below states its model in a line, and the rule for reading a store-integrity row off
/// one is in `docs/design/native-arm-store-integrity-design-v0.1.md` § 6 O1 — the reason it is a
/// reading rule and not a column is that a column is a change to a printed table's format, which
/// is the operator's decision and not this type's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Arm {
    /// The instructions alone: no plugin, no enforcement.
    ///
    /// **Enforcement model: none.** Nothing on this arm was ever in a position to refuse, so every
    /// cell is compliance — what the model chose to do, never what it was stopped from doing.
    Raw,
    /// The shipped plugin is installed, and nothing decides the agent's calls.
    ///
    /// **Enforcement model: policy injected into a vendor's loop.** A refusal here is the vendor
    /// hook's, and it reaches only as far as what the vendor shows the hook.
    Plugin,
    /// The run is driven, and every tool call is decided at a seam.
    ///
    /// **Enforcement model: a per-call seam.** Every admitted call is put to the driver and
    /// answered before it runs, so a store-integrity cell on this arm *can* say **enforced**: the
    /// refusal is in the run's own record, as a `tool.decided` event and in the census.
    Driven,
    /// The loop is ours, and the published toolset **is** the policy.
    ///
    /// The fourth treatment, and it is not a fourth flavour of the third. In `plugin` the policy is
    /// injected into a vendor's loop; in `driven` it is imposed on one from outside, per call, at a
    /// seam. Here there is no vendor loop: the tools a run may call are computed from what the
    /// machine can confine and published, so a tool outside the surface is not refused — it does
    /// not exist. What this arm measures is whether *that* changes what a model does.
    ///
    /// **Enforcement model: publication by absence, which is not enforcement on a path.** Nothing
    /// adjudicates a call here, so a store-integrity cell — `store_broken` absent,
    /// `census.denied = 0` — says **compliance, or not observable, and never enforced**, unless the
    /// run carried a `scope:` or a loop hook that was in a position to refuse. A clean row on this
    /// arm means what a clean row on `raw` means.
    Native,
}

impl Arm {
    /// Reads the word a manifest wrote, or nothing when it is not one of the arms.
    pub fn parse(written: &str) -> Option<Self> {
        match written {
            "raw" => Some(Self::Raw),
            "plugin" => Some(Self::Plugin),
            "driven" => Some(Self::Driven),
            "native" => Some(Self::Native),
            _ => None,
        }
    }

    /// The word a manifest writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Plugin => "plugin",
            Self::Driven => "driven",
            Self::Native => "native",
        }
    }

    /// Every arm, in programme order, for a refusal that lists what was expected.
    ///
    /// It is the *whole* list or it is a lie: this array is the only thing `EVAL-MANIFEST-002`
    /// prints, so an arm missing from it is an arm the refusal tells a reader does not exist.
    /// `dce6db5` added `Native` to the enum, to `parse` and to `as_str` and left this array at
    /// three, so the refusal that exists to *list the arms* omitted one — which is why
    /// `the_arms_the_refusal_lists_are_every_arm_the_type_has` checks it against
    /// `ValueEnum::value_variants` rather than against a second hand-written list.
    pub const ALL: [Self; 4] = [Self::Raw, Self::Plugin, Self::Driven, Self::Native];
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
pub enum Outcome {
    /// The expectation held.
    Held,
    /// The run contradicted it.
    Violated,
    /// The record could not decide it — or recorded no verdict at all.
    Unobservable,
    /// An **advisory** row contradicted it.
    ///
    /// Its own column, and this is the decision: an advisory row judges whether the *evidence* is
    /// worth anything, not whether the run did anything wrong. Counting one in `violated` would
    /// publish `violated: 1` against a run that behaved perfectly, and a matrix that mislabels good
    /// runs is one nobody trusts about bad ones. Dropping it instead would be worse — an
    /// observation nobody can see is one that stops being made.
    Advisory,
}

/// The three counts, which is all a cell of the matrix ever holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Counts {
    /// How many facts held.
    pub held: usize,
    /// How many were contradicted.
    pub violated: usize,
    /// How many nobody could find out.
    pub unobservable: usize,
    /// How many advisory rows were contradicted. Observations, not violations.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub advisory: usize,
}

/// Whether a count is zero, so a matrix written before advisory rows existed reads unchanged.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub const fn is_zero(count: &usize) -> bool {
    *count == 0
}

impl Counts {
    /// Adds one outcome.
    pub fn add(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Held => self.held += 1,
            Outcome::Violated => self.violated += 1,
            Outcome::Unobservable => self.unobservable += 1,
            Outcome::Advisory => self.advisory += 1,
        }
    }

    /// Adds another cell's counts.
    pub fn absorb(&mut self, other: Self) {
        self.held += other.held;
        self.violated += other.violated;
        self.unobservable += other.unobservable;
        self.advisory += other.advisory;
    }
}

// --- refusals ---------------------------------------------------------------------------------

/// Every way a pair of documents is refused at this boundary, by name.
///
/// A code and a sentence, on the reasoning invariant 4 gives for `ValidationCode`: a test matching
/// on `EVAL-MANIFEST-005` still passes when the sentence is rewritten, and a test matching on the
/// sentence pins prose that nobody meant to freeze.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The document does not claim to be a run manifest.
    NotAManifest {
        /// What it claimed instead, where it claimed anything.
        found: Option<String>,
    },
    /// The `arm` is not one the programme has.
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
    /// Arm `plugin` is the arm whose subject is the plugin, and this manifest names no plugin at
    /// all — neither a directory digest nor a marketplace plugin.
    PluginDigestAbsentOnPluginArm,
    /// Arm `raw` is the arm with no plugin in it, and this manifest lists a marketplace plugin.
    MarketplacePluginOnRawArm {
        /// The first one it lists.
        plugin: String,
    },
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
    /// A required record field was not written.
    RecordFieldMissing {
        /// Which field.
        field: &'static str,
    },
    /// A required record field was written as `null`.
    RecordFieldNull {
        /// Which field.
        field: &'static str,
    },
    /// A required record field was empty.
    RecordFieldEmpty {
        /// Which field.
        field: &'static str,
    },
    /// A required record field had the wrong shape or value grammar.
    RecordFieldMalformed {
        /// Which field.
        field: &'static str,
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
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotAManifest { .. } => "EVAL-MANIFEST-001",
            Self::ArmUnknown { .. } => "EVAL-MANIFEST-002",
            Self::FieldMissing { .. } => "EVAL-MANIFEST-003",
            Self::FieldEmpty { .. } => "EVAL-MANIFEST-004",
            Self::PluginDigestOnRawArm => "EVAL-MANIFEST-005",
            Self::PluginDigestAbsentOnPluginArm => "EVAL-MANIFEST-006",
            Self::MarketplacePluginOnRawArm { .. } => "EVAL-MANIFEST-008",
            Self::DigestMalformed { .. } => "EVAL-MANIFEST-007",
            Self::NotARecord { .. } => "EVAL-RECORD-001",
            Self::RowWithoutId { .. } => "EVAL-RECORD-002",
            Self::VerdictUnknown { .. } => "EVAL-RECORD-003",
            Self::RecordFieldMissing { .. } => "EVAL-RECORD-004",
            Self::RecordFieldNull { .. } => "EVAL-RECORD-005",
            Self::RecordFieldEmpty { .. } => "EVAL-RECORD-006",
            Self::RecordFieldMalformed { .. } => "EVAL-RECORD-007",
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
    // One arm per refusal, each carrying the whole sentence a person reads. The same reasoning
    // `RunRefusal`'s own `Display` gives below: splitting it would put half the sentences somewhere
    // else without making any of them shorter, and reading them together is the point.
    #[allow(clippy::too_many_lines)]
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
                "`{written}` is not one of the arms this evaluation has: {}. Another arm is a \
                 change to the programme, not a label",
                known_arms()
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
                 `plugin_digest: null` and lists no `plugins`. A matrix row that cannot say which \
                 plugin was measured measures nothing.\n\
                 \n\
                 Either mechanism answers this: a directory copied in with `--plugin-dir` writes \
                 the digest, and a pinned marketplace plugin installed with `--plugin` writes a \
                 `plugins` entry. What is refused is a treated arm naming neither"
            ),
            Self::MarketplacePluginOnRawArm { plugin } => write!(
                f,
                "arm `raw` is the arm with no plugin in it, and this manifest lists the \
                 marketplace plugin `{plugin}`. Either the run had it — in which case it is arm \
                 `plugin` — or the list belongs to another run"
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
            refusal @ (Self::RecordFieldMissing { .. }
            | Self::RecordFieldNull { .. }
            | Self::RecordFieldEmpty { .. }
            | Self::RecordFieldMalformed { .. }) => fmt_record_field_refusal(f, refusal),
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

/// Lists the experiment's arms for a refusal without lengthening its formatter.
pub fn known_arms() -> String {
    Arm::ALL
        .iter()
        .map(|arm| format!("`{arm}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Renders the four ways a required record field can fail at the JSON boundary.
pub fn fmt_record_field_refusal(f: &mut fmt::Formatter<'_>, refusal: &Refusal) -> fmt::Result {
    match refusal {
        Refusal::RecordFieldMissing { field } => write!(
            f,
            "the record states no `{field}`, so its verdict cannot be bound to the input that produced it"
        ),
        Refusal::RecordFieldNull { field } => write!(
            f,
            "the record writes `{field}: null`, but this field must identify the input that produced its verdict"
        ),
        Refusal::RecordFieldEmpty { field } => write!(
            f,
            "the record's `{field}` is empty, which binds its verdict to nothing"
        ),
        Refusal::RecordFieldMalformed { field } => write!(
            f,
            "the record's `{field}` is not in the shape this boundary requires"
        ),
        _ => unreachable!("only record-field refusals are passed here"),
    }
}

/// The ` (it claims to be X)` clause, where a document claimed anything at all.
pub fn claimed(found: Option<&str>) -> String {
    found.map_or_else(
        || " and states no `format`".to_owned(),
        |format| format!(" — it states `format: {format}`"),
    )
}

/// Every refusal in one message, in the shape this binary already uses for a refused document.
pub fn refused(subject: &Path, refusals: &[Refusal]) -> anyhow::Error {
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
pub struct RawRunManifest {
    /// The format claim.
    pub format: Option<String>,
    /// Which arm.
    pub arm: Option<String>,
    /// Which harness ran it.
    pub harness: Option<String>,
    /// The workflow the case is a run of.
    pub workflow: Option<String>,
    /// The case or task.
    pub case: Option<String>,
    /// The plugin the run was given, or an explicit `null`.
    ///
    /// A [`Written`] and not an `Option`, because serde maps *the key is not there* and *the key
    /// says `null`* onto the same `None` — and those are the two facts this field exists to keep
    /// apart.
    #[serde(default, deserialize_with = "written_down")]
    pub plugin_digest: Written,
    /// The pinned marketplace plugins the run declared and the attestation confirmed.
    ///
    /// An `Option`-free `Vec` and **not** a [`Written`], which is the asymmetry with the field
    /// above rather than an inconsistency: `plugin_digest`'s two absences are different facts
    /// (*nobody recorded which plugin* against *there was no plugin*), and this list's are not —
    /// a manifest with no `plugins` key and one with an empty list both say the run installed
    /// nothing from a marketplace. So the key is written only where there is something to write,
    /// and every manifest committed before `--plugin` existed keeps its bytes.
    #[serde(default)]
    pub plugins: Vec<RawManifestPlugin>,
    /// The model, as the harness resolved it, or an explicit `null`.
    ///
    /// A [`Written`] for `plugin_digest`'s reason, and it earned it the same way — on a live run.
    /// Codex's wire states no model at session start at all, so *nobody wrote the model down* and
    /// *the harness never said which model* are different facts, and only the second is a run this
    /// verb can honestly describe. The key is still required.
    #[serde(default, deserialize_with = "written_down")]
    pub model: Written,
    /// The model the run **asked** for, where it asked for one.
    ///
    /// A plain `Option` and not a [`Written`], which is the asymmetry with the field above rather
    /// than an inconsistency: `model`'s two absences are different facts (*nobody recorded it*
    /// against *the harness never said*), and this one's are not — a manifest with no
    /// `model_requested` key and one that asked for nothing are the same run. So the key is written
    /// only where there is something to write, and every manifest assembled before `--model`
    /// existed keeps its bytes.
    pub model_requested: Option<String>,
    /// The harness version the arm is pinned to.
    pub harness_version: Option<String>,
    /// The transcript the record beside it was judged over.
    pub transcript_digest: Option<String>,
    /// When the run was observed.
    pub observed_at: Option<String>,
    /// What it cost, in millionths of a US dollar.
    pub cost_micro_usd: Option<u64>,
    /// How many tokens it used.
    pub tokens: Option<u64>,
    /// How long it took, in milliseconds.
    pub wall_time_ms: Option<u64>,
}

/// One marketplace plugin as a manifest writes it down.
///
/// `deny_unknown_fields` for [`RawRunManifest`]'s reason: the document is ours, so a misspelled key
/// here is a typo to refuse and not another producer's field to tolerate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawManifestPlugin {
    /// The spelling the run declared: `<repo>@<name>@<pin>`.
    pub plugin: Option<String>,
    /// The digest the instrument attested for it.
    pub digest: Option<String>,
}

/// One marketplace plugin, once it has been read through the rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestPlugin {
    /// The spelling the run declared.
    pub plugin: String,
    /// The digest the instrument attested.
    pub digest: String,
}

/// What one run manifest says the run cost, in millionths of a US dollar.
///
/// `None` when the file is not there, is not a `eval.run-manifest/1`, or stated no cost — and
/// **never `Some(0)`** for any of those. A run whose cost nobody recorded and a run that was free
/// are different facts; the table that reads this prints the first as `unknown`, on the same
/// reasoning the matrix totals a cell over the runs that stated a cost rather than treating an
/// absent key as zero.
///
/// Read through [`RawRunManifest`] and not through a fresh reader, so the one document format this
/// crate publishes has one parser: a second reader that accepted a manifest this one refuses would
/// be a second definition of what a manifest is.
pub fn manifest_cost_micro_usd(path: &Path) -> Option<u64> {
    let text = std::fs::read_to_string(path).ok()?;
    let raw: RawRunManifest = serde_yaml::from_str(&text).ok()?;
    (raw.format.as_deref() == Some(MANIFEST_FORMAT))
        .then_some(raw.cost_micro_usd)
        .flatten()
}

/// What a document wrote for a key that must be written down.
///
/// Three states, because there are three: the key is not there, the key is there and says `null`,
/// the key is there and says something. Collapsing the first two is what an `Option` does, and it
/// is the collapse `plugin_digest` cannot survive — *nobody recorded which plugin* and *there was
/// no plugin* are the difference between a manifest with a hole in it and a run of arm `raw`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Written {
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
pub fn written_down<'de, D>(deserializer: D) -> Result<Written, D::Error>
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
pub struct RunManifest {
    /// Which arm.
    pub arm: Arm,
    /// Which harness.
    pub harness: String,
    /// The workflow.
    pub workflow: String,
    /// The case or task.
    pub case: String,
    /// The plugin the run was given, where it had one.
    pub plugin_digest: Option<String>,
    /// The pinned marketplace plugins it was given, where it had any.
    pub plugins: Vec<ManifestPlugin>,
    /// The model, where the harness said which.
    pub model: Option<String>,
    /// The model the run asked for, where it asked for one.
    ///
    /// Read and kept apart from [`Self::model`]; nothing here reconciles the two, and `eval matrix`
    /// groups on neither. A phase that fixed a model checks it by reading both.
    pub model_requested: Option<String>,
    /// The harness version pin.
    pub harness_version: String,
    /// The transcript this manifest claims to describe.
    pub transcript_digest: String,
    /// When the run was observed, as the manifest wrote it.
    pub observed_at: String,
    /// What it cost, in millionths of a US dollar, where it said.
    pub cost_micro_usd: Option<u64>,
    /// How many tokens, where it said.
    pub tokens: Option<u64>,
    /// How long, in milliseconds, where it said.
    pub wall_time_ms: Option<u64>,
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

        let plugins = manifest_plugins(&mut refusals, arm, &raw.plugins);
        let plugin_digest = plugin_digest(&mut refusals, arm, &raw.plugin_digest, &plugins);

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
            plugins,
            model,
            model_requested: raw.model_requested,
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
pub fn required(
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
pub fn written_or_null(
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
pub fn check_digest(refusals: &mut Vec<Refusal>, field: &'static str, written: &str) {
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

/// The `plugins` rule: every entry names a plugin and a digest, and arm `raw` lists none.
///
/// The list is the marketplace half of the treatment, and it is validated **before** the digest
/// rule below because that rule now reads it: arm `plugin` is satisfied by either mechanism, and
/// only a manifest naming neither is refused.
pub fn manifest_plugins(
    refusals: &mut Vec<Refusal>,
    arm: Option<Arm>,
    written: &[RawManifestPlugin],
) -> Vec<ManifestPlugin> {
    let mut plugins = Vec::new();
    for entry in written {
        let Some(plugin) = required(refusals, "plugins[].plugin", entry.plugin.as_deref()) else {
            continue;
        };
        let Some(digest) = required(refusals, "plugins[].digest", entry.digest.as_deref()) else {
            continue;
        };
        check_digest(refusals, "plugins[].digest", &digest);
        plugins.push(ManifestPlugin { plugin, digest });
    }
    if arm == Some(Arm::Raw) {
        if let Some(first) = plugins.first() {
            refusals.push(Refusal::MarketplacePluginOnRawArm {
                plugin: first.plugin.clone(),
            });
        }
    }
    plugins
}

/// The `plugin_digest` rule: written always, and `null` on arm `raw` or where the treatment came
/// from a marketplace instead of a directory.
///
/// Arm `driven` may answer either way, and that is a decision rather than an oversight. What
/// enforces a driven run is the driver at the seam; whether the plugin was *also* installed is a
/// fact about that run, so both answers describe a run that could have happened — and the key is
/// still required, so the manifest has to state which.
///
/// **The two mechanisms stay apart.** `plugin_digest` is the digest of a directory metaharness
/// copied in with `--plugin-dir`; `plugins` is what it placed into the scratch config home from a
/// marketplace with `--plugin`. Arm `plugin` needs one of the two and not a particular one, so a
/// bench arm whose whole treatment is a pinned third-party plugin writes `plugin_digest: null`
/// honestly rather than being refused for having no directory to hash.
pub fn plugin_digest(
    refusals: &mut Vec<Refusal>,
    arm: Option<Arm>,
    written: &Written,
    plugins: &[ManifestPlugin],
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
        (Some(Arm::Plugin), Written::Null) if plugins.is_empty() => {
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
pub struct RawRecord {
    /// The format claim.
    pub format: Option<String>,
    /// The specification's id.
    #[serde(default, deserialize_with = "json_written")]
    pub spec_id: WrittenJson,
    /// The specification's digest.
    #[serde(default, deserialize_with = "json_written")]
    pub spec_digest: WrittenJson,
    /// The transcript it was judged over.
    #[serde(default, deserialize_with = "json_written")]
    pub transcript_digest: WrittenJson,
    /// One row per expectation.
    #[serde(default, deserialize_with = "json_written")]
    pub expectations: WrittenJson,
}

/// Whether a producer wrote a JSON key, preserving explicit `null` and malformed values.
#[derive(Debug, Default)]
pub enum WrittenJson {
    /// The key was not present.
    #[default]
    Absent,
    /// The key was present with this JSON value, including `null`.
    Value(serde_json::Value),
}

/// Deserializes a present JSON key without imposing its field grammar yet.
pub fn json_written<'de, D>(deserializer: D) -> Result<WrittenJson, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde_json::Value::deserialize(deserializer).map(WrittenJson::Value)
}

/// One expectation's row.
#[derive(Debug, Deserialize)]
pub struct RawRow {
    /// The id the specification gave it.
    pub id: Option<String>,
    /// `advisory` or `gate`, as the checker wrote it. Absent reads as a gate row, which is the safe
    /// default: a row whose severity nobody stated is one whose contradiction should be seen.
    pub severity: Option<String>,
    /// The verdict after the expectation's own `on_unknown` policy — the same value the report's
    /// summary counts and its exit code is derived from.
    pub verdict: Option<String>,
}

/// What a record says, once read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The specification's id.
    pub specification: String,
    /// The specification's digest.
    pub spec_digest: String,
    /// The transcript it was judged over.
    pub transcript_digest: String,
    /// Each expectation and what it said, in the record's own order.
    pub rows: Vec<(String, Outcome)>,
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

        let specification = record_string(&mut refusals, "spec_id", &raw.spec_id, false);
        let spec_digest = record_string(&mut refusals, "spec_digest", &raw.spec_digest, true);
        let transcript_digest = record_string(
            &mut refusals,
            "transcript_digest",
            &raw.transcript_digest,
            true,
        );
        let expectations = record_expectations(&mut refusals, &raw.expectations);

        let mut rows = Vec::new();
        for (position, row) in expectations.unwrap_or_default().into_iter().enumerate() {
            let Some(id) = row.id.filter(|id| !id.trim().is_empty()) else {
                refusals.push(Refusal::RowWithoutId { position });
                continue;
            };
            match outcome_of(row.verdict.as_deref(), row.severity.as_deref()) {
                Ok(outcome) => rows.push((id, outcome)),
                Err(written) => refusals.push(Refusal::VerdictUnknown { id, written }),
            }
        }

        if !refusals.is_empty() {
            return Err(refusals);
        }

        Ok(Self {
            specification: specification.expect("a specification id was read"),
            spec_digest: spec_digest.expect("a specification digest was read"),
            transcript_digest: transcript_digest.expect("a transcript digest was read"),
            rows,
        })
    }
}

/// Reads a required record string and, for digest fields, checks its exact grammar.
pub fn record_string(
    refusals: &mut Vec<Refusal>,
    field: &'static str,
    written: &WrittenJson,
    digest: bool,
) -> Option<String> {
    match written {
        WrittenJson::Absent => {
            refusals.push(Refusal::RecordFieldMissing { field });
            None
        }
        WrittenJson::Value(serde_json::Value::Null) => {
            refusals.push(Refusal::RecordFieldNull { field });
            None
        }
        WrittenJson::Value(serde_json::Value::String(value)) if value.trim().is_empty() => {
            refusals.push(Refusal::RecordFieldEmpty { field });
            None
        }
        WrittenJson::Value(serde_json::Value::String(value)) => {
            let well_formed = !digest
                || (value.len() == DIGEST_WIDTH
                    && value.chars().all(|character| {
                        character.is_ascii_digit() || ('a'..='f').contains(&character)
                    }));
            if well_formed {
                Some(value.clone())
            } else {
                refusals.push(Refusal::RecordFieldMalformed { field });
                None
            }
        }
        WrittenJson::Value(_) => {
            refusals.push(Refusal::RecordFieldMalformed { field });
            None
        }
    }
}

/// Reads the required, non-empty expectation rows while keeping malformed JSON inside this boundary.
pub fn record_expectations(refusals: &mut Vec<Refusal>, written: &WrittenJson) -> Option<Vec<RawRow>> {
    match written {
        WrittenJson::Absent => {
            refusals.push(Refusal::RecordFieldMissing {
                field: "expectations",
            });
            None
        }
        WrittenJson::Value(serde_json::Value::Null) => {
            refusals.push(Refusal::RecordFieldNull {
                field: "expectations",
            });
            None
        }
        WrittenJson::Value(serde_json::Value::Array(rows)) if rows.is_empty() => {
            refusals.push(Refusal::RecordFieldEmpty {
                field: "expectations",
            });
            None
        }
        WrittenJson::Value(value @ serde_json::Value::Array(_)) => {
            if let Ok(rows) = serde_json::from_value(value.clone()) {
                Some(rows)
            } else {
                refusals.push(Refusal::RecordFieldMalformed {
                    field: "expectations",
                });
                None
            }
        }
        WrittenJson::Value(_) => {
            refusals.push(Refusal::RecordFieldMalformed {
                field: "expectations",
            });
            None
        }
    }
}

/// The polarity of this whole verb, in one function.
///
/// A verdict the record does not state — absent, or written `null` — is **unobservable**. Never
/// held: a checker that recorded nothing about a row has not established anything about it, and a
/// matrix that counted silence as a pass would be the one number this programme refuses to produce.
/// A word this build cannot read is refused instead of bucketed, because *the checker said
/// something new* and *nobody found out* are different facts.
pub fn outcome_of(verdict: Option<&str>, severity: Option<&str>) -> Result<Outcome, String> {
    match verdict {
        Some("ok") => Ok(Outcome::Held),
        // An advisory row's gap is an observation about the evidence, not a contradiction by the
        // run. Absent severity reads as a gate row: a row whose severity nobody stated is one whose
        // contradiction should be seen.
        Some("gap") if severity == Some("advisory") => Ok(Outcome::Advisory),
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
pub struct Reported {
    /// How many runs in the cell stated this quantity.
    pub runs: usize,
    /// Their total.
    pub total: u64,
}

/// The three resource columns of a cell, each absent until some run states one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Resources {
    /// Cost, in millionths of a US dollar.
    pub cost_micro_usd: Option<Reported>,
    /// Tokens.
    pub tokens: Option<Reported>,
    /// Wall time, in milliseconds.
    pub wall_time_ms: Option<Reported>,
}

impl Resources {
    /// Folds one run's quantities in.
    pub fn absorb(&mut self, manifest: &RunManifest) {
        add(&mut self.cost_micro_usd, manifest.cost_micro_usd);
        add(&mut self.tokens, manifest.tokens);
        add(&mut self.wall_time_ms, manifest.wall_time_ms);
    }
}

/// Adds a run's quantity to a column, creating the column the first time one is stated.
pub fn add(column: &mut Option<Reported>, stated: Option<u64>) {
    let Some(value) = stated else { return };
    let reported = column.get_or_insert(Reported { runs: 0, total: 0 });
    reported.runs += 1;
    reported.total = reported.total.saturating_add(value);
}

/// One run, as the matrix reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunRow {
    /// The workflow.
    pub workflow: String,
    /// The case.
    pub case: String,
    /// The harness.
    pub harness: String,
    /// The arm.
    pub arm: Arm,
    /// The model, where the harness said which — `null` and never an omitted key, because a wire
    /// that states no model is stating something.
    pub model: Option<String>,
    /// The harness version pin.
    pub harness_version: String,
    /// The plugin the run was given — `null` on arm `raw`, and never an omitted key.
    pub plugin_digest: Option<String>,
    /// The specification its record was judged by.
    pub specification: String,
    /// The transcript.
    pub transcript_digest: String,
    /// When it was observed.
    pub observed_at: String,
    /// What its expectations said.
    #[serde(flatten)]
    pub counts: Counts,
    /// What it cost, where it said — `null` and never an omitted key, for the manifest's reason.
    pub cost_micro_usd: Option<u64>,
    /// How many tokens, where it said.
    pub tokens: Option<u64>,
    /// How long, where it said.
    pub wall_time_ms: Option<u64>,
}

/// One harness × arm × workflow cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cell {
    /// The workflow.
    pub workflow: String,
    /// The harness.
    pub harness: String,
    /// The arm.
    pub arm: Arm,
    /// How many runs went into it.
    pub runs: usize,
    /// What their expectations said.
    #[serde(flatten)]
    pub counts: Counts,
    /// What they cost, over the runs that said.
    #[serde(flatten)]
    pub resources: Resources,
}

/// One expectation in one cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExpectationRow {
    /// The workflow.
    pub workflow: String,
    /// The expectation's id.
    pub expectation: String,
    /// The harness.
    pub harness: String,
    /// The arm.
    pub arm: Arm,
    /// How many runs judged it.
    pub runs: usize,
    /// What they said.
    #[serde(flatten)]
    pub counts: Counts,
}

/// The specification a set of records was judged by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecificationRow {
    /// Its id.
    pub id: String,
    /// Its digest, where the records state one.
    pub digest: Option<String>,
    /// How many runs it judged.
    pub runs: usize,
}

/// The deliverable: counts of facts, per run, per cell and per expectation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Matrix {
    /// The format claim.
    pub format: &'static str,
    /// The specifications the records were judged by.
    pub specifications: Vec<SpecificationRow>,
    /// Every run.
    pub runs: Vec<RunRow>,
    /// Every harness × arm × workflow cell.
    pub cells: Vec<Cell>,
    /// Every expectation, per cell.
    pub expectations: Vec<ExpectationRow>,
    /// The three counts over everything, which is as close to a summary as this document goes.
    #[serde(flatten)]
    pub totals: Counts,
}

/// Assembles the matrix, or every reason it cannot be assembled.
///
/// Sorted by construction: the pairs arrive sorted by path, and every aggregate is built in a
/// `BTreeMap` whose key is the tuple it is grouped by (invariant 9 — no `HashMap` anywhere near an
/// output ordering).
pub fn assemble(pairs: Vec<(RunManifest, Record)>) -> Result<Matrix, Vec<Refusal>> {
    let mut refusals = Vec::new();

    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (manifest, record) in &pairs {
        *seen.entry(manifest.transcript_digest.clone()).or_default() += 1;
        if record.transcript_digest != manifest.transcript_digest {
            refusals.push(Refusal::TranscriptMismatch {
                manifest: manifest.transcript_digest.clone(),
                record: record.transcript_digest.clone(),
            });
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
        entry.0.insert(record.spec_digest.clone());
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
pub fn per_expectation(
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
pub fn fold(
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
pub fn collect(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
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
pub fn sibling(path: &Path, from: &str, to: &str) -> PathBuf {
    let name = path.to_string_lossy();
    PathBuf::from(format!("{}{to}", name.trim_end_matches(from)))
}

/// Reads one pair: the manifest, then the record beside it.
pub fn read_pair(manifest_path: &Path) -> Result<(RunManifest, Record)> {
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
pub enum MatrixFormat {
    /// Human-readable tables.
    Text,
    /// JSON, for another tool to read.
    Json,
}

/// What can be done with a set of evaluation runs.
#[derive(Debug, Subcommand)]
pub enum EvalCommand {
    /// Assemble the outcome matrix from run manifests and the records they accompany.
    ///
    /// One row per run, one cell per harness × arm × workflow, one row per expectation per cell —
    /// each of them three counts: held, contradicted, and nobody found out. No score, no ranking
    /// and no percentage: see the [module documentation](self) for why a scalar cannot be produced
    /// here honestly.
    ///
    /// The arm word is also the enforcement model, and there is no column for it. On `driven` every
    /// call was answered at a seam, so a clean store-integrity row can mean *refused*; on `raw` and
    /// `native` nothing adjudicated anything, so the same row means only that the model did not do
    /// it. A rendering that prints a `native` cell says so under the table.
    Matrix(MatrixArgs),
    /// Run one arm of one case and leave the three documents `eval matrix` reads.
    ///
    /// The runner drives `metaharness` as a **tool**, the way this repository drives `git`: the
    /// binary is looked for on `PATH`, a machine without it is told so by name and exits `2`, and
    /// no crate crosses in either direction. Nothing spawns without `METAHARNESS_LIVE=1` and a
    /// `--budget-usd` cap; `--stream` ingests a run somebody already recorded and spends nothing.
    Run(Box<RunArgs>),
}

/// The arguments of `protocol eval matrix`.
#[derive(Debug, Args)]
pub struct MatrixArgs {
    /// The runs: a directory holding `*.manifest.yaml` beside `*.report.json`, or a manifest
    /// itself. Several may be named.
    #[arg(required = true)]
    pub runs: Vec<PathBuf>,
    /// How to render it.
    #[arg(long, value_enum, default_value_t = MatrixFormat::Text)]
    pub format: MatrixFormat,
    /// Where to write it. Without it, the matrix goes to standard output.
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// The `eval` verb family, one arm per subcommand.
pub fn run(command: EvalCommand) -> Result<ExitCode> {
    match command {
        EvalCommand::Matrix(args) => matrix(&args),
        EvalCommand::Run(args) => run_arm(&args),
    }
}

/// `protocol eval matrix`
pub fn matrix(args: &MatrixArgs) -> Result<ExitCode> {
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
pub fn to_text(matrix: &Matrix) -> String {
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
        ("advisory", 9),
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
            (&cell.counts.advisory.to_string(), 9),
        ]));
    }

    // The arm's own word is the only enforcement label a cell gets, and one of the four does not
    // mean what the column above reads as. Printed only when a `native` cell is in the table, and
    // as a line rather than a column: a column is a change to this table's format, which is the
    // operator's decision (`docs/design/native-arm-store-integrity-design-v0.1.md` § 6 O1, § 8
    // OQ4).
    if matrix.cells.iter().any(|cell| cell.arm == Arm::Native) {
        lines.push(String::new());
        lines.push(
            "reading a `native` cell: nothing on that arm adjudicates a call, so a clean \
             store-integrity row is compliance or not observable, and never enforced — unless the \
             run carried a `scope:` or a loop hook that could refuse. `denied: 0` there says \
             nobody asked, not that nothing was refused."
                .to_owned(),
        );
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
        ("advisory", 9),
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
            (&expectation.counts.advisory.to_string(), 9),
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
pub fn row(columns: &[(&str, usize)]) -> String {
    let rendered: Vec<String> = columns
        .iter()
        .map(|(text, width)| format!("{text:<width$}"))
        .collect();
    rendered.join("  ").trim_end().to_owned()
}

/// A cost column: dollars from micro-dollars by integer arithmetic, never a float.
pub fn cost(reported: Option<Reported>, runs: usize) -> String {
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
pub fn quantity(reported: Option<Reported>, runs: usize, unit: &str) -> String {
    reported.map_or_else(
        || "—".to_owned(),
        |reported| format!("{}{unit} {}", reported.total, coverage(reported.runs, runs)),
    )
}

/// `(2/3)`, and nothing at all when every run in the cell answered.
pub fn coverage(reporting: usize, runs: usize) -> String {
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
pub const METAHARNESS_BINARY: &str = "metaharness";

/// Where to look for that binary when it is not on `PATH` under its own name.
pub const METAHARNESS_BIN_ENV: &str = "METAHARNESS_BIN";

/// The environment variable that must say `1` before anything is spawned and paid for.
pub const METAHARNESS_LIVE_ENV: &str = "METAHARNESS_LIVE";

/// The exit code for *the tool this verb drives is not installed*.
///
/// Distinct from `1`, which is every refusal about a document, because the two want different
/// reactions: one is *install something*, the other is *fix what you wrote*. It is the code the
/// programme's design constant 4 asks for — an absent binary is a skip, never a red gate — so a
/// caller can tell the two apart without reading prose off stderr.
pub const TOOL_MISSING_EXIT: u8 = 2;

/// How a run's raw event stream is named on disk.
pub const EVENTS_SUFFIX: &str = ".events.jsonl";

/// What a run whose stream states no cost is counted at, in US dollars.
///
/// Conservative rather than accurate, and it is a budget input rather than a measurement: a run
/// whose cost nobody recorded is counted against the cap at this rate so that an unpriced wire
/// cannot spend without limit. It never reaches a manifest — `cost_micro_usd` stays absent there,
/// because the matrix reports totals over the runs that stated one and an assumed number would
/// silently become a measurement.
pub const ASSUMED_USD_PER_RUN: &str = "0.25";

/// Which harness a run is of.
///
/// Closed here and open in the manifest, and the asymmetry is the difference between reading a run
/// and launching one. The matrix takes `harness` as free text because a third harness is a run and
/// not a redesign; the runner has to know which vendor word `metaharness run` takes, while the
/// plugin directory is an explicit machine-local input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Harness {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
    /// The b10x harness: our own agent loop, over a model API directly.
    B10x,
}

impl Harness {
    /// The word `metaharness run` takes, and the word the manifest writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::B10x => "b10x",
        }
    }

}

impl fmt::Display for Harness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A marketplace plugin, spelled the way metaharness 0.5.0 spells one: `<repo>@<name>@<pin>`.
///
/// **Forwarded verbatim, resolved never.** metaharness `run claude --plugin` reads the operator's
/// own `known_marketplaces.json` and `installed_plugins.json`, matches the pin against an entry's
/// `version` or its `gitCommitSha`, and copies the tree into the scratch config home. None of that
/// is this runner's: a second resolver here would be a second thing to get wrong and a second
/// place for the two builds to disagree about what a pin means.
///
/// What this side owns is the spelling. A `--plugin` that names no pin is refused **before**
/// anything is spawned, because the run underneath would refuse it anyway — at parse, before a
/// `RunSpec` exists — and an operator should meet that refusal once rather than after paying for a
/// process to start and stop. The sentence is metaharness's own, quoted rather than reworded:
/// `docs/design/runs-side-by-side-v0.1.md` § 3.2 and `MarketplacePluginError` in
/// `metaharness-protocol`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketplacePlugin {
    /// The marketplace's **source repository**, as the caller named it — never its name.
    pub repo: String,
    /// The plugin's own name inside that marketplace.
    pub name: String,
    /// The pin: a version or a commit, and which of the two it is, is metaharness's to decide.
    pub pin: String,
}

impl MarketplacePlugin {
    /// Reads one `--plugin` value, or the sentence metaharness refuses it with.
    ///
    /// Split on the **last two** `@`, which is metaharness's own rule and the reason [`fmt::Display`]
    /// below reproduces the caller's bytes exactly: a repository spelling never contains one and a
    /// plugin name never does, while a pin might.
    ///
    /// # Errors
    ///
    /// The refusal's own words, for the two spellings that name nothing reproducible: fewer than
    /// three segments, and a segment that is there and empty.
    pub fn parse(given: &str) -> Result<Self, String> {
        let unpinned = || {
            format!(
                "`{given}` names no pin. Write `<repo>@<name>@<version-or-commit>`: an unpinned \
                 plugin can change between two runs that both claim to have used it, which makes \
                 the two arms of a comparison incomparable and neither of them reproducible"
            )
        };
        let (head, pin) = given.rsplit_once('@').ok_or_else(unpinned)?;
        let (repo, name) = head.rsplit_once('@').ok_or_else(unpinned)?;
        for (segment, value) in [("repo", repo), ("name", name), ("pin", pin)] {
            if value.is_empty() {
                return Err(format!(
                    "`{given}` has an empty {segment}. Write \
                     `<repo>@<name>@<version-or-commit>`; a blank segment names nothing and would \
                     be resolved against everything"
                ));
            }
        }
        Ok(Self {
            repo: repo.to_owned(),
            name: name.to_owned(),
            pin: pin.to_owned(),
        })
    }

    /// Whether the instrument's attestation row is about this plugin.
    ///
    /// The match is on `source`, which is the instrument's own identifier for what it placed —
    /// `<repo>@<name>@<pin> (marketplace <name>)` — and deliberately not on `loaded_by`, which is a
    /// sentence written for a person, or on `name`, which two marketplaces may share.
    pub fn attested_by(&self, source: &str) -> bool {
        let spelling = self.to_string();
        source == spelling || source.starts_with(&format!("{spelling} "))
    }
}

impl fmt::Display for MarketplacePlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}@{}", self.repo, self.name, self.pin)
    }
}

// --- what the runner refuses before anything is spawned -----------------------------------------

/// Every way the runner refuses to start, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunRefusal {
    /// The binary this verb drives is not installed.
    ToolMissing {
        /// What was looked for, and where.
        looked_for: String,
    },
    /// A live spawn was asked for and the environment does not permit one.
    NotLive,
    /// A live spawn was asked for with no cap on what it may spend.
    NoBudget,
    /// The plugin arm was asked for and no plugin was named, by either mechanism.
    NoPluginTreatment,
    /// A `--plugin` value names nothing that can be reproduced.
    PluginSpelling {
        /// What was written.
        given: String,
        /// metaharness's own sentence for it.
        detail: String,
    },
    /// `--plugin` was given for a harness that has no marketplace.
    PluginHarnessHasNoMarketplace {
        /// The harness with none.
        harness: Harness,
    },
    /// `--model` was given for a harness whose adapter does not take one.
    ModelHarnessTakesNoModel {
        /// The harness that does not.
        harness: Harness,
        /// What was asked for, so the operator can move it to the right invocation.
        model: String,
    },
    /// Arm `raw` is the arm with no plugin in it, and a `--plugin` names one.
    PluginOnRawArm {
        /// What was named.
        plugin: String,
    },
    /// Arm `driven` is not launched from here.
    DrivenIsNotLaunchedHere,
    /// Arm `native` is not launched from here either, and for a different reason.
    NativeIsNotLaunchedHere,
    /// The `aep` the spawned session would run is not this binary's version.
    ChildAepMismatch {
        /// The binary the child's `PATH` resolves.
        child: String,
        /// What it reported.
        found: String,
        /// What this runner is.
        own: String,
        /// The `PATH` the child gets.
        child_path: String,
    },
    /// A case's subject names an `ess-specify` (or `ess-schema`) skill and the child's `PATH` has
    /// no `ess`.
    ChildEssMissing {
        /// The case.
        case: String,
        /// The `PATH` the child gets.
        child_path: String,
    },
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
    pub fn code(&self) -> &'static str {
        match self {
            Self::ToolMissing { .. } => "EVAL-RUN-001",
            Self::NotLive => "EVAL-RUN-002",
            Self::NoBudget => "EVAL-RUN-003",
            Self::NoPluginTreatment => "EVAL-RUN-012",
            Self::PluginSpelling { .. } => "EVAL-RUN-013",
            Self::PluginHarnessHasNoMarketplace { .. } => "EVAL-RUN-014",
            Self::PluginOnRawArm { .. } => "EVAL-RUN-015",
            Self::ModelHarnessTakesNoModel { .. } => "EVAL-RUN-016",
            Self::DrivenIsNotLaunchedHere => "EVAL-RUN-004",
            Self::NoWorkingTree => "EVAL-RUN-005",
            Self::BudgetWouldBeExceeded { .. } => "EVAL-RUN-006",
            Self::NoCase => "EVAL-RUN-007",
            Self::StreamIsOneRun { .. } => "EVAL-RUN-008",
            Self::InstructionsMissing { .. } => "EVAL-RUN-009",
            Self::SpawnFailed { .. } => "EVAL-RUN-010",
            Self::NativeIsNotLaunchedHere => "EVAL-RUN-011",
            Self::ChildAepMismatch { .. } => "EVAL-RUN-017",
            Self::ChildEssMissing { .. } => "EVAL-RUN-018",
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
            Self::NoPluginTreatment => write!(
                f,
                "arm `plugin` needs a plugin: `--plugin-dir DIR` for one checked out on this \
                 machine, or `--plugin <repo>@<name>@<version-or-commit>` for one installed from a \
                 marketplace. AEP does not bundle agent plugins and guesses no path; name the \
                 treatment explicitly, and the launch record will say which mechanism delivered it"
            ),
            Self::PluginSpelling { given, detail } => write!(
                f,
                "`--plugin {given}` is refused before anything is spawned, and this is \
                 metaharness 0.5.0's own sentence for it: {detail}.\n\
                 \n\
                 This runner forwards `--plugin` verbatim and resolves nothing, so the spelling is \
                 checked here rather than after a process has been started to be refused"
            ),
            Self::PluginHarnessHasNoMarketplace { harness } => write!(
                f,
                "`--plugin` names a marketplace plugin and `{harness}` has no marketplace \
                 metaharness 0.5.0 can resolve one from — it refuses the pair by name rather than \
                 accepting and ignoring it, and so does this. An operator who declared a plugin \
                 would otherwise believe the run had one.\n\
                 \n\
                 `--plugin` is Claude Code only. A checked-out plugin directory is not: \
                 `--plugin-dir DIR` is offered on every harness metaharness drives"
            ),
            Self::ModelHarnessTakesNoModel { harness, model } => write!(
                f,
                "`--model {model}` is forwarded to `metaharness run <harness> --model`, and \
                 `{harness}` has no adapter that takes one at metaharness 0.5.0 — it is refused by \
                 name rather than accepted and dropped, because a run that silently used the \
                 default model would enter the matrix as a run that pinned one.\n\
                 \n\
                 `--model` is Claude Code only. When the other adapters take it, this refusal is \
                 what has to be removed for them, one harness at a time"
            ),
            Self::ChildAepMismatch {
                child,
                found,
                own,
                child_path,
            } => write!(
                f,
                "the `aep` the session will run is not this one. metaharness gives the child a \
                 constructed PATH ({child_path}), never the runner's own, and the first `aep` on it \
                 is {child}, version {found}; this runner is {own}. A case's task runs that binary, \
                 so a live spawn now would pay for a run against a stale CLI — on 2026-09-03 a 0.40.1 \
                 there stopped the golden path at `unrecognized subcommand 'doctor'` after $10.96. \
                 Refresh it (`task install` in the aep checkout rewrites a real ~/.local/bin/aep), or \
                 point ~/.local/bin/aep at this binary, then run again"
            ),
            Self::ChildEssMissing { case, child_path } => write!(
                f,
                "case `{case}` names an {spellings} skill in its subject, and the child's PATH \
                 ({child_path}) has no `ess`: the step that skill runs would be drafted by hand and \
                 never validated, which is what the 2026-09-03 recording showed. Put `ess` on \
                 ~/.local/bin (a symlink to ~/.cargo/bin/ess is enough) and run again",
                spellings = ess_skill_prefixes_phrase()
            ),
            Self::PluginOnRawArm { plugin } => write!(
                f,
                "arm `raw` is the arm with no plugin in it, and `--plugin {plugin}` names one. \
                 Either the run is arm `plugin`, or the flag belongs to another invocation.\n\
                 \n\
                 The same contradiction is refused after a run too, off the stream's own \
                 attestation (`EVAL-STREAM-007`) — this is the half that costs nothing to find"
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
pub enum CaseRefusal {
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
    pub fn code(&self) -> &'static str {
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
pub enum StreamRefusal {
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
    /// A `--plugin` was declared and the instrument attests nothing that is it.
    MarketplacePluginUnattested {
        /// What was declared.
        plugin: String,
        /// What the attestation does name, so the reader can see the near miss.
        attested: Vec<String>,
    },
}

impl StreamRefusal {
    /// The stable code a test matches on.
    pub fn code(&self) -> &'static str {
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
            Self::MarketplacePluginUnattested { .. } => "EVAL-STREAM-013",
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
            Self::MarketplacePluginUnattested { plugin, attested } => write!(
                f,
                "this run declared `--plugin {plugin}` and \
                 `session.started.{INSTALLED_PLUGINS}` attests nothing whose `source` is it. \
                 The attestation names: {}.\n\
                 \n\
                 The runner declares and the instrument attests; the manifest records what **both** \
                 said. A declaration nothing attested would enter the matrix as a plugin the run \
                 never had, which is the one thing this document exists to make impossible",
                if attested.is_empty() {
                    "nothing".to_owned()
                } else {
                    attested.join(", ")
                }
            ),
        }
    }
}

/// The format claim an eval case carries.
pub const CASE_FORMAT: &str = "eval-case/1";

/// The `subject.skills` prefixes whose skill runs `ess` inside the session.
///
/// Two spellings for one plugin, and both are load-bearing. `ess-specify:` is the current id;
/// `ess-schema:` is what the plugin was called before `agentplugins@a2077d2` renamed it, and every
/// case authored or recorded under the old name still spells it that way. `EVAL-RUN-018` exists to
/// stop a live spawn paying for a session whose `ess` step would be drafted by hand and never
/// validated, so it has to fire on whichever spelling the case in front of it happens to use —
/// dropping the old one would silently switch the preflight off for the corpus that already exists.
///
/// The matcher, the refusal a person reads, and the test that holds them together all read this
/// list, so a third spelling is one edit and cannot be added to one of the three alone.
pub const ESS_SKILL_PREFIXES: [&str; 2] = ["ess-specify:", "ess-schema:"];

/// Every spelling in [`ESS_SKILL_PREFIXES`], as a phrase for the refusal message.
pub fn ess_skill_prefixes_phrase() -> String {
    ESS_SKILL_PREFIXES
        .map(|prefix| format!("`{prefix}`"))
        .join(" or ")
}

// --- the case ------------------------------------------------------------------------------------

/// An eval case as it is written down.
///
/// Only the fields the runner needs, and deliberately **not** `deny_unknown_fields`: the corpus's
/// shape has an owner already — `crates/edge/aep-cli/tests/eval_corpus.rs` reads every field and
/// denies the unknown — and a second denier would be a second place to update when a case grows a
/// key, which is how two readers of one document start disagreeing.
///
/// That deference is only sound while the owner accepts what this reader keys on. It did not: the
/// corpus reader had no `subject` and refused a case declaring the block `EVAL-RUN-018` reads. Both
/// halves are now asserted there, by
/// `a_case_may_declare_what_it_is_about_and_a_typo_inside_it_is_refused`.
#[derive(Debug, Deserialize)]
pub struct RawCase {
    /// The format claim.
    pub format: Option<String>,
    /// The case's id, which must be its directory's name.
    pub id: Option<String>,
    /// The workflow it is a run of.
    pub workflow: Option<String>,
    /// What the agent is asked to do.
    pub task: Option<String>,
    /// The `trace-spec/1` document it is judged by, relative to the case directory.
    pub expectations: Option<String>,
    /// What the case is a case about. Only `skills` is read here, and only to ask what the child's
    /// `PATH` has to hold (`preflight_child_path`); the corpus test owns the rest of the shape.
    pub subject: Option<RawSubject>,
}

/// The `subject:` block of a case, as far as this runner reads it.
#[derive(Debug, Deserialize)]
pub struct RawSubject {
    /// The skills the case is about, `<plugin>:<skill>`.
    pub skills: Option<Vec<String>>,
}

/// A case, once read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    /// Its id.
    pub id: String,
    /// The workflow.
    pub workflow: String,
    /// The task statement.
    pub task: String,
    /// The document it is judged by.
    pub expectations: PathBuf,
    /// Whether the case's subject names a skill under one of [`ESS_SKILL_PREFIXES`], whose step
    /// runs `ess` in the session.
    pub needs_ess: bool,
}

/// Reads the case in a directory.
pub fn read_case(directory: &Path) -> Result<Case> {
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
    let needs_ess = raw
        .subject
        .as_ref()
        .and_then(|subject| subject.skills.as_ref())
        .is_some_and(|skills| {
            skills.iter().any(|skill| {
                ESS_SKILL_PREFIXES
                    .iter()
                    .any(|prefix| skill.starts_with(prefix))
            })
        });

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
        needs_ess,
    })
}

/// Every refusal in one message, in this binary's shape for a refused document.
pub fn refused_run<R: fmt::Display>(subject: &Path, refusals: &[R]) -> anyhow::Error {
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
pub fn select_cases(args: &RunArgs) -> Result<Vec<Case>> {
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
pub struct Session {
    /// The harness and its version, as the manifest spells it: `claude 2.1.239`.
    pub harness_version: String,
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
    pub model: Option<String>,
    /// The plugin that was installed from a directory, where one was.
    pub plugin_digest: Option<String>,
    /// The pinned marketplace plugins the run declared, with the digests the instrument attested.
    ///
    /// In the order they were declared, which is the order they reached the argv: a manifest whose
    /// list order moved between two ingests of the same stream would diff against itself.
    pub plugins: Vec<AttestedPlugin>,
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
    /// It then summed **every** terminal record until 2026-09-03, when a golden-path recording
    /// reported `$30.002816` for a session that spent `$15.00140784`. One session had written two
    /// terminal records: the `result` for the work, and a second one saying
    /// `subtype: error_max_budget_usd` because `--max-budget-usd` had stopped it. Both restate the
    /// same running total, so summing them charged the run twice — and the row that read the figure
    /// called a $15 run a $30 one against a $15 cap.
    ///
    /// So the fold is per session: [`highest`] within one, [`accumulate`] across them. Both runs
    /// above now report what they spent. [`accumulate`] holds the absence rule: a session stating
    /// nothing adds nothing and never a zero, which is the same rule [`add`] applies one level up
    /// when a cell totals over its runs.
    pub cost_micro_usd: Option<u64>,
    /// How many tokens it used, totalled over every session that said.
    ///
    /// Same fold as [`Self::cost_micro_usd`], and the reason [`highest`] takes the larger rather
    /// than the last: the budget-exhausted record restates the cost but zeroes its `usage`.
    pub tokens: Option<u64>,
    /// How long it took, totalled over every session that said.
    ///
    /// A sum and not a span: the driver runs its sessions one after another, and nothing here reads
    /// a clock to find out (invariant 9). Within one session it is the same fold as the other two.
    pub wall_time_ms: Option<u64>,
}

/// One marketplace plugin, as the run declared it and the instrument attested it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedPlugin {
    /// The spelling the run declared, `<repo>@<name>@<pin>`.
    pub plugin: String,
    /// The digest the attestation stated for it, byte for byte.
    pub digest: String,
}

/// The four token counts a `usage` object carries, which is what the manifest's `tokens` totals.
pub const TOKEN_KEYS: [&str; 4] = [
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
    pub fn read(
        events: &[u8],
        arm: Arm,
        harness: Harness,
        declared: &[MarketplacePlugin],
    ) -> Result<Self, Vec<StreamRefusal>> {
        let text = String::from_utf8_lossy(events);
        let mut started: Option<serde_json::Value> = None;
        // One bucket per session, in stream order, each holding that session's terminal records.
        // A stream that opens with a terminal record before any `session.started` gets the leading
        // bucket, so nothing is dropped on the way to `NoTerminalEvent`.
        let mut sessions: Vec<Vec<serde_json::Value>> = vec![Vec::new()];
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
                Some("session.started") => {
                    if started.is_none() {
                        started = Some(value);
                    }
                    sessions.push(Vec::new());
                }
                Some("session.ended") => sessions
                    .last_mut()
                    .expect("the leading bucket is never popped")
                    .push(value),
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

        let (plugin_digest, plugins) =
            plugin_attestation(&mut refusals, &started, arm, declared);

        if sessions.iter().all(Vec::is_empty) {
            refusals.push(StreamRefusal::NoTerminalEvent);
            return Err(refusals);
        }

        // Read before the emptiness check, so an unreadable cost joins the other refusals rather
        // than being discovered after them (invariant 3: validation accumulates).
        let Totals {
            cost,
            tokens,
            wall_time_ms,
        } = totals_of(&sessions, &mut refusals);

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
            plugins,
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
pub fn accumulate(total: &mut Option<u64>, stated: Option<u64>) {
    let Some(value) = stated else { return };
    let running = total.get_or_insert(0);
    *running = running.saturating_add(value);
}

/// The three quantities [`Session`] totals off a stream's terminal records.
pub struct Totals {
    /// Observed spend in microdollars, absent when unstated.
    pub cost: Option<u64>,
    /// Observed token use, absent when unstated.
    pub tokens: Option<u64>,
    /// Observed elapsed milliseconds, absent when unstated.
    pub wall_time_ms: Option<u64>,
}

/// Totals a stream's terminal records: [`highest`] within a session, [`accumulate`] across them.
///
/// The buckets arrive in stream order, one per `session.started`, and an empty one totals nothing.
/// An unreadable cost is pushed as a refusal and the rest of the fold continues, because validation
/// accumulates (invariant 3) rather than stopping at the first thing it cannot read.
pub fn totals_of(sessions: &[Vec<serde_json::Value>], refusals: &mut Vec<StreamRefusal>) -> Totals {
    let mut totals = Totals {
        cost: None,
        tokens: None,
        wall_time_ms: None,
    };
    for terminal in sessions {
        let mut cost = None;
        let mut tokens = None;
        let mut wall_time_ms = None;
        for ended in terminal {
            match cost_of(ended) {
                Ok(figure) => highest(&mut cost, figure),
                Err(reason) => refusals.push(StreamRefusal::CostUnreadable { reason }),
            }
            highest(&mut tokens, tokens_of(ended));
            highest(
                &mut wall_time_ms,
                ended.get("duration_ms").and_then(serde_json::Value::as_u64),
            );
        }
        accumulate(&mut totals.cost, cost);
        accumulate(&mut totals.tokens, tokens);
        accumulate(&mut totals.wall_time_ms, wall_time_ms);
    }
    totals
}

/// Keeps the larger of one session's terminal records, where the record stated one.
///
/// [`accumulate`]'s sibling, and the split between them is which axis is being crossed. That one
/// crosses **sessions**, where two figures are two spends. This one crosses the terminal records of
/// **one** session, where two figures are one spend counted twice: `total_cost_usd`, `usage` and
/// `duration_ms` are all running counters the harness restates in full every time it writes one.
///
/// The larger and not the last, because the records are not ordered by completeness: Claude Code's
/// budget-exhausted record restates the cost and the wall clock but zeroes its `usage`, so *last*
/// would report a run of no tokens. Absent stays absent, as in [`accumulate`].
pub fn highest(total: &mut Option<u64>, stated: Option<u64>) {
    let Some(value) = stated else { return };
    let running = total.get_or_insert(value);
    *running = (*running).max(value);
}

/// Where the instrument attests what it installed, as against what a vendor happened to echo.
pub const INSTALLED_PLUGINS: &str = "hermetic.installed_plugins";

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
pub fn plugin_attestation(
    refusals: &mut Vec<StreamRefusal>,
    started: &serde_json::Value,
    arm: Arm,
    declared: &[MarketplacePlugin],
) -> (Option<String>, Vec<AttestedPlugin>) {
    let Some(entries) = started
        .get("hermetic")
        .and_then(|hermetic| hermetic.get("installed_plugins"))
        .and_then(serde_json::Value::as_array)
    else {
        refusals.push(StreamRefusal::SessionSaysNothing {
            field: INSTALLED_PLUGINS,
        });
        return (None, Vec::new());
    };

    if arm == Arm::Raw {
        if let Some(entry) = entries.first() {
            refusals.push(StreamRefusal::PluginAttestedOnRawArm {
                source: plugin_source(entry),
            });
        }
        return (None, Vec::new());
    }

    // The split, and the whole of *keeping the two apart*: the run said which marketplace plugins
    // it asked metaharness to place, and the instrument's rows say what arrived. A row claimed by a
    // declaration is the marketplace treatment; every row left over is what `--plugin-dir` copied,
    // and it goes on being the single `plugin_digest` the matrix has always read. Nothing here
    // parses `loaded_by`, which is a sentence for a person and not an identifier.
    let mut claimed = vec![false; entries.len()];
    let mut plugins = Vec::new();
    for wanted in declared {
        let found = entries.iter().enumerate().find(|(position, entry)| {
            !claimed[*position] && wanted.attested_by(&plugin_source(entry))
        });
        let Some((position, entry)) = found else {
            refusals.push(StreamRefusal::MarketplacePluginUnattested {
                plugin: wanted.to_string(),
                attested: entries.iter().map(plugin_source).collect(),
            });
            continue;
        };
        claimed[position] = true;
        match entry.get("digest").and_then(serde_json::Value::as_str) {
            Some(digest) if !digest.trim().is_empty() => plugins.push(AttestedPlugin {
                plugin: wanted.to_string(),
                digest: digest.to_owned(),
            }),
            _ => refusals.push(StreamRefusal::PluginWithoutDigest {
                name: wanted.to_string(),
            }),
        }
    }

    let directories: Vec<&serde_json::Value> = entries
        .iter()
        .enumerate()
        .filter(|(position, _)| !claimed[*position])
        .map(|(_, entry)| entry)
        .collect();

    if entries.is_empty() && arm == Arm::Plugin {
        refusals.push(StreamRefusal::PluginUnattestedOnPluginArm);
        return (None, plugins);
    }

    if directories.len() > 1 {
        refusals.push(StreamRefusal::SeveralPluginsAttested {
            names: directories.iter().map(|entry| plugin_name(entry)).collect(),
        });
        return (None, plugins);
    }

    let Some(entry) = directories.first() else {
        return (None, plugins);
    };

    match entry.get("digest").and_then(serde_json::Value::as_str) {
        Some(digest) if !digest.trim().is_empty() => (Some(digest.to_owned()), plugins),
        _ => {
            refusals.push(StreamRefusal::PluginWithoutDigest {
                name: plugin_name(entry),
            });
            (None, plugins)
        }
    }
}

/// An entry's `source`, which is the instrument's own identifier for what it placed.
///
/// Falls back to the name where a row states none, so a refusal still says which row it is about.
pub fn plugin_source(entry: &serde_json::Value) -> String {
    entry
        .get("source")
        .and_then(serde_json::Value::as_str)
        .map_or_else(|| plugin_name(entry), ToOwned::to_owned)
}

/// A plugin entry's name, however the attestation spelled it.
pub fn plugin_name(entry: &serde_json::Value) -> String {
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
pub fn cost_of(ended: &serde_json::Value) -> Result<Option<u64>, String> {
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
/// [`crate::money::micro_usd`]'s sibling, and the split between them is the whole of this fix. That one reads an
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
pub fn micro_usd_stated(written: &str) -> Result<u64, String> {
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
pub fn tokens_of(ended: &serde_json::Value) -> Option<u64> {
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

/// The stream as it goes to disk and into the digest: redacted when the caller asked for it.
///
/// One function, called on both paths, because the manifest's `transcript_digest` is taken over
/// whatever this returns — a runner that wrote redacted bytes and digested the raw ones would
/// publish a manifest naming a file that does not exist.
pub fn stream_for(events: Vec<u8>, redact: bool, cwd: Option<&Path>) -> Vec<u8> {
    if !redact {
        return events;
    }
    // The run's own `--cwd` as well as this process's, because the identity that reaches a stream
    // is the one the **session** committed with. A runner checked out with a bot `user.name` and a
    // fixture carrying the operator's own is the ordinary case here, not a corner of it.
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut at: Vec<&Path> = vec![here.as_path()];
    if let Some(cwd) = cwd {
        at.push(cwd);
    }
    crate::redaction::redacted(events, &at)
}

// --- assembling one run's three documents ---------------------------------------------------------

/// The three documents one run leaves behind.
pub struct Products {
    /// The `eval.run-manifest/1` document.
    pub manifest: String,
    /// The `trace-report/1` record `protocol trace check --format json` writes.
    pub report: String,
    /// What the run cost, for the budget, where its stream stated one.
    pub cost_micro_usd: Option<u64>,
    /// What the check said, for the line the runner prints.
    pub verdict: String,
    /// The exit code that verdict calls for, straight off the record.
    ///
    /// [`trace_spec::report::CheckReport::exit_code`] and never a second table: the sentence in
    /// `verdict` above already names a code — `not conformant … (exit 1)` — and a runner that
    /// derived the status from anywhere else could print one number and exit another, which is
    /// what this one did until `story:eval-run-stream-exit-status`.
    pub exit_code: u8,
    /// The model the stream stated, for the same line — `None` where it stated none.
    pub model: Option<String>,
}

/// One run of one case in one arm on one harness.
pub struct Plan {
    /// The case.
    pub case: Case,
    /// The arm.
    pub arm: Arm,
    /// The harness.
    pub harness: Harness,
    /// The pinned marketplace plugins this run declared, in the order they were declared.
    ///
    /// On the plan rather than read back off the stream, because they are the runner's own fact —
    /// *this run asked for these* — and the stream's job is to say whether they arrived. The two
    /// are joined in [`plugin_attestation`], and a declaration nothing attested is refused there.
    pub plugins: Vec<MarketplacePlugin>,
    /// The model this run asked for, where it named one.
    ///
    /// The runner's own fact for the same reason the plugins are, and kept **apart** from the model
    /// the attestation reports rather than reconciled with it: the flag states and the vendor
    /// resolves, so `claude-sonnet-4-6` asked for and `claude-sonnet-4-6-20260814` attested is a
    /// run that went as planned, and a runner that folded the two into one field would have thrown
    /// away the only evidence that it did.
    pub model_requested: Option<String>,
}

impl Plan {
    /// How this run's three documents are named, which is also how the matrix pairs them.
    pub fn name(&self) -> String {
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
pub fn ingest(plan: &Plan, events: &[u8], observed_at: &str, redact: bool) -> Result<Products> {
    let ir = trace_spec::reader::read_any(events).map_err(|errors| {
        refused_run(
            &plan.case.expectations,
            &[StreamRefusal::Unreadable {
                reason: errors.to_string(),
            }],
        )
    })?;
    let session = Session::read(events, plan.arm, plan.harness, &plan.plugins)
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
        exit_code: report.exit_code(),
        model: session.model.clone(),
    })
}

/// The manifest, as the bytes that go on disk.
///
/// Written out rather than serialised from a struct, for two reasons that are the same reason: the
/// key order is the one the committed fixtures already use, so two waves' manifests diff against
/// each other; and `plugin_digest` has to appear as an explicit `null` on arm `raw`, which is the
/// one field whose *absence* the matrix refuses.
pub fn manifest_text(
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
    // Beside `plugin_digest` and never merged into it, because the two answer different questions:
    // one names the bytes of a directory the operator checked out, the other names a pinned
    // third-party plugin and the pin it was resolved at. Written only where there is one, so every
    // manifest assembled before `--plugin` existed keeps its bytes and two waves still diff.
    // Immediately after the model the attestation reported, because that is the field a reader
    // compares it against. Written **only** where the run named one, on the `plugins:` block's
    // reasoning: every manifest assembled before `--model` existed keeps its bytes, so two waves
    // still diff and no golden churns for a column nobody added.
    if let Some(requested) = &plan.model_requested {
        let after_model = lines
            .iter()
            .position(|line| line.starts_with("model:"))
            .expect("the manifest always writes a model line")
            + 1;
        lines.insert(after_model, format!("model_requested: {requested}"));
    }
    if !session.plugins.is_empty() {
        let block = std::iter::once("plugins:".to_owned())
            .chain(session.plugins.iter().map(|plugin| {
                format!("  - plugin: {}\n    digest: {}", plugin.plugin, plugin.digest)
            }))
            .collect::<Vec<_>>()
            .join("\n");
        // Immediately after `plugin_digest`, which is the field it is a companion to and the one a
        // reader compares it against. Found rather than counted, so a later field added above it
        // cannot silently move the block somewhere else.
        let after_digest = lines
            .iter()
            .position(|line| line.starts_with("plugin_digest:"))
            .expect("the manifest always writes a plugin_digest line")
            + 1;
        lines.insert(after_digest, block);
    }
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

// --- the verb -------------------------------------------------------------------------------------

/// The arguments of `protocol eval run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// A case directory — one holding a `case.yaml`. Repeatable.
    #[arg(long = "case", value_name = "DIR")]
    pub cases: Vec<PathBuf>,
    /// Every case of one workflow, taken from the corpus.
    #[arg(long, value_name = "WORKFLOW_ID")]
    pub workflow: Option<String>,
    /// Where the corpus is, for `--workflow`.
    #[arg(long, value_name = "DIR", default_value = "conformance/eval")]
    pub corpus: PathBuf,
    /// Which arm this run belongs to.
    ///
    /// `driven` and `native` are refused for a spawn and accepted for `--stream`: a driven run is
    /// launched by `protocol drive run` and a native run by `b10x-harness`, and this verb reads the
    /// stream each of them wrote.
    ///
    /// The word is also the run's enforcement model, because that is the only label a matrix cell
    /// ever carries. `driven` answers every call at a seam; `plugin` injects a policy into a
    /// vendor's loop; `raw` and `native` adjudicate nothing at all — so a clean store-integrity row
    /// on a `native` cell is compliance or not observable, and never enforced, unless the run
    /// carried a `scope:` or a loop hook that was in a position to refuse.
    #[arg(long, value_enum)]
    pub arm: Arm,
    /// Which harness runs it.
    #[arg(long, value_enum)]
    pub harness: Harness,
    /// Where the three documents per run are written.
    #[arg(long, value_name = "DIR")]
    pub out: PathBuf,
    /// Ingest a stream that already exists instead of spawning one.
    ///
    /// The whole runner minus the spawn, and it spends nothing: no binary is looked for, no
    /// environment variable is consulted and no cap is needed. It is how a driven run enters the
    /// matrix, how a paid run is re-ingested after its manifest rules changed, and how `task check`
    /// exercises this pipeline end to end for free.
    #[arg(long, value_name = "FILE")]
    pub stream: Option<PathBuf>,
    /// When the run was observed, as a date or epoch milliseconds.
    ///
    /// Required, and deliberately not defaulted to now as `protocol trace evidence` does. The
    /// difference is what the document is for: an evidence record is minted by the process that
    /// performed the observation, and a manifest is a committed document that must assemble to the
    /// same bytes twice. A clock in it would make every re-ingest a diff.
    #[arg(long, value_name = "DATE")]
    pub observed_at: String,
    /// The tree the session works in. Required for a spawn.
    #[arg(long, value_name = "DIR")]
    pub cwd: Option<PathBuf>,
    /// The external agent plugin to install for arm `plugin`.
    ///
    /// AEP deliberately ships no marketplace sources. Name the installed or checked-out plugin
    /// explicitly so the launch record identifies the treatment the run received.
    #[arg(long, value_name = "DIR")]
    pub plugin_dir: Option<PathBuf>,
    /// A pinned marketplace plugin to install, forwarded to `metaharness run claude` verbatim.
    ///
    /// `<repo>@<name>@<version-or-commit>`, repeatable, and combinable with `--plugin-dir` — the
    /// two mechanisms place a plugin differently, the attestation lists both, and the manifest
    /// keeps them apart. An unpinned spelling is refused before anything is spawned, in
    /// metaharness's own words: a plugin that can change between two runs that both name it makes
    /// the two arms of a comparison incomparable.
    ///
    /// Claude Code only, because that is the only harness metaharness 0.5.0 can resolve a
    /// marketplace for. The other kinds are refused by name rather than accepted and ignored.
    #[arg(long = "plugin", value_name = "REPO@NAME@PIN")]
    pub plugins: Vec<String>,
    /// The cap on what this invocation may spend, in US dollars. Required for a spawn.
    #[arg(long, value_name = "USD")]
    pub budget_usd: Option<String>,
    /// What a run whose stream states no cost is counted at, in US dollars.
    #[arg(long, value_name = "USD", default_value = ASSUMED_USD_PER_RUN)]
    pub assume_usd_per_run: String,
    /// The rendered instruction documents arm `raw` is given.
    #[arg(long, value_name = "DIR", default_value = "generated/instructions")]
    pub instructions: PathBuf,
    /// The model the harness is asked to use, forwarded to `metaharness run <harness> --model`.
    ///
    /// **Verbatim, and resolved never.** metaharness 0.5.0 passes the string through to the vendor,
    /// which resolves it; a runner that normalised an alias here would be pinning something other
    /// than what the operator wrote down, and the manifest would say the wrong thing about what a
    /// bench phase asked for.
    ///
    /// Claude Code only, on `--plugin`'s reasoning: the other adapters take no model flag at
    /// metaharness 0.5.0, and a flag accepted and dropped is worse than one refused.
    ///
    /// The manifest records this beside the `model` the attestation reports. The two are different
    /// facts — *what was asked for* and *what ran* — and a phase that fixes a model is checked by
    /// comparing them.
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,
    /// Cite event indices and digests only in the record beside each run.
    ///
    /// Opt-in, exactly as `protocol trace check --redact` is and for the same reason — a report is
    /// most useful with its evidence visible. Every record committed to this repository is written
    /// with it, because a report that quotes a transcript is not a thing to publish.
    #[arg(long)]
    pub redact: bool,
}

/// `protocol eval run --stream FILE`: the runner minus the spawn.
///
/// Split out of [`run_arm`] because it shares none of that function's machinery — no binary, no
/// live flag, no cap, no working tree — and because a reader looking for *what happens for free*
/// should find it in one piece.
///
/// **The status it answers with is the verdict**: `0` conformant, `1` contradicted, `3` undecided,
/// the codes `trace check` already uses. See the return below for why the spawn path does not.
pub fn ingest_recorded(
    args: &RunArgs,
    cases: Vec<Case>,
    stream: &Path,
    plugins: Vec<MarketplacePlugin>,
) -> Result<ExitCode> {
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
        plugins,
        model_requested: args.model.clone(),
    };
    let raw = std::fs::read(stream)
        .with_context(|| format!("reading the stream at {}", stream.display()))?;
    let events = stream_for(raw, args.redact, args.cwd.as_deref());
    // Judged **before** anything is written, so a refused ingest leaves the output directory as it
    // found it (invariant 7). The runner already assembles every document before writing one; the
    // redacted stream joins them rather than getting a head start.
    let products = ingest(&plan, &events, &args.observed_at, args.redact)?;
    // Under `--redact` the redacted stream is a **product**, and this is the path that produces it:
    // a paid run recorded on the operator's machine is re-ingested here to get the documents a
    // public `recorded/` directory takes, with the manifest's digest over the bytes that were
    // written. Without `--redact` the caller's file is the record and nothing is rewritten.
    let recorded = if args.redact {
        let path = args.out.join(format!("{}{EVENTS_SUFFIX}", plan.name()));
        std::fs::write(&path, &events).with_context(|| format!("writing {}", path.display()))?;
        Some(path)
    } else {
        None
    };
    write_products(&args.out, &plan, recorded.as_deref(), &products)?;
    // The verdict, as the status. A caller reading the exit code of an ingest gets the same answer
    // as a person reading the line it just printed — `trace check`'s own three codes, from the same
    // report: 0 conformant, 1 contradicted, 3 undecided. This was `SUCCESS` whatever the record
    // said until `story:eval-run-stream-exit-status`, and on 2026-09-03 a gate that read it
    // reported a replay contradicting two expectations as a replayed transcript.
    //
    // **The spawn path above deliberately does not do this.** It launches several runs against a
    // budget and its last line is a ledger — `2 run(s), $0.53 spent` — so there is no single
    // verdict for a status to agree with; a paid run that produced records must not read as a run
    // that failed to happen. `--stream` is one recorded run and one verdict, which is what makes
    // the status meaningful here.
    Ok(ExitCode::from(products.exit_code))
}

/// Refuses the two arms this verb reads and does not launch, each naming the verb that does.
///
/// A second way to launch either one would be a second policy to forget, which is the mistake
/// `epic:metaharness-migration` retired.
pub fn launched_elsewhere(args: &RunArgs) -> Result<()> {
    match args.arm {
        Arm::Driven => Err(refused_run(
            &args.out,
            &[RunRefusal::DrivenIsNotLaunchedHere],
        )),
        Arm::Native => Err(refused_run(
            &args.out,
            &[RunRefusal::NativeIsNotLaunchedHere],
        )),
        Arm::Raw | Arm::Plugin => Ok(()),
    }
}

/// Refuses a plugin arm whose treatment was not named explicitly, by either mechanism.
///
/// Invariant 12 unchanged and widened by one word: no repository-local fallback chooses a plugin,
/// and *a plugin* is now either a directory on this machine or a pinned marketplace coordinate.
/// Both are the operator's explicit authority; neither is guessed from a path under this checkout.
pub fn require_plugin_treatment(args: &RunArgs, plugins: &[MarketplacePlugin]) -> Result<()> {
    if args.arm == Arm::Plugin && args.plugin_dir.is_none() && plugins.is_empty() {
        return Err(refused_run(&args.out, &[RunRefusal::NoPluginTreatment]));
    }
    Ok(())
}

/// Reads the flags this runner forwards to `metaharness` verbatim, and refuses what it may not.
///
/// `--plugin`, whose spelling must be pinned and whose harness must have a marketplace, and
/// `--model`, whose harness must have an adapter that takes one.
///
/// **Before the tool is looked for and before the live flag is read**, so a spelling mistake costs
/// nothing to find and does not depend on what is installed. Every refusal is collected rather than
/// the first returned (invariant 3: validation accumulates) — an operator fixing two spellings
/// should be told about two.
pub fn declared_plugins(args: &RunArgs) -> Result<Vec<MarketplacePlugin>> {
    let mut refusals = Vec::new();
    let mut plugins = Vec::new();
    for given in &args.plugins {
        match MarketplacePlugin::parse(given) {
            Ok(plugin) => plugins.push(plugin),
            Err(detail) => refusals.push(RunRefusal::PluginSpelling {
                given: given.clone(),
                detail,
            }),
        }
    }
    // Refused by name on the harnesses whose adapters take no model, and on **both** paths: a
    // `--stream` re-ingest records `model_requested` in the manifest, so a flag this runner would
    // not have been able to forward must not be able to enter the record either.
    if let Some(model) = &args.model {
        if args.harness != Harness::Claude {
            refusals.push(RunRefusal::ModelHarnessTakesNoModel {
                harness: args.harness,
                model: model.clone(),
            });
        }
    }
    if !args.plugins.is_empty() {
        if args.harness != Harness::Claude {
            refusals.push(RunRefusal::PluginHarnessHasNoMarketplace {
                harness: args.harness,
            });
        }
        if args.arm == Arm::Raw {
            refusals.push(RunRefusal::PluginOnRawArm {
                plugin: args.plugins.join("`, `"),
            });
        }
    }
    if refusals.is_empty() {
        Ok(plugins)
    } else {
        Err(refused_run(&args.out, &refusals))
    }
}

/// How a model nobody stated is spelled for a person.
///
/// Obviously not a model name, and deliberately not blank: a column that renders an unstated model
/// as nothing reads exactly like a column nobody looked at. The matrix's own **text** rendering has
/// no model column at all — it groups by harness × arm × workflow — so this is where a person meets
/// one; its JSON writes `"model": null`, which is the same fact in the shape a program reads.
pub const MODEL_UNSTATED: &str = "(unstated)";

/// Writes one run's documents and says what was left where.
pub fn write_products(
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


/// Ingest a recorded evaluation stream without launching an execution host.
pub fn run_arm(args: &RunArgs) -> Result<ExitCode> {
    let stream = args.stream.as_ref().context("live evaluation moved to `metaharness aep drive eval run`; use --stream for offline ingestion")?;
    crate::observation_time(Some(&args.observed_at))?;
    let plugins = declared_plugins(args)?;
    let cases = select_cases(args)?;
    std::fs::create_dir_all(&args.out).with_context(|| format!("creating {}", args.out.display()))?;
    ingest_recorded(args, cases, stream, plugins)
}
