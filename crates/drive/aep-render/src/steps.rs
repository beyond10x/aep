//! What a driver does in each state, as this crate needs it.
//!
//! # Why this is a plain struct and not the driver's `StepMap`
//!
//! The same decision [`crate::run`] records, for the same reason. A `StepMap` lives in
//! `aep-driver-spec`, and a renderer that named that type would drag the driver — and everything
//! behind it — into a tree whose whole job is to write text. [`StepsView`] is the seam instead: the
//! caller owns the conversion, and this crate keeps `aep-domain` as its only dependency.
//!
//! # Why the overlay exists at all
//!
//! A workflow says what may happen and never what runs. Read on its own it is honest and not
//! actionable: a reader of `acquire` learns that the world is read before anything is normalised,
//! and not that a program called `brain ledger acquire` is what reads it. The step map is the
//! missing half, and it is a separate document because one workflow governs instances whose
//! programs differ.
//!
//! So a document rendered with a map answers *what happens here*, and a document rendered without
//! one still answers *what may happen* — which is why the map is optional rather than required.

use std::collections::BTreeMap;

use aep_domain::ids::StateId;

/// One step of one state.
///
/// Carries what a reader of the prose needs and nothing a runner would need: the kind, because
/// `command`, `llm` and `operator` are three different things to be told; and one line saying what
/// it does. Retry budgets, evidence mappings and circuit breakers are the driver's business and are
/// not in a document a person reads to understand the round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepView {
    /// `command`, `llm` or `operator`, in the driver's own words.
    ///
    /// A string rather than an enum copied from the driver, so a kind added there renders here
    /// without this crate being edited. The three that exist today are the three the notation has.
    pub kind: String,
    /// One line naming what the step does.
    pub label: String,
}

/// What a driver runs, per state.
///
/// A state the map is silent about is absent from [`Self::states`] rather than present and empty,
/// because those are different facts: *nothing runs here* is a claim the map makes, and *this map
/// does not cover this state* is a gap in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepsView {
    /// What the map is called, for the line that says where the steps came from.
    pub reference: String,
    /// The steps of each state the map covers, in the order the author wrote them.
    pub states: BTreeMap<StateId, Vec<StepView>>,
}

impl StepsView {
    /// The steps of one state, or nothing where the map is silent about it.
    #[must_use]
    pub fn of(&self, state: &StateId) -> Option<&[StepView]> {
        self.states.get(state).map(Vec::as_slice)
    }
}
