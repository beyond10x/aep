//! This operational control uses Linux process identity and local Unix sockets.

use std::process::ExitCode;

use aep_contract::migration::{
    AuthorityCoordinateV1, AuthoritySnapshotIdV1, DigestV1, MigrationIntentV2,
    SourceSnapshotIdV1,
};
use anyhow::{Result, bail};
use clap::Subcommand;

use super::CommonArgs;

#[derive(Debug, Subcommand)]
pub(crate) enum WriterControlCommand {
    /// Available only on Linux hosts with local writer sessions.
    Hold,
}

pub(super) struct PublicWriterControl;

impl PublicWriterControl {
    pub(super) fn new(_common: Option<CommonArgs>) -> Self { Self }
}

pub(super) fn hold(_command: WriterControlCommand) -> Result<ExitCode> {
    bail!("foreground planning writer control requires Linux process observation")
}

impl aep_planning_migration::WriterControl for PublicWriterControl {
    type Guard = ();

    fn acquire(&self, _intent: &MigrationIntentV2) -> Result<Self::Guard, aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }

    fn recheck(&self, _guard: &mut Self::Guard, _source_snapshot: SourceSnapshotIdV1, _selector_digest: DigestV1) -> Result<(), aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }

    fn retire_source(&self, _guard: &mut Self::Guard) -> Result<(), aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }
}

impl aep_planning_migration::AuthorityWriterControl for PublicWriterControl {
    type Guard = ();

    fn acquire_authority(&self, _authority: &AuthorityCoordinateV1, _requested: AuthoritySnapshotIdV1) -> Result<Self::Guard, aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }

    fn recheck_authority(&self, _guard: &mut Self::Guard, _authority: &AuthorityCoordinateV1, _requested: AuthoritySnapshotIdV1) -> Result<(), aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }
}
