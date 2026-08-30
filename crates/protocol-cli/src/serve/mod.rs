//! `protocol serve` — the plan in a browser, and the transitions that move it.
//!
//! A board is a shape and a terminal prints lines, so triage is the thing the CLI is worst at. This
//! answers a browser on loopback with the same facts `protocol artifact board`, `show` and `explain`
//! print, and takes status moves back through the same decision `protocol artifact move` makes.
//!
//! # It binds `127.0.0.1` and there is no flag that widens it
//!
//! This mutates a governed store over an unauthenticated port. A `--bind` flag is one typo away from
//! an open write endpoint on somebody's network, so widening it is a source change and a review.
//! Reaching it from another machine is `ssh -L`, which puts authentication where authentication
//! belongs.
//!
//! **There is no authentication and no authorisation.** The run token proves the caller read the
//! terminal and nothing more, and a move made here is attributed to the same actor a terminal move
//! is. A shared machine is out of scope.
//!
//! # One backend per request
//!
//! A handle held for the life of the process would serve documents from the moment it was opened —
//! `EntityBackend` hydrates once — and would latch permanently the first time the operator also used
//! the CLI. Opening per request is what the CLI already does per invocation, so the two coexist and
//! neither can serve a stale board.

pub(crate) mod api;
pub(crate) mod http;

use std::collections::hash_map::RandomState;
use std::fmt::Write as _;
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::planning::StoreLocation;

/// How long a connection may take to send its request or read its answer.
///
/// Not optional. Browsers open speculative connections that send no bytes, and without this each one
/// parks a thread on `read_line` until the tab is closed.
const PATIENCE: Duration = Duration::from_secs(10);

/// How many connections may be in flight at once.
///
/// Six per origin is what a browser opens; the rest of the margin is for a page that reloads while
/// its previous fetches are still running. Above this the answer is `503`, so a runaway page cannot
/// spawn threads without bound.
const AT_ONCE: usize = 64;

/// Serves the plan until interrupted.
pub(crate) fn run(location: &StoreLocation, port: u16, read_only: bool) -> Result<ExitCode> {
    // Read once, before the listener exists, so a store that will not open is reported as a refusal
    // to start rather than as a page full of errors.
    let opening =
        crate::planning::store_of(location).context("the plan this would serve does not read")?;

    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("127.0.0.1:{port} is not available"))?;
    let bound = listener.local_addr()?.port();
    let token = run_token();

    let serving = Arc::new(api::Serving {
        location: location.clone(),
        token: token.clone(),
        port: bound,
        read_only,
    });

    // stdout directly, not `outln!`: that macro exits the process on a write error, which is right
    // for a verb that has finished and wrong for a server that has not started.
    let mut out = std::io::stdout();
    writeln!(out, "protocol serve — {}", serialised(&opening))?;
    writeln!(out, "http://127.0.0.1:{bound}/?t={token}")?;
    if read_only {
        writeln!(out, "read-only: every transition is refused")?;
    }
    out.flush()?;

    let live = Arc::new(AtomicUsize::new(0));
    for accepted in listener.incoming() {
        let Ok(stream) = accepted else { continue };
        if live.load(Ordering::SeqCst) >= AT_ONCE {
            let _ = http::Response::refusal(503, "too many connections at once").write_to(
                &mut match stream.try_clone() {
                    Ok(clone) => clone,
                    Err(_) => continue,
                },
            );
            continue;
        }
        live.fetch_add(1, Ordering::SeqCst);
        let serving = Arc::clone(&serving);
        let live = Arc::clone(&live);
        // A panicking handler kills its own thread and the loop carries on; the connection dies with
        // no answer, which the browser reports as a failed fetch. Swallowing it would be worse.
        let _ = std::thread::Builder::new()
            .name("protocol-serve".to_owned())
            .spawn(move || {
                converse(&serving, stream);
                live.fetch_sub(1, Ordering::SeqCst);
            });
    }
    Ok(ExitCode::SUCCESS)
}

/// One connection: one request in, one answer out, then closed.
fn converse(serving: &api::Serving, mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(PATIENCE));
    let _ = stream.set_write_timeout(Some(PATIENCE));
    let answer = match http::read_from(&stream) {
        Ok(request) => api::answer(serving, &request),
        Err(refused) => refused,
    };
    let _ = answer.write_to(&mut stream);
}

/// A token for this run, so a page that never saw the terminal cannot write to the store.
///
/// `RandomState` is seeded by the operating system once per process, which is what makes this
/// unguessable; the clock and the process id are mixed in so two servers started in the same second
/// do not share one. No dependency, and nothing here claims to be a cryptographic random.
fn run_token() -> String {
    let mut token = String::new();
    for salt in 0_u64..2 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(salt);
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos()),
        );
        hasher.write_u32(std::process::id());
        let _ = write!(token, "{:016x}", hasher.finish());
    }
    token
}

/// The startup line's description of the plan.
fn serialised<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "a plan".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_token_is_long_enough_to_be_worth_having_and_differs_between_runs() {
        let one = run_token();
        let two = run_token();
        assert_eq!(one.len(), 32, "128 bits, written as hex");
        assert_ne!(
            one, two,
            "two servers started in the same second must not share a token, or the second one \
             inherits the first one's reach"
        );
        assert!(one.chars().all(|character| character.is_ascii_hexdigit()));
    }
}
