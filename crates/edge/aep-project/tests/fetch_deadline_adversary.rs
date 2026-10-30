//! Adversary cases for `story:an-unreachable-protocol-source-fails-within-a-deadline`: where the
//! deadline, the stopped transport and the named refusal do not hold as `aep` runs.
//!
//! Each resolution runs in a child process of this test binary, as `tests/support` does, so the
//! variables it reads are set on the child only.

mod support;

use std::io::Read as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::{resolve_in_child, PinnedRemote};

/// The child entry `resolve_in_child` and `child_command` run.
const ENTRY: &str = "child_resolves_the_source_its_parent_named";

#[test]
#[ignore = "a child process entry point, run by resolve_in_child"]
fn child_resolves_the_source_its_parent_named() {
    support::child_entry();
}

/// This binary's child entry resolving `locator` with exactly `environment` set, not yet run.
fn child_command(locator: &str, environment: &[(&str, &str)]) -> Command {
    let mut command =
        Command::new(std::env::current_exe().expect("the test binary can name itself"));
    command.args([
        ENTRY,
        "--exact",
        "--ignored",
        "--nocapture",
        "--test-threads",
        "1",
    ]);
    for variable in [
        "AEP_CACHE_DIR",
        "AEP_CACHE_GROUP_WRITABLE",
        "AEP_FETCH_TIMEOUT_SECS",
    ] {
        command.env_remove(variable);
    }
    command
        .env("AEP_TEST_PROTOCOL_SOURCE", locator)
        .envs(environment.iter().copied());
    command
}

/// Reads `stream` until its peer closes it or `watch` passes; whether it closed.
fn closes_within(stream: &mut std::net::TcpStream, watch: Duration) -> bool {
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

/// A terminal's Ctrl-C reaches the foreground process group, which is `aep`'s. Before this unit
/// Git and `git-remote-https` were in that group and went with it; now Git starts a group of its
/// own, so `aep` dies and Git, no longer under any deadline, keeps the connection open. GNU
/// `timeout` signals its command's whole group the same way, so a caller's limit leaks Git too.
#[cfg(unix)]
#[test]
fn an_interrupt_to_aeps_process_group_stops_the_fetch_and_its_transport_with_it() {
    use std::os::unix::process::CommandExt as _;

    let root = support::scratch("adv-fetch-interrupt");
    let cache = root.join("cache");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("bound").port();
    let locator = format!(
        "git+https://127.0.0.1:{port}/protocols.git#{}",
        "d".repeat(40)
    );
    let mut command = child_command(
        &locator,
        &[
            ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path")),
            ("AEP_FETCH_TIMEOUT_SECS", "60"),
        ],
    );
    // The child stands in for `aep` run from a shell: the leader of the foreground group.
    let mut child = command
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the child runs");

    listener.set_nonblocking(true).expect("non-blocking accept");
    let accept_until = Instant::now() + Duration::from_secs(20);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= accept_until {
                    child.kill().ok();
                    child.wait().ok();
                    panic!("Git never connected to the listener");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accepting: {error}"),
        }
    };
    stream.set_nonblocking(false).expect("blocking reads");

    // Ctrl-C: SIGINT to the foreground process group.
    Command::new("kill")
        .args(["-s", "INT", "--", &format!("-{}", child.id())])
        .status()
        .expect("kill runs");
    let interrupted = Instant::now();
    loop {
        if child
            .try_wait()
            .expect("the child can be waited on")
            .is_some()
        {
            break;
        }
        if interrupted.elapsed() > Duration::from_secs(10) {
            child.kill().ok();
            child.wait().ok();
            panic!("the interrupted child did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let closed = closes_within(&mut stream, Duration::from_secs(3));
    // Closing our end makes an orphaned Git's transport fail and exit, so nothing outlives the test.
    drop(stream);
    support::remove(&root);
    assert!(
        closed,
        "Git's transport kept its connection open 3 s after an interrupt stopped aep: Git runs in a \
         process group of its own and nothing stops it once aep is gone"
    );
}

/// Acceptance 1 calls any positive integer a deadline. The largest one overflows the deadline's
/// `Instant` and panics the process instead of fetching or refusing by name.
#[test]
fn the_largest_whole_number_of_seconds_is_a_deadline_not_a_panic() {
    let root = support::scratch("adv-fetch-deadline-max");
    let cache = root.join("cache");
    let remote = root.join("no-such-repository");
    let locator = format!("git+file://{}#{}", remote.display(), "e".repeat(40));
    let output = child_command(
        &locator,
        &[
            ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path")),
            ("AEP_FETCH_TIMEOUT_SECS", &u64::MAX.to_string()),
        ],
    )
    .output()
    .expect("the child runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let outcome = stdout
        .lines()
        .find_map(|line| line.split_once("aep-test-outcome: ").map(|(_, rest)| rest));
    support::remove(&root);
    let outcome = outcome.unwrap_or_else(|| {
        panic!(
            "resolution produced no outcome ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert!(
        outcome.contains(&locator) || outcome.contains("AEP_FETCH_TIMEOUT_SECS"),
        "{outcome}"
    );
}

/// The pod case of the story with a read-only cache mount that does not hold this pin (the pin
/// moved, the image's cache did not): the refusal must name the pin, the cache and
/// `AEP_CACHE_DIR` like any other failure to fetch it (acceptance 2, the story's outcome).
#[cfg(unix)]
#[test]
fn a_read_only_cache_without_the_pin_is_refused_naming_the_pin_the_cache_and_aep_cache_dir() {
    use std::os::unix::fs::PermissionsExt as _;

    let root = support::scratch("adv-fetch-read-only-cache");
    let remote = PinnedRemote::create(&root);
    let cache = root.join("cache");
    std::fs::create_dir_all(&cache).expect("the cache directory");
    std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o555))
        .expect("the test owns the cache");
    if std::fs::create_dir(cache.join("probe")).is_ok() {
        // A privileged user writes through the mode; the case cannot be built here.
        support::remove(&root);
        return;
    }
    let refusal = resolve_in_child(
        ENTRY,
        &remote.locator(),
        &[("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path"))],
    )
    .refused("a read-only cache without the pin cannot fetch it");
    support::remove(&root);
    assert!(
        refusal.contains(&remote.locator()),
        "the refusal names the pin: {refusal}"
    );
    assert!(
        refusal.contains(&cache.display().to_string()),
        "the refusal names the cache: {refusal}"
    );
    assert!(
        refusal.contains("AEP_CACHE_DIR"),
        "the refusal names AEP_CACHE_DIR: {refusal}"
    );
}

/// Acceptance 4 says "refusals change nothing". A fetch that fails leaves the bare object cache
/// `git init --bare` created behind it.
#[test]
fn a_refused_fetch_changes_nothing_in_the_cache() {
    let root = support::scratch("adv-fetch-refusal-changes-nothing");
    let cache = root.join("cache");
    let remote = root.join("no-such-repository");
    let locator = format!("git+file://{}#{}", remote.display(), "e".repeat(40));
    resolve_in_child(
        ENTRY,
        &locator,
        &[("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path"))],
    )
    .refused("there is no repository to fetch from");
    let left = list(&cache);
    support::remove(&root);
    assert!(left.is_empty(), "a refused fetch left {left:?}");
}

/// Every path below `directory`, relative to it.
fn list(directory: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).into_iter().flatten().flatten() {
            let path = entry.path();
            found.push(
                path.strip_prefix(directory)
                    .expect("below the directory")
                    .display()
                    .to_string(),
            );
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                pending.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The deadline bounds Git, not the reading of its output. A transport that leaves a process of
/// another session holding Git's standard error (a daemonizing helper; `setsid` stands in for
/// one) makes Git finish at once and resolution wait for that process instead, past the deadline.
#[cfg(target_os = "linux")]
#[test]
fn a_transport_that_leaves_a_detached_process_holding_stderr_does_not_outlast_the_deadline() {
    use std::os::unix::fs::PermissionsExt as _;

    let root = support::scratch("adv-fetch-detached-stderr");
    let cache = root.join("cache");
    let bin = root.join("bin");
    let pids = root.join("detached.pids");
    support::write(
        &bin.join("ssh"),
        &format!(
            "#!/bin/sh\nsetsid sleep 12 </dev/null >/dev/null &\necho $! >> '{}'\nexit 255\n",
            pids.display()
        ),
    );
    std::fs::set_permissions(bin.join("ssh"), std::fs::Permissions::from_mode(0o755))
        .expect("the fixture is writable");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let locator = format!(
        "git+ssh://adversary.invalid/protocols.git#{}",
        "f".repeat(40)
    );

    let started = Instant::now();
    let output = child_command(
        &locator,
        &[
            ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path")),
            ("AEP_FETCH_TIMEOUT_SECS", "2"),
            ("PATH", &path),
        ],
    )
    .output()
    .expect("the child runs");
    let elapsed = started.elapsed();

    for pid in std::fs::read_to_string(&pids)
        .unwrap_or_default()
        .split_whitespace()
    {
        Command::new("kill")
            .args(["-s", "KILL", "--", pid])
            .stderr(Stdio::null())
            .status()
            .ok();
    }
    support::remove(&root);
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("aep-test-outcome: "),
        "the child reported an outcome"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "a 2 s fetch deadline returned after {elapsed:?}"
    );
}
