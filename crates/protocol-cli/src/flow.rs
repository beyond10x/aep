//! `protocol workflow flow` — a workflow, written as a document the b10x harness runs natively.
//!
//! # Two shapes, and the one thing that does not translate
//!
//! A workflow here is a **state graph with guards**, and it goes backwards: `adp/default/2` has
//! three edges that do. `b10x-harness-flow` is a **DAG of sub-trees**, and refuses a cycle by
//! construction — a group holds nodes, an edge may only join siblings, and there is no syntax for
//! an edge that leaves a group.
//!
//! So the translation is not edge-for-edge. A **retreat becomes a group that repeats**: every state
//! a back-edge spans is gathered into one sub-tree with a bound on its attempts. That is the
//! harness's own position on retreats (`b10x-harness-flow`'s `Repeat`), and it is the only shape
//! this graph fits into without losing the route back that `workflows/development/default.yaml`
//! spends its header arguing for.
//!
//! # What is dropped, said out loud
//!
//! * **Terminal states.** `complete` and `declined` are outcomes, not work: nothing runs in them.
//!   The emitted document therefore has no node for either, and a reader comparing the two by
//!   counting states will find the flow shorter.
//! * **Guards.** A flow node has no `when`. Every guard travels in the node's `run` payload as text
//!   for whoever binds the step, and nothing in the harness evaluates it. A run under the emitted
//!   document is *ordered* like the workflow and is **not** *governed* like it — the engine is what
//!   governs, and it is not on the other side of this translation.
//! * **The declining route.** `specify -> declined` is an early exit, and the flow notation has
//!   none: a walk runs its plan. An adapter that quietly dropped it would produce a document that
//!   cannot express *we decided not to*, so it is named here and in the emitted header rather than
//!   left for a reader to notice.
//!
//! This verb is therefore an **honest projection and not an equivalence**, which is exactly what it
//! is for: it answers *does this workflow fit that notation at all*, for free, before anything is
//! paid to run one.
//!
//! # With `--map`, a node says what is done in the state and not only which state it is
//!
//! A workflow states the order; a step map states the work. Without one the nodes carry the
//! state's own summary, which answers *does the shape fit* and nothing else — a harness reading it
//! has no prompt to send, no files to put in front of the model and no scope to hold it to. With
//! one, each node's `run` carries the step's own fields, and the mapping between the two documents
//! is this:
//!
//! | the state's steps | what the state's group holds |
//! |---|---|
//! | none, or the state is absent from the map | one node, `run` as it always was: `state` and `summary` |
//! | exactly one | one node, that step's fields beside `state` and `summary` |
//! | several | its steps, chained by `needs` in the order the map wrote them |
//!
//! # Every state is a section
//!
//! **Every non-terminal state is emitted as a group named for the state**, whatever the map gave
//! it — one node, several or none. The b10x loop asks its `transition` hook on each side of a
//! *group* boundary and nowhere else (harness design 0003 § 3), so a state that were a bare node
//! would be a state the governor is never asked about; the fifth paid native walk
//! (2026-08-29) was consulted four times, all at `root`, because no state of a bare projection was
//! a group. A group of one is the price of being governed at every state, and it is a small one.
//!
//! **The group is named for the state and not for the steps**, because it is what its neighbours'
//! `needs` name — `adversarial_verify` waits for `verify` whether `verify` holds one node or six,
//! and an edge may only join siblings. The nodes inside are `<state>-1`, `<state>-2`, … in the
//! map's order, and each carries `state:` so a reader of the record knows where it is from the
//! payload alone. A retreat span is therefore a group of groups: `implement-to-review` repeats,
//! and the four states inside it are each a section of their own.
//!
//! **Order is never sorted anywhere here.** A step map's steps are the author's order, and a
//! step's `scope:` is a first-match-wins list, so re-ordering either changes what the document
//! means rather than how it reads.
//!
//! What the map cannot do is change the shape: the states, the layering and the retreat are the
//! workflow's, and a map that named a state the workflow does not declare — or that is pinned to a
//! version other than the one being projected — is refused before anything is written, in the
//! words [`protocol drive run`](crate::drive) refuses it in.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Args;

use aep_domain::ids::StateId;
use aep_domain::workflow::Workflow;
use aep_driver_spec::map::{ScopeRule, Step, StepMap, StepMapId};
use aep_render::Layout;

/// The inputs of one projection.
#[derive(Debug, Args)]
pub(crate) struct FlowArgs {
    /// Which workflow, such as `adp/default` or `adp/default/2`.
    #[arg(long)]
    pub(crate) id: String,
    /// The document tree to load it from.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// A step map, so each node carries what a harness actually does in that state.
    ///
    /// Without one the nodes carry the state's own summary and nothing else, which is enough to
    /// answer whether the shape fits and not enough to run.
    #[arg(long)]
    pub(crate) map: Option<PathBuf>,
    /// How many attempts a retreating group may take.
    ///
    /// A number, because the notation wants one: the workflow bounds a retreat with the engine's
    /// iteration budget, which is not in the document and cannot be read off it.
    #[arg(long, default_value_t = 3)]
    pub(crate) max_attempts: u32,
    /// Where to write. Standard output when absent.
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

/// One node of the emitted document.
#[derive(Debug)]
enum Emitted<'a> {
    Step {
        id: String,
        needs: Vec<String>,
        run: Run<'a>,
    },
    Group {
        id: String,
        needs: Vec<String>,
        /// `None` for a group that is one state's steps: only a retreat repeats.
        repeat: Option<u32>,
        /// The line written above the node — the retreat's edges, or the state's summary.
        comment: Option<String>,
        nodes: Vec<Emitted<'a>>,
    },
}

/// What one node's `run` payload carries.
///
/// The step is borrowed from the map rather than copied into a shape of this module's own: the
/// fields a harness reads are the map's fields, and a second struct holding them is a second place
/// for the two to drift.
#[derive(Debug)]
struct Run<'a> {
    /// Which state of the workflow this node is in.
    state: String,
    /// The state's own summary, on a node that is the whole of the state's work — the only node
    /// of its group. Empty on a step among several, where the group carries it as a comment.
    summary: String,
    /// The step this node runs, when a map said what it is.
    step: Option<&'a Step>,
}

/// `protocol workflow flow`
pub(crate) fn flow(args: &FlowArgs) -> Result<String> {
    let documents = crate::load_documents(&args.root)?;
    let registry = documents.registry;
    let workflow = crate::render::named(&registry, &args.id, &args.root)?;
    let map = args
        .map
        .as_deref()
        .map(|named| step_map(named, &documents.drivers, workflow))
        .transpose()?;
    project(workflow, args.max_attempts, map.as_ref())
}

/// The map `--map` names: a file, or the id of one already in the document tree.
///
/// The same two forms `protocol drive run --map` takes, read the same way, so one word means one
/// thing across the two verbs that accept it.
fn step_map(
    named: &Path,
    drivers: &aep_project::load::DriverRegistry,
    workflow: &Workflow,
) -> Result<StepMap> {
    let map = if named.is_file() {
        let text = std::fs::read_to_string(named)
            .with_context(|| format!("reading {}", named.display()))?;
        aep_schema::parse::step_map(&text, Some(&named.display().to_string()))
            .map_err(|error| anyhow::anyhow!("{error}"))?
    } else {
        let id: StepMapId = named.to_string_lossy().parse().map_err(|error| {
            anyhow::anyhow!(
                "{} is not a file and not a step map id: {error}",
                named.display()
            )
        })?;
        drivers
            .get(&id)
            .with_context(|| format!("no step map `{id}` is in the document tree"))?
            .clone()
    };
    checked(map, workflow)
}

/// Refuses a map that does not apply to the workflow being projected, in the words that already
/// exist for it.
///
/// [`StepMap::cross_validate`] is where a pin and a state name are decided, and it is what the
/// driver puts every map through. A second opinion here would be a second place for the two verbs
/// to disagree about whether a document applies — and the disagreement would show up as a
/// projection that looks runnable and is not.
fn checked(map: StepMap, workflow: &Workflow) -> Result<StepMap> {
    let errors = map.cross_validate(workflow);
    if errors.is_empty() {
        return Ok(map);
    }
    bail!(
        "step map `{}` does not apply to `{}/{}`: {errors}",
        map.id,
        workflow.id,
        workflow.version
    )
}

/// Turns a workflow into the flow document, or says why it will not go.
fn project(workflow: &Workflow, max_attempts: u32, map: Option<&StepMap>) -> Result<String> {
    let layout = Layout::of(workflow);

    // The same layering the picture is drawn from, so the document and the figure agree by
    // construction rather than by two people keeping two orderings in step.
    let mut order: Vec<&StateId> = workflow
        .states
        .values()
        .filter(|state| !state.is_terminal())
        .map(|state| &state.id)
        .collect();
    order.sort_by_key(|id| (layout.layer_of(id).unwrap_or(usize::MAX), (*id).clone()));

    let position: BTreeMap<&StateId, usize> = order
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();

    if order.is_empty() {
        bail!(
            "`{}` has nothing to run: every state it declares is terminal",
            workflow.id
        );
    }

    // A retreat is any edge that lands at or before where it started. Each spans the states from
    // its target through its source, and overlapping spans are one scope: two retreats into the
    // same stretch of work are not two sections.
    let mut spans: Vec<(usize, usize, Vec<String>)> = Vec::new();
    for transition in &workflow.transitions {
        let (Some(&from), Some(&to)) =
            (position.get(&transition.from), position.get(&transition.to))
        else {
            continue;
        };
        if to <= from && layout.is_back_edge(&transition.from, &transition.to) {
            spans.push((
                to,
                from,
                vec![format!("{} -> {}", transition.from, transition.to)],
            ));
        }
    }
    let spans = merge(spans);

    let mut nodes: Vec<Emitted> = Vec::new();
    let mut index = 0;
    let mut previous: Option<String> = None;
    while index < order.len() {
        if let Some((start, end, why)) = spans.iter().find(|(start, _, _)| *start == index) {
            let inside: Vec<Emitted> = order[*start..=*end]
                .iter()
                .enumerate()
                .map(|(offset, id)| {
                    let needs = if offset == 0 {
                        Vec::new()
                    } else {
                        vec![order[start + offset - 1].to_string()]
                    };
                    node_for(workflow, map, id, needs)
                })
                .collect();
            let id = group_name(&order[*start..=*end]);
            nodes.push(Emitted::Group {
                id: id.clone(),
                needs: previous.iter().cloned().collect(),
                repeat: Some(max_attempts),
                comment: Some(format!("the retreat: {}", why.join(", "))),
                nodes: inside,
            });
            previous = Some(id);
            index = end + 1;
        } else {
            let id = order[index].to_string();
            nodes.push(node_for(
                workflow,
                map,
                order[index],
                previous.iter().cloned().collect(),
            ));
            previous = Some(id);
            index += 1;
        }
    }

    Ok(render(workflow, &nodes, &layout, map))
}

/// One state, as the section it is: a group named for it, holding its steps.
///
/// The three cases are the module's table, and all three are a group (module header, *Every
/// state is a section*). A state the map is silent about is **not** an error —
/// `StepMap::steps_for` says so, and a workflow state whose transition is unguarded needs no work
/// done in it — so its one node keeps the payload it had before there was a map at all rather
/// than disappearing or acquiring an empty one.
fn node_for<'a>(
    workflow: &Workflow,
    map: Option<&'a StepMap>,
    state: &StateId,
    needs: Vec<String>,
) -> Emitted<'a> {
    let summary = summary_of(workflow, state);
    let steps = map.map_or(&[][..], |map| map.steps_for(state));
    // A state with no step still holds one node: a group that runs nothing is refused by the
    // notation, and the node is what carries the summary a harness sends when no map gave it a
    // prompt.
    let steps: Vec<Option<&'a Step>> = if steps.is_empty() {
        vec![None]
    } else {
        steps.iter().map(Some).collect()
    };
    let alone = steps.len() == 1;
    Emitted::Group {
        id: state.to_string(),
        needs,
        // Only a retreat repeats. A state's steps run once each, in order, and a bound here
        // would re-run a command step the engine has already recorded a verdict for.
        repeat: None,
        // The summary is the state's and the group is the state. On a node that is alone it is
        // the payload; among several it would read as a description of one step, so it is the
        // group's comment instead.
        comment: (!alone && !summary.is_empty()).then(|| summary.clone()),
        nodes: steps
            .into_iter()
            .enumerate()
            .map(|(offset, step)| Emitted::Step {
                // Numbered from the state, so a node reads as what it is from its path alone:
                // a step has no id of its own in a map, and a description is prose.
                id: format!("{state}-{}", offset + 1),
                needs: if offset == 0 {
                    Vec::new()
                } else {
                    vec![format!("{state}-{offset}")]
                },
                run: Run {
                    state: state.to_string(),
                    summary: if alone {
                        summary.clone()
                    } else {
                        String::new()
                    },
                    step,
                },
            })
            .collect(),
    }
}

/// Overlapping or touching retreat spans become one, carrying every edge that made them.
fn merge(mut spans: Vec<(usize, usize, Vec<String>)>) -> Vec<(usize, usize, Vec<String>)> {
    spans.sort_by_key(|(start, end, _)| (*start, *end));
    let mut merged: Vec<(usize, usize, Vec<String>)> = Vec::new();
    for (start, end, why) in spans {
        match merged.last_mut() {
            Some((_, last_end, last_why)) if start <= *last_end + 1 => {
                *last_end = (*last_end).max(end);
                last_why.extend(why);
            }
            _ => merged.push((start, end, why)),
        }
    }
    merged
}

/// A group is named for the stretch it holds — `implement-to-review` — because a generated name
/// like `group_1` tells a reader nothing about what they are looking at.
fn group_name(states: &[&StateId]) -> String {
    match (states.first(), states.last()) {
        (Some(first), Some(last)) if first == last => format!("{first}-again"),
        (Some(first), Some(last)) => format!("{first}-to-{last}"),
        _ => "retreat".to_owned(),
    }
}

fn summary_of(workflow: &Workflow, id: &StateId) -> String {
    workflow
        .states
        .get(id)
        .and_then(|state| state.summary.clone())
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The document, written out rather than serialised, so the header carries what a projection owes
/// its reader and the key order is one a person reads top to bottom.
fn render(
    workflow: &Workflow,
    nodes: &[Emitted<'_>],
    layout: &Layout,
    map: Option<&StepMap>,
) -> String {
    let mut out = String::new();
    let dropped: Vec<String> = workflow
        .terminal_states()
        .iter()
        .map(|state| state.id.to_string())
        .collect();
    let guards = workflow
        .transitions
        .iter()
        .filter(|transition| !matches!(transition.when, aep_domain::predicate::Predicate::Always))
        .count();

    let _ = write!(out,
        "# Projected from `{}/{}` by `protocol workflow flow`. Do not edit.\n\
         #\n\
         # **An ordering, not a government.** {guards} guard(s) in the source decide whether a run\n\
         # may move; a flow node has no `when`, so none of them is here. What this document\n\
         # reproduces is the order the states run in and the routes back; what governs a run is the\n\
         # engine, which is not on this side of the translation.\n\
         #\n\
         # Dropped, because nothing runs in them: {}.\n\
         # Also dropped: any early exit. `declined` is an outcome the flow notation cannot express,\n\
         # so a run under this document always walks its whole plan.\n\
         #\n\
         # A retreat is a group that repeats, which is `b10x-harness-flow`'s own shape for one: a\n\
         # DAG has no back-edge, and every state a retreat spans is gathered into a sub-tree that\n\
         # re-runs whole. Its bound is a number given on the command line, because the source bounds\n\
         # a retreat with the engine's iteration budget and that is not in the document.\n\
         #\n\
         # Every state is a section: a group named for the state, holding its steps — one node when\n\
         # the map gave it one step or none. The loop asks its `transition` hook only at a group's\n\
         # boundaries, so this is what makes a governor's *every section* mean every state.\n",
        workflow.id,
        workflow.version.get(),
        if dropped.is_empty() {
            "none".to_owned()
        } else {
            dropped.join(", ")
        },
    );
    let _ = write!(
        out,
        "#\n# {} state(s) in {} layer(s) of the source; {} node(s) here.\n",
        workflow.states.len(),
        layout.depth(),
        nodes.len()
    );
    // Which map filled the nodes, and which workflow that map is written against. Both, because a
    // reader holding only this file cannot otherwise tell a projection of one map from a
    // projection of its sibling — `development/default` and `development/checks` are pinned to the
    // same workflow and answer the same states with different work.
    if let Some(map) = map {
        let _ = writeln!(
            out,
            "# Steps from step map `{}`, which is pinned to `{}`.",
            map.id, map.workflow
        );
    }

    let _ = writeln!(out, "id: {}", workflow.id);
    out.push_str("root:\n  id: root\n  nodes:\n");
    for node in nodes {
        write_node(&mut out, node, 4);
    }
    out
}

fn write_node(out: &mut String, node: &Emitted<'_>, indent: usize) {
    let pad = " ".repeat(indent);
    match node {
        Emitted::Step { id, needs, run } => {
            let _ = writeln!(out, "{pad}- id: {id}");
            if !needs.is_empty() {
                let _ = writeln!(out, "{pad}  needs: [{}]", needs.join(", "));
            }
            write_run(out, run, indent + 2);
        }
        Emitted::Group {
            id,
            needs,
            repeat,
            comment,
            nodes,
        } => {
            if let Some(comment) = comment {
                let _ = writeln!(out, "{pad}# {comment}");
            }
            let _ = writeln!(out, "{pad}- id: {id}");
            if !needs.is_empty() {
                let _ = writeln!(out, "{pad}  needs: [{}]", needs.join(", "));
            }
            if let Some(repeat) = repeat {
                let _ = writeln!(out, "{pad}  repeat: {{max: {repeat}}}");
            }
            let _ = writeln!(out, "{pad}  nodes:");
            for inner in nodes {
                write_node(out, inner, indent + 4);
            }
        }
    }
}

/// The `run:` payload, which is the only thing on this side of the translation a harness reads.
///
/// A node with nothing to say carries no `run:` at all rather than an empty one: a key holding
/// nothing is a claim that there was something to hold.
fn write_run(out: &mut String, run: &Run<'_>, indent: usize) {
    if run.summary.is_empty() && run.step.is_none() {
        return;
    }
    let pad = " ".repeat(indent);
    let key = " ".repeat(indent + 2);
    let _ = writeln!(out, "{pad}run:\n{key}state: {}", run.state);
    if !run.summary.is_empty() {
        let _ = writeln!(out, "{key}summary: {}", quote(&run.summary));
    }
    let Some(step) = run.step else {
        return;
    };
    let _ = writeln!(out, "{key}kind: {}", step.kind());
    match step {
        Step::Llm(llm) => {
            let _ = writeln!(out, "{key}prompt: {}", quote(&llm.prompt));
            write_list(out, indent + 2, "context", &llm.context);
            write_list(out, indent + 2, "scope", &scope_words(&llm.scope));
            let _ = writeln!(out, "{key}harness: {}", quote(&llm.harness));
            write_description(out, &key, llm.description.as_deref());
        }
        Step::Command(command) => {
            let _ = writeln!(out, "{key}command: [{}]", quoted(&command.run).join(", "));
            if let Some(evidence) = &command.evidence {
                // What running it establishes, which is the reason a command step is in a map at
                // all: the model's word for *the suite is green* is not evidence, and this is the
                // record the driver would submit for the same step.
                let inner = " ".repeat(indent + 4);
                let _ = writeln!(out, "{key}evidence:");
                let _ = writeln!(out, "{inner}kind: {}", evidence.kind.as_str());
                let _ = writeln!(out, "{inner}verifier: {}", evidence.verifier);
                if let Some(suite) = &evidence.suite {
                    let _ = writeln!(out, "{inner}suite: {}", suite.as_str());
                }
            }
            write_description(out, &key, command.description.as_deref());
        }
        Step::Operator(operator) => {
            let _ = writeln!(out, "{key}prompt: {}", quote(&operator.prompt));
            write_description(out, &key, operator.description.as_deref());
        }
    }
}

fn write_description(out: &mut String, key: &str, description: Option<&str>) {
    if let Some(description) = description {
        let _ = writeln!(out, "{key}description: {}", quote(description));
    }
}

/// A list under its key, or nothing at all when there is none.
fn write_list(out: &mut String, indent: usize, key: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    let pad = " ".repeat(indent);
    let item = " ".repeat(indent + 2);
    let _ = writeln!(out, "{pad}{key}:");
    for value in values {
        let _ = writeln!(out, "{item}- {}", quote(value));
    }
}

/// A step's write scope in the grammar `--write-scope` already has: `<glob>=<word>`.
///
/// Flattened in the order the map wrote it and **never sorted**: first match wins, so the order
/// is the rule. The word comes from the driver's own table rather than from `WriteScope`'s
/// `Serialize`, so a projection and a driven run cannot come to spell the same scope differently.
fn scope_words(scope: &[ScopeRule]) -> Vec<String> {
    scope
        .iter()
        .flat_map(|rule| {
            rule.paths
                .iter()
                .map(move |path| format!("{path}={}", crate::drive::write_scope_word(rule.write)))
        })
        .collect()
}

fn quoted(values: &[String]) -> Vec<String> {
    values.iter().map(|value| quote(value)).collect()
}

/// YAML-safe, and nothing clever: a summary is one line of prose.
///
/// A prompt is not always one line — a map may write it with `|` — so the three characters that
/// would end the scalar early are escaped rather than assumed absent.
fn quote(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', "\\r")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workflow() -> Workflow {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../workflows/development/default.yaml"),
        )
        .expect("the repository's own development workflow");
        let raw: aep_domain::raw::RawWorkflow = serde_yaml::from_str(&text).expect("parses");
        Workflow::try_from(raw).expect("validates")
    }

    #[test]
    fn every_state_that_runs_lands_somewhere_and_the_terminals_do_not() {
        let workflow = workflow();
        let document = project(&workflow, 3, None).expect("projects");

        for state in workflow.states.values() {
            let named = document.contains(&format!("id: {}", state.id));
            if state.is_terminal() {
                assert!(!named, "`{}` is an outcome, not work", state.id);
            } else {
                assert!(named, "`{}` runs and must be somewhere", state.id);
            }
        }
    }

    #[test]
    fn the_three_retreats_become_one_repeating_group_because_they_span_one_stretch() {
        // `verify -> implement`, `adversarial_verify -> implement` and `review -> implement` all
        // land in the same place. Two retreats into one stretch of work are one section, not two.
        let document = project(&workflow(), 3, None).expect("projects");
        assert_eq!(
            document.matches("repeat: {max: 3}").count(),
            1,
            "one scope, not three:\n{document}"
        );
        assert!(
            document.contains("id: implement-to-review"),
            "and it is named for the stretch it holds:\n{document}"
        );
        for state in ["implement", "verify", "adversarial_verify", "review"] {
            assert!(document.contains(&format!("id: {state}")), "{state}");
        }
    }

    #[test]
    fn the_document_says_what_it_dropped_rather_than_dropping_it_quietly() {
        let document = project(&workflow(), 3, None).expect("projects");
        assert!(
            document.contains("An ordering, not a government"),
            "{document}"
        );
        assert!(
            document.contains("declined"),
            "the early exit is named: {document}"
        );
        assert!(
            document.contains("complete"),
            "the terminals are named: {document}"
        );
    }

    #[test]
    fn what_this_verb_emits_is_a_document_the_harness_plans() {
        // The contract between the two repositories, checked from this side: the projection is
        // valid YAML with the keys the notation declares, in the nesting it declares them in.
        let document = project(&workflow(), 3, None).expect("projects");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&document).expect("valid YAML");
        assert!(parsed.get("id").is_some());
        let nodes = parsed
            .get("root")
            .and_then(|root| root.get("nodes"))
            .and_then(|nodes| nodes.as_sequence())
            .expect("root.nodes is a sequence");
        assert!(nodes.iter().all(|node| node.get("id").is_some()));
        let group = nodes
            .iter()
            .find(|node| node.get("repeat").is_some())
            .expect("the retreating group");
        assert!(group.get("nodes").is_some(), "a group holds nodes");
        assert_eq!(group["repeat"]["max"].as_u64(), Some(3));
    }

    /// The projection without a map, byte for byte — the document the b10x harness commits as
    /// `harness-flow/fixtures/adp-default.projected.yaml` and walks in its own suite. A golden
    /// rather than a property, because *these bytes* are what the other repository holds: when
    /// this changes, that fixture is refreshed from this verb, not edited by hand.
    const NO_MAP: &str = r#"# Projected from `adp/default/2` by `protocol workflow flow`. Do not edit.
#
# **An ordering, not a government.** 11 guard(s) in the source decide whether a run
# may move; a flow node has no `when`, so none of them is here. What this document
# reproduces is the order the states run in and the routes back; what governs a run is the
# engine, which is not on this side of the translation.
#
# Dropped, because nothing runs in them: complete, declined.
# Also dropped: any early exit. `declined` is an outcome the flow notation cannot express,
# so a run under this document always walks its whole plan.
#
# A retreat is a group that repeats, which is `b10x-harness-flow`'s own shape for one: a
# DAG has no back-edge, and every state a retreat spans is gathered into a sub-tree that
# re-runs whole. Its bound is a number given on the command line, because the source bounds
# a retreat with the engine's iteration budget and that is not in the document.
#
# Every state is a section: a group named for the state, holding its steps — one node when
# the map gave it one step or none. The loop asks its `transition` hook only at a group's
# boundaries, so this is what makes a governor's *every section* mean every state.
#
# 10 state(s) in 9 layer(s) of the source; 5 node(s) here.
id: adp/default
root:
  id: root
  nodes:
    - id: receive
      nodes:
        - id: receive-1
          run:
            state: receive
            summary: "Take in the request and record what is being asked for, unedited."
    - id: specify
      needs: [receive]
      nodes:
        - id: specify-1
          run:
            state: specify
            summary: "State the required behaviour. Anything left only in the prompt cannot be checked later, so this is where an objective becomes something a verifier can disagree with."
    - id: decompose
      needs: [specify]
      nodes:
        - id: decompose-1
          run:
            state: decompose
            summary: "Break the specification into units that can each be implemented and verified on their own."
    - id: establish_verifiers
      needs: [decompose]
      nodes:
        - id: establish_verifiers-1
          run:
            state: establish_verifiers
            summary: "Write the tests, contracts and checks that will decide whether the work is done — before there is an implementation for them to be shaped around."
    # the retreat: verify -> implement, adversarial_verify -> implement, review -> implement
    - id: implement-to-review
      needs: [establish_verifiers]
      repeat: {max: 3}
      nodes:
        - id: implement
          nodes:
            - id: implement-1
              run:
                state: implement
                summary: "Make the smallest change that satisfies the decomposed unit and its verifiers."
        - id: verify
          needs: [implement]
          nodes:
            - id: verify-1
              run:
                state: verify
                summary: "Run the verifiers established earlier against the change."
        - id: adversarial_verify
          needs: [verify]
          nodes:
            - id: adversarial_verify-1
              run:
                state: adversarial_verify
                summary: "Try to break what the previous state just declared working: mutants, edge cases, property violations, contract drift."
        - id: review
          needs: [adversarial_verify]
          nodes:
            - id: review-1
              run:
                state: review
                summary: "Judge the change as a whole — intent, fit and risk, not just green checks."
"#;

    /// The two shipped maps, read from the tree they ship in and checked against the workflow they
    /// are pinned to — which is the same route `--map <file>` takes.
    fn shipped(name: &str) -> StepMap {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../drivers/development")
            .join(name);
        let text = std::fs::read_to_string(&path).expect("the repository's own step map");
        let map = aep_schema::parse::step_map(&text, Some(name)).expect("parses");
        checked(map, &workflow()).expect("applies to the workflow it is pinned to")
    }

    /// A map naming one state, pinned to whatever the caller says, so the two rules that need a
    /// map the tree does not hold — the wrong pin, and a state the map is silent about — have one.
    fn one_state(pin: &str) -> StepMap {
        let text = format!(
            "format: aep.driver-steps/1\n\
             id: development/oneshot\n\
             workflow: {pin}\n\
             states:\n  \
               receive:\n    \
                 steps:\n      \
                   - kind: llm\n        \
                       prompt: Read the task and write down what was asked for.\n"
        );
        aep_schema::parse::step_map(&text, None).expect("parses")
    }

    /// Every `run:` payload in the document, by the id of the node carrying it.
    fn runs(document: &str) -> BTreeMap<String, serde_yaml::Value> {
        let parsed: serde_yaml::Value = serde_yaml::from_str(document).expect("valid YAML");
        let mut found = BTreeMap::new();
        gather(&parsed["root"], &mut found);
        found
    }

    fn gather(node: &serde_yaml::Value, into: &mut BTreeMap<String, serde_yaml::Value>) {
        if let Some(nodes) = node.get("nodes").and_then(serde_yaml::Value::as_sequence) {
            for inner in nodes {
                gather(inner, into);
            }
        }
        if let (Some(id), Some(run)) = (
            node.get("id").and_then(serde_yaml::Value::as_str),
            node.get("run"),
        ) {
            into.insert(id.to_owned(), run.clone());
        }
    }

    fn keys(run: &serde_yaml::Value) -> Vec<String> {
        run.as_mapping()
            .expect("a run is a mapping")
            .keys()
            .filter_map(|key| key.as_str().map(str::to_owned))
            .collect()
    }

    fn strings(run: &serde_yaml::Value, key: &str) -> Vec<String> {
        run.get(key)
            .and_then(serde_yaml::Value::as_sequence)
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect()
    }

    /// The projection the other repository holds as its fixture, byte by byte.
    ///
    /// `--map` is an addition and not a change: a caller who does not pass one gets the same shape
    /// with the same payloads, and `harness-flow`'s `fixtures/adp-default.projected.yaml` — which
    /// is these bytes — keeps walking.
    #[test]
    fn a_projection_without_a_map_is_the_document_it_always_was() {
        assert_eq!(project(&workflow(), 3, None).expect("projects"), NO_MAP);
    }

    /// The other half of it: nothing a harness could *run* appears without a map, and the shape is
    /// the fixture's — five sections, the last a repeating group of four sections.
    #[test]
    fn without_a_map_a_node_says_which_state_it_is_and_nothing_more() {
        let document = project(&workflow(), 3, None).expect("projects");
        for (id, run) in runs(&document) {
            assert_eq!(
                keys(&run),
                ["state", "summary"],
                "`{id}` carries something no map put there"
            );
        }

        let parsed: serde_yaml::Value = serde_yaml::from_str(&document).expect("valid YAML");
        let nodes = parsed["root"]["nodes"]
            .as_sequence()
            .expect("root.nodes is a sequence");
        let ids: Vec<&str> = nodes
            .iter()
            .filter_map(|node| node["id"].as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "receive",
                "specify",
                "decompose",
                "establish_verifiers",
                "implement-to-review"
            ]
        );
        let group = nodes.last().expect("the retreat");
        assert_eq!(group["repeat"]["max"].as_u64(), Some(3));
        assert_eq!(
            group["nodes"].as_sequence().map(Vec::len),
            Some(4),
            "the four states the retreat spans"
        );
    }

    /// The reason a one-step state is a group of one: the b10x loop asks its `transition` hook at
    /// a group boundary and nowhere else, so a state that were a bare node would be a state the
    /// governor is never asked about. With or without a map, every non-terminal state is a group
    /// named for it, every node inside is `<state>-<n>` and says which state it is in, and only a
    /// retreat repeats.
    #[test]
    fn every_state_is_a_section_so_a_governor_asked_at_every_section_is_asked_at_every_state() {
        let workflow = workflow();
        let map = shipped("default.yaml");
        for (name, map) in [("without a map", None), ("with a map", Some(&map))] {
            let document = project(&workflow, 3, map).expect("projects");
            let parsed: serde_yaml::Value = serde_yaml::from_str(&document).expect("valid YAML");
            let mut sections: Vec<(String, serde_yaml::Value)> = Vec::new();
            groups(&parsed["root"], &mut sections);
            for state in workflow
                .states
                .values()
                .filter(|state| !state.is_terminal())
            {
                let (_, group) = sections
                    .iter()
                    .find(|(id, _)| *id == state.id.to_string())
                    .unwrap_or_else(|| panic!("{name}: `{}` is not a section", state.id));
                assert!(
                    group.get("repeat").is_none(),
                    "{name}: only a retreat repeats"
                );
                let inside = group["nodes"].as_sequence().expect("a group holds nodes");
                assert!(!inside.is_empty(), "{name}: a section that runs nothing");
                for (offset, node) in inside.iter().enumerate() {
                    assert_eq!(
                        node["id"].as_str(),
                        Some(format!("{}-{}", state.id, offset + 1).as_str()),
                        "{name}: numbered from the state"
                    );
                    assert_eq!(
                        node["run"]["state"].as_str(),
                        Some(state.id.to_string().as_str()),
                        "{name}: every node says which state it is in"
                    );
                }
            }
            let repeating: Vec<&str> = sections
                .iter()
                .filter(|(_, group)| group.get("repeat").is_some())
                .map(|(id, _)| id.as_str())
                .collect();
            assert_eq!(repeating, ["implement-to-review"], "{name}");
        }
    }

    /// Every group below `root`, by id, in document order.
    fn groups(node: &serde_yaml::Value, into: &mut Vec<(String, serde_yaml::Value)>) {
        let Some(nodes) = node.get("nodes").and_then(serde_yaml::Value::as_sequence) else {
            return;
        };
        for inner in nodes {
            if inner.get("nodes").is_some() {
                if let Some(id) = inner.get("id").and_then(serde_yaml::Value::as_str) {
                    into.push((id.to_owned(), inner.clone()));
                }
                groups(inner, into);
            }
        }
    }

    /// What `--map` is for: a node a harness can act on rather than a name it can print.
    #[test]
    fn a_state_with_one_llm_step_carries_the_prompt_the_scope_and_the_harness() {
        let map = shipped("default.yaml");
        let document = project(&workflow(), 3, Some(&map)).expect("projects");
        let runs = runs(&document);

        for state in ["receive", "specify", "decompose"] {
            let run = runs.get(&format!("{state}-1")).expect(state);
            assert_eq!(
                keys(run),
                [
                    "state",
                    "summary",
                    "kind",
                    "prompt",
                    "scope",
                    "harness",
                    "description"
                ],
                "{state}"
            );
            assert_eq!(run["kind"].as_str(), Some("llm"), "{state}");
            assert_eq!(run["harness"].as_str(), Some("b10x"), "{state}");
            assert!(
                run["prompt"].as_str().is_some_and(|text| !text.is_empty()),
                "{state} has nothing to send"
            );
            // **No `context:` any more, and its absence is the assertion.** The map handed every
            // `llm` step the planning skill eagerly while neither harness could read a plugin's
            // skills; both read it now, so the document is one `skill` call away instead of being
            // billed on every turn of a stateless loop. A `context` key reappearing here is the
            // map regressing to eager delivery, not a test that needs relaxing.
            assert!(
                run.get("context").is_none(),
                "{state} carries a context file the map no longer declares"
            );
            assert!(!strings(run, "scope").is_empty(), "{state} is unscoped");
        }
    }

    /// A scope is first-match-wins, so its order is the rule and not the presentation. The map
    /// writes the planning store's `denied` **before** the catch-all and before the `allowed`
    /// tree; sorted, `.engineering/planning/**` would fall after `crates/**` and the rule that
    /// keeps the store out of a native writer's hands would be the second one consulted.
    #[test]
    fn the_scope_of_a_step_is_written_in_the_order_the_map_wrote_it() {
        let map = shipped("default.yaml");
        let document = project(&workflow(), 3, Some(&map)).expect("projects");
        let scope = strings(runs(&document).get("receive-1").expect("receive"), "scope");
        assert_eq!(
            scope,
            [
                ".engineering/planning/**=denied",
                "crates/**=allowed",
                "docs/**=allowed",
                "conformance/**=allowed",
                "drivers/**=allowed",
                "**=denied",
            ],
            "the map's order, flattened and never sorted"
        );

        let mut sorted = scope.clone();
        sorted.sort();
        assert_ne!(
            scope, sorted,
            "a fixture that is already in sorted order would not catch a sort"
        );
    }

    /// A state whose map entry holds several steps is a sub-tree, chained in the order the author
    /// wrote them: `establish_verifiers` writes the tests, runs them red, and then asks a person.
    #[test]
    fn a_state_with_several_steps_becomes_a_group_chained_in_the_order_the_map_wrote_them() {
        let map = shipped("default.yaml");
        let document = project(&workflow(), 3, Some(&map)).expect("projects");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&document).expect("valid YAML");
        let group = parsed["root"]["nodes"]
            .as_sequence()
            .expect("nodes")
            .iter()
            .find(|node| node["id"].as_str() == Some("establish_verifiers"))
            .expect("the state is a group now");
        assert!(
            group.get("repeat").is_none(),
            "only a retreat repeats: {group:?}"
        );

        let inside = group["nodes"].as_sequence().expect("a group holds nodes");
        let ids: Vec<&str> = inside
            .iter()
            .filter_map(|node| node["id"].as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "establish_verifiers-1",
                "establish_verifiers-2",
                "establish_verifiers-3"
            ]
        );
        assert!(
            inside[0].get("needs").is_none(),
            "the first waits for nobody"
        );
        assert_eq!(
            strings(&inside[1], "needs"),
            ["establish_verifiers-1"],
            "the chain is the map's order"
        );
        assert_eq!(strings(&inside[2], "needs"), ["establish_verifiers-2"]);

        let kinds: Vec<&str> = inside
            .iter()
            .filter_map(|node| node["run"]["kind"].as_str())
            .collect();
        assert_eq!(kinds, ["llm", "command", "operator"]);
        for node in inside {
            assert_eq!(
                node["run"]["state"].as_str(),
                Some("establish_verifiers"),
                "every step names the state it is in"
            );
        }
    }

    /// A command step travels as its argv and as what running it establishes — the two facts a
    /// model's own account of a test run cannot supply.
    #[test]
    fn a_command_step_carries_its_argv_and_the_evidence_running_it_establishes() {
        let map = shipped("default.yaml");
        let document = project(&workflow(), 3, Some(&map)).expect("projects");
        let run = runs(&document)
            .remove("establish_verifiers-2")
            .expect("the suite step");
        assert_eq!(run["kind"].as_str(), Some("command"));
        assert_eq!(strings(&run, "command"), ["cargo", "test", "--workspace"]);
        assert_eq!(run["evidence"]["kind"].as_str(), Some("test_result"));
        assert_eq!(run["evidence"]["verifier"].as_str(), Some("test-runner"));
        assert_eq!(run["evidence"]["suite"].as_str(), Some("unit"));
    }

    /// An operator step travels as what it asks the person for. `review` is one step, and it is
    /// still a section of one: the governor is asked on both sides of it like every other state.
    #[test]
    fn an_operator_step_carries_what_it_asks_and_a_state_of_one_step_is_a_section_of_one() {
        let map = shipped("default.yaml");
        let document = project(&workflow(), 3, Some(&map)).expect("projects");
        let run = runs(&document).remove("review-1").expect("review");
        assert_eq!(run["kind"].as_str(), Some("operator"));
        assert!(
            run["prompt"]
                .as_str()
                .is_some_and(|text| text.contains("Review this change as a whole")),
            "{run:?}"
        );
    }

    /// A map is not obliged to answer every state, so a state it is silent about keeps the payload
    /// it had before there was a map — and does not acquire an empty one that reads like a step.
    #[test]
    fn a_state_the_map_is_silent_about_keeps_the_payload_it_always_had() {
        let map = checked(one_state("adp/default/2"), &workflow()).expect("applies");
        let document = project(&workflow(), 3, Some(&map)).expect("projects");
        let runs = runs(&document);
        assert_eq!(runs["receive-1"]["kind"].as_str(), Some("llm"));
        assert_eq!(
            keys(&runs["specify-1"]),
            ["state", "summary"],
            "the map says nothing about `specify`"
        );
    }

    /// The refusal is the driver's, and it is the driver's on purpose: `cross_validate` is the one
    /// place that decides whether a map applies, so `protocol workflow flow` and
    /// `protocol drive run` cannot come to different answers about the same two documents.
    #[test]
    fn a_map_pinned_to_another_version_is_refused_in_the_words_the_driver_refuses_it_in() {
        let refusal = checked(one_state("adp/default/1"), &workflow())
            .expect_err("the workflow in the tree is at version 2")
            .to_string();
        assert!(
            refusal.contains(
                "the map pins `adp/default/1` and the workflow in the tree is at version 2"
            ),
            "{refusal}"
        );
        assert!(
            refusal.contains("development/oneshot"),
            "and it names the document that will not apply: {refusal}"
        );
    }

    /// The header says which map filled the nodes, because two maps fit `adp/default/2` and
    /// answer the same states with different work.
    #[test]
    fn the_header_names_the_map_and_the_workflow_it_is_pinned_to() {
        let map = shipped("checks.yaml");
        let document = project(&workflow(), 3, Some(&map)).expect("projects");
        assert!(
            document.contains(
                "# Steps from step map `development/checks`, which is pinned to `adp/default/2`."
            ),
            "{document}"
        );
    }

    /// The contract with the other repository, checked with a map in place: a node's payload grew
    /// and the document is still a flow — `id`, a `root` group of nodes, `needs`, `repeat: {max}`
    /// and a `run` on every leaf that runs something.
    #[test]
    fn a_projection_with_a_map_is_still_a_document_the_harness_plans() {
        for name in ["default.yaml", "checks.yaml"] {
            let map = shipped(name);
            let document = project(&workflow(), 3, Some(&map)).expect("projects");
            let parsed: serde_yaml::Value =
                serde_yaml::from_str(&document).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(parsed["id"].as_str(), Some("adp/default"), "{name}");
            assert_eq!(parsed["root"]["id"].as_str(), Some("root"), "{name}");
            let nodes = parsed["root"]["nodes"]
                .as_sequence()
                .expect("root.nodes is a sequence");
            assert!(nodes.iter().all(|node| node.get("id").is_some()), "{name}");
            let group = nodes
                .iter()
                .find(|node| node.get("repeat").is_some())
                .expect("the retreating group");
            assert_eq!(group["repeat"]["max"].as_u64(), Some(3), "{name}");
            assert!(group.get("nodes").is_some(), "{name}: a group holds nodes");
            for (id, run) in runs(&document) {
                assert!(
                    run.get("state").is_some(),
                    "{name}: `{id}` carries a payload naming no state"
                );
            }
        }
    }

    #[test]
    fn a_bound_the_caller_names_is_the_bound_that_is_written() {
        let document = project(&workflow(), 7, None).expect("projects");
        assert!(document.contains("repeat: {max: 7}"), "{document}");
    }
}
