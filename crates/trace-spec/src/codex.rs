//! The Codex rollout adapter: session JSONL in, [`TraceIr`] out.
//!
//! # Why this exists
//!
//! Gap register `:38`. Every behavioural document in this repository is published as
//! harness-neutral, and until now exactly one adapter existed — so *"neutral"* was a claim with no
//! second case behind it. A vocabulary tested against one harness is a vocabulary shaped like that
//! harness, and nobody can tell which from the inside.
//!
//! This is the second case. It reads codex-cli's session rollout and produces the same
//! [`TraceIr`] the Claude adapter does, so the same specification decides both.
//!
//! # The input is the rollout, not stdout
//!
//! `docs/reviews/2026-08-21-codex-harness-research.md` establishes this and it is the one thing
//! that changed the plan: `codex exec --json` prints a thinner stream than the harness records.
//! The richer record is `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<ISO8601>-<uuid7>.jsonl`, written
//! by exec runs too.
//!
//! # What one line is
//!
//! An envelope with a `type` and a `payload`, where the payload carries its own `type`:
//!
//! | envelope | payload | becomes |
//! |---|---|---|
//! | `session_meta` | — | [`EventKind::SessionStart`] |
//! | `response_item` | `function_call`, `custom_tool_call` | [`EventKind::ToolCall`] |
//! | `response_item` | `function_call_output`, `custom_tool_call_output` | [`EventKind::ToolResult`] |
//! | `response_item` | `message` with `role: assistant` | [`EventKind::AssistantText`] |
//! | `response_item` | `reasoning` | [`EventKind::AssistantThinking`] |
//! | `event_msg` | `user_message` | [`EventKind::SyntheticInjection`] |
//! | anything else | | [`EventKind::Opaque`] |
//!
//! # The two rules that are not this adapter's to invent
//!
//! **Correlation** is [`TraceIr::new`]'s, by `call_id`, exactly as it is for the other adapter. Two
//! adapters correlating separately are two places for the pairing to disagree.
//!
//! **Opaque rather than dropped** is invariant 5, and it is the rule that makes a second adapter
//! worth having. Codex emits event families this build has never seen — `token_count`,
//! `exec_command_end`, `patch_apply_end`, `turn_context` — and a reader that discarded them would
//! report *"the tool was never called"* when what happened is that it stopped being able to see
//! tool calls. Every unrecognised line becomes an opaque record carrying its declared types and the
//! digest of the line, so the expectations that depend on it read `unk`.
//!
//! # `operations` is deliberately empty
//!
//! [`ToolCall::operations`] is the neutral vocabulary a specification scopes by, and Codex's
//! rollout does not publish it: a `function_call` names `shell` or `apply_patch` and says nothing
//! about whether that was a read or a write. **Empty means the record did not say**, which makes an
//! operations-scoped row `unk` here rather than false.
//!
//! Resisting the temptation to fill it in is the point. Mapping `apply_patch` to `file.write` would
//! be this adapter guessing at a rendering's question, and the guess would be invisible in the
//! report — a row would read as decided when a human had decided it, in Rust, months earlier. The
//! research file says the same about the format generally: no stability guarantee is documented.

use std::collections::BTreeMap;

use serde_json::Value;
use trace_domain::code::{TraceCode, ValidationErrors};
use trace_domain::digest::digest_of_bytes;
use trace_domain::ir::{
    AdapterRef, EventKind, OpaqueEvent, Recorded, SessionStart, ToolCall, ToolResult, TraceEvent,
    TraceIr, VENDOR_CLOSURE_MARKERS,
};

/// This adapter, and the harness versions it was written against.
///
/// Versioned for the same reason the Claude one is, and with more cause: the research file records
/// that **no stability guarantee is documented** for the rollout format, and that drift between
/// codex-cli versions has already been observed. A verdict that changed because the reader changed
/// must be visible as that rather than as a change in the agent's behaviour.
pub const CODEX_ROLLOUT: AdapterRef = AdapterRef {
    name: "codex/rollout-jsonl",
    written_against: &["0.145.0"],
};

/// Reads a codex rollout into the IR.
///
/// # Errors
///
/// `TRACE-ADAPT-001` for bytes that are not UTF-8 or a line that is not JSON; `TRACE-ADAPT-002` for
/// a rollout with no events at all. The same two the other adapter refuses on, and for the same
/// reasons — which is itself a small piece of evidence that the seam is neutral.
pub fn read_rollout(bytes: &[u8]) -> Result<TraceIr, ValidationErrors> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let mut errors = ValidationErrors::new();
            errors.refuse(
                TraceCode::AdapterMalformedTranscript,
                "rollout",
                format!("the rollout's bytes are not UTF-8: {error}"),
            );
            return Err(errors);
        }
    };
    read_text(bytes, text)
}

/// Reads a rollout that is already text.
///
/// # Errors
///
/// As [`read_rollout`], less the not-UTF-8 case a `&str` cannot be in.
pub fn read_rollout_str(text: &str) -> Result<TraceIr, ValidationErrors> {
    read_text(text.as_bytes(), text)
}

fn read_text(bytes: &[u8], text: &str) -> Result<TraceIr, ValidationErrors> {
    let mut errors = ValidationErrors::new();
    let mut events: Vec<TraceEvent> = Vec::new();

    for (offset, line) in text.lines().enumerate() {
        let source_line = offset + 1;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(value) => read_line(&value, line, source_line, &mut events),
            Err(error) => errors.refuse(
                TraceCode::AdapterMalformedTranscript,
                format!("line[{source_line}]"),
                format!("line {source_line} is not JSON: {error}"),
            ),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    if events.is_empty() {
        errors.refuse(
            TraceCode::AdapterEmptyTranscript,
            "rollout",
            "the rollout holds no events at all: there is nothing to judge",
        );
        return Err(errors);
    }

    // A rollout carries no terminal record at all — this reader lifts none — so it states nothing
    // about its own completeness and a negative expectation over one stays `unk`. That is the
    // honest answer rather than a gap in this reader: a rollout file is an append log a session
    // wrote as it went, and nothing in it says the session finished writing.
    Ok(
        TraceIr::new(digest_of_bytes(bytes), CODEX_ROLLOUT, events, Vec::new())
            .closes_with(VENDOR_CLOSURE_MARKERS, None),
    )
}

/// One rollout line into zero or more IR events, never zero in the end.
fn read_line(value: &Value, raw: &str, source_line: usize, events: &mut Vec<TraceEvent>) {
    let timestamp = value
        .get("timestamp")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let envelope = value.get("type").and_then(Value::as_str);
    let payload = value.get("payload");
    let inner = payload.and_then(|p| p.get("type")).and_then(Value::as_str);

    let kind = match (envelope, inner) {
        (Some("session_meta"), _) => {
            payload.map(|payload| EventKind::SessionStart(Box::new(session_start(payload))))
        }
        (Some("response_item"), Some("function_call" | "custom_tool_call")) => {
            payload.map(|payload| EventKind::ToolCall(Box::new(tool_call(payload))))
        }
        (Some("response_item"), Some("function_call_output" | "custom_tool_call_output")) => {
            payload.map(|payload| EventKind::ToolResult(Box::new(tool_result(payload))))
        }
        (Some("response_item"), Some("message")) => payload.and_then(assistant_text),
        (Some("response_item"), Some("reasoning")) => payload.and_then(|payload| {
            reasoning_text(payload).map(|text| EventKind::AssistantThinking { text })
        }),
        (Some("event_msg"), Some("user_message")) => payload.and_then(|payload| {
            payload.get("message").and_then(Value::as_str).map(|text| {
                EventKind::SyntheticInjection {
                    text: text.to_owned(),
                }
            })
        }),
        _ => None,
    };

    // Never zero. A line that yielded nothing recognisable is preserved rather than dropped, so no
    // line of the file is unrepresented and the expectations resting on it go `unk`.
    let kind = kind.unwrap_or_else(|| {
        EventKind::Opaque(Box::new(OpaqueEvent {
            event_type: envelope.map(ToOwned::to_owned),
            subtype: inner.map(ToOwned::to_owned),
            digest: digest_of_bytes(raw.as_bytes()),
        }))
    });
    events.push(TraceEvent::new(source_line, timestamp, kind));
}

/// The opening record, from `session_meta`.
///
/// Most of [`SessionStart`]'s fields have no counterpart in a rollout and stay `None`. That is the
/// honest reading and it is load-bearing: `env.*` expectations over a field Codex does not publish
/// must read `unk`, never a default that happens to pass.
fn session_start(payload: &Value) -> SessionStart {
    let mut start = SessionStart::default();
    let meta = payload.get("payload").unwrap_or(payload);
    let text = |key: &str| meta.get(key).and_then(Value::as_str).map(ToOwned::to_owned);
    start.model = text("model");
    start.cwd = text("cwd");
    start.harness_version = text("cli_version");
    // `tools` stays `None`, and that is the load-bearing absence: a rollout does not publish the
    // session's tool inventory, so `env.tool_available` must read `unk` here rather than decide
    // against an empty list. An empty `Vec` would answer *no tool was available*, which is a claim
    // nobody made — and the same conflation of *absent* with *empty* is what gap register `:40`
    // found on the other harness.
    start
}

fn tool_call(payload: &Value) -> ToolCall {
    // `function_call` spells its arguments `arguments`; `custom_tool_call` spells them `input`.
    // Both are a JSON string, not an object, so they are recorded under one key rather than
    // re-parsed: a string that fails to parse would otherwise become an empty argument map, which
    // reads as a call that was made with nothing.
    let arguments = payload
        .get("arguments")
        .or_else(|| payload.get("input"))
        .cloned();
    let bytes = arguments
        .as_ref()
        .and_then(Value::as_str)
        .map_or(0, str::len);
    let mut input: BTreeMap<String, Recorded> = BTreeMap::new();
    if let Some(arguments) = arguments {
        input.insert("arguments".to_owned(), arguments);
    }
    ToolCall {
        call_id: payload
            .get("call_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        name: payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        // Empty on purpose — see the module note. The rollout does not say what a call *was*.
        operations: Vec::new(),
        subjects: Vec::new(),
        input,
        input_bytes: bytes,
        joined_argv: None,
        result_event: None,
    }
}

fn tool_result(payload: &Value) -> ToolResult {
    let content = payload.get("output").map(|output| match output.as_str() {
        Some(text) => text.to_owned(),
        None => serde_json::to_string(output).unwrap_or_default(),
    });
    ToolResult {
        call_id: payload
            .get("call_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        // Codex's rollout does not flag a tool result as an error on the record itself, so this is
        // `None` rather than `false`. `false` would be this adapter asserting the call succeeded.
        is_error: None,
        content_bytes: content.as_ref().map_or(0, String::len),
        content,
        fields: BTreeMap::new(),
    }
}

/// An assistant message's text, joined from its content blocks.
///
/// `None` for a `user` message: those arrive on the `event_msg` side, and reading them here as well
/// would double-count every turn.
fn assistant_text(payload: &Value) -> Option<EventKind> {
    if payload.get("role").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    let text = joined_text(payload.get("content")?)?;
    Some(EventKind::AssistantText {
        text,
        request_id: None,
    })
}

/// A reasoning item's text: its `summary` blocks, which are the readable half.
///
/// `encrypted_content` is deliberately not touched. It is opaque by construction, and a reader that
/// put ciphertext into a field a `text.matches` row reads would produce matches nobody can explain.
fn reasoning_text(payload: &Value) -> Option<String> {
    joined_text(payload.get("summary")?)
}

/// Joins a content array's `text` fields, skipping blocks that carry none.
fn joined_text(blocks: &Value) -> Option<String> {
    let blocks = blocks.as_array()?;
    let text: Vec<&str> = blocks
        .iter()
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();
    (!text.is_empty()).then(|| text.join("\n"))
}
