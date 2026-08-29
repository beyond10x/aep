//! Bounds, matchers and selectors: the whole of the expectation language, and no more of it.
//!
//! # Structured matchers, not an expression language
//!
//! Design decision **D2**, held here by the types. A matcher applies to **one named field** of a
//! tool's input or of its result. There are no boolean combinators, no arithmetic, and no nesting
//! beyond one field — because the growth path when that becomes insufficient is *not* a second
//! predicate language, it is to project trace facts into the namespace the protocol's existing
//! three-valued predicate language already reads, exactly as `infra-spec`'s `workload_predicate`
//! does. This repository has met that fork once and chose projection; inventing an expression
//! language here would take the other branch by accident.
//!
//! # `glob` is anchored, `regex` searches, and the difference is the thing to know
//!
//! Design § 3.4 lists both, and both ship. They are **not** two spellings of one matcher, and the
//! two ways to get this wrong are opposite:
//!
//! | | [`FieldMatcher::Glob`] | [`FieldMatcher::Regex`] |
//! |---|---|---|
//! | where it must hold | the **whole** field, anchored at both ends | **anywhere** in the field, like `contains:` |
//! | `*` | any run of characters | *repeat the previous item* — a bare `*` does not compile |
//! | `.` | a literal dot | any character |
//! | alternation, groups, classes, quantifiers | none | all of them |
//!
//! So `{glob: "a.c"}` matches the three characters `a.c` and nothing else, and `{regex: "a.c"}`
//! also matches `abc`. Neither meaning moved when the second one arrived: a specification written
//! against `glob` reads exactly as it always did, which
//! `a_glob_keeps_its_literal_reading_now_that_a_regex_matcher_exists_beside_it` pins.
//!
//! **The engine is `regex`, and the choice of engine is a guarantee rather than a convenience.**
//! It compiles to an automaton and never backtracks, so a match is linear in the length of the
//! subject — which is what [`glob_matches`] was hand-written to preserve, and what a checker
//! reading whatever a transcript happens to contain cannot do without. `AGENTS.md`
//! § *Dependencies* carries the rest of the decision, including why the alternatives were refused.
//!
//! A pattern is compiled **once, at validation**, so a specification that will not compile is
//! refused before a transcript is opened ([`crate::code::TraceCode::SpecUnusableMatcher`]) rather
//! than failing per-event in the middle of a report.
//!
//! # A bare number is not a bound
//!
//! `count: 1` cannot be read as "at least once" by one author and "exactly once" by the next,
//! because [`CountBound`] has no shorthand for it. And [`RangeBound`] — the bound over money,
//! ratios and derived durations — has **no `exactly` at all**, which is design decision **D6**
//! made structural: a cost expectation exists to catch a run that looped for forty minutes, not
//! to detect a 12% regression, and an equality over a float is a CI job people learn to ignore.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Serialize;

use crate::ir::{Recorded, ToolCall, ToolResult};

/// A bound over a whole number: a count, a token total, a duration in milliseconds.
///
/// At least one side is always set — a bound that bounds nothing is refused at validation
/// ([`crate::code::TraceCode::SpecInvalidExpectation`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct CountBound {
    /// The lowest acceptable value.
    pub at_least: Option<u64>,
    /// The highest acceptable value.
    pub at_most: Option<u64>,
    /// The only acceptable value. Never combined with the two above.
    pub exactly: Option<u64>,
}

impl CountBound {
    /// A bound that accepts exactly this value.
    pub fn exactly(value: u64) -> Self {
        Self {
            exactly: Some(value),
            ..Self::default()
        }
    }

    /// A bound that accepts this value or more.
    pub fn at_least(value: u64) -> Self {
        Self {
            at_least: Some(value),
            ..Self::default()
        }
    }

    /// A bound that accepts this value or less.
    pub fn at_most(value: u64) -> Self {
        Self {
            at_most: Some(value),
            ..Self::default()
        }
    }

    /// `true` when the observed value satisfies it.
    pub fn holds(self, value: u64) -> bool {
        if let Some(exactly) = self.exactly {
            return value == exactly;
        }
        self.at_least.is_none_or(|floor| value >= floor)
            && self.at_most.is_none_or(|ceiling| value <= ceiling)
    }

    /// `true` when it states no side at all.
    pub fn is_empty(self) -> bool {
        self.at_least.is_none() && self.at_most.is_none() && self.exactly.is_none()
    }

    /// `true` when it can never hold: a floor above its ceiling.
    pub fn is_unsatisfiable(self) -> bool {
        match (self.at_least, self.at_most) {
            (Some(floor), Some(ceiling)) => floor > ceiling,
            _ => false,
        }
    }
}

impl fmt::Display for CountBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.exactly, self.at_least, self.at_most) {
            (Some(exactly), _, _) => write!(f, "exactly {exactly}"),
            (_, Some(floor), Some(ceiling)) => write!(f, "between {floor} and {ceiling}"),
            (_, Some(floor), None) => write!(f, "at least {floor}"),
            (_, None, Some(ceiling)) => write!(f, "at most {ceiling}"),
            (None, None, None) => f.write_str("unbounded"),
        }
    }
}

/// A bound over a fractional quantity: money, a ratio, a utilization.
///
/// **No `exactly`, by construction** — design decision D6. Every quantity this bounds varies run
/// to run with model routing, cache state, service tier and load, and an equality over one is a
/// gate that goes red for reasons that have nothing to do with the change.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct RangeBound {
    /// The lowest acceptable value.
    pub at_least: Option<f64>,
    /// The highest acceptable value.
    pub at_most: Option<f64>,
}

impl RangeBound {
    /// A bound that accepts this value or less.
    pub fn at_most(value: f64) -> Self {
        Self {
            at_least: None,
            at_most: Some(value),
        }
    }

    /// A bound that accepts this value or more.
    pub fn at_least(value: f64) -> Self {
        Self {
            at_least: Some(value),
            at_most: None,
        }
    }

    /// `true` when the observed value satisfies it.
    pub fn holds(self, value: f64) -> bool {
        self.at_least.is_none_or(|floor| value >= floor)
            && self.at_most.is_none_or(|ceiling| value <= ceiling)
    }

    /// `true` when it states no side at all.
    pub fn is_empty(self) -> bool {
        self.at_least.is_none() && self.at_most.is_none()
    }

    /// `true` when it can never hold: a floor above its ceiling.
    pub fn is_unsatisfiable(self) -> bool {
        match (self.at_least, self.at_most) {
            (Some(floor), Some(ceiling)) => floor > ceiling,
            _ => false,
        }
    }
}

impl fmt::Display for RangeBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.at_least, self.at_most) {
            (Some(floor), Some(ceiling)) => write!(f, "between {floor} and {ceiling}"),
            (Some(floor), None) => write!(f, "at least {floor}"),
            (None, Some(ceiling)) => write!(f, "at most {ceiling}"),
            (None, None) => f.write_str("unbounded"),
        }
    }
}

/// A scalar an `equals` matcher compares against.
///
/// No float variant, deliberately: `equals: 0.62` over a recorded utilization is the equality D6
/// refuses, and offering the spelling would invite it. A fractional number is refused at
/// validation with the advice to write a bound instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ScalarValue {
    /// A boolean, such as `userModified: {equals: false}`.
    Bool(bool),
    /// A whole number.
    Integer(i64),
    /// A string.
    Text(String),
}

impl ScalarValue {
    /// `true` when a recorded value equals this, comparing like with like.
    ///
    /// Typed rather than textual: `equals: false` does not match the string `"false"`, because a
    /// harness that started recording a boolean as a string has changed the fact, and an
    /// expectation that kept passing across that change was never checking it.
    pub fn matches(&self, recorded: &Recorded) -> bool {
        match (self, recorded) {
            (Self::Bool(expected), Recorded::Bool(actual)) => expected == actual,
            (Self::Integer(expected), Recorded::Number(actual)) => {
                actual.as_i64() == Some(*expected)
            }
            (Self::Text(expected), Recorded::String(actual)) => expected == actual,
            _ => false,
        }
    }
}

impl fmt::Display for ScalarValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => write!(f, "{value}"),
            Self::Integer(value) => write!(f, "{value}"),
            Self::Text(value) => write!(f, "{value}"),
        }
    }
}

/// A regular expression, compiled once and carried beside the text it was written as.
///
/// The wrapper exists because [`FieldMatcher`] is `PartialEq` and `Serialize` and `regex::Regex`
/// is neither, and both answers matter:
///
/// * **Equality is over the pattern as written.** Two matchers are the same matcher when an author
///   typed the same thing, not when this build's automata happen to agree — nothing can decide the
///   latter, and a matcher that compared as equal for a reason no reader could see would make a
///   `PartialEq` on an expectation useless for saying *these two rows are the same row*.
/// * **It serializes as the pattern, never as the automaton.** The specification digest is taken
///   over the canonical JSON of the validated document
///   ([`digest_of_canonical`](crate::digest::digest_of_canonical)), so a specification's identity
///   stays its text: a `regex` release that compiles the same pattern differently must not rename
///   every run judged against it.
///
/// The compiled form is never exposed. `regex` stays out of this crate's public signatures —
/// [`Pattern::new`] returns the engine's complaint as a `String` — so a caller cannot come to
/// depend on the engine through a type this crate hands them.
#[derive(Debug, Clone)]
pub struct Pattern(regex::Regex);

impl Pattern {
    /// Compiles a pattern, or returns what the engine said was wrong with it.
    ///
    /// # Errors
    ///
    /// When the text is not a regular expression this engine can compile — an unclosed group, an
    /// unknown escape, or a pattern whose compiled form would exceed the engine's size limit. The
    /// message is the engine's own, which names the offending position.
    pub fn new(pattern: &str) -> Result<Self, String> {
        regex::Regex::new(pattern)
            .map(Self)
            .map_err(|error| error.to_string())
    }

    /// The pattern as the author wrote it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// `true` when the pattern is found **anywhere** in the subject.
    ///
    /// A search and not a whole-field test, which is the one thing to know about this matcher
    /// beside [`FieldMatcher::Glob`]: anchoring is `^` and `$`, spelled by the author, because a
    /// silently anchored regular expression would refuse `{regex: "rm\\s+-[rf]+"}` — a row about
    /// a command *somewhere* in a shell line, which is the shape almost every one of these is.
    #[must_use]
    pub fn is_match(&self, subject: &str) -> bool {
        self.0.is_match(subject)
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Pattern {}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Pattern {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// How one named field is compared.
///
/// Externally tagged — `{"contains": "protocol artifact new"}` — which is both the shape a
/// document writes and a shape serde can serialize a newtype variant into. An internally tagged
/// form would refuse a variant holding a bare string at run time rather than at compile time,
/// which is a failure a digest would only meet in production.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldMatcher {
    /// The whole field, character for character.
    Exact(String),
    /// A substring of the field.
    Contains(String),
    /// A glob over the field: `*` for any run of characters, `?` for one, everything else
    /// literal, and anchored at both ends.
    Glob(String),
    /// A regular expression searched for anywhere in the field. See [`Pattern`], and the table at
    /// the top of this module for how it differs from [`FieldMatcher::Glob`].
    Regex(Pattern),
    /// A scalar field, compared like with like.
    Equals(ScalarValue),
}

impl FieldMatcher {
    /// `true` when a recorded value satisfies it.
    ///
    /// The three textual matchers read a non-string field through [`text_of`], so
    /// `contains: "planning"` finds it inside a nested object without the document needing a
    /// second syntax for reaching in. `equals` never does that: it compares types.
    pub fn matches(&self, recorded: &Recorded) -> bool {
        match self {
            Self::Exact(expected) => text_of(recorded) == *expected,
            Self::Contains(expected) => text_of(recorded).contains(expected.as_str()),
            Self::Glob(pattern) => glob_matches(pattern, &text_of(recorded)),
            Self::Regex(pattern) => pattern.is_match(&text_of(recorded)),
            Self::Equals(expected) => expected.matches(recorded),
        }
    }

    /// `true` when a plain string satisfies it.
    ///
    /// Used where the subject is already text — a final assistant message, a plugin name — rather
    /// than a recorded JSON value.
    pub fn matches_text(&self, text: &str) -> bool {
        match self {
            // `equals` on a string is spelled differently from `exact` and means the same thing
            // here, because a bare text subject has no type to compare. On a *recorded* value the
            // two diverge — see [`Self::matches`] — which is why both spellings exist.
            Self::Exact(expected) | Self::Equals(ScalarValue::Text(expected)) => text == expected,
            Self::Contains(expected) => text.contains(expected.as_str()),
            Self::Glob(pattern) => glob_matches(pattern, text),
            Self::Regex(pattern) => pattern.is_match(text),
            Self::Equals(_) => false,
        }
    }
}

impl fmt::Display for FieldMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(value) => write!(f, "= {value:?}"),
            Self::Contains(value) => write!(f, "~ {value:?}"),
            Self::Glob(value) => write!(f, "glob {value:?}"),
            // A different word from `glob`, because a report a person reads to decide whether a
            // row is wrong has to say which of the two languages the pattern was written in.
            Self::Regex(value) => write!(f, "regex {:?}", value.as_str()),
            Self::Equals(value) => write!(f, "== {value}"),
        }
    }
}

/// A recorded value as text: a string as itself, anything else as its compact JSON.
///
/// One rule, written down, because the alternative is two readers disagreeing about whether
/// `contains: "true"` should find a boolean. It should: the textual matchers read the field as it
/// would be printed, and `equals` is the one that reads it as it was typed.
#[must_use]
pub fn text_of(recorded: &Recorded) -> String {
    match recorded {
        Recorded::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// Matches a glob against a subject: `*` any run of characters, `?` exactly one.
///
/// **Anchored at both ends**, which is the difference from [`FieldMatcher::Regex`] an author has
/// to know: `abc` does not match `abcd`, where the regular expression `abc` would.
///
/// Iterative with one backtrack point, so it is linear in practice and cannot blow up the way a
/// *backtracking* regular expression can — which matters for a checker that reads whatever a
/// transcript happens to contain. The engine beside it holds the same property by construction and
/// not by hand: `regex` compiles to an automaton and never backtracks, which is why it and not a
/// backtracking one is the crate this workspace took (`AGENTS.md` § *Dependencies*).
#[must_use]
pub fn glob_matches(pattern: &str, subject: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let subject: Vec<char> = subject.chars().collect();
    let (mut p, mut s) = (0usize, 0usize);
    let (mut star, mut resume) = (None, 0usize);

    while s < subject.len() {
        match pattern.get(p) {
            Some('*') => {
                star = Some(p);
                resume = s;
                p += 1;
            }
            Some('?') => {
                p += 1;
                s += 1;
            }
            Some(literal) if *literal == subject[s] => {
                p += 1;
                s += 1;
            }
            _ => match star {
                Some(at) => {
                    p = at + 1;
                    resume += 1;
                    s = resume;
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|character| *character == '*')
}

/// Which tool calls an expectation is about.
///
/// The scope half of every tool-family kind. It is deliberately *scoped* rather than global,
/// because the design's error family has a nuance that must not be papered over: **a refusal this
/// project designed is correct behaviour, not a failure.** `protocol artifact move` exits 1 when
/// the move is illegal, so a run in which the model asked for an illegal move, received the
/// refusal and relayed it behaved exactly right — and contains a failed tool call. A blanket
/// `tool.error_rate: 0` would forbid the plugin's own intended behaviour; a selector lets a
/// specification say *no failed `Read`* and leave the deliberate refusal alone.
/// # One claim, several tool names
///
/// The scope is a **set** of tool names rather than one, because a harness spells *put these bytes
/// in this file* several ways and an ordering claim is about the writing, not about which verb the
/// model reached for. The first live pilot is the evidence: a run asked to write a test before the
/// code did exactly that with Claude Code's `Edit`, and a selector naming `Write` alone reported
/// `never_occurred` — the checker saying *it did not happen* about work that visibly had.
///
/// A set widens **what can witness a claim** and never the claim. `Edit` before `Edit` is the same
/// ordering assertion as `Write` before `Write`; what changes is that the assertion is now
/// decidable against a run that used the other verb. The alternative — dropping the tool scope and
/// matching on `file_path` alone — would have been a genuine weakening, because `Read` carries a
/// `file_path` too and *read the test first* is not *wrote the test first*.
///
/// There is deliberately no way to say *this tool **or** that argument*: design decision **D2**
/// keeps the matcher language free of boolean combinators, and the growth path when one set of
/// names is not enough is a second expectation, not an expression language.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct CallSelector {
    /// The tools whose calls are in scope. Empty selects every tool.
    pub tools: BTreeSet<String>,
    /// The neutral operations whose calls are in scope. Empty selects every operation.
    ///
    /// The cross-harness way to say what a selector is about. `operations: [file.write]` selects a
    /// write on every harness that resolves one, where `tools: [Edit, NotebookEdit, Write,
    /// workspace_write, workspace_edit]` selected a write on the harnesses somebody had remembered
    /// to list — and silently selected nothing on the rest.
    ///
    /// # Written beside `tools:`, the two **union**
    ///
    /// They are two vocabularies for one scope, not two conditions on it. A row that means *this
    /// call is a write* is satisfied by a record that spells a write `Write` and by one that
    /// resolves it to `file.write`, and it has to be, because the two streams this repository reads
    /// do exactly one each: Claude Code's own `stream-json` carries tool names and no operations,
    /// and `metaharness.event/1` carries both. An intersection would make every such row
    /// undecidable against the first — which is how this was first written, and it would have
    /// turned the whole Claude arm `unk` while looking like a widening.
    ///
    /// This is not the boolean combinator design decision D2 refuses. D2 is about **matchers over
    /// field values** — no `and`, no `or`, no nesting inside `args:`. Enumerating a scope is what
    /// `tools:` has always done; this enumerates the same scope in a second vocabulary. `args:`
    /// still intersects with whatever the scope selected, unchanged.
    ///
    /// The cost is real and worth stating: a selector naming both keys cannot express *a write, and
    /// specifically Claude Code's*. Nothing in the corpus wants that, and a row that did would name
    /// the tools alone.
    ///
    /// **Skipped when empty**, and that is about identity rather than about bytes: a specification's
    /// digest is what a committed matrix names it by, and a selector that says nothing new must
    /// digest to what it always did. A field added to this struct that serialized as `[]` would
    /// silently re-identify every specification in the repository — the eval matrix's own
    /// `specifications[].digest` moved the moment this was added without the skip.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub operations: BTreeSet<String>,
    /// A matcher over what the call touched. Holds when **any** subject matches.
    ///
    /// The cross-harness way to say *which file*. `subject: {glob: "file:*/tests/*"}` selects a
    /// call against `src/tests/x.rs` on every harness that records a subject, where
    /// `args: {file_path: {glob: "*/tests/*"}}` reads one vendor's argument name and finds nothing
    /// on a wire that nests the path a level down under another.
    ///
    /// Any and not all, because a call may touch several things and the claim is about one of
    /// them: an ordering row asking *was a test written first* is satisfied by the call that wrote
    /// the test, whatever else the same call touched.
    ///
    /// The scheme prefix is part of the value, so a glob says which kind it means: `file:*` is a
    /// path and `proc:*` is a program. A matcher that dropped it would let a row about a file
    /// select a program with a similar name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<FieldMatcher>,
    /// Matchers over named arguments, all of which must hold.
    pub args: BTreeMap<String, FieldMatcher>,
}

impl CallSelector {
    /// A selector for one tool and nothing else.
    pub fn tool(name: impl Into<String>) -> Self {
        Self {
            tools: BTreeSet::from([name.into()]),
            operations: BTreeSet::new(),
            subject: None,
            args: BTreeMap::new(),
        }
    }

    /// A selector for any of several tools.
    pub fn tools<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tools: names.into_iter().map(Into::into).collect(),
            operations: BTreeSet::new(),
            subject: None,
            args: BTreeMap::new(),
        }
    }

    /// A selector for any call the harness resolved to one of these operations.
    ///
    /// The cross-harness spelling. See [`CallSelector::operations`].
    pub fn of_operations<I, S>(operations: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tools: BTreeSet::new(),
            operations: operations.into_iter().map(Into::into).collect(),
            subject: None,
            args: BTreeMap::new(),
        }
    }

    /// `true` when a call is in scope.
    ///
    /// An argument the call does not carry does **not** match: a matcher over `command` on a call
    /// that has no `command` is a claim about a field that is not there, and reading absence as a
    /// match would let a selector widen silently when a harness renames a field.
    pub fn matches(&self, call: &ToolCall) -> bool {
        let named = self.tools.contains(&call.name);
        // A call whose record names no operation never satisfies an operation scope. The harness
        // did not say what the call was, and reading silence as a match would let one harness's
        // unresolved calls satisfy a claim about writes.
        let resolved = call
            .operations
            .iter()
            .any(|operation| self.operations.contains(operation));
        let in_scope = match (self.tools.is_empty(), self.operations.is_empty()) {
            (true, true) => true,
            (false, true) => named,
            (true, false) => resolved,
            // The union. See the field's own documentation for why it is not an intersection.
            (false, false) => named || resolved,
        };
        if !in_scope {
            return false;
        }
        // A call whose record names no subject never satisfies a subject matcher, on the same rule
        // an absent argument does not match: the harness did not say what was touched, and reading
        // silence as a match would let a row about `*/tests/*` select a call about anything.
        if let Some(matcher) = &self.subject {
            let any = call
                .subjects
                .iter()
                .any(|subject| matcher.matches(&Recorded::String(subject.clone())));
            if !any {
                return false;
            }
        }
        self.args.iter().all(|(field, matcher)| {
            call.argument(field)
                .is_some_and(|value| matcher.matches(value))
        })
    }

    /// `true` when it selects everything — no tool name, no operation and no argument matcher.
    ///
    /// What `tool.absent` refuses. **An operation alone is a scope**, and leaving it out of this
    /// check refused `operations: [file.write]` as *forbids every tool call* — the one spelling of
    /// that row a reader would reach for on a harness whose write verb this document does not know.
    pub fn is_unscoped(&self) -> bool {
        self.tools.is_empty()
            && self.operations.is_empty()
            && self.subject.is_none()
            && self.args.is_empty()
    }
}

impl fmt::Display for CallSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // One name reads as itself, so every report written before the scope became a set says
        // what it always said. Several read as an alternation, which is what they are.
        let alternation =
            |names: &BTreeSet<String>| names.iter().cloned().collect::<Vec<_>>().join("|");
        // The operation is what a reader wants to see first when a selector has one: it is the
        // claim, where the tool names are the harnesses it happened to be spelled for.
        let scope = match (self.operations.len(), self.tools.len()) {
            (0, 0) => "any tool".to_owned(),
            (0, 1) => self.tools.iter().next().cloned().unwrap_or_default(),
            (0, _) => alternation(&self.tools),
            (_, 0) => alternation(&self.operations),
            (_, _) => format!(
                "{} or {}",
                alternation(&self.operations),
                alternation(&self.tools)
            ),
        };
        f.write_str(&scope).and_then(|()| {
            if self.args.is_empty() {
                Ok(())
            } else {
                let rendered: Vec<String> = self
                    .args
                    .iter()
                    .map(|(field, matcher)| format!("{field} {matcher}"))
                    .collect();
                write!(f, "({})", rendered.join(", "))
            }
        })
    }
}

/// Matchers over a tool result's named fields, all of which must hold.
///
/// Separate from [`CallSelector`] because `tool.called` matches the **request** and `tool.result`
/// matches what came back, and the two are different claims: a `Bash` call whose command matched
/// and whose `interrupted` is `true` satisfies the first and should fail the second.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct ResultMatcher {
    /// Matchers over named result fields.
    pub fields: BTreeMap<String, FieldMatcher>,
}

impl ResultMatcher {
    /// `true` when every named field is present and satisfies its matcher.
    pub fn matches(&self, result: &ToolResult) -> bool {
        self.fields
            .iter()
            .all(|(field, matcher)| result.field(field).is_some_and(|it| matcher.matches(it)))
    }

    /// The fields it names that a result does not carry.
    ///
    /// A missing field is not a failed match: the transcript did not say, and the verdict is
    /// `unk`. This is what lets the checker tell the two apart.
    pub fn absent_fields(&self, result: &ToolResult) -> Vec<String> {
        self.fields
            .keys()
            .filter(|field| result.field(field).is_none())
            .cloned()
            .collect()
    }

    /// `true` when it names no field.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl fmt::Display for ResultMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered: Vec<String> = self
            .fields
            .iter()
            .map(|(field, matcher)| format!("{field} {matcher}"))
            .collect();
        f.write_str(&rendered.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_bound_reads_exactly_before_either_side_so_one_number_has_one_meaning() {
        assert!(CountBound::exactly(0).holds(0));
        assert!(!CountBound::exactly(0).holds(1));
        assert!(CountBound::at_least(1).holds(9));
        assert!(!CountBound::at_least(1).holds(0));
        assert!(CountBound::at_most(3).holds(3));
        assert!(!CountBound::at_most(3).holds(4));
    }

    #[test]
    fn a_bound_with_a_floor_above_its_ceiling_is_unsatisfiable_and_says_so() {
        let bound = CountBound {
            at_least: Some(5),
            at_most: Some(2),
            exactly: None,
        };
        assert!(bound.is_unsatisfiable());
        assert!(!bound.holds(3), "nothing can satisfy it");
        assert!(RangeBound {
            at_least: Some(1.0),
            at_most: Some(0.5),
        }
        .is_unsatisfiable());
    }

    #[test]
    fn equals_compares_like_with_like_so_a_boolean_becoming_a_string_is_visible() {
        let matcher = FieldMatcher::Equals(ScalarValue::Bool(false));
        assert!(matcher.matches(&serde_json::json!(false)));
        assert!(
            !matcher.matches(&serde_json::json!("false")),
            "a harness that started recording a boolean as a string changed the fact"
        );
    }

    #[test]
    fn a_textual_matcher_reads_a_non_string_field_through_its_json_rendering() {
        let matcher = FieldMatcher::Contains("planning".to_owned());
        assert!(matcher.matches(&serde_json::json!({ "skill": "protocols:planning" })));
        assert!(!matcher.matches(&serde_json::json!({ "skill": "protocols:review" })));
    }

    #[test]
    fn a_glob_matches_the_paths_the_design_writes_and_refuses_the_ones_it_does_not() {
        assert!(glob_matches(
            "*/.engineering/planning/*.md",
            "/work/project/.engineering/planning/story/x.md"
        ));
        assert!(!glob_matches(
            "*/.engineering/planning/*.md",
            "/work/project/docs/plan/x.md"
        ));
        assert!(
            glob_matches("*", ""),
            "a lone star matches the empty string"
        );
        assert!(glob_matches("a?c", "abc"));
        assert!(!glob_matches("a?c", "ac"), "`?` is exactly one character");
        assert!(
            !glob_matches("abc", "abcd"),
            "a glob is anchored at both ends"
        );
    }

    #[test]
    fn a_regex_searches_the_field_where_a_glob_of_the_same_text_has_to_be_the_whole_of_it() {
        // The one difference between the two matchers an author has to hold in their head, and
        // the reason `regex` is not just "glob with more syntax". One subject, one spelling, two
        // answers — and the answers are what the table at the top of the module promises.
        let subject = Recorded::String("cargo test --workspace".to_owned());
        let searched = FieldMatcher::Regex(Pattern::new("test").expect("a pattern"));
        let anchored = FieldMatcher::Glob("test".to_owned());
        assert!(searched.matches(&subject));
        assert!(!anchored.matches(&subject));

        // And the metacharacter half: `.` is a literal in one language and any character in the
        // other, which is what a specification silently re-read from one to the other would lose.
        let dotted = Recorded::String("axc".to_owned());
        assert!(FieldMatcher::Regex(Pattern::new("^a.c$").expect("a pattern")).matches(&dotted));
        assert!(!FieldMatcher::Glob("a.c".to_owned()).matches(&dotted));
    }

    #[test]
    fn a_pattern_compares_and_serializes_by_the_text_it_was_written_as_and_not_by_its_automaton() {
        // Two patterns that accept exactly the same language and were not written the same way.
        // If either answer came off the compiled form, a specification's identity would be this
        // build's opinion about regular expressions rather than the document the author wrote.
        let written = Pattern::new("ab|ac").expect("a pattern");
        let equivalent = Pattern::new("a(b|c)").expect("a pattern");
        assert!(written.is_match("ab") && equivalent.is_match("ab"));
        assert_ne!(written, equivalent);
        assert_eq!(written, Pattern::new("ab|ac").expect("a pattern"));
        assert_eq!(
            serde_json::to_string(&FieldMatcher::Regex(written)).expect("a matcher serializes"),
            r#"{"regex":"ab|ac"}"#
        );
    }

    #[test]
    fn a_selector_over_an_argument_the_call_does_not_carry_does_not_match() {
        // The widening this refuses: if a harness renamed `command` tomorrow, reading the absent
        // field as a match would turn "every Bash call running the CLI" into "every Bash call".
        let call = ToolCall {
            call_id: None,
            name: "Bash".to_owned(),
            operations: Vec::new(),
            subjects: Vec::new(),
            joined_argv: None,
            input: BTreeMap::new(),
            input_bytes: 0,
            result_event: None,
        };
        let mut selector = CallSelector::tool("Bash");
        selector.args.insert(
            "command".to_owned(),
            FieldMatcher::Contains("protocol".to_owned()),
        );
        assert!(!selector.matches(&call));
        assert!(
            CallSelector::tool("Bash").matches(&call),
            "the tool name alone still selects it"
        );
    }

    fn call(name: &str, operations: &[&str]) -> ToolCall {
        ToolCall {
            call_id: None,
            name: name.to_owned(),
            operations: operations.iter().map(|op| (*op).to_owned()).collect(),
            subjects: Vec::new(),
            joined_argv: None,
            input: BTreeMap::new(),
            input_bytes: 0,
            result_event: None,
        }
    }

    #[test]
    fn an_operation_scope_selects_a_write_whatever_the_harness_calls_the_tool() {
        // The whole point. Two harnesses, two tool names, one selector — and neither name appears
        // in it, so a third harness needs no edit here.
        let selector = CallSelector::of_operations(["file.write"]);
        assert!(selector.matches(&call("Write", &["file.write"])));
        assert!(selector.matches(&call("tool_invoke", &["file.write"])));
        assert!(!selector.matches(&call("Read", &["file.read"])));
    }

    #[test]
    fn a_call_the_record_left_unresolved_never_satisfies_an_operation_scope() {
        // Silence is not a match. A harness that says nothing about what a call was must not have
        // its calls counted as writes, or the row would report a verdict nobody earned.
        assert!(!CallSelector::of_operations(["file.write"]).matches(&call("Write", &[])));
    }

    #[test]
    fn naming_both_vocabularies_selects_a_call_that_answers_to_either() {
        // The union, and the regression it exists for: written as an intersection, this row went
        // undecidable against every Claude Code transcript — which carries tool names and no
        // operations — while reading like a widening.
        let mut selector = CallSelector::of_operations(["file.write", "file.edit"]);
        selector.tools = BTreeSet::from(["Edit".to_owned(), "Write".to_owned()]);

        assert!(
            selector.matches(&call("Write", &[])),
            "a name-only stream still decides it"
        );
        assert!(
            selector.matches(&call("tool_invoke", &["file.write"])),
            "and so does a stream that resolves operations under a name nobody listed"
        );
        assert!(!selector.matches(&call("Read", &["file.read"])));
        assert!(!selector.matches(&call("apply_patch", &[])), "neither road");
    }

    #[test]
    fn an_argument_matcher_still_narrows_whatever_the_scope_selected() {
        // `args:` intersects, unchanged. The union is between the two spellings of the scope, not
        // between the scope and the arguments.
        let mut selector = CallSelector::of_operations(["file.write"]);
        selector
            .args
            .insert("path".to_owned(), FieldMatcher::Glob("*/src/*".to_owned()));

        let mut inside = call("tool_invoke", &["file.write"]);
        inside
            .input
            .insert("path".to_owned(), serde_json::json!("a/src/b.rs"));
        assert!(selector.matches(&inside));

        let mut outside = call("tool_invoke", &["file.write"]);
        outside
            .input
            .insert("path".to_owned(), serde_json::json!("a/docs/b.md"));
        assert!(!selector.matches(&outside));
    }

    #[test]
    fn a_selector_reads_back_naming_the_claim_before_the_harnesses_it_was_spelled_for() {
        assert_eq!(
            CallSelector::of_operations(["file.write"]).to_string(),
            "file.write"
        );
        let mut both = CallSelector::of_operations(["file.write"]);
        both.tools = BTreeSet::from(["Write".to_owned()]);
        assert_eq!(both.to_string(), "file.write or Write");
    }

    #[test]
    fn a_result_matcher_separates_a_field_that_disagrees_from_a_field_that_is_absent() {
        let mut fields = BTreeMap::new();
        fields.insert("userModified".to_owned(), serde_json::json!(true));
        let result = ToolResult {
            call_id: None,
            is_error: None,
            content_bytes: 0,
            content: None,
            fields,
        };
        let mut matcher = ResultMatcher::default();
        matcher.fields.insert(
            "userModified".to_owned(),
            FieldMatcher::Equals(ScalarValue::Bool(false)),
        );
        assert!(!matcher.matches(&result), "the field disagrees");
        assert!(
            matcher.absent_fields(&result).is_empty(),
            "and it is present, which is what makes this a gap rather than an unknown"
        );

        let mut missing = ResultMatcher::default();
        missing.fields.insert(
            "interrupted".to_owned(),
            FieldMatcher::Equals(ScalarValue::Bool(false)),
        );
        assert_eq!(missing.absent_fields(&result), vec!["interrupted"]);
    }
}
