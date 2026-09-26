//! Adversarial cases for beyond10x/aep#38: the fenced `findings` block written as JSON, and the
//! JSON array `new --findings` takes.
//!
//! The contract under test is the unit's own documentation: `website/docs/reference/cli.md` says
//! "Every JSON array of entries is a YAML sequence, so the block reads it unchanged", and
//! `findings::render_block` says every message survives being written into the block as JSON.

use aep_backend_markdown::findings::{self, Finding, FindingsInput};

/// A body whose only content after its prose is `block`, fenced as `findings`.
fn body_with(block: &str) -> String {
    format!("# A review\n\nProse first.\n\n```findings\n{block}\n```\n")
}

/// The one finding a block written as JSON by a serializer holds, with `message` as its message.
fn only_message(block: &str) -> String {
    let parsed = findings::parse(&body_with(block))
        .unwrap_or_else(|error| panic!("a JSON block a serializer wrote is read: {error}"));
    assert_eq!(parsed.len(), 1, "one entry in, one finding out");
    parsed[0].message.clone()
}

/// Python's `json.dumps`, with its default `ensure_ascii=True`, writes a character outside the
/// Basic Multilingual Plane as a UTF-16 surrogate pair of `\u` escapes. That is valid JSON, so by
/// cli.md the block reads it unchanged.
#[test]
fn a_json_block_holding_a_surrogate_pair_escape_is_read_as_the_character_it_spells() {
    let message = only_message(
        r#"[{"file": "a.rs", "category": "c", "severity": "note", "message": "rocket 🚀 launched"}]"#,
    );
    assert_eq!(message, "rocket \u{1F680} launched");
}

/// PHP's `json_encode` escapes every `/` by default; `\/` is a JSON escape.
#[test]
fn a_json_block_holding_an_escaped_solidus_is_read() {
    let parsed = findings::parse(&body_with(
        r#"[{"file": "src\/lib.rs", "category": "c", "severity": "note", "message": "m"}]"#,
    ))
    .unwrap_or_else(|error| panic!("`\\/` is JSON: {error}"));
    assert_eq!(parsed[0].file, "src/lib.rs");
}

/// Go's `json.MarshalIndent(v, "", "\t")` and Python's `json.dumps(v, indent="\t")` indent with tabs.
#[test]
fn a_json_block_indented_with_tabs_is_read() {
    let message = only_message(
        "[\n\t{\n\t\t\"file\": \"a.rs\",\n\t\t\"category\": \"c\",\n\t\t\"severity\": \"note\",\n\t\t\"message\": \"m\"\n\t}\n]",
    );
    assert_eq!(message, "m");
}

/// Property: whatever `--findings` accepts, the block it renders reads back as the same findings.
///
/// `new --findings` runs `parse_json`, then `render_block`, then `parse` over the body. Each
/// message below is valid JSON, accepted by `parse_json`, and written raw by `serde_json` — and each
/// is a character YAML treats as a line break or refuses as non-printable.
#[test]
fn every_message_parse_json_accepts_reads_back_unchanged_from_the_block_render_block_writes() {
    let escaped = [
        ("LINE SEPARATOR", r"a b"),
        ("NEXT LINE", r"a\u0085b"),
        ("ZERO WIDTH NO-BREAK SPACE", r"zero﻿width"),
        ("DELETE", r"del\u007fchar"),
        ("C1 control", r"c1\u0080char"),
    ];
    let mut failures = Vec::new();
    for (name, message) in escaped {
        let text = format!(
            r#"[{{"file": "a.rs", "category": "c", "severity": "note", "message": "{message}"}}]"#
        );
        let accepted = findings::parse_json(&text)
            .unwrap_or_else(|error| panic!("{name}: `--findings` accepts it: {error}"));
        let body = format!("# A review\n\n{}", findings::render_block(&accepted));
        match findings::parse(&body) {
            Ok(read_back) if read_back == accepted => {}
            Ok(read_back) => failures.push(format!(
                "{name}: wrote {:?}, read back {:?}",
                accepted[0].message, read_back[0].message
            )),
            Err(error) => failures.push(format!("{name}: accepted, then refused: {error}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A vocabulary refusal in `--findings` input is positioned at the line that writes the value, when
/// the value is written the way `json.dumps` writes it — `ensure_ascii` escapes `ö`.
#[test]
fn a_vocabulary_refusal_in_json_input_is_positioned_at_the_line_a_serializer_escaped() {
    let text = "[\n  {\"file\": \"a.rs\", \"category\": \"c\", \"severity\": \"note\", \"message\": \"fine\"},\n  {\"file\": \"b.rs\", \"category\": \"c\", \"severity\": \"bl\\u00f6cker\", \"message\": \"m\"}\n]\n";
    let error = findings::parse_json(text).expect_err("`blöcker` is not a severity");
    assert_eq!(error.input, FindingsInput::Json);
    assert!(error.detail.contains("blöcker"), "{error}");
    assert_eq!(error.line, 3, "the second entry is on line 3: {error}");
}

/// A review's prose that quotes an example block inside a longer fence — as `cli.md` itself does,
/// with a `` ````markdown `` fence — is not stating findings: a markdown renderer shows it as code.
const QUOTED_EXAMPLE: &str = "# A review\n\nThe block should look like this:\n\n````markdown\n```findings\n[{\"file\": \"example.rs\", \"category\": \"c\", \"severity\": \"note\", \"message\": \"an example\"}]\n```\n````\n";

#[test]
fn a_findings_fence_quoted_inside_a_longer_fence_does_not_open_a_block() {
    assert!(
        !findings::opens_a_block(QUOTED_EXAMPLE),
        "a quoted example is prose, so `new --findings` must not refuse this body as ambiguous"
    );
}

#[test]
fn the_findings_a_review_states_are_not_the_example_its_prose_quotes() {
    let body = format!(
        "{QUOTED_EXAMPLE}\n```findings\n[{{\"file\": \"real.rs\", \"category\": \"c\", \"severity\": \"blocker\", \"message\": \"the real one\"}}]\n```\n"
    );
    let parsed: Vec<Finding> =
        findings::parse(&body).unwrap_or_else(|error| panic!("readable: {error}"));
    let files: Vec<&str> = parsed.iter().map(|finding| finding.file.as_str()).collect();
    assert_eq!(files, vec!["real.rs"]);
}

/// Acceptance 2: a refusal carries **one** coordinate. `serde_yaml` spells a reader error's place
/// `at position N` (a byte offset into the block), which `without_positions` does not strip. A
/// DEL in a message — accepted by `parse_json` — reaches it.
#[test]
fn a_refusal_for_a_control_character_carries_one_coordinate_and_quotes_the_offending_line() {
    let body = "# A review\n\n```findings\n[\n{\"file\": \"a.rs\", \"category\": \"c\", \"severity\": \"note\", \"message\": \"del\u{7f}char\"}\n]\n```\n";
    let error = findings::parse(body).expect_err("libyaml refuses a DEL");
    let shown = error.to_string();
    assert!(
        !shown.contains("position"),
        "one coordinate, not the body line plus a block offset: {shown}"
    );
    assert_eq!(error.line, 5, "the DEL is on body line 5: {shown}");
}

/// cli.md's own example block, verbatim.
#[test]
fn the_json_block_cli_md_documents_is_read() {
    let message = only_message(
        r#"[{"file": "src/lib.rs", "line": 12, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the judge reports \"(platform): X\" for the caller's step"}]"#,
    );
    assert_eq!(
        message,
        "the judge reports \"(platform): X\" for the caller's step"
    );
}
