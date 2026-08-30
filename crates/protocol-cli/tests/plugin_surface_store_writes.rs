//! No document under `integrations/` tells its reader to write the planning store by hand.
//!
//! # What this file is for
//!
//! `protocol artifact` is the store's only writer, and every shipped instruction surface says so in
//! prose. Prose is where that rule regressed once already: the planning skill **told** agents to
//! patch bodies directly, and nothing noticed, because nothing in this repository reads a skill's
//! text — the only code that touches one joins its path and asserts the file exists
//! (`crates/protocol-cli/src/drive.rs:7838`). A prohibition guarded by nobody is the same shape as
//! the defect it was written to fix.
//!
//! The cost is asymmetric, which is what makes a tripwire worth its false-negative rate: a surface
//! that regresses to *edit the frontmatter directly* ships green, installs into every adopter, and
//! is found when a store's revision counter stops matching its history.
//!
//! # What is scanned
//!
//! Every `*.md` under `integrations/` — the five `SKILL.md`, the six agent charters, the codex
//! standing instruction, and the reference documents a skill points at. They ship by the same
//! mechanism and carry the same instructions, so a rule that held only for files named `SKILL.md`
//! would be a rule about a filename. The walk takes every markdown file rather than a list, so a
//! sixth skill or a seventh charter is covered on the day it lands and not on the day somebody
//! remembers this file.
//!
//! # How a claim is read
//!
//! Substring matching over normalised text, sentence by sentence: this workspace carries no regular
//! expression engine outside `trace-domain`, and does not need one here. A write **verb** followed
//! within `REACH` bytes by a store **surface**, in one sentence, is an instruction to write the
//! store directly — unless the sentence forbids it (`EXEMPTIONS`, which must appear at or before the
//! verb) or names the sanctioned writer (`SANCTIONED`, anywhere in the sentence). Both exemptions
//! exist because the shipped text is *full* of the phrase this test hunts: *"never edit a store file
//! directly"* is the rule, written in the words of its own violation.
//!
//! Text is normalised before any of that: lowercased, backticks and emphasis removed, hard wraps
//! joined — a phrase split across two lines by the 100-column wrap is still one phrase — and blank
//! lines end a paragraph, so a prohibition never exempts the paragraph after it.
//!
//! # The limits, named
//!
//! This is a tripwire over known shapes, not a proof. Three limits are left open on purpose,
//! because closing any of them costs more false positives than it buys:
//!
//! * A sentence that both forbids and instructs — *"do not ask the operator, edit the frontmatter"*
//!   — is exempted by its own first clause.
//! * A sentence naming `protocol artifact` anywhere is exempted, so an instruction hidden beside a
//!   mention of the CLI passes.
//! * The plural `bodies` is **not** a surface. `SKILL.md` § 4 says *"writing new artifacts and
//!   editing bodies needs no confirmation beyond the request that prompted it"* — a sentence about
//!   who decides, not about which program writes — and it is the one place in the shipped corpus
//!   that reads as an instruction to this scan. Adding `bodies` back turns two shipped skills red;
//!   the singular `the body` still catches *patch the body in place*, which is the shape that
//!   matters.
//!
//! What the guard does catch is the regression that actually happened: a plain instruction, in its
//! own sentence, to edit a store file by hand.

use std::path::{Path, PathBuf};

/// The tree of harness surfaces this repository ships to adopters.
const SURFACE_TREE: &str = "integrations";

/// Surfaces the acceptance names by hand, asserted present so a broken walk cannot pass by
/// scanning nothing.
const REQUIRED_SURFACES: &[&str] = &[
    "integrations/claude-code/skills/planning/SKILL.md",
    "integrations/claude-code/skills/schema-contracts/SKILL.md",
    "integrations/claude-code/skills/wave/SKILL.md",
    "integrations/codex/skills/planning/SKILL.md",
    "integrations/codex/skills/schema-contracts/SKILL.md",
];

/// Verbs that put bytes on disk. Extend this list; the scan reads it and has no other source of
/// verbs. Each entry matches as a whole word, so `edit` does not fire inside `editing` and both are
/// written out.
const WRITE_VERBS: &[&str] = &[
    "edit",
    "edits",
    "edited",
    "editing",
    "patch",
    "patches",
    "patched",
    "patching",
    "modify",
    "modifies",
    "modified",
    "modifying",
    "rewrite",
    "rewrites",
    "rewrote",
    "rewriting",
    "overwrite",
    "overwrites",
    "overwriting",
    "write",
    "writes",
    "wrote",
    "writing",
    "append",
    "appends",
    "appending",
    "insert",
    "inserts",
    "inserting",
    "update",
    "updates",
    "updated",
    "updating",
    "change",
    "changes",
    "changed",
    "changing",
    "set",
    "sets",
    "setting",
    "replace",
    "replaces",
    "replacing",
    "fix",
    "fixes",
    "fixing",
    "sed",
    "tee",
];

/// The parts of a planning artifact a write verb must not reach. Extend this list the same way.
const STORE_SURFACES: &[&str] = &[
    "frontmatter",
    "front matter",
    "status:",
    "status field",
    "revision:",
    "revision field",
    "relations:",
    // Singular only — see the plural's entry in this file's header.
    "body",
    ".engineering/planning",
    "store file",
    "store files",
    "planning file",
    "planning files",
    "artifact file",
    "artifact files",
    "artifact's file",
    "story file",
    "task file",
];

/// Phrases that make a sentence a prohibition of the write rather than an instruction to perform
/// it. Matched at or before the verb, so `never edit` exempts and `edit … never` does not.
const EXEMPTIONS: &[&str] = &[
    "never",
    "not",
    "no",
    "nor",
    "cannot",
    "can't",
    "don't",
    "doesn't",
    "without",
    "rather than",
    "instead of",
    "refuse",
    "refuses",
    "refused",
    "deny",
    "denies",
    "denied",
    "forbid",
    "forbids",
    "forbidden",
    "banned",
    "avoid",
    "stop",
    "before editing",
    "before writing",
    "before changing",
];

/// The sanctioned writer. A sentence that names it is describing the route through the CLI.
const SANCTIONED: &[&str] = &["protocol artifact"];

/// How far a surface may sit behind its verb, in bytes of normalised text.
const REACH: usize = 60;

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// A path as the repository writes it — relative to the root, forward slashes.
fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .expect("the path is inside the repository")
        .to_string_lossy()
        .replace('\\', "/")
}

/// Every markdown document under `integrations/`, repo-relative and sorted.
fn shipped_documents() -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut directories = vec![root().join(SURFACE_TREE)];

    while let Some(directory) = directories.pop() {
        let entries = std::fs::read_dir(&directory)
            .unwrap_or_else(|_| panic!("{} is readable", directory.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                directories.push(path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
                found.push(relative(&path));
            }
        }
    }

    found.sort();
    found
}

/// One block of consecutive non-blank lines, normalised, with the source line of every byte.
struct Paragraph {
    /// The block's text: lowercased, unwrapped, emphasis and backticks removed.
    text: String,
    /// `(offset into `text`, source line number)`, one entry per line, ascending.
    lines: Vec<(usize, usize)>,
}

impl Paragraph {
    /// The source line the byte at `offset` was read from.
    fn line_at(&self, offset: usize) -> usize {
        self.lines
            .iter()
            .rev()
            .find(|(start, _)| *start <= offset)
            .map_or(0, |(_, line)| *line)
    }
}

/// One place where a document tells its reader to write the store directly.
struct Refusal {
    /// The document, repo-relative.
    path: String,
    /// The line the verb was read from.
    line: usize,
    /// The verb that matched.
    verb: &'static str,
    /// The store surface it reached.
    surface: &'static str,
    /// The sentence, as normalised.
    sentence: String,
}

impl Refusal {
    /// The refusal as a reader needs it: the file, the line, and the sentence itself.
    fn render(&self) -> String {
        let mut sentence = self.sentence.clone();
        sentence.truncate(180);
        format!(
            "  - {}:{} instructs `{}` … `{}`:\n      {sentence}",
            self.path, self.line, self.verb, self.surface
        )
    }
}

/// One line, lowercased, with backticks and emphasis removed and runs of whitespace collapsed — so
/// a phrase a hard wrap or a pair of backticks split is still one phrase.
fn normalize(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut pending = false;
    for character in line.chars() {
        if character == '`' || character == '*' {
            continue;
        }
        if character.is_whitespace() {
            pending = true;
            continue;
        }
        if pending && !out.is_empty() {
            out.push(' ');
        }
        pending = false;
        out.push(character.to_ascii_lowercase());
    }
    out
}

/// The document as paragraphs. A blank line ends one, so a prohibition never reaches across it.
fn paragraphs(text: &str) -> Vec<Paragraph> {
    let mut out: Vec<Paragraph> = Vec::new();
    let mut current = Paragraph {
        text: String::new(),
        lines: Vec::new(),
    };

    for (index, raw) in text.lines().enumerate() {
        let normalized = normalize(raw);
        if normalized.is_empty() {
            if !current.text.is_empty() {
                out.push(std::mem::replace(
                    &mut current,
                    Paragraph {
                        text: String::new(),
                        lines: Vec::new(),
                    },
                ));
            }
            continue;
        }
        if !current.text.is_empty() {
            current.text.push(' ');
        }
        current.lines.push((current.text.len(), index + 1));
        current.text.push_str(&normalized);
    }

    if !current.text.is_empty() {
        out.push(current);
    }
    out
}

/// The sentences of a paragraph, as byte ranges. `. `, `? ` and `! ` end one.
fn sentences(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    for index in 0..bytes.len() {
        let terminator = matches!(bytes[index], b'.' | b'?' | b'!');
        if terminator && bytes.get(index + 1) == Some(&b' ') {
            out.push((start, index + 1));
            start = index + 2;
        }
    }
    if start < bytes.len() {
        out.push((start, bytes.len()));
    }
    out
}

/// The first whole-word occurrence of `needle` in `haystack` at or after `from`.
///
/// Whole-word, because `body` lives inside `nobody` and `anybody`, and a scan that did not know
/// that would refuse half the shipped prose. A boundary is required only at an end where the needle
/// itself is alphanumeric, so `.engineering/planning` still matches mid-path.
fn find_word(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let mut at = from;
    while let Some(offset) = haystack.get(at..).and_then(|rest| rest.find(needle)) {
        let start = at + offset;
        let end = start + needle.len();
        let left = !needle.starts_with(|c: char| c.is_alphanumeric())
            || start == 0
            || !bytes[start - 1].is_ascii_alphanumeric();
        let right = !needle.ends_with(|c: char| c.is_alphanumeric())
            || end == bytes.len()
            || !bytes[end].is_ascii_alphanumeric();
        if left && right {
            return Some(start);
        }
        at = start + 1;
    }
    None
}

/// The first direct-write instruction in one sentence, if it holds one: the verb, the store surface
/// it reached, and the offset the verb was read from.
fn instruction(sentence: &str) -> Option<(&'static str, &'static str, usize)> {
    if SANCTIONED
        .iter()
        .any(|marker| find_word(sentence, marker, 0).is_some())
    {
        return None;
    }

    for verb in WRITE_VERBS {
        let mut at = 0;
        while let Some(start) = find_word(sentence, verb, at) {
            let end = start + verb.len();
            at = end;
            if EXEMPTIONS
                .iter()
                .any(|marker| find_word(&sentence[..end], marker, 0).is_some())
            {
                continue;
            }
            let reach = (end + REACH).min(sentence.len());
            for surface in STORE_SURFACES {
                if find_word(sentence, surface, end).is_some_and(|found| found < reach) {
                    return Some((verb, surface, start));
                }
            }
        }
    }
    None
}

/// Every direct-write instruction in one document's text. One refusal per sentence.
fn refusals(path: &str, text: &str) -> Vec<Refusal> {
    let mut found = Vec::new();
    for paragraph in paragraphs(text) {
        for (start, end) in sentences(&paragraph.text) {
            let sentence = &paragraph.text[start..end];
            if let Some((verb, surface, at)) = instruction(sentence) {
                found.push(Refusal {
                    path: path.to_owned(),
                    line: paragraph.line_at(start + at),
                    verb,
                    surface,
                    sentence: sentence.trim().to_owned(),
                });
            }
        }
    }
    found
}

/// A shipped document's text, by repo-relative path.
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// Instructions planted one at a time into a copy of a shipped skill. Each is a shape the store's
/// single-writer rule exists to refuse, and each must be caught on the line it was planted at.
const PLANTED: &[&str] = &[
    "Edit the frontmatter directly.",
    "When the operator asks for a status change, set `status: active` in the file.",
    "Patch the body in place with sed.",
    "Update `.engineering/planning/story/credential-store.md` yourself.",
];

#[test]
fn a_planted_instruction_in_a_copy_of_every_shipped_document_is_caught() {
    // The acceptance the story is written around, and the mutation `AGENTS.md` § *Conventions* asks
    // for — run against a copy in memory, so the committed tree is never touched. The copies are the
    // real documents rather than hand-written fixtures, which makes this test say something the
    // corpus test cannot: each shipped text plus one line is refused, and each shipped text alone is
    // not. Every document is planted into, so a scan that quietly skipped one is caught here rather
    // than by its silence.
    for path in &shipped_documents() {
        let shipped = read(path);
        let clean = refusals(path, &shipped);
        assert!(
            clean.is_empty(),
            "the copy of {path} must start clean or this test proves nothing:\n{}",
            clean
                .iter()
                .map(Refusal::render)
                .collect::<Vec<_>>()
                .join("\n")
        );

        for planted in PLANTED {
            let text = format!("{shipped}\n\n{planted}\n");
            let at = text.lines().count();
            let found = refusals(path, &text);
            assert_eq!(
                found.len(),
                1,
                "planting {planted:?} in {path} must be refused exactly once, and was refused {} \
                 time(s):\n{}",
                found.len(),
                found
                    .iter()
                    .map(Refusal::render)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            assert_eq!(
                found[0].line, at,
                "the refusal must name the planted line of {path}, and named line {}",
                found[0].line
            );
            assert!(
                found[0].render().contains(path.as_str()),
                "the refusal must name the file"
            );
        }
    }
}

#[test]
fn a_prohibition_is_not_read_as_the_instruction_it_forbids() {
    // Why the shipped skills pass: not because they are quiet about hand-editing, but because they
    // forbid it in the words of the thing they forbid. Both strings below are the same sentence —
    // the first is `integrations/claude-code/skills/planning/SKILL.md:44` verbatim, the second is
    // that sentence with its two prohibitions removed. A scan that could not tell them apart would
    // either refuse every skill that states the rule or catch none that breaks it.
    let forbidden = "Never edit the `status:` field in frontmatter, and never write it into a \
                     file with `Edit` or a heredoc.";
    let instructed = "Edit the `status:` field in frontmatter, and write it into a file with \
                      `Edit` or a heredoc.";

    assert!(
        refusals("fixture.md", forbidden).is_empty(),
        "the shipped prohibition must pass:\n{}",
        refusals("fixture.md", forbidden)
            .iter()
            .map(Refusal::render)
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(
        refusals("fixture.md", instructed).len(),
        1,
        "the same sentence without its prohibition must be refused"
    );
}

#[test]
fn a_hard_wrap_does_not_hide_an_instruction() {
    // The shipped documents wrap at 100 columns, so the phrase this test hunts is routinely split
    // across two lines — `SKILL.md:44` ends mid-phrase at `field in`. A line-at-a-time scan would
    // read the corpus and find nothing, and would go on finding nothing for as long as the wrap fell
    // where it does.
    let wrapped = "Set the `status:` field in\nfrontmatter yourself when the operator asks.\n";
    let found = refusals("fixture.md", wrapped);

    assert_eq!(
        found.len(),
        1,
        "an instruction split by a wrap is still an instruction"
    );
    assert_eq!(
        found[0].line, 1,
        "the refusal names the line the verb was read from"
    );

    // And the line is the verb's, not the sentence's first: a reader given the top of a paragraph
    // has to find the offending phrase themselves, which is the hour this message exists to save.
    let late = "The operator has asked for this, so the right answer is to\nedit the frontmatter \
                directly.\n";
    let found = refusals("fixture.md", late);

    assert_eq!(found.len(), 1, "the instruction is on the second line");
    assert_eq!(found[0].line, 2, "the refusal names the verb's own line");
}

#[test]
fn a_prohibition_does_not_exempt_the_paragraph_after_it() {
    // What a blank line is for here. The shipped rule and the regression that breaks it would sit
    // in the same document, paragraphs apart, and a scan that let the rule's `never` reach forward
    // would pass the very file it was written to refuse.
    let text = "Never edit a store file directly.\n\nEdit the frontmatter directly.\n";
    let found = refusals("fixture.md", text);

    assert_eq!(found.len(), 1, "the second paragraph is not exempt");
    assert_eq!(found[0].line, 3, "the refusal names the second paragraph");
}

#[test]
fn no_shipped_surface_instructs_a_direct_store_write() {
    // The corpus. Every markdown document under `integrations/`, read as text.
    let mut refused: Vec<String> = Vec::new();
    let documents = shipped_documents();

    for path in &documents {
        for refusal in refusals(path, &read(path)) {
            refused.push(refusal.render());
        }
    }

    assert!(
        refused.is_empty(),
        "{} instruction(s) to write the planning store by hand, in {} document(s) under \
         {SURFACE_TREE}/. The store has one writer, `protocol artifact`; say that instead:\n{}",
        refused.len(),
        documents.len(),
        refused.join("\n")
    );
}

#[test]
fn the_scan_reaches_every_surface_that_ships() {
    // Totality, in the shape `workflow_coverage.rs` uses: a walk that has stopped finding files
    // passes every other test in this file silently.
    let documents = shipped_documents();
    let missing: Vec<&&str> = REQUIRED_SURFACES
        .iter()
        .filter(|required| !documents.iter().any(|path| path == *required))
        .collect();

    assert!(
        missing.is_empty(),
        "the scan did not reach {missing:?}; it read {documents:?}"
    );
    assert!(
        documents.len() >= REQUIRED_SURFACES.len(),
        "the walk over {SURFACE_TREE}/ found {} document(s)",
        documents.len()
    );
}

#[test]
fn the_pattern_set_is_not_empty() {
    // What stops this file being satisfied by deleting its data. Every other test here passes
    // trivially against an empty verb list or an empty surface list.
    assert!(
        !WRITE_VERBS.is_empty(),
        "a scan with no write verbs refuses nothing"
    );
    assert!(
        !STORE_SURFACES.is_empty(),
        "a scan with no store surfaces refuses nothing"
    );
    assert!(
        !SANCTIONED.is_empty(),
        "with no sanctioned writer every mention of the CLI is a refusal"
    );
}
