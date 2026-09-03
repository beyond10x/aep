//! What each path answers, and what a caller must prove before it answers anything.
//!
//! Every read here is the compute half of a verb the terminal already has, reached through
//! `planning`'s served surface and through nothing else — so the board in a browser and
//! `protocol artifact board` are the same facts, not two renderings that can drift.
//!
//! # The guards run before the handlers, in this order
//!
//! 1. **The run token.** Printed once at startup and required on every request. A page in the
//!    operator's own browser can `fetch` a loopback port; it cannot read a token it never saw.
//! 2. **`Host`.** Must be this server's own, which is what stops a name that resolves to `127.0.0.1`
//!    from being used to reach it.
//! 3. **`Origin`, on writes.** Absent or exactly this server.
//! 4. **`--read-only`**, which refuses every write by name.
//!
//! None of that is authentication. The token proves the caller read the terminal, and a move it
//! makes is attributed to the same actor a terminal move is.

use crate::planning;

use super::http::{Request, Response};

/// The page, compiled in. No build step, no bundler, and nothing fetched at runtime — the gate
/// reaches no network, and a page that pulled a script from a CDN would put one in it.
const PAGE: &str = include_str!("index.html");

/// What a request needs to know about the server answering it.
pub(crate) struct Serving {
    /// Where the plan is.
    pub(crate) location: planning::StoreLocation,
    /// The token every request carries.
    pub(crate) token: String,
    /// The port, so `Host` can be checked against it.
    pub(crate) port: u16,
    /// Whether every write is refused.
    pub(crate) read_only: bool,
}

/// Answers one request.
pub(crate) fn answer(serving: &Serving, request: &Request) -> Response {
    if let Some(refused) = refuse_the_caller(serving, request) {
        return refused;
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => Response::html(PAGE.to_owned()),
        ("GET", "/api/store") => read(|| planning::store_of(&serving.location)),
        ("GET", "/api/board") => {
            let kind = request.query_value("kind").filter(|kind| !kind.is_empty());
            read(|| planning::board_of(&serving.location, kind))
        }
        ("GET", path) if path.starts_with("/api/artifact/") => {
            let rest = &path["/api/artifact/".len()..];
            match rest.strip_suffix("/explain") {
                Some(id) => read(|| planning::explained_of(&serving.location, id)),
                None => read(|| planning::shown_of(&serving.location, rest)),
            }
        }
        ("POST", path) if path.starts_with("/api/artifact/") => {
            let Some(id) = path["/api/artifact/".len()..].strip_suffix("/move") else {
                return Response::refusal(404, "no such path");
            };
            move_it(serving, request, id)
        }
        _ => Response::refusal(404, "no such path"),
    }
}

/// Why this caller is not answered, when it is not.
fn refuse_the_caller(serving: &Serving, request: &Request) -> Option<Response> {
    if request.query_value("t") != Some(serving.token.as_str())
        && request.header("x-protocol-token") != Some(serving.token.as_str())
    {
        return Some(Response::refusal(
            403,
            "this server answers the token it printed at startup, and nothing else",
        ));
    }
    // A `Host` this server does not answer to is a name that resolved here, which is the shape of a
    // rebinding attack: the browser thinks it is talking to somebody else's origin.
    let expected = [
        format!("127.0.0.1:{}", serving.port),
        format!("localhost:{}", serving.port),
    ];
    match request.header("host") {
        Some(host) if expected.iter().any(|one| one == host) => {}
        _ => {
            return Some(Response::refusal(
                403,
                "this server answers to 127.0.0.1 and localhost on its own port",
            ))
        }
    }
    if request.method == "POST" {
        if let Some(origin) = request.header("origin") {
            let mine = [
                format!("http://127.0.0.1:{}", serving.port),
                format!("http://localhost:{}", serving.port),
            ];
            if !mine.iter().any(|one| one == origin) {
                return Some(Response::refusal(
                    403,
                    "a write comes from this server's own page",
                ));
            }
        }
    }
    None
}

/// Serialises whatever a read produced, or reports why it produced nothing.
///
/// An `Err` here is *this is not a question* — no such artifact, a store that will not read — which
/// is a `404` when the caller named something absent and a `503` when the store itself is the
/// problem. The two are told apart by what the message says, because that is the only signal the
/// error carries; a caller sees a distinct code either way.
fn read<T: serde::Serialize>(produce: impl FnOnce() -> anyhow::Result<T>) -> Response {
    match produce() {
        Ok(value) => match serde_json::to_string(&value) {
            Ok(body) => Response::json(200, body),
            Err(error) => {
                Response::refusal(503, &format!("the answer would not serialise: {error}"))
            }
        },
        Err(error) => {
            let detail = format!("{error:#}");
            let status = if detail.contains("no artifact") || detail.contains("not in the store") {
                404
            } else {
                503
            };
            Response::refusal(status, &detail)
        }
    }
}

/// Moves an artifact, and answers with what it did or with the refusal the ladder gave.
fn move_it(serving: &Serving, request: &Request, id: &str) -> Response {
    if serving.read_only {
        return Response::refusal(
            403,
            "this server was started `--read-only`; restart it without that to move anything",
        );
    }
    let Ok(asked) = serde_json::from_str::<serde_json::Value>(&request.body) else {
        return Response::refusal(400, "a move is `{\"to\": \"<status>\"}`");
    };
    let Some(to) = asked.get("to").and_then(serde_json::Value::as_str) else {
        return Response::refusal(400, "a move names the status to reach, as `to`");
    };
    let now = planning::now_at_the_edge();
    match planning::moved_by(&serving.location, id, to, &now) {
        Ok(outcome) => {
            let status = if outcome.was_refused() { 409 } else { 200 };
            match serde_json::to_string(&outcome) {
                Ok(body) => Response::json(status, body),
                Err(error) => {
                    Response::refusal(503, &format!("the answer would not serialise: {error}"))
                }
            }
        }
        Err(error) => Response::refusal(400, &format!("{error:#}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::http::read_request;

    fn serving(read_only: bool) -> Serving {
        Serving {
            location: planning::StoreLocation::at(None, None),
            token: "abc123".to_owned(),
            port: 8899,
            read_only,
        }
    }

    fn request(raw: &str) -> Request {
        read_request(&mut raw.as_bytes()).expect("a well-formed request")
    }

    #[test]
    fn a_request_without_the_run_token_is_refused_even_from_localhost() {
        let asked = request("GET /api/board HTTP/1.1\r\nHost: 127.0.0.1:8899\r\n\r\n");
        let answer = answer(&serving(false), &asked);
        assert_eq!(
            answer.status, 403,
            "loopback is not a credential: any page in the same browser can reach this port"
        );
    }

    #[test]
    fn a_host_this_server_does_not_answer_to_is_refused() {
        let asked = request("GET /api/board?t=abc123 HTTP/1.1\r\nHost: plan.example:8899\r\n\r\n");
        assert_eq!(
            answer(&serving(false), &asked).status,
            403,
            "a name that resolves here is the shape of a rebinding attack"
        );
    }

    #[test]
    fn a_write_from_another_origin_is_refused() {
        let asked = request(
            "POST /api/artifact/story:x/move?t=abc123 HTTP/1.1\r\n\
             Host: 127.0.0.1:8899\r\nOrigin: http://evil.example\r\n\
             Content-Length: 16\r\n\r\n{\"to\":\"active\"}\n",
        );
        assert_eq!(answer(&serving(false), &asked).status, 403);
    }

    #[test]
    fn a_transition_is_refused_by_name_when_the_server_is_read_only() {
        let asked = request(
            "POST /api/artifact/story:x/move?t=abc123 HTTP/1.1\r\n\
             Host: 127.0.0.1:8899\r\nContent-Length: 16\r\n\r\n{\"to\":\"active\"}\n",
        );
        let answer = answer(&serving(true), &asked);
        assert_eq!(answer.status, 403);
        assert!(
            answer.body.contains("--read-only"),
            "the refusal names the flag that caused it: {}",
            answer.body
        );
    }

    #[test]
    fn a_path_this_server_does_not_answer_is_a_not_found_rather_than_a_guess() {
        let asked = request("GET /api/nothing?t=abc123 HTTP/1.1\r\nHost: 127.0.0.1:8899\r\n\r\n");
        assert_eq!(answer(&serving(false), &asked).status, 404);
    }

    #[test]
    fn the_page_is_served_at_the_root_and_carries_no_remote_reference() {
        let asked = request("GET /?t=abc123 HTTP/1.1\r\nHost: 127.0.0.1:8899\r\n\r\n");
        let answer = answer(&serving(false), &asked);
        assert_eq!(answer.status, 200);
        assert!(
            !answer.body.contains("http://") && !answer.body.contains("https://"),
            "a page that fetched a script from a network would put a network in the gate"
        );
    }
}
