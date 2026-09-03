//! Every published pattern, **evaluated** rather than paraphrased.
//!
//! A published pattern is the second definition of a rule whose first definition is a constructor,
//! and a document is valid to an editor by the first and valid to the loader by the second. So the
//! property worth stating is that the two agree on every input — and the only way to state it is
//! to *run* the pattern, which is what the interpreter below is for. This workspace carries no
//! regular-expression engine, deliberately (`AGENTS.md`), so it carries this one, in a test, where
//! its narrowness is a feature.
//!
//! Since `story:workflow-id-pattern-numeric-tail` the patterns are composed rather than copied:
//! `aep_domain::identifier_pattern!` holds one body per charset and `WorkflowId`, `StepMapId`,
//! `PinnedWorkflowRef` and the versioned references all `concat!` theirs from it. Sharing a body
//! is not the same as being right about it, and it does not make composition sound either —
//! stripping anchors off a constant that later gains a top-level alternation (`^a|legacy$`) yields
//! `^(workflow:)?a|legacy/[1-9][0-9]*$`, which matches every string beginning with `a`, while an
//! `assert_eq!` on the two texts stays green because both sides moved together. Measured: with
//! that alternation planted, every unit test in this crate passed. The cases here are stated over
//! behaviour, so they do not.
//!
//! The interpreter is deliberately narrow — anchors, literals, classes, groups, alternation, `?`,
//! `*`, `+` — and **panics** on anything else, including a `^` or `$` anywhere but the ends. A
//! pattern this file cannot read fails loudly instead of being silently mis-evaluated.

use aep_domain::artifact::ArtifactStatus;
use aep_domain::domain_event::DomainEventType;
use aep_domain::entity::{ActorRef, EntityLocator, EntityType};
use aep_domain::facts::{FactPath, FactPattern};
use aep_domain::ids::{
    ApprovalId, AuditId, ClaimId, CommandId, CorrelationId, EventId, EvidenceId, ExecutionId,
    IdempotencyKey, ObligationId, PhaseId, PrincipleId, ProfileId, ProtocolId, ProviderId,
    RelationId, RepositoryRef, RequestId, ServiceId, StateId, SubjectRef, TaskId, ToolRef,
    WorkflowId,
};
use aep_domain::version::{PrincipleRef, ProfileVersionedRef, ProtocolRef, WorkflowRef};
use aep_driver_spec::map::{RawStepMap, StepMap, StepMapId};
use aep_driver_spec::pin::PinnedWorkflowRef;

// --- a very small regular-expression interpreter ---------------------------------------------

#[derive(Debug, Clone)]
enum Node {
    Empty,
    Char(char),
    Any,
    Class {
        negated: bool,
        items: Vec<(char, char)>,
    },
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat {
        node: Box<Node>,
        min: u32,
        max: Option<u32>,
    },
}

struct Parser<'a> {
    chars: Vec<char>,
    at: usize,
    whole: &'a str,
}

impl<'a> Parser<'a> {
    fn new(body: &'a str) -> Self {
        Self {
            chars: body.chars().collect(),
            at: 0,
            whole: body,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.at += 1;
        }
        c
    }

    fn alt(&mut self) -> Node {
        let mut branches = vec![self.concat()];
        while self.peek() == Some('|') {
            self.bump();
            branches.push(self.concat());
        }
        if branches.len() == 1 {
            branches.pop().expect("one branch")
        } else {
            Node::Alt(branches)
        }
    }

    fn concat(&mut self) -> Node {
        let mut parts = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            parts.push(self.repeat());
        }
        match parts.len() {
            0 => Node::Empty,
            1 => parts.pop().expect("one part"),
            _ => Node::Concat(parts),
        }
    }

    fn repeat(&mut self) -> Node {
        let mut node = self.atom();
        loop {
            let (min, max) = match self.peek() {
                Some('?') => (0, Some(1)),
                Some('*') => (0, None),
                Some('+') => (1, None),
                Some('{') => panic!(
                    "this interpreter does not read counted repetition; pattern: {}",
                    self.whole
                ),
                _ => break,
            };
            self.bump();
            node = Node::Repeat {
                node: Box::new(node),
                min,
                max,
            };
        }
        node
    }

    fn atom(&mut self) -> Node {
        match self.bump() {
            Some('(') => {
                // `(?:` and friends are not read rather than guessed at.
                assert!(
                    self.peek() != Some('?'),
                    "this interpreter does not read a `(?...)` group; pattern: {}",
                    self.whole
                );
                let inner = self.alt();
                assert_eq!(
                    self.bump(),
                    Some(')'),
                    "unbalanced `(` in pattern: {}",
                    self.whole
                );
                inner
            }
            Some('[') => self.class(),
            Some('.') => Node::Any,
            Some('\\') => Node::Char(
                self.bump()
                    .unwrap_or_else(|| panic!("a trailing backslash in pattern: {}", self.whole)),
            ),
            Some(c @ ('^' | '$')) => panic!(
                "`{c}` is anchored somewhere other than the ends of the pattern, which this \
                 interpreter refuses to guess at: {}",
                self.whole
            ),
            Some(c) => Node::Char(c),
            None => Node::Empty,
        }
    }

    fn class(&mut self) -> Node {
        let negated = self.peek() == Some('^');
        if negated {
            self.bump();
        }
        let mut items = Vec::new();
        loop {
            let c = self
                .bump()
                .unwrap_or_else(|| panic!("unterminated `[` in pattern: {}", self.whole));
            if c == ']' {
                break;
            }
            if self.peek() == Some('-') && self.chars.get(self.at + 1).copied() != Some(']') {
                self.bump();
                let end = self
                    .bump()
                    .unwrap_or_else(|| panic!("unterminated range in pattern: {}", self.whole));
                items.push((c, end));
            } else {
                items.push((c, c));
            }
        }
        Node::Class { negated, items }
    }
}

fn walk(node: &Node, s: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
    match node {
        Node::Empty => k(i),
        Node::Char(c) => i < s.len() && s[i] == *c && k(i + 1),
        Node::Any => i < s.len() && k(i + 1),
        Node::Class { negated, items } => {
            i < s.len()
                && (items.iter().any(|(lo, hi)| s[i] >= *lo && s[i] <= *hi) != *negated)
                && k(i + 1)
        }
        Node::Concat(parts) => concat(parts, s, i, k),
        Node::Alt(branches) => branches.iter().any(|branch| walk(branch, s, i, k)),
        Node::Repeat { node, min, max } => repeat(node, 0, *min, *max, s, i, k),
    }
}

fn concat(parts: &[Node], s: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
    match parts.split_first() {
        None => k(i),
        Some((head, tail)) => walk(head, s, i, &mut |j| concat(tail, s, j, k)),
    }
}

fn repeat(
    node: &Node,
    done: u32,
    min: u32,
    max: Option<u32>,
    s: &[char],
    i: usize,
    k: &mut dyn FnMut(usize) -> bool,
) -> bool {
    if done >= min && k(i) {
        return true;
    }
    if max.is_some_and(|cap| done >= cap) {
        return false;
    }
    // `j > i` keeps a body that can match nothing from looping forever. Every repeated body in
    // these patterns consumes at least one character, and `the_interpreter_reads_the_constructs_\
    // these_patterns_are_made_of` is what says so.
    walk(node, s, i, &mut |j| {
        j > i && repeat(node, done + 1, min, max, s, j, k)
    })
}

/// Reads `pattern` into a tree, once, so a case that applies it to a corpus parses it once.
///
/// The pattern must be anchored at both ends, which every published one in this workspace is.
fn compile(pattern: &str) -> Node {
    let body = pattern
        .strip_prefix('^')
        .and_then(|rest| rest.strip_suffix('$'))
        .unwrap_or_else(|| panic!("a published pattern is anchored at both ends: {pattern}"));
    let mut parser = Parser::new(body);
    let node = parser.alt();
    assert_eq!(
        parser.at,
        parser.chars.len(),
        "the pattern was not read to the end, so this interpreter cannot judge it: {pattern}"
    );
    node
}

/// `true` when a compiled pattern matches the whole of `value`.
fn matches_compiled(node: &Node, value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    walk(node, &chars, 0, &mut |end| end == chars.len())
}

/// `true` when `pattern` matches the whole of `value`.
fn matches(pattern: &str, value: &str) -> bool {
    matches_compiled(&compile(pattern), value)
}

// --- the published rule, which is not the pattern alone ---------------------------------------

/// The whole of what a schema says about one string: the `pattern` an editor applies and the
/// `maxLength` beside it.
///
/// Reading the pattern alone is not reading the rule, and the difference is a whole class of
/// divergence. `validate` refuses an identifier over 200 characters
/// (`crates/govern/aep-domain/src/ids.rs`) and **no regular expression can say so**: bounding the length
/// of `[a-z][a-z0-9]*(-[a-z0-9]+)*` is a constraint on the *sum* of the parts, and a regex has no
/// way to write one — every unit of that pattern consumes one character or two, so a counted
/// repetition bounds the number of segments and not the number of characters. JSON Schema writes
/// it in a second keyword instead, and an editor applies both. So a case that wants to say *the
/// schema and the constructor agree* has to read both.
///
/// Read from the [`schemars::JsonSchema`] implementation, which is what `cargo xtask schema`
/// writes into `schemas/generated/` — not from a `PATTERN` constant, which is only what a type
/// *says* it publishes.
struct PublishedRule {
    /// The published `pattern`.
    pattern: String,
    /// The published `maxLength`, where there is one.
    max_length: Option<u32>,
    /// The pattern, read once.
    compiled: Node,
}

impl PublishedRule {
    /// The rule `T` publishes.
    fn read<T: schemars::JsonSchema>() -> Self {
        let mut generator = schemars::gen::SchemaGenerator::default();
        let schemars::schema::Schema::Object(object) = T::json_schema(&mut generator) else {
            panic!("expected a string schema for {}", T::schema_name());
        };
        let validation = object
            .string
            .expect("this type publishes string validation");
        let pattern = validation
            .pattern
            .expect("this type publishes a string pattern");
        let compiled = compile(&pattern);
        Self {
            pattern,
            max_length: validation.max_length,
            compiled,
        }
    }

    /// `true` when a document holding `value` here satisfies everything this schema says.
    fn accepts(&self, value: &str) -> bool {
        let length = u32::try_from(value.chars().count()).unwrap_or(u32::MAX);
        matches_compiled(&self.compiled, value) && self.max_length.is_none_or(|cap| length <= cap)
    }
}

// --- the corpus ------------------------------------------------------------------------------

/// Every string of length 1..=4 over the characters these rules turn on, plus the spellings worth
/// naming. Small enough to run in a moment, wide enough that a class of divergence cannot hide.
fn identifier_corpus() -> Vec<String> {
    let alphabet = ['a', 'z', '1', '0', '-', '.', '/'];
    let mut corpus: Vec<String> = Vec::new();
    let mut level: Vec<String> = vec![String::new()];
    for _ in 0..4 {
        let mut next = Vec::new();
        for prefix in &level {
            for c in alphabet {
                let mut candidate = prefix.clone();
                candidate.push(c);
                corpus.push(candidate.clone());
                next.push(candidate);
            }
        }
        level = next;
    }
    for extra in [
        "adp/default",
        "development.standard",
        "adp/2",
        "adp.2",
        "adp/22",
        "adp/2-3",
        "adp/1x",
        "a-0.0",
        "incident-standard",
    ] {
        corpus.push(extra.to_owned());
    }
    corpus
}

/// A corpus for the charset rules, wider than [`identifier_corpus`] in alphabet and shorter in
/// length: every string of length 1..=3 over the characters *all four* charset rules turn on.
///
/// [`identifier_corpus`] is built for `Charset::Dotted` alone, so it carries no upper case, no
/// `_` and no `:` — and those are exactly the characters that tell `Charset::Loose` and
/// `Charset::DottedSnake` apart from it. Three characters is enough for every divergence class
/// these rules have: a trailing separator is `a-`, a repeated one is `a--`, and a numeric tail is
/// `a/1`. The named spellings below are the ones worth reading in a failure message.
///
/// **Three characters is not enough for every rule in the census.** Round 1 added seven rows to
/// [`published_identifiers`] and four of them — `EntityType`, `EntityLocator`, `DomainEventType`
/// and `ActorRef` — have a shortest accepted string longer than three characters, so a
/// breadth-first enumeration to length 3 asks them nothing they can answer yes to. Measured before
/// the well-formed spellings below were added: those four accepted 0, 0, 0 and 1 of the 1135
/// strings here, which is a census row that cannot fail. The spellings at the end of the list are
/// one well-formed string per shape — `aep.design/v1`, `aep.design.created/v1`,
/// `ep://acme/payments/story/AUTH-142`, `human:alice` — so that every row is compared somewhere it
/// says yes, and [`every_rule_in_the_census_is_measured_by_the_corpus_the_census_runs_over`] is
/// what keeps that true for a row added tomorrow.
fn charset_corpus() -> Vec<String> {
    let alphabet = ['a', 'z', 'A', '0', '9', '-', '.', '/', '_', ':'];
    let mut corpus: Vec<String> = Vec::new();
    let mut level: Vec<String> = vec![String::new()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for prefix in &level {
            for c in alphabet {
                let mut candidate = prefix.clone();
                candidate.push(c);
                corpus.push(candidate.clone());
                next.push(candidate);
            }
        }
        level = next;
    }
    for extra in [
        "adp/default",
        "development.standard",
        "adp/2",
        "adp.2",
        "adp/22",
        "adp/2-3",
        "adp/1x",
        "adp/default/1",
        "a--b",
        "a-b-c",
        "incident-standard",
        "adversarial_verify",
        "before_implementation",
        "before_implementation/2",
        "state.adversarial_verify/1",
        "auth-api",
        "cargo-nextest",
        "AUTH-142",
        "AUTH_142",
        "acme/payments",
        "task:AUTH-142",
        "task:AUTH_142",
        "service:auth-api",
        "suite:unit",
        "a-b:c_d",
        // One well-formed string per shape the enumeration above cannot reach, so that the four
        // rows it left unmeasured are compared somewhere they say yes. Each of these is accepted
        // by both its pattern and its constructor, which is what makes adding them a measurement
        // rather than a finding.
        "aep.design/v1",
        "acme.payments.order/v2",
        "aep.design.created/v1",
        "acme.order.placed/v2",
        "ep://a/b/c/d",
        "ep://acme/payments/story/AUTH-142",
        "human:alice",
        "agent:claude",
        "system:engine",
    ] {
        corpus.push(extra.to_owned());
    }
    corpus
}

/// One published pattern, the name it is published under, and the code that really decides.
type Published = (&'static str, &'static str, fn(&str) -> bool);

/// One list of published types, read two ways.
///
/// A case that only wants the `pattern` takes the first function; a case that needs the whole
/// published rule — `pattern` and `maxLength` together — takes the second. One list, so the two
/// views cannot drift, which is the defect this whole file is about and would be a poor joke to
/// reintroduce in the file that tests for it.
macro_rules! published_table {
    ($patterns:ident, $rules:ident, [$(($name:ident, $accepts:expr)),+ $(,)?]) => {
        /// Each type below, with the pattern constant it names and the code that decides it.
        fn $patterns() -> Vec<Published> {
            vec![$((stringify!($name), $name::PATTERN, $accepts as fn(&str) -> bool)),+]
        }

        /// Each type below, with the rule its schema really publishes and the code that decides
        /// it.
        fn $rules() -> Vec<(&'static str, PublishedRule, fn(&str) -> bool)> {
            vec![$((
                stringify!($name),
                PublishedRule::read::<$name>(),
                $accepts as fn(&str) -> bool,
            )),+]
        }
    };
}

// The identifiers these two crates publish a pattern for that one constructor over one string
// decides, with that constructor.
//
// Listed one by one rather than sampled, because the defect this table is here for is *an
// identifier having two definitions*, and a sample of the identifiers is a sample of the defect.
//
// It is not, and no longer claims to be, every string these two crates publish a pattern for — its
// docstring claimed exactly that while omitting fourteen types, which is a sample wearing a
// census's clothes. Here is where the rest are decided, so a reader looking for a type can see
// whether anything evaluates it:
//
// * `TestSuite`, `TaskKind`, `ArtifactKind`, `Environment` and `Verifier` — open vocabularies, a
//   named list plus a constructor. Decided by `published_vocabularies` below, which reads their
//   patterns out of `JsonSchema` because they have no `PATTERN` constant.
// * `ProtocolRef`, `PrincipleRef`, `WorkflowRef`, `ProfileVersionedRef`, `PinnedWorkflowRef` —
//   decided by `published_references` below.
// * `ArtifactId` and `ArtifactRef` — **not decided anywhere, and diverging today.**
//   `ArtifactId::new` accepts any of `-`, `_`, `.` and `/` anywhere in either half, including
//   first; `ArtifactId::PATTERN` requires each half to *start* with `[A-Za-z0-9]`. Measured, both
//   sides: `_a:b` and `.a:b` are taken by the constructor and refused by the pattern, and — against
//   the report that sent this here — `design:auth_flow` and `a-:b` are taken by **both**, because
//   `_` is inside the pattern's character class. So the divergence is a leading separator and not
//   an underscore, and it runs schema-stricter-than-loader: an editor marks a document invalid
//   that would load.
//
//   **The class is a leading separator in *either* half, not only the first.** The sentence above
//   named the first half alone and that was too narrow: `a:.`, `a:-`, `a:_` and `a:/` diverge the
//   same way, because the pattern's second half is anchored on `[A-Za-z0-9]` exactly as its first
//   is. Re-measured over 56,594 + 4,687 strings against Python `re` rather than the interpreter
//   below: 601 divergences, every one of them schema-stricter. It is the same class of defect as
//   this file's subject and it is *not* fixed here, because the story names neither type and a
//   second fix hiding inside this one is how a change stops being reviewable. Reported for its own
//   unit.
// * `EntityId`, `VersionedEntityRef`, `SpecDigest` and `TranscriptDigest` — their patterns use
//   counted repetition (`{12,128}`, `{16,64}`, `{64}`), which the interpreter above refuses by
//   design and *loudly*, so adding one here would panic rather than mislead.
// * `Horizon` (`crates/govern/aep-domain/src/time.rs`) — its pattern uses `\s`, which the interpreter
//   would read as the letter `s` and would **not** complain about. That is the one exclusion worth
//   being uncomfortable about: it is out because the instrument would silently mis-evaluate it,
//   and the honest fix is to teach the interpreter `\s` and a counted repetition, in a change
//   whose subject is the instrument rather than a pattern.
//
// That list was assembled by reading every `pattern = Some(` in the two crates' `src/`, and
// nothing checks that it stays complete: a type that starts publishing a pattern tomorrow appears
// in no table and fails no case. Making the census checkable needs the generated schemas or a
// source scan, and either is its own unit.
//
// **Only a source scan will do, and here is the measurement that says so** — recorded here so the
// next reader does not re-derive it. Walking every object carrying a `"pattern"` key across the
// sixteen files of `schemas/generated/` yields **40** sites: 37 named `definitions` plus
// `Field/properties/name`, `Horizon/oneOf/1` and `Version/oneOf/0`. Six of those are `ess-domain`'s
// (`FormatVersion`, `OutcomeName`, `QualifiedName`, `StateName`, and the two anonymous ones) and
// are legitimately outside this census, which is about `aep-domain` and `aep-driver-spec`. The
// problem is the other direction: **13 of the 42 rows below appear in no generated schema at all**
// — `ActorRef`, `AuditId`, `CommandId`, `CorrelationId`, `DomainEventType`, `EntityLocator`,
// `EntityType`, `EventId`, `IdempotencyKey`, `ProviderId`, `RelationId`, `RepositoryRef`,
// `RequestId` — because nothing that reaches a generated document holds one yet. A checker built
// on `schemas/generated/` would therefore certify a census that is missing a third of itself,
// including both types the open `EntityType`/`DomainEventType` divergence lives on. The 42 are the
// 32 identifiers here, the 5 references in `published_references` and the 5 open vocabularies in
// `published_vocabularies`.
published_table![
    published_identifiers,
    published_identifier_rules,
    [
        (ProtocolId, |v| ProtocolId::new(v).is_ok()),
        (PrincipleId, |v| PrincipleId::new(v).is_ok()),
        (ProfileId, |v| ProfileId::new(v).is_ok()),
        (WorkflowId, |v| WorkflowId::new(v).is_ok()),
        (StateId, |v| StateId::new(v).is_ok()),
        (PhaseId, |v| PhaseId::new(v).is_ok()),
        (ObligationId, |v| ObligationId::new(v).is_ok()),
        (ApprovalId, |v| ApprovalId::new(v).is_ok()),
        (ClaimId, |v| ClaimId::new(v).is_ok()),
        (ToolRef, |v| ToolRef::new(v).is_ok()),
        (ServiceId, |v| ServiceId::new(v).is_ok()),
        (ProviderId, |v| ProviderId::new(v).is_ok()),
        (RepositoryRef, |v| RepositoryRef::new(v).is_ok()),
        (CommandId, |v| CommandId::new(v).is_ok()),
        (RequestId, |v| RequestId::new(v).is_ok()),
        (CorrelationId, |v| CorrelationId::new(v).is_ok()),
        (IdempotencyKey, |v| IdempotencyKey::new(v).is_ok()),
        (AuditId, |v| AuditId::new(v).is_ok()),
        (EventId, |v| EventId::new(v).is_ok()),
        (RelationId, |v| RelationId::new(v).is_ok()),
        (TaskId, |v| TaskId::new(v).is_ok()),
        (EvidenceId, |v| EvidenceId::new(v).is_ok()),
        (ExecutionId, |v| ExecutionId::new(v).is_ok()),
        (SubjectRef, |v| SubjectRef::new(v).is_ok()),
        (StepMapId, |v| StepMapId::new(v).is_ok()),
        (ArtifactStatus, |v| ArtifactStatus::parse(v).is_ok()),
        (EntityType, |v| EntityType::parse(v).is_ok()),
        (EntityLocator, |v| EntityLocator::parse(v).is_ok()),
        (ActorRef, |v| ActorRef::parse(v).is_ok()),
        (DomainEventType, |v| DomainEventType::parse(v).is_ok()),
        (FactPath, |v| FactPath::new(v).is_ok()),
        (FactPattern, |v| FactPattern::new(v).is_ok()),
    ]
];

// Every versioned reference these two crates publish a pattern for, with its parser.
published_table![
    published_references,
    published_reference_rules,
    [
        (ProtocolRef, |v| v.parse::<ProtocolRef>().is_ok()),
        (PrincipleRef, |v| v.parse::<PrincipleRef>().is_ok()),
        (WorkflowRef, |v| v.parse::<WorkflowRef>().is_ok()),
        (ProfileVersionedRef, |v| v
            .parse::<ProfileVersionedRef>()
            .is_ok()),
        (PinnedWorkflowRef, |v| v.parse::<WorkflowRef>().is_ok_and(
            |reference| PinnedWorkflowRef::try_from(reference).is_ok()
        )),
    ]
];

/// `true` when the loader takes a step map pinned to `workflow`.
fn loads(workflow: &str) -> bool {
    let text = format!(
        r#"{{"format":"aep.driver-steps/1","id":"development/default",
            "workflow":"{workflow}","states":{{"implement":{{"steps":[]}}}}}}"#
    );
    serde_json::from_str::<RawStepMap>(&text).is_ok_and(|raw| StepMap::try_from(raw).is_ok())
}

// --- the cases -------------------------------------------------------------------------------

/// The interpreter reads the constructs these patterns are made of, and refuses the rest.
///
/// Without this the cases below could fail because the interpreter is wrong rather than because a
/// pattern is, which is a typo reported as a finding.
#[test]
fn the_interpreter_reads_the_constructs_these_patterns_are_made_of() {
    for (pattern, value, expected) in [
        ("^a$", "a", true),
        ("^a$", "b", false),
        ("^a$", "aa", false),
        ("^ab*$", "a", true),
        ("^ab*$", "abbb", true),
        ("^ab+$", "a", false),
        ("^ab?c$", "ac", true),
        ("^ab?c$", "abc", true),
        ("^(ab|cd)$", "cd", true),
        ("^(ab|cd)$", "ac", false),
        ("^[a-z0-9]+$", "a9", true),
        ("^[a-z0-9]+$", "A", false),
        ("^[^a]$", "b", true),
        ("^[^a]$", "a", false),
        ("^(a)?b$", "b", true),
        ("^a.c$", "axc", true),
        // Backtracking: the greedy `[a-z]*` must give a character back for `a$` to match.
        ("^[a-z]*a$", "zza", true),
        // The two published patterns, on spellings settled by hand.
        (WorkflowId::PATTERN, "adp/default", true),
        (WorkflowId::PATTERN, "adp-", false),
        (WorkflowId::PATTERN, "a--b", false),
        (WorkflowId::PATTERN, "Adp", false),
        // The numeric-tail rule, by hand: a bare integer as the last `.`/`/` component is a
        // version reference and not part of an id, and `WorkflowId::new` says the same. This row
        // read `true` while the published pattern was looser than the constructor.
        (WorkflowId::PATTERN, "adp/2", false),
        (WorkflowId::PATTERN, "adp.2", false),
        (WorkflowId::PATTERN, "adp/22", false),
        (WorkflowId::PATTERN, "adp/2-3", true),
        (WorkflowId::PATTERN, "adp/1x", true),
        (PinnedWorkflowRef::PATTERN, "adp/default/1", true),
        (PinnedWorkflowRef::PATTERN, "workflow:adp/default/1", true),
        (PinnedWorkflowRef::PATTERN, "adp/default", false),
        (PinnedWorkflowRef::PATTERN, "adp/default/0", false),
        (PinnedWorkflowRef::PATTERN, "adp-/1", false),
        (PinnedWorkflowRef::PATTERN, "adp/2/1", false),
    ] {
        assert_eq!(
            matches(pattern, value),
            expected,
            "the interpreter reads `{pattern}` wrongly on {value:?}"
        );
    }
}

/// The pin's pattern accepts `<id>/1` for exactly the identifiers the workflow pattern accepts.
///
/// This is what `assert_eq!` on the two strings in `pin.rs` cannot say. Concatenating the body of
/// one anchored pattern into another is sound only while that body has no top-level alternation;
/// the day `WorkflowId::PATTERN` gains one, the two strings still agree and the composed pattern
/// means something else entirely. Stated over the corpus, so it is behaviour and not spelling.
#[test]
fn the_pin_pattern_admits_exactly_the_identifiers_the_workflow_pattern_admits() {
    let divergent: Vec<String> = identifier_corpus()
        .into_iter()
        .filter(|id| {
            matches(WorkflowId::PATTERN, id)
                != matches(PinnedWorkflowRef::PATTERN, &format!("{id}/1"))
        })
        .collect();
    assert!(
        divergent.is_empty(),
        "the pin's pattern is not `{}` with a mandatory version — it disagrees on {} identifier(s), \
         the first few being {:?}",
        WorkflowId::PATTERN,
        divergent.len(),
        divergent.iter().take(8).collect::<Vec<_>>()
    );
}

/// The version group is required of every string the pattern accepts, not just spelled at its end.
///
/// `pin.rs` says this with `PATTERN.ends_with("/[1-9][0-9]*$")` and
/// `!PATTERN.contains("(/[1-9][0-9]*)?")` — two checks on the *text*. Both survive the composition
/// going wrong: give `WorkflowId::PATTERN` a top-level alternative and mechanically re-compose,
/// and the pin becomes `^(workflow:)?<body>|legacy/[1-9][0-9]*$`, whose first branch matches a bare
/// `adp/default`. The suffix is still spelled at the end; it is no longer required of anything.
/// Measured: with that alternation planted, every existing case about this pattern stays green.
#[test]
fn the_published_pattern_requires_a_version_of_every_string_it_accepts() {
    // The corpus is built from an alphabet that includes `/` and digits, so some of it already
    // carries a version tail — `a/1` is a pin, not a counterexample. Those are dropped here, and
    // `the_sample_of_unversioned_references_is_not_empty` keeps that filter from eating everything.
    let carries_a_version = |value: &str| {
        value.rsplit_once('/').is_some_and(|(head, tail)| {
            !head.is_empty()
                && tail.starts_with(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
                && tail.chars().all(|c| c.is_ascii_digit())
        })
    };
    let sample: Vec<String> = identifier_corpus()
        .into_iter()
        .flat_map(|id| [format!("workflow:{id}"), id])
        .chain(["adp/default", "adp/default/", "adp/default/0", "legacy"].map(str::to_owned))
        .filter(|value| !carries_a_version(value))
        .collect();
    assert!(
        sample.len() > 1_000,
        "the filter above ate the sample, so this case would pass on nothing: {}",
        sample.len()
    );
    let unversioned: Vec<String> = sample
        .into_iter()
        .filter(|value| matches(PinnedWorkflowRef::PATTERN, value))
        .collect();
    assert!(
        unversioned.is_empty(),
        "the published pattern accepts {} reference(s) with no major version, so the version group \
         is not required of them: {:?}\npublished pattern: {}",
        unversioned.len(),
        unversioned.iter().take(8).collect::<Vec<_>>(),
        PinnedWorkflowRef::PATTERN
    );
}

/// The direction nobody has tested: the schema must not be **stricter** than the loader either.
///
/// Both existing cases ask whether the schema says yes where the loader says no. The mirror image
/// is an editor marking a map invalid that would load perfectly — a tightening that overshoots —
/// and nothing says it must not happen.
#[test]
fn every_identifier_the_loader_takes_is_one_the_published_pattern_takes() {
    let refused_by_the_schema: Vec<String> = identifier_corpus()
        .into_iter()
        .filter(|id| WorkflowId::new(id.as_str()).is_ok())
        .filter(|id| !matches(PinnedWorkflowRef::PATTERN, &format!("{id}/1")))
        .collect();
    assert!(
        refused_by_the_schema.is_empty(),
        "the loader takes these workflow ids and the published pattern calls the pin invalid, so \
         an editor marks a map wrong that loads: {:?}",
        refused_by_the_schema.iter().take(8).collect::<Vec<_>>()
    );
}

// --- correction round 2 ------------------------------------------------------------------------
//
// `tests/pin_pattern_agrees_with_the_loader.rs` used to state the residue against a hand-written
// paraphrase of `PinnedWorkflowRef::PATTERN`, so it could not notice the pattern changing: with a
// correct upstream fix planted and the pin regenerated, it passed while asserting that the pattern
// accepts `adp/2/1`, which by then it did not. That file is gone and its one unique statement is
// here, where the constant can be read.

/// No pin the published pattern calls valid is one the loader refuses. **Inverted, not deleted.**
///
/// This case used to assert the opposite and count it: 183 pins that
/// `schemas/generated/driver-steps.schema.json` accepted and `StepMap` refused, all of them the
/// numeric-tail class — `adp/2/1`, `adp.2/1`, `adp/22/1` — because `WorkflowId::PATTERN` was
/// looser than `WorkflowId::new`. Its own message said what to do when the gap closed, and
/// `story:workflow-id-pattern-numeric-tail` closed it, so the count is gone and the property is
/// stated instead: acceptance line 3 of `story:driver-spec-crate` — *an editor cannot tell an
/// author a map is fine that the loader will refuse* — now holds for the pin.
///
/// It is inverted rather than deleted because the statement it was making is the statement worth
/// keeping; only its sign was a report of an open defect.
///
/// **Still open, and deliberately:** `adp/default/4294967296` matches this pattern and does not
/// load, because a major version is a `u32` (`crates/govern/aep-domain/src/version.rs`) and a JSON Schema
/// `pattern` cannot express an integer ceiling. It is out of scope by name in
/// `story:workflow-id-pattern-numeric-tail`, and the corpus below carries no version that large,
/// so this case is silent about it on purpose rather than by accident.
#[test]
fn no_pin_the_published_pattern_calls_valid_is_one_the_loader_refuses() {
    let mut pins: Vec<String> = identifier_corpus()
        .iter()
        .flat_map(|id| {
            [
                format!("{id}/1"),
                format!("workflow:{id}/1"),
                format!("{id}/2"),
            ]
        })
        .collect();
    // Every spelling a pin can take, named forms included, so this is the whole surface and not a
    // sample of it. The middle three are the class this story closed.
    for extra in [
        "adp/default/1",
        "adp/2/1",
        "adp.2/1",
        "adp/22/1",
        "adp/default/0",
    ] {
        pins.push(extra.to_owned());
    }
    let pattern = compile(PinnedWorkflowRef::PATTERN);
    let divergent: Vec<String> = pins
        .into_iter()
        .filter(|pin| matches_compiled(&pattern, pin) && !loads(pin))
        .collect();

    assert!(
        divergent.is_empty(),
        "the published schema accepts {} pin(s) the loader refuses, so an editor tells an author \
         a step map is fine that will not load. First few: {:?}\npublished pattern: {}",
        divergent.len(),
        divergent.iter().take(8).collect::<Vec<_>>(),
        PinnedWorkflowRef::PATTERN
    );
}

/// The numeric-tail rule **is** expressible as a `pattern`, and this is the body that shipped.
///
/// Recorded here because the argument for leaving the gap open was written down once as *"a
/// `pattern` cannot express it readably"*, which is false and makes a chosen decision read as a
/// forced one. The real reason was where the rule lives, not whether it can be written — and when
/// `story:workflow-id-pattern-numeric-tail` put it where it lives, this is the body it took.
///
/// Two statements, both over the corpus rather than over the text: the body agrees with
/// `WorkflowId::new`, and what `WorkflowId` publishes agrees with the body. The second one used to
/// say the opposite — that the published pattern accepted things this body refuses — which is what
/// an open defect looks like when it is written as a test.
#[test]
fn the_numeric_tail_rule_is_expressible_as_a_pattern() {
    const PROPOSED: &str = "^[a-z][a-z0-9]*(-[a-z0-9]+)*(([./][a-z0-9]+(-[a-z0-9]+)*)*[./]([a-z0-9]*[a-z][a-z0-9]*(-[a-z0-9]+)*|[0-9]+(-[a-z0-9]+)+))?$";

    let corpus = identifier_corpus();
    let proposed = compile(PROPOSED);
    let mismatches: Vec<&String> = corpus
        .iter()
        .filter(|id| matches_compiled(&proposed, id) != WorkflowId::new(id.as_str()).is_ok())
        .collect();
    assert!(
        mismatches.is_empty(),
        "the proposed body disagrees with `WorkflowId::new` on {} of {} strings: {:?}",
        mismatches.len(),
        corpus.len(),
        mismatches.iter().take(8).collect::<Vec<_>>()
    );

    // And it is what `WorkflowId` publishes — in behaviour, not in spelling, so the constant may
    // be reformulated or composed without this case having an opinion about how it is written.
    let published = compile(WorkflowId::PATTERN);
    let divergent: Vec<&String> = corpus
        .iter()
        .filter(|id| matches_compiled(&published, id) != matches_compiled(&proposed, id))
        .collect();
    assert!(
        divergent.is_empty(),
        "`WorkflowId::PATTERN` and the body written for \
         `story:workflow-id-pattern-numeric-tail` disagree on {} string(s): {:?}\npublished: {}",
        divergent.len(),
        divergent.iter().take(8).collect::<Vec<_>>(),
        WorkflowId::PATTERN
    );
}

// --- one identifier, one definition --------------------------------------------------------
//
// The two cases below are the general form of the defect the two above are the specific form of.
// `story:workflow-id-pattern-numeric-tail` says it in one line: *the defect is that one identifier
// has two definitions, and fixing one of two leaves the defect*. A constructor and a published
// pattern are those two definitions, for every identifier in these two crates and not only for the
// one the defect was found on.

/// Every published identifier pattern accepts exactly what its own constructor accepts.
///
/// An editor applies the pattern from `schemas/generated/`; the loader applies the constructor.
/// Every string the two disagree about is a document one of them calls valid and the other
/// refuses, which is invariant 1 inverted — and which is what `story:driver-spec-crate` found for
/// `PinnedWorkflowRef`, what `story:workflow-id-pattern-numeric-tail` found upstream of it for
/// `WorkflowId`, and what neither would have found for `ServiceId` or `ObligationId` because
/// nobody was looking there.
///
/// **What this case cannot say.** It reads the `pattern` and nothing else, and a pattern is not
/// the whole published rule. `validate` refuses an identifier longer than 200 characters, no
/// regular expression can express a length bound over a body whose units are one character or two,
/// and the schema says it in `maxLength` instead — so this corpus stays short and this case is
/// silent about length. The case that reads both keywords is
/// [`every_published_identifier_pattern_agrees_with_its_constructor_at_the_length_bound`].
///
/// The sentence this replaces argued the gap away: *"the divergence exists and is a tightening the
/// loader applies on top, not a document an editor calls valid and the loader rejects on a
/// spelling."* A 201-character identifier is exactly a document an editor called valid and the
/// loader rejected, so the argument was false, and a false argument in a doc comment is worse than
/// no comment — it is the sentence that stops the next reader looking.
#[test]
fn every_published_identifier_pattern_accepts_exactly_what_its_constructor_accepts() {
    let corpus = charset_corpus();
    let mut divergent: Vec<String> = Vec::new();
    for (name, pattern, accepts) in published_identifiers() {
        let compiled = compile(pattern);
        for value in &corpus {
            let by_pattern = matches_compiled(&compiled, value);
            let by_constructor = accepts(value);
            if by_pattern != by_constructor {
                divergent.push(format!(
                    "{name}: {value:?} — the published pattern says {by_pattern}, \
                     `{name}::new` says {by_constructor}"
                ));
            }
        }
    }
    assert!(
        divergent.is_empty(),
        "{} disagreement(s) between a published pattern and the constructor it is published for, \
         over {} strings. Each is a spelling an editor and the loader answer differently. First \
         few: {:#?}",
        divergent.len(),
        corpus.len(),
        divergent.iter().take(12).collect::<Vec<_>>()
    );
}

/// Every reference the published pattern accepts is one the parser accepts.
///
/// One direction, and the direction is the point: the failure that matters is a schema calling a
/// reference valid that will not load. The converse is not asserted here because two spellings the
/// parser takes are ones no `pattern` should: `adp/default/01`, a non-canonical major, and
/// `adp/default/4294967296`, which the parser refuses only when the `u32` overflows — the ceiling
/// `story:workflow-id-pattern-numeric-tail` puts out of scope by name, because a JSON Schema
/// `pattern` cannot express an integer bound.
#[test]
fn every_reference_the_published_pattern_accepts_is_one_the_parser_accepts() {
    let corpus = charset_corpus();
    let mut accepted = 0_usize;
    let mut divergent: Vec<String> = Vec::new();
    for (name, pattern, parses) in published_references() {
        let compiled = compile(pattern);
        for value in &corpus {
            if !matches_compiled(&compiled, value) {
                continue;
            }
            accepted += 1;
            if !parses(value) {
                divergent.push(format!("{name}: {value:?}"));
            }
        }
    }
    assert!(
        accepted > 100,
        "the published reference patterns accepted {accepted} of {} strings, which is too few for \
         this case to be saying anything",
        corpus.len()
    );
    assert!(
        divergent.is_empty(),
        "{} reference(s) the published pattern calls valid and the parser refuses, so an editor \
         tells an author a document is fine that will not load. First few: {:#?}",
        divergent.len(),
        divergent.iter().take(12).collect::<Vec<_>>()
    );
}

// --- adversarial round 1 ------------------------------------------------------------------------
//
// Added while attacking `story:workflow-id-pattern-numeric-tail`. Nothing above this line is
// changed: these state the same property the cases above state, over two input classes the two
// corpora cannot reach — a string longer than the bound `validate` itself draws, and the open
// vocabularies whose published pattern is still a hand-written paraphrase of `Charset::Kebab`.

use aep_domain::artifact::ArtifactKind;
use aep_domain::capability::Environment;
use aep_domain::evidence::TestSuite;
use aep_domain::task::TaskKind;
use aep_domain::verification::Verifier;

/// The pattern `schemars` publishes for `T` — the string `cargo xtask schema` writes into
/// `schemas/generated/`.
///
/// Read from the `JsonSchema` implementation rather than from a `PATTERN` constant, because the
/// types below have no `PATTERN` constant: their pattern is a literal inside `json_schema`, which
/// is the shape this change was meant to remove. One reader, [`PublishedRule::read`], so that a
/// case reading the pattern and a case reading the whole rule cannot come to disagree about where
/// either lives.
fn published_pattern_of<T: schemars::JsonSchema>() -> String {
    PublishedRule::read::<T>().pattern
}

/// One published open-vocabulary pattern, the name it is published under, and its parser.
type PublishedVocabulary = (&'static str, String, fn(&str) -> bool);

/// The open vocabularies: a named list, plus *any other identifier*, decided by a constructor.
///
/// Each of these publishes a pattern and parses with a constructor, which is the same two
/// definitions of one rule that `identifier_pattern!` exists to collapse. None of them is in
/// [`published_identifiers`], so nothing above evaluates them.
fn published_vocabularies() -> Vec<PublishedVocabulary> {
    vec![
        (
            "TestSuite",
            published_pattern_of::<TestSuite>(),
            (|value| TestSuite::parse(value).is_ok()) as fn(&str) -> bool,
        ),
        ("TaskKind", published_pattern_of::<TaskKind>(), |value| {
            TaskKind::parse(value).is_ok()
        }),
        (
            "ArtifactKind",
            published_pattern_of::<ArtifactKind>(),
            |value| ArtifactKind::parse(value).is_ok(),
        ),
        (
            "Environment",
            published_pattern_of::<Environment>(),
            |value| Environment::parse(value).is_ok(),
        ),
        ("Verifier", published_pattern_of::<Verifier>(), |value| {
            Verifier::parse(value).is_ok()
        }),
    ]
}

/// Every published open-vocabulary pattern accepts exactly what its own parser accepts.
///
/// The commit that introduced `identifier_pattern!` is titled *one definition per charset, so a
/// published pattern cannot drift from its constructor*. These five publish a pattern and decide
/// with a constructor, exactly as the twenty-five in [`published_identifiers`] do, and four of them
/// still carry the hand-written `[a-z][a-z0-9-]*` paraphrase that `SubjectRef`'s own doc comment
/// names as the historical defect — the one that puts `-` inside the character class instead of
/// between segments, and so calls a trailing or repeated hyphen valid that `PrincipleId::new` and
/// `PhaseId::new` refuse.
#[test]
fn every_published_open_vocabulary_pattern_accepts_exactly_what_its_parser_accepts() {
    let mut corpus = charset_corpus();
    // The aliases and named members each of these parsers writes down by hand. `end_to_end` is the
    // one worth reading twice: `TestSuite::parse` names it explicitly
    // (`crates/govern/aep-domain/src/evidence.rs:101`) and the published pattern has no `_` in it, so the
    // divergence runs the other way there — an editor marking a document invalid that loads.
    for extra in [
        "end_to_end",
        "end-to-end",
        "e2e",
        "in_review",
        "dev",
        "prod",
        "stage",
        "any",
        "*",
        "adr",
        "prd",
        "ess",
        "spec",
        "review",
        "fix",
        "bug",
        "feat",
        "test-runner",
        "human-review",
    ] {
        corpus.push(extra.to_owned());
    }
    let mut divergent: Vec<String> = Vec::new();
    for (name, pattern, accepts) in published_vocabularies() {
        let compiled = compile(&pattern);
        for value in &corpus {
            let by_pattern = matches_compiled(&compiled, value);
            let by_parser = accepts(value);
            if by_pattern != by_parser {
                divergent.push(format!(
                    "{name}: {value:?} — the published pattern says {by_pattern}, \
                     `{name}::parse` says {by_parser} (pattern: {pattern})"
                ));
            }
        }
    }
    assert!(
        divergent.is_empty(),
        "{} disagreement(s) between a published pattern and the parser it is published for, over \
         {} strings. Each is a spelling an editor and the loader answer differently. First few: \
         {:#?}",
        divergent.len(),
        corpus.len(),
        divergent.iter().take(12).collect::<Vec<_>>()
    );
}

/// The empty string and a one-character identifier, which neither corpus contains.
///
/// `identifier_corpus` and `charset_corpus` are both built from a `level` seeded with `""` that
/// only ever pushes candidates of length one or more, so the empty string — the first boundary
/// there is — is in neither. This case is the *green* half of the boundary: it is here so the red
/// one below cannot be read as "the boundary was never checked".
#[test]
fn the_published_pattern_and_the_constructor_agree_on_the_empty_string() {
    for (name, pattern, accepts) in published_identifiers() {
        let compiled = compile(pattern);
        for value in ["", "a", "a:a"] {
            assert_eq!(
                matches_compiled(&compiled, value),
                accepts(value),
                "{name} disagrees with its constructor on {value:?}"
            );
        }
    }
}

/// Every published identifier rule agrees with its constructor at the length bound `validate`
/// draws, which is 200 characters.
///
/// `validate` refuses an identifier longer than 200 characters (`crates/govern/aep-domain/src/ids.rs`),
/// and when this case was written no published rule said so, so the schema called a 201-character
/// identifier valid and the loader refused it. That is the acceptance statement of
/// `story:workflow-id-pattern-numeric-tail` — *"the published schema and `WorkflowId::new` agree
/// on every input"* — with a counterexample.
///
/// **It is asked of the rule, not of the pattern.** A JSON Schema `pattern` cannot express a
/// length bound over any of these bodies: every unit of `[a-z][a-z0-9]*(-[a-z0-9]+)*` consumes one
/// character or two, so a counted repetition bounds segments and not characters, and the exact
/// bound needs a constraint on their sum, which a regular expression has no way to write. The
/// keyword for it is `maxLength`; every rule here whose constructor draws a length bound now
/// carries one, and [`PublishedRule`] is what reads the pair. The seven whose constructors draw
/// none — `ArtifactStatus`, `EntityType`, `EntityLocator`, `ActorRef`, `DomainEventType`,
/// `FactPath` and `FactPattern` — publish no `maxLength` either, which is the agreement this case
/// asks for and not an omission.
///
/// **The residue, and why it is a residue.** `maxLength` bounds the *whole* string, and
/// `SubjectRef::new` bounds each *half* — a kind of at most 200 and an id of at most 200. Those
/// two rules are not the same rule and neither implies the other: `a:` followed by 201 characters
/// is 203 long and refused by the constructor, while a 200-character kind and a 200-character id
/// is 401 long and accepted by it. So no single `maxLength` is right, and JSON Schema has no
/// per-component length keyword to write the real rule with. That puts it in the same class as the
/// `u32` major-version ceiling, which `story:workflow-id-pattern-numeric-tail` puts out of scope by
/// name for the same reason — and it is pinned below rather than described, so that a new
/// divergence fails here and a fixed one does too.
///
/// **What the pinned list covers, and what it does not.** The `assert_eq!` below is a list over
/// four fixed inputs — `a×200`, `a×201`, `a:` + `a×201`, `a×201` + `:a` — and a list over four
/// inputs cannot bound a class. `a:` followed by 399 `a` is 401 characters, which is exactly the
/// published `maxLength`, and it is the same per-half residue; it appears in none of the four, so
/// a reader taking the pin as *the residue is these two spellings* is wrong. The class itself is
/// stated after the list, as a predicate over a grid of component lengths: every divergence there
/// must be a string the schema takes whose halves are not both inside
/// [`MAX_IDENTIFIER_LENGTH`] while the whole is inside [`SUBJECT_MAX_LENGTH`]. The list stays
/// because a named spelling in a failure message is worth more than a predicate; the predicate is
/// what makes it a bound.
#[test]
fn every_published_identifier_pattern_agrees_with_its_constructor_at_the_length_bound() {
    let long = "a".repeat(201);
    let at_bound = "a".repeat(200);
    let values = [
        ("a 200-character identifier", at_bound.clone()),
        ("a 201-character identifier", long.clone()),
        ("`a:` and a 201-character second half", format!("a:{long}")),
        ("a 201-character first half and `:a`", format!("{long}:a")),
    ];
    let mut divergent: Vec<String> = Vec::new();
    for (name, rule, accepts) in published_identifier_rules() {
        for (shape, value) in &values {
            let by_rule = rule.accepts(value);
            let by_constructor = accepts(value);
            if by_rule != by_constructor {
                divergent.push(format!(
                    "{name}: {shape} ({} characters) — the published rule says {by_rule}, the \
                     constructor says {by_constructor}",
                    value.len()
                ));
            }
        }
    }
    let (composite, single): (Vec<String>, Vec<String>) = divergent
        .into_iter()
        .partition(|divergence| divergence.starts_with("SubjectRef:"));
    assert!(
        single.is_empty(),
        "{} disagreement(s) at the 200-character bound, so an editor calls an identifier valid \
         that the loader refuses on length alone — or refuses one it would take. Every rule here \
         is a pattern over one charset plus a `maxLength`, and both are expressible, so there is \
         nothing here to record as out of scope. First few: {:#?}",
        single.len(),
        single.iter().take(12).collect::<Vec<_>>()
    );
    assert_eq!(
        composite,
        vec![
            "SubjectRef: `a:` and a 201-character second half (203 characters) — the published \
             rule says true, the constructor says false"
                .to_owned(),
            "SubjectRef: a 201-character first half and `:a` (203 characters) — the published \
             rule says true, the constructor says false"
                .to_owned(),
        ],
        "the per-component length residue is not what it was. A longer list is a new divergence \
         and is a defect; a shorter one means somebody found a way to publish a per-half bound, \
         and this expectation should shrink with it."
    );

    the_only_subject_ref_divergence_is_the_per_half_residue();
}

/// The class the pinned list in
/// [`every_published_identifier_pattern_agrees_with_its_constructor_at_the_length_bound`] is four
/// instances of, stated as a predicate so that a divergence in a shape nobody listed fails too.
///
/// `SubjectRef::new` bounds each half at [`MAX_IDENTIFIER_LENGTH`]; the schema bounds the whole
/// string at [`SUBJECT_MAX_LENGTH`]. Every legitimate divergence is therefore a subject the schema
/// takes with a half over the bound and a whole inside it, and nothing else — in particular,
/// nothing the schema *refuses* and the constructor takes, which would be the other direction and
/// is not a residue.
fn the_only_subject_ref_divergence_is_the_per_half_residue() {
    let subject = PublishedRule::read::<SubjectRef>();
    assert_eq!(
        subject.max_length,
        Some(u32::try_from(SUBJECT_MAX_LENGTH).expect("a short bound")),
        "`SubjectRef` does not publish two identifier bounds and a `:`, so the residue below is \
         being measured against the wrong number"
    );
    let lengths = [
        1,
        100,
        MAX_IDENTIFIER_LENGTH - 1,
        MAX_IDENTIFIER_LENGTH,
        MAX_IDENTIFIER_LENGTH + 1,
        300,
        399,
        400,
    ];
    let mut in_the_class = 0_usize;
    let mut outside_the_class: Vec<String> = Vec::new();
    for kind_length in lengths {
        for id_length in lengths {
            let value = format!("{}:{}", "a".repeat(kind_length), "a".repeat(id_length));
            let by_rule = subject.accepts(&value);
            let by_constructor = SubjectRef::new(value.as_str()).is_ok();
            if by_rule == by_constructor {
                continue;
            }
            let a_half_is_over =
                kind_length > MAX_IDENTIFIER_LENGTH || id_length > MAX_IDENTIFIER_LENGTH;
            if by_rule && a_half_is_over && value.len() <= SUBJECT_MAX_LENGTH {
                in_the_class += 1;
            } else {
                outside_the_class.push(format!(
                    "a {kind_length}-character kind and a {id_length}-character id ({} characters) \
                     — the published rule says {by_rule}, the constructor says {by_constructor}",
                    value.len()
                ));
            }
        }
    }
    assert!(
        outside_the_class.is_empty(),
        "{} divergence(s) between `SubjectRef`'s published rule and its constructor that are not \
         the per-half residue — a whole string inside the published bound of {SUBJECT_MAX_LENGTH} \
         with one half over {MAX_IDENTIFIER_LENGTH}. Each is a defect and not a thing JSON Schema \
         cannot say: {:#?}",
        outside_the_class.len(),
        outside_the_class
    );
    assert!(
        in_the_class > 0,
        "the grid found no residue at all, so the predicate above is passing on nothing and the \
         list it is meant to bound is unbounded again"
    );
}

/// Every published reference rule agrees with its parser at the length bound too.
///
/// The references were the half of the length gap nobody had a case for: `every_reference_the_\
/// published_pattern_accepts_is_one_the_parser_accepts` runs over a corpus of three-character
/// strings, so it says nothing about a 201-character identifier with `/1` after it — and
/// `crates/govern/aep-domain/src/version.rs` claims in a doc comment that this residue is pinned by name
/// here. It is, from here on, rather than only claimed.
///
/// A reference's rule now carries a `maxLength` of `prefix:` plus the identifier's own bound plus
/// `/` plus the ten digits a `u32` is written with. That is the honest bound for the whole string
/// and it is not the rule the parser applies, which bounds the *identifier component*: a
/// 201-character id is 201 characters and the whole-string bound is 220, so the schema takes it
/// and `FromStr` does not. Same class as the `u32` ceiling, same treatment — pinned, not described.
///
/// **What the pinned list covers, and what it does not.** The `assert_eq!` below is a list over
/// five fixed inputs, none of them carrying a `protocol:`/`principle:`/`workflow:`/`profile:`
/// prefix — so the whole prefixed half of the residue appears in none of its entries, and a reader
/// taking the list as *the residue is these eight spellings* is wrong. The class is stated after
/// the list, as a predicate over a grid of identifier lengths in all three spellings a reference
/// takes: bare, `<id>/1`, and `<prefix><id>/1`. Every divergence there must be a string inside the
/// prefix's own published bound whose identifier component is over [`MAX_IDENTIFIER_LENGTH`], and
/// nothing else. The list stays because a named spelling in a failure message is worth more than a
/// predicate; the predicate is what makes it a bound.
#[test]
fn every_published_reference_rule_agrees_with_its_parser_at_the_length_bound() {
    let long = "a".repeat(201);
    let at_bound = "a".repeat(200);
    let values = [
        ("a 200-character identifier", at_bound.clone()),
        ("a 201-character identifier", long.clone()),
        (
            "a 200-character identifier at `/1`",
            format!("{at_bound}/1"),
        ),
        ("a 201-character identifier at `/1`", format!("{long}/1")),
        (
            "a 400-character identifier at `/1`",
            format!("{}/1", "a".repeat(400)),
        ),
    ];
    let mut divergent: Vec<String> = Vec::new();
    for (name, rule, parses) in published_reference_rules() {
        for (shape, value) in &values {
            let by_rule = rule.accepts(value);
            let by_parser = parses(value);
            if by_rule != by_parser {
                divergent.push(format!(
                    "{name}: {shape} ({} characters) — the published rule says {by_rule}, the \
                     parser says {by_parser}",
                    value.len()
                ));
            }
        }
    }
    assert_eq!(
        divergent,
        vec![
            "ProtocolRef: a 201-character identifier at `/1` (203 characters) — the published rule says true, the parser says false"
                .to_owned(),
            "PrincipleRef: a 201-character identifier (201 characters) — the published rule says true, the parser says false"
                .to_owned(),
            "PrincipleRef: a 201-character identifier at `/1` (203 characters) — the published rule says true, the parser says false"
                .to_owned(),
            "WorkflowRef: a 201-character identifier (201 characters) — the published rule says true, the parser says false"
                .to_owned(),
            "WorkflowRef: a 201-character identifier at `/1` (203 characters) — the published rule says true, the parser says false"
                .to_owned(),
            "ProfileVersionedRef: a 201-character identifier (201 characters) — the published rule says true, the parser says false"
                .to_owned(),
            "ProfileVersionedRef: a 201-character identifier at `/1` (203 characters) — the published rule says true, the parser says false"
                .to_owned(),
            "PinnedWorkflowRef: a 201-character identifier at `/1` (203 characters) — the published rule says true, the parser says false"
                .to_owned(),
        ],
        "the per-component length residue on the references is not what it was. Every entry here \
         is a whole string inside the published `maxLength` whose *identifier component* is over \
         the bound `FromStr` applies to it, which JSON Schema has no keyword for. Anything else is \
         a new divergence and is a defect; a shorter list means the component bound became \
         expressible and this expectation should shrink with it."
    );

    the_only_reference_divergence_is_the_per_component_residue();
}

/// The class the pinned list in
/// [`every_published_reference_rule_agrees_with_its_parser_at_the_length_bound`] is instances of,
/// stated as a predicate over identifier lengths in every spelling a reference takes — including
/// the prefixed one, which the list does not carry at all.
///
/// Each rule's bound is spelled here from [`MAX_IDENTIFIER_LENGTH`] and
/// [`MAJOR_VERSION_DIGITS`] rather than read off the schema, so a wrong published bound moves one
/// side and not both.
fn the_only_reference_divergence_is_the_per_component_residue() {
    let mut in_the_class = 0_usize;
    let mut outside_the_class: Vec<String> = Vec::new();
    for (name, prefix, rule, parses) in [
        (
            "ProtocolRef",
            "protocol:",
            PublishedRule::read::<ProtocolRef>(),
            (|value: &str| value.parse::<ProtocolRef>().is_ok()) as fn(&str) -> bool,
        ),
        (
            "PrincipleRef",
            "principle:",
            PublishedRule::read::<PrincipleRef>(),
            |value| value.parse::<PrincipleRef>().is_ok(),
        ),
        (
            "WorkflowRef",
            "workflow:",
            PublishedRule::read::<WorkflowRef>(),
            |value| value.parse::<WorkflowRef>().is_ok(),
        ),
        (
            "ProfileVersionedRef",
            "profile:",
            PublishedRule::read::<ProfileVersionedRef>(),
            |value| value.parse::<ProfileVersionedRef>().is_ok(),
        ),
        (
            "PinnedWorkflowRef",
            "workflow:",
            PublishedRule::read::<PinnedWorkflowRef>(),
            |value| {
                value
                    .parse::<WorkflowRef>()
                    .is_ok_and(|reference| PinnedWorkflowRef::try_from(reference).is_ok())
            },
        ),
    ] {
        let bound = prefix.len() + MAX_IDENTIFIER_LENGTH + "/".len() + MAJOR_VERSION_DIGITS;
        assert_eq!(
            rule.max_length,
            Some(u32::try_from(bound).expect("a short bound")),
            "{name} does not publish its prefix plus an identifier at its bound plus `/` plus the \
             ten digits a `u32` is written with, so the residue below is being measured against \
             the wrong number"
        );
        for length in [
            1,
            MAX_IDENTIFIER_LENGTH - 1,
            MAX_IDENTIFIER_LENGTH,
            MAX_IDENTIFIER_LENGTH + 1,
            210,
            bound - "/1".len(),
            bound,
            bound + 1,
        ] {
            let identifier = "a".repeat(length);
            for value in [
                identifier.clone(),
                format!("{identifier}/1"),
                format!("{prefix}{identifier}/1"),
            ] {
                let by_rule = rule.accepts(&value);
                let by_parser = parses(&value);
                if by_rule == by_parser {
                    continue;
                }
                if by_rule && length > MAX_IDENTIFIER_LENGTH && value.len() <= bound {
                    in_the_class += 1;
                } else {
                    outside_the_class.push(format!(
                        "{name}: a {length}-character identifier written {value:?} ({} characters) \
                         — the published rule says {by_rule}, the parser says {by_parser}",
                        value.len()
                    ));
                }
            }
        }
    }
    assert!(
        outside_the_class.is_empty(),
        "{} divergence(s) between a published reference rule and its parser that are not the \
         per-component residue — a whole string inside that reference's own published bound whose \
         identifier component is over {MAX_IDENTIFIER_LENGTH}. Each is a defect and not a thing \
         JSON Schema cannot say: {:#?}",
        outside_the_class.len(),
        outside_the_class.iter().take(12).collect::<Vec<_>>()
    );
    assert!(
        in_the_class > 0,
        "the grid found no residue at all, so the predicate above is passing on nothing and the \
         list it is meant to bound is unbounded again"
    );
}

/// No pin the published rule calls valid is one the loader refuses — at the length bound, as far
/// as JSON Schema can say it.
///
/// The same statement as
/// [`no_pin_the_published_pattern_calls_valid_is_one_the_loader_refuses`], over the one input class
/// its corpus cannot reach: `identifier_corpus` builds nothing longer than twelve characters, so a
/// workflow id past the 200-character bound is outside it. When this case was written
/// `schemas/generated/driver-steps.schema.json` called every one of these pins valid and
/// `StepMap::try_from` refused two of them.
///
/// The pin's rule now carries a `maxLength` — `workflow:`, an identifier at its own bound, `/` and
/// a major version at the ten digits a `u32` is written with — and the first assertion is that it
/// does something: a pin past that bound is refused by the schema now and was not before.
///
/// **That first assertion used to be `x == x`, and did not say that.** It compared
/// `rule.accepts(pin)` against `length <= bound && matches(PinnedWorkflowRef::PATTERN, pin)`, which
/// is the definition of `PublishedRule::accepts` recomputed from the same two published values —
/// so it held whatever the published bound was. Demonstrated: inflating the bound to 10000 tripped
/// only the second assertion, never the first. It is stated against
/// [`PIN_MAX_LENGTH`] now — the same number spelled here from `MAX_IDENTIFIER_LENGTH`, not read
/// off the schema — for the reason [`MAX_IDENTIFIER_LENGTH`] itself exists: a case that reads the
/// constant it is checking says only that the schema agrees with itself.
///
/// **The residue.** The bound the loader applies is on the *identifier component*, and `maxLength`
/// bounds the whole string; a 201-character id with `/1` after it is 203 characters, inside the
/// whole-string bound and outside the component one. JSON Schema has no per-component length
/// keyword, so this is the `u32`-ceiling class again: not a thing left undone, a thing the format
/// cannot say. It is pinned by exact value rather than described.
#[test]
fn no_pin_the_published_pattern_calls_valid_is_one_the_loader_refuses_past_the_length_bound() {
    let pins = [
        format!("{}/1", "a".repeat(200)),
        format!("{}/1", "a".repeat(201)),
        format!("workflow:{}/1", "a".repeat(201)),
        format!("{}/1", "a".repeat(400)),
        format!("workflow:{}/4294967295", "a".repeat(300)),
    ];
    let rule = PublishedRule::read::<PinnedWorkflowRef>();
    let bound = rule.max_length.expect("the pin publishes a length bound");
    assert_eq!(
        bound,
        u32::try_from(PIN_MAX_LENGTH).expect("a short bound"),
        "the published `maxLength` is not `workflow:` plus an identifier at its bound plus `/` \
         plus the ten digits a `u32` is written with"
    );
    for pin in &pins {
        let length = pin.len();
        // Every pin above is a lower-case identifier with a version tail, so the pattern half says
        // yes to all five and the length half is the only thing deciding them. Said out loud so
        // the expectation below is not silently carried by the pattern.
        assert!(
            matches(PinnedWorkflowRef::PATTERN, pin),
            "this {length}-character pin was meant to be one the pattern accepts, so that the \
             `maxLength` is what decides it"
        );
        assert_eq!(
            rule.accepts(pin),
            length <= PIN_MAX_LENGTH,
            "the published rule is not refusing a {length}-character pin at the bound of \
             {PIN_MAX_LENGTH} this file spells for itself"
        );
    }
    let divergent: Vec<&String> = pins
        .iter()
        .filter(|pin| rule.accepts(pin) && !loads(pin))
        .collect();
    assert_eq!(
        divergent.iter().map(|pin| pin.len()).collect::<Vec<_>>(),
        vec![203, 212],
        "the pins the published rule calls valid and the loader refuses are not the two the \
         per-component bound accounts for. A longer list is an editor telling an author a step map \
         is fine that will not load; a shorter one means the component bound became expressible, \
         and this expectation should shrink with it.\npublished rule: {} with maxLength {bound}",
        rule.pattern
    );
}

// --- adversarial round 2 -------------------------------------------------------------------------
//
// Added while attacking `story:workflow-id-pattern-numeric-tail` a second time. Nothing above this
// line is changed. These state the same property the cases above state, over the one input class
// *both* corpora are structurally unable to reach: a string in the *shape* of the seven types the
// round-1 correction added to `published_identifiers`.
//
// `charset_corpus` is every string of length 1..=3 over `a z A 0 9 - . / _ :` plus a fixed list of
// spellings. None of those characters is `*`, and none of those lengths reaches `<ns>.<name>/v1`.
// So `EntityType`, `EntityLocator` and `DomainEventType` are in the census and the corpus contains
// **no string their pattern accepts** — adding them to the table added a row and no measurement —
// and `FactPattern`'s one distinguishing character never appears.

/// Strings in the *shape* each published rule is written for.
///
/// [`charset_corpus`] is a breadth-first enumeration over one alphabet, which is the right
/// instrument for a flat identifier and the wrong one for a rule whose shortest accepted string is
/// `ep://a/b/c/d`. This one composes segments into each published spelling instead, so that every
/// rule in the census is evaluated on strings it can actually accept.
fn shape_corpus() -> Vec<String> {
    let segments = [
        "a", "1", "-", "*", "_", "A", "a-b", "a.b", "**", "", "v1", "a1",
    ];
    let short = ["a", "-", "*", "_", ""];
    let mut corpus: Vec<String> = Vec::new();
    for a in segments {
        corpus.push(a.to_owned());
        for b in segments {
            corpus.push(format!("{a}.{b}"));
            corpus.push(format!("{a}:{b}"));
            corpus.push(format!("{a}.{b}/v1"));
            corpus.push(format!("{a}/{b}"));
            for c in short {
                corpus.push(format!("{a}.{b}.{c}"));
                corpus.push(format!("{a}.{b}.{c}/v1"));
            }
        }
    }
    for a in short {
        for b in short {
            for c in short {
                for d in short {
                    corpus.push(format!("ep://{a}/{b}/{c}/{d}"));
                }
            }
        }
    }
    for kind in ["human", "agent", "service", "system", "robot"] {
        corpus.push(kind.to_owned());
        for name in segments {
            corpus.push(format!("{kind}:{name}"));
        }
    }
    for extra in [
        "tests.**",
        "tests.**.failed",
        "tests.*.failed",
        "tests.unit.failed",
        "ingest.**",
        "corpus.**",
        "aep.design/v1",
        "aep.design.created/v1",
        "ep://acme/payments/story/AUTH-142",
    ] {
        corpus.push(extra.to_owned());
    }
    corpus.sort();
    corpus.dedup();
    corpus
}

/// The three rules whose published pattern and constructor disagree today, with the number of
/// strings in [`shape_corpus`] each disagrees on.
///
/// Every one of these is the same defect `story:workflow-id-pattern-numeric-tail` fixed for four
/// charsets — a separator inside the character class instead of between segments — on a type that
/// story does not name. They are open, they are somebody's next unit, and they are pinned by count
/// here so that the gap cannot widen unnoticed while it waits.
const OPEN_DIVERGENCES: [(&str, usize); 3] = [
    // `^[a-z0-9][a-z0-9.-]*\.[a-z0-9-]+/v[1-9][0-9]*$` — `[a-z0-9-]+` accepts a segment that is a
    // bare `-`, `[a-z0-9.-]*` accepts an empty segment and a trailing `-`. `a.-/v1`, `a..a/v1` and
    // `a-b.-.-/v1` are valid to the schema and refused by `EntityType::parse`.
    ("EntityType", 66),
    // The same shape one segment longer, and the same divergence.
    ("DomainEventType", 49),
    // `^([A-Za-z0-9_*-]+)(\.[A-Za-z0-9_*-]+)*$` — `*` is *inside* the character class, so `a*`,
    // `*a`, `***`, `**.a` and `tests.**.failed` are valid to the schema, while `FactPattern::new`
    // allows `*` only as a whole segment and `**` only as the last one.
    ("FactPattern", 96),
];

/// Every published identifier rule accepts exactly what its own constructor accepts — over strings
/// in that rule's own shape — except for the three open divergences [`OPEN_DIVERGENCES`] names.
///
/// The same statement as
/// [`every_published_identifier_pattern_accepts_exactly_what_its_constructor_accepts`], over
/// [`shape_corpus`]. It is a separate case rather than a wider corpus on the existing one because
/// the existing one is green and this one was not: the difference between them was the whole
/// finding.
///
/// **What this asserted when it was written, and why it does not any more.** It asserted that
/// *no* published rule disagrees with its constructor over `shape_corpus`, and it failed: 211
/// disagreements over 2,727 strings, on `EntityType` (66), `DomainEventType` (49) and
/// `FactPattern` (96), every one of them the schema calling a string valid that the constructor
/// refuses. Those three types are not named by `story:workflow-id-pattern-numeric-tail`, whose
/// subject is `WorkflowId` and the four charsets `identifier_pattern!` holds — measured at 0
/// divergences over 56,594 strings — so fixing them here would be a second change hiding inside
/// this one. They are **open defects outside this unit**, and the case is stated against today's
/// numbers instead of being deleted, ignored or filtered down to a class that hides them.
///
/// **What to do when this goes red.** Read which side moved.
///
/// * A rule *not* in [`OPEN_DIVERGENCES`] diverging is a new defect, in the exact class this file
///   exists for: a document an editor and the loader answer differently. Fix the pattern or the
///   constructor; do not add a row.
/// * A count in [`OPEN_DIVERGENCES`] going **up** is the same defect widening. Fix it there.
/// * A count going **down to zero** means somebody closed one. Delete its row and this case
///   tightens back to the statement it was written as.
/// * A count going down but not to zero means a partial fix, which is the shape that leaves a
///   defect behind. Say which strings are still divergent before changing the number.
///
/// Two companion cases bound how far these three reach today, so that *open* is a measured claim
/// and not a shrug: [`the_open_fact_pattern_divergence_reaches_no_observable_a_protocol_ships`]
/// and [`the_open_entity_and_event_type_divergence_reaches_no_generated_schema`].
#[test]
fn every_published_identifier_rule_accepts_exactly_what_its_constructor_accepts_in_its_own_shape() {
    let corpus = shape_corpus();
    let mut divergent: Vec<String> = Vec::new();
    let mut tally: Vec<(&'static str, usize, usize)> = Vec::new();
    for (name, rule, accepts) in published_identifier_rules() {
        let mut looser = 0_usize;
        let mut stricter = 0_usize;
        for value in &corpus {
            let by_rule = rule.accepts(value);
            let by_constructor = accepts(value);
            if by_rule == by_constructor {
                continue;
            }
            if by_rule {
                looser += 1;
            } else {
                stricter += 1;
            }
            if OPEN_DIVERGENCES.iter().all(|(open, _)| *open != name) {
                divergent.push(format!(
                    "{name}: {value:?} — the published rule says {by_rule}, the constructor says \
                     {by_constructor} (pattern: {})",
                    rule.pattern
                ));
            }
        }
        if looser + stricter > 0 {
            tally.push((name, looser, stricter));
        }
    }

    assert!(
        divergent.is_empty(),
        "{} disagreement(s) between a published rule and the constructor it is published for, over \
         {} strings in the shapes those rules are written for, on a rule `OPEN_DIVERGENCES` does \
         not name. Each is a spelling an editor and the loader answer differently, and each is a \
         defect to fix rather than a row to add. First few: {:#?}",
        divergent.len(),
        corpus.len(),
        divergent.iter().take(16).collect::<Vec<_>>()
    );

    // The three open ones, by their exact counts and their exact direction. Every divergence on
    // these is the schema calling a string valid that the constructor refuses — an editor telling
    // an author a document is fine that will not load — and none runs the other way; a stricter
    // count above zero would be a different defect wearing the same name.
    assert_eq!(
        tally,
        OPEN_DIVERGENCES
            .iter()
            .map(|(name, looser)| (*name, *looser, 0))
            .collect::<Vec<_>>(),
        "the open divergences outside `story:workflow-id-pattern-numeric-tail` are not what they \
         were, over {} strings. Each tuple is (rule, strings the schema calls valid and the \
         constructor refuses, strings the other way). A rule appearing here that \
         `OPEN_DIVERGENCES` does not name is a new defect; a count that grew is an old one \
         widening; a count that fell to zero is one closed, and its row should go. See this case's \
         doc comment before changing a number.",
        corpus.len()
    );
}

/// Every rule in the census is measured by **each** corpus the census is run over.
///
/// A row in `published_identifiers` whose pattern accepts nothing in a corpus is a row that cannot
/// fail on that corpus: it asks the rule no question the rule can answer yes to, so the
/// constructor and the pattern are compared only where both say no. That is a census entry with no
/// measurement behind it, and it is indistinguishable from a passing one in the output.
///
/// **This was red when it was written, and the corpus is what changed.** Round 1 added seven rows
/// to `published_identifiers` and `charset_corpus` accepted **nothing** for three of them —
/// `EntityType`, `EntityLocator`, `DomainEventType`, 0 of 1135 each — and one string for
/// `ActorRef`. The answer was to widen the corpus with one well-formed spelling per shape, not to
/// move this assertion: a census row is measurable or it is decoration, and which corpus it is
/// measurable in is exactly what this case is for. Both corpora are checked, because both are run
/// over the same census and either going blind on a row is the same hole.
#[test]
fn every_rule_in_the_census_is_measured_by_the_corpus_the_census_runs_over() {
    let mut unmeasured: Vec<String> = Vec::new();
    for (corpus_name, corpus) in [
        ("charset_corpus", charset_corpus()),
        ("shape_corpus", shape_corpus()),
    ] {
        for (name, rule, _) in published_identifier_rules() {
            if corpus.iter().any(|value| rule.accepts(value)) {
                continue;
            }
            unmeasured.push(format!(
                "{name} accepts nothing in the {}-string {corpus_name}",
                corpus.len()
            ));
        }
    }
    assert!(
        unmeasured.is_empty(),
        "{} rule(s) in `published_identifiers` accept nothing in a corpus the census is evaluated \
         over, so the cases that evaluate them cannot fail on their account. Widen the corpus with \
         a well-formed string in that rule's own shape — do not relax this case: {:#?}",
        unmeasured.len(),
        unmeasured
    );
}

/// One open vocabulary's whole published rule, the name it is published under, and its parser.
type PublishedVocabularyRule = (&'static str, PublishedRule, fn(&str) -> bool);

/// The open vocabularies, read as rules — `pattern` **and** `maxLength` — rather than as patterns.
///
/// [`published_vocabularies`] reads the pattern alone, so the `maxLength` these five gained in
/// `story:workflow-id-pattern-numeric-tail` is invisible to every case that uses it. Measured:
/// deleting `schema.string().max_length` from any of the five leaves
/// `cargo test -p aep-domain -p aep-driver-spec` at exit 0.
fn published_vocabulary_rules() -> Vec<PublishedVocabularyRule> {
    vec![
        (
            "TestSuite",
            PublishedRule::read::<TestSuite>(),
            (|value| TestSuite::parse(value).is_ok()) as fn(&str) -> bool,
        ),
        ("TaskKind", PublishedRule::read::<TaskKind>(), |value| {
            TaskKind::parse(value).is_ok()
        }),
        (
            "ArtifactKind",
            PublishedRule::read::<ArtifactKind>(),
            |value| ArtifactKind::parse(value).is_ok(),
        ),
        (
            "Environment",
            PublishedRule::read::<Environment>(),
            |value| Environment::parse(value).is_ok(),
        ),
        ("Verifier", PublishedRule::read::<Verifier>(), |value| {
            Verifier::parse(value).is_ok()
        }),
    ]
}

/// The longest string each constructor takes is one its published rule takes, and one character
/// more is one it refuses.
///
/// The boundary the round-1 correction drew and nobody stands on. The cases that read a
/// `maxLength` run over four fixed shapes built from `a` — `a×200`, `a×201`, `a:` + `a×201`,
/// `a×201` + `:a` — and none of those is the longest string any of these constructors actually
/// takes. Three bounds are therefore unmeasured, and each was confirmed by mutation, one line at a
/// time, with `cargo test -p aep-domain -p aep-driver-spec --no-fail-fast`:
///
/// | mutation | suite |
/// |---|---|
/// | drop `max_length` from `ArtifactKind` / `TaskKind` / `Environment` / `TestSuite` / `Verifier` | exit 0 |
/// | `SubjectRef`'s `2 * MAX_LENGTH + 1` → `2 * MAX_LENGTH` | exit 0 |
/// | `version.rs`'s `MAJOR_DIGITS` 10 → 9 | exit 0 |
/// | `published_max_length("protocol:")` → `published_max_length("")` | exit 0 |
///
/// Each of those is the defect this story exists to close, one keyword along: a document an editor
/// calls valid that the loader refuses, or one it calls invalid that loads. This case is green on
/// the tree it was written against and red under every row above.
#[test]
fn the_longest_string_each_constructor_takes_is_one_its_published_rule_takes() {
    let at_bound = "a".repeat(MAX_IDENTIFIER_LENGTH);
    let past_bound = "a".repeat(MAX_IDENTIFIER_LENGTH + 1);
    let mut divergent: Vec<String> = Vec::new();
    let mut check = |name: &str, shape: &str, value: &str, rule: &PublishedRule, wanted: bool| {
        if rule.accepts(value) != wanted {
            divergent.push(format!(
                "{name}: {shape} ({} characters) — the published rule says {}, the constructor \
                 says {wanted} (maxLength: {:?})",
                value.chars().count(),
                rule.accepts(value),
                rule.max_length
            ));
        }
    };

    // The five open vocabularies. Each sends an unnamed value to a constructor that bounds it at
    // `MAX_IDENTIFIER_LENGTH`, so the longest name each takes is that many kebab characters.
    for (name, rule, parses) in published_vocabulary_rules() {
        assert!(
            parses(&at_bound),
            "{name} was expected to take a {MAX_IDENTIFIER_LENGTH}-character name"
        );
        assert!(
            !parses(&past_bound),
            "{name} was expected to refuse a name one character over the bound"
        );
        check(name, "the longest name it takes", &at_bound, &rule, true);
        check(name, "one character more", &past_bound, &rule, false);
    }

    // `SubjectRef`: a kind at the bound, a `:`, and an id at the bound. The longest subject
    // `SubjectRef::new` takes, and the number `2 * MAX_LENGTH + 1` is published for.
    let longest_subject = format!("{at_bound}:{at_bound}");
    assert!(
        SubjectRef::new(longest_subject.as_str()).is_ok(),
        "a kind and an id both at the bound is a subject the constructor takes"
    );
    let subject = PublishedRule::read::<SubjectRef>();
    check(
        "SubjectRef",
        "a kind and an id both at the bound",
        &longest_subject,
        &subject,
        true,
    );

    // The references: the prefix, an identifier at the bound, `/`, and the largest `u32`. The
    // longest reference each parser takes, which is the number `published_max_length` is written
    // to be.
    for (name, prefix, rule) in [
        (
            "ProtocolRef",
            "protocol:",
            PublishedRule::read::<ProtocolRef>(),
        ),
        (
            "PrincipleRef",
            "principle:",
            PublishedRule::read::<PrincipleRef>(),
        ),
        (
            "WorkflowRef",
            "workflow:",
            PublishedRule::read::<WorkflowRef>(),
        ),
        (
            "ProfileVersionedRef",
            "profile:",
            PublishedRule::read::<ProfileVersionedRef>(),
        ),
        (
            "PinnedWorkflowRef",
            "workflow:",
            PublishedRule::read::<PinnedWorkflowRef>(),
        ),
    ] {
        let longest = format!("{prefix}{at_bound}/{}", u32::MAX);
        check(
            name,
            "the longest reference it takes",
            &longest,
            &rule,
            true,
        );
    }

    assert!(
        divergent.is_empty(),
        "{} published rule(s) disagree with their constructor at the longest string that \
         constructor takes, so an editor and the loader answer a document differently on length \
         alone: {:#?}",
        divergent.len(),
        divergent
    );
}

/// The bound `aep_domain::ids::MAX_LENGTH` names, spelled here so this file does not read the
/// constant it is testing against.
///
/// Reading `MAX_LENGTH` here would make the case above say *the schema agrees with itself*: the
/// same constant decides the published `maxLength`, so a wrong value moves both sides together and
/// nothing fails. The number is `validate`'s documented bound
/// (`crates/govern/aep-domain/src/ids.rs` — "must be at most 200 characters"), written down once.
const MAX_IDENTIFIER_LENGTH: usize = 200;

/// How many digits the largest major version is written with, spelled here for the same reason
/// [`MAX_IDENTIFIER_LENGTH`] is: `4294967295` is ten characters, and reading `MAJOR_DIGITS` out of
/// `version.rs` or `pin.rs` would make a case comparing against it say only that the schema agrees
/// with itself.
const MAJOR_VERSION_DIGITS: usize = 10;

/// The whole-string bound `SubjectRef` publishes: a kind at the identifier bound, a `:`, and an id
/// at the identifier bound.
const SUBJECT_MAX_LENGTH: usize = 2 * MAX_IDENTIFIER_LENGTH + 1;

/// The whole-string bound a pin publishes: `workflow:`, an identifier at its bound, `/`, and a
/// major version at [`MAJOR_VERSION_DIGITS`].
const PIN_MAX_LENGTH: usize =
    "workflow:".len() + MAX_IDENTIFIER_LENGTH + "/".len() + MAJOR_VERSION_DIGITS;

// --- correction round 3 ---------------------------------------------------------------------
//
// Two cases that bound how far the three open divergences of `OPEN_DIVERGENCES` reach in this
// tree today. A defect left open is a claim about exposure, and a claim about exposure is worth
// exactly what measures it — so each of these is a live read of the tree rather than a sentence.
// Both are green today and both go red the moment somebody writes a document into the gap.

/// No observable any shipped protocol declares falls in the open `FactPattern` divergence.
///
/// `FactPattern::PATTERN` puts `*` inside its character class, so `a*`, `**.a` and
/// `tests.**.failed` are valid to `schemas/generated/protocol.schema.json` and refused by
/// `FactPattern::new`. That definition is referenced for `observables`, which
/// `protocols/aep/1.yaml`, `protocols/adp/1.yaml` and `protocols/aop/1.yaml` all declare — so the
/// divergence is reachable from a real document and the only thing keeping it harmless is that
/// nobody has written one of those spellings yet.
///
/// Measured when this was written: 46 observables across the three protocols, all agreed on by
/// both definitions. The exposure is the next author, and this case is what turns that from a
/// remark into a check: write `tests.**.failed` into a protocol and this goes red, naming it,
/// while the editor says the document is fine.
///
/// **What to do when it goes red.** Either the observable is wrong and should be spelled as
/// `FactPattern::new` requires, or the open defect at `crates/govern/aep-domain/src/facts.rs` has finally
/// been reached by a document and should be fixed — `*` between segments rather than inside the
/// class, the way `identifier_pattern!` writes `-`. Do not delete the observable to get green.
#[test]
fn the_open_fact_pattern_divergence_reaches_no_observable_a_protocol_ships() {
    let observables = shipped_observables();
    assert!(
        observables.len() >= 46,
        "the scan found {} observable(s) across the shipped protocols, fewer than the 46 that were \
         there when it was written, so it has most likely stopped reading the files rather than \
         found a smaller tree",
        observables.len()
    );
    let rule = PublishedRule::read::<FactPattern>();
    let divergent: Vec<String> = observables
        .iter()
        .filter(|(_, observable)| rule.accepts(observable) != FactPattern::new(observable).is_ok())
        .map(|(file, observable)| format!("{file}: {observable:?}"))
        .collect();
    assert!(
        divergent.is_empty(),
        "{} of {} shipped observable(s) fall in the open `FactPattern` divergence — the schema and \
         `FactPattern::new` answer them differently, so an editor and the engine disagree about a \
         protocol that ships: {:#?}\npublished pattern: {}",
        divergent.len(),
        observables.len(),
        divergent,
        rule.pattern
    );
}

/// Every observable declared under `observables:` in a shipped protocol, with the file it is in.
///
/// Read as text rather than as YAML: this crate takes exactly one dependency
/// (`crates/drive/aep-driver-spec/Cargo.toml` says why) and a parser added for a test would be a second.
/// The scan takes the run of `- ` items after a column-zero `observables:` key, skipping blanks
/// and comments and stopping at the next key — and the case above refuses a result smaller than
/// what was there when it was written, so a scan that has silently stopped working fails rather
/// than passes on nothing.
fn shipped_observables() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../protocols");
    let mut found: Vec<(String, String)> = Vec::new();
    for name in ["aep/1.yaml", "adp/1.yaml", "aop/1.yaml"] {
        let path = root.join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        let mut inside = false;
        for line in text.lines() {
            if line.starts_with("observables:") {
                inside = true;
                continue;
            }
            if !inside {
                continue;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some(item) = trimmed.strip_prefix("- ") else {
                inside = false;
                continue;
            };
            let item = item.trim().trim_matches(['\'', '"']);
            found.push((name.to_owned(), item.to_owned()));
        }
    }
    found
}

/// The open `EntityType` and `DomainEventType` divergence is not in any generated schema.
///
/// This is what makes those two smaller than the `FactPattern` one beside them: `FactPattern` is
/// published in `schemas/generated/protocol.schema.json` and referenced for `observables`, so an
/// editor really applies the wrong rule to a document that ships. Neither of these two appears in
/// any file under `schemas/generated/` at all — nothing that reaches a generated document holds
/// one yet — so today the wrong rule lives only in `json_schema()`, reachable by a test and by
/// nothing an author edits.
///
/// **What to do when it goes red.** A generated schema has started publishing one of these
/// patterns, which means the divergence `OPEN_DIVERGENCES` records has just become reachable from
/// a document. Fix the pattern in `crates/govern/aep-domain/src/entity.rs` or
/// `crates/govern/aep-domain/src/domain_event.rs` — a separator between segments rather than inside the
/// character class — before landing whatever put it there. Do not relax this case.
#[test]
fn the_open_entity_and_event_type_divergence_reaches_no_generated_schema() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../schemas/generated");
    let entries = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()));
    let mut schemas = 0_usize;
    let mut reached: Vec<String> = Vec::new();
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        schemas += 1;
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        for (name, pattern) in [
            ("EntityType", EntityType::PATTERN),
            ("DomainEventType", DomainEventType::PATTERN),
        ] {
            // The pattern's own text, not the type name: a schema may name a definition anything,
            // and what would carry the divergence to an author is the pattern being published.
            // JSON escapes each `\` in it, which is why the needle is escaped here too.
            let published = pattern.replace('\\', "\\\\");
            if text.contains(&published) {
                reached.push(format!(
                    "{name} is published by {}",
                    path.file_name().expect("a named file").to_string_lossy()
                ));
            }
        }
    }
    assert!(
        schemas > 10,
        "the scan read {schemas} generated schema(s), which is fewer than this tree has, so it is \
         most likely looking in the wrong place rather than finding a smaller tree"
    );
    assert!(
        reached.is_empty(),
        "{} open pattern divergence(s) have reached a generated schema, so an editor now applies \
         the wrong rule to a document an author writes rather than only to a test: {:#?}",
        reached.len(),
        reached
    );
}
