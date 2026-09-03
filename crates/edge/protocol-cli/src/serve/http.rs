//! One HTTP/1.1 request in, one response out, over `std::net`.
//!
//! # Why this is hand-written
//!
//! `AGENTS.md` § *Dependencies* asks for a refusal to be recorded where the refusal happens, and
//! this is that record. A framework would have cost three things this workspace does not spend:
//!
//! * **An async runtime.** There is none. The contract's traits are `async fn` and every call site
//!   drives them with [`aep_contract::testing::block_on`], which busy-polls and is documented as
//!   belonging "in tests and in synchronous backends, nowhere else". `axum`/`hyper`/`tokio` would
//!   make this the first crate here to need a reactor, for a listener that answers one operator.
//! * **The declared MSRV.** `rust-version = "1.85"`, and `task msrv` builds the whole workspace on
//!   it with `--locked`. The modern HTTP stack tracks above that, and the break arrives through the
//!   lockfile with no commit of ours touching a line of Rust — which has already happened once here,
//!   via `idna_adapter`.
//! * **Surface nobody asked for.** This server answers a browser on loopback. It does not need
//!   TLS, HTTP/2, keep-alive, compression, multipart or a router.
//!
//! # What it refuses, and why that is safe
//!
//! | Refused | Answer | Why |
//! |---|---|---|
//! | anything but `HTTP/1.x` | `400` | a browser only speaks h2 over TLS, which is not offered |
//! | `Transfer-Encoding` | `411`, naming `Content-Length` | `fetch` chunks only a stream body; the page sends a fixed string |
//! | keep-alive | every answer says `Connection: close` | one request per connection; six per page load |
//! | a body over [`MAX_BODY`] | `413` | a server that allocates whatever it is told to is one anybody can stop |
//!
//! A TLS `ClientHello` arriving here is not valid UTF-8 and becomes a `400`, which is the correct
//! and legible failure rather than a hang.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

/// The most body this server will read, in bytes.
///
/// A status move carries under two hundred bytes. The cap is four orders of magnitude above that
/// and still small enough that a request claiming more is refused before anything is allocated.
pub(crate) const MAX_BODY: usize = 64 * 1024;

/// One request, as much of it as this server reads.
#[derive(Debug)]
pub(crate) struct Request {
    /// `GET` or `POST`; anything else is refused before this is built.
    pub(crate) method: String,
    /// The path, percent-decoded, with the query removed.
    pub(crate) path: String,
    /// The query, percent-decoded, in the order it arrived.
    pub(crate) query: Vec<(String, String)>,
    /// Header names lowercased, so a caller matches one spelling.
    pub(crate) headers: BTreeMap<String, String>,
    /// The body, empty when there was none.
    pub(crate) body: String,
}

impl Request {
    /// The first value given for a query key, if any.
    pub(crate) fn query_value(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    /// One header, by its lowercased name.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

/// What to send back.
#[derive(Debug)]
pub(crate) struct Response {
    /// The status code.
    pub(crate) status: u16,
    /// The media type, sent verbatim.
    pub(crate) content_type: &'static str,
    /// The bytes.
    pub(crate) body: String,
}

impl Response {
    /// A JSON answer.
    pub(crate) fn json(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body,
        }
    }

    /// An HTML page.
    pub(crate) fn html(body: String) -> Self {
        Self {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body,
        }
    }

    /// A refusal, as JSON, so a caller parses one shape whether it was answered or refused.
    ///
    /// The key is `refused` and not `error`: this server distinguishes *the answer is no* from *this
    /// is not a question*, the same split the CLI draws between an exit code and a message on
    /// stderr, and a caller that has to tell them apart should not have to read prose to do it.
    pub(crate) fn refusal(status: u16, detail: &str) -> Self {
        let body = serde_json::json!({ "refused": detail }).to_string();
        Self::json(status, body)
    }

    /// Writes the response and closes.
    pub(crate) fn write_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let reason = reason_for(self.status);
        // `no-store` because every answer is a fact about a store that another process may be
        // changing, and a cached board is a board that lies. `nosniff` because the body is JSON and
        // a browser that guesses otherwise is a browser running something.
        let head = format!(
            "HTTP/1.1 {} {reason}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Cache-Control: no-store\r\n\
             X-Content-Type-Options: nosniff\r\n\
             Connection: close\r\n\r\n",
            self.status,
            self.content_type,
            self.body.len()
        );
        stream.write_all(head.as_bytes())?;
        stream.write_all(self.body.as_bytes())?;
        stream.flush()
    }
}

/// Reads one request, or says why it will not.
///
/// Takes a [`BufRead`] rather than a socket so the whole parser is testable from a byte slice.
pub(crate) fn read_request(reader: &mut impl BufRead) -> Result<Request, Response> {
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.is_empty() {
        return Err(Response::refusal(400, "no request line arrived"));
    }
    let mut parts = line.trim_end().split(' ');
    let (Some(method), Some(target), Some(version)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(Response::refusal(
            400,
            "a request line is `METHOD TARGET HTTP/1.1`",
        ));
    };
    if !version.starts_with("HTTP/1.") {
        return Err(Response::refusal(
            400,
            "this server speaks HTTP/1.1 and nothing else",
        ));
    }
    if method != "GET" && method != "POST" {
        return Err(Response::refusal(405, "this server answers GET and POST"));
    }

    let (raw_path, raw_query) = target.split_once('?').unwrap_or((target, ""));
    let path = percent_decoded(raw_path);
    let query = raw_query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decoded(key), percent_decoded(value))
        })
        .collect();

    let mut headers = BTreeMap::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() {
            return Err(Response::refusal(400, "the headers did not read"));
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            headers.insert(name.trim().to_lowercase(), value.trim().to_owned());
        }
    }

    if headers.contains_key("transfer-encoding") {
        return Err(Response::refusal(
            411,
            "this server reads a body only with Content-Length",
        ));
    }

    let length = match headers.get("content-length") {
        None => 0,
        Some(raw) => match raw.parse::<usize>() {
            Ok(length) => length,
            Err(_) => return Err(Response::refusal(400, "Content-Length is not a number")),
        },
    };
    if length > MAX_BODY {
        return Err(Response::refusal(
            413,
            "a request to this server is smaller than 64 KiB",
        ));
    }
    let mut body = vec![0_u8; length];
    if length > 0 && reader.read_exact(&mut body).is_err() {
        return Err(Response::refusal(400, "the body did not read"));
    }
    let Ok(body) = String::from_utf8(body) else {
        return Err(Response::refusal(400, "the body is not UTF-8"));
    };

    Ok(Request {
        method: method.to_owned(),
        path,
        query,
        headers,
        body,
    })
}

/// Reads one request off a socket.
pub(crate) fn read_from(stream: &TcpStream) -> Result<Request, Response> {
    let mut reader = BufReader::new(stream);
    read_request(&mut reader)
}

/// `%20` and `+` become what they stand for; anything else is left alone.
fn percent_decoded(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                } else {
                    out.push(b'%');
                    index += 1;
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The reason phrase for a status, for the statuses this server sends.
fn reason_for(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Payload Too Large",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(raw: &str) -> Result<Request, Response> {
        read_request(&mut raw.as_bytes())
    }

    #[test]
    fn a_request_line_without_a_version_is_refused_rather_than_guessed() {
        let refused = read("GET /api/board\r\n\r\n").expect_err("two words are not a request line");
        assert_eq!(refused.status, 400);
        assert!(
            refused.body.contains("METHOD TARGET"),
            "the refusal says what a request line is: {}",
            refused.body
        );
    }

    #[test]
    fn a_version_this_server_does_not_speak_is_refused_by_name() {
        let refused = read("GET / HTTP/2.0\r\n\r\n").expect_err("h2 is not offered");
        assert_eq!(refused.status, 400);
        assert!(refused.body.contains("HTTP/1.1"), "{}", refused.body);
    }

    #[test]
    fn a_chunked_body_is_refused_with_the_header_that_would_have_worked() {
        let refused = read("POST /api/x HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n")
            .expect_err("chunked is not read");
        assert_eq!(refused.status, 411);
        assert!(
            refused.body.contains("Content-Length"),
            "a refusal that does not name the way through is a dead end: {}",
            refused.body
        );
    }

    #[test]
    fn a_body_longer_than_the_cap_is_refused_before_it_is_allocated() {
        let raw = format!(
            "POST /api/x HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_BODY + 1
        );
        let refused = read(&raw).expect_err("the cap holds");
        assert_eq!(refused.status, 413);
    }

    #[test]
    fn a_query_string_is_parsed_and_percent_decoded() {
        let request = read("GET /api/board?kind=story&q=a%20b HTTP/1.1\r\n\r\n").expect("read");
        assert_eq!(request.path, "/api/board");
        assert_eq!(request.query_value("kind"), Some("story"));
        assert_eq!(
            request.query_value("q"),
            Some("a b"),
            "a percent escape is decoded, or an id with one in it never matches"
        );
    }

    #[test]
    fn a_header_name_is_matched_however_the_client_spelled_it() {
        let request = read("GET / HTTP/1.1\r\nHOST: 127.0.0.1:1\r\n\r\n").expect("read");
        assert_eq!(
            request.header("host"),
            Some("127.0.0.1:1"),
            "header names are case-insensitive and a check that missed one would be a check nobody \
             passes"
        );
    }

    #[test]
    fn a_method_this_server_does_not_answer_is_refused_naming_the_two_it_does() {
        let refused = read("DELETE / HTTP/1.1\r\n\r\n").expect_err("DELETE is not answered");
        assert_eq!(refused.status, 405);
        assert!(refused.body.contains("GET and POST"), "{}", refused.body);
    }

    #[test]
    fn a_body_is_read_to_its_stated_length_and_no_further() {
        let request = read("POST /x HTTP/1.1\r\nContent-Length: 4\r\n\r\n{\"a\":1}")
            .expect("a body of the length it declared");
        assert_eq!(request.body, "{\"a\"");
    }
}
