//! `AEP_FETCH_TIMEOUT_SECS` and `AEP_CACHE_DIR`, read from the environment of the process that
//! resolves a pinned protocol source
//! (`story:an-unreachable-protocol-source-fails-within-a-deadline`).
//!
//! The deadline itself, the stopped transport and the cached resolution are tested in
//! `project.rs` through the values these variables become; this file proves the variables are
//! what decide them, as `aep` runs.

mod support;

#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use support::resolve_in_child;

/// The child entry `resolve_in_child` runs.
const ENTRY: &str = "child_resolves_the_source_its_parent_named";

#[test]
#[ignore = "a child process entry point, run by resolve_in_child"]
fn child_resolves_the_source_its_parent_named() {
    support::child_entry();
}

/// When the one connection a never-answering loopback listener accepted was closed, if it was
/// within `watch`.
#[cfg(target_os = "linux")]
fn closed_within(listener: &std::net::TcpListener, watch: Duration) -> Option<Instant> {
    use std::io::Read as _;

    let until = Instant::now() + watch;
    listener.set_nonblocking(true).expect("non-blocking accept");
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= until {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accepting: {error}"),
        }
    };
    stream.set_nonblocking(false).expect("blocking reads");
    let mut buffer = [0_u8; 4096];
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        stream.set_read_timeout(Some(left)).expect("a read timeout");
        match stream.read(&mut buffer) {
            Ok(0) => return Some(Instant::now()),
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return None
            }
            Err(_) => return Some(Instant::now()),
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn the_fetch_deadline_in_the_environment_bounds_a_source_that_never_answers() {
    let root = support::scratch("fetch-deadline-env");
    let cache = root.join("cache");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("bound").port();
    let watcher = std::thread::spawn(move || closed_within(&listener, Duration::from_secs(20)));
    let locator = format!(
        "git+https://127.0.0.1:{port}/protocols.git#{}",
        "d".repeat(40)
    );

    let started = Instant::now();
    let refusal = resolve_in_child(
        ENTRY,
        &locator,
        &[
            ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path")),
            ("AEP_FETCH_TIMEOUT_SECS", "2"),
        ],
    )
    .refused("the source never answers");
    let returned = Instant::now();

    // The default is sixty seconds; returning in under five proves the variable set the deadline.
    let elapsed = returned - started;
    assert!(
        elapsed >= Duration::from_secs(2) && elapsed < Duration::from_secs(5),
        "{elapsed:?}: {refusal}"
    );
    assert!(refusal.contains("did not finish within 2 s"), "{refusal}");
    assert!(
        refusal.contains(&locator),
        "the refusal names the pin: {refusal}"
    );
    assert!(
        refusal.contains(&format!("into the cache {}", cache.display())),
        "the refusal names the cache `AEP_CACHE_DIR` selected: {refusal}"
    );
    assert!(refusal.contains("`AEP_CACHE_DIR`"), "{refusal}");
    let closed = watcher
        .join()
        .expect("the watcher finishes")
        .expect("Git's transport closed its connection");
    assert!(
        closed <= returned + Duration::from_secs(1),
        "the transport outlived the refusal by {:?}",
        closed.saturating_duration_since(returned)
    );
    support::remove(&root);
}

#[test]
fn a_fetch_deadline_that_is_not_a_positive_whole_number_is_refused_by_name() {
    let root = support::scratch("fetch-deadline-invalid");
    let cache = root.join("cache");
    let remote = root.join("no-such-repository");
    let locator = format!("git+file://{}#{}", remote.display(), "e".repeat(40));
    for value in ["0", "-5", "1.5", "ten", "60s", " 60"] {
        let refusal = resolve_in_child(
            ENTRY,
            &locator,
            &[
                ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path")),
                ("AEP_FETCH_TIMEOUT_SECS", value),
            ],
        )
        .refused(&format!("`{value}` is not a deadline"));
        assert!(
            refusal.contains("AEP_FETCH_TIMEOUT_SECS"),
            "{value}: {refusal}"
        );
        assert!(
            refusal.contains(&format!("`{value}`")),
            "{value}: {refusal}"
        );
        assert!(!cache.exists(), "{value}: a refusal changes nothing");
    }
    support::remove(&root);
}
