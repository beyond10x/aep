//! The scene as instructions, in words.
//!
//! The fourth emitter, and the first that is not a picture. [`crate::svg`], [`crate::html`] and
//! [`crate::ansi`] answer *where is this state on a canvas*; this one answers *what am I not allowed
//! to do yet*, for a reader who has no canvas — a person reading a file, or an agent being handed
//! one.
//!
//! # Why a repository of typed documents needs a prose rendering at all
//!
//! Because the alternative is somebody typing the rules into a prompt. A workflow, its guards and
//! the principles timed against its phases are already written down here, exactly and in one place;
//! an instruction document that is *rendered* from them is a committed, reproducible artifact that
//! cannot drift from the specification without a check going red. One that is typed is a claim
//! about the specification, made once, by whoever was at the keyboard.
//!
//! # Everything here is derived, and the connectives are the only writing
//!
//! No sentence in the output describes a state, a guard, a phase or an obligation in words this
//! module chose. Titles, summaries, guards, requirements, timings and failure policies are the
//! documents' own text, printed verbatim. What this module supplies is fixed connective phrasing —
//! *You may not enter this state until*, *From here you may move to*, *only while* — and the order
//! things are said in. That line is worth holding: the moment a renderer starts explaining what a
//! state means, the explanation is a second specification that nothing validates.
//!
//! # Determinism
//!
//! Byte-identical for the same inputs, like every other emitter here, and for a sharper reason: the
//! documents this produces are **committed** under `generated/instructions/`, and a re-render that
//! reordered a list would be a diff nobody chose. Everything it walks is already ordered by
//! [`crate::scene`] and [`crate::obligations`]; nothing here sorts, and nothing here reads a clock,
//! so the output carries no date and no version of the binary that wrote it.

use std::fmt::Write as _;

use crate::obligations::{BoundPrinciple, Landing, Obligations};
use crate::scene::{Edge, Node, Scene};
use crate::steps::StepsView;

/// How wide the prose is wrapped.
///
/// 96 columns: wide enough that a guard like `(tests.unit.failed == 0 and tests.contract.failed ==
/// 0 and static_analysis.errors == 0)` is rarely broken, narrow enough to read in a diff beside
/// another file. Wrapping is here rather than left to the reader's viewer because these documents
/// are reviewed as text in a pull request.
const WIDTH: usize = 96;

/// One rendered instruction document, and where it belongs among others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// Its path inside a directory of instruction documents, such as `adp/default.md`.
    ///
    /// Derived from the workflow's declared id, so the file a workflow renders to is decided by the
    /// document's own identity and never by the name of the file it was read from — which is
    /// invariant 10 reaching one directory further out.
    pub path: String,
    /// The workflow's reference, as `<id>/<major>`.
    pub reference: String,
    /// The workflow's human title.
    pub title: String,
    /// The document itself.
    pub document: String,
}

/// A workflow and the principles that bind it, as one instruction document.
///
/// `steps` is the driver's half, and it is optional because the two documents are separate on
/// purpose: without it the document says what may happen, with it the document also says what runs.
pub fn instruction(
    scene: &Scene,
    obligations: &Obligations,
    steps: Option<&StepsView>,
) -> Instruction {
    Instruction {
        path: format!("{}.md", scene.id),
        reference: scene.reference.clone(),
        title: scene.title.clone(),
        document: render(scene, obligations, steps),
    }
}

/// A workflow and the principles that bind it, as instructions.
pub fn render(scene: &Scene, obligations: &Obligations, steps: Option<&StepsView>) -> String {
    let mut out = String::with_capacity(8192);

    let _ = writeln!(
        out,
        "<!-- Rendered from `{}` by `protocol govern workflow instruct`. Do not edit: change the \
         workflow document or the principles timed against its phases, and render again. -->",
        scene.reference
    );
    let _ = writeln!(out, "\n# {}\n", scene.title);
    let _ = writeln!(
        out,
        "`{}` · {} states · {} transitions · {} principles bind it.\n",
        scene.reference,
        scene.nodes.len(),
        scene.edges.len(),
        obligations.principles.len()
    );
    if let Some(summary) = &scene.summary {
        let _ = writeln!(out, "{}\n", wrapped(summary));
    }

    header(&mut out, scene);
    states(&mut out, scene, obligations, steps);
    principles(&mut out, obligations);

    out
}

/// An index over a directory of instruction documents.
///
/// Generated from the same list the directory is written from, so a document cannot land here
/// undocumented — and, being generated, it is not a file nothing produces either.
pub fn index(instructions: &[Instruction]) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str("<!-- Rendered by `protocol govern workflow instruct`. Do not edit. -->\n");
    out.push_str("\n# Workflows, as instructions\n\n");
    out.push_str(&wrapped(
        "One document per workflow this tree declares: its states, what opens each move between \
         them, and the principles that time obligations against the phases its states belong to. \
         Every sentence is derived from a typed document — nothing here was written by hand, and \
         nothing here can disagree with the specification without the gate going red.",
    ));
    out.push_str("\n\nRegenerate with:\n\n");
    out.push_str(
        "```console\nprotocol govern workflow instruct --out generated/instructions\n```\n\n",
    );
    for instruction in instructions {
        let _ = writeln!(
            out,
            "* [`{}`]({}) — {} (`{}`)",
            instruction.path, instruction.path, instruction.title, instruction.reference
        );
    }
    out
}

/// The paragraph that says how the rest of the document binds.
fn header(out: &mut String, scene: &Scene) {
    out.push_str("## How to read this\n\n");
    let _ = writeln!(
        out,
        "{}\n",
        wrapped(&format!(
            "Work moves through the states below and through nothing else. It starts in `{}`. You \
             may not enter a state until a transition into it opens, and a transition opens only \
             when its guard holds and everything listed under it has been met. An unobserved fact \
             does not open one: where a guard reads something nobody has recorded, the move stays \
             shut, because not knowing is not the same as knowing it is fine.",
            scene.initial
        ))
    );
}

/// Every state, in the order work reaches them.
fn states(out: &mut String, scene: &Scene, obligations: &Obligations, steps: Option<&StepsView>) {
    out.push_str("## The states\n\n");
    for (index, node) in scene.nodes.iter().enumerate() {
        let _ = writeln!(out, "### {}. `{}` — {}\n", index + 1, node.id, node.title);
        if let Some(summary) = &node.summary {
            let _ = writeln!(out, "{}\n", wrapped(summary));
        }
        state_facts(out, node, obligations);
        state_steps(out, node, steps);
        moves(out, scene, node);
    }
}

/// What a driver runs in one state, when the caller supplied a map.
///
/// Silence is two different facts and they are written differently. A map that does not mention
/// this state says nothing here at all, because the map's author may simply not have got to it. A
/// map that mentions it with no steps says so, because that is a claim: nothing runs, the state is
/// a place the run passes through.
fn state_steps(out: &mut String, node: &Node, steps: Option<&StepsView>) {
    let Some(view) = steps else { return };
    let Some(entries) = view.of(&node.id) else {
        return;
    };
    if entries.is_empty() {
        out.push_str("The driver runs nothing here.\n\n");
        return;
    }
    // "in this order" only where there is an order. On a single step it is a phrase that says
    // nothing and reads as though something were missing.
    out.push_str(if entries.len() == 1 {
        "One step runs here:\n\n"
    } else {
        "These steps run here, in this order:\n\n"
    });
    for entry in entries {
        let _ = writeln!(out, "* `{}` — {}", entry.kind, entry.label);
    }
    out.push('\n');
}

/// What the document says about one state, before anything about leaving it.
fn state_facts(out: &mut String, node: &Node, obligations: &Obligations) {
    if !node.phases.is_empty() {
        let _ = writeln!(
            out,
            "{} {}.\n",
            if node.phases.len() == 1 {
                "It belongs to phase"
            } else {
                "It belongs to phases"
            },
            code_list(&node.phases)
        );
    }
    if !node.requires.is_empty() {
        out.push_str("You may not enter it until all of this holds:\n\n");
        bullets(out, &node.requires);
    }

    let before = due_here(obligations, node, "before");
    if !before.is_empty() {
        out.push_str("These obligations fall due before you may enter it:\n\n");
        bullets(out, &before);
    }
    let during = due_here(obligations, node, "during");
    if !during.is_empty() {
        out.push_str(
            "These obligations hold while you are here, and are checked as you leave:\n\n",
        );
        bullets(out, &during);
    }

    if node.irreversible {
        out.push_str("Work done here cannot be undone. There is no route back from it.\n\n");
    }
    if let Some(policy) = &node.on_failure {
        let _ = writeln!(out, "If a requirement here is not met: {policy}.\n");
    }
}

/// The obligations timed `kind` that land on `node`, as one line each.
///
/// A cross-reference and not a copy: the requirement text lives once, under the principle that
/// imposes it, and this is what tells a reader standing in a state which of those to go and read.
/// Repeating the requirements here would be two places for one rule to be written down.
fn due_here(obligations: &Obligations, node: &Node, kind: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for principle in &obligations.principles {
        for obligation in &principle.obligations {
            if obligation.landing.states.contains(&node.id)
                && obligation.landing.timing.starts_with(kind)
            {
                lines.push(format!(
                    "`{}` — {}",
                    obligation.id,
                    obligation
                        .description
                        .clone()
                        .unwrap_or_else(|| principle.title.clone())
                ));
            }
        }
    }
    lines
}

/// Where a reader may go from one state, and what opens each move.
fn moves(out: &mut String, scene: &Scene, node: &Node) {
    let leaving: Vec<&Edge> = scene
        .edges
        .iter()
        .filter(|edge| edge.from == node.id)
        .collect();
    if leaving.is_empty() {
        if node.terminal {
            out.push_str(
                "The workflow ends here. Nothing leaves this state, and reaching it is what \
                 finishing means.\n\n",
            );
        }
        return;
    }
    out.push_str("From here you may move to:\n\n");
    for edge in leaving {
        let opening = match &edge.guard {
            Some(guard) => format!("only while `{guard}`"),
            None => "nothing gates this move".to_owned(),
        };
        let _ = writeln!(out, "* **`{}`** — {}.", edge.to, opening);
        if let Some(description) = &edge.description {
            let _ = writeln!(out, "{}", indented(description));
        }
        for requirement in &edge.requires {
            let _ = writeln!(
                out,
                "{}",
                indented(&format!("It also requires: {requirement}."))
            );
        }
        if let Some(policy) = &edge.on_failure {
            let _ = writeln!(
                out,
                "{}",
                indented(&format!("When the guard does not hold: {policy}."))
            );
        }
    }
    out.push('\n');
}

/// Every principle that binds the workflow, and what each one costs the reader.
fn principles(out: &mut String, obligations: &Obligations) {
    out.push_str("## What binds you here\n\n");
    if obligations.is_empty() {
        out.push_str(
            "No principle in this tree times an obligation against a phase these states declare, \
             withdraws a capability or requires evidence. The transitions above are the whole of \
             it.\n",
        );
        return;
    }
    out.push_str(&wrapped(
        "These principles reach this workflow: each times an obligation against a phase one of its \
         states declares, withdraws a capability, or requires evidence that must exist before the \
         work is finished. A principle applies unless the condition under *Applies when* is \
         observed to be false — an unobserved condition does not switch a principle off.",
    ));
    out.push_str("\n\n");
    for principle in &obligations.principles {
        principle_section(out, principle);
    }
}

/// One principle, as it bears on this workflow.
fn principle_section(out: &mut String, principle: &BoundPrinciple) {
    let _ = writeln!(out, "### `{}` — {}\n", principle.reference, principle.title);
    if let Some(summary) = &principle.summary {
        let _ = writeln!(out, "{}\n", wrapped(summary));
    }
    if let Some(condition) = &principle.applies_when {
        let _ = writeln!(out, "Applies when `{condition}`.\n");
    }

    for obligation in &principle.obligations {
        let _ = writeln!(out, "{}\n", when(&obligation.landing));
        if let Some(description) = &obligation.description {
            let _ = writeln!(out, "{}\n", wrapped(description));
        }
        if obligation.requires.is_empty() {
            out.push_str("It states no requirement of its own.\n\n");
        } else {
            bullets(out, &obligation.requires);
        }
    }

    if !principle.evidence.is_empty() {
        out.push_str("Evidence it requires, which must exist before the work is finished:\n\n");
        bullets(out, &principle.evidence);
    }
    if !principle.verification.is_empty() {
        out.push_str("Verifiers that must have spoken:\n\n");
        let lines: Vec<String> = principle
            .verification
            .iter()
            .map(|check| {
                if check.landing.reaches() {
                    format!("{} — {}", check.statement, when_phrase(&check.landing))
                } else {
                    check.statement.clone()
                }
            })
            .collect();
        bullets(out, &lines);
    }
    if !principle.denied.is_empty() {
        let _ = writeln!(
            out,
            "{}\n",
            wrapped(&format!(
                "You may not use {}. This is withdrawn, not gated: there is no approval that \
                 returns it.",
                code_list(&principle.denied)
            ))
        );
    }
    if !principle.approval_required.is_empty() {
        let _ = writeln!(
            out,
            "{}\n",
            wrapped(&format!(
                "You may use {} only with a recorded approval.",
                code_list(&principle.approval_required)
            ))
        );
    }
    if !principle.allowed.is_empty() {
        let _ = writeln!(
            out,
            "{}\n",
            wrapped(&format!("It grants {}.", code_list(&principle.allowed)))
        );
    }
    let _ = writeln!(
        out,
        "If one of its requirements is not met: {}.\n",
        principle.on_failure
    );
    if !principle.elsewhere.is_empty() {
        let _ = writeln!(
            out,
            "{}\n",
            wrapped(&format!(
                "It also obliges {}, {}. {} named so that a reader knows this document is a view \
                 of the principle and not the whole of it.",
                code_list(&principle.elsewhere),
                if principle.elsewhere.len() == 1 {
                    "a timing no state of this workflow declares, so nothing here comes due for it"
                } else {
                    "timings no state of this workflow declares, so nothing here comes due for them"
                },
                if principle.elsewhere.len() == 1 {
                    "It is"
                } else {
                    "They are"
                }
            ))
        );
    }
}

/// The sentence that introduces one obligation, from where it lands.
fn when(landing: &Landing) -> String {
    format!("**{}:**", capitalised(&when_phrase(landing)))
}

/// Where and when an obligation comes due, in words.
///
/// The phase is named beside the states rather than instead of them: the states are what a reader
/// acts on, and the phase is what makes the same principle reusable across workflows — dropping
/// either one loses half of why the obligation is here.
fn when_phrase(landing: &Landing) -> String {
    if landing.everywhere {
        return "owed at every transition".to_owned();
    }
    let verb = if landing.timing.starts_with("during") {
        "while in"
    } else {
        "before entering"
    };
    let states = code_list(
        &landing
            .states
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    );
    match &landing.phase {
        Some(phase) => format!("{verb} {states}, the {phase} phase"),
        None => format!("{verb} {states}"),
    }
}

/// One list of things, each in backticks, joined for a sentence.
fn code_list(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|item| format!("`{item}`")).collect();
    match quoted.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// A bullet list, one item per line, followed by a blank line.
fn bullets(out: &mut String, lines: &[String]) {
    for line in lines {
        let _ = writeln!(out, "* {line}");
    }
    out.push('\n');
}

/// A continuation line under a bullet.
fn indented(text: &str) -> String {
    wrap(text, WIDTH - 2)
        .into_iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` wrapped to [`WIDTH`].
fn wrapped(text: &str) -> String {
    wrap(text, WIDTH).join("\n")
}

/// `text` as lines of at most `width` characters, breaking only at spaces.
///
/// Greedy and deliberately simple: no hyphenation, no penalty function, and a word longer than the
/// width gets a line of its own rather than being cut. Two runs over one input produce one answer,
/// which is the only property this has to have.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }
        if current.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// `text` with its first character upper-cased.
fn capitalised(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obligations::Obligations;
    use crate::steps::StepView;
    use crate::testing::{fixture_principles, fixture_workflow, principle_from};

    /// The repository's own development workflow, with the principles the repository ships.
    fn rendered() -> String {
        let workflow = fixture_workflow();
        let scene = Scene::build(&workflow, None);
        let obligations = Obligations::of(&workflow, fixture_principles().iter());
        render(&scene, &obligations, None)
    }

    /// The same document, with a driver's half laid over it.
    fn rendered_with(steps: &StepsView) -> String {
        let workflow = fixture_workflow();
        let scene = Scene::build(&workflow, None);
        let obligations = Obligations::of(&workflow, fixture_principles().iter());
        render(&scene, &obligations, Some(steps))
    }

    /// A map covering the workflow's first state and nothing else.
    fn one_state_map(scene: &Scene, entries: Vec<StepView>) -> StepsView {
        let mut states = std::collections::BTreeMap::new();
        states.insert(scene.initial.clone(), entries);
        StepsView {
            reference: "fixture/map".to_owned(),
            states,
        }
    }

    #[test]
    fn without_a_map_the_document_says_what_may_happen_and_never_what_runs() {
        let written = rendered();
        assert!(
            !written.contains("runs here"),
            "a workflow read on its own has no driver: {written}"
        );
    }

    #[test]
    fn a_mapped_state_says_what_runs_in_it_and_in_which_order() {
        let workflow = fixture_workflow();
        let scene = Scene::build(&workflow, None);
        let map = one_state_map(
            &scene,
            vec![
                StepView {
                    kind: "command".to_owned(),
                    label: "read the world".to_owned(),
                },
                StepView {
                    kind: "llm".to_owned(),
                    label: "decide what it meant".to_owned(),
                },
            ],
        );
        let written = rendered_with(&map);

        assert!(
            written.contains("These steps run here, in this order:"),
            "{written}"
        );
        let first = written.find("read the world").expect("the first step");
        let second = written
            .find("decide what it meant")
            .expect("the second step");
        assert!(first < second, "the author's order survives: {written}");
        assert!(
            written.contains("* `command` — read the world"),
            "{written}"
        );
    }

    #[test]
    fn one_step_is_not_told_it_has_an_order() {
        let workflow = fixture_workflow();
        let scene = Scene::build(&workflow, None);
        let map = one_state_map(
            &scene,
            vec![StepView {
                kind: "operator".to_owned(),
                label: "ask a person".to_owned(),
            }],
        );
        let written = rendered_with(&map);
        assert!(written.contains("One step runs here:"), "{written}");
        assert!(!written.contains("in this order"), "{written}");
    }

    #[test]
    fn a_state_a_map_covers_with_nothing_reads_differently_from_one_it_omits() {
        let workflow = fixture_workflow();
        let scene = Scene::build(&workflow, None);
        let claimed = rendered_with(&one_state_map(&scene, Vec::new()));
        assert!(
            claimed.contains("The driver runs nothing here."),
            "a map that covers a state with no steps is making a claim: {claimed}"
        );

        let omitted = rendered_with(&StepsView {
            reference: "fixture/map".to_owned(),
            states: std::collections::BTreeMap::new(),
        });
        assert!(
            !omitted.contains("The driver runs nothing here."),
            "a map silent about a state says nothing about it: {omitted}"
        );
        assert!(!omitted.contains("runs here"), "{omitted}");
    }

    // There is deliberately **no whole-document snapshot in `fixtures/` here**, which is where the
    // other three emitters keep theirs. Theirs is the only committed copy of what they draw; this
    // one's committed copy is the artifact itself, `generated/instructions/adp/default.md`, held
    // byte-identical by `crates/edge/aep-cli/tests/instructions.rs`. A fixture beside it would be a
    // second copy of one five-hundred-line document with nothing asserting the two agree — the
    // drift this repository writes drift checks to avoid. What is asserted here is what the prose
    // has to *say*; that it says it in exactly those bytes is asserted where the bytes are shipped.

    #[test]
    fn two_renderings_of_one_workflow_are_the_same_bytes() {
        assert_eq!(
            rendered(),
            rendered(),
            "a committed document that re-renders differently is a diff nobody chose"
        );
    }

    #[test]
    fn the_committed_document_carries_no_date() {
        // The one way a derived document acquires a diff nobody chose. The crate's determinism scan
        // already refuses a clock in these sources; this reads the *output*, which is what a
        // reviewer sees, and it would also catch a date copied out of a document.
        let document = rendered();
        let dated: Vec<&str> = document.lines().filter(|line| looks_dated(line)).collect();
        assert!(
            dated.is_empty(),
            "these lines carry a date, and a dated document re-renders differently tomorrow: {dated:?}"
        );
    }

    /// `true` when `line` holds something shaped like `2026-08-23`.
    fn looks_dated(line: &str) -> bool {
        let characters: Vec<char> = line.chars().collect();
        characters.windows(10).any(|window| {
            window[..4].iter().all(char::is_ascii_digit)
                && window[4] == '-'
                && window[5..7].iter().all(char::is_ascii_digit)
                && window[7] == '-'
                && window[8..].iter().all(char::is_ascii_digit)
        })
    }

    #[test]
    fn the_date_scan_sees_a_date_and_leaves_a_guard_alone() {
        assert!(looks_dated("observed 2026-08-23 by somebody"));
        assert!(!looks_dated("* `error_rate < service.slo.error_threshold`"));
    }

    #[test]
    fn a_guard_is_the_documents_own_predicate_and_an_unguarded_move_says_so() {
        let document = rendered();
        assert!(
            document.contains("only while `artifact.specification.exists`"),
            "the guard is printed as the document writes it, not paraphrased"
        );
        assert!(
            document.contains("**`specify`** — nothing gates this move."),
            "an unguarded move has to say that it is unguarded, or a reader assumes a hidden rule"
        );
    }

    #[test]
    fn the_state_a_principle_times_against_is_named_in_the_instruction() {
        let document = rendered();
        assert!(
            document.contains("**Before entering `implement`, the implementation phase:**"),
            "the join is the whole point: a principle times against a phase, and the reader needs \
             the state"
        );
    }

    #[test]
    fn a_withdrawn_capability_is_stated_as_a_refusal_and_not_as_a_preference() {
        let document = rendered();
        assert!(
            document.contains("You may not use `secret.read`"),
            "`least-privilege` denies it, and an instruction document that softened that would be \
             telling a reader something the engine will not"
        );
    }

    #[test]
    fn a_terminal_state_says_that_nothing_leaves_it() {
        let document = rendered();
        assert!(document.contains("The workflow ends here."));
    }

    #[test]
    fn a_failure_policy_on_a_state_reaches_the_instructions() {
        // `adp/default` declares none, so this reads the workflow that does — the rule is only
        // load-bearing where a state carries a policy.
        let workflow = crate::testing::workflow_at("workflows/releases/progressive.yaml");
        let scene = Scene::build(&workflow, None);
        assert!(
            scene.nodes.iter().any(|node| node.on_failure.is_some()),
            "the fixture must carry a failure policy, or this asserts nothing"
        );
        let document = render(
            &scene,
            &Obligations::of(&workflow, fixture_principles().iter()),
            None,
        );
        assert!(
            document.contains(
                "If a requirement here is not met: roll back (requires \
                 deployment.previous_revision.exists)."
            ),
            "a rollback that names its precondition is the sentence a reader needs at three in the \
             morning"
        );
    }

    #[test]
    fn a_transition_failure_policy_reaches_the_instructions_too() {
        // No committed workflow declares one on a transition, so the rule is reached with a
        // document written for it rather than left uncovered.
        let workflow = {
            let raw: aep_domain::workflow::RawWorkflow = serde_yaml::from_str(
                r"
id: test/transition-failure
version: 1
title: Transition failure
initial: start
states:
  start:
    title: Start
  done:
    title: Done
    terminal: true
transitions:
  - from: start
    to: done
    when: review.approved
    on_failure:
      action: escalate
      to: oncall
",
            )
            .expect("the fixture parses");
            aep_domain::workflow::Workflow::try_from(raw)
                .unwrap_or_else(|errors| panic!("the fixture validates: {errors}"))
        };
        let scene = Scene::build(&workflow, None);
        let document = render(&scene, &Obligations::of(&workflow, []), None);
        assert!(
            document.contains("When the guard does not hold: escalate to oncall."),
            "a transition's failure policy is an instruction, not decoration: {document}"
        );
    }

    #[test]
    fn a_workflow_nothing_binds_says_so_rather_than_leaving_an_empty_heading() {
        let workflow = crate::testing::workflow_with(&["one", "two"], &[("one", "two")], "two");
        let document = render(
            &Scene::build(&workflow, None),
            &Obligations::of(&workflow, []),
            None,
        );
        assert!(
            document.contains("No principle in this tree times an obligation"),
            "an empty section reads as a missing section: {document}"
        );
    }

    #[test]
    fn an_obligation_owed_at_every_transition_says_so_without_naming_a_state() {
        let principle = principle_from(
            r"
id: always-fixture
title: Always
requires:
  always:
    predicates:
      - provenance.recorded
",
        );
        let workflow = fixture_workflow();
        let document = render(
            &Scene::build(&workflow, None),
            &Obligations::of(&workflow, [&principle]),
            None,
        );
        assert!(
            document.contains("**Owed at every transition:**"),
            "an obligation with no phase must not be written as though it had one: {document}"
        );
    }

    #[test]
    fn wrapping_breaks_only_at_spaces_and_never_loses_a_word() {
        let text = "one two three four five six seven eight nine ten";
        let lines = wrap(text, 12);
        assert!(
            lines.iter().all(|line| line.chars().count() <= 12),
            "{lines:?}"
        );
        assert_eq!(
            lines.join(" "),
            text,
            "wrapping is a rendering decision and must not edit the text"
        );
        assert_eq!(
            wrap("supercalifragilistic", 5),
            vec!["supercalifragilistic".to_owned()],
            "a word longer than the width gets a line rather than being cut in half"
        );
    }

    #[test]
    fn a_list_of_one_reads_as_a_word_and_a_list_of_three_reads_as_a_sentence() {
        assert_eq!(code_list(&["a".to_owned()]), "`a`");
        assert_eq!(code_list(&["a".to_owned(), "b".to_owned()]), "`a` and `b`");
        assert_eq!(
            code_list(&["a".to_owned(), "b".to_owned(), "c".to_owned()]),
            "`a`, `b` and `c`"
        );
    }

    #[test]
    fn the_index_names_every_document_it_was_given() {
        let workflow = fixture_workflow();
        let one = instruction(
            &Scene::build(&workflow, None),
            &Obligations::of(&workflow, fixture_principles().iter()),
            None,
        );
        assert_eq!(
            one.path, "adp/default.md",
            "the path comes from the declared id"
        );
        let listing = index(std::slice::from_ref(&one));
        assert!(listing.contains("[`adp/default.md`](adp/default.md)"));
        assert!(listing.contains("Standard development workflow"));
    }
}
