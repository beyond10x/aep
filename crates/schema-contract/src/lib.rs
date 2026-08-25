//! Offline tooling for adopter-owned JSON Schema contracts.
//!
//! A schema remains the authored source. [`validate`] checks instances against it and
//! [`typescript`] projects the structural types a TypeScript consumer needs. Neither operation
//! reaches a network, reads a clock, or treats generated output as a second contract.

pub mod typescript;
pub mod validate;
