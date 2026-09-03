//! The official runtime-neutral client and versioned network projection of AEP.
//!
//! [`AepClient`] implements the same semantic command and query traits as an in-process backend.
//! The HTTP exchange and credential acquisition are injected traits: this crate chooses no async
//! runtime, TLS stack or token source. [`wire`] owns the strict version-1 documents shared with the
//! service implementation.

mod client;
pub mod conformance;
pub mod wire;

pub use client::{
    AepClient, BearerToken, ClientConfigurationError, CredentialError, CredentialProvider,
    Transport, TransportError,
};
