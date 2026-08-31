//! `protocol serve` over a real socket.
//!
//! The unit tests in `src/serve/` drive the parser and the guards from byte slices, which is where
//! the shapes are decided. This drives the whole verb: a child process, an ephemeral port, and HTTP
//! over a `TcpStream` — because the things it checks cannot be checked any other way. That the
//! startup line names a port a client can reach, that the token it prints is the one the server
//! wants, and that a move made over the wire is on disk afterwards are facts about a process, not
//! about a function.
//!
//! Nothing here reaches a network: the server binds `127.0.0.1:0`, and the client is a `TcpStream`
//! to the port it printed.

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

/// A child that is killed however this test leaves.
///
/// Without this a failing assertion leaves a server holding a port and a store for as long as the
/// test runner lives.
struct Served {
    child: Child,
    // Keep the pipe open for the child's lifetime. The read-only server prints one more startup
    // line after its URL; dropping the reader at the URL races that write and kills the server
    // with `Broken pipe` before the first request can be answered.
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// A store of this test's own making, built through the CLI so every document in it is one the
/// store itself wrote.
///
/// Deliberately not a copy of this repository's plan. That plan carries cross-repository edges whose
/// manifest is not copied with it, and — more to the point — a test that asserted on its contents
/// would go red whenever somebody filed a story.
fn scratch_store(name: &str) -> PathBuf {
    let store = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("serve-{name}"));
    let _ = std::fs::remove_dir_all(&store);
    std::fs::create_dir_all(&store).expect("the store directory");
    let root = repository();
    for (kind, slug, title) in [
        ("story", "first-light", "The first thing the board shows"),
        ("story", "second-thought", "Something to move"),
        ("epic", "the-whole-point", "What the stories decompose"),
    ] {
        let made = Command::new(env!("CARGO_BIN_EXE_protocol"))
            .args([
                "artifact",
                "new",
                kind,
                slug,
                "--title",
                title,
                "--store",
                &store.display().to_string(),
                "--root",
                &root.display().to_string(),
            ])
            .output()
            .expect("the protocol binary runs");
        assert!(
            made.status.success(),
            "the fixture is built by the store itself: {}",
            String::from_utf8_lossy(&made.stderr)
        );
    }
    store
}

/// Starts the server and learns its port and token from the line it prints.
///
/// The startup line is the only way this test can know either — `--port 0` means the operating
/// system chose — so a server that printed nothing leaves nothing to connect to, and the failure
/// says so rather than timing out against a port that was never opened.
fn serve(store: &Path, extra: &[&str]) -> Served {
    let root = repository();
    let mut args: Vec<String> = vec![
        "serve".to_owned(),
        "--store".to_owned(),
        store.display().to_string(),
        "--root".to_owned(),
        root.display().to_string(),
        "--port".to_owned(),
        "0".to_owned(),
    ];
    args.extend(extra.iter().map(|flag| (*flag).to_owned()));

    let mut child = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(&args)
        .stdout(Stdio::piped())
        // Handler panics belong in the test output. Piping stderr without draining it hid the
        // server-side cause behind a client-side `ECONNRESET` in the release gate.
        .stderr(Stdio::inherit())
        .spawn()
        .expect("the protocol binary runs");

    let stdout = child.stdout.take().expect("stdout is piped");
    let mut stdout = BufReader::new(stdout);
    let mut url = None;
    for _ in 0..5 {
        let mut line = String::new();
        match stdout.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) if line.starts_with("http://127.0.0.1:") => {
                url = Some(line);
                break;
            }
            Ok(_) => {}
        }
    }
    let url = url.unwrap_or_else(|| {
        let _ = child.kill();
        panic!("the startup line is how this test learns the port and the token, and none arrived")
    });

    let rest = url.trim_end().trim_start_matches("http://127.0.0.1:");
    let (port, token) = rest.split_once("/?t=").expect("the startup line's shape");
    Served {
        child,
        _stdout: stdout,
        port: port.parse().expect("a port"),
        token: token.to_owned(),
    }
}

/// One request over a fresh connection, because the server answers `Connection: close`.
fn ask(served: &Served, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
    let mut stream =
        TcpStream::connect(("127.0.0.1", served.port)).expect("the port it printed is open");
    let join = if path.contains('?') { "&" } else { "?" };
    let mut request = format!(
        "{method} {path}{join}t={} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
        served.token, served.port
    );
    if let Some(body) = body {
        let _ = write!(
            request,
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        );
    }
    request.push_str("Connection: close\r\n\r\n");
    if let Some(body) = body {
        request.push_str(body);
    }
    stream.write_all(request.as_bytes()).expect("written");
    stream.flush().expect("flushed");

    let answer = read_answer(&mut stream);
    let status = answer
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = answer.split_once("\r\n\r\n").map_or("", |(_, rest)| rest);
    (status, body.to_owned())
}

/// A request that carries no token at all.
fn ask_untokened(served: &Served, path: &str) -> u16 {
    let mut stream = TcpStream::connect(("127.0.0.1", served.port)).expect("open");
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        served.port
    );
    stream.write_all(request.as_bytes()).expect("written");
    let answer = read_answer(&mut stream);
    answer
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0)
}

/// Reads exactly one HTTP answer according to its framing, without depending on how TCP reports the
/// server's close.
///
/// The response contract carries `Content-Length`, so reading until EOF adds no evidence and made
/// the test depend on whether a runner reports the close as EOF or `ECONNRESET`. Missing or
/// truncated framing still fails through `read_exact`.
fn read_answer(stream: &mut TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut head = String::new();
    loop {
        let read = reader
            .read_line(&mut head)
            .expect("the HTTP response headers read");
        assert_ne!(read, 0, "the HTTP response headers are complete");
        if head.ends_with("\r\n\r\n") {
            break;
        }
    }
    let length = response_length(&head).expect("the HTTP response carries Content-Length");
    let mut body = vec![0_u8; length];
    reader
        .read_exact(&mut body)
        .expect("the complete Content-Length body arrives");
    head.push_str(&String::from_utf8(body).expect("the HTTP answer body is UTF-8"));
    head
}

fn response_length(head: &str) -> Option<usize> {
    head.lines().find_map(|line| {
        line.strip_prefix("Content-Length: ")
            .and_then(|raw| raw.parse::<usize>().ok())
    })
}

#[test]
fn response_framing_names_exactly_how_many_body_bytes_must_arrive() {
    assert_eq!(
        response_length("HTTP/1.1 403 Forbidden\r\nContent-Length: 4\r\n\r\n"),
        Some(4)
    );
    assert_eq!(response_length("HTTP/1.1 403 Forbidden\r\n\r\n"), None);
}

/// **The board a browser reads is the board the terminal prints**, and the token is what stands
/// between the store and every other page open in the same browser.
#[test]
fn the_board_is_answered_over_a_socket_and_only_to_the_token_it_printed() {
    let store = scratch_store("board");
    let served = serve(&store, &[]);

    let (status, body) = ask(&served, "GET", "/api/board", None);
    assert_eq!(status, 200, "{body}");
    let columns: serde_json::Value = serde_json::from_str(&body).expect("the board is JSON");
    let columns = columns.as_array().expect("a list of columns");
    assert!(!columns.is_empty(), "the fixture put three artifacts in it");
    assert!(
        columns[0].get("status").is_some() && columns[0].get("artifacts").is_some(),
        "a column is a status and what stands in it: {}",
        columns[0]
    );

    assert_eq!(
        ask_untokened(&served, "/api/board"),
        403,
        "loopback is not a credential: any page in the same browser can reach this port, and only \
         the token tells them apart"
    );
}

/// **A refused move answers with what the ladder would have permitted**, so the page can offer it.
#[test]
fn an_illegal_move_is_a_conflict_carrying_every_legal_target() {
    let store = scratch_store("illegal");
    let served = serve(&store, &[]);

    let (status, body) = ask(
        &served,
        "POST",
        "/api/artifact/story:first-light/move",
        Some(r#"{"to":"implemented"}"#),
    );
    assert_eq!(status, 409, "{body}");
    let answered: serde_json::Value = serde_json::from_str(&body).expect("JSON");
    let legal = answered["refusal"]["refused"]["legal"]
        .as_array()
        .expect("a refusal names where it could have gone instead");
    assert!(
        !legal.is_empty(),
        "a refusal that does not answer the question it creates is a dead end: {body}"
    );
    assert_eq!(
        answered["refusal"]["refused"]["reason"], "not_on_the_ladder",
        "the reason is a flat tag a page branches on: {body}"
    );
    assert_eq!(
        answered["made"].as_array().expect("a list").len(),
        0,
        "nothing was written, so nothing is claimed"
    );
}

/// **A legal move is written, and the next read sees it** — which is what makes the board usable
/// for triage rather than for looking at.
#[test]
fn a_legal_move_is_written_and_the_next_read_sees_it() {
    let store = scratch_store("legal");
    let served = serve(&store, &[]);

    let (before, body) = ask(&served, "GET", "/api/artifact/story:second-thought", None);
    assert_eq!(before, 200, "{body}");
    let before: serde_json::Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(before["status"], "draft");

    let (status, body) = ask(
        &served,
        "POST",
        "/api/artifact/story:second-thought/move",
        Some(r#"{"to":"archived"}"#),
    );
    assert_eq!(status, 200, "{body}");
    let moved: serde_json::Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(moved["made"][0]["from"], "draft");
    assert_eq!(moved["made"][0]["to"], "archived");

    let (_, body) = ask(&served, "GET", "/api/artifact/story:second-thought", None);
    let after: serde_json::Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(after["status"], "archived");
    assert!(
        after["revision"].as_u64() > before["revision"].as_u64(),
        "a write that changed the document bumped its revision"
    );

    // The document on disk, not the answer the server gave about it.
    let written = std::fs::read_to_string(store.join("story/second-thought.md")).expect("readable");
    assert!(
        written.contains("status: archived"),
        "the move reached the file a second process would read: {written}"
    );
}

/// **`--read-only` refuses every write by name**, so the board can be opened without a write
/// surface at all.
#[test]
fn a_read_only_server_answers_reads_and_refuses_every_move() {
    let store = scratch_store("readonly");
    let served = serve(&store, &["--read-only"]);

    let (reads, _) = ask(&served, "GET", "/api/board", None);
    assert_eq!(reads, 200, "read-only is about writes");

    let (status, body) = ask(
        &served,
        "POST",
        "/api/artifact/story:second-thought/move",
        Some(r#"{"to":"archived"}"#),
    );
    assert_eq!(status, 403, "{body}");
    assert!(
        body.contains("--read-only"),
        "the refusal names the flag that caused it, or the operator hunts for it: {body}"
    );

    let written = std::fs::read_to_string(store.join("story/second-thought.md")).expect("readable");
    assert!(written.contains("status: draft"), "nothing was written");
}

/// **The rungs the page draws come with their prices**, so a cost is read rather than learnt by
/// being refused.
#[test]
fn explain_answers_the_rungs_a_page_would_draw_and_what_each_costs() {
    let store = scratch_store("explain");
    let served = serve(&store, &[]);

    let (status, body) = ask(
        &served,
        "GET",
        "/api/artifact/story:second-thought/explain",
        None,
    );
    assert_eq!(status, 200, "{body}");
    let explained: serde_json::Value = serde_json::from_str(&body).expect("JSON");
    let next = explained["next"].as_array().expect("the rungs from here");
    assert!(
        !next.is_empty(),
        "a draft story has somewhere to go: {body}"
    );
    assert!(
        next[0].get("status").is_some() && next[0].get("needs").is_some(),
        "each rung is a status and what it costs: {}",
        next[0]
    );
}
