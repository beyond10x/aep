//! Conformance checks of the protocol source cache's integrity and process-containment claims
//! (`story:a-group-writable-snapshot-is-admitted-explicitly`,
//! `story:an-unreachable-protocol-source-fails-within-a-deadline`), as `aep` runs them: each
//! resolution happens in a child process of its own, with exactly the variables a case sets.
//!
//! - The cross-process stamp never admits a snapshot its process's write check would refuse.
//! - A fetch does not outlive the process that started it when that process's group is stopped.
//! - Every positive whole number `AEP_FETCH_TIMEOUT_SECS` accepts is a deadline, not a crash.

mod support;

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::{Outcome, PinnedRemote};

/// The child entry [`run_child`] runs; a no-op when no parent named a source.
const ENTRY: &str = "child_entry_resolves_only_when_a_parent_names_a_source";

/// The variable `support::child_entry` reads the locator from.
const SOURCE_VARIABLE: &str = "AEP_TEST_PROTOCOL_SOURCE";

/// Marks the one line of a child's standard output that carries its outcome.
const OUTCOME_PREFIX: &str = "aep-test-outcome: ";

/// Every variable the source resolution reads, removed from a child unless a case sets it.
const RESOLUTION_VARIABLES: [&str; 3] = [
    "AEP_CACHE_DIR",
    "AEP_CACHE_GROUP_WRITABLE",
    "AEP_FETCH_TIMEOUT_SECS",
];

#[test]
fn child_entry_resolves_only_when_a_parent_names_a_source() {
    if std::env::var_os(SOURCE_VARIABLE).is_some() {
        support::child_entry();
    }
}

/// A child of this test binary that resolves `locator` with exactly `environment` set.
fn child_command(locator: &str, environment: &[(&str, &str)]) -> Command {
    let mut command =
        Command::new(std::env::current_exe().expect("the test binary can name itself"));
    command.args([ENTRY, "--exact", "--nocapture", "--test-threads", "1"]);
    for variable in RESOLUTION_VARIABLES {
        command.env_remove(variable);
    }
    command
        .env(SOURCE_VARIABLE, locator)
        .envs(environment.iter().copied());
    command
}

/// Runs a child to its end: its outcome, or a description of how it ended without one.
fn run_child(locator: &str, environment: &[(&str, &str)]) -> Result<Outcome, String> {
    let output = child_command(locator, environment)
        .output()
        .expect("the child runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(line) = stdout
        .lines()
        .find_map(|line| line.split_once(OUTCOME_PREFIX).map(|(_, outcome)| outcome))
    else {
        return Err(format!(
            "the child ended ({}) without an outcome:\n{stdout}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    };
    let outcome: serde_json::Value = serde_json::from_str(line).expect("one JSON outcome");
    Ok(match outcome["resolved"].as_str() {
        Some(path) => Outcome::Resolved(PathBuf::from(path)),
        None => Outcome::Refused(
            outcome["refused"]
                .as_str()
                .expect("an outcome is resolved or refused")
                .to_owned(),
        ),
    })
}

/// [`run_child`] for a child that must report an outcome.
fn outcome_of(locator: &str, environment: &[(&str, &str)]) -> Outcome {
    run_child(locator, environment).unwrap_or_else(|ended| panic!("{ended}"))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .expect("the test owns the entry");
}

/// The metadata a snapshot stamp holds one entry to, as `aep.snapshot-verified/2` spells it.
#[cfg(unix)]
fn entry_stamp(path: &Path) -> serde_json::Value {
    use std::os::unix::fs::MetadataExt as _;
    let metadata = std::fs::symlink_metadata(path).expect("the entry exists");
    serde_json::json!({
        "len": metadata.len(),
        "mtime_ns": metadata.mtime() * 1_000_000_000 + metadata.mtime_nsec(),
        "ctime_ns": metadata.ctime() * 1_000_000_000 + metadata.ctime_nsec(),
        "ino": metadata.ino(),
        "dev": metadata.dev(),
        "mode": metadata.mode(),
    })
}

/// A published snapshot of a fresh pin, with the stamp a full verification left beside it.
///
/// Returns the scratch root, the locator, the cache variable's value, the snapshot and its stamp.
#[cfg(unix)]
fn stamped_snapshot(name: &str) -> (PathBuf, String, String, PathBuf, PathBuf) {
    let root = support::scratch(name);
    let remote = PinnedRemote::create(&root);
    let locator = remote.locator();
    let cache = root
        .join("cache")
        .to_str()
        .expect("a UTF-8 scratch path")
        .to_owned();
    let snapshot = outcome_of(&locator, &[("AEP_CACHE_DIR", &cache)])
        .resolved("an empty cache fetches the pin");
    // Past the two-second racy window, so the next full verification leaves a stamp.
    std::thread::sleep(Duration::from_millis(2_100));
    outcome_of(&locator, &[("AEP_CACHE_DIR", &cache)])
        .resolved("the sealed snapshot verifies in full");
    let stamp = snapshot.with_file_name(format!("{}.verified.json", remote.revision));
    assert!(
        stamp.exists(),
        "precondition: a full verification stamped {}",
        snapshot.display()
    );
    (root, locator, cache, snapshot, stamp)
}

/// Builds a state the full check refuses and a stamp the write check never sees, then resolves.
///
/// `profiles/` gets `directory_mode`, so a principal the process may not trust can write it; that
/// principal replaces `profiles/standard.yaml` with bytes the sealed manifest does not hold, and
/// rewrites the stamp, given `stamp_mode`, to describe the snapshot as it now is with the writable
/// directory left out. The manifest, the snapshot root and every other entry are untouched.
#[cfg(unix)]
fn resolve_through_a_stamp_written_by_an_untrusted_principal(
    name: &str,
    directory_mode: u32,
    stamp_mode: u32,
    declaration: Option<&str>,
) -> (Outcome, Outcome) {
    let (root, locator, cache, snapshot, stamp) = stamped_snapshot(name);
    let mut environment = vec![("AEP_CACHE_DIR", cache.as_str())];
    environment.extend(declaration.map(|value| ("AEP_CACHE_GROUP_WRITABLE", value)));

    let profiles = snapshot.join("profiles");
    set_mode(&profiles, directory_mode);
    let replacement = profiles.join(".standard.yaml.new");
    std::fs::write(&replacement, "not what the pin holds\n").expect("the directory is writable");
    // Sealed like the file it replaces, so no write bit of its own is what a check could catch.
    set_mode(&replacement, 0o444);
    std::fs::rename(&replacement, profiles.join("standard.yaml")).expect("the entry is replaced");

    // Without a matching stamp, the full check is what answers.
    let full = outcome_of(&locator, &environment);

    let mut forged: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&stamp).expect("stamped")).expect("a JSON stamp");
    forged["directories"]
        .as_object_mut()
        .expect("a directory map")
        .remove("profiles");
    forged["files"]["profiles/standard.yaml"] = entry_stamp(&profiles.join("standard.yaml"));
    std::fs::write(
        &stamp,
        serde_json::to_vec_pretty(&forged).expect("serializable"),
    )
    .expect("the stamp is writable");
    set_mode(&stamp, stamp_mode);

    let stamped = outcome_of(&locator, &environment);
    support::remove(&root);
    (full, stamped)
}

/// Acceptance 6 of `story:a-group-writable-snapshot-is-admitted-explicitly`, and the K2 rule that
/// the stamp never lets a group-writable snapshot through without the declaration in this process.
#[cfg(unix)]
#[test]
fn a_stamp_the_group_can_write_never_admits_a_group_writable_snapshot_without_the_declaration() {
    let (full, stamped) = resolve_through_a_stamp_written_by_an_untrusted_principal(
        "sec-stamp-group",
        0o775,
        0o664,
        None,
    );
    let refusal = full.refused("precondition: the full check refuses a group-writable directory");
    assert!(
        refusal.contains("AEP_CACHE_GROUP_WRITABLE"),
        "precondition: {refusal}"
    );
    let refusal = stamped.refused(
        "a stamp the untrusted group could write admitted a group-writable snapshot whose \
         profiles/standard.yaml is not the pinned byte",
    );
    assert!(refusal.contains("AEP_CACHE_GROUP_WRITABLE"), "{refusal}");
}

/// Acceptance 3: a snapshot others can write is refused with or without the variable, the stamp
/// included.
#[cfg(unix)]
#[test]
fn a_stamp_others_can_write_never_admits_a_snapshot_others_can_write() {
    let (full, stamped) = resolve_through_a_stamp_written_by_an_untrusted_principal(
        "sec-stamp-others",
        0o757,
        0o666,
        Some("1"),
    );
    let refusal = full.refused("precondition: the full check refuses a directory others can write");
    assert!(
        refusal.contains("writable by others"),
        "precondition: {refusal}"
    );
    let refusal = stamped.refused(
        "a stamp others could write admitted a snapshot others can write, whose \
         profiles/standard.yaml is not the pinned byte",
    );
    assert!(refusal.contains("writable by others"), "{refusal}");
}

/// Waits up to `watch` for a connection on `listener`.
fn accept_within(listener: &std::net::TcpListener, watch: Duration) -> std::net::TcpStream {
    let until = Instant::now() + watch;
    listener.set_nonblocking(true).expect("non-blocking accept");
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).expect("blocking reads");
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    Instant::now() < until,
                    "precondition: Git's transport connected within {watch:?}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accepting: {error}"),
        }
    }
}

/// Whether the peer of `stream` closed it within `watch`; it never receives an answer.
fn closed_within(stream: &mut std::net::TcpStream, watch: Duration) -> bool {
    let until = Instant::now() + watch;
    let mut buffer = [0_u8; 4096];
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        stream.set_read_timeout(Some(left)).expect("a read timeout");
        match stream.read(&mut buffer) {
            Ok(0) => return true,
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return false
            }
            Err(_) => return true,
        }
    }
}

/// Signals process group `group` with the system's `kill`.
#[cfg(unix)]
fn signal_group(group: u32, signal: &str) {
    let status = Command::new("kill")
        .args(["-s", signal, "--", &format!("-{group}")])
        .status()
        .expect("kill runs");
    assert!(status.success(), "signalling group {group}: {status}");
}

/// The outcome of `story:an-unreachable-protocol-source-fails-within-a-deadline` is a bounded
/// fetch. Ctrl-C at a terminal, coreutils `timeout` and a caller's `killpg` stop a command by
/// signalling its process group; the Git fetch and its transport must stop with it, as they did
/// before the fetch had a deadline, rather than hold the connection and the cache's lock files for
/// as long as the source never answers.
#[cfg(unix)]
#[test]
fn a_fetch_stops_with_the_command_when_the_commands_process_group_is_interrupted() {
    use std::os::unix::process::CommandExt as _;

    let root = support::scratch("sec-fetch-group-interrupt");
    let cache = root.join("cache");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("bound").port();
    let locator = format!(
        "git+https://127.0.0.1:{port}/protocols.git#{}",
        "d".repeat(40)
    );
    let mut child = child_command(
        &locator,
        &[
            ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path")),
            ("AEP_FETCH_TIMEOUT_SECS", "60"),
        ],
    )
    .process_group(0)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .expect("the child runs");

    let mut connection = accept_within(&listener, Duration::from_secs(15));
    // What Ctrl-C does: SIGINT to every process in the foreground group.
    signal_group(child.id(), "INT");
    let until = Instant::now() + Duration::from_secs(5);
    while child
        .try_wait()
        .expect("the child can be waited for")
        .is_none()
    {
        if Instant::now() >= until {
            child.kill().ok();
            child.wait().ok();
            panic!("precondition: SIGINT stops the command");
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let closed = closed_within(&mut connection, Duration::from_secs(3));
    // Closing the never-answering side ends whatever is still reading it.
    drop(connection);
    drop(listener);
    std::thread::sleep(Duration::from_millis(500));
    support::remove(&root);
    assert!(
        closed,
        "the command was interrupted and exited, and Git's transport still held its connection \
         3 s later"
    );
}

/// Acceptance 1: `AEP_FETCH_TIMEOUT_SECS` is a positive integer or a refusal naming it. The largest
/// value it accepts must bound the fetch or be refused by name, not end the command in a panic.
#[test]
fn the_largest_fetch_deadline_the_variable_accepts_ends_in_a_refusal_not_a_crash() {
    let root = support::scratch("sec-fetch-deadline-max");
    let cache = root.join("cache");
    let missing = root.join("no-such-repository");
    let locator = format!("git+file://{}#{}", missing.display(), "e".repeat(40));
    let largest = u64::MAX.to_string();
    let ended = run_child(
        &locator,
        &[
            ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path")),
            ("AEP_FETCH_TIMEOUT_SECS", &largest),
        ],
    );
    support::remove(&root);
    ended
        .unwrap_or_else(|ended| panic!("AEP_FETCH_TIMEOUT_SECS={largest}: {ended}"))
        .refused("there is no repository to fetch from");
}
