//! Foreground operator custody for the machine's manually started planning writers.
//!
//! The socket proves a live holder answered a fresh challenge. The operator's continuing
//! no-restart commitment is the operational authority; neither a socket nor a PID scan can
//! enforce it against an operator who starts another unfenced writer.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write as _};
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use aep_contract::migration::{
    AuthorityCoordinateV1, AuthoritySnapshotIdV1, DigestV1, MigrationIdV1, MigrationIntentV2,
    MigrationPhaseObservationV2, PhaseRecordV2, PresenceV1, ProjectVersionV1,
    SourceCoordinateV1, SourceSnapshotIdV1,
};
use anyhow::{Context as _, Result};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::{CommonArgs, Resolved, resolve};

#[derive(Debug, Subcommand)]
pub(crate) enum WriterControlCommand {
    /// Observe the listed live writer processes stop, then keep custody in this terminal.
    Hold(HoldArgs),
}

#[derive(Debug, Args)]
pub(crate) struct HoldArgs {
    #[command(flatten)]
    common: CommonArgs,
    /// Apply identity. Requires --snapshot.
    #[arg(long, requires = "snapshot", conflicts_with = "authority_snapshot")]
    migration: Option<String>,
    /// Inspected raw source snapshot to bind to the stopped writers.
    #[arg(long, requires = "migration")]
    snapshot: Option<String>,
    /// Selected Eventlog authority snapshot for projection rebuild custody.
    #[arg(long, conflicts_with = "migration", required_unless_present = "migration")]
    authority_snapshot: Option<String>,
    /// Every applicable live writer process; descendants observed while they run are included.
    #[arg(long = "writer-pid", conflicts_with = "resume_stop", required_unless_present = "resume_stop")]
    writer_pids: Vec<u32>,
    /// Reuse only prior observed stop facts after a holder crash; requires fresh custody.
    #[arg(long, conflicts_with = "writer_pids")]
    resume_stop: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    project: PathBuf,
    mode: Mode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Mode {
    Apply {
        migration: MigrationIdV1,
        snapshot: SourceSnapshotIdV1,
        selector_digest: DigestV1,
        config_digest: DigestV1,
        source: Box<SourceCoordinateV1>,
    },
    Rebuild {
        authority: AuthorityCoordinateV1,
        snapshot: AuthoritySnapshotIdV1,
        selector_digest: DigestV1,
        config_digest: DigestV1,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessIdentity {
    pid: u32,
    start_time: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StopWitness {
    format: StopWitnessFormat,
    binding: Binding,
    boot_id: String,
    stopped: Vec<ProcessIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum StopWitnessFormat {
    #[serde(rename = "aep.writer-stop/1")]
    V1,
}

#[derive(Clone)]
pub(super) struct PublicWriterControl {
    common: Option<CommonArgs>,
}

pub(super) struct HeldGuard {
    binding: Binding,
    common: CommonArgs,
    source_snapshot: Option<SourceSnapshotIdV1>,
    generation: String,
}

impl PublicWriterControl {
    pub(super) fn new(common: Option<CommonArgs>) -> Self {
        Self { common }
    }

    fn common(&self) -> Result<&CommonArgs, aep_planning_migration::WriterControlError> {
        self.common
            .as_ref()
            .ok_or(aep_planning_migration::WriterControlError::Unavailable)
    }
}

impl aep_planning_migration::WriterControl for PublicWriterControl {
    type Guard = HeldGuard;

    fn acquire(
        &self,
        intent: &MigrationIntentV2,
    ) -> Result<Self::Guard, aep_planning_migration::WriterControlError> {
        let common = self.common()?;
        let resolved = resolve(common).map_err(|_| aep_planning_migration::WriterControlError::Unavailable)?;
        let binding = Binding {
            project: canonical_project(&resolved).map_err(|_| aep_planning_migration::WriterControlError::Unavailable)?,
            mode: Mode::Apply {
                migration: intent.migration_id.clone(),
                snapshot: intent.source_snapshot,
                selector_digest: intent.selector_digest,
                config_digest: intent.config_digest,
                source: Box::new(intent.source_coordinate.clone()),
            },
        };
        check_selection(&resolved, &binding).map_err(|_| aep_planning_migration::WriterControlError::Unavailable)?;
        let generation = challenge(&binding).map_err(|_| aep_planning_migration::WriterControlError::Unavailable)?;
        Ok(HeldGuard { binding, common: common.clone(), source_snapshot: Some(intent.source_snapshot), generation })
    }

    fn recheck(
        &self,
        guard: &mut Self::Guard,
        snapshot: SourceSnapshotIdV1,
        selector_digest: DigestV1,
    ) -> Result<(), aep_planning_migration::WriterControlError> {
        let Mode::Apply { selector_digest: bound_selector, .. } = &guard.binding.mode else {
            return Err(aep_planning_migration::WriterControlError::Lost);
        };
        if guard.source_snapshot != Some(snapshot) || *bound_selector != selector_digest {
            return Err(aep_planning_migration::WriterControlError::Lost);
        }
        let resolved = resolve(&guard.common).map_err(|_| aep_planning_migration::WriterControlError::Lost)?;
        check_selection(&resolved, &guard.binding).map_err(|_| aep_planning_migration::WriterControlError::Lost)?;
        recheck_generation(guard).map_err(|_| aep_planning_migration::WriterControlError::Lost)
    }

    fn retire_source(
        &self,
        guard: &mut Self::Guard,
    ) -> Result<(), aep_planning_migration::WriterControlError> {
        let resolved = resolve(&guard.common).map_err(|_| aep_planning_migration::WriterControlError::RetirementUnproved)?;
        check_selection(&resolved, &guard.binding).map_err(|_| aep_planning_migration::WriterControlError::RetirementUnproved)?;
        recheck_generation(guard).map_err(|_| aep_planning_migration::WriterControlError::RetirementUnproved)
    }
}

impl aep_planning_migration::AuthorityWriterControl for PublicWriterControl {
    type Guard = HeldGuard;

    fn acquire_authority(
        &self,
        authority: &AuthorityCoordinateV1,
        requested: AuthoritySnapshotIdV1,
    ) -> Result<Self::Guard, aep_planning_migration::WriterControlError> {
        let common = self.common()?;
        let resolved = resolve(common).map_err(|_| aep_planning_migration::WriterControlError::Unavailable)?;
        let binding = Binding {
            project: canonical_project(&resolved).map_err(|_| aep_planning_migration::WriterControlError::Unavailable)?,
            mode: Mode::Rebuild {
                authority: authority.clone(), snapshot: requested,
                selector_digest: resolved.selection.selector_digest,
                config_digest: resolved.selection.config_digest,
            },
        };
        check_selection(&resolved, &binding).map_err(|_| aep_planning_migration::WriterControlError::Unavailable)?;
        let generation = challenge(&binding).map_err(|_| aep_planning_migration::WriterControlError::Unavailable)?;
        Ok(HeldGuard { binding, common: common.clone(), source_snapshot: None, generation })
    }

    fn recheck_authority(
        &self,
        guard: &mut Self::Guard,
        authority: &AuthorityCoordinateV1,
        requested: AuthoritySnapshotIdV1,
    ) -> Result<(), aep_planning_migration::WriterControlError> {
        if !matches!(&guard.binding.mode, Mode::Rebuild { authority: held, snapshot, .. } if held == authority && *snapshot == requested) {
            return Err(aep_planning_migration::WriterControlError::Lost);
        }
        let resolved = resolve(&guard.common).map_err(|_| aep_planning_migration::WriterControlError::Lost)?;
        check_selection(&resolved, &guard.binding).map_err(|_| aep_planning_migration::WriterControlError::Lost)?;
        recheck_generation(guard).map_err(|_| aep_planning_migration::WriterControlError::Lost)
    }
}

fn canonical_project(resolved: &Resolved) -> Result<PathBuf> {
    fs::canonicalize(&resolved.selector_path).context("canonicalizing selected project")
}

fn check_selection(resolved: &Resolved, binding: &Binding) -> Result<()> {
    if canonical_project(resolved)? != binding.project {
        anyhow::bail!("project selector changed");
    }
    match &binding.mode {
        Mode::Apply { migration, snapshot, selector_digest, config_digest, source } => {
            if resolved.selection.project_version == ProjectVersionV1::V1 {
                if resolved.selection.selector_digest != *selector_digest
                    || resolved.selection.config_digest != *config_digest
                    || resolved.selection.source != **source {
                    anyhow::bail!("legacy source or selector changed");
                }
            } else {
                let root = super::migration_root(&resolved.engineering, migration)?;
                let intent: MigrationIntentV2 = serde_json::from_slice(&fs::read(root.join("intent.json"))?)?;
                if intent.migration_id != *migration || intent.source_snapshot != *snapshot
                    || intent.source_coordinate != **source || intent.selector_digest != *selector_digest
                    || intent.config_digest != *config_digest
                    || aep_planning_migration::intent_digest_v2(&intent)? != intent.intent_digest {
                    anyhow::bail!("migration intent changed");
                }
                let record: PhaseRecordV2 = serde_json::from_slice(&fs::read(root.join("phases/02-destination-provisioned.json"))?)?;
                if record.intent_digest != intent.intent_digest {
                    anyhow::bail!("migration binding changed");
                }
                let bound = record.observations.iter().find_map(|observation| match observation {
                    MigrationPhaseObservationV2::Binding(value) => Some(value), _ => None,
                }).context("migration has no provider binding")?;
                if resolved.selection.authority != PresenceV1::Present(bound.authority.clone())
                    || resolved.selection.selector_digest != bound.intended_selector_digest
                    || resolved.selection.config_digest != aep_contract::migration::config_digest_v1(bound.intended_selector_bytes.as_bytes()) {
                    anyhow::bail!("selected authority differs from migration binding");
                }
            }
        }
        Mode::Rebuild { authority, selector_digest, config_digest, .. } => {
            if resolved.selection.project_version != ProjectVersionV1::V2
                || resolved.selection.authority != PresenceV1::Present(authority.clone())
                || resolved.selection.selector_digest != *selector_digest
                || resolved.selection.config_digest != *config_digest {
                anyhow::bail!("selected authority changed");
            }
        }
    }
    Ok(())
}

fn control_directory(project: &Path) -> Result<PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(project.as_os_str().as_encoded_bytes());
    let name = format!("aep-writer-control-{}", hex_bytes(&hasher.finalize()));
    let directory = std::env::temp_dir().join(&name[..name.len().min(51)]);
    fs::create_dir_all(&directory)?;
    let metadata = fs::symlink_metadata(&directory)?;
    if !metadata.file_type().is_dir() || metadata.uid() != fs::metadata("/proc/self")?.uid() {
        anyhow::bail!("writer-control directory is not owned by this operator");
    }
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    Ok(directory)
}

fn socket_path(binding: &Binding) -> Result<PathBuf> {
    Ok(control_directory(&binding.project)?.join("hold.sock"))
}

fn witness_path(binding: &Binding) -> Result<PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(binding)?);
    Ok(control_directory(&binding.project)?.join(format!("{}.stop.json", hex_bytes(&hasher.finalize()))))
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes { write!(&mut output, "{byte:02x}").expect("writing to String cannot fail"); }
    output
}

fn challenge(binding: &Binding) -> Result<String> {
    let mut stream = UnixStream::connect(socket_path(binding)?)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(&serde_json::to_vec(binding)?)?;
    stream.write_all(b"\n")?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer)?;
    answer.strip_prefix("OK ").and_then(|value| value.strip_suffix('\n'))
        .map(ToOwned::to_owned).context("no matching live writer-control custody")
}

fn recheck_generation(guard: &HeldGuard) -> Result<()> {
    if challenge(&guard.binding)? != guard.generation {
        anyhow::bail!("the original holder ended; reacquire under current custody");
    }
    Ok(())
}

pub(super) fn hold(command: WriterControlCommand) -> Result<ExitCode> {
    match command { WriterControlCommand::Hold(args) => run_holder(&args) }
}

#[allow(clippy::too_many_lines)] // Stop observation, fresh source check and custody stay ordered.
fn run_holder(args: &HoldArgs) -> Result<ExitCode> {
    if matches!(args.common.format, super::StoreOutputFormat::Json) {
        anyhow::bail!("interactive writer control supports only --format text");
    }
    let resolved = resolve(&args.common)?;
    let project = canonical_project(&resolved)?;
    let mode = match (&args.migration, &args.snapshot, &args.authority_snapshot) {
        (Some(migration), Some(snapshot), None) => {
            let migration = MigrationIdV1::new(migration.clone())?;
            let snapshot = SourceSnapshotIdV1(DigestV1::parse(snapshot)?);
            if resolved.selection.project_version == ProjectVersionV1::V2 {
                if !args.resume_stop { anyhow::bail!("selected migration requires a prior observed stop witness"); }
                let root = super::migration_root(&resolved.engineering, &migration)?;
                let intent: MigrationIntentV2 = serde_json::from_slice(&fs::read(root.join("intent.json"))?)?;
                if intent.migration_id != migration || intent.source_snapshot != snapshot {
                    anyhow::bail!("selected migration intent does not match this request");
                }
                Mode::Apply {
                    migration, snapshot,
                    selector_digest: intent.selector_digest,
                    config_digest: intent.config_digest,
                    source: Box::new(intent.source_coordinate),
                }
            } else {
                Mode::Apply {
                    migration, snapshot,
                    selector_digest: resolved.selection.selector_digest,
                    config_digest: resolved.selection.config_digest,
                    source: Box::new(resolved.selection.source.clone()),
                }
            }
        },
        (None, None, Some(snapshot)) => {
            let PresenceV1::Present(authority) = &resolved.selection.authority else {
                anyhow::bail!("rebuild needs selected Eventlog authority");
            };
            Mode::Rebuild {
                authority: authority.clone(),
                snapshot: AuthoritySnapshotIdV1(DigestV1::parse(snapshot)?),
                selector_digest: resolved.selection.selector_digest,
                config_digest: resolved.selection.config_digest,
            }
        }
        _ => anyhow::bail!("choose migration plus snapshot or authority snapshot"),
    };
    let binding = Binding { project, mode };
    let stop_path = witness_path(&binding)?;
    let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id")?.trim().to_owned();
    let stopped = if args.resume_stop {
        let witness: StopWitness = serde_json::from_slice(&fs::read(&stop_path)
            .context("no observed stop witness for this binding")?)?;
        if witness.format != StopWitnessFormat::V1 || witness.binding != binding
            || witness.stopped.is_empty() || witness.boot_id != boot_id {
            anyhow::bail!("stale or empty stop witness");
        }
        for stopped in &witness.stopped {
            if process_state(stopped.pid)?.is_some_and(|state| state.live) {
                anyhow::bail!("a witnessed writer PID is live again");
            }
        }
        witness.stopped
    } else {
        if args.writer_pids.is_empty() { anyhow::bail!("name at least one live writer process"); }
        let stopped = observe_stops(&args.writer_pids)?;
        let witness = StopWitness {
            format: StopWitnessFormat::V1,
            binding: binding.clone(), boot_id, stopped: stopped.clone(),
        };
        let path = stop_path;
        let bytes = serde_json::to_vec(&witness)?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(&path)
            .context("a stop witness already exists; resume its custody instead")?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        stopped
    };
    // Re-resolve and re-read after the observed drain. A stopped PID is not a snapshot.
    let current = resolve(&args.common)?;
    check_selection(&current, &binding)?;
    if let Mode::Apply { snapshot, .. } = &binding.mode {
        if current.selection.project_version == ProjectVersionV1::V1 {
            let captured = current.capture().context("capturing stopped legacy source")?;
            let aep_contract::migration::ObservationOutcomeV1::Complete(complete) = captured.observation else {
                anyhow::bail!("stopped source is not complete");
            };
            if complete.raw_snapshot_id != snapshot.0 { anyhow::bail!("stopped source snapshot changed"); }
        }
    } else if let Mode::Rebuild { authority, snapshot, .. } = &binding.mode {
        let crate::planning::Plan::Eventlog { authority_root, .. } = &current.plan else {
            anyhow::bail!("rebuild no longer selects Eventlog authority");
        };
        let captured = aep_backend_eventlog::complete_file_snapshot(authority_root, entity_eventlog::Authority {
            logical_scope: authority.logical_scope.as_str().to_owned(),
            tenant: authority.tenant.as_str().to_owned(),
            stream_identity: authority.stream_identity.as_str().to_owned(),
        }).map_err(anyhow::Error::msg)?;
        let (observed, _) = aep_planning_migration::authority_snapshot_identity(authority, &captured)?;
        if observed != *snapshot { anyhow::bail!("selected authority snapshot changed"); }
    }
    println!("Retained stop evidence for {} writer processes. Keep this foreground command running, and do not restart any applicable writer until the selected authority is verified.", stopped.len());
    println!("Type HOLD to take continuing no-restart custody; an empty line later releases it.");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    if line.trim_end() != "HOLD" { anyhow::bail!("operator custody was not taken"); }
    serve(&binding, &stopped)
}

#[derive(Clone, Copy)]
struct ProcessState { parent: u32, start_time: u64, live: bool }

fn process_state(pid: u32) -> Result<Option<ProcessState>> {
    let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let after = stat.rsplit_once(") ").context("invalid process stat")?.1;
    let fields = after.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 20 { anyhow::bail!("truncated process stat"); }
    Ok(Some(ProcessState {
        parent: fields[1].parse()?, start_time: fields[19].parse()?,
        live: fields[0] != "Z" && fields[0] != "X",
    }))
}

fn observe_stops(pids: &[u32]) -> Result<Vec<ProcessIdentity>> {
    let mut observed = BTreeMap::new();
    for pid in pids {
        let state = process_state(*pid)?.context("a named writer PID was already absent")?;
        if !state.live { anyhow::bail!("a named writer PID was already stopped"); }
        if *pid == std::process::id() { anyhow::bail!("holder cannot name itself as a writer"); }
        observed.insert(*pid, state.start_time);
    }
    loop {
        let mut living = BTreeSet::new();
        let mut states = BTreeMap::new();
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else { continue };
            if let Ok(Some(state)) = process_state(pid) { states.insert(pid, state); }
        }
        // Capture descendants while their recorded parent is still live. Operators must name
        // independently daemonized writers as roots; an unknown process cannot be guessed.
        loop {
            let mut added = false;
            for (pid, state) in &states {
                if !observed.contains_key(pid) && observed.get(&state.parent).is_some_and(|start| states.get(&state.parent).is_some_and(|parent| parent.start_time == *start && parent.live)) {
                    observed.insert(*pid, state.start_time);
                    added = true;
                }
            }
            if !added { break; }
        }
        for (pid, start_time) in &observed {
            // A failed read is not an observed exit. Re-read each tracked identity directly;
            // the broad /proc census only discovers descendants.
            if let Some(state) = process_state(*pid)? {
                if state.start_time != *start_time { anyhow::bail!("writer PID was reused during stop observation"); }
                if state.live { living.insert(*pid); }
            }
        }
        if living.is_empty() { break; }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(observed.into_iter().map(|(pid, start_time)| ProcessIdentity { pid, start_time }).collect())
}

fn serve(binding: &Binding, stopped: &[ProcessIdentity]) -> Result<ExitCode> {
    let generation = fs::read_to_string("/proc/sys/kernel/random/uuid")?.trim().to_owned();
    let path = socket_path(binding)?;
    if let Ok(mut old) = UnixStream::connect(&path) {
        old.write_all(b"\n")?;
        anyhow::bail!("another holder is already active for this project");
    }
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if !metadata.file_type().is_socket() { anyhow::bail!("control path is not a socket"); }
        fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    let active = Arc::new(AtomicBool::new(true));
    let input_active = Arc::clone(&active);
    thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        input_active.store(false, Ordering::SeqCst);
    });
    println!("Writer control held. Run the matching apply or rebuild in another terminal; press Enter here only after verification.");
    std::io::stdout().flush()?;
    while active.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                let mut request = String::new();
                let stopped_remain_stopped = stopped.iter().all(|identity| {
                    process_state(identity.pid).is_ok_and(|state| !state.is_some_and(|state| state.live))
                });
                if stopped_remain_stopped
                    && BufReader::new(&mut stream).read_line(&mut request).is_ok()
                    && serde_json::from_str::<Binding>(&request).ok() == Some(binding.clone())
                    && active.load(Ordering::SeqCst) {
                    stream.write_all(format!("OK {generation}\n").as_bytes())?;
                } else { let _ = stream.write_all(b"REFUSED\n"); }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(error.into()),
        }
    }
    fs::remove_file(path)?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_contract::migration::AuthorityValueV1;

    #[test]
    fn stop_witness_has_a_closed_version_and_no_live_custody_claim() {
        let witness = StopWitness {
            format: StopWitnessFormat::V1,
            binding: Binding {
                project: PathBuf::from("/tmp/stop-witness-test/project.yaml"),
                mode: Mode::Rebuild {
                    authority: AuthorityCoordinateV1 {
                        logical_scope: AuthorityValueV1::new("scope").expect("scope"),
                        tenant: AuthorityValueV1::new("tenant").expect("tenant"),
                        stream_identity: AuthorityValueV1::new("stream").expect("stream"),
                    },
                    snapshot: AuthoritySnapshotIdV1(DigestV1::from_bytes([1; 32])),
                    selector_digest: DigestV1::from_bytes([2; 32]),
                    config_digest: DigestV1::from_bytes([3; 32]),
                },
            },
            boot_id: "boot".to_owned(),
            stopped: vec![ProcessIdentity { pid: 42, start_time: 7 }],
        };
        let mut value = serde_json::to_value(&witness).expect("witness serializes");
        assert_eq!(value["format"], "aep.writer-stop/1");
        assert!(value.get("held_guard").is_none());
        value["format"] = serde_json::json!("aep.writer-stop/2");
        assert!(serde_json::from_value::<StopWitness>(value.clone()).is_err());
        value["format"] = serde_json::json!("aep.writer-stop/1");
        value["held_guard"] = serde_json::json!(true);
        assert!(serde_json::from_value::<StopWitness>(value).is_err());
    }
}
