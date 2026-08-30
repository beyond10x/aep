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
//! standing instruction, the two references and the three READMEs. They ship by the same mechanism
//! and carry the same instructions, so a rule that held only for files named `SKILL.md` would be a
//! rule about a filename. The walk takes every markdown file rather than a list, so a sixth skill or
//! a seventh charter is scanned on the day it lands.
//!
//! `REQUIRED_SURFACES` then pins **every one of them by name**, in both directions: a walk that has
//! stopped finding the agent charters fails here instead of passing quietly over a third of the
//! corpus, and a document added under `integrations/` fails here until it is written down. Pinning
//! only the five `SKILL.md` — which this file did until an adversary measured it — left twelve
//! documents covered by nothing but the walk, so deleting `integrations/claude-code/agents/` kept
//! the whole suite green.
//!
//! # How a claim is read
//!
//! Substring matching over normalised text, sentence by sentence: this workspace carries no regular
//! expression engine outside `trace-domain`, and does not need one here. A write **verb** followed
//! within `REACH` bytes by a store **surface**, in one sentence, is an instruction to write the
//! store directly — unless one of two exemptions holds.
//!
//! **A prohibition exempts its own clause, not the sentence.** `EXEMPTIONS` are matched inside the
//! verb's clause — the span between the punctuation that brackets it (`CLAUSE_BREAKS`) — and
//! `TRAILING_EXEMPTIONS` catch the passive, where the prohibition follows its subject (*`Edit`,
//! `Write` and `NotebookEdit` **are denied** under `.engineering/planning/`*). A sentence-wide
//! exemption is a sentence-wide hole: *do not ask the operator, edit the frontmatter* was excused by
//! its own first clause for as long as the window was the whole sentence.
//!
//! **Naming the sanctioned writer is not a licence to route around it.** A sentence naming
//! `SANCTIONED` is describing the road through the CLI — *write the body through `protocol artifact
//! body`* — and the shipped corpus needs that exemption in sixteen places. But the defect this story
//! was written about *was* a skill routing around the writer, and the sentence that does that names
//! it: *Skip `protocol artifact move` and write `status: active` into the file.* So a `ROUTING`
//! marker anywhere in the sentence — *yourself*, *by hand*, *skip*, *unavailable*, *your editor* —
//! voids the exemption, and the sentence is read on its verbs alone. In the same spirit an exemption
//! whose object **is** the CLI (*instead of `protocol artifact body`, edit the file*) is not an
//! exemption at all; and inside a sentence that has named the store, `NAMED_STORE_SURFACES` reads a
//! bare *the file* as a store file, which it is not worth reading as anywhere else.
//!
//! Text is normalised before any of that: lowercased, backticks and emphasis removed, hard wraps
//! joined — a phrase split across two lines by the 100-column wrap is still one phrase. A blank line
//! ends a block, and so does a heading, a list item, a table row, a fence and a blockquote marker.
//! Markdown table rows carry no sentence-ending punctuation, so a scan that joined them read a whole
//! ownership table as one sentence and let the `never edited` in its first row exempt every row
//! under it — which is where a regression to this rule would actually be written.
//!
//! One thing survives the lowercasing: a **capitalised** word opening a wrapped line, after a line
//! that did not end in a terminator, opens a sentence (`opens_a_sentence`). It is the difference
//! between a wrap and a new statement, and without it the tail of whatever paragraph a regression
//! lands in reaches forward and excuses it — 141 of 2278 line positions of this corpus swallowed a
//! planted instruction until this was here.
//!
//! # The limits, named
//!
//! This is a tripwire over known shapes, not a proof. What is left open is left open on purpose:
//!
//! * **Only `*.md` is read.** Three shipped surfaces carry model-visible prose and are not markdown:
//!   `integrations/codex/.codex-plugin/plugin.json`, whose `interface.longDescription` and
//!   `interface.defaultPrompt` are shown to a reader choosing the plugin;
//!   `integrations/claude-code/.claude-plugin/plugin.json`, which carries a `description` and
//!   neither of those two keys; and the thirty-five comment lines of
//!   `integrations/codex/eval/check-instruction-surface.sh`. Closing this needs a JSON reader and a
//!   shell reader in a test whose whole method is *read the markdown*; it is a gap, not a
//!   decision — measured 2026-08-30, and the keys were checked rather than assumed.
//! * **The plural `bodies` is not a surface.** `SKILL.md` § 4 says *"writing new artifacts and
//!   editing bodies needs no confirmation beyond the request that prompted it"* — a sentence about
//!   who decides, not about which program writes. The singular `the body` still catches *patch the
//!   body in place*, which is the shape that matters.
//! * **`ROUTING` is a list of phrases, and a regression that invents a new one passes.** It is the
//!   one place in this file where the check is a blacklist rather than a shape, and it is here
//!   because *route around the CLI* is a matter of intent that punctuation cannot see. Extend it.
//! * **Paraphrase is invisible.** *Open the document and correct the top block* names no surface in
//!   `STORE_SURFACES` and passes.
//!
//! What the guard does catch is the regression that actually happened: a plain instruction, in its
//! own sentence, table row or list item, to edit a store file by hand.

use std::path::{Path, PathBuf};

/// The tree of harness surfaces this repository ships to adopters.
const SURFACE_TREE: &str = "integrations";

/// Every document that ships under `integrations/`, pinned by name.
///
/// Held to the walk in **both** directions by `the_scan_reaches_every_surface_that_ships`: nothing
/// here may go missing, and nothing may appear under `integrations/` without being added. The second
/// half is the one that costs a line when a skill lands, and it is the half that makes the first
/// half mean anything.
const REQUIRED_SURFACES: &[&str] = &[
    "integrations/claude-code/README.md",
    "integrations/claude-code/agents/adversary.md",
    "integrations/claude-code/agents/decomposer.md",
    "integrations/claude-code/agents/implementor.md",
    "integrations/claude-code/agents/plan-reviewer.md",
    "integrations/claude-code/agents/reverse-engineer.md",
    "integrations/claude-code/agents/story-scoper.md",
    "integrations/claude-code/skills/planning/SKILL.md",
    "integrations/claude-code/skills/planning/references/store-conventions.md",
    "integrations/claude-code/skills/schema-contracts/SKILL.md",
    "integrations/claude-code/skills/wave/SKILL.md",
    "integrations/claude-code/skills/wave/references/branch-and-merge.md",
    "integrations/codex/AGENTS.planning.md",
    "integrations/codex/README.md",
    "integrations/codex/eval/README.md",
    "integrations/codex/skills/planning/SKILL.md",
    "integrations/codex/skills/schema-contracts/SKILL.md",
];

/// Verbs that put bytes on disk. Extend this list; the scan reads it and has no other source of
/// verbs. Each entry matches as a whole word, so `edit` does not fire inside `editing` and both are
/// written out. `hand-edit` needs no entry of its own: the hyphen is a word boundary, so `edit`
/// already finds it.
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
    "overwrote",
    "overwriting",
    "write",
    "writes",
    "wrote",
    "written",
    "writing",
    "append",
    "appends",
    "appended",
    "appending",
    "insert",
    "inserts",
    "inserted",
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
    "replaced",
    "replacing",
    "fix",
    "fixes",
    "fixed",
    "fixing",
    "amend",
    "amends",
    "amended",
    "amending",
    "adjust",
    "adjusts",
    "adjusted",
    "adjusting",
    "correct",
    "corrects",
    "corrected",
    "correcting",
    "tweak",
    "tweaks",
    "tweaked",
    "tweaking",
    "bump",
    "bumps",
    "bumped",
    "bumping",
    "add",
    "adds",
    "added",
    "adding",
    "delete",
    "deletes",
    "deleted",
    "deleting",
    "remove",
    "removes",
    "removed",
    "removing",
    "save",
    "saves",
    "saved",
    "saving",
    "alter",
    "alters",
    "altered",
    "altering",
    "revise",
    "revises",
    "revised",
    "revising",
    "drop",
    "drops",
    "dropped",
    "dropping",
    "paste",
    "pastes",
    "pasted",
    "pasting",
    "put",
    "puts",
    "putting",
    "sed",
    "tee",
];

/// The parts of a planning artifact a write verb must not reach. Extend this list the same way.
///
/// `the artifact` carries its article on purpose. Bare `artifact` is a word this corpus uses about
/// artifacts in general — *writing new artifacts*, and `protocol artifact` itself — while *the*
/// artifact is one file on disk. The article is what tells those apart, and the whole-word match
/// makes `the artifact` find `the artifact's` too.
const STORE_SURFACES: &[&str] = &[
    "frontmatter",
    "front matter",
    "yaml header",
    "yaml block",
    "yaml at the top",
    "status:",
    "status field",
    "status line",
    "revision:",
    "revision field",
    "relations:",
    "relations field",
    // Singular only — see the plural's entry in this file's header.
    "body",
    ".engineering/planning",
    "store file",
    "store files",
    "planning file",
    "planning files",
    "planning-store file",
    "planning-store files",
    "planning document",
    "planning documents",
    "artifact file",
    "artifact files",
    "artifact's file",
    "story file",
    "task file",
    "the artifact",
];

/// Nouns that name a store file only once the sentence has said which store it is talking about.
///
/// `the file` is the commonest noun in a corpus about tests, worktrees and branches, and reading it
/// as a store surface everywhere costs four false positives in the shipped text — *a hand-edited
/// status is indistinguishable in the file from a legal one* is the rule, not a breach of it. In a
/// sentence that names `SANCTIONED`, though, the file under discussion is a planning artifact and
/// nothing else, and that is precisely the sentence a regression writes: *instead of `protocol
/// artifact body`, edit the file*.
const NAMED_STORE_SURFACES: &[&str] = &[
    "the file",
    "a file",
    "this file",
    "that file",
    "the document",
];

/// Phrases that make a clause a prohibition of the write rather than an instruction to perform it.
///
/// Matched inside the verb's own clause. A prohibition one clause away is not a prohibition of this
/// one: *do not ask the operator, edit the frontmatter* forbids asking, and instructs the write.
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

/// Prohibitions that follow their subject, so they sit **after** the verb in the same clause.
///
/// English puts the negation last in the passive: *`Edit`, `Write` and `NotebookEdit` are denied
/// under `.engineering/planning/`* is a statement of the rule with three write verbs at the front of
/// it. Deliberately a short, explicit list rather than "any exemption later in the clause", which
/// would let a paragraph's later prose excuse an instruction earlier in it.
const TRAILING_EXEMPTIONS: &[&str] = &[
    "are denied",
    "is denied",
    "are refused",
    "is refused",
    "are forbidden",
    "is forbidden",
    "are banned",
    "is banned",
    "are never",
    "is never",
    "are not",
    "is not",
    "must not",
    "may not",
];

/// The sanctioned writer. A sentence that names it is describing the route through the CLI.
const SANCTIONED: &[&str] = &["protocol artifact"];

/// Phrases that mean *go around the sanctioned writer*, and so void naming it as an exemption.
///
/// The regression this file exists for names the CLI: it names it in order to say *not that way*.
/// Every entry is either an adverbial of manual action (*yourself*, *by hand*) or a reason to skip
/// the writer (*unavailable*, *is down*, *skip*). This is the one blacklist in the file; a phrase it
/// does not know passes, so add to it.
const ROUTING: &[&str] = &[
    "yourself",
    "by hand",
    "by-hand",
    "hand-edit",
    "hand-edits",
    "hand edit",
    "hand-written",
    "hand-edited",
    "manually",
    "by yourself",
    "directly",
    "in place",
    "your editor",
    "skip",
    "skips",
    "skipping",
    "unavailable",
    "is down",
    "not available",
    "too slow",
    "slow route",
    "bypass",
    "bypasses",
    "shortcut",
    "faster to",
];

/// How far a surface may sit behind its verb, in bytes of normalised text.
///
/// One hundred, because one ordinary qualifying clause is longer than sixty: *Edit, when the
/// operator has asked for it and the CLI is unavailable, the frontmatter* carries its object
/// sixty-nine bytes behind its verb, which is what English does whenever the condition is the
/// interesting part. Pinned at both ends by `reach_is_load_bearing_at_both_ends`, because a constant
/// no test moves is a constant nothing is holding.
const REACH: usize = 100;

/// Punctuation that brackets a clause.
///
/// A comma is deliberately **not** here: *`Edit`, `Write` and `NotebookEdit` are denied* is one
/// clause with a list in it, and cutting at every comma would leave `edit` alone in a clause of its
/// own with its prohibition on the far side of the punctuation.
const CLAUSE_BREAKS: &[char] = &[';', ':', '|', '(', ')', '\u{2014}', '\u{2013}'];

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

/// One block of consecutive lines that markdown reads as one run of prose, normalised, with the
/// source line of every byte.
struct Paragraph {
    /// The block's text: lowercased, unwrapped, emphasis and backticks removed.
    text: String,
    /// `(offset into `text`, source line number)`, one entry per line, ascending.
    lines: Vec<(usize, usize)>,
}

impl Paragraph {
    /// An empty block, ready to collect lines.
    fn new() -> Self {
        Self {
            text: String::new(),
            lines: Vec::new(),
        }
    }

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
    /// How much of the offending sentence a message carries.
    const SHOWN: usize = 180;

    /// The refusal as a reader needs it: the file, the line, and the sentence itself.
    ///
    /// This is the only path that runs *after* the guard has caught something, so a panic here
    /// destroys the message the whole file exists to produce. It cut with `String::truncate`, which
    /// asserts a character boundary, over a corpus written with an em dash about once a sentence.
    fn render(&self) -> String {
        let sentence = clip(&self.sentence, Self::SHOWN);
        format!(
            "  - {}:{} instructs `{}` … `{}`:\n      {sentence}",
            self.path, self.line, self.verb, self.surface
        )
    }
}

/// At most `limit` bytes of `text`, cut at a character boundary and marked when it was cut.
fn clip(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let cut = (0..=limit)
        .rev()
        .find(|at| text.is_char_boundary(*at))
        .unwrap_or(0);
    format!("{}…", &text[..cut])
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

/// Whether a raw line opens a markdown block: a heading, a list item, a table row, a fence, a
/// blockquote or a rule.
///
/// Read from the **raw** line, before `normalize` strips the `*` an emphasis and a bullet share.
fn starts_block(raw: &str) -> bool {
    let line = raw.trim_start();
    let bullet = matches!(line.as_bytes().first(), Some(b'-' | b'*' | b'+'))
        && matches!(line.as_bytes().get(1), Some(b' ' | b'\t') | None);
    let digits = line
        .as_bytes()
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    let ordered = digits > 0
        && matches!(line.as_bytes().get(digits), Some(b'.' | b')'))
        && matches!(line.as_bytes().get(digits + 1), Some(b' ' | b'\t') | None);
    line.starts_with('#')
        || line.starts_with('|')
        || line.starts_with("```")
        || line.starts_with('>')
        || bullet
        || ordered
}

/// Whether a wrapped line opens a sentence of its own: it is capitalised, and the line above it did
/// not end in a terminator.
///
/// English capitalises mid-sentence only for proper nouns, so this is the one signal `normalize`
/// throws away that a scan of hard-wrapped prose cannot do without. Without it the tail of whatever
/// line a regression lands under reaches forward into it, and a `no` three words up excuses the
/// instruction: an adversary planted one line at all 2278 positions of this corpus and the leak
/// swallowed 141 of them. Splitting here costs thirteen proper nouns across the corpus — *the\nCLI
/// owns*, *since\nFebruary 2024* — cut from their own sentence, and none of the thirteen changes an
/// answer.
fn opens_a_sentence(joined: &str, raw: &str) -> bool {
    if joined.ends_with(['.', '?', '!']) {
        return false;
    }
    raw.trim_start()
        .trim_start_matches(['*', '`', '_', '"', '(', '['])
        .starts_with(|character: char| character.is_ascii_uppercase())
}

/// The document as blocks. A blank line ends one, and so does the start of the next markdown block,
/// so a prohibition reaches neither the paragraph nor the table row after it.
fn paragraphs(text: &str) -> Vec<Paragraph> {
    let mut out: Vec<Paragraph> = Vec::new();
    let mut current = Paragraph::new();

    for (index, raw) in text.lines().enumerate() {
        let normalized = normalize(raw);
        if (normalized.is_empty() || starts_block(raw)) && !current.text.is_empty() {
            out.push(std::mem::replace(&mut current, Paragraph::new()));
        }
        if normalized.is_empty() {
            continue;
        }
        if !current.text.is_empty() {
            if opens_a_sentence(&current.text, raw) {
                current.text.push('.');
            }
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

/// The sentences of a block, as byte ranges. `. `, `? ` and `! ` end one.
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

/// The start of the clause holding the byte at `offset`: just past the punctuation that opened it.
fn clause_start(sentence: &str, offset: usize) -> usize {
    sentence[..offset]
        .char_indices()
        .rev()
        .find(|(_, character)| CLAUSE_BREAKS.contains(character))
        .map_or(0, |(at, character)| at + character.len_utf8())
}

/// The end of the clause holding the byte at `offset`: the punctuation that closes it, or the end.
fn clause_end(sentence: &str, offset: usize) -> usize {
    sentence[offset..]
        .char_indices()
        .find(|(_, character)| CLAUSE_BREAKS.contains(character))
        .map_or(sentence.len(), |(at, _)| offset + at)
}

/// Whether the sanctioned writer is the object of the phrase ending at `after` — *instead of
/// `protocol artifact body`* — which makes that phrase a way around the writer, not a prohibition.
fn governs_sanctioned(sentence: &str, after: usize) -> bool {
    SANCTIONED.iter().any(|marker| {
        find_word(sentence, marker, after)
            .is_some_and(|found| sentence[after..found].split_whitespace().count() == 0)
    })
}

/// Whether the clause holding the verb at `start..end` forbids the write rather than instructing it.
fn exempted(sentence: &str, start: usize, end: usize) -> bool {
    let opens = clause_start(sentence, start);
    let clause = &sentence[opens..end];
    for marker in EXEMPTIONS {
        let mut at = 0;
        while let Some(found) = find_word(clause, marker, at) {
            at = found + 1;
            if !governs_sanctioned(sentence, opens + found + marker.len()) {
                return true;
            }
        }
    }

    let closes = clause_end(sentence, end);
    let rest = &sentence[end..closes];
    TRAILING_EXEMPTIONS
        .iter()
        .any(|marker| find_word(rest, marker, 0).is_some())
}

/// Every write verb in the sentence, earliest first, longest first at one position.
fn verb_occurrences(sentence: &str) -> Vec<(usize, &'static str)> {
    let mut found: Vec<(usize, &'static str)> = Vec::new();
    for verb in WRITE_VERBS {
        let mut at = 0;
        while let Some(start) = find_word(sentence, verb, at) {
            found.push((start, verb));
            at = start + 1;
        }
    }
    found.sort_by_key(|(start, verb)| (*start, std::cmp::Reverse(verb.len())));
    found
}

/// Whether the sentence names the store's one writer.
fn names_the_sanctioned_writer(sentence: &str) -> bool {
    SANCTIONED
        .iter()
        .any(|marker| find_word(sentence, marker, 0).is_some())
}

/// Whether the sentence says how to get around the writer it just named.
fn routes_around_the_sanctioned_writer(sentence: &str) -> bool {
    ROUTING
        .iter()
        .any(|marker| find_word(sentence, marker, 0).is_some())
        || EXEMPTIONS.iter().any(|marker| {
            let mut at = 0;
            while let Some(found) = find_word(sentence, marker, at) {
                at = found + 1;
                if governs_sanctioned(sentence, found + marker.len()) {
                    return true;
                }
            }
            false
        })
}

/// The first direct-write instruction in one sentence, if it holds one: the verb, the store surface
/// it reached, and the offset the verb was read from.
fn instruction(sentence: &str) -> Option<(&'static str, &'static str, usize)> {
    let named = names_the_sanctioned_writer(sentence);
    if named && !routes_around_the_sanctioned_writer(sentence) {
        return None;
    }
    let surfaces = STORE_SURFACES.iter().chain(if named {
        NAMED_STORE_SURFACES.iter()
    } else {
        [].iter()
    });

    for (start, verb) in verb_occurrences(sentence) {
        let end = start + verb.len();
        if exempted(sentence, start, end) {
            continue;
        }
        let reach = (end + REACH).min(sentence.len());
        for surface in surfaces.clone() {
            if find_word(sentence, surface, end).is_some_and(|found| found < reach) {
                return Some((verb, surface, start));
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

/// The refusals of a document, rendered one per line.
fn rendered(found: &[Refusal]) -> String {
    found
        .iter()
        .map(Refusal::render)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Instructions planted one at a time into a copy of a shipped skill. Each is a shape the store's
/// single-writer rule exists to refuse, and each must be caught on the line it was planted at.
const PLANTED: &[&str] = &[
    "Edit the frontmatter directly.",
    "When the operator asks for a status change, set `status: active` in the file.",
    "Patch the body in place with sed.",
    "Update `.engineering/planning/story/credential-store.md` yourself.",
    "Amend the frontmatter yourself.",
    "Bump the `revision:` field by hand.",
    "Skip `protocol artifact move` and write `status: active` into the file.",
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
            rendered(&clean)
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
                rendered(&found)
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
fn a_planted_instruction_survives_every_position_in_every_shipped_document() {
    // Where the previous version of this file was weakest, measured rather than argued: planting at
    // end-of-file is the one position where a fresh block and a fresh sentence are both guaranteed.
    // An adversary planted the same line at all 2295 line positions of the corpus and 409 of them —
    // 17.8% — escaped, because a prohibition earlier in whatever paragraph, table or list the line
    // landed in reached forward and excused it. So plant at every position, not the easy one.
    let planted = "Update `status:` in the file yourself.";
    let mut escaped: Vec<String> = Vec::new();
    let mut positions = 0_usize;

    for path in &shipped_documents() {
        let shipped = read(path);
        let lines: Vec<&str> = shipped.lines().collect();
        for at in 0..=lines.len() {
            positions += 1;
            let mut text: Vec<&str> = lines.clone();
            text.insert(at, planted);
            let found = refusals(path, &text.join("\n"));
            if !found.iter().any(|refusal| refusal.line == at + 1) {
                escaped.push(format!("  - {path}:{}", at + 1));
            }
        }
    }

    assert!(
        escaped.is_empty(),
        "{} of {positions} positions swallow a planted instruction:\n{}",
        escaped.len(),
        escaped.join("\n")
    );
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
        rendered(&refusals("fixture.md", forbidden))
    );
    assert_eq!(
        refusals("fixture.md", instructed).len(),
        1,
        "the same sentence without its prohibition must be refused"
    );
}

#[test]
fn a_prohibition_does_not_exempt_the_clause_after_it() {
    // The hole a sentence-wide exemption is. Both halves of each pair below are one sentence; the
    // first clause forbids something, and the second instructs the write. Read whole, the sentence
    // is excused by its own opening — which is how *do not ask the operator, edit the frontmatter*
    // shipped green.
    let cases = [
        "Do not ask the operator: edit the frontmatter.",
        "You cannot reach the CLI; set `status: active` in the file.",
        "The operator has not asked — patch the body in place.",
    ];

    for case in cases {
        let found = refusals("fixture.md", case);
        assert_eq!(
            found.len(),
            1,
            "a prohibition of something else does not licence this write: {case:?}\n{}",
            rendered(&found)
        );
    }

    // And the same clause still exempts itself, which is the whole shipped corpus's shape.
    for case in [
        "Do not edit the frontmatter.",
        "The CLI is down; never set `status:` in the file.",
    ] {
        assert!(
            refusals("fixture.md", case).is_empty(),
            "a prohibition in the verb's own clause must still exempt it: {case:?}"
        );
    }
}

#[test]
fn a_prohibition_that_follows_its_subject_still_exempts() {
    // The passive, which is how the plugin README states the rule: the three write tools are the
    // subject and the prohibition is the predicate, so every `EXEMPTIONS` entry sits behind the
    // verbs rather than in front of them.
    let passive = "`Edit`, `Write`, and `NotebookEdit` are denied under `.engineering/planning/`.";
    assert!(
        refusals("fixture.md", passive).is_empty(),
        "a prohibition written in the passive must exempt its own clause:\n{}",
        rendered(&refusals("fixture.md", passive))
    );

    // It reaches no further than that clause, so the instruction after the semicolon is still read.
    let mixed = "`Edit` and `Write` are denied under `.engineering/planning/`; update the \
                 frontmatter yourself.";
    assert_eq!(
        refusals("fixture.md", mixed).len(),
        1,
        "the clause after the prohibition is not covered by it"
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
fn a_prohibition_does_not_exempt_the_block_after_it() {
    // What a block boundary is for. The shipped rule and the regression that breaks it sit in the
    // same document, and a scan that let the rule's `never` reach forward would pass the very file
    // it was written to refuse. Each pair below is one prohibition followed by one instruction, with
    // no sentence-ending punctuation between them — so `sentences()` cannot do this test's work and
    // the block boundary is the only thing separating them.
    let cases = [
        // A blank line.
        (
            "Never edit a store file directly\n\nedit the frontmatter directly\n",
            3,
        ),
        // A table row. The shipped frontmatter-ownership table is exactly this shape: its `id` row
        // reads *never edited*, and the row under it is where a regression would be written.
        (
            "| `id` | machine | fixed at creation and never touched by hand |\n| `status` | you | \
             update `status:` in the file yourself when the CLI is down |\n",
            2,
        ),
        // A list item.
        (
            "- Never edit a store file directly\n- edit the frontmatter directly\n",
            2,
        ),
        // A heading.
        (
            "Never edit a store file directly\n# edit the frontmatter directly\n",
            2,
        ),
    ];

    for (case, line) in cases {
        let found = refusals("fixture.md", case);
        assert_eq!(
            found.len(),
            1,
            "the second block is not exempted by the first: {case:?}\n{}",
            rendered(&found)
        );
        assert_eq!(
            found[0].line, line,
            "the refusal names the second block's own line"
        );
    }

    // Lower-cased on purpose above: a capitalised second line would open a sentence of its own even
    // without the block boundary, and this test is about the boundary.
}

#[test]
fn a_row_planted_in_the_shipped_ownership_table_is_refused() {
    // The same defect against the committed bytes: one row added to the frontmatter-ownership table
    // of `store-conventions.md`, which is where a reader goes to learn which fields they may touch
    // and therefore the single most likely place for this regression to land.
    let path = "integrations/claude-code/skills/planning/references/store-conventions.md";
    let shipped = read(path);
    let mut lines: Vec<&str> = shipped.lines().collect();
    let title = lines
        .iter()
        .position(|line| line.starts_with("| `title` |"))
        .expect("the frontmatter ownership table has a `title` row");
    let planted = "| `status` | you | update `status:` in the file yourself when the CLI is down |";
    lines.insert(title + 1, planted);

    let found = refusals(path, &lines.join("\n"));
    assert!(
        found.iter().any(|refusal| refusal.line == title + 2),
        "a row planted at {path}:{} must be refused:\n  {planted}\n{}",
        title + 2,
        rendered(&found)
    );
}

#[test]
fn naming_the_sanctioned_writer_does_not_licence_the_write_beside_it() {
    // The defect the story was written about was a skill routing *around* the writer, and a sentence
    // that does that names the writer — so an exemption for naming it is an exemption for the
    // regression. Each case below mentions `protocol artifact` and instructs a hand edit anyway.
    let routed = [
        "Skip `protocol artifact move` and write `status: active` into the file.",
        // `the file` names a store surface only because the sentence already named the store —
        // `NAMED_STORE_SURFACES`. Outside such a sentence it is the commonest noun in the corpus.
        "Instead of `protocol artifact body`, edit the file.",
        "Instead of `protocol artifact body`, edit the frontmatter.",
        "When `protocol artifact body` is unavailable, edit the story file with your editor.",
        "`protocol artifact body` is the slow route; edit the frontmatter yourself.",
    ];
    for case in routed {
        assert_eq!(
            refusals("fixture.md", case).len(),
            1,
            "naming the writer in order to route around it is not an exemption: {case:?}"
        );
    }

    // And the road *through* the CLI still passes, which is what the exemption is for: nine shipped
    // sentences describe a write and name the writer in the same breath.
    let sanctioned = [
        "Then write each story's complete body through `protocol artifact body <id> --from <path>`.",
        "A body is changed with `protocol artifact body`, never with an editor.",
        "| replace its complete markdown body | `protocol artifact body <id> --from <path>` |",
    ];
    for case in sanctioned {
        assert!(
            refusals("fixture.md", case).is_empty(),
            "the road through the CLI is not an instruction to write by hand: {case:?}\n{}",
            rendered(&refusals("fixture.md", case))
        );
    }
}

#[test]
fn every_write_verb_and_every_store_surface_is_load_bearing() {
    // What stops this file being satisfied by shortening its data. Cutting `WRITE_VERBS` from
    // forty-six entries to four, and `STORE_SURFACES` from eighteen to four, left every test in the
    // previous version of this file green — so the lists were decoration, and the scan's whole
    // sensitivity was unguarded. One case per entry, generated from the entry itself.
    let mut idle: Vec<String> = Vec::new();

    for verb in WRITE_VERBS {
        let case = format!("{verb} the frontmatter.");
        if refusals("fixture.md", &case).is_empty() {
            idle.push(format!("  - verb `{verb}` fires on nothing: {case:?}"));
        }
    }
    for surface in STORE_SURFACES {
        let case = format!("Edit {surface} now.");
        if refusals("fixture.md", &case).is_empty() {
            idle.push(format!(
                "  - surface `{surface}` fires on nothing: {case:?}"
            ));
        }
    }
    for surface in NAMED_STORE_SURFACES {
        // Only a store surface once the sentence has named the store, so the case has to name it.
        let case = format!("Instead of `protocol artifact body`, edit {surface} now.");
        if refusals("fixture.md", &case).is_empty() {
            idle.push(format!(
                "  - surface `{surface}` fires on nothing: {case:?}"
            ));
        }
        let loose = format!("Edit {surface} now.");
        if !refusals("fixture.md", &loose).is_empty() {
            idle.push(format!(
                "  - surface `{surface}` fires without the store being named: {loose:?}"
            ));
        }
    }

    assert!(
        idle.is_empty(),
        "{} pattern(s) in the set change no answer:\n{}",
        idle.len(),
        idle.join("\n")
    );

    assert!(
        !SANCTIONED.is_empty(),
        "with no sanctioned writer every mention of the CLI is a refusal"
    );
    assert!(
        !NAMED_STORE_SURFACES.is_empty(),
        "with no named-store surface, `edit the file` beside the writer's own name reaches nothing"
    );
    assert!(
        !ROUTING.is_empty(),
        "with no routing marker, naming the CLI excuses every sentence that names it"
    );
    assert!(
        !EXEMPTIONS.is_empty() && !TRAILING_EXEMPTIONS.is_empty(),
        "with no exemptions the shipped prohibitions are read as the instructions they forbid"
    );
}

#[test]
fn reach_is_load_bearing_at_both_ends() {
    // `REACH` was sixty, and setting it to eight, twenty or four hundred left every test in the
    // previous version of this file green: the constant was guarded by nothing, which is why nobody
    // noticed that one ordinary qualifying clause is longer than sixty bytes. Pinned here from both
    // sides, so shrinking it and growing it each fail.
    // Absolute distances, not `REACH`-relative ones: a case written in terms of the constant moves
    // with it and pins nothing. Eighty-eight bytes of clause must still be crossed and a hundred and
    // fifty-eight must not, which brackets the constant from both sides.
    let near = format!("Edit, {}, the frontmatter.", "x".repeat(80));
    let far = format!("Edit, {}, the frontmatter.", "x".repeat(150));

    assert_eq!(
        refusals("fixture.md", &near).len(),
        1,
        "a surface 88 bytes behind its verb is still its object, and REACH is {REACH}"
    );
    assert!(
        refusals("fixture.md", &far).is_empty(),
        "a surface 158 bytes behind its verb is a different subject, and REACH is {REACH}"
    );

    // The clause this constant exists for, in the wording English actually uses.
    for case in [
        "Edit, when the operator has asked for it and the CLI is unavailable, the frontmatter.",
        "Write the corrected wording, keeping the rest of the document as it stands, into the \
         store file.",
    ] {
        assert_eq!(
            refusals("fixture.md", case).len(),
            1,
            "a subordinate clause does not put the surface out of reach: {case:?}"
        );
    }
}

#[test]
fn normalize_joins_what_markup_split() {
    // Two of the three things `normalize` does are load-bearing in a way no end-to-end case reaches:
    // dropping `*` was mutated away and every test stayed green, because emphasis usually wraps a
    // whole word. It matters when it wraps part of a phrase.
    assert_eq!(
        normalize("the **store** file"),
        "the store file",
        "emphasis inside a phrase must not break the phrase"
    );
    assert_eq!(
        normalize("the `status:` field"),
        "the status: field",
        "backticks inside a phrase must not break the phrase"
    );
    assert_eq!(
        normalize("  Edit   the\tFrontmatter  "),
        "edit the frontmatter",
        "case and runs of whitespace are normalised away"
    );
    assert!(
        refusals("fixture.md", "Edit the **store** file yourself.\n").len() == 1,
        "and the scan reads the joined phrase"
    );
}

#[test]
fn a_question_or_an_exclamation_ends_a_sentence() {
    // `sentences()` splits on `?` and `!` as well as `.`, and dropping either left every test in the
    // previous version of this file green. Each case below is a prohibition, then an instruction:
    // read as one sentence the prohibition excuses the instruction, read as two it does not.
    for case in [
        "Would you never do that? Edit the frontmatter directly.",
        "Never do that! Edit the frontmatter directly.",
    ] {
        let found = refusals("fixture.md", case);
        assert_eq!(
            found.len(),
            1,
            "the terminator ends the prohibition's sentence: {case:?}\n{}",
            rendered(&found)
        );
    }
}

#[test]
fn a_refusal_renders_whole_however_the_sentence_is_encoded() {
    // The reporting path is the one that only runs once the guard has caught something, so a panic
    // here is the guard failing in the one place it was supposed to speak. It cut with
    // `String::truncate`, which asserts a character boundary, over a corpus written with an em dash
    // about once a sentence. The instruction below carries one at byte 178 of its normalised form.
    let sentence = "Edit the frontmatter directly when the operator has asked for it and the CLI \
                    is unavailable to you and nobody else is holding the file open and the branch \
                    is yours to finish off \u{2014} a hand-written status is not a faster move, \
                    only an unvalidated one.";
    let found = refusals("fixture.md", sentence);

    assert_eq!(found.len(), 1, "the instruction is refused");
    let message = found[0].render();
    assert!(
        message.contains("fixture.md:1"),
        "the refusal names the file and the line: {message}"
    );

    // And the cut itself, at every byte a multi-byte character could straddle.
    for at in 0..8 {
        let text = format!(
            "{}\u{2014}{}",
            "x".repeat(Refusal::SHOWN - at),
            "y".repeat(20)
        );
        assert!(
            clip(&text, Refusal::SHOWN).len() <= Refusal::SHOWN + '…'.len_utf8(),
            "the clip stays inside its limit with an em dash {at} bytes from it"
        );
    }
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
    // Totality, in the shape `workflow_coverage.rs` uses, and in both directions. A walk that has
    // stopped finding files passes every other test in this file silently, and a pin that names five
    // of seventeen documents leaves twelve of them protected by nothing but that walk.
    let documents = shipped_documents();

    let missing: Vec<&&str> = REQUIRED_SURFACES
        .iter()
        .filter(|required| !documents.iter().any(|path| path == *required))
        .collect();
    assert!(
        missing.is_empty(),
        "the scan did not reach {missing:?}; it read {documents:?}"
    );

    let unpinned: Vec<&String> = documents
        .iter()
        .filter(|path| !REQUIRED_SURFACES.contains(&path.as_str()))
        .collect();
    assert!(
        unpinned.is_empty(),
        "{} document(s) ship under {SURFACE_TREE}/ that REQUIRED_SURFACES does not name, so \
         losing them would keep this suite green. Add them:\n{}",
        unpinned.len(),
        unpinned
            .iter()
            .map(|path| format!("    \"{path}\","))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
