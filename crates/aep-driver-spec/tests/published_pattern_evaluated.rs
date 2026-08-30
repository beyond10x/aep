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
    ] {
        corpus.push(extra.to_owned());
    }
    corpus
}

/// One published pattern, the name it is published under, and the code that really decides.
type Published = (&'static str, &'static str, fn(&str) -> bool);

/// Every identifier these two crates publish a pattern for, with the constructor that decides it.
///
/// Listed one by one rather than sampled, because the defect this table is here for is *an
/// identifier having two definitions*, and a sample of the identifiers is a sample of the defect.
fn published_identifiers() -> Vec<Published> {
    vec![
        ("ProtocolId", ProtocolId::PATTERN, |v| {
            ProtocolId::new(v).is_ok()
        }),
        ("PrincipleId", PrincipleId::PATTERN, |v| {
            PrincipleId::new(v).is_ok()
        }),
        ("ProfileId", ProfileId::PATTERN, |v| {
            ProfileId::new(v).is_ok()
        }),
        ("WorkflowId", WorkflowId::PATTERN, |v| {
            WorkflowId::new(v).is_ok()
        }),
        ("StateId", StateId::PATTERN, |v| StateId::new(v).is_ok()),
        ("PhaseId", PhaseId::PATTERN, |v| PhaseId::new(v).is_ok()),
        ("ObligationId", ObligationId::PATTERN, |v| {
            ObligationId::new(v).is_ok()
        }),
        ("ApprovalId", ApprovalId::PATTERN, |v| {
            ApprovalId::new(v).is_ok()
        }),
        ("ClaimId", ClaimId::PATTERN, |v| ClaimId::new(v).is_ok()),
        ("ToolRef", ToolRef::PATTERN, |v| ToolRef::new(v).is_ok()),
        ("ServiceId", ServiceId::PATTERN, |v| {
            ServiceId::new(v).is_ok()
        }),
        ("ProviderId", ProviderId::PATTERN, |v| {
            ProviderId::new(v).is_ok()
        }),
        ("RepositoryRef", RepositoryRef::PATTERN, |v| {
            RepositoryRef::new(v).is_ok()
        }),
        ("CommandId", CommandId::PATTERN, |v| {
            CommandId::new(v).is_ok()
        }),
        ("RequestId", RequestId::PATTERN, |v| {
            RequestId::new(v).is_ok()
        }),
        ("CorrelationId", CorrelationId::PATTERN, |v| {
            CorrelationId::new(v).is_ok()
        }),
        ("IdempotencyKey", IdempotencyKey::PATTERN, |v| {
            IdempotencyKey::new(v).is_ok()
        }),
        ("AuditId", AuditId::PATTERN, |v| AuditId::new(v).is_ok()),
        ("EventId", EventId::PATTERN, |v| EventId::new(v).is_ok()),
        ("RelationId", RelationId::PATTERN, |v| {
            RelationId::new(v).is_ok()
        }),
        ("TaskId", TaskId::PATTERN, |v| TaskId::new(v).is_ok()),
        ("EvidenceId", EvidenceId::PATTERN, |v| {
            EvidenceId::new(v).is_ok()
        }),
        ("ExecutionId", ExecutionId::PATTERN, |v| {
            ExecutionId::new(v).is_ok()
        }),
        ("SubjectRef", SubjectRef::PATTERN, |v| {
            SubjectRef::new(v).is_ok()
        }),
        ("StepMapId", StepMapId::PATTERN, |v| {
            StepMapId::new(v).is_ok()
        }),
    ]
}

/// Every versioned reference these two crates publish a pattern for, with its parser.
fn published_references() -> Vec<Published> {
    vec![
        ("ProtocolRef", ProtocolRef::PATTERN, |v| {
            v.parse::<ProtocolRef>().is_ok()
        }),
        ("PrincipleRef", PrincipleRef::PATTERN, |v| {
            v.parse::<PrincipleRef>().is_ok()
        }),
        ("WorkflowRef", WorkflowRef::PATTERN, |v| {
            v.parse::<WorkflowRef>().is_ok()
        }),
        ("ProfileVersionedRef", ProfileVersionedRef::PATTERN, |v| {
            v.parse::<ProfileVersionedRef>().is_ok()
        }),
        ("PinnedWorkflowRef", PinnedWorkflowRef::PATTERN, |v| {
            v.parse::<WorkflowRef>()
                .is_ok_and(|reference| PinnedWorkflowRef::try_from(reference).is_ok())
        }),
    ]
}

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
/// load, because a major version is a `u32` (`crates/aep-domain/src/version.rs`) and a JSON Schema
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
/// **What this case cannot say.** `validate` refuses an identifier longer than 200 characters and
/// no pattern here bounds length, so the corpus stays short; the divergence exists and is a
/// tightening the loader applies on top, not a document an editor calls valid and the loader
/// rejects on a spelling.
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
