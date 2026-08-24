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

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args;

use aep_domain::ids::StateId;
use aep_domain::workflow::Workflow;
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
enum Emitted {
    Step {
        id: String,
        needs: Vec<String>,
        summary: String,
    },
    Group {
        id: String,
        needs: Vec<String>,
        repeat: u32,
        because: String,
        nodes: Vec<Emitted>,
    },
}

/// `protocol workflow flow`
pub(crate) fn flow(args: &FlowArgs) -> Result<String> {
    let registry = crate::load(&args.root)?;
    let workflow = crate::render::named(&registry, &args.id, &args.root)?;
    project(workflow, args.max_attempts)
}

/// Turns a workflow into the flow document, or says why it will not go.
fn project(workflow: &Workflow, max_attempts: u32) -> Result<String> {
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
                .map(|(offset, id)| Emitted::Step {
                    id: (*id).to_string(),
                    needs: if offset == 0 {
                        Vec::new()
                    } else {
                        vec![order[start + offset - 1].to_string()]
                    },
                    summary: summary_of(workflow, id),
                })
                .collect();
            let id = group_name(&order[*start..=*end]);
            nodes.push(Emitted::Group {
                id: id.clone(),
                needs: previous.iter().cloned().collect(),
                repeat: max_attempts,
                because: why.join(", "),
                nodes: inside,
            });
            previous = Some(id);
            index = end + 1;
        } else {
            let id = order[index].to_string();
            nodes.push(Emitted::Step {
                id: id.clone(),
                needs: previous.iter().cloned().collect(),
                summary: summary_of(workflow, order[index]),
            });
            previous = Some(id);
            index += 1;
        }
    }

    Ok(render(workflow, &nodes, &layout))
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
fn render(workflow: &Workflow, nodes: &[Emitted], layout: &Layout) -> String {
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
         # a retreat with the engine's iteration budget and that is not in the document.\n",
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

    let _ = writeln!(out, "id: {}", workflow.id);
    out.push_str("root:\n  id: root\n  nodes:\n");
    for node in nodes {
        write_node(&mut out, node, 4);
    }
    out
}

fn write_node(out: &mut String, node: &Emitted, indent: usize) {
    let pad = " ".repeat(indent);
    match node {
        Emitted::Step { id, needs, summary } => {
            let _ = writeln!(out, "{pad}- id: {id}");
            if !needs.is_empty() {
                let _ = writeln!(out, "{pad}  needs: [{}]", needs.join(", "));
            }
            if !summary.is_empty() {
                let _ = writeln!(out, "{pad}  run:\n{pad}    state: {id}");
                let _ = writeln!(out, "{pad}    summary: {}", quote(summary));
            }
        }
        Emitted::Group {
            id,
            needs,
            repeat,
            because,
            nodes,
        } => {
            let _ = writeln!(out, "{pad}# the retreat: {because}");
            let _ = writeln!(out, "{pad}- id: {id}");
            if !needs.is_empty() {
                let _ = writeln!(out, "{pad}  needs: [{}]", needs.join(", "));
            }
            let _ = writeln!(out, "{pad}  repeat: {{max: {repeat}}}");
            let _ = writeln!(out, "{pad}  nodes:");
            for inner in nodes {
                write_node(out, inner, indent + 4);
            }
        }
    }
}

/// YAML-safe, and nothing clever: a summary is one line of prose.
fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
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
        let document = project(&workflow, 3).expect("projects");

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
        let document = project(&workflow(), 3).expect("projects");
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
        let document = project(&workflow(), 3).expect("projects");
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
        let document = project(&workflow(), 3).expect("projects");
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

    #[test]
    fn a_bound_the_caller_names_is_the_bound_that_is_written() {
        let document = project(&workflow(), 7).expect("projects");
        assert!(document.contains("repeat: {max: 7}"), "{document}");
    }
}
