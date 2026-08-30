//! The published pattern, **evaluated** rather than paraphrased.
//!
//! `PinnedWorkflowRef::PATTERN` is now composed at compile time from `WorkflowId::PATTERN` by
//! string concatenation, and `pin.rs` asserts that composition with `assert_eq!` on the two
//! strings. That says the pin's spelling is the identifier's spelling with a version bolted on. It
//! does not say the composed pattern *behaves* like the identifier rule plus a version, and
//! nothing else in the tree does either: every other case about the pattern is a hand-written
//! paraphrase of it (`tests/pin_pattern_agrees_with_the_loader.rs`) or a `contains`/`ends_with`
//! check on its text (`pin.rs`). Three encodings of one rule, none of them executable.
//!
//! Concatenation is not composition. `strip_prefix('^')` + `strip_suffix('$')` on a constant that
//! later gains a top-level alternation — `^a|legacy$` — yields `^(workflow:)?a|legacy/[1-9][0-9]*$`,
//! which matches every string beginning with `a`, and `assert_eq!` on the two strings stays green
//! because both sides moved together. This file evaluates the constant, so the property that
//! matters is stated about behaviour.
//!
//! The interpreter is deliberately narrow — anchors, literals, classes, groups, alternation, `?`,
//! `*`, `+` — and **panics** on anything else, including a `^` or `$` anywhere but the ends. A
//! pattern this file cannot read fails loudly instead of being silently mis-evaluated.

use aep_domain::ids::WorkflowId;
use aep_driver_spec::map::{RawStepMap, StepMap};
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

/// `true` when `pattern` matches the whole of `value`.
///
/// The pattern must be anchored at both ends, which every published one in this workspace is.
fn matches(pattern: &str, value: &str) -> bool {
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
    let chars: Vec<char> = value.chars().collect();
    walk(&node, &chars, 0, &mut |end| end == chars.len())
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
        (WorkflowId::PATTERN, "adp/2", true),
        (PinnedWorkflowRef::PATTERN, "adp/default/1", true),
        (PinnedWorkflowRef::PATTERN, "workflow:adp/default/1", true),
        (PinnedWorkflowRef::PATTERN, "adp/default", false),
        (PinnedWorkflowRef::PATTERN, "adp/default/0", false),
        (PinnedWorkflowRef::PATTERN, "adp-/1", false),
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

/// The identifier half of a pin, and its last `.`/`/` component.
fn identifier_of(pin: &str) -> Option<(String, String)> {
    let body = pin.strip_prefix("workflow:").unwrap_or(pin);
    let (id, _) = body.rsplit_once('/')?;
    let last = id.rsplit(['.', '/']).next()?.to_owned();
    Some((id.to_owned(), last))
}

/// The divergence left is **exactly** the numeric-tail class, and it is this big.
///
/// Acceptance line 3 of `story:driver-spec-crate` — *an editor cannot tell an author a map is
/// fine that the loader will refuse* — is **not met**, and this is the case that says so without
/// being red about it. A failing test would say the same thing and take the whole suite's exit
/// status with it; the defect is real, open and owned by
/// `story:workflow-id-pattern-numeric-tail`, and a story is where an unfixed defect lives.
///
/// So this asserts *which* divergences and *how many*, so a new class arriving is a failure rather
/// than a bigger number in a message nobody re-reads. It fixes the two things the hand-written
/// paraphrase it replaces got wrong: it reads the constant, and it names 183 rather than the three
/// spellings that happened to be in a sample.
///
/// `WorkflowId::new` refuses an id whose last `.`/`/` component is a bare integer — the form is
/// reserved for `<id>/<major>` — and `WorkflowId::PATTERN` does not say so, so neither does the
/// pin's. **This is not because a `pattern` cannot express it.** It can, and the body below is
/// checked against `WorkflowId::new` over this file's corpus by
/// `the_numeric_tail_rule_is_expressible_as_a_pattern`. The reason it is not fixed here is that
/// the rule belongs to `aep-domain`, in one place, for `WorkflowId` and `ProfileId` together;
/// a stricter copy in this crate would put back the second, drifting rule that this crate has
/// already been bitten by once. Tracked as `story:workflow-id-pattern-numeric-tail`.
///
/// When that lands, this case fails — the count moves and the set empties — and whoever lands it
/// regenerates `schemas/generated/driver-steps.schema.json` with it. That is the point of the
/// count being exact.
#[test]
fn the_only_pins_the_published_pattern_calls_valid_that_the_loader_refuses_are_the_numeric_tail() {
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
    // Every spelling a pin can take, named forms included, so the count below is the whole gap
    // and not a sample of it.
    for extra in [
        "adp/default/1",
        "adp/2/1",
        "adp.2/1",
        "adp/22/1",
        "adp/default/0",
    ] {
        pins.push(extra.to_owned());
    }
    let divergent: Vec<String> = pins
        .into_iter()
        .filter(|pin| matches(PinnedWorkflowRef::PATTERN, pin) && !loads(pin))
        .collect();

    let not_numeric_tail: Vec<&String> = divergent
        .iter()
        .filter(|pin| {
            identifier_of(pin).is_none_or(|(_, last)| {
                last.is_empty() || !last.chars().all(|c| c.is_ascii_digit())
            })
        })
        .collect();
    assert!(
        not_numeric_tail.is_empty(),
        "a divergence that is not the numeric-tail class has appeared, and it is not the one this \
         case records: {:?}",
        not_numeric_tail.iter().take(8).collect::<Vec<_>>()
    );

    assert_eq!(
        divergent.len(),
        183,
        "the published schema accepts {} pin(s) the loader refuses. If this went to 0, \
         `story:workflow-id-pattern-numeric-tail` has landed: delete this case, and acceptance \
         line 3 of `story:driver-spec-crate` is met. If it grew, something reopened the gap. \
         First few: {:?}",
        divergent.len(),
        divergent.iter().take(8).collect::<Vec<_>>()
    );
}

/// The numeric-tail rule **is** expressible as a `pattern`, and this is the body that does it.
///
/// Recorded here because the argument for leaving the gap open was written down once as *"a
/// `pattern` cannot express it readably"*, which is false and makes a chosen decision read as a
/// forced one. The real reason is where the rule lives, not whether it can be written.
///
/// Checked, not asserted: this body agrees with `WorkflowId::new` on every string in the corpus.
/// `story:workflow-id-pattern-numeric-tail` can take it as is.
#[test]
fn the_numeric_tail_rule_is_expressible_as_a_pattern() {
    const PROPOSED: &str = "^[a-z][a-z0-9]*(-[a-z0-9]+)*(([./][a-z0-9]+(-[a-z0-9]+)*)*[./]([a-z0-9]*[a-z][a-z0-9]*(-[a-z0-9]+)*|[0-9]+(-[a-z0-9]+)+))?$";

    let corpus = identifier_corpus();
    let mismatches: Vec<&String> = corpus
        .iter()
        .filter(|id| matches(PROPOSED, id) != WorkflowId::new(id.as_str()).is_ok())
        .collect();
    assert!(
        mismatches.is_empty(),
        "the proposed body disagrees with `WorkflowId::new` on {} of {} strings: {:?}",
        mismatches.len(),
        corpus.len(),
        mismatches.iter().take(8).collect::<Vec<_>>()
    );

    // And it is strictly tighter than what is published, which is the whole point of proposing it.
    let current_body = WorkflowId::PATTERN;
    let closed: Vec<&String> = corpus
        .iter()
        .filter(|id| matches(current_body, id) && !matches(PROPOSED, id))
        .collect();
    assert!(
        !closed.is_empty(),
        "the proposed body refuses nothing the published one accepts. If \
         `story:workflow-id-pattern-numeric-tail` has landed, this body *is* the published one and \
         this case has done its job: delete it. Otherwise something reopened the gap."
    );
}
