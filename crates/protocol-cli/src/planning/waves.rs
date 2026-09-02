//! Waves, derived from declared scope and `depends_on`.
//!
//! A wave is the claim that N units of work may run at once, and the property it rests on is that
//! they touch different surfaces. Until a story could declare a scope that claim was a pairwise
//! reading of prose — the wave skill's own selection step says "name the overlap risk honestly, per
//! pair" — and an unassessed story read exactly like a safe one.
//!
//! # What is decided here and what is not
//!
//! This module is a function. It is handed candidates, each with its declared surfaces and its
//! `depends_on` edges, and it returns the waves, the pairs it kept apart, the stories it could not
//! assess, and any cycle it found. It reads no store, opens no file, prints nothing and decides
//! nothing about **whether a wave should be run** — `story:wave-as-a-surface` puts that judgement
//! in front of the operator on purpose, and this keeps it there by reporting the collision rather
//! than resolving it.
//!
//! Ordered collections throughout, so the same store yields the same bytes: that is what makes a
//! recorded fixture answer something a test can compare against.
//!
//! # The three rules
//!
//! 1. **Inside a wave, no two stories share a scope path.** A story that would collide is placed in
//!    the first later wave where it does not, and the pair is reported.
//! 2. **A dependency is an ordering constraint, never a filter.** A story is placed strictly after
//!    every story it depends on — including through a story that was not itself placed, so an
//!    unassessed link in the chain cannot make a dependency disappear.
//! 3. **A story with no scope is never placed.** It is listed as unassessed, because a wave that
//!    silently included it would be resting on a surface nobody established.

use std::collections::{BTreeMap, BTreeSet};

use aep_domain::artifact::{ScopeConfidence, ScopeEntry};

/// One artifact the derivation was asked about.
#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    /// Its id, such as `story:passkey-login`.
    pub(crate) id: String,
    /// The surfaces it declares, ordered by path and one entry per path.
    pub(crate) scope: Vec<ScopeEntry>,
    /// The ids it declares a `depends_on` edge to, whether or not they are candidates too.
    pub(crate) depends_on: BTreeSet<String>,
}

/// What the derivation answered.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Derivation {
    /// The waves, in the order they must run. Never holds an empty wave.
    pub(crate) waves: Vec<Wave>,
    /// Every pair that shares a path, with the path — what kept them apart.
    pub(crate) collisions: Vec<Collision>,
    /// The candidates that declare no scope, in id order. Never placed.
    pub(crate) unassessed: Vec<String>,
    /// Every `depends_on` cycle among the candidates. A non-empty list means the waves are not
    /// derivable and the verb exits 2.
    pub(crate) cycles: Vec<Vec<String>>,
}

/// One wave: the stories that may be worked at once.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Wave {
    /// Which wave, counting from 1 as a reader does.
    pub(crate) wave: usize,
    /// Its members, in id order.
    pub(crate) artifacts: Vec<Placed>,
}

/// One story in a wave.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Placed {
    /// Its id.
    pub(crate) id: String,
    /// Whether any of its surfaces was worked out rather than read.
    ///
    /// Always written, never omitted when false: *this wave rests on two cited surfaces and one
    /// inferred one* is a sentence an operator can act on, and it is only available to a reader
    /// who can see which member is which.
    pub(crate) inferred: bool,
    /// The surfaces it declares.
    pub(crate) scope: Vec<ScopeEntry>,
}

/// One pair kept apart, and the path that did it.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Collision {
    /// The lower of the two ids.
    pub(crate) a: String,
    /// The higher of the two ids.
    pub(crate) b: String,
    /// The path they both declare.
    pub(crate) path: String,
    /// `inferred` when either side worked the path out rather than reading it.
    ///
    /// An inferred entry **counts** as a collision and says so. Treating it as unknown instead
    /// would put the pair in one wave on the strength of a guess, which is the failure this whole
    /// derivation exists against.
    pub(crate) confidence: ScopeConfidence,
}

/// Derives the waves.
///
/// `candidates` is whatever the caller selected — a kind, a status, or the whole store. Everything
/// outside it is invisible here, including a `depends_on` edge that leaves the set: the verb
/// answers about what it was asked to sequence, and an edge to something it was not asked about is
/// not an ordering it can honour.
pub(crate) fn derive(candidates: &[Candidate]) -> Derivation {
    let held: BTreeSet<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    let deps: BTreeMap<&str, BTreeSet<&str>> = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.id.as_str(),
                candidate
                    .depends_on
                    .iter()
                    .map(String::as_str)
                    .filter(|target| held.contains(target))
                    .collect(),
            )
        })
        .collect();

    let cycles = cycles_in(&deps);
    if !cycles.is_empty() {
        // No ordering exists, so none is invented. The ids are the answer.
        return Derivation {
            waves: Vec::new(),
            collisions: Vec::new(),
            unassessed: Vec::new(),
            cycles,
        };
    }

    let by_id: BTreeMap<&str, &Candidate> = candidates
        .iter()
        .map(|candidate| (candidate.id.as_str(), candidate))
        .collect();
    let assessable: BTreeSet<&str> = candidates
        .iter()
        .filter(|candidate| !candidate.scope.is_empty())
        .map(|candidate| candidate.id.as_str())
        .collect();

    // The assessable stories each candidate must follow, looking **through** the ones that will not
    // be placed: `x depends_on u`, `u depends_on b`, `u` unassessed, still means `x` comes after
    // `b`. Without this an unassessed link in a chain would quietly drop a real ordering.
    let mut frontier: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for id in deps.keys() {
        frontier_of(id, &deps, &assessable, &mut frontier);
    }

    let mut depth: BTreeMap<&str, usize> = BTreeMap::new();
    for id in &assessable {
        depth_of(id, &frontier, &mut depth);
    }
    let mut order: Vec<&str> = assessable.iter().copied().collect();
    order.sort_by_key(|id| (depth.get(id).copied().unwrap_or(0), *id));

    let waves = pack(&order, &frontier, &by_id);

    Derivation {
        waves,
        collisions: collisions_among(&assessable, &by_id),
        unassessed: candidates
            .iter()
            .filter(|candidate| candidate.scope.is_empty())
            .map(|candidate| candidate.id.clone())
            .collect(),
        cycles: Vec::new(),
    }
}

/// Packs the ordered stories into waves, greedily and never before what they depend on.
///
/// `order` is topological: every story appears after everything in its frontier, so the floor
/// below is read off placements already made. Inside a wave no two stories share a path, and a
/// story that would collide takes the **first later wave with room** rather than one of its own —
/// two waves of one where one wave of two was available is a coordinator running twice.
fn pack<'a>(
    order: &[&'a str],
    frontier: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    by_id: &BTreeMap<&'a str, &Candidate>,
) -> Vec<Wave> {
    let mut placed_in: BTreeMap<&str, usize> = BTreeMap::new();
    let mut members: Vec<Vec<&str>> = Vec::new();
    let mut surfaces: Vec<BTreeSet<&str>> = Vec::new();
    for id in order {
        let floor = frontier
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|ahead| placed_in.get(ahead).map(|wave| wave + 1))
            .max()
            .unwrap_or(0);
        let paths: Vec<&str> = by_id[id]
            .scope
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        let mut wave = floor;
        loop {
            while members.len() <= wave {
                members.push(Vec::new());
                surfaces.push(BTreeSet::new());
            }
            if paths.iter().all(|path| !surfaces[wave].contains(path)) {
                break;
            }
            wave += 1;
        }
        members[wave].push(id);
        surfaces[wave].extend(paths);
        placed_in.insert(id, wave);
    }

    members
        .into_iter()
        .enumerate()
        .map(|(index, mut ids)| {
            ids.sort_unstable();
            Wave {
                wave: index + 1,
                artifacts: ids
                    .into_iter()
                    .map(|id| Placed {
                        id: id.to_owned(),
                        inferred: by_id[id]
                            .scope
                            .iter()
                            .any(|entry| entry.confidence == ScopeConfidence::Inferred),
                        scope: by_id[id].scope.clone(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Every pair of assessable candidates that shares a path, once per shared path.
///
/// Every such pair is reported, not only the pairs the packing happened to try: a pair separated by
/// a dependency is still a pair that could never have run together, and a reader deciding whether
/// to override the derivation needs the whole list rather than the part the greedy order reached.
fn collisions_among(
    assessable: &BTreeSet<&str>,
    by_id: &BTreeMap<&str, &Candidate>,
) -> Vec<Collision> {
    let ids: Vec<&str> = assessable.iter().copied().collect();
    let mut collisions = Vec::new();
    for (index, a) in ids.iter().enumerate() {
        for b in ids.iter().skip(index + 1) {
            for left in &by_id[a].scope {
                let Some(right) = by_id[b]
                    .scope
                    .iter()
                    .find(|entry| entry.path == left.path)
                else {
                    continue;
                };
                collisions.push(Collision {
                    a: (*a).to_owned(),
                    b: (*b).to_owned(),
                    path: left.path.clone(),
                    confidence: if left.confidence == ScopeConfidence::Inferred
                        || right.confidence == ScopeConfidence::Inferred
                    {
                        ScopeConfidence::Inferred
                    } else {
                        ScopeConfidence::Cited
                    },
                });
            }
        }
    }
    collisions
}

/// Fills `into` with the assessable stories reachable from `id` through `depends_on`.
fn frontier_of<'a>(
    id: &'a str,
    deps: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    assessable: &BTreeSet<&'a str>,
    into: &mut BTreeMap<&'a str, BTreeSet<&'a str>>,
) -> BTreeSet<&'a str> {
    if let Some(known) = into.get(id) {
        return known.clone();
    }
    let mut ahead: BTreeSet<&str> = BTreeSet::new();
    for target in deps.get(id).into_iter().flatten() {
        if assessable.contains(target) {
            ahead.insert(target);
        } else {
            ahead.extend(frontier_of(target, deps, assessable, into));
        }
    }
    into.insert(id, ahead.clone());
    ahead
}

/// The longest chain of assessable dependencies behind `id`, which is the earliest wave it can be
/// considered for.
fn depth_of<'a>(
    id: &'a str,
    frontier: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    into: &mut BTreeMap<&'a str, usize>,
) -> usize {
    if let Some(known) = into.get(id) {
        return *known;
    }
    let deepest = frontier
        .get(id)
        .into_iter()
        .flatten()
        .map(|ahead| depth_of(ahead, frontier, into) + 1)
        .max()
        .unwrap_or(0);
    into.insert(id, deepest);
    deepest
}

/// Every `depends_on` cycle, each written from its lowest id and closed by repeating it.
///
/// A three-colour depth-first search: one cycle per back edge, and a node explored once. Reported
/// rather than cut, because which edge of a cycle is the wrong one is a judgement about the plan
/// and not something a topological sort gets to make.
fn cycles_in(deps: &BTreeMap<&str, BTreeSet<&str>>) -> Vec<Vec<String>> {
    let mut done: BTreeSet<&str> = BTreeSet::new();
    let mut found: BTreeSet<Vec<String>> = BTreeSet::new();
    for start in deps.keys() {
        let mut path: Vec<&str> = Vec::new();
        let mut open: BTreeSet<&str> = BTreeSet::new();
        walk(start, deps, &mut done, &mut open, &mut path, &mut found);
    }
    found.into_iter().collect()
}

fn walk<'a>(
    id: &'a str,
    deps: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    done: &mut BTreeSet<&'a str>,
    open: &mut BTreeSet<&'a str>,
    path: &mut Vec<&'a str>,
    found: &mut BTreeSet<Vec<String>>,
) {
    if open.contains(id) {
        let at = path.iter().position(|seen| *seen == id).unwrap_or(0);
        found.insert(from_lowest(&path[at..]));
        return;
    }
    if !done.insert(id) {
        return;
    }
    open.insert(id);
    path.push(id);
    for target in deps.get(id).into_iter().flatten() {
        walk(target, deps, done, open, path, found);
    }
    path.pop();
    open.remove(id);
}

/// One cycle, rotated to start at its lowest id and closed by repeating it.
///
/// Rotated so that the same cycle found from two starts is one line rather than two.
fn from_lowest(cycle: &[&str]) -> Vec<String> {
    if cycle.is_empty() {
        return Vec::new();
    }
    let mut lowest = 0;
    for (index, id) in cycle.iter().enumerate() {
        if *id < cycle[lowest] {
            lowest = index;
        }
    }
    let mut written: Vec<String> = cycle[lowest..]
        .iter()
        .chain(cycle[..lowest].iter())
        .map(|id| (*id).to_owned())
        .collect();
    if let Some(first) = written.first().cloned() {
        written.push(first);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, scope: &[(&str, ScopeConfidence)], depends_on: &[&str]) -> Candidate {
        Candidate {
            id: id.to_owned(),
            scope: scope
                .iter()
                .map(|(path, confidence)| {
                    ScopeEntry::new(*path, *confidence).expect("a path is a surface")
                })
                .collect(),
            depends_on: depends_on.iter().map(|id| (*id).to_owned()).collect(),
        }
    }

    fn placed(derivation: &Derivation) -> Vec<Vec<&str>> {
        derivation
            .waves
            .iter()
            .map(|wave| {
                wave.artifacts
                    .iter()
                    .map(|entry| entry.id.as_str())
                    .collect()
            })
            .collect()
    }

    /// An ordering through a story that is never placed is still an ordering. Dropping it would put
    /// a story in the same wave as something it depends on, two hops away, where nobody would look.
    #[test]
    fn a_dependency_through_an_unassessed_story_still_orders_the_two_that_are_placed() {
        let derivation = derive(&[
            candidate("story:base", &[("crates/base.rs", ScopeConfidence::Cited)], &[]),
            candidate("story:middle", &[], &["story:base"]),
            candidate(
                "story:last",
                &[("crates/last.rs", ScopeConfidence::Cited)],
                &["story:middle"],
            ),
        ]);
        assert_eq!(
            placed(&derivation),
            vec![vec!["story:base"], vec!["story:last"]],
            "the surfaces are disjoint, so only the dependency separates them"
        );
        assert_eq!(derivation.unassessed, vec!["story:middle".to_owned()]);
    }

    /// A cycle has no ordering, so nothing is placed and the ids are the whole answer — written
    /// from the lowest id, so the same cycle reached from two starts is one line.
    #[test]
    fn a_cycle_is_reported_once_from_its_lowest_id_and_nothing_is_placed() {
        let derivation = derive(&[
            candidate(
                "story:two",
                &[("crates/two.rs", ScopeConfidence::Cited)],
                &["story:one"],
            ),
            candidate(
                "story:one",
                &[("crates/one.rs", ScopeConfidence::Cited)],
                &["story:two"],
            ),
        ]);
        assert_eq!(
            derivation.cycles,
            vec![vec![
                "story:one".to_owned(),
                "story:two".to_owned(),
                "story:one".to_owned()
            ]]
        );
        assert!(derivation.waves.is_empty(), "{:?}", derivation.waves);
    }

    /// The greedy packing is the point: the third story goes into the first wave that has room for
    /// it rather than into a wave of its own.
    #[test]
    fn a_collision_moves_a_story_to_the_first_wave_with_room_and_not_to_the_end() {
        let derivation = derive(&[
            candidate("story:a", &[("crates/x.rs", ScopeConfidence::Cited)], &[]),
            candidate("story:b", &[("crates/x.rs", ScopeConfidence::Cited)], &[]),
            candidate("story:c", &[("crates/y.rs", ScopeConfidence::Cited)], &[]),
        ]);
        assert_eq!(
            placed(&derivation),
            vec![vec!["story:a", "story:c"], vec!["story:b"]]
        );
        assert_eq!(derivation.collisions.len(), 1);
        assert_eq!(derivation.collisions[0].path, "crates/x.rs");
        assert_eq!(derivation.collisions[0].confidence, ScopeConfidence::Cited);
    }

    /// One inferred side is enough to mark the pair: the claim rests on a guess either way.
    #[test]
    fn a_collision_with_one_inferred_side_is_marked_inferred() {
        let derivation = derive(&[
            candidate("story:read", &[("crates/x.rs", ScopeConfidence::Cited)], &[]),
            candidate(
                "story:guessed",
                &[("crates/x.rs", ScopeConfidence::Inferred)],
                &[],
            ),
        ]);
        assert_eq!(derivation.collisions.len(), 1);
        assert_eq!(
            derivation.collisions[0].confidence,
            ScopeConfidence::Inferred
        );
        assert!(
            derivation.waves[0].artifacts[0].inferred
                || derivation.waves[1].artifacts[0].inferred,
            "the member resting on a guess says so"
        );
    }

    /// An edge out of the selected set is not an ordering this answer can honour, and pretending
    /// otherwise would make `--status proposed` unable to place anything that ever depended on a
    /// story somebody has already finished.
    #[test]
    fn a_dependency_on_something_outside_the_selection_does_not_hold_a_story_back() {
        let derivation = derive(&[candidate(
            "story:only",
            &[("crates/x.rs", ScopeConfidence::Cited)],
            &["story:elsewhere"],
        )]);
        assert_eq!(placed(&derivation), vec![vec!["story:only"]]);
    }
}
