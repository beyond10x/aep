//! Acquisition edge for AEP document trees and adopting projects.
//!
//! Filesystem discovery, environment selection, schema parsing and Git process execution live
//! here. The semantic engine receives validated domain documents through [`aep_engine::Registry`]
//! and never discovers or acquires them itself.

pub mod load;
pub mod project;

pub use load::{load_tree, load_tree_report, LoadErrors, LoadFailure, LoadOutcome};
pub use project::{discover, Project};
