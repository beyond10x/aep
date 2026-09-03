//! The status ladder, decided by `entity-core` — now the adapter's, re-exported here.
//!
//! Wave H, story 2 moved the bridge into `aep-backend-entity`, where `describe_type` renders the
//! same `EntityDefinition` the kernel decides with, so a harness reading the descriptor and a move
//! getting the verdict cannot drift apart. Every path that named `crate::kernel::…` still does;
//! `tests/kernel_equivalence.rs` holds the verdicts equal over this repository's own store, as it
//! always has.

pub use aep_backend_entity::kernel::*;
