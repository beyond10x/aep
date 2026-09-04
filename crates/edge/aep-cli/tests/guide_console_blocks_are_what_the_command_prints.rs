//! The console blocks in *Check a transcript* are the output of the commands above them.
//!
//! The guide is written as a recipe a reader pastes: it opens by setting `B=target/debug/aep` and
//! every block after that is a command and, immediately below it, a fenced block presented as what
//! that command printed. `story:plugin-names-follow-agentplugins` added a paragraph to this page
//! saying so in as many words — *"this is the output of checking a transcript recorded before the
//! rename, **quoted as it was printed**"* — and nothing in the gate compares the two. `docs-check`
//! asks only whether every verb has a row in the CLI reference (`app.rs`'s `cli_reference`); the
//! website step asks whether Docusaurus can resolve the links. A console block that stopped being
//! the command's output builds green for ever, which is the same failure the CLI-reference check
//! was written for one page over.
//!
//! Scoped to `trace check` and `trace inspect`, which are what this page is about and which read
//! committed fixtures and write nothing. `trace evidence` is deliberately out: its block is a file
//! it writes into the checkout, and a test that ran it would leave one behind.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The page under test, relative to the repository root.
const GUIDE: &str = "website/docs/guides/check-a-transcript.md";

/// The fence languages this page uses for *a command*.
const COMMAND_LANGUAGES: &[&str] = &["bash"];

/// The fence languages this page uses for *what a command printed*.
const OUTPUT_LANGUAGES: &[&str] = &["text", "console", "json"];

/// One fenced block: its language tag, its body, and the 1-based line its fence opened on.
struct Fence {
    /// The info string after the opening backticks.
    language: String,
    /// The lines between the fences.
    body: String,
    /// Where the opening fence is, so a failure cites the page.
    line: usize,
}

/// Every fenced block on a page, in order.
fn fences(text: &str) -> Vec<Fence> {
    let mut found = Vec::new();
    let mut lines = text.lines().enumerate();
    while let Some((index, line)) = lines.next() {
        let Some(language) = line.trim_end().strip_prefix("```") else {
            continue;
        };
        let language = language.trim().to_owned();
        let mut body = String::new();
        for (_, inner) in lines.by_ref() {
            if inner.trim_end() == "```" {
                break;
            }
            body.push_str(inner);
            body.push('\n');
        }
        found.push(Fence {
            language,
            body,
            line: index + 1,
        });
    }
    found
}

/// The verb an invocation reaches, with a leading area word removed.
///
/// The page spells commands by their area — `observe trace check` — and the flat spelling
/// `trace check` reaches the same leaf. Both have to select the same block, and the day one of
/// them stopped selecting it, this test would have gone on passing with one fewer block compared:
/// the `compared >= 3` floor below is the only thing that would have noticed, and only if enough
/// of them moved at once.
fn leaf<'a>(arguments: &'a [&'a str]) -> &'a [&'a str] {
    match arguments {
        ["govern" | "plan" | "drive" | "observe", rest @ ..] => rest,
        _ => arguments,
    }
}

/// Whether a line of a command block is a shell assignment rather than a command.
fn is_assignment(line: &str) -> bool {
    let Some((name, _)) = line.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && !name.contains(char::is_whitespace)
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// The logical commands in a command block, with `\` continuations joined.
fn logical_commands(body: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut current = String::new();
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let (text, continues) = match line.strip_suffix('\\') {
            Some(head) => (head.trim(), true),
            None => (line, false),
        };
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(text);
        if !continues {
            commands.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        commands.push(current);
    }
    commands
        .into_iter()
        .filter(|command| !is_assignment(command))
        .collect()
}

/// What a command's output is piped through, when this test can reproduce it.
enum Filter {
    /// The whole of it.
    Whole,
    /// `head -n`.
    Head(usize),
    /// `tail -n`.
    Tail(usize),
}

/// The filter `pipeline` names, or `None` for one this test does not reproduce.
fn filter_of(pipeline: &str) -> Option<Filter> {
    let words: Vec<&str> = pipeline.split_whitespace().collect();
    match words.as_slice() {
        ["head", count] => count.trim_start_matches('-').parse().ok().map(Filter::Head),
        ["tail", count] => count.trim_start_matches('-').parse().ok().map(Filter::Tail),
        _ => None,
    }
}

/// `text`, with `filter` applied, as the page's block would have it.
fn apply(filter: &Filter, text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let kept: Vec<&str> = match filter {
        Filter::Whole => lines,
        Filter::Head(count) => lines.into_iter().take(*count).collect(),
        Filter::Tail(count) => {
            let skip = lines.len().saturating_sub(*count);
            lines.into_iter().skip(skip).collect()
        }
    };
    kept.join("\n")
}

/// A block's text with trailing blank lines and trailing spaces removed, for comparison.
fn normalized(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| line.trim_end().to_owned())
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// The first line at which two renderings differ, and both of its sides.
fn first_difference(claimed: &[String], printed: &[String]) -> String {
    for (index, (left, right)) in claimed.iter().zip(printed.iter()).enumerate() {
        if left != right {
            return format!(
                "line {} of the block:\n      page:    {left}\n      command: {right}",
                index + 1
            );
        }
    }
    format!(
        "the block has {} line(s) and the command printed {}",
        claimed.len(),
        printed.len()
    )
}

/// Every `trace check` and `trace inspect` block on the page prints what the page says it prints.
///
/// A page that says *quoted as it was printed* is a claim this repository can answer, and this is
/// that claim. Non-vacuous by construction: the count of blocks actually run is asserted, because a
/// parser that matched nothing would pass silently and certify a page nobody checked.
#[test]
fn every_quoted_trace_report_on_the_transcript_guide_is_what_the_command_prints() {
    let root = repo_root();
    let binary = env!("CARGO_BIN_EXE_aep");
    let page = std::fs::read_to_string(root.join(GUIDE)).expect("reading the transcript guide");
    let blocks = fences(&page);

    let mut compared = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    let mut findings: Vec<String> = Vec::new();

    for pair in blocks.windows(2) {
        let (command_block, output_block) = (&pair[0], &pair[1]);
        if !COMMAND_LANGUAGES.contains(&command_block.language.as_str())
            || !OUTPUT_LANGUAGES.contains(&output_block.language.as_str())
        {
            continue;
        }
        let commands = logical_commands(&command_block.body);
        let [command] = commands.as_slice() else {
            continue;
        };
        let (invocation, pipeline) = match command.split_once('|') {
            Some((left, right)) => (left.trim(), Some(right.trim())),
            None => (command.as_str(), None),
        };
        let mut words = invocation.split_whitespace();
        let Some(program) = words.next() else {
            continue;
        };
        if !matches!(program, "$B" | "target/debug/aep" | "target/debug/protocol") {
            continue;
        }
        let arguments: Vec<&str> = words.collect();
        if !matches!(leaf(&arguments), ["trace", verb, ..] if *verb == "check" || *verb == "inspect")
        {
            continue;
        }
        let filter = match pipeline {
            None => Filter::Whole,
            Some(pipeline) => {
                let Some(filter) = filter_of(pipeline) else {
                    skipped.push(format!(
                        "{GUIDE}:{} — `| {pipeline}` is not one this test reproduces",
                        command_block.line
                    ));
                    continue;
                };
                filter
            }
        };

        let output = Command::new(binary)
            .args(&arguments)
            .current_dir(&root)
            .output()
            .expect("the CLI starts");
        let printed = apply(&filter, &String::from_utf8_lossy(&output.stdout));
        let claimed = normalized(&output_block.body);
        let printed = normalized(&printed);
        compared += 1;
        if claimed != printed {
            findings.push(format!(
                "  {GUIDE}:{} — `aep {}`{}\n    {}",
                output_block.line,
                arguments.join(" "),
                pipeline.map_or(String::new(), |pipeline| format!(" | {pipeline}")),
                first_difference(&claimed, &printed)
            ));
        }
    }

    // Four, which is how many the page currently pairs a command with an output for. A floor and
    // not an equality so the page may grow, but the floor is the real number rather than a round
    // one below it: the failure this catches is a *selector* that stops matching — a spelling that
    // moved, a fence language that changed — and a floor with slack in it absorbs exactly that.
    assert!(
        compared >= 4,
        "only {compared} console block(s) were run, so this test is asserting less than it did; \
         skipped: {skipped:?}"
    );
    assert!(
        findings.is_empty(),
        "{} console block(s) on {GUIDE} are not what the command above them prints. The page's own \
         paragraph says these blocks are quoted as they were printed, and \
         `story:plugin-names-follow-agentplugins` added that sentence:\n{}",
        findings.len(),
        findings.join("\n")
    );
}
