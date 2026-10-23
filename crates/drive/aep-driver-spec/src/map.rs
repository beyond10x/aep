//! The step map: the half of a run a workflow deliberately does not state.
//!
//! A step map is a document like the four already in the tree — parsed into a `Raw*` type,
//! validated into a domain type, published as a generated schema, cross-validated against the
//! documents it names. It says, per workflow state, what a harness should *do* while it is there.
//! Order inside a state is the map author's; order **between** states is the workflow's, and
//! nothing here can override it.
//!
//! # Cross-validation runs in two phases, because the two halves are knowable at different times
//!
//! | phase | what it checks | when |
//! |---|---|---|
//! | [`cross_validate`](StepMap::cross_validate) | every state the map names is a state of the workflow; every **named** verifier a step names can produce the kind it claims | at load, against the registry |
//! | [`check_run`](StepMap::check_run) | every evidence kind a step declares is declared by the protocol in force; the resolved workflow's id and version equal the pin | at run start, against the resolved plan |
//!
//! The second is not a duplicate of the first and cannot be folded into it: the protocol in force
//! comes from the **task**, which no document loader has seen. A loader that guessed would let a
//! map validate and then fail at the transition that needed the evidence — the exact failure the
//! check exists to prevent.
//!
//! **A named verifier is checked; an external tool is not.** `kinds_for_verifier` filters on
//! `default_verifiers()`, which is a table of **defaults** rather than of constraints — `diff`
//! defaults to `[compiler, static-analyzer]`, so a diff produced by `git` would be refused for a
//! defect that is not one. `Verifier::ExternalTool` is what `Verifier::parse` falls through to for
//! anything unrecognised, it appears in no row of that table, and its kind is still checked at run
//! start against the protocol. Review finding **F5**.
//!
//! # An `llm` step cannot carry an evidence block, and the type is what makes that true
//!
//! Not a validation rule that could be relaxed later: [`LlmStep`] has no evidence field. An
//! agent's own statement never satisfies an independence requirement, so a step kind that could
//! mint evidence from a model's output would be the single change that unpicks the whole loop.
//! Anything an `llm` step is supposed to have achieved that is *checkable* is observed by a
//! subsequent [`CommandStep`]: the model writes the code, and `cargo test` says whether it works.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use aep_domain::error::{ParseError, ValidationCode, ValidationError, ValidationErrors};
use aep_domain::evidence::{EvidenceKind, TestSuite};
use aep_domain::ids::{StateId, SubjectRef, ToolRef};
use aep_domain::protocol::Protocol;
use aep_domain::verification::Verifier;
use aep_domain::version::WorkflowRef;
use aep_domain::workflow::Workflow;

use crate::digest::digest_of_canonical;
use crate::pin::PinnedWorkflowRef;
use crate::{STEP_MAP_FORMAT, STEP_MAP_FORMAT_V1};

/// How many times a state may be entered before the driver stops, when the map does not say.
///
/// `adp/default` has a deliberate `verify → implement` back-edge, and the workflow's own comment
/// explains why: *"a workflow that can only go forwards is a lie about how engineering works"*. So
/// a driver must be able to go round again — and must not go round forever.
pub const DEFAULT_VISIT_BUDGET: u32 = 3;

/// How many times a `command` step is retried before its budget is spent, when the map is silent.
///
/// A retry is for *a process died*, not for *the suite is red*: a suite that ran and failed
/// produced a verdict, and the verdict is submitted rather than retried.
pub const DEFAULT_COMMAND_RETRIES: u32 = 2;

/// How many times an `llm` step is retried.
///
/// Once, and not configurable: a model call that errored is worth one more attempt, and a second
/// one is a token budget nobody stated.
pub const LLM_RETRIES: u32 = 1;

/// Identifier of a step map, such as `development/default`.
///
/// The same charset rule as a workflow identifier — lowercase kebab segments joined by `.` or `/`,
/// never ending in a numeric segment, because that form is reserved for a version reference.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct StepMapId(String);

impl StepMapId {
    /// The pattern published in generated JSON Schema.
    ///
    /// `Charset::Dotted`'s own pattern, taken from `aep-domain` rather than written here: this
    /// constant was a fourth copy of that rule, and it had already drifted from
    /// [`Self::new`] — it called `development/2` valid, which this constructor refuses.
    pub const PATTERN: &'static str = aep_domain::identifier_pattern!(Dotted, "^", "$");

    /// Parses and validates an identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let value = value.into();
        let reject = |reason: String| Err(ParseError::identifier("step map", &value, reason));
        if value.is_empty() {
            return reject("must not be empty".to_owned());
        }
        if value.len() > aep_domain::ids::MAX_LENGTH as usize {
            return reject(format!(
                "must be at most {} characters, got {}",
                aep_domain::ids::MAX_LENGTH,
                value.len()
            ));
        }
        for segment in value.split(['.', '/', '-']) {
            if segment.is_empty() {
                return reject(
                    "has an empty segment; separators (./-) must not lead, trail or repeat"
                        .to_owned(),
                );
            }
            if let Some(bad) = segment
                .chars()
                .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
            {
                return reject(format!("contains disallowed character {bad:?}"));
            }
        }
        if !value.starts_with(|c: char| c.is_ascii_lowercase()) {
            return reject("must start with a lowercase letter".to_owned());
        }
        if value
            .rsplit(['.', '/'])
            .next()
            .is_some_and(|last| last.chars().all(|c| c.is_ascii_digit()))
        {
            return reject(
                "must not end in a numeric segment; that form is reserved for version references"
                    .to_owned(),
            );
        }
        Ok(Self(value))
    }

    /// The identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for StepMapId {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<String> for StepMapId {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<StepMapId> for String {
    fn from(value: StepMapId) -> Self {
        value.0
    }
}

impl fmt::Display for StepMapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for StepMapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StepMapId({:?})", self.0)
    }
}

impl schemars::JsonSchema for StepMapId {
    fn schema_name() -> String {
        "StepMapId".to_owned()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            ..Default::default()
        };
        schema.string().pattern = Some(Self::PATTERN.to_owned());
        // The same bound `Self::new` applies, published because a `pattern` cannot express one:
        // without it the schema calls a 201-character step-map id valid and the loader refuses it.
        schema.string().max_length = Some(aep_domain::ids::MAX_LENGTH);
        schema.metadata().description =
            Some("Identifier of a step map, such as `development/default`.".to_owned());
        schema.into()
    }
}

/// A step map as it is written, before anything has checked what it claims.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawStepMap {
    /// The format version the document says it is written in.
    pub format: String,
    /// What this map is, such as `development/default`.
    pub id: StepMapId,
    /// The workflow it is written against, pinned to a major version.
    ///
    /// Read as a [`WorkflowRef`] so an unpinned one reaches validation and is refused there,
    /// **accumulating** with every other problem in the document rather than short-circuiting as a
    /// deserialization error would. The published schema is
    /// [`PinnedWorkflowRef`]'s, which is the strict one — the two
    /// halves of review finding F6.
    #[schemars(with = "PinnedWorkflowRef")]
    pub workflow: WorkflowRef,
    /// A human sentence for a report's heading.
    #[serde(default)]
    pub title: Option<String>,
    /// What to do in each state, by state name.
    #[serde(default)]
    pub states: BTreeMap<StateId, RawStateSteps>,
}

/// One state's steps, as written.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawStateSteps {
    /// How many times this state may be entered before the run stops.
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub visit_budget: Option<u32>,
    /// Whether the state's consecutive `command` steps run at once.
    ///
    /// Each maximal run of consecutive `command` steps is then one batch: its steps run at once,
    /// and their results are submitted in map order once every one of them has ended. An `llm` or
    /// `operator` step is a barrier between batches. Absent is `false`: one step at a time.
    #[serde(default)]
    pub concurrent: bool,
    /// The steps, in the order the author wants them run.
    #[serde(default)]
    pub steps: Vec<RawStep>,
}

/// One step, as written.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RawStep {
    /// Run a program and map its result to evidence.
    Command(RawCommandStep),
    /// Ask a model, with a tool set the protocol derived.
    Llm(RawLlmStep),
    /// Stop and hand the run to a person.
    Operator(RawOperatorStep),
}

/// How much of a file an operation class may change, where a scope rule admits it at all.
///
/// **Granularity, not identity.** A step map is a document about work; naming an operation here —
/// `file.write`, `file.edit` — would couple it to another protocol's vocabulary at its most
/// volatile point, and every operation added later would need a key in every map or silently fall
/// outside every scope. Which of a harness's operations replace a whole file is that harness's
/// fact, and it is the one place that knows.
///
/// Three words, and `PartialOnly` is not invented for the planning store: it is a property the tool
/// catalogues already document — *write one file, whole* against *change part of one*.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum WriteScope {
    /// Anything in the class may act on these paths.
    Allowed,
    /// Part of a file may be changed; a whole file may never be replaced.
    PartialOnly,
    /// Nothing in the class may act here.
    Denied,
}

/// One rule of a step's write scope, as written.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawScopeRule {
    /// The paths this rule is about, as globs relative to the working tree.
    #[schemars(length(min = 1), inner(regex(pattern = r"\S")))]
    pub paths: Vec<String>,
    /// What writing is allowed on them.
    pub write: WriteScope,
}

/// One validated rule of a step's write scope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
pub struct ScopeRule {
    /// The paths this rule is about.
    pub paths: Vec<String>,
    /// What writing is allowed on them.
    pub write: WriteScope,
}

/// One argv prefix a driven `llm` step's shell admits: the program, then the leading words.
///
/// **A prefix, not a pattern.** `[aep, plan, artifact]` admits `aep plan artifact new …` and
/// `aep plan artifact move …`; `[cargo, test]` admits `cargo test --workspace` and not `cargo
/// install`. The words are matched against one simple invocation — the driver has already refused
/// anything that composes, redirects or substitutes — so a prefix is a statement about *which
/// command*, in the vocabulary of the repository the map drives, and nothing finer. A rule about
/// which flags a command may carry is a rule the map cannot state, and this type does not pretend
/// it can.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
pub struct CommandPrefix {
    /// The words, program first. Never empty, never blank.
    pub words: Vec<String>,
}

impl CommandPrefix {
    /// The program, which is the first word.
    pub fn program(&self) -> &str {
        self.words.first().map_or("", String::as_str)
    }

    /// Programs a map may not declare, because each exists to run a program, or a string, the
    /// declaration did not name.
    ///
    /// *No general shell* is the rule the declaration exists to state, and a declared `bash` would
    /// state the opposite in the same key. Four groups: the shells and the shell words that
    /// evaluate a string (`eval`, `exec`, `source`, `.`, `command`, `builtin`); the wrappers that
    /// run their arguments as a program (`env`, `xargs`, `sudo`, `timeout`, `nohup`, `find` with
    /// `-exec`, …); the script interpreters, whose one-line programs are a shell in another
    /// syntax; and `npx`, which fetches a program and runs it.
    ///
    /// **A list of the runners the loader knows, not a proof.** A program outside it may still run
    /// another program through a flag of its own (`git -c alias.x='!…'`, `cargo --config
    /// target.…runner=…`), and the driver refuses the flags it knows at call time
    /// (`aep-cli`'s `RUNNER_FLAGS`). The property a map holds is the one its author kept: it
    /// names the programs the step's work needs, and the loader refuses the ones that name
    /// nothing.
    pub const PROGRAM_RUNNERS: &'static [&'static str] = &[
        // shells, and the shell words that evaluate a string
        "sh",
        "bash",
        "zsh",
        "dash",
        "fish",
        "ksh",
        "mksh",
        "csh",
        "tcsh",
        "busybox",
        "eval",
        "exec",
        "source",
        ".",
        "command",
        "builtin",
        // wrappers that run their arguments as a program
        "env",
        "xargs",
        "sudo",
        "doas",
        "su",
        "runuser",
        "pkexec",
        "run0",
        "systemd-run",
        "nohup",
        "timeout",
        "nice",
        "ionice",
        "setsid",
        "stdbuf",
        "script",
        "chroot",
        "unshare",
        "flock",
        "taskset",
        "time",
        "watch",
        "strace",
        "ltrace",
        "find",
        // interpreters of a program given on the command line
        "perl",
        "python",
        "python2",
        "python3",
        "node",
        "nodejs",
        "ruby",
        "php",
        "lua",
        "luajit",
        "deno",
        "bun",
        "awk",
        "gawk",
        "mawk",
        "nawk",
        // fetches a program and runs it
        "npx",
    ];
}

impl fmt::Display for CommandPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.words.join(" "))
    }
}

/// What is wrong with one context entry as the session would be handed it, if anything.
///
/// A blank entry names no file, and a glob is refused by name rather than expanded: a step map is
/// pinned, so a glob is a promise about what a directory will contain later. One rule for an entry
/// the map wrote ([`validated_context`]) and for one an input was filled into
/// ([`StepMap::check_inputs`]), so a value cannot hand a session what the loader refuses.
fn context_entry_defect(entry: &str) -> Option<String> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return Some("a blank context entry names no file".to_owned());
    }
    if trimmed.contains('*') || trimmed.contains('?') {
        return Some(format!(
            "`{trimmed}` is a glob, and context is a list of files: a step map is pinned, so a \
             glob is a promise about what a directory will hold later. Name the files."
        ));
    }
    None
}

/// Refuses, at `at`, every context entry that the run's inputs make blank or a glob.
///
/// Only an entry naming an input is read: the loader has already checked the others, and an
/// entry naming an input the run was not given is refused as missing beside this. The entry is
/// read with its inputs filled and `{run_directory}` as written, which is neither blank nor a glob.
fn check_filled_context(
    context: &[String],
    given: &BTreeMap<String, String>,
    at: &str,
    errors: &mut ValidationErrors,
) {
    for entry in context {
        let names: Vec<&str> = placeholders_in(entry)
            .into_iter()
            .filter_map(input_named_by)
            .collect();
        if names.is_empty() || !names.iter().all(|name| given.contains_key(*name)) {
            continue;
        }
        let Ok(filled) = substitute(entry, |name| match input_named_by(name) {
            Some(_) => input_value(name, given),
            None => Ok(None),
        }) else {
            continue;
        };
        if let Some(defect) = context_entry_defect(&filled) {
            errors.push(
                ValidationError::new(
                    ValidationCode::EmptyDeclaration,
                    at.to_owned(),
                    format!(
                        "`{entry}` with {} filled in: {defect}",
                        names
                            .iter()
                            .map(|name| format!("`--input {name}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
                .with_hint("give the input a file name, which the session is handed as written"),
            );
        }
    }
}

/// Checks a step's declared context list ([`context_entry_defect`], and its placeholders).
fn validated_context(
    context: &[String],
    location: &str,
    errors: &mut ValidationErrors,
) -> Vec<String> {
    let mut kept = Vec::new();
    for entry in context {
        let trimmed = entry.trim();
        if let Some(defect) = context_entry_defect(trimmed) {
            errors.push(ValidationError::new(
                ValidationCode::EmptyDeclaration,
                format!("{location}.context"),
                defect,
            ));
            continue;
        }
        let unknown: Vec<&str> = placeholders_in(trimmed)
            .into_iter()
            .filter(|name| !LlmStep::PLACEHOLDERS.contains(name) && input_named_by(name).is_none())
            .collect();
        if !unknown.is_empty() {
            for name in unknown {
                errors.push(
                    ValidationError::new(
                        ValidationCode::UndeclaredReference,
                        format!("{location}.context"),
                        format!("nothing in a context entry expands `{{{name}}}`"),
                    )
                    .with_hint(
                        "a context entry may name `{run_directory}`, where an earlier command \
                         step's `output:` is kept, and `{input.<name>}`",
                    ),
                );
            }
            continue;
        }
        kept.push(trimmed.to_owned());
    }
    kept
}

/// Checks an `llm` step's judgement: present when the map's format requires one, and never blank.
///
/// Refused at `<step>.judgement` with a stable code: `missing_declaration` when the key is absent
/// from an `aep.driver-steps/2` map, `empty_declaration` when it says nothing, in either version.
/// An `aep.driver-steps/1` map adopted no such rule, so its step may omit the key.
///
/// `None` is a refusal; `Some(None)` is a `/1` step that wrote no judgement.
#[allow(clippy::option_option)] // Refused, absent and present are three different answers.
fn validated_judgement(
    judgement: Option<&str>,
    format: StepMapFormat,
    location: &str,
    errors: &mut ValidationErrors,
) -> Option<Option<String>> {
    match judgement.map(str::trim) {
        None if !format.requires_judgement() => Some(None),
        None => {
            errors.push(
                ValidationError::new(
                    ValidationCode::MissingDeclaration,
                    format!("{location}.judgement"),
                    "an llm step says what it decides that no command could, and this one does \
                     not",
                )
                .with_hint(
                    "write `judgement:` with one line naming the decision only a model can make; a \
                     step that has none is a candidate for a `command` step",
                ),
            );
            None
        }
        Some("") => {
            errors.push(ValidationError::new(
                ValidationCode::EmptyDeclaration,
                format!("{location}.judgement"),
                "a blank judgement says nothing about why this step needs a model",
            ));
            None
        }
        Some(line) => Some(Some(line.to_owned())),
    }
}

/// Checks a step's write scope.
///
/// Two rules, and the second is the one that matters: the **last rule must be a catch-all**. A path
/// nobody mentioned has to have an answer, and leaving it to a default is how a scope silently
/// stops covering the tree it was written for.
fn validated_scope(
    scope: &[RawScopeRule],
    location: &str,
    errors: &mut ValidationErrors,
) -> Option<Vec<ScopeRule>> {
    if scope.is_empty() {
        return Some(Vec::new());
    }
    let mut kept = Vec::new();
    let mut usable = true;
    for rule in scope {
        if rule.paths.is_empty() || rule.paths.iter().any(|path| path.trim().is_empty()) {
            errors.push(ValidationError::new(
                ValidationCode::EmptyDeclaration,
                format!("{location}.scope"),
                "a scope rule with no paths is about nothing",
            ));
            usable = false;
            continue;
        }
        kept.push(ScopeRule {
            paths: rule
                .paths
                .iter()
                .map(|path| path.trim().to_owned())
                .collect(),
            write: rule.write,
        });
    }
    if usable
        && !kept
            .iter()
            .last()
            .is_some_and(|rule| rule.paths.iter().any(|path| path == "**"))
    {
        errors.push(ValidationError::new(
            ValidationCode::EmptyDeclaration,
            format!("{location}.scope"),
            "the last scope rule must name `**`, so every path has an answer. A scope whose tail \
             is silent leaves whatever nobody thought of to a default, which is how one stops \
             covering the tree it was written for.",
        ));
        usable = false;
    }
    usable.then_some(kept)
}

/// Checks the commands a step declares its shell admits.
///
/// Three refusals, each at the prefix's own index so a map with two bad prefixes hears about both
/// (invariant *Validation accumulates*): a prefix with no words names no command; a blank word, or
/// one with whitespace inside it, can never match a word of a split invocation; and a program on
/// [`CommandPrefix::PROGRAM_RUNNERS`] would admit whatever it is handed, which is a general shell
/// declared under another name. Empty is **no declaration**, exactly as [`validated_scope`] reads
/// an empty scope: the step keeps whatever surface the driver holds an undeclared step to.
fn validated_commands(
    commands: &[Vec<String>],
    location: &str,
    errors: &mut ValidationErrors,
) -> Option<Vec<CommandPrefix>> {
    let mut kept = Vec::new();
    let mut usable = true;
    for (index, words) in commands.iter().enumerate() {
        let at = format!("{location}.commands[{index}]");
        if words.is_empty() {
            errors.push(ValidationError::new(
                ValidationCode::EmptyDeclaration,
                at,
                "a command prefix with no words names no command",
            ));
            usable = false;
            continue;
        }
        if words
            .iter()
            .any(|word| word.trim().is_empty() || word.contains(char::is_whitespace))
        {
            errors.push(ValidationError::new(
                ValidationCode::EmptyDeclaration,
                at,
                "every word of a command prefix is one word of an invocation: a blank, or a word \
                 with whitespace inside it, can never match one",
            ));
            usable = false;
            continue;
        }
        let program = words[0].as_str();
        if program.contains('/') {
            errors.push(ValidationError::new(
                ValidationCode::UnsupportedConstruct,
                at,
                format!(
                    "`{program}` is a path, and a prefix names its program by name: the driver \
                     compares an invocation's program by its last path component, so a path \
                     here would promise a location the comparison does not hold."
                ),
            ));
            usable = false;
            continue;
        }
        if CommandPrefix::PROGRAM_RUNNERS.contains(&program) {
            errors.push(ValidationError::new(
                ValidationCode::UnsupportedConstruct,
                at,
                format!(
                    "`{program}` runs a program this prefix does not name, so declaring it \
                     declares a general shell. Name the program itself; the driver runs one \
                     simple invocation of it."
                ),
            ));
            usable = false;
            continue;
        }
        kept.push(CommandPrefix {
            words: words.clone(),
        });
    }
    usable.then_some(kept)
}

/// A `command` step, as written.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawCommandStep {
    /// The program and its arguments.
    #[schemars(length(min = 1))]
    pub run: Vec<String>,
    /// What this step is for, in one line.
    #[serde(default)]
    pub description: Option<String>,
    /// What to record when the program produces a verdict.
    #[serde(default)]
    pub evidence: Option<RawEvidenceMapping>,
    /// How many times to retry a step that could not run at all.
    #[serde(default)]
    pub retries: Option<u32>,
    /// The outside thing this step needs, if it needs one.
    #[serde(default)]
    #[schemars(regex(pattern = r"\S"))]
    pub depends_on: Option<String>,
    /// After how many failures of `depends_on` to stop attempting it in this run.
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub circuit_breaker: Option<u32>,
    /// Which exit status is the passing verdict. `success` when the document is silent.
    #[serde(default)]
    pub expect: Option<Expect>,
    /// A file name in the run directory the program's standard output is written to.
    #[serde(default)]
    #[schemars(regex(pattern = r"^[^./\\][^/\\]*$"))]
    pub output: Option<String>,
}

/// Which exit status of a `command` step is the passing verdict.
///
/// `failure` is how a bug fix names its reproducing command: the command is expected to fail
/// before the fix, so a nonzero exit is what *reproduced* means, and an exit of zero is a failing
/// verdict whose reason is [`NOTHING_REPRODUCED`]. A process that did not finish is still no
/// verdict at all, whatever the step expects.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    /// Exit `0` is the pass, as every step before the key existed read it.
    #[default]
    Success,
    /// Any exit but `0` is the pass; exit `0` reproduced nothing.
    Failure,
}

impl Expect {
    /// `true` for the default, which is not serialized, so a map that does not write the key
    /// digests exactly as it did before the key existed.
    #[allow(clippy::trivially_copy_pass_by_ref)] // serde's `skip_serializing_if` passes a reference
    pub fn is_success(&self) -> bool {
        *self == Self::Success
    }
}

/// The reason a step expecting failure gives when its command exited `0`.
pub const NOTHING_REPRODUCED: &str = "nothing reproduced";

/// What a `command` step's exit status says, read against what the step expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandVerdict {
    /// Whether the verdict is a pass.
    pub passed: bool,
    /// Why the verdict is what it is, when the exit status alone does not say:
    /// [`NOTHING_REPRODUCED`] for a step expecting failure whose command exited `0`.
    pub reason: Option<&'static str>,
}

/// Whether `name` may name a run input: a lowercase letter, then lowercase letters, digits, `_`
/// and `-`.
///
/// The same rule `{input.<name>}` is recognised by, so every input `--input` accepts is one a map
/// can name, and a name a map writes outside it is ordinary text rather than a placeholder.
pub fn valid_input_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// The prefix of a placeholder naming a run input, `{input.<name>}`.
pub const INPUT_PLACEHOLDER: &str = "input.";

/// The input a placeholder names, when it names one.
pub fn input_named_by(placeholder: &str) -> Option<&str> {
    placeholder.strip_prefix(INPUT_PLACEHOLDER)
}

/// Fills every placeholder of `word` in one pass, left to right.
///
/// One pass, so a value that was filled in is never read again for placeholders: a value is text.
/// `value_of` answers a placeholder's name with its value, or with `Ok(None)` to leave it as it was
/// written for a later reader, or with the reason it cannot be filled.
pub fn substitute(
    word: &str,
    mut value_of: impl FnMut(&str) -> Result<Option<String>, String>,
) -> Result<String, String> {
    let mut filled = String::with_capacity(word.len());
    let mut rest = word;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        let name = &after[..close];
        filled.push_str(&rest[..open]);
        if let Some(Some(value)) = is_placeholder(name).then(|| value_of(name)).transpose()? {
            filled.push_str(&value);
        } else {
            filled.push('{');
            filled.push_str(name);
            filled.push('}');
        }
        rest = &after[close + 1..];
    }
    filled.push_str(rest);
    Ok(filled)
}

/// One word of a [`CommandStep`] — a `run:` word or the evidence `record:` path — read once into
/// its text and its placeholders.
///
/// **Why the placeholders are kept apart rather than found again in the text each time.** A step's
/// inputs are filled in when its run starts ([`CommandStep::with_inputs`]) and the rest of its
/// placeholders when it is spawned. Were the second pass to read the text the first one produced,
/// a value would be read again for placeholders: `--input a={` and `--input b=task}` would spell
/// `{task}` between them and hand the step the task document's path, and a value with an
/// unclosed `{` would swallow the map's own `{run_directory}` after it. Here a filled value is a
/// text part, and only a part the map wrote can be a placeholder, so every pass fills the
/// placeholders the map wrote and nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// The word as it reads: each text part as it is and each placeholder as `{<name>}`.
    text: String,
    /// The word in order, text and placeholders.
    parts: Vec<WordPart>,
}

/// One part of a [`Word`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum WordPart {
    /// Text, which no pass reads for placeholders.
    Text(String),
    /// A placeholder the map wrote, by its name (`task`, `input.repro`).
    Placeholder(String),
}

impl Word {
    /// Reads a word as a map writes it, finding its placeholders as [`placeholders_in`] does.
    pub fn parse(word: &str) -> Self {
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut rest = word;
        while let Some(open) = rest.find('{') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('}') else {
                break;
            };
            let name = &after[..close];
            text.push_str(&rest[..open]);
            if is_placeholder(name) {
                if !text.is_empty() {
                    parts.push(WordPart::Text(std::mem::take(&mut text)));
                }
                parts.push(WordPart::Placeholder(name.to_owned()));
            } else {
                text.push('{');
                text.push_str(name);
                text.push('}');
            }
            rest = &after[close + 1..];
        }
        text.push_str(rest);
        if !text.is_empty() {
            parts.push(WordPart::Text(text));
        }
        Self::from_parts(parts)
    }

    fn from_parts(parts: Vec<WordPart>) -> Self {
        let mut text = String::new();
        for part in &parts {
            match part {
                WordPart::Text(value) => text.push_str(value),
                WordPart::Placeholder(name) => {
                    text.push('{');
                    text.push_str(name);
                    text.push('}');
                }
            }
        }
        Self { text, parts }
    }

    /// The word as it reads, with each placeholder not yet filled as `{<name>}`.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The placeholders the map wrote in this word and no pass has filled, in order.
    pub fn placeholders(&self) -> impl Iterator<Item = &str> {
        self.parts.iter().filter_map(|part| match part {
            WordPart::Placeholder(name) => Some(name.as_str()),
            WordPart::Text(_) => None,
        })
    }

    /// This word with its placeholders filled in, in one pass.
    ///
    /// `value_of` answers a placeholder's name with its value, `Ok(None)` to leave it for a later
    /// pass, or the reason it cannot be filled. A value becomes text, which no later pass reads
    /// for placeholders.
    ///
    /// # Errors
    ///
    /// The first reason `value_of` gives.
    pub fn fill(
        &self,
        mut value_of: impl FnMut(&str) -> Result<Option<String>, String>,
    ) -> Result<Self, String> {
        let mut parts = Vec::with_capacity(self.parts.len());
        for part in &self.parts {
            match part {
                WordPart::Placeholder(name) => match value_of(name)? {
                    Some(value) => parts.push(WordPart::Text(value)),
                    None => parts.push(part.clone()),
                },
                WordPart::Text(_) => parts.push(part.clone()),
            }
        }
        Ok(Self::from_parts(parts))
    }
}

impl From<&str> for Word {
    fn from(word: &str) -> Self {
        Self::parse(word)
    }
}

impl From<&Word> for Word {
    fn from(word: &Word) -> Self {
        word.clone()
    }
}

impl fmt::Display for Word {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl PartialEq<str> for Word {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl PartialEq<&str> for Word {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

impl PartialEq<String> for Word {
    fn eq(&self, other: &String) -> bool {
        self.text == *other
    }
}

/// Serialized as the word reads, so a map digests exactly as it did when its words were strings.
impl serde::Serialize for Word {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.text)
    }
}

/// Whether the text between a `{` and the next `}` has the shape of a placeholder.
///
/// A word of `a-z` and `_`, or one such word, a `.` and a word an input name may be. Deliberately
/// narrow, so `{}` and a jq program's `{a: .b}` stay text.
fn is_placeholder(name: &str) -> bool {
    let word =
        |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    match name.split_once('.') {
        None => word(name),
        Some((head, tail)) => word(head) && valid_input_name(tail),
    }
}

/// An `llm` step, as written.
///
/// It has no `evidence` key and cannot be given one: see the module documentation.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawLlmStep {
    /// What the model is asked to do.
    #[schemars(regex(pattern = r"\S"))]
    pub prompt: String,
    /// Files the run is given before it starts, instead of discovering them.
    ///
    /// A list and never a glob: a glob is a promise about a directory's future size, and a step map
    /// is pinned. A named file that is absent at run time refuses the run — a run given a smaller
    /// context than its map declares is one nobody can reproduce from the map.
    ///
    /// Not free. A stateless harness replays its conversation every turn, so a preloaded file is
    /// paid for on every turn rather than once. It is still usually a saving, because what it
    /// replaces — a discovery read — is a call, a turn, *and* a result that joins the same replay.
    #[serde(default)]
    #[schemars(inner(regex(pattern = r"\S")))]
    pub context: Vec<String>,
    /// Where this step may write, in order. First match wins.
    #[serde(default)]
    pub scope: Vec<RawScopeRule>,
    /// The commands this step's shell admits, each an argv prefix with the program first.
    ///
    /// `[[aep, plan, artifact]]` admits one simple invocation of `aep plan artifact …` and nothing
    /// else; `[[task], [cargo, test]]` admits the repository's tasks and `cargo test`. Absent
    /// means the step declares nothing and is held to the driver's default surface.
    #[serde(default)]
    pub commands: Vec<Vec<String>>,
    /// Skills the harness should make available, by name.
    #[serde(default)]
    pub skills: Vec<String>,
    /// Which harness runs it. `claude-code` when the document is silent.
    #[serde(default)]
    pub harness: Option<String>,
    /// What this step is for, in one line.
    #[serde(default)]
    pub description: Option<String>,
    /// What this step decides that no command could, in one line.
    ///
    /// Required in an `aep.driver-steps/2` map and optional in an `aep.driver-steps/1` one, so it
    /// is read as optional: a `/2` step without one reaches validation and is refused there with
    /// its path, accumulating with every other problem in the map, rather than failing the whole
    /// document as a deserialization error would. The published schema requires it for `/2` at
    /// the map's level, where the format version is.
    #[serde(default)]
    #[schemars(regex(pattern = r"\S"))]
    pub judgement: Option<String>,
}

/// An `operator` step, as written.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawOperatorStep {
    /// What the person is being asked for.
    #[schemars(regex(pattern = r"\S"))]
    pub prompt: String,
    /// What this step is for, in one line.
    #[serde(default)]
    pub description: Option<String>,
}

/// How a command's result becomes evidence, as written.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawEvidenceMapping {
    /// Which kind of evidence the program establishes.
    pub kind: EvidenceKind,
    /// Which class of verifier it is.
    pub verifier: Verifier,
    /// Which suite ran, for a `test_result`.
    #[serde(default)]
    pub suite: Option<TestSuite>,
    /// What the evidence is about.
    #[serde(default)]
    pub subject: Option<SubjectRef>,
    /// The tool that produced it, for provenance.
    #[serde(default)]
    pub tool: Option<ToolRef>,
    /// Where the program writes the evidence record, when it writes one itself.
    #[serde(default)]
    #[schemars(regex(pattern = r"\S"))]
    pub record: Option<String>,
}

/// Which sort of step this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// Runs a program.
    Command,
    /// Asks a model.
    Llm,
    /// Stops for a person.
    Operator,
}

impl StepKind {
    /// The kind as written in documents and reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Llm => "llm",
            Self::Operator => "operator",
        }
    }
}

impl fmt::Display for StepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A validated step.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    /// Run a program and map its result to evidence.
    Command(CommandStep),
    /// Ask a model, with a tool set the protocol derived.
    Llm(LlmStep),
    /// Stop and hand the run to a person.
    Operator(OperatorStep),
}

impl Step {
    /// Which sort of step this is.
    pub fn kind(&self) -> StepKind {
        match self {
            Self::Command(_) => StepKind::Command,
            Self::Llm(_) => StepKind::Llm,
            Self::Operator(_) => StepKind::Operator,
        }
    }

    /// How many times this step may be retried after producing no verdict at all.
    ///
    /// Per step kind, as decision D5 takes it: a `command` step retries because a process died, an
    /// `llm` step retries once because a model call errored, and an `operator` step never retries
    /// because a person is not a flaky dependency.
    pub fn retry_budget(&self) -> u32 {
        match self {
            Self::Command(step) => step.retries,
            Self::Llm(_) => LLM_RETRIES,
            Self::Operator(_) => 0,
        }
    }

    /// The run inputs this step names, once each, in the order it first names them.
    pub fn inputs_named(&self) -> Vec<String> {
        match self {
            Self::Command(step) => step.inputs_named(),
            Self::Llm(step) => step.inputs_named(),
            Self::Operator(_) => Vec::new(),
        }
    }

    /// One line naming the step, for a report.
    pub fn label(&self) -> String {
        match self {
            Self::Command(step) => step
                .description
                .clone()
                .unwrap_or_else(|| step.command_line()),
            Self::Llm(step) => step
                .description
                .clone()
                .unwrap_or_else(|| step.prompt.clone()),
            Self::Operator(step) => step
                .description
                .clone()
                .unwrap_or_else(|| step.prompt.clone()),
        }
    }
}

/// A validated `command` step.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CommandStep {
    /// The program and its arguments, never empty, each read once into its placeholders.
    pub run: Vec<Word>,
    /// What this step is for, in one line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What to record when the program produces a verdict.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<EvidenceMapping>,
    /// How many times to retry a step that could not run at all.
    pub retries: u32,
    /// The outside thing this step needs, if it needs one.
    ///
    /// A name, not an address: `github-api`, `staging-cluster`. It is what a circuit breaker is
    /// keyed by, so two steps that call the same dependency share its fate — which is the whole
    /// difference between a circuit breaker and a retry budget. Retry bounds *this step keeps
    /// crashing*; a breaker bounds *this dependency keeps failing*, across every step that needs
    /// it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<String>,
    /// After how many failures of [`Self::depends_on`] to stop attempting it in this run.
    ///
    /// `None` is no breaker, which is what every map written before this had. When set, the run
    /// stops *attempting* the dependency once the count is reached: later steps naming it are
    /// skipped and recorded as skipped, rather than run and failed.
    ///
    /// Skipped and **not** failed, because a step that was never attempted produced no observation,
    /// and recording a failure for it would fabricate one — the same rule D5 already applies to a
    /// step that crashed. The difference a reader needs is between *we looked and it was broken*
    /// and *we stopped looking*, and only the second is what an open circuit means.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<u32>,
    /// Which exit status is the passing verdict; see [`Self::verdict`].
    ///
    /// Not serialized when it is the default, so a map that does not write `expect:` digests as
    /// it did before the key existed and a run pinned to that digest still resumes.
    #[serde(skip_serializing_if = "Expect::is_success")]
    pub expect: Expect,
    /// A file name in the run directory the program's standard output is written to, so a later
    /// step names it as `{run_directory}/<file>` — an `llm` step's context included.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl CommandStep {
    /// The names the run directory already uses for its own records, which an `output:` would
    /// overwrite. A name starting with `.` is refused besides these.
    pub const RESERVED_OUTPUTS: &'static [&'static str] = &[
        "cursor.json",
        "cursor.writing",
        "snapshot.json",
        "snapshot.writing",
        "state.json",
        "state.writing",
        "launch.json",
        "launch.json.next",
        "commands.jsonl",
        "generations",
        "transcripts",
    ];

    /// The names of a step's own log in the run directory, `<state>-<index>-<attempt>.log`, which
    /// an `output:` would overwrite: any text, `-`, digits, `-`, digits and `.log`, in any ASCII
    /// case. Narrower than every name ending in `.log`, so `notes.log` stays an output a map may
    /// keep.
    pub fn is_step_log_name(name: &str) -> bool {
        let lowered = name.to_ascii_lowercase();
        let Some(stem) = lowered.strip_suffix(".log") else {
            return false;
        };
        let mut pieces = stem.rsplitn(3, '-');
        let digits = |piece: Option<&str>| {
            piece
                .is_some_and(|piece| !piece.is_empty() && piece.bytes().all(|b| b.is_ascii_digit()))
        };
        digits(pieces.next())
            && digits(pieces.next())
            && pieces.next().is_some_and(|state| !state.is_empty())
    }

    /// What an exit status says about this step, read against [`Self::expect`].
    ///
    /// A process killed before it exited has no exit status and is no verdict at all; that is the
    /// caller's to decide before it asks.
    ///
    /// # Errors
    ///
    /// No verdict, with the reason naming the status, when the step expects failure and the
    /// command exited 126 or 127: a shell's "could not execute" and "not found". A reproduction
    /// spelt `sh -c '<command>'` whose command is missing fails that way, and nothing was
    /// reproduced, as for a program the driver could not spawn itself. Any other nonzero exit, 128
    /// and above included, is still a reproduction: a crash reproduced through a shell exits that
    /// way, and it cannot be told apart from a signal the shell reports. Under `expect: success`
    /// both are a failing verdict, as every exit but 0 was before the key existed.
    pub fn verdict(&self, exit_code: i32) -> Result<CommandVerdict, String> {
        match (self.expect, exit_code) {
            (Expect::Success, code) => Ok(CommandVerdict {
                passed: code == 0,
                reason: None,
            }),
            (Expect::Failure, 0) => Ok(CommandVerdict {
                passed: false,
                reason: Some(NOTHING_REPRODUCED),
            }),
            (Expect::Failure, code @ (126 | 127)) => Err(format!(
                "exit status {code} is a shell's \"{}\", so the command was never run and \
                 nothing was reproduced",
                if code == 126 {
                    "could not execute"
                } else {
                    "command not found"
                }
            )),
            (Expect::Failure, _) => Ok(CommandVerdict {
                passed: true,
                reason: None,
            }),
        }
    }

    /// The run inputs this step names, once each, in the order it first names them.
    pub fn inputs_named(&self) -> Vec<String> {
        inputs_in(self.expandable().flat_map(Word::placeholders))
    }

    /// This step with every `{input.<name>}` filled in from `inputs`, in one pass.
    ///
    /// The other placeholders are left as written: where the run directory, the task document and
    /// the preceding transcript are is the executor's to say. An input value becomes text of its
    /// [`Word`], which no later pass reads for placeholders, and [`StepMap::check_inputs`] refuses
    /// one that names any besides.
    ///
    /// # Errors
    ///
    /// The reason, naming the placeholder, when the step names an input `inputs` does not hold.
    pub fn with_inputs(&self, inputs: &BTreeMap<String, String>) -> Result<Self, String> {
        let fill = |word: &Word| word.fill(|name| input_value(name, inputs));
        let mut filled = self.clone();
        filled.run = self.run.iter().map(fill).collect::<Result<_, _>>()?;
        if let Some(mapping) = filled.evidence.as_mut() {
            mapping.record = mapping.record.as_ref().map(fill).transpose()?;
        }
        Ok(filled)
    }

    /// The names a `{placeholder}` in [`Self::run`] or in `evidence.record` may have.
    ///
    /// Three, and all three are things only the run knows: where its directory is, which task
    /// document it was started from, and which transcript the `llm` step before this one wrote. A
    /// map is a document in the repository and a run directory is allocated when the run starts,
    /// so a step that has to name one has no way to write it down — which is why
    /// `aep observe trace check` could not be a step of a map before these existed.
    ///
    /// **`{task}` is the third, and it is here because a verb that binds to *this* run's task had
    /// no way to be told which one.** `aep observe specification evidence` selects the specification
    /// of the work the task declares; with no way to name the task in a map, a run driven with
    /// `aep drive run --task <a path that is not the project's>` reached that verb through
    /// discovery and bound to the *project's* task instead — writing a record about the wrong
    /// story, or refusing over the wrong one. The driver expands it to the **absolute** path of
    /// the task document the run was started from, because a `command` step runs in the project
    /// directory and a path typed on a command line is relative to wherever it was typed.
    ///
    /// The list is closed and an unknown name is refused at load, because the alternative is a
    /// command line containing the literal characters `{transcirpt}` and a checker reporting that
    /// it cannot read that file, halfway through a run.
    pub const PLACEHOLDERS: &'static [&'static str] = &["run_directory", "task", "transcript"];

    /// The program, which a validated step always has.
    pub fn program(&self) -> &str {
        self.run.first().map_or("", Word::as_str)
    }

    /// The arguments after the program.
    pub fn arguments(&self) -> &[Word] {
        self.run.get(1..).unwrap_or_default()
    }

    /// The step's words as they read, joined by spaces, for a report.
    pub fn command_line(&self) -> String {
        self.run
            .iter()
            .map(Word::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Every word of this step that a placeholder may appear in.
    pub fn expandable(&self) -> impl Iterator<Item = &Word> {
        self.run.iter().chain(
            self.evidence
                .as_ref()
                .and_then(|mapping| mapping.record.as_ref()),
        )
    }
}

/// Every `{placeholder}` in `word`, in the order they appear.
///
/// A placeholder is `{` then one or more of `a-z` and `_` then `}` — deliberately narrow, so that
/// the argument `{}` that `find -exec` takes, and a jq program's `{a: .b}`, are ordinary text
/// rather than a refusal — or such a word, a `.` and an input name, as `{input.repro}` is.
/// Anything that *does* match the shape is checked against [`CommandStep::PLACEHOLDERS`] and the
/// `input.` prefix, so a misspelling is refused rather than passed to a program as literal braces.
pub fn placeholders_in(word: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = word;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        let name = &after[..close];
        if is_placeholder(name) {
            found.push(name);
        }
        rest = &after[close + 1..];
    }
    found
}

/// Each run input `placeholders` name, once, in the order they first name it.
fn inputs_in<'a>(placeholders: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut named: Vec<String> = Vec::new();
    for name in placeholders.filter_map(input_named_by) {
        if !named.iter().any(|seen| seen == name) {
            named.push(name.to_owned());
        }
    }
    named
}

/// The value `inputs` gives a placeholder naming a run input; `None` for any other placeholder.
fn input_value(name: &str, inputs: &BTreeMap<String, String>) -> Result<Option<String>, String> {
    let Some(input) = input_named_by(name) else {
        return Ok(None);
    };
    inputs.get(input).cloned().map(Some).ok_or_else(|| {
        format!(
            "`{{{name}}}` names an input this run was not given; pass `--input {input}=<value>`"
        )
    })
}

/// A validated `llm` step.
///
/// There is no `evidence` field, and that absence is the mechanism rather than an omission.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LlmStep {
    /// What the model is asked to do.
    pub prompt: String,
    /// Files the run is given before it starts. See [`RawLlmStep::context`].
    pub context: Vec<String>,
    /// Where this step may write, in order. **First match wins**, and the last rule is a
    /// catch-all — validation refuses a scope without one, because a path nobody mentioned must
    /// have an answer and leaving it to a default is how a scope stops covering its own tree.
    ///
    /// Empty means no scope was declared, which is not the same as a scope that allows
    /// everything: nothing is enforced and nothing claims to be.
    pub scope: Vec<ScopeRule>,
    /// The commands this step's shell admits, in the order the map wrote them.
    ///
    /// Empty means no command was declared, and the driver holds the step to the surface it
    /// holds every undeclared step to. A declared list replaces that surface's commands: an
    /// invocation that matches none of its prefixes is refused by name, whatever the undeclared
    /// surface admits. The read-only programs a state that admits `repository.read` may run stay
    /// admitted beside a declaration; they are the state's, not the step's.
    ///
    /// Skipped when empty, so a map that declares nothing keeps the digest it had before this key
    /// existed and a run under it resumes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<CommandPrefix>,
    /// Skills the harness makes available, by name.
    pub skills: Vec<String>,
    /// Which harness runs it.
    ///
    /// The selection seam § 4.9 point 3 names: a second adapter is a second free function chosen by
    /// this name, not a trait added before there is a second implementation to design it against.
    pub harness: String,
    /// What this step is for, in one line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What this step decides that no command could, in one line; never blank.
    ///
    /// Always present in an `aep.driver-steps/2` map, because a model is the expensive,
    /// unrepeatable way to decide anything: a step that cannot say why it needs one is a candidate
    /// for a `command` step. `None` only in an `aep.driver-steps/1` map, which predates the rule.
    /// Skipped when absent, so a `/1` map's digest is the one 0.71.x computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub judgement: Option<String>,
}

impl LlmStep {
    /// The harness a step runs under when the document does not say.
    pub const DEFAULT_HARNESS: &'static str = "claude-code";

    /// The names a `{placeholder}` in [`Self::context`] may have, besides `{input.<name>}`.
    ///
    /// Only the run directory: a context entry is a file the session is given, and the run
    /// directory is where an earlier `command` step's `output:` is kept. The task document and a
    /// transcript are another session's or the run's own, and are not context.
    pub const PLACEHOLDERS: &'static [&'static str] = &["run_directory"];

    /// The run inputs this step's context names, once each, in the order it first names them.
    pub fn inputs_named(&self) -> Vec<String> {
        inputs_in(self.context.iter().flat_map(|entry| placeholders_in(entry)))
    }

    /// This step with its context entries' placeholders filled in, in one pass: `{run_directory}`
    /// with `run_directory` and `{input.<name>}` with `inputs`.
    ///
    /// # Errors
    ///
    /// The reason, naming the placeholder, when the context names an input `inputs` does not
    /// hold.
    pub fn with_context_resolved(
        &self,
        run_directory: &str,
        inputs: &BTreeMap<String, String>,
    ) -> Result<Self, String> {
        let mut resolved = self.clone();
        resolved.context = self
            .context
            .iter()
            .map(|entry| {
                substitute(entry, |name| match name {
                    "run_directory" => Ok(Some(run_directory.to_owned())),
                    other => input_value(other, inputs),
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(resolved)
    }
}

/// A validated `operator` step.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct OperatorStep {
    /// What the person is being asked for.
    pub prompt: String,
    /// What this step is for, in one line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A validated evidence mapping.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EvidenceMapping {
    /// Which kind of evidence the program establishes.
    pub kind: EvidenceKind,
    /// Which class of verifier it is.
    pub verifier: Verifier,
    /// Which suite ran, for a `test_result`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite: Option<TestSuite>,
    /// What the evidence is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<SubjectRef>,
    /// The tool that produced it, for provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolRef>,
    /// Where the program writes the evidence record, when it writes one itself.
    ///
    /// Without it a `command` step's evidence is minted from the exit status, which is why
    /// [`Self::MINTABLE`] is short: an exit status carries *did it produce a verdict, and was it
    /// yes or no*, and a kind whose record holds digests and counts cannot be built from that
    /// without inventing them. With it the driver mints nothing — it reads the document the
    /// verifier wrote, exactly as `aep govern evaluate --evidence` reads one, and submits what the
    /// document says. `aep observe trace evidence` is the case it was added for: the record carries
    /// the specification's digest, the transcript's digest and three counts, all of which are
    /// facts about a check this process did not run.
    ///
    /// The path admits the same placeholders [`CommandStep::run`] does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<Word>,
}

impl EvidenceMapping {
    /// The evidence kinds a driver can mint from a program's exit status.
    ///
    /// A closed set, and short on purpose. Everything here is establishable from *did the verifier
    /// produce a verdict, and was it yes or no* — which is all an exit status carries. A kind whose
    /// payload needs numbers nobody read out of the output (a metric, a deployment, an approval)
    /// is refused at load rather than minted with invented values, because a fabricated count is
    /// worse than a missing record: the engine cannot tell it apart from an observed one.
    pub const MINTABLE: &'static [EvidenceKind] = &[
        EvidenceKind::TestResult,
        EvidenceKind::StaticAnalysis,
        EvidenceKind::ContractResult,
        EvidenceKind::Diff,
    ];
}

/// One state's steps, validated.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StateSteps {
    /// How many times this state may be entered before the run stops.
    pub visit_budget: u32,
    /// Whether the state's consecutive `command` steps run at once ([`StepMap::batch_at`]).
    ///
    /// Not serialized when `false`, so a map that does not write the key digests exactly as it
    /// did before the key existed, and a run started by an earlier build still resumes.
    #[serde(skip_serializing_if = "is_false")]
    pub concurrent: bool,
    /// The steps, in the order the author wrote them.
    pub steps: Vec<Step>,
}

/// `serde`'s `skip_serializing_if` for a `bool` that is only written when set.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !*value
}

/// A step-map format version this build reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StepMapFormat {
    /// `aep.driver-steps/1`, which every protocol tree before 0.72.0 ships: an `llm` step's
    /// judgement is optional, and one it writes must not be blank.
    V1,
    /// `aep.driver-steps/2`: every `llm` step carries a judgement.
    V2,
}

impl StepMapFormat {
    /// The version a document spells, or `None` for one this build does not read.
    pub fn parse(format: &str) -> Option<Self> {
        match format {
            STEP_MAP_FORMAT_V1 => Some(Self::V1),
            STEP_MAP_FORMAT => Some(Self::V2),
            _ => None,
        }
    }

    /// The version a document spells, refusing one this build does not read.
    ///
    /// A refused version still answers the newest, so the document's steps are checked under the
    /// strictest rules and every other problem of it is reported beside the refusal.
    fn validated(format: &str, errors: &mut ValidationErrors) -> Self {
        Self::parse(format).unwrap_or_else(|| {
            errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedFormatVersion,
                    "driver-steps.format",
                    format!(
                        "this build reads `{STEP_MAP_FORMAT}` and `{STEP_MAP_FORMAT_V1}`, and the \
                         document is written in `{format}`"
                    ),
                )
                .with_hint(
                    "a step map from a newer driver may name step kinds this one cannot run, and \
                     running the rest would be a run that silently skipped them",
                ),
            );
            Self::V2
        })
    }

    /// The version as a document spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V1 => STEP_MAP_FORMAT_V1,
            Self::V2 => STEP_MAP_FORMAT,
        }
    }

    /// Whether an `llm` step of a map in this version must carry a judgement.
    pub fn requires_judgement(self) -> bool {
        self == Self::V2
    }
}

/// A validated step map.
///
/// Obtained only through [`TryFrom<RawStepMap>`], which is what makes possession of one the
/// evidence that its format version, its pin and its steps were checked.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StepMap {
    /// The format version this map is written in, which decides whether its `llm` steps must
    /// carry a judgement.
    ///
    /// Not serialized, so not in [`Self::digest`]: a `/1` map digests exactly as 0.71.x computed
    /// it, and a run that build started resumes under this one.
    #[serde(skip)]
    pub format: StepMapFormat,
    /// What this map is.
    pub id: StepMapId,
    /// The workflow it is written against, pinned to a major version.
    pub workflow: PinnedWorkflowRef,
    /// A human sentence for a report's heading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What to do in each state.
    pub states: BTreeMap<StateId, StateSteps>,
}

impl StepMap {
    /// The format version this map is written in, as the document spells it.
    pub fn format(&self) -> &'static str {
        self.format.as_str()
    }

    /// The steps of one state, or nothing when the map is silent about it.
    ///
    /// A state with no steps is not an error: a workflow state whose transition is unguarded needs
    /// no work done in it, and a map that had to say so for every such state would be noise.
    pub fn steps_for(&self, state: &StateId) -> &[Step] {
        self.states
            .get(state)
            .map_or(&[], |entry| entry.steps.as_slice())
    }

    /// The steps that run together starting at `index` of `state`: the batch, as a range of
    /// indices into [`Self::steps_for`].
    ///
    /// In a `concurrent: true` state, a `command` step at `index` runs with every consecutive
    /// `command` step after it, up to the next `llm` or `operator` step or the end of the list. In
    /// any other case the batch is the one step at `index`, and past the end of the list it is
    /// empty. From `index` rather than from the batch's own start, because the cursor's pointer is
    /// where a run is, and a batch is what remains of the run of commands from there.
    pub fn batch_at(&self, state: &StateId, index: usize) -> std::ops::Range<usize> {
        let steps = self.steps_for(state);
        if index >= steps.len() {
            return index..index;
        }
        let concurrent = self.states.get(state).is_some_and(|entry| entry.concurrent);
        if !concurrent || !matches!(steps[index], Step::Command(_)) {
            return index..index + 1;
        }
        let end = steps[index..]
            .iter()
            .position(|step| !matches!(step, Step::Command(_)))
            .map_or(steps.len(), |offset| index + offset);
        index..end
    }

    /// How many times `state` may be entered before the run stops.
    pub fn visit_budget(&self, state: &StateId) -> u32 {
        self.states
            .get(state)
            .map_or(DEFAULT_VISIT_BUDGET, |entry| entry.visit_budget)
    }

    /// How many `llm` steps this map declares, over every state.
    pub fn llm_steps_declared(&self) -> u32 {
        let count = self
            .states
            .values()
            .flat_map(|entry| entry.steps.iter())
            .filter(|step| step.kind() == StepKind::Llm)
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    /// Every evidence kind any step of this map declares.
    pub fn declared_evidence_kinds(&self) -> BTreeSet<EvidenceKind> {
        self.states
            .values()
            .flat_map(|entry| entry.steps.iter())
            .filter_map(|step| match step {
                Step::Command(command) => command.evidence.as_ref().map(|mapping| mapping.kind),
                Step::Llm(_) | Step::Operator(_) => None,
            })
            .collect()
    }

    /// Every run input any step of this map names.
    pub fn inputs_named(&self) -> BTreeSet<String> {
        self.states
            .values()
            .flat_map(|entry| entry.steps.iter())
            .flat_map(Step::inputs_named)
            .collect()
    }

    /// The map against the inputs a run was given, before any step runs.
    ///
    /// Three refusals, accumulated over every step: an `{input.<name>}` the run was not given, at
    /// the step that names it with code `undeclared_reference`; a given value that would itself
    /// read as a placeholder, with code `unsupported_construct` — a value is filled in as text, and
    /// one that named `{task}` would hand a step a document the map never named; and a value that
    /// leaves an `llm` step's context entry blank or a glob once filled in, at that step's
    /// `context` with code `empty_declaration`, the refusal the loader gives the same entry when a
    /// map writes it. An input given and never named is not refused: a release map and a bug-fix
    /// map can share one launch line.
    pub fn check_inputs(&self, given: &BTreeMap<String, String>) -> ValidationErrors {
        let mut errors = ValidationErrors::new();
        for (name, value) in given {
            let named = placeholders_in(value);
            if let Some(placeholder) = named.first() {
                errors.push(ValidationError::new(
                    ValidationCode::UnsupportedConstruct,
                    format!("driver-steps[{}].inputs.{name}", self.id),
                    format!(
                        "the value of input `{name}` names `{{{placeholder}}}`, and an input's \
                         value is filled in as text, never read again for placeholders"
                    ),
                ));
            }
        }
        for (state, entry) in &self.states {
            for (index, step) in entry.steps.iter().enumerate() {
                let field = if matches!(step, Step::Llm(_)) {
                    "context"
                } else {
                    "run"
                };
                for name in step.inputs_named() {
                    if given.contains_key(&name) {
                        continue;
                    }
                    errors.push(
                        ValidationError::new(
                            ValidationCode::UndeclaredReference,
                            format!(
                                "driver-steps[{}].states.{state}.steps[{index}].{field}",
                                self.id
                            ),
                            format!("`{{input.{name}}}` names an input this run was not given"),
                        )
                        .with_hint(format!("pass `--input {name}=<value>`")),
                    );
                }
                if let Step::Llm(llm) = step {
                    let at = format!(
                        "driver-steps[{}].states.{state}.steps[{index}].context",
                        self.id
                    );
                    check_filled_context(&llm.context, given, &at, &mut errors);
                }
            }
        }
        errors
    }

    /// The digest of this map's canonical JSON, which the cursor pins a run to.
    pub fn digest(&self) -> String {
        digest_of_canonical(self)
    }

    /// Phase one: the map against the workflow it names, at load time.
    ///
    /// Accumulating, and it does not stop at the first bad state: a map with four renamed states
    /// reports four, because fixing a document one error per run is how a validation step becomes
    /// something people avoid running.
    pub fn cross_validate(&self, workflow: &Workflow) -> ValidationErrors {
        let mut errors = ValidationErrors::new();

        if workflow.id != *self.workflow.id() {
            errors.push(ValidationError::new(
                ValidationCode::UnknownWorkflow,
                format!("driver-steps[{}].workflow", self.id),
                format!(
                    "the map is written against `{}` and was checked against `{}`",
                    self.workflow, workflow.id
                ),
            ));
        } else if !self.workflow.accepts(workflow.version) {
            errors.push(
                ValidationError::new(
                    ValidationCode::VersionMismatch,
                    format!("driver-steps[{}].workflow", self.id),
                    format!(
                        "the map pins `{}` and the workflow in the tree is at version {}",
                        self.workflow, workflow.version
                    ),
                )
                .with_hint(
                    "a major version exists because the change could not be expressed additively, \
                     so the map is rewritten against the new state graph rather than migrated",
                ),
            );
        }

        let known: Vec<&str> = workflow.states.keys().map(StateId::as_str).collect();
        for state in self.states.keys() {
            if !workflow.states.contains_key(state) {
                errors.push(
                    ValidationError::new(
                        ValidationCode::UnknownState,
                        format!("driver-steps[{}].states.{state}", self.id),
                        format!("`{state}` is not a state of workflow `{}`", workflow.id),
                    )
                    .with_hint(format!("the workflow declares: {}", known.join(", "))),
                );
            }
        }

        for (state, entry) in &self.states {
            for (index, step) in entry.steps.iter().enumerate() {
                let Step::Command(command) = step else {
                    continue;
                };
                let Some(mapping) = &command.evidence else {
                    continue;
                };
                // An external tool is exempt: `default_verifiers` is a table of defaults, not of
                // constraints, and it has no row for a tool the protocol has never heard of.
                // Review finding F5.
                if !Verifier::NAMED.contains(&mapping.verifier) {
                    continue;
                }
                if !mapping.kind.default_verifiers().contains(&mapping.verifier) {
                    let can: Vec<&str> = EvidenceKind::ALL
                        .iter()
                        .filter(|kind| kind.default_verifiers().contains(&mapping.verifier))
                        .map(|kind| kind.as_str())
                        .collect();
                    errors.push(
                        ValidationError::new(
                            ValidationCode::NoVerifierForEvidence,
                            format!(
                                "driver-steps[{}].states.{state}.steps[{index}].evidence",
                                self.id
                            ),
                            format!(
                                "a `{}` does not establish `{}`",
                                mapping.verifier,
                                mapping.kind.as_str()
                            ),
                        )
                        .with_hint(format!(
                            "`{}` establishes: {}",
                            mapping.verifier,
                            if can.is_empty() {
                                "nothing this protocol names".to_owned()
                            } else {
                                can.join(", ")
                            }
                        )),
                    );
                }
            }
        }

        errors
    }

    /// Phase two: the map against the protocol in force and the workflow the plan resolved to.
    ///
    /// Run before the first step executes, because the alternative is failing at the transition
    /// that needed the evidence — halfway through a run that has already spent a token budget.
    pub fn check_run(&self, protocol: &Protocol, workflow: &Workflow) -> ValidationErrors {
        let mut errors = self.cross_validate(workflow);

        for (state, entry) in &self.states {
            for (index, step) in entry.steps.iter().enumerate() {
                let Step::Command(command) = step else {
                    continue;
                };
                let Some(mapping) = &command.evidence else {
                    continue;
                };
                if !protocol.declares_evidence(mapping.kind) {
                    errors.push(
                        ValidationError::new(
                            ValidationCode::UndeclaredEvidenceKind,
                            format!(
                                "driver-steps[{}].states.{state}.steps[{index}].evidence.kind",
                                self.id
                            ),
                            format!(
                                "protocol `{}` does not declare `{}`",
                                protocol.reference(),
                                mapping.kind.as_str()
                            ),
                        )
                        .with_hint(format!(
                            "the protocol declares: {}",
                            protocol
                                .evidence_kinds
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                    );
                }
            }
        }

        errors
    }
}

impl TryFrom<RawStepMap> for StepMap {
    type Error = ValidationErrors;

    fn try_from(raw: RawStepMap) -> Result<Self, Self::Error> {
        let mut errors = ValidationErrors::new();

        let format = StepMapFormat::validated(&raw.format, &mut errors);

        let workflow = match PinnedWorkflowRef::try_from(raw.workflow.clone()) {
            Ok(pin) => Some(pin),
            Err(refusals) => {
                errors.extend(refusals);
                None
            }
        };

        if raw.states.is_empty() {
            errors.push(ValidationError::new(
                ValidationCode::EmptyDeclaration,
                format!("driver-steps[{}].states", raw.id),
                "a step map with no states tells a driver nothing about how to run anything",
            ));
        }

        let mut states = BTreeMap::new();
        for (state, entry) in raw.states {
            let location = format!("driver-steps[{}].states.{state}", raw.id);
            if entry.visit_budget == Some(0) {
                errors.push(ValidationError::new(
                    ValidationCode::EmptyDeclaration,
                    format!("{location}.visit_budget"),
                    "a visit budget of zero forbids entering a state the workflow can reach",
                ));
            }
            let mut steps = Vec::with_capacity(entry.steps.len());
            // Where each admitted step was written, so a refusal about two of them names their own
            // paths even when a step between them was refused.
            let mut written_at = Vec::with_capacity(entry.steps.len());
            for (index, step) in entry.steps.into_iter().enumerate() {
                if let Some(step) = validate_step(
                    step,
                    &format!("{location}.steps[{index}]"),
                    format,
                    &mut errors,
                ) {
                    steps.push(step);
                    written_at.push(index);
                }
            }
            // `{transcript}` is the transcript of the `llm` step this step follows, so a state
            // with no `llm` step before the one that names it has nothing to expand. Decidable
            // here, from the document alone, which is where a step map's mistakes belong: the
            // alternative is a run that has already paid for a model session discovering that the
            // step after it cannot name what to check.
            let mut preceded_by_llm = false;
            for (index, step) in steps.iter().enumerate() {
                match step {
                    Step::Llm(_) => preceded_by_llm = true,
                    Step::Command(command)
                        if !preceded_by_llm
                            && command.expandable().any(|word| {
                                word.placeholders().any(|name| name == "transcript")
                            }) =>
                    {
                        errors.push(
                            ValidationError::new(
                                ValidationCode::MissingDeclaration,
                                format!("{location}.steps[{index}].run"),
                                format!(
                                    "`{{transcript}}` is the transcript of the `llm` step this \
                                         one follows, and no step of `{state}` before it is one"
                                ),
                            )
                            .with_hint(
                                "put the step after the `llm` step whose transcript it reads, \
                                 in the same state: a transcript from another state belongs to \
                                 another session and another prompt",
                            ),
                        );
                    }
                    _ => {}
                }
            }
            if entry.concurrent {
                refuse_shared_outputs(&steps, &written_at, &location, &mut errors);
            }

            states.insert(
                state,
                StateSteps {
                    visit_budget: entry.visit_budget.unwrap_or(DEFAULT_VISIT_BUDGET),
                    concurrent: entry.concurrent,
                    steps,
                },
            );
        }

        let Some(workflow) = workflow else {
            return Err(errors);
        };

        errors.into_result(Self {
            format,
            id: raw.id,
            workflow,
            title: raw.title,
            states,
        })
    }
}

/// The circuit-breaker half of a command step's validation, refusing a breaker that cannot work.
///
/// Its own function because `validate_step` is at the line limit, and because these three refusals
/// are one idea: a breaker that is declared and cannot fire is worse than no breaker at all, since
/// the belief that one exists is what stops somebody adding a real one.
fn validate_circuit_breaker(
    command: &RawCommandStep,
    location: &str,
    errors: &mut ValidationErrors,
) -> bool {
    let mut usable = true;
    if command.circuit_breaker.is_some() && command.depends_on.is_none() {
        errors.push(ValidationError::new(
            ValidationCode::EmptyDeclaration,
            format!("{location}.circuit_breaker"),
            "a circuit breaker is keyed by `depends_on`, and this step names no dependency; \
             say which outside thing keeps failing",
        ));
        usable = false;
    }
    // Zero opens before anything is attempted, which is not a breaker — it is a step that never
    // runs, spelled in a way that hides it.
    if command.circuit_breaker == Some(0) {
        errors.push(ValidationError::new(
            ValidationCode::EmptyDeclaration,
            format!("{location}.circuit_breaker"),
            "a circuit breaker of 0 opens before anything is attempted; that is a step that \
             never runs, so delete the step or raise the threshold",
        ));
        usable = false;
    }
    if command
        .depends_on
        .as_deref()
        .is_some_and(|name| name.trim().is_empty())
    {
        errors.push(ValidationError::new(
            ValidationCode::EmptyDeclaration,
            format!("{location}.depends_on"),
            "a dependency with no name cannot be shared with the other steps that need it",
        ));
        usable = false;
    }
    usable
}

/// Refuses two `command` steps of one `concurrent: true` state that write the same `output:`.
///
/// Run at once they would race for one file in the run directory; one after another the later
/// write wins, deterministically, so a serial state is never asked. Compared in any ASCII case,
/// as [`validated_output`] compares a reserved name: on a file system that ignores case, two
/// spellings are one file. Refused at the later step's `output` with `unsupported_construct`,
/// naming both steps (ESS `aep.drive.DeclareConcurrentOutput`, `concurrent-output-shared`).
fn refuse_shared_outputs(
    steps: &[Step],
    written_at: &[usize],
    location: &str,
    errors: &mut ValidationErrors,
) {
    let mut first_writer: Vec<(&str, usize)> = Vec::new();
    for (step, &index) in steps.iter().zip(written_at) {
        let Step::Command(CommandStep {
            output: Some(output),
            ..
        }) = step
        else {
            continue;
        };
        let earlier = first_writer
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(output))
            .map(|&(_, at)| at);
        match earlier {
            Some(at) => errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedConstruct,
                    format!("{location}.steps[{index}].output"),
                    format!(
                        "`{location}.steps[{at}]` and `{location}.steps[{index}]` both write \
                         `{}`, and in a `concurrent` state they would race for that one file",
                        output.escape_debug()
                    ),
                )
                .with_hint(
                    "give each step an output of its own, or drop `concurrent` so the later \
                     write wins",
                ),
            ),
            None => first_writer.push((output, index)),
        }
    }
}

/// Checks a command step's `output:`: a file name of its own in the run directory.
///
/// `Err` when it was refused, `Ok(None)` when the step keeps no output. Refused at
/// `<step>.output`: `empty_declaration` for a blank name, `unsupported_construct` for one holding
/// a path separator or a NUL byte, one starting with `.` (which covers `.` and `..`), one of
/// [`CommandStep::RESERVED_OUTPUTS`] in any ASCII case, and one of a step log's form
/// ([`CommandStep::is_step_log_name`]).
fn validated_output(
    output: Option<String>,
    location: &str,
    errors: &mut ValidationErrors,
) -> Result<Option<String>, ()> {
    let Some(output) = output else {
        return Ok(None);
    };
    let at = format!("{location}.output");
    if output.trim().is_empty() {
        errors.push(ValidationError::new(
            ValidationCode::EmptyDeclaration,
            at,
            "an output with no name says nowhere to keep what the command printed",
        ));
        return Err(());
    }
    // Every name the driver could not write, or would write over something of the run's own, is
    // refused here rather than when the step has already run: a kept output that cannot be
    // written is a step retried to exhaustion, and one written over a record corrupts the run.
    let why = if output.contains(['/', '\\']) {
        Some("holds a path separator, and an output is a file name in the run directory")
    } else if output.contains('\0') {
        Some("holds a NUL byte, which no file name on any target can")
    } else if output.starts_with('.') {
        Some("starts with `.`, and an output is a visible file name in the run directory")
    } else if CommandStep::RESERVED_OUTPUTS
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(&output))
    {
        // In any case: the darwin release targets run on file systems that ignore it, where
        // `State.json` is the run's own `state.json`.
        Some("is a name the run directory keeps its own records under, in some case")
    } else if CommandStep::is_step_log_name(&output) {
        Some(
            "has the form `<state>-<index>-<attempt>.log`, the name the run directory keeps a \
             step's own log under",
        )
    } else {
        None
    };
    if let Some(why) = why {
        errors.push(
            ValidationError::new(
                ValidationCode::UnsupportedConstruct,
                at,
                format!("`{}` {why}", output.escape_debug()),
            )
            .with_hint("name a file such as `collected.txt`; a later step reads it as `{run_directory}/collected.txt`"),
        );
        return Err(());
    }
    Ok(Some(output))
}

/// A command step's `expect:`, or `None` when it was refused.
///
/// `expect: failure` inverts an exit status, and a step whose verifier writes its own `record:`
/// reads no exit status, so the pair is refused at `<step>.expect` with `unsupported_construct`.
fn validated_expect(
    command: &RawCommandStep,
    location: &str,
    errors: &mut ValidationErrors,
) -> Option<Expect> {
    let expect = command.expect.unwrap_or_default();
    let reads_record = command
        .evidence
        .as_ref()
        .is_some_and(|mapping| mapping.record.is_some());
    if expect == Expect::Failure && reads_record {
        errors.push(ValidationError::new(
            ValidationCode::UnsupportedConstruct,
            format!("{location}.expect"),
            "`expect: failure` inverts an exit status, and a step whose verifier writes its own \
             `record:` reads no exit status: its verdict is the document's",
        ));
        return None;
    }
    Some(expect)
}

/// Refuses every placeholder of a command step that nothing expands, returning whether there was
/// none.
fn placeholders_are_known(
    step: &CommandStep,
    location: &str,
    errors: &mut ValidationErrors,
) -> bool {
    let mut known = true;
    for word in step.expandable() {
        for name in word.placeholders() {
            if CommandStep::PLACEHOLDERS.contains(&name) || input_named_by(name).is_some() {
                continue;
            }
            errors.push(
                ValidationError::new(
                    ValidationCode::UndeclaredReference,
                    format!("{location}.run"),
                    format!("nothing in a run expands `{{{name}}}`"),
                )
                .with_hint(format!(
                    "a step may name: {}, and a run input as `{{input.<name>}}`",
                    CommandStep::PLACEHOLDERS
                        .iter()
                        .map(|name| format!("{{{name}}}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            );
            known = false;
        }
    }
    known
}

/// Validates one step, pushing what is wrong with it and returning it when nothing is.
fn validate_step(
    raw: RawStep,
    location: &str,
    format: StepMapFormat,
    errors: &mut ValidationErrors,
) -> Option<Step> {
    match raw {
        RawStep::Command(command) => {
            let mut usable = true;
            if command.run.is_empty() || command.run.iter().all(|word| word.trim().is_empty()) {
                errors.push(ValidationError::new(
                    ValidationCode::EmptyDeclaration,
                    format!("{location}.run"),
                    "a command step with nothing to run cannot observe anything",
                ));
                usable = false;
            }
            if !validate_circuit_breaker(&command, location, errors) {
                usable = false;
            }
            let expect = validated_expect(&command, location, errors);
            usable &= expect.is_some();
            let output = validated_output(command.output, location, errors);
            usable &= output.is_ok();
            let evidence = command.evidence.and_then(|mapping| {
                let validated = validate_mapping(mapping, location, errors);
                usable &= validated.is_some();
                validated
            });
            let step = CommandStep {
                run: command.run.iter().map(|word| Word::parse(word)).collect(),
                description: command.description,
                evidence,
                retries: command.retries.unwrap_or(DEFAULT_COMMAND_RETRIES),
                depends_on: command.depends_on.clone(),
                circuit_breaker: command.circuit_breaker,
                expect: expect.unwrap_or_default(),
                output: output.unwrap_or_default(),
            };
            usable &= placeholders_are_known(&step, location, errors);
            usable.then_some(Step::Command(step))
        }
        RawStep::Llm(step) => validate_llm_step(step, location, format, errors),
        RawStep::Operator(step) => {
            if step.prompt.trim().is_empty() {
                errors.push(ValidationError::new(
                    ValidationCode::EmptyDeclaration,
                    format!("{location}.prompt"),
                    "an operator step with an empty prompt tells a person nothing",
                ));
                return None;
            }
            Some(Step::Operator(OperatorStep {
                prompt: step.prompt,
                description: step.description,
            }))
        }
    }
}

/// Validates one `llm` step of a map written in `format`.
fn validate_llm_step(
    step: RawLlmStep,
    location: &str,
    format: StepMapFormat,
    errors: &mut ValidationErrors,
) -> Option<Step> {
    // Every field is checked before any refusal returns, so a blank prompt and a missing
    // judgement are reported together (invariant *Validation accumulates*).
    let prompted = !step.prompt.trim().is_empty();
    if !prompted {
        errors.push(ValidationError::new(
            ValidationCode::EmptyDeclaration,
            format!("{location}.prompt"),
            "an llm step with an empty prompt asks for nothing",
        ));
    }
    let judgement = validated_judgement(step.judgement.as_deref(), format, location, errors);
    let context = validated_context(&step.context, location, errors);
    let commands = validated_commands(&step.commands, location, errors);
    let scope = validated_scope(&step.scope, location, errors)?;
    let commands = commands?;
    let judgement = judgement?;
    if !prompted {
        return None;
    }
    Some(Step::Llm(LlmStep {
        prompt: step.prompt,
        context,
        scope,
        commands,
        skills: step.skills,
        harness: step
            .harness
            .unwrap_or_else(|| LlmStep::DEFAULT_HARNESS.to_owned()),
        description: step.description,
        judgement,
    }))
}

/// Validates an evidence mapping against what a driver can mint from an exit status.
///
/// `record:` changes which question is being asked. Without it the driver builds the record from
/// the exit status, so the kind has to be one an exit status can carry; with it the verifier wrote
/// the record and the driver only reads it, so every kind the protocol declares is admissible and
/// the suite comes out of the document rather than out of this file.
fn validate_mapping(
    raw: RawEvidenceMapping,
    location: &str,
    errors: &mut ValidationErrors,
) -> Option<EvidenceMapping> {
    let mut usable = true;
    let record = match raw.record {
        Some(path) if path.trim().is_empty() => {
            errors.push(ValidationError::new(
                ValidationCode::EmptyDeclaration,
                format!("{location}.evidence.record"),
                "a record path with nothing in it names no document for the driver to read",
            ));
            usable = false;
            None
        }
        other => other.as_deref().map(Word::parse),
    };
    let written = record.is_some();
    if !written && !EvidenceMapping::MINTABLE.contains(&raw.kind) {
        errors.push(
            ValidationError::new(
                ValidationCode::UnsupportedConstruct,
                format!("{location}.evidence.kind"),
                format!(
                    "a driver reads an exit status, which cannot establish `{}`",
                    raw.kind.as_str()
                ),
            )
            .with_hint(format!(
                "a command step can mint: {}. A verifier that writes the record itself declares \
                 `record:` beside `kind:`, and then the driver reads the document instead of \
                 minting anything",
                EvidenceMapping::MINTABLE
                    .iter()
                    .map(|kind| kind.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        );
        usable = false;
    }
    if !written && raw.kind == EvidenceKind::TestResult && raw.suite.is_none() {
        errors.push(
            ValidationError::new(
                ValidationCode::MissingDeclaration,
                format!("{location}.evidence.suite"),
                "a test result names the suite that ran, and nothing in an exit status says which",
            )
            .with_hint("write `suite: unit`, `suite: contract` or another declared suite"),
        );
        usable = false;
    }
    usable.then_some(EvidenceMapping {
        kind: raw.kind,
        verifier: raw.verifier,
        suite: raw.suite,
        subject: raw.subject,
        tool: raw.tool,
        record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<StepMap, ValidationErrors> {
        let raw: RawStepMap = serde_yaml_free(text);
        StepMap::try_from(raw)
    }

    /// Deserializes a document without a YAML dependency in this crate: JSON is the wire form the
    /// tests use, and `aep-schema` is what reads YAML in production.
    fn serde_yaml_free(json: &str) -> RawStepMap {
        serde_json::from_str(json).expect("the fixture deserializes")
    }

    /// A map in the current format, `aep.driver-steps/2`, which adopts the judgement rule.
    fn map_json(workflow: &str, states: &str) -> String {
        format!(
            r#"{{"format":"aep.driver-steps/2","id":"development/default",
                "workflow":"{workflow}","states":{states}}}"#
        )
    }

    /// A map in `aep.driver-steps/1`, as every protocol tree before 0.72.0 writes one.
    fn map_json_v1(workflow: &str, states: &str) -> String {
        format!(
            r#"{{"format":"aep.driver-steps/1","id":"development/default",
                "workflow":"{workflow}","states":{states}}}"#
        )
    }

    const ONE_COMMAND: &str = r#"{"implement":{"steps":[
        {"kind":"command","run":["cargo","test"],
         "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}}]}}"#;

    #[test]
    fn a_map_validates_and_carries_its_pin() {
        let map = read(&map_json("adp/default/1", ONE_COMMAND)).expect("valid");
        assert_eq!(map.workflow.to_string(), "adp/default/1");
        assert_eq!(map.format(), STEP_MAP_FORMAT);
        assert_eq!(map.steps_for(&StateId::new("implement").unwrap()).len(), 1);
        assert_eq!(
            map.visit_budget(&StateId::new("implement").unwrap()),
            DEFAULT_VISIT_BUDGET
        );
    }

    #[test]
    fn an_unpinned_workflow_is_refused() {
        let errors = read(&map_json("adp/default", ONE_COMMAND)).expect_err("refused");
        assert_eq!(errors.len(), 1);
        assert!(errors.contains(ValidationCode::MissingDeclaration));
    }

    #[test]
    fn a_wrong_format_version_is_refused() {
        let text = r#"{"format":"aep.driver-steps/3","id":"development/default",
                       "workflow":"adp/default/1","states":{"implement":{"steps":[]}}}"#;
        let errors = read(text).expect_err("refused");
        assert!(errors.contains(ValidationCode::UnsupportedFormatVersion));
    }

    /// Both format versions this build reads load, and the map says which one it is.
    #[test]
    fn a_map_carries_the_format_version_it_was_written_in() {
        let v1 = read(&map_json_v1("adp/default/1", ONE_COMMAND)).expect("a /1 map loads");
        let v2 = read(&map_json("adp/default/1", ONE_COMMAND)).expect("a /2 map loads");
        assert_eq!(v1.format(), "aep.driver-steps/1");
        assert_eq!(v2.format(), "aep.driver-steps/2");
        assert_eq!(
            STEP_MAP_FORMAT, "aep.driver-steps/2",
            "new maps are written in /2"
        );
    }

    /// Release 0.72.1: an `aep.driver-steps/1` map adopted no judgement rule. Every protocol tree
    /// before 0.72.0 ships one, and a store pinning such a tree can never add the key, so its
    /// `llm` step loads without one, as it did in 0.71.x, and is still counted.
    #[test]
    fn a_v1_llm_step_without_a_judgement_loads_as_it_did_before_0_72() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["cargo","test"]},
            {"kind":"llm","prompt":"write the change"}]},
            "review":{"steps":[{"kind":"llm","prompt":"review it"}]}}"#;
        let map = read(&map_json_v1("adp/default/1", states))
            .unwrap_or_else(|errors| panic!("a /1 map needs no judgement: {errors}"));
        let Step::Llm(step) = &map.steps_for(&StateId::new("implement").unwrap())[1] else {
            panic!("the second step is the llm step");
        };
        assert_eq!(step.judgement, None, "nothing is invented for it");
        assert_eq!(map.llm_steps_declared(), 2, "both llm steps are counted");
    }

    /// A /1 map that writes a judgement is held to it: a blank one says nothing, in either format.
    #[test]
    fn a_v1_llm_step_with_a_blank_judgement_is_still_refused() {
        let states = r#"{"implement":{"steps":[
            {"kind":"llm","prompt":"write the change","judgement":"  "}]}}"#;
        let errors = read(&map_json_v1("adp/default/1", states)).expect_err("refused");
        assert_eq!(errors.len(), 1, "{errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(refusal.code, ValidationCode::EmptyDeclaration, "{errors}");
        assert_eq!(
            refusal.location,
            "driver-steps[development/default].states.implement.steps[0].judgement"
        );
    }

    /// A /1 map's judgement, when written, is kept as a /2 map's is.
    #[test]
    fn a_v1_llm_step_keeps_the_judgement_it_writes() {
        let states = r#"{"implement":{"steps":[
            {"kind":"llm","prompt":"a","judgement":"  what the change should say  "}]}}"#;
        let map = read(&map_json_v1("adp/default/1", states)).expect("valid");
        let Step::Llm(step) = &map.steps_for(&StateId::new("implement").unwrap())[0] else {
            panic!("the first step is the llm step");
        };
        assert_eq!(
            step.judgement.as_deref(),
            Some("what the change should say")
        );
    }

    #[test]
    fn four_broken_steps_report_four_errors() {
        // Invariant *Validation accumulates*, asserted as an exact count rather than as "is an
        // error": a validator that returned on the first failure passes an `is_err` test and fails
        // this one.
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":[]},
            {"kind":"command","run":["x"],"evidence":{"kind":"metric_observation","verifier":"compiler"}},
            {"kind":"command","run":["x"],"evidence":{"kind":"test_result","verifier":"test-runner"}},
            {"kind":"llm","prompt":"  ","judgement":"which change"}]}}"#;
        let errors = read(&map_json("adp/default/1", states)).expect_err("refused");
        assert_eq!(
            errors.len(),
            4,
            "one error per broken step, not one for the document: {errors}"
        );
    }

    /// A kind an exit status cannot carry loads when the verifier writes the record itself.
    ///
    /// `trace_conformance` is the case: its record holds two digests and three counts, which is
    /// why it is not in `MINTABLE` and why it was unreachable from a step map until `record:`
    /// existed. The suite rule relaxes with it — a `test_result` a verifier wrote names its own
    /// suite in the document, and requiring the map to name it too would let the two disagree.
    #[test]
    fn a_kind_no_exit_status_can_carry_loads_when_the_verifier_writes_the_record() {
        let states = r#"{"implement":{"steps":[
            {"kind":"llm","prompt":"do the work","judgement":"what the work is, which no command decides"},
            {"kind":"command","run":["aep","observe","trace","evidence","--transcript","{transcript}"],
             "evidence":{"kind":"trace_conformance","verifier":"trace-checker",
                         "record":"{run_directory}/trace.yaml"}}]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let steps = map.steps_for(&StateId::new("implement").unwrap());
        let Step::Command(command) = &steps[1] else {
            panic!("the second step is the command");
        };
        assert_eq!(
            command
                .evidence
                .as_ref()
                .and_then(|mapping| mapping.record.clone())
                .as_ref()
                .map(Word::as_str),
            Some("{run_directory}/trace.yaml")
        );

        // And without `record:` the same step is refused, because then the driver would have to
        // invent the digests.
        let minted = r#"{"implement":{"steps":[
            {"kind":"command","run":["aep","observe","trace","check"],
             "evidence":{"kind":"trace_conformance","verifier":"trace-checker"}}]}}"#;
        let errors = read(&map_json("adp/default/1", minted)).expect_err("refused");
        assert!(
            errors.contains(ValidationCode::UnsupportedConstruct),
            "{errors}"
        );
    }

    /// A misspelled placeholder is refused at load, where it costs nothing to fix.
    #[test]
    fn a_placeholder_nothing_expands_is_refused_and_a_brace_that_is_not_one_is_left_alone() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["check","--at","{transcirpt}"]}]}}"#;
        let errors = read(&map_json("adp/default/1", states)).expect_err("refused");
        assert!(
            errors.contains(ValidationCode::UndeclaredReference),
            "{errors}"
        );

        // `{}` is `find -exec`'s argument and `{a: .b}` is a jq program: neither is a placeholder,
        // and a validator that refused them would refuse ordinary command lines.
        let literal = r#"{"implement":{"steps":[
            {"kind":"command","run":["find",".","-exec","rm","{}",";"]},
            {"kind":"command","run":["jq","{a: .b}"]}]}}"#;
        read(&map_json("adp/default/1", literal)).expect("braces that name nothing are text");
    }

    /// `{task}` is in the vocabulary, and the hint a misspelling gets names all three.
    ///
    /// Both halves in one test on purpose: the hint is what a person acts on, and a vocabulary
    /// that grew without the hint growing with it is a refusal that tells the reader to write one
    /// of the two names that were legal last year. It is read out of `PLACEHOLDERS` rather than
    /// spelled here, so this asserts the three names arrive rather than that a literal matches
    /// itself.
    #[test]
    fn the_task_document_can_be_named_and_a_misspelling_is_offered_all_three_names() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["aep","observe","specification","evidence","--task","{task}"]}]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("`{task}` is expandable");
        let steps = map.steps_for(&StateId::new("implement").unwrap());
        let Step::Command(command) = &steps[0] else {
            panic!("the only step is the command");
        };
        assert!(
            command.expandable().any(|word| word == "{task}"),
            "the placeholder survives validation as written: {:?}",
            command.run
        );

        // And a state with no `llm` step in it is fine, unlike `{transcript}`: which task a run is
        // driving is known before the first step, so nothing about the order can withhold it.
        assert_eq!(
            steps.len(),
            1,
            "`{{task}}` needs no step before it, so this map is one step long"
        );

        let misspelled = r#"{"implement":{"steps":[
            {"kind":"command","run":["aep","observe","specification","evidence","--task","{tsak}"]}]}}"#;
        let errors = read(&map_json("adp/default/1", misspelled)).expect_err("refused");
        assert!(
            errors.contains(ValidationCode::UndeclaredReference),
            "{errors}"
        );
        let said = errors.to_string();
        for name in CommandStep::PLACEHOLDERS {
            assert!(
                said.contains(&format!("{{{name}}}")),
                "the hint offers every name a step may write, and `{{{name}}}` is missing: {said}"
            );
        }
    }

    /// `{transcript}` in a state with no `llm` step before it names a session that never happened.
    #[test]
    fn a_transcript_placeholder_with_no_session_before_it_is_refused() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["aep","observe","trace","check","--transcript","{transcript}"]},
            {"kind":"llm","prompt":"too late to help the step above","judgement":"j"}]}}"#;
        let errors = read(&map_json("adp/default/1", states)).expect_err("refused");
        assert!(
            errors.contains(ValidationCode::MissingDeclaration),
            "{errors}"
        );
    }

    /// story:llm-step-declares-judgement, acceptance 1: an `llm` step that does not say why it
    /// needs a model is refused at load, with a stable code at the step's own path.
    ///
    /// The step is the second of its state so the path's index is read, not assumed; and a
    /// `command` and an `operator` step beside it carry no judgement and are not refused.
    #[test]
    fn an_llm_step_without_a_judgement_is_refused_with_a_stable_code_at_its_path() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["cargo","test"]},
            {"kind":"llm","prompt":"write the change"},
            {"kind":"operator","prompt":"approve it"}]}}"#;
        let errors = read(&map_json("adp/default/1", states)).expect_err("refused");
        assert_eq!(errors.len(), 1, "one refusal, for the llm step: {errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(refusal.code, ValidationCode::MissingDeclaration, "{errors}");
        assert_eq!(
            refusal.location,
            "driver-steps[development/default].states.implement.steps[1].judgement",
            "the refusal names the step it is about"
        );
    }

    /// A judgement of nothing says nothing, and is refused at the same path.
    #[test]
    fn an_llm_step_with_a_blank_judgement_is_refused_at_its_path() {
        let states = r#"{"implement":{"steps":[
            {"kind":"llm","prompt":"write the change","judgement":"   "}]}}"#;
        let errors = read(&map_json("adp/default/1", states)).expect_err("refused");
        assert_eq!(errors.len(), 1, "{errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(refusal.code, ValidationCode::EmptyDeclaration, "{errors}");
        assert_eq!(
            refusal.location,
            "driver-steps[development/default].states.implement.steps[0].judgement"
        );
    }

    /// Missing judgements accumulate, one per step, across states (invariant *Validation
    /// accumulates*).
    #[test]
    fn every_llm_step_without_a_judgement_is_reported_not_only_the_first() {
        let states = r#"{"implement":{"steps":[{"kind":"llm","prompt":"a"},{"kind":"llm","prompt":"b"}]},
            "review":{"steps":[{"kind":"llm","prompt":"c"}]}}"#;
        let errors = read(&map_json("adp/default/1", states)).expect_err("refused");
        let locations: Vec<&str> = errors
            .as_slice()
            .iter()
            .filter(|error| error.code == ValidationCode::MissingDeclaration)
            .map(|error| error.location.as_str())
            .collect();
        assert_eq!(
            locations,
            [
                "driver-steps[development/default].states.implement.steps[0].judgement",
                "driver-steps[development/default].states.implement.steps[1].judgement",
                "driver-steps[development/default].states.review.steps[0].judgement",
            ],
            "{errors}"
        );
    }

    /// A judgement that says something is kept, trimmed, and the map counts its `llm` steps.
    #[test]
    fn an_llm_step_keeps_its_judgement_and_the_map_counts_its_llm_steps() {
        let states = r#"{"implement":{"steps":[
            {"kind":"llm","prompt":"a","judgement":"  what the change should say  "},
            {"kind":"command","run":["cargo","test"]}]},
            "review":{"steps":[{"kind":"llm","prompt":"b","judgement":"whether it is right"},
            {"kind":"operator","prompt":"approve it"}]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let Step::Llm(step) = &map.steps_for(&StateId::new("implement").unwrap())[0] else {
            panic!("the first step is the llm step");
        };
        assert_eq!(
            step.judgement.as_deref(),
            Some("what the change should say")
        );
        assert_eq!(map.llm_steps_declared(), 2, "two llm steps over two states");
    }

    #[test]
    fn an_empty_map_is_refused() {
        let errors = read(&map_json("adp/default/1", "{}")).expect_err("refused");
        assert!(errors.contains(ValidationCode::EmptyDeclaration));
    }

    #[test]
    fn an_llm_step_has_nowhere_to_put_evidence() {
        // The type is the mechanism: `deny_unknown_fields` refuses the key outright, so this is a
        // deserialization failure rather than a validation one, and no relaxation of a rule can
        // reintroduce it.
        let text = r#"{"format":"aep.driver-steps/1","id":"a","workflow":"adp/default/1",
            "states":{"implement":{"steps":[{"kind":"llm","prompt":"go","judgement":"j",
            "evidence":{"kind":"test_result","verifier":"test-runner"}}]}}}"#;
        let outcome: Result<RawStepMap, _> = serde_json::from_str(text);
        assert!(
            outcome.is_err(),
            "an llm step must not accept an evidence block"
        );
    }

    /// Gap register `:78`. A breaker that cannot be keyed, or that opens before anything runs,
    /// is refused at load rather than ignored.
    ///
    /// A map whose author believes it has a circuit breaker and does not is worse than one with
    /// none, because the belief is what stops them adding a real one.
    #[test]
    fn a_circuit_breaker_that_cannot_work_is_refused_at_load() {
        for (fragment, expected) in [
            (
                r#"{"kind":"command","run":["x"],"circuit_breaker":2}"#,
                "names no dependency",
            ),
            (
                r#"{"kind":"command","run":["x"],"depends_on":"api","circuit_breaker":0}"#,
                "opens before anything is attempted",
            ),
            (
                r#"{"kind":"command","run":["x"],"depends_on":"  ","circuit_breaker":2}"#,
                "cannot be shared",
            ),
        ] {
            let mut errors = ValidationErrors::new();
            let raw: RawStep = serde_json::from_str(fragment).expect("parses as a raw step");
            let step = validate_step(raw, "steps[0]", StepMapFormat::V2, &mut errors);
            assert!(step.is_none(), "{fragment} must not load");
            let said = errors.to_string();
            assert!(
                said.contains(expected),
                "the refusal must say what is wrong ({expected}): {said}"
            );
        }
    }

    /// And a well-formed one loads, so the guard is a filter and not a wall.
    #[test]
    fn a_well_formed_circuit_breaker_loads() {
        let mut errors = ValidationErrors::new();
        let raw: RawStep = serde_json::from_str(
            r#"{"kind":"command","run":["x"],"depends_on":"staging-cluster","circuit_breaker":3}"#,
        )
        .expect("parses as a raw step");
        let step = validate_step(raw, "steps[0]", StepMapFormat::V2, &mut errors).expect("loads");
        let Step::Command(command) = step else {
            panic!("expected a command step");
        };
        assert_eq!(command.depends_on.as_deref(), Some("staging-cluster"));
        assert_eq!(command.circuit_breaker, Some(3));
    }

    #[test]
    fn retry_budgets_are_per_step_kind() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["x"]},
            {"kind":"command","run":["y"],"retries":0},
            {"kind":"llm","prompt":"go","judgement":"j"},
            {"kind":"operator","prompt":"decide"}]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let steps = map.steps_for(&StateId::new("implement").unwrap());
        assert_eq!(steps[0].retry_budget(), DEFAULT_COMMAND_RETRIES);
        assert_eq!(steps[1].retry_budget(), 0);
        assert_eq!(steps[2].retry_budget(), LLM_RETRIES);
        assert_eq!(
            steps[3].retry_budget(),
            0,
            "a person is not a flaky dependency"
        );
    }

    #[test]
    fn the_digest_ignores_a_reordered_document_and_moves_with_a_changed_step() {
        let first = read(&map_json("adp/default/1", ONE_COMMAND)).expect("valid");
        let second = read(&map_json("adp/default/1", ONE_COMMAND)).expect("valid");
        assert_eq!(first.digest(), second.digest());
        let other = r#"{"implement":{"steps":[
            {"kind":"command","run":["cargo","test","--workspace"],
             "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}}]}}"#;
        let changed = read(&map_json("adp/default/1", other)).expect("valid");
        assert_ne!(first.digest(), changed.digest());
    }

    #[test]
    fn an_external_tool_verifier_loads_where_a_named_one_would_be_refused() {
        let external = r#"{"implement":{"steps":[
            {"kind":"command","run":["ruff"],
             "evidence":{"kind":"static_analysis","verifier":"ruff"}}]}}"#;
        let map = read(&map_json("adp/default/1", external)).expect("valid");
        let workflow = fixture_workflow();
        assert!(
            map.cross_validate(&workflow).is_empty(),
            "an external tool has no row in the defaults table, so there is nothing to refuse it \
             against"
        );
    }

    #[test]
    fn a_named_verifier_that_cannot_produce_the_kind_is_refused_at_load() {
        let wrong = r#"{"implement":{"steps":[
            {"kind":"command","run":["x"],
             "evidence":{"kind":"contract_result","verifier":"test-runner"}}]}}"#;
        let map = read(&map_json("adp/default/1", wrong)).expect("valid");
        let errors = map.cross_validate(&fixture_workflow());
        assert_eq!(errors.len(), 1);
        assert!(errors.contains(ValidationCode::NoVerifierForEvidence));
    }

    #[test]
    fn a_renamed_state_and_a_moved_major_accumulate_rather_than_short_circuit() {
        let states = r#"{"implement":{"steps":[]},"polish":{"steps":[]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let mut workflow = fixture_workflow();
        workflow.version = MajorVersion::new(2).expect("a major version");
        let errors = map.cross_validate(&workflow);
        assert_eq!(errors.len(), 2, "{errors}");
        assert!(errors.contains(ValidationCode::VersionMismatch));
        assert!(errors.contains(ValidationCode::UnknownState));
    }

    /// Phase two refuses a kind the protocol in force does not declare — and phase one cannot.
    ///
    /// The map is well formed and the workflow is the one it pins, so `cross_validate` has nothing
    /// to say: which protocol is in force comes from the task, and no document loader has seen it.
    /// The same map against a protocol that does declare the kind loads, so this is a filter and
    /// not a wall.
    #[test]
    fn an_evidence_kind_the_protocol_in_force_does_not_declare_is_refused_at_run_start() {
        let map = read(&map_json("adp/default/1", ONE_COMMAND)).expect("valid");
        let workflow = fixture_workflow();
        assert!(
            map.cross_validate(&workflow).is_empty(),
            "the fixture reaches phase two: phase one has nothing to refuse"
        );

        let errors = map.check_run(&fixture_protocol(r#"["approval"]"#), &workflow);
        assert_eq!(errors.len(), 1, "{errors}");
        assert!(
            errors.contains(ValidationCode::UndeclaredEvidenceKind),
            "{errors}"
        );
        let said = errors.to_string();
        assert!(
            said.contains("test_result"),
            "the refusal names the kind the step declares: {said}"
        );
        assert!(
            said.contains("aep/1"),
            "and the protocol that does not declare it: {said}"
        );
        assert!(
            said.contains("approval"),
            "and what the protocol does declare: {said}"
        );

        assert!(
            map.check_run(&fixture_protocol(r#"["test_result"]"#), &workflow)
                .is_empty(),
            "the same map against a protocol that declares the kind is not refused"
        );
    }

    use aep_domain::version::MajorVersion;

    /// A protocol declaring exactly the evidence kinds named, for the phase-two check.
    fn fixture_protocol(evidence_kinds: &str) -> Protocol {
        let raw: aep_domain::raw::RawProtocol = serde_json::from_str(&format!(
            r#"{{"id":"aep","version":1,"title":"t","evidence_kinds":{evidence_kinds},
                "verifiers":["test-runner","human-approval"],"observables":["tests.**"]}}"#
        ))
        .expect("the fixture deserializes");
        Protocol::try_from(raw).expect("the fixture validates")
    }

    /// A workflow with one state, built through its own validator.
    fn fixture_workflow() -> Workflow {
        let raw: aep_domain::raw::RawWorkflow = serde_json::from_str(
            r#"{"id":"adp/default","version":1,"title":"t","initial":"implement",
                "states":{"implement":{"title":"Implement","terminal":true}},
                "transitions":[]}"#,
        )
        .expect("the fixture deserializes");
        Workflow::try_from(raw).expect("the fixture validates")
    }

    // --- declared context and write scope ---------------------------------------------------

    fn llm_step(extra: &str) -> String {
        format!(
            r#"{{"specify":{{"steps":[{{"kind":"llm","prompt":"write it down","judgement":"what to write down"{extra}}}]}}}}"#
        )
    }

    fn only_llm_step(extra: &str) -> Result<StepMap, ValidationErrors> {
        read(&map_json("adp/default/1", &llm_step(extra)))
    }

    #[test]
    fn a_step_may_declare_the_files_it_is_given_and_where_it_may_write() {
        let map = only_llm_step(
            r#","context":["docs/skill.md"],
                "scope":[{"paths":[".engineering/planning/**"],"write":"partial-only"},
                         {"paths":["**"],"write":"denied"}]"#,
        )
        .expect("valid");
        let Some(Step::Llm(step)) = map.states.values().next().expect("one state").steps.first()
        else {
            panic!("one llm step");
        };
        assert_eq!(step.context, vec!["docs/skill.md".to_owned()]);
        assert_eq!(step.scope.len(), 2);
        assert_eq!(step.scope[0].write, WriteScope::PartialOnly);
        assert_eq!(step.scope[1].write, WriteScope::Denied);
    }

    #[test]
    fn a_step_that_declares_neither_is_unchanged_and_claims_nothing() {
        // Empty is not "allows everything": nothing is enforced and nothing says it is. Every map
        // written before this existed keeps meaning exactly what it meant.
        let map = only_llm_step("").expect("valid");
        let Some(Step::Llm(step)) = map.states.values().next().expect("one state").steps.first()
        else {
            panic!("one llm step");
        };
        assert!(step.context.is_empty() && step.scope.is_empty());
    }

    #[test]
    fn context_is_a_list_of_files_and_a_glob_is_refused_by_name() {
        // A step map is pinned, so a glob is a promise about what a directory will hold later.
        let errors = only_llm_step(r#","context":["crates/**/*.rs"]"#).expect_err("refused");
        assert!(format!("{errors}").contains("glob"), "{errors}");
        assert!(
            only_llm_step(r#","context":[" "]"#).is_err(),
            "and a blank names no file"
        );
    }

    #[test]
    fn a_scope_whose_last_rule_is_not_a_catch_all_is_refused() {
        // A path nobody mentioned must have an answer. Leaving it to a default is how a scope
        // silently stops covering the tree it was written for.
        let errors = only_llm_step(r#","scope":[{"paths":["crates/**"],"write":"allowed"}]"#)
            .expect_err("refused");
        assert!(format!("{errors}").contains("`**`"), "{errors}");
    }

    #[test]
    fn a_scope_rule_about_no_path_is_refused() {
        assert!(
            only_llm_step(r#","scope":[{"paths":[],"write":"denied"}]"#).is_err(),
            "a rule with no paths is about nothing"
        );
    }

    // --- declared command surface (story:checks-map-per-phase-command-surface) --------------

    #[test]
    fn a_step_may_declare_the_commands_its_shell_admits_and_they_are_kept_in_order() {
        let map =
            only_llm_step(r#","commands":[["task"],["cargo","test"],["aep","plan","artifact"]]"#)
                .expect("valid");
        let Some(Step::Llm(step)) = map.states.values().next().expect("one state").steps.first()
        else {
            panic!("one llm step");
        };
        let rendered: Vec<String> = step.commands.iter().map(ToString::to_string).collect();
        assert_eq!(rendered, ["task", "cargo test", "aep plan artifact"]);
        assert_eq!(step.commands[1].program(), "cargo");
    }

    #[test]
    fn a_command_prefix_with_no_words_or_a_blank_word_is_refused_at_its_own_index() {
        // Both defects in one map, both reported: a prefix that names nothing and a word that
        // can never match one word of a split invocation.
        let errors = only_llm_step(r#","commands":[[],["cargo","test --workspace"],["task"]]"#)
            .expect_err("refused");
        let codes: Vec<(ValidationCode, String)> = errors
            .as_slice()
            .iter()
            .map(|error| (error.code, error.location.clone()))
            .collect();
        assert!(
            codes.contains(&(
                ValidationCode::EmptyDeclaration,
                "driver-steps[development/default].states.specify.steps[0].commands[0]".to_owned()
            )),
            "the empty prefix is refused at index 0: {codes:?}"
        );
        assert!(
            codes.contains(&(
                ValidationCode::EmptyDeclaration,
                "driver-steps[development/default].states.specify.steps[0].commands[1]".to_owned()
            )),
            "the word with whitespace inside it is refused at index 1: {codes:?}"
        );
    }

    #[test]
    fn a_program_that_runs_another_program_cannot_be_declared() {
        // `bash -c` admits whatever it is handed, which is a general shell declared under
        // another name — the one thing the declaration exists to rule out. The list is every
        // runner the loader knows, so a name that falls off it fails here by name.
        let accepted: Vec<&str> = [
            "bash",
            "sh",
            "zsh",
            "dash",
            "fish",
            "ksh",
            "mksh",
            "csh",
            "tcsh",
            "busybox",
            "eval",
            "exec",
            "source",
            ".",
            "command",
            "builtin",
            "env",
            "xargs",
            "sudo",
            "doas",
            "su",
            "runuser",
            "pkexec",
            "run0",
            "systemd-run",
            "nohup",
            "timeout",
            "nice",
            "ionice",
            "setsid",
            "stdbuf",
            "script",
            "chroot",
            "unshare",
            "flock",
            "taskset",
            "time",
            "watch",
            "strace",
            "ltrace",
            "find",
            "perl",
            "python",
            "python2",
            "python3",
            "node",
            "nodejs",
            "ruby",
            "php",
            "lua",
            "luajit",
            "deno",
            "bun",
            "awk",
            "gawk",
            "mawk",
            "nawk",
            "npx",
        ]
        .into_iter()
        .filter(|runner| {
            let loaded = only_llm_step(&format!(r#","commands":[["{runner}","-c"]]"#));
            !loaded.is_err_and(|errors| errors.contains(ValidationCode::UnsupportedConstruct))
        })
        .collect();
        assert!(
            accepted.is_empty(),
            "the loader accepts these program runners as declared commands: {accepted:?}"
        );
    }

    #[test]
    fn an_empty_word_in_a_prefix_is_refused() {
        // A word with whitespace inside it is one case; the empty word is the other, and it
        // contains no whitespace character at all. Both are refused with the same code.
        let errors = only_llm_step(r#","commands":[["cargo",""]]"#).expect_err("refused");
        assert!(
            errors.contains(ValidationCode::EmptyDeclaration),
            "an empty word is refused: {errors}"
        );
    }

    #[test]
    fn a_prefix_names_its_program_and_not_a_path() {
        // The driver compares programs by their last path component, so `[/usr/bin/cargo, test]`
        // would read as a promise about `/usr/bin` that `./cargo test` satisfies. The loader
        // refuses the path, and the map says `[cargo, test]`.
        let errors =
            only_llm_step(r#","commands":[["/usr/bin/cargo","test"]]"#).expect_err("refused");
        assert!(
            errors.contains(ValidationCode::UnsupportedConstruct),
            "a program written as a path is refused: {errors}"
        );
        assert!(
            errors.to_string().contains("`/usr/bin/cargo` is a path"),
            "the refusal names the path: {errors}"
        );
    }

    #[test]
    fn a_step_that_declares_no_commands_keeps_the_digest_it_had() {
        // The cursor pins a run to its map's digest, and every map written before `commands:`
        // existed declares none. Their digests must not move, or every in-flight run under
        // them refuses to resume after an upgrade.
        let before = only_llm_step("").expect("valid");
        let serialized = serde_json::to_value(&before).expect("a map serializes");
        let step = &serialized["states"]["specify"]["steps"][0];
        assert!(
            step.get("commands").is_none(),
            "an undeclared `commands` is absent from the canonical form: {step}"
        );
        let declared = only_llm_step(r#","commands":[["cargo","test"]]"#).expect("valid");
        assert_ne!(
            before.digest(),
            declared.digest(),
            "a declaration is part of the map the cursor pins"
        );
    }

    #[test]
    fn a_step_that_declares_no_commands_declares_nothing() {
        // Empty is not "admits nothing": the driver holds an undeclared step to its own default
        // surface, and every map written before this key existed keeps meaning what it meant.
        let map = only_llm_step("").expect("valid");
        let Some(Step::Llm(step)) = map.states.values().next().expect("one state").steps.first()
        else {
            panic!("one llm step");
        };
        assert_eq!(step.commands, Vec::<CommandPrefix>::new());
    }

    // --- expected failure, run inputs and kept output (story:command-step-expects-failure) ---

    /// The one command step of `states`, which must load.
    fn only_command(states: &str) -> CommandStep {
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let Some(Step::Command(command)) = map.states.values().next().and_then(|s| s.steps.first())
        else {
            panic!("the first step is a command");
        };
        command.clone()
    }

    /// Acceptance 1 and 2, decided where the driver reads them: a nonzero exit is the pass, and
    /// exit 0 is a failing verdict whose reason is "nothing reproduced".
    #[test]
    fn a_step_expecting_failure_reads_its_exit_status_inverted() {
        let command = only_command(
            r#"{"implement":{"steps":[{"kind":"command","run":["cargo","test","repro"],
                "expect":"failure",
                "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}}]}}"#,
        );
        assert_eq!(command.expect, Expect::Failure);
        assert_eq!(
            command.verdict(1),
            Ok(CommandVerdict {
                passed: true,
                reason: None
            }),
            "a reproduced failure is the pass"
        );
        assert_eq!(
            command.verdict(0),
            Ok(CommandVerdict {
                passed: false,
                reason: Some(NOTHING_REPRODUCED)
            }),
            "exit 0 reproduced nothing, and says so"
        );
        assert_eq!(NOTHING_REPRODUCED, "nothing reproduced");
    }

    /// A shell's "could not execute" (126) and "not found" (127) reproduce nothing: no verdict,
    /// naming the status. Any other nonzero exit, 128 and above included, is still a reproduction,
    /// and under `expect: success` 126 and 127 stay a failing verdict.
    #[test]
    fn a_step_expecting_failure_reads_126_and_127_as_no_verdict() {
        let command = only_command(
            r#"{"implement":{"steps":[{"kind":"command","run":["sh","-c","repro"],
                "expect":"failure",
                "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}}]}}"#,
        );
        for code in [126, 127] {
            let reason = command
                .verdict(code)
                .expect_err("a command the shell never ran reproduced nothing");
            assert!(
                reason.contains(&format!("exit status {code}")),
                "the reason names the status: {reason}"
            );
        }
        for code in [1, 2, 101, 125, 128, 134, 139, 255] {
            assert_eq!(
                command.verdict(code),
                Ok(CommandVerdict {
                    passed: true,
                    reason: None
                }),
                "exit {code} is a reproduction"
            );
        }
        let plain = only_command(ONE_COMMAND);
        for code in [126, 127] {
            assert_eq!(
                plain.verdict(code),
                Ok(CommandVerdict {
                    passed: false,
                    reason: None
                }),
                "under `expect: success` exit {code} is a failing verdict, as before"
            );
        }
    }

    /// Acceptance 3: a step that does not write `expect` reads its exit status as before, and its
    /// map digests as before — the key is not serialized, so a run pinned to an older map's digest
    /// still resumes against it.
    #[test]
    fn a_step_without_expect_reads_and_digests_exactly_as_before() {
        let plain = only_command(ONE_COMMAND);
        assert_eq!(plain.expect, Expect::Success);
        assert_eq!(
            plain.verdict(0),
            Ok(CommandVerdict {
                passed: true,
                reason: None
            })
        );
        assert_eq!(
            plain.verdict(2),
            Ok(CommandVerdict {
                passed: false,
                reason: None
            })
        );

        let explicit = r#"{"implement":{"steps":[
            {"kind":"command","run":["cargo","test"],"expect":"success",
             "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}}]}}"#;
        let unwritten = read(&map_json("adp/default/1", ONE_COMMAND)).expect("valid");
        let written = read(&map_json("adp/default/1", explicit)).expect("valid");
        assert_eq!(unwritten.digest(), written.digest());
        let serialized = serde_json::to_string(&unwritten).expect("a map serializes");
        assert!(
            !serialized.contains("expect") && !serialized.contains("output"),
            "an unwritten key adds nothing to the canonical form: {serialized}"
        );
    }

    /// `expect: failure` inverts an exit status; a step whose verifier writes the record reads no
    /// exit status, so there is nothing to invert and the pair is refused at the key.
    #[test]
    fn an_expected_failure_whose_verifier_writes_the_record_is_refused_at_its_path() {
        let states = r#"{"implement":{"steps":[
            {"kind":"llm","prompt":"p","judgement":"j"},
            {"kind":"command","run":["aep","observe","trace","evidence"],"expect":"failure",
             "evidence":{"kind":"trace_conformance","verifier":"trace-checker",
                         "record":"{run_directory}/trace.yaml"}}]}}"#;
        let errors = read(&map_json("adp/default/1", states)).expect_err("refused");
        assert_eq!(errors.len(), 1, "{errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(
            refusal.code,
            ValidationCode::UnsupportedConstruct,
            "{errors}"
        );
        assert_eq!(
            refusal.location,
            "driver-steps[development/default].states.implement.steps[1].expect"
        );
    }

    /// `expect` and `output` are a command step's: an `llm` or `operator` step that writes one
    /// does not parse.
    #[test]
    fn expect_and_output_belong_to_a_command_step_only() {
        for fragment in [
            r#"{"kind":"llm","prompt":"p","judgement":"j","expect":"failure"}"#,
            r#"{"kind":"llm","prompt":"p","judgement":"j","output":"a.txt"}"#,
            r#"{"kind":"operator","prompt":"p","expect":"success"}"#,
            r#"{"kind":"operator","prompt":"p","output":"a.txt"}"#,
        ] {
            let outcome: Result<RawStep, _> = serde_json::from_str(fragment);
            assert!(outcome.is_err(), "{fragment} must not parse");
        }
    }

    /// A kept output is a file name of its own in the run directory: not blank, no separator, not
    /// hidden, and none of the run's own records, which a step must never overwrite.
    #[test]
    fn an_output_that_is_not_a_file_name_of_its_own_is_refused_at_its_path() {
        let kept = only_command(
            r#"{"implement":{"steps":[{"kind":"command","run":["git","log"],"output":"collected.txt"}]}}"#,
        );
        assert_eq!(kept.output.as_deref(), Some("collected.txt"));

        for (output, code) in [
            ("", ValidationCode::EmptyDeclaration),
            ("   ", ValidationCode::EmptyDeclaration),
            ("logs/collected.txt", ValidationCode::UnsupportedConstruct),
            ("..\\up", ValidationCode::UnsupportedConstruct),
            ("..", ValidationCode::UnsupportedConstruct),
            (".hidden", ValidationCode::UnsupportedConstruct),
            ("cursor.json", ValidationCode::UnsupportedConstruct),
            ("launch.json", ValidationCode::UnsupportedConstruct),
            ("transcripts", ValidationCode::UnsupportedConstruct),
        ] {
            let states = format!(
                r#"{{"implement":{{"steps":[{{"kind":"command","run":["git","log"],"output":{}}}]}}}}"#,
                serde_json::to_string(output).expect("a string serializes")
            );
            let errors = read(&map_json("adp/default/1", &states)).expect_err("refused");
            assert_eq!(errors.len(), 1, "`{output}`: {errors}");
            let refusal = &errors.as_slice()[0];
            assert_eq!(refusal.code, code, "`{output}`: {errors}");
            assert_eq!(
                refusal.location,
                "driver-steps[development/default].states.implement.steps[0].output",
                "`{output}`"
            );
        }
        for reserved in CommandStep::RESERVED_OUTPUTS {
            assert!(
                !reserved.starts_with('.'),
                "`{reserved}` is already refused as hidden; the list is for the visible names"
            );
        }
    }

    /// The step-log rule is the log writer's form, `<state>-<index>-<attempt>.log` in any ASCII
    /// case, and no wider: a name merely ending in `.log` is an output a map may keep.
    #[test]
    fn an_output_named_like_a_log_but_not_a_step_log_is_kept() {
        for refused in [
            "implement-0-1.log",
            "review-12-3.LOG",
            "a-b-0-1.log",
            "Implement-0-1.Log",
        ] {
            assert!(
                CommandStep::is_step_log_name(refused),
                "`{refused}` is a step log's name"
            );
        }
        for kept in [
            "notes.log",
            "implement-0.log",
            "implement-x-1.log",
            "-0-1.log",
            "0-1.log",
            "implement-0-1.log.txt",
        ] {
            assert!(
                !CommandStep::is_step_log_name(kept),
                "`{kept}` is no step log's name"
            );
            let states = format!(
                r#"{{"implement":{{"steps":[{{"kind":"command","run":["git","log"],"output":{}}}]}}}}"#,
                serde_json::to_string(kept).expect("a string serializes")
            );
            read(&map_json("adp/default/1", &states))
                .unwrap_or_else(|errors| panic!("`{kept}` is kept: {errors}"));
        }
    }

    /// A word is read once: a value filled in is text, so two values cannot spell a placeholder
    /// and an unclosed `{` in one cannot swallow a placeholder the map wrote after it.
    #[test]
    fn a_filled_value_is_never_read_again_for_placeholders() {
        let word = Word::parse("{input.a}{input.b} > {run_directory}/x {} {a: .b}");
        assert_eq!(
            word.placeholders().collect::<Vec<_>>(),
            ["input.a", "input.b", "run_directory"]
        );
        let given: BTreeMap<String, String> = [
            ("a".to_owned(), "{".to_owned()),
            ("b".to_owned(), "task}".to_owned()),
        ]
        .into();
        let filled = word
            .fill(|name| input_value(name, &given))
            .expect("both given");
        assert_eq!(filled, "{task} > {run_directory}/x {} {a: .b}");
        assert_eq!(filled.placeholders().collect::<Vec<_>>(), ["run_directory"]);
        let spawned = filled
            .fill(|name| Ok((name == "run_directory").then(|| "/run".to_owned())))
            .expect("the run directory is known");
        assert_eq!(spawned, "{task} > /run/x {} {a: .b}");
        assert_eq!(spawned.placeholders().count(), 0);
    }

    /// `{input.<name>}` is admitted in a command's words and record path and in an `llm` step's
    /// context, beside `{run_directory}`, and the map says which inputs its steps name.
    #[test]
    fn a_step_may_name_run_inputs_and_the_map_lists_the_ones_it_names() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["sh","-c","{input.repro}","{input.repro}"],
             "output":"collected.txt"},
            {"kind":"llm","prompt":"fix it","judgement":"what the fix is",
             "context":["{run_directory}/collected.txt","notes/{input.release-version}.md"]}]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let steps = map.steps_for(&StateId::new("implement").expect("a state"));
        assert_eq!(steps[0].inputs_named(), ["repro"], "each name once");
        assert_eq!(steps[1].inputs_named(), ["release-version"]);
        assert_eq!(
            map.inputs_named().into_iter().collect::<Vec<_>>(),
            ["release-version", "repro"]
        );

        // A context entry is a file the run is given, so only what names a file is expandable
        // there: the transcript of another session and the task document are refused by name.
        for placeholder in ["{transcript}", "{task}", "{input.Bad}x{nope}"] {
            let errors =
                only_llm_step(&format!(r#","context":["{placeholder}"]"#)).expect_err("refused");
            assert!(
                errors.contains(ValidationCode::UndeclaredReference),
                "{placeholder}: {errors}"
            );
        }
        // And a dotted placeholder other than an input is a misspelling, not text.
        let errors = read(&map_json(
            "adp/default/1",
            r#"{"implement":{"steps":[{"kind":"command","run":["x","{inptu.repro}"]}]}}"#,
        ))
        .expect_err("refused");
        assert!(
            errors.contains(ValidationCode::UndeclaredReference),
            "{errors}"
        );
    }

    /// Acceptance 5, decided from the map and the run's inputs: a placeholder naming an input the
    /// run was not given is refused, naming it, at the step that names it.
    #[test]
    fn an_input_the_run_was_not_given_is_refused_naming_it() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["sh","-c","{input.repro}"]},
            {"kind":"llm","prompt":"p","judgement":"j","context":["{input.notes}"]}]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let given: BTreeMap<String, String> = [("notes".to_owned(), "n.md".to_owned())].into();
        let errors = map.check_inputs(&given);
        assert_eq!(errors.len(), 1, "{errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(refusal.code, ValidationCode::UndeclaredReference);
        assert_eq!(
            refusal.location,
            "driver-steps[development/default].states.implement.steps[0].run"
        );
        assert!(
            refusal.to_string().contains("{input.repro}"),
            "the refusal names the placeholder: {refusal}"
        );

        let both: BTreeMap<String, String> = [
            ("notes".to_owned(), "n.md".to_owned()),
            ("repro".to_owned(), "cargo test".to_owned()),
        ]
        .into();
        assert!(map.check_inputs(&both).is_empty());
    }

    /// An input's value is text: one that would itself be expanded as a placeholder is refused,
    /// so a value can never name a file or a document the map did not.
    #[test]
    fn an_input_value_that_names_a_placeholder_is_refused() {
        let map = read(&map_json(
            "adp/default/1",
            r#"{"implement":{"steps":[{"kind":"command","run":["echo","{input.v}"]}]}}"#,
        ))
        .expect("valid");
        let given: BTreeMap<String, String> = [("v".to_owned(), "{task}".to_owned())].into();
        let errors = map.check_inputs(&given);
        assert!(
            errors.contains(ValidationCode::UnsupportedConstruct),
            "{errors}"
        );
        let braces: BTreeMap<String, String> = [("v".to_owned(), "{a: .b}".to_owned())].into();
        assert!(
            map.check_inputs(&braces).is_empty(),
            "braces that name nothing are text, as in a step's own words"
        );
    }

    /// The driver hands a step its inputs and run directory filled in, in one pass: a value is
    /// never read a second time for placeholders.
    #[test]
    fn inputs_and_the_run_directory_are_filled_in_once() {
        let states = r#"{"implement":{"steps":[
            {"kind":"command","run":["sh","-c","{input.repro} --at {run_directory}"],
             "evidence":{"kind":"trace_conformance","verifier":"trace-checker",
                         "record":"{run_directory}/{input.repro}.yaml"}},
            {"kind":"llm","prompt":"p","judgement":"j",
             "context":["{run_directory}/collected.txt","{input.repro}"]}]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let steps = map.steps_for(&StateId::new("implement").expect("a state"));
        let given: BTreeMap<String, String> = [("repro".to_owned(), "t".to_owned())].into();

        let Step::Command(command) = &steps[0] else {
            panic!("the first step is a command");
        };
        let filled = command.with_inputs(&given).expect("every input is given");
        assert_eq!(filled.run[2], "t --at {run_directory}");
        assert_eq!(
            filled
                .evidence
                .and_then(|mapping| mapping.record)
                .as_ref()
                .map(Word::as_str),
            Some("{run_directory}/t.yaml"),
            "the run directory is the executor's to fill in for a command step"
        );

        let Step::Llm(llm) = &steps[1] else {
            panic!("the second step is the llm step");
        };
        let resolved = llm
            .with_context_resolved("/runs/T-1/1", &given)
            .expect("every input is given");
        assert_eq!(resolved.context, ["/runs/T-1/1/collected.txt", "t"]);

        assert!(
            command.with_inputs(&BTreeMap::new()).is_err(),
            "an input nobody gave is not filled in with nothing"
        );
    }

    #[test]
    fn an_input_name_is_a_lowercase_word() {
        for good in ["repro", "release-version", "v2", "a_b"] {
            assert!(valid_input_name(good), "{good}");
        }
        for bad in ["", "Repro", "2v", "-a", "a.b", "a b", "a=b"] {
            assert!(!valid_input_name(bad), "{bad}");
        }
    }

    /// Two command steps writing `a.txt`, in a state that opts into concurrency or not.
    fn shared_output_states(concurrent: &str) -> String {
        format!(
            r#"{{"diagnose":{{{concurrent}"steps":[
                {{"kind":"command","run":["sh","-c","echo one"],"output":"a.txt"}},
                {{"kind":"command","run":["sh","-c","echo two"],"output":"a.txt"}}]}}}}"#
        )
    }

    /// ESS `aep.drive.DeclareConcurrentOutput`: the pairwise comparison that fills `shared_with`
    /// is the loader's, and this pins it.
    #[test]
    fn a_concurrent_state_refuses_two_command_steps_writing_one_output() {
        let errors = read(&map_json(
            "adp/default/1",
            &shared_output_states(r#""concurrent":true,"#),
        ))
        .expect_err("two steps of one batch would race for one file");
        assert_eq!(errors.len(), 1, "{errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(
            refusal.code,
            ValidationCode::UnsupportedConstruct,
            "{errors}"
        );
        assert_eq!(
            refusal.location,
            "driver-steps[development/default].states.diagnose.steps[1].output"
        );
        for path in [
            "driver-steps[development/default].states.diagnose.steps[0]",
            "driver-steps[development/default].states.diagnose.steps[1]",
        ] {
            assert!(
                refusal.message.contains(path),
                "the refusal names `{path}`: {}",
                refusal.message
            );
        }

        let serial = read(&map_json("adp/default/1", &shared_output_states("")))
            .expect("one step after another, the later write wins deterministically");
        assert!(!serial.states[&StateId::new("diagnose").unwrap()].concurrent);
    }

    #[test]
    fn a_concurrent_state_batches_its_consecutive_command_steps_between_barriers() {
        let states = r#"{"diagnose":{"concurrent":true,"steps":[
            {"kind":"command","run":["a"]},
            {"kind":"command","run":["b"]},
            {"kind":"operator","prompt":"look"},
            {"kind":"command","run":["c"]},
            {"kind":"command","run":["d"]},
            {"kind":"command","run":["e"]}]},
            "verify":{"steps":[
            {"kind":"command","run":["a"]},
            {"kind":"command","run":["b"]}]}}"#;
        let map = read(&map_json("adp/default/1", states)).expect("valid");
        let diagnose = StateId::new("diagnose").unwrap();
        let verify = StateId::new("verify").unwrap();
        assert!(map.states[&diagnose].concurrent);
        assert_eq!(map.batch_at(&diagnose, 0), 0..2);
        assert_eq!(
            map.batch_at(&diagnose, 2),
            2..3,
            "an operator step is a barrier"
        );
        assert_eq!(map.batch_at(&diagnose, 3), 3..6);
        assert_eq!(
            map.batch_at(&diagnose, 4),
            4..6,
            "from the cursor to the batch's end"
        );
        assert_eq!(
            map.batch_at(&verify, 0),
            0..1,
            "a state without the key runs serially"
        );
        assert_eq!(
            map.batch_at(&diagnose, 6),
            6..6,
            "past the end there is nothing"
        );
    }

    #[test]
    fn a_state_without_concurrent_digests_exactly_as_before() {
        let unwritten = read(&map_json("adp/default/1", ONE_COMMAND)).expect("valid");
        let written_false = r#"{"implement":{"concurrent":false,"steps":[
            {"kind":"command","run":["cargo","test"],
             "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}}]}}"#;
        let written_true = written_false.replace("false", "true");
        let explicit = read(&map_json("adp/default/1", written_false)).expect("valid");
        let opted = read(&map_json("adp/default/1", &written_true)).expect("valid");
        assert_eq!(unwritten.digest(), explicit.digest());
        assert_ne!(
            unwritten.digest(),
            opted.digest(),
            "a run that started serial must not resume concurrent"
        );
        let serialized = serde_json::to_string(&unwritten).expect("a map serializes");
        assert!(!serialized.contains("concurrent"), "{serialized}");
    }
}
