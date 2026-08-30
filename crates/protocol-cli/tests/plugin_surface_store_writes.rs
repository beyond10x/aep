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
//! expression engine outside `trace-domain`, and does not need one here. A write **verb** within
//! `REACH` bytes of a store **surface**, in one sentence, is an instruction to write the store
//! directly — unless one of two exemptions holds. Where a sentence holds several such pairs the
//! refusal names the verb **nearest** its surface, because that is the one a reader is looking for.
//!
//! **Distance, not direction.** The surface is looked for on both sides of the verb: *the
//! frontmatter is yours to edit* is the same instruction as *edit the frontmatter*, and a scan that
//! only looked forward read one and not the other. Behind the verb the reading is narrower, because
//! a noun in front of a verb is usually not its object: a compound noun (*the last body edit*), a
//! passive participle (*how a body is written*) and a surface on the far side of a comma (*preserves
//! frontmatter, validates the store, and bumps the revision*) are all excluded, and each of the
//! three is a shipped sentence.
//!
//! **A prohibition exempts its own clause, not the sentence.** `EXEMPTIONS` are matched inside the
//! verb's clause, and the comma is what opens one: *do not ask the operator, edit the frontmatter*
//! forbids asking and instructs the write, which is the sentence this window exists for. Ahead of
//! the verb the comma is **not** a break (`CLAUSE_BREAKS` against `CLAUSE_OPENERS`), because English
//! puts a coordinated list's predicate last — *`Edit`, `Write` and `NotebookEdit` **are denied***,
//! which `TRAILING_EXEMPTIONS` reads. For the same reason a prohibition reaches across a comma when
//! everything past it is coordination (*denies edit, write and notebookedit*) or when a coordinator
//! follows it (*…is executable, **and** hand-writing the frontmatter it owns is the failure these
//! rules exist to prevent*). Both are shipped shapes; a bare comma is not.
//!
//! **Naming the sanctioned writer is not a licence to route around it.** A sentence naming
//! `SANCTIONED` is describing the road through the CLI — *write the body through `protocol artifact
//! body`* — and the shipped corpus needs that exemption in sixteen places. But the defect this story
//! was written about *was* a skill routing around the writer, and the sentence that does that names
//! it. So naming it exempts a verb only when the writer is that verb's **own instrument**
//! (`writer_is_the_instrument`): named with no punctuation and no clause word between the two, as in
//! *written **through** `protocol artifact body`*. A writer named in another clause — *when
//! `protocol artifact body` fails, edit the frontmatter* — is named in order to be left, and the
//! sentence is read on its verbs alone. The exemption survives where the verb's clause holds no
//! store surface at all, which is what a console transcript and a table of subcommands look like.
//! In the same spirit an exemption whose object **is** the CLI (*instead of **using** `protocol
//! artifact body`, edit the file*) is not an exemption; and inside a sentence that has named the
//! store, `NAMED_STORE_SURFACES` reads a bare *the file* as a store file, which it is not worth
//! reading as anywhere else.
//!
//! Text is normalised before any of that: lowercased, backticks and emphasis removed, hard wraps
//! joined — a phrase split across two lines by the 100-column wrap is still one phrase. A blank line
//! ends a block, and so does a heading, a list item, a table row, a fence, a blockquote marker and a
//! rule. Markdown table rows carry no sentence-ending punctuation, so a scan that joined them read a
//! whole ownership table as one sentence and let the `never edited` in its first row exempt every
//! row under it — which is where a regression to this rule would actually be written.
//!
//! One thing survives the lowercasing: a **capitalised** word opening a wrapped line, after a line
//! that did not end in a terminator, opens a sentence (`opens_a_sentence`). It is the difference
//! between a wrap and a new statement, and without it the tail of whatever paragraph a regression
//! lands in reaches forward and excuses it — 1797 of 18224 planted line positions, measured with the
//! clause window already narrowed. It is also the one thing here that reads the shape of the file
//! rather than its content, so it is held back where the line above it cannot have ended: on a
//! `CONTINUATIONS` word, on a comma, or in front of an acronym.
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
//! * **`ROUTING` is what is left of a blacklist, and it is still a blacklist.** The shape above
//!   catches the sentence that names the writer in another clause, which is where a measured
//!   thirteen of fifteen plainly-worded routing sentences used to escape. What it does not catch is
//!   the sentence that keeps the writer beside the verb and says *go round it anyway* — *edit the
//!   frontmatter yourself with `protocol artifact body` in mind* — because that is intent, and
//!   punctuation cannot see it. Every entry of `ROUTING` is pinned by a sentence in
//!   `REGRESSION_CORPUS` that only that entry catches; a phrasing none of them knows still passes,
//!   so extend it.
//! * **Where a hard wrap falls can still change a verdict.** `opens_a_sentence` is held back at the
//!   three places a wrap demonstrably falls, and every shipped document rewrapped at six widths
//!   gives the answer it gives as committed (`rewrapping_the_corpus_changes_no_verdict`). That is a
//!   measurement over this corpus, not a proof over all prose: a capital at a line head, after a
//!   line ending on a word not in `CONTINUATIONS`, still opens a sentence that the unwrapped text
//!   would not have. Removing the rule outright costs 1797 of 18224 planted positions, so it stays.
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
/// What is left of a blacklist after `writer_is_the_instrument` took the shape out of it. That rule
/// reads the sentence that names the CLI in *another* clause; these read the one that keeps it
/// beside the verb and says go round it anyway — *edit the frontmatter yourself with `protocol
/// artifact body` in mind*. Every entry is either an adverbial of manual action (*yourself*, *by
/// hand*) or a reason to skip the writer (*unavailable*, *is down*, *skip*), and every entry is
/// pinned by a sentence in `REGRESSION_CORPUS` that only it catches. A phrase none of them knows
/// still passes, so add to it.
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
/// interesting part. Pinned to this exact value by `reach_is_the_exact_distance_it_says_it_is` —
/// ninety-nine bytes of filler is crossed and a hundred is not — because a constant a test only
/// brackets is a constant four values wide.
const REACH: usize = 100;

/// Punctuation that closes a clause **ahead** of a verb.
///
/// A comma is deliberately not here, and is in `CLAUSE_OPENERS` instead. English puts a coordinated
/// list's predicate after the list — *`Edit`, `Write` and `NotebookEdit` are denied* — so a
/// forward window that stopped at the first comma would leave `edit` alone with its prohibition on
/// the far side of the punctuation. Behind the verb the same comma is what separates a spliced
/// clause from the one before it, and there it does break.
const CLAUSE_BREAKS: &[char] = &[';', ':', '|', '(', ')', '\u{2014}', '\u{2013}'];

/// Punctuation that opens a clause **behind** a verb — `CLAUSE_BREAKS` and the comma.
///
/// *Do not ask the operator, edit the frontmatter* is the sentence this file's header names as the
/// reason the clause window exists, and for as long as the comma was missing here it was excused by
/// its own first clause — the one thing the window was built to stop.
const CLAUSE_OPENERS: &[char] = &[',', ';', ':', '|', '(', ')', '\u{2014}', '\u{2013}'];

/// Punctuation that separates the sanctioned writer from a verb it does not serve.
///
/// `|` is deliberately absent: a markdown table writes the change in one cell and the command that
/// makes it in the next, and `store-conventions.md:52` is exactly that row.
const WRITER_BREAKS: &[char] = &[',', ';', ':', '(', ')', '\u{2014}', '\u{2013}'];

/// Words that hang one clause off another, so a writer named past one of them is not the writer
/// this verb goes through: *edit the frontmatter **when** `protocol artifact body` fails*.
const CLAUSE_WORDS: &[&str] = &[
    "and", "or", "nor", "but", "then", "when", "if", "unless", "while", "because", "since", "so",
    "instead", "rather", "though", "although", "whenever", "once", "until", "before", "after",
];

/// Words that join items of one list.
const COORDINATORS: &[&str] = &["and", "or", "nor"];

/// Forms of *be*. A verb behind one is a passive participle, and the noun in front of it is its
/// subject rather than its object.
const COPULAS: &[&str] = &["is", "are", "was", "were", "be", "been", "being"];

/// How many words may link an exemption to the writer it names.
const LINKING_WORDS: usize = 3;

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
    let dense: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    let rule = dense.len() >= 3
        && ['-', '_', '=', '*']
            .iter()
            .any(|mark| dense.chars().all(|c| c == *mark));
    line.starts_with('#')
        || line.starts_with('|')
        || line.starts_with("```")
        || line.starts_with('>')
        || bullet
        || ordered
        || rule
}

/// Words a line cannot end a sentence on, so a capital after one of them is a proper noun a wrap
/// pushed to the head of the next line and not a new statement.
///
/// Three entries, and every one of them costs something. Widening this list is how the verdict
/// stops depending on where a formatter breaks the line, and it is also how a planted instruction
/// gets excused by the paragraph it landed in: the articles, prepositions and auxiliaries a first
/// draft of this list held let 784 of 18224 planted line positions through, against 0 for these
/// three. `rewrapping_the_corpus_changes_no_verdict` and
/// `a_planted_instruction_survives_every_position_in_every_shipped_document` are the two ends of
/// that trade, and both are measured over the shipped corpus rather than argued.
const CONTINUATIONS: &[&str] = &["use", "uses", "using"];

/// Whether a wrapped line opens a sentence of its own: it is capitalised, and the line above it
/// neither ended in a terminator nor in a word that has to carry on.
///
/// The head is read as a word rather than as a first character, so the emphasis, backtick or
/// quotation mark a markdown writer opens a line with does not hide the capital behind it. An
/// all-capital head is a proper noun — `YAML`, `CLI`, `JSON` — and not a new statement.
fn opens_a_sentence(joined: &str, raw: &str) -> bool {
    if joined.ends_with(['.', '?', '!', ',', ';', ':']) {
        return false;
    }
    if joined
        .rsplit(' ')
        .next()
        .is_some_and(|token| CONTINUATIONS.contains(&word_of(token)))
    {
        return false;
    }
    let head = raw.split_whitespace().next().map_or("", word_of);
    let acronym = head.len() > 1 && head.chars().all(|c| c.is_ascii_uppercase());
    !acronym && head.starts_with(|character: char| character.is_ascii_uppercase())
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
        .find(|(_, character)| CLAUSE_OPENERS.contains(character))
        .map_or(0, |(at, character)| at + character.len_utf8())
}

/// The same, ignoring the comma.
fn wide_clause_start(sentence: &str, offset: usize) -> usize {
    sentence[..offset]
        .char_indices()
        .rev()
        .find(|(_, character)| CLAUSE_BREAKS.contains(character))
        .map_or(0, |(at, character)| at + character.len_utf8())
}

/// The end of the comma-delimited clause holding the byte at `offset`.
fn narrow_clause_end(sentence: &str, offset: usize) -> usize {
    sentence[offset..]
        .char_indices()
        .find(|(_, character)| CLAUSE_OPENERS.contains(character))
        .map_or(sentence.len(), |(at, _)| offset + at)
}

/// The word, stripped of the punctuation around it.
fn word_of(token: &str) -> &str {
    token.trim_matches(|character: char| !character.is_alphanumeric())
}

/// Whether every word of `span` is a coordinator or a write verb.
fn coordination_only(span: &str) -> bool {
    span.split_whitespace().all(|token| {
        let word = word_of(token);
        COORDINATORS.contains(&word) || WRITE_VERBS.contains(&word)
    })
}

/// Whether a prohibition `span` bytes in front of a verb still forbids it, across the comma
/// between them.
///
/// It does over a coordinated list — *denies `Edit`, `Write` and `NotebookEdit`* — and over a
/// comma that a coordinator follows, which continues the sentence rather than starting a new one:
/// *without the CLI none of the above is executable, and hand-writing the frontmatter it owns is
/// the failure these rules exist to prevent*. It does not over a bare comma, which is the splice
/// this window was built for: *do not ask the operator, edit the frontmatter*.
fn exemption_reaches(span: &str) -> bool {
    if coordination_only(span) {
        return true;
    }
    span.contains(',')
        && span.split(',').skip(1).all(|chunk| {
            chunk
                .split_whitespace()
                .next()
                .is_some_and(|token| COORDINATORS.contains(&word_of(token)))
        })
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
        find_word(sentence, marker, after).is_some_and(|found| {
            let between = &sentence[after..found];
            between.split_whitespace().count() <= LINKING_WORDS
                && !between.contains(|c: char| CLAUSE_OPENERS.contains(&c))
        })
    })
}

/// Whether the clause holding the verb at `start..end` forbids the write rather than instructing it.
fn exempted(sentence: &str, start: usize, end: usize) -> bool {
    let opens = clause_start(sentence, start);
    let wide = wide_clause_start(sentence, start);
    let clause = &sentence[wide..end];
    for marker in EXEMPTIONS {
        let mut at = 0;
        while let Some(found) = find_word(clause, marker, at) {
            at = found + 1;
            let marker_ends = wide + found + marker.len();
            let reaches = wide + found >= opens
                || exemption_reaches(&sentence[marker_ends.min(start)..start]);
            if reaches && !governs_sanctioned(sentence, marker_ends) {
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

/// The occurrence of `surface` nearest the verb at `start..end`, on either side, within `REACH`.
///
/// Direction is not evidence. *The frontmatter is yours to edit* is the same instruction as *edit
/// the frontmatter*, and a scan that only looked forward from the verb read the second and not the
/// first.
fn surface_near(sentence: &str, start: usize, end: usize, surface: &str) -> Option<usize> {
    let ahead = find_word(sentence, surface, end).filter(|found| found - end < REACH);

    let mut behind = None;
    let mut at = 0;
    while let Some(found) = find_word(sentence, surface, at) {
        at = found + 1;
        let stops = found + surface.len();
        if stops > start {
            break;
        }
        let between = &sentence[stops..start];
        let words: Vec<&str> = between.split_whitespace().map(word_of).collect();
        let joined = !words.is_empty()
            && !between.contains(|c: char| CLAUSE_OPENERS.contains(&c))
            && !words.last().is_some_and(|word| COPULAS.contains(word));
        if joined && start - stops < REACH {
            behind = Some(found);
        }
    }

    match (ahead, behind) {
        (Some(ahead), Some(behind)) => Some(if ahead - end <= start - (behind + surface.len()) {
            ahead
        } else {
            behind
        }),
        (found, None) | (None, found) => found,
    }
}

/// Whether the sanctioned writer is this verb's own instrument: named with no punctuation and no
/// clause word between the two, so the write the verb describes is the one that goes through it.
fn writer_is_the_instrument(sentence: &str, start: usize, end: usize) -> bool {
    SANCTIONED.iter().any(|marker| {
        let mut at = 0;
        while let Some(found) = find_word(sentence, marker, at) {
            at = found + 1;
            let span = if found >= end {
                &sentence[end..found]
            } else if found + marker.len() <= start {
                &sentence[found + marker.len()..start]
            } else {
                continue;
            };
            let separated = span.contains(|c: char| WRITER_BREAKS.contains(&c))
                || span.split_whitespace().any(|token| {
                    CLAUSE_WORDS.contains(&token.trim_matches(|c: char| !c.is_alphanumeric()))
                });
            if !separated {
                return true;
            }
        }
        false
    })
}

/// Whether naming the writer excuses this verb reaching the surface at `surface_at`.
///
/// It does when the writer is the verb's own instrument, and it does when the surface the verb
/// reaches is not even in the verb's own clause — which is what a console transcript and a table of
/// subcommands look like. It does not when the verb's clause holds a store surface and the writer
/// is named somewhere else: that is a sentence naming the writer in order to leave it.
fn sanctioned_covers(sentence: &str, start: usize, end: usize, surface_at: usize) -> bool {
    if writer_is_the_instrument(sentence, start, end) {
        return true;
    }
    let opens = clause_start(sentence, start);
    let closes = narrow_clause_end(sentence, end);
    if !(opens <= surface_at && surface_at < closes) {
        return true;
    }
    let between = if surface_at >= end {
        &sentence[end..surface_at]
    } else {
        &sentence[surface_at..start]
    };
    between
        .split_whitespace()
        .any(|token| CLAUSE_WORDS.contains(&word_of(token)))
}

/// The first direct-write instruction in one sentence, if it holds one: the verb, the store surface
/// it reached, and the offset the verb was read from.
fn instruction(sentence: &str) -> Option<(&'static str, &'static str, usize)> {
    let named = names_the_sanctioned_writer(sentence);
    let routed = named && routes_around_the_sanctioned_writer(sentence);
    let surfaces: Vec<&'static str> = STORE_SURFACES
        .iter()
        .copied()
        .chain(NAMED_STORE_SURFACES.iter().copied().take(if named {
            NAMED_STORE_SURFACES.len()
        } else {
            0
        }))
        .collect();

    let mut best: Option<(usize, &'static str, &'static str, usize)> = None;
    for (start, verb) in verb_occurrences(sentence) {
        let end = start + verb.len();
        if exempted(sentence, start, end) {
            continue;
        }
        for surface in &surfaces {
            let Some(found) = surface_near(sentence, start, end, surface) else {
                continue;
            };
            if named && !routed && sanctioned_covers(sentence, start, end, found) {
                continue;
            }
            let gap = if found >= end {
                found - end
            } else {
                start - (found + surface.len())
            };
            if best.is_none_or(|(shortest, ..)| gap < shortest) {
                best = Some((gap, verb, surface, start));
            }
        }
    }
    best.map(|(_, verb, surface, start)| (verb, surface, start))
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

    // A semicolon is a harder boundary than a comma: nothing carries a prohibition across one,
    // not even the coordinated list that carries it across a comma.
    assert_eq!(
        refusals("fixture.md", "Never edit; write the frontmatter.").len(),
        1,
        "a prohibition does not reach across a semicolon"
    );

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

    // And the emphasis, backtick or bracket a markdown writer opens a line with does not hide the
    // capital behind it: without the strip, `**Edit**` at a line head reads as lower case, the
    // line joins the one above it, and the `not` three words up excuses the write.
    let emphasised = "The operator has asked us to\n**Edit** the frontmatter directly.\n";
    let found = refusals("fixture.md", emphasised);
    assert_eq!(
        found.len(),
        1,
        "a capital behind markup still opens a sentence:\n{}",
        rendered(&found)
    );
    assert_eq!(found[0].line, 2, "the refusal names the emphasised line");
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
        // A fence. The shipped skills open a `console` block right after the rule they state, and
        // the fence marker normalises to a word, so without this the block runs on.
        (
            "Never edit a store file directly\n```console\nupdate the frontmatter\n```\n",
            3,
        ),
        // A blockquote.
        (
            "Never edit a store file directly\n> update the frontmatter\n",
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
fn a_block_does_not_reach_forward_into_the_block_below_it() {
    // The other half of what a block boundary is for, and the half no case reached: without it a
    // verb in one row reaches a noun in the next and refuses a table that says nothing. Both
    // fixtures below are two ordinary rows of a two-column table and two ordinary steps of a
    // numbered list; read as one run of prose, `set … body` and `set … the body` are a write verb
    // twenty-odd bytes from a store surface, and this scan would refuse them.
    for (case, what) in [
        (
            "| `title` | you | set it at creation |\n| `body` | you | authored by the operator |\n",
            "a table row",
        ),
        (
            "1) set the title at creation\n2) the body is authored by the operator\n",
            "an ordered list item",
        ),
    ] {
        assert!(
            refusals("fixture.md", case).is_empty(),
            "{what} must not reach into the one below it:\n{}",
            rendered(&refusals("fixture.md", case))
        );
    }
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

/// One sentence per entry of `WRITE_VERBS`, `STORE_SURFACES`, `NAMED_STORE_SURFACES` and
/// `ROUTING`, written out rather than generated from the entry.
///
/// The version of this file an adversary measured generated one case per entry **from the entry**,
/// so deleting an entry deleted its own case and 188 of 214 entries could each be dropped with the
/// whole suite green. A corpus that shrinks when the thing it guards shrinks guards nothing. These
/// are literals: drop `overwrote` from `WRITE_VERBS` and *Overwrote the frontmatter.* stops being
/// refused, and this test says so.
const REGRESSION_CORPUS: &[&str] = &[
    "Edit the frontmatter.",
    "Edits the frontmatter.",
    "Edited the frontmatter.",
    "Editing the frontmatter.",
    "Patch the frontmatter.",
    "Patches the frontmatter.",
    "Patched the frontmatter.",
    "Patching the frontmatter.",
    "Modify the frontmatter.",
    "Modifies the frontmatter.",
    "Modified the frontmatter.",
    "Modifying the frontmatter.",
    "Rewrite the frontmatter.",
    "Rewrites the frontmatter.",
    "Rewrote the frontmatter.",
    "Rewriting the frontmatter.",
    "Overwrite the frontmatter.",
    "Overwrites the frontmatter.",
    "Overwrote the frontmatter.",
    "Overwriting the frontmatter.",
    "Write the frontmatter.",
    "Writes the frontmatter.",
    "Wrote the frontmatter.",
    "Written the frontmatter.",
    "Writing the frontmatter.",
    "Append the frontmatter.",
    "Appends the frontmatter.",
    "Appended the frontmatter.",
    "Appending the frontmatter.",
    "Insert the frontmatter.",
    "Inserts the frontmatter.",
    "Inserted the frontmatter.",
    "Inserting the frontmatter.",
    "Update the frontmatter.",
    "Updates the frontmatter.",
    "Updated the frontmatter.",
    "Updating the frontmatter.",
    "Change the frontmatter.",
    "Changes the frontmatter.",
    "Changed the frontmatter.",
    "Changing the frontmatter.",
    "Set the frontmatter.",
    "Sets the frontmatter.",
    "Setting the frontmatter.",
    "Replace the frontmatter.",
    "Replaces the frontmatter.",
    "Replaced the frontmatter.",
    "Replacing the frontmatter.",
    "Fix the frontmatter.",
    "Fixes the frontmatter.",
    "Fixed the frontmatter.",
    "Fixing the frontmatter.",
    "Amend the frontmatter.",
    "Amends the frontmatter.",
    "Amended the frontmatter.",
    "Amending the frontmatter.",
    "Adjust the frontmatter.",
    "Adjusts the frontmatter.",
    "Adjusted the frontmatter.",
    "Adjusting the frontmatter.",
    "Correct the frontmatter.",
    "Corrects the frontmatter.",
    "Corrected the frontmatter.",
    "Correcting the frontmatter.",
    "Tweak the frontmatter.",
    "Tweaks the frontmatter.",
    "Tweaked the frontmatter.",
    "Tweaking the frontmatter.",
    "Bump the frontmatter.",
    "Bumps the frontmatter.",
    "Bumped the frontmatter.",
    "Bumping the frontmatter.",
    "Add the frontmatter.",
    "Adds the frontmatter.",
    "Added the frontmatter.",
    "Adding the frontmatter.",
    "Delete the frontmatter.",
    "Deletes the frontmatter.",
    "Deleted the frontmatter.",
    "Deleting the frontmatter.",
    "Remove the frontmatter.",
    "Removes the frontmatter.",
    "Removed the frontmatter.",
    "Removing the frontmatter.",
    "Save the frontmatter.",
    "Saves the frontmatter.",
    "Saved the frontmatter.",
    "Saving the frontmatter.",
    "Alter the frontmatter.",
    "Alters the frontmatter.",
    "Altered the frontmatter.",
    "Altering the frontmatter.",
    "Revise the frontmatter.",
    "Revises the frontmatter.",
    "Revised the frontmatter.",
    "Revising the frontmatter.",
    "Drop the frontmatter.",
    "Drops the frontmatter.",
    "Dropped the frontmatter.",
    "Dropping the frontmatter.",
    "Paste the frontmatter.",
    "Pastes the frontmatter.",
    "Pasted the frontmatter.",
    "Pasting the frontmatter.",
    "Put the frontmatter.",
    "Puts the frontmatter.",
    "Putting the frontmatter.",
    "A `sed` one-liner over the frontmatter is quicker.",
    "A `tee` into the story file is quicker.",
    "Edit frontmatter now.",
    "Edit front matter now.",
    "Edit yaml header now.",
    "Edit yaml block now.",
    "Edit yaml at the top now.",
    "Edit status: now.",
    "Edit status field now.",
    "Edit status line now.",
    "Edit revision: now.",
    "Edit revision field now.",
    "Edit relations: now.",
    "Edit relations field now.",
    "Edit body now.",
    "Edit .engineering/planning now.",
    "Edit store file now.",
    "Edit store files now.",
    "Edit planning file now.",
    "Edit planning files now.",
    "Edit planning document now.",
    "Edit planning documents now.",
    "Edit artifact file now.",
    "Edit artifact files now.",
    "Edit artifact's file now.",
    "Edit story file now.",
    "Edit task file now.",
    "Edit the artifact now.",
    "Instead of `protocol artifact body`, edit the file now.",
    "Instead of `protocol artifact body`, edit a file now.",
    "Instead of `protocol artifact body`, edit this file now.",
    "Instead of `protocol artifact body`, edit that file now.",
    "Instead of `protocol artifact body`, edit the document now.",
    "Edit the frontmatter yourself with `protocol artifact body` in mind.",
    "Edit the frontmatter by hand with `protocol artifact body` in mind.",
    "Edit the frontmatter by-hand with `protocol artifact body` in mind.",
    "Edit the frontmatter as a hand-edit with `protocol artifact body` in mind.",
    "Edit the frontmatter as hand-edits with `protocol artifact body` in mind.",
    "Edit the frontmatter as a hand edit with `protocol artifact body` in mind.",
    "Edit the frontmatter as a hand-written change with `protocol artifact body` in mind.",
    "Edit the frontmatter as a hand-edited change with `protocol artifact body` in mind.",
    "Edit the frontmatter manually with `protocol artifact body` in mind.",
    "Edit the frontmatter directly with `protocol artifact body` in mind.",
    "Edit the frontmatter in place with `protocol artifact body` in mind.",
    "Edit the frontmatter in your editor with `protocol artifact body` in mind.",
    "Edit the frontmatter to skip `protocol artifact body` today.",
    "Edit the frontmatter in a change that skips `protocol artifact body`.",
    "Edit the frontmatter skipping `protocol artifact body` today.",
    "Edit the frontmatter with `protocol artifact body` unavailable.",
    "Edit the frontmatter with `protocol artifact body` is down.",
    "Edit the frontmatter with `protocol artifact body` not available.",
    "Edit the frontmatter with `protocol artifact body` too slow to bother.",
    "Edit the frontmatter to duck the slow route of `protocol artifact body`.",
    "Edit the frontmatter to bypass `protocol artifact body` today.",
    "Edit the frontmatter in a change that bypasses `protocol artifact body`.",
    "Edit the frontmatter as a shortcut past `protocol artifact body`.",
    "It is faster to edit the frontmatter than `protocol artifact body` is.",
];

/// Sentences the shipped corpus is full of and this scan must not refuse: a prohibition in the
/// verb's own clause, a prohibition that follows its subject, the road through the CLI, and the
/// nouns that name a store file only once the store has been named.
///
/// The other half of the same job. `EXEMPTIONS`, `TRAILING_EXEMPTIONS` and `SANCTIONED` are only
/// load-bearing if something goes red when they shrink, and nothing here is generated from them
/// either.
const LAWFUL_CORPUS: &[&str] = &[
    "Do never edit the frontmatter.",
    "Do not edit the frontmatter.",
    "Do no edit the frontmatter.",
    "Do nor edit the frontmatter.",
    "Do cannot edit the frontmatter.",
    "Do can't edit the frontmatter.",
    "Do don't edit the frontmatter.",
    "Do doesn't edit the frontmatter.",
    "Do without edit the frontmatter.",
    "Do rather than edit the frontmatter.",
    "Do instead of edit the frontmatter.",
    "Do refuse edit the frontmatter.",
    "Do refuses edit the frontmatter.",
    "Do refused edit the frontmatter.",
    "Do deny edit the frontmatter.",
    "Do denies edit the frontmatter.",
    "Do denied edit the frontmatter.",
    "Do forbid edit the frontmatter.",
    "Do forbids edit the frontmatter.",
    "Do forbidden edit the frontmatter.",
    "Do banned edit the frontmatter.",
    "Do avoid edit the frontmatter.",
    "Do stop edit the frontmatter.",
    "Do before editing edit the frontmatter.",
    "Do before writing edit the frontmatter.",
    "Do before changing edit the frontmatter.",
    "`Edit`, `Write` and `NotebookEdit` are denied under `.engineering/planning/`.",
    "`Edit` is denied under `.engineering/planning/`.",
    "`Edit` and `Write` are refused under `.engineering/planning/`.",
    "`Edit` is refused under `.engineering/planning/`.",
    "`Edit` and `Write` are forbidden under `.engineering/planning/`.",
    "`Edit` is forbidden under `.engineering/planning/`.",
    "`Edit` and `Write` are banned under `.engineering/planning/`.",
    "`Edit` is banned under `.engineering/planning/`.",
    "`Edit` and `Write` are never how the frontmatter moves.",
    "`Edit` is never how the frontmatter moves.",
    "`Edit` and `Write` are not how the frontmatter moves.",
    "`Edit` is not how the frontmatter moves.",
    "`Edit` must not reach the frontmatter.",
    "`Edit` may not reach the frontmatter.",
    "Edit the file now.",
    "Edit a file now.",
    "Edit this file now.",
    "Edit that file now.",
    "Edit the document now.",
    "Then write each story's complete body through `protocol artifact body <id> --from <path>`.",
    "A body is changed with `protocol artifact body`, never with an editor.",
    "| replace its complete markdown body | `protocol artifact body <id> --from <path>` |",
];

#[test]
fn a_comma_does_not_carry_a_prohibition_into_the_clause_after_it() {
    // The sentence this file's header quotes as the reason the clause window exists, written the
    // way the header writes it — with a comma. `CLAUSE_BREAKS` did not hold one, so the clause
    // holding `edit` opened at byte zero and the `not` in *do not ask* sat inside it; the case that
    // was supposed to cover this substituted a colon, which is the one mark that made it work.
    for case in [
        "Do not ask the operator, edit the frontmatter.",
        "Never open a pull request for this, edit the frontmatter directly.",
        "The operator has not asked, patch the body in place.",
        "You cannot reach the CLI, set `status: active` in the file yourself.",
        "There is no need to stop, update the `status:` field yourself.",
    ] {
        let found = refusals("fixture.md", case);
        assert_eq!(
            found.len(),
            1,
            "a prohibition one comma away does not licence this write: {case:?}\n{}",
            rendered(&found)
        );
    }

    // And the comma a prohibition does reach across: the one that separates items of a list, and
    // the one a coordinator follows. Both are shipped shapes, and both must still pass.
    for case in [
        "The plugin denies `Edit`, `Write` and `NotebookEdit` under `.engineering/planning/`.",
        "Never edit, patch, or rewrite the frontmatter.",
        "Without the CLI none of this is executable, and hand-writing the frontmatter it owns is \
         the failure these rules exist to prevent.",
    ] {
        assert!(
            refusals("fixture.md", case).is_empty(),
            "a prohibition still reaches across a list's own comma: {case:?}\n{}",
            rendered(&refusals("fixture.md", case))
        );
    }
}

#[test]
fn an_exemption_whose_object_is_the_writer_is_no_exemption_however_it_is_phrased() {
    // *Instead of `protocol artifact body`, edit the file* is a way around the writer, not a
    // prohibition of the write — and the rule that said so required **zero** words between the
    // marker and the CLI's name, so the gerund every writer of English reaches for restored the
    // exemption. The marker was present, matched, and then discarded on a whitespace count.
    for case in [
        "Instead of `protocol artifact body`, edit the file.",
        "Instead of using `protocol artifact body`, edit the file.",
        "Instead of using `protocol artifact body`, edit the frontmatter.",
        "Rather than calling `protocol artifact move`, set `status: active` in the frontmatter.",
        "Instead of shelling out to `protocol artifact body`, patch the body in place.",
        "Edit the frontmatter yourself instead of using `protocol artifact body`.",
        // No comma to open a clause and no `ROUTING` phrase to void the exemption: the only thing
        // that refuses this one is the marker's object being the CLI.
        "Rather than calling `protocol artifact move` set `status: active` in the frontmatter.",
    ] {
        assert_eq!(
            refusals("fixture.md", case).len(),
            1,
            "an exemption that names the CLI as the thing to avoid is not an exemption: {case:?}"
        );
    }

    // The other side of that rule: a prohibition and a mention of the CLI in two clauses of one
    // sentence is the shipped shape, and the punctuation between them is what says so. Without it
    // the `not` here would be discarded as an exemption governing the writer, and a sentence that
    // forbids the write would be read as instructing it.
    assert!(
        refusals(
            "fixture.md",
            "Do not patch the body; `protocol artifact` is the writer."
        )
        .is_empty(),
        "a prohibition in one clause and the writer in the next is not a route around it"
    );
}

#[test]
fn a_surface_in_front_of_its_verb_is_still_the_verb_s_object() {
    // English puts the object in front of the verb in the copular and cleft moods a documentation
    // writer reaches for constantly, and a scan that only looked forward from the verb read
    // *edit the frontmatter* and not *the frontmatter is yours to edit*. Both name `frontmatter`
    // and `status:`, which are spelled out in `STORE_SURFACES`; neither is paraphrase.
    for case in [
        "The frontmatter is yours to edit.",
        "The `status:` field is the one you set by hand.",
        "It is the frontmatter you edit, and nothing else.",
        "The frontmatter is what you change when a review lands.",
    ] {
        let found = refusals("fixture.md", case);
        assert_eq!(
            found.len(),
            1,
            "an instruction that names its object first is still an instruction: {case:?}\n{}",
            rendered(&found)
        );
    }

    // Looking behind the verb is not looking anywhere: a compound noun, a passive participle and a
    // surface in another clause are all in front of a verb and none of them is its object. All
    // three are shipped shapes — `plan-reviewer.md:56`, `plan-reviewer.md:43`, `SKILL.md:57`.
    for case in [
        "The evidence is the date of the last body edit.",
        "A style disagreement about how a body is written is not a finding.",
        "The CLI preserves frontmatter, validates the store, and bumps the revision once, so \
         nothing here is a hand edit.",
    ] {
        assert!(
            refusals("fixture.md", case).is_empty(),
            "a noun in front of a verb is not automatically its object: {case:?}\n{}",
            rendered(&refusals("fixture.md", case))
        );
    }

    // And the comma is the boundary that decides it: the same words with nothing but a comma
    // between the surface and the verb are two statements, not one instruction.
    assert!(
        refusals("fixture.md", "The frontmatter, however, you edit.").is_empty(),
        "a surface on the far side of a comma is not the verb's object"
    );
    assert_eq!(
        refusals("fixture.md", "The frontmatter however you edit.").len(),
        1,
        "and without the comma it is"
    );
}

#[test]
fn a_rule_ends_a_block_as_starts_block_says_it_does() {
    // `starts_block` documented itself as recognising "a heading, a list item, a table row, a
    // fence, a blockquote or a rule" and recognised no rule: `---`, `___` and a setext underline
    // all fell through, so a prohibition on one side of a horizontal rule reached across it. `---`
    // is not hypothetical here — it opens and closes the YAML frontmatter of all five shipped
    // `SKILL.md`, and `integrations/codex/AGENTS.planning.md:17` is a thematic break.
    for (rule, what) in [
        ("---", "a horizontal rule"),
        ("___", "an underscore rule"),
        ("===", "a setext heading underline"),
        ("***", "an asterisk rule"),
    ] {
        let document = format!("Never edit a store file directly\n{rule}\nedit the frontmatter\n");
        let found = refusals("fixture.md", &document);
        assert_eq!(
            found.len(),
            1,
            "{what} ({rule:?}) must end the block:\n{}",
            rendered(&found)
        );
        assert_eq!(
            found[0].line, 3,
            "the refusal names the line after the rule"
        );
    }
}

#[test]
fn rewrapping_the_corpus_changes_no_verdict() {
    // The acceptance is about content, and `opens_a_sentence` reads a line break. It has to — a
    // hard-wrapped corpus has no other signal for where one statement stops and the next starts —
    // but a capitalised word at a wrapped line head was enough to split a prohibition from the
    // clause it protected, so a formatter reflowing this corpus could redden a green tree. That is
    // how a guard gets deleted rather than fixed. Every shipped document is rewrapped at six widths
    // here and must give the same answer it gives as committed.
    for path in &shipped_documents() {
        let shipped = read(path);
        for width in [60_usize, 72, 80, 90, 100, 110] {
            let mut rewrapped = String::new();
            for block in shipped.split("\n\n") {
                let mut column = 0;
                for word in block.split_whitespace() {
                    if column > 0 && column + 1 + word.len() > width {
                        rewrapped.push('\n');
                        column = 0;
                    } else if column > 0 {
                        rewrapped.push(' ');
                        column += 1;
                    }
                    rewrapped.push_str(word);
                    column += word.len();
                }
                rewrapped.push_str("\n\n");
            }
            let found = refusals(path, &rewrapped);
            assert!(
                found.is_empty(),
                "{path} rewrapped at {width} columns is refused, and as committed it is not:\n{}",
                rendered(&found)
            );
        }
    }

    // And the shape that made this measurable: one sentence, wrapped two ways, must answer twice
    // the same. Each of these is a prohibition whose wrapped form was refused.
    for (one_line, wrapped) in [
        (
            "The plugin denies `Edit`, `Write` and `NotebookEdit` under `.engineering/planning/`.",
            "The plugin denies `Edit`,\n`Write` and `NotebookEdit` under `.engineering/planning/`.",
        ),
        (
            "Never hand a status change to `sed`, and never let a YAML tool write the `status:` \
             field.",
            "Never hand a status change to `sed`, and never let a\nYAML tool write the `status:` \
             field.",
        ),
        (
            "No agent may reach for `sed`, and none may use Edit on the frontmatter of a planning \
             document.",
            "No agent may reach for `sed`, and none may use\nEdit on the frontmatter of a planning \
             document.",
        ),
    ] {
        assert_eq!(
            refusals("fixture.md", one_line).is_empty(),
            refusals("fixture.md", wrapped).is_empty(),
            "where the line breaks changes the verdict: {one_line:?}"
        );
    }
}

#[test]
fn a_sentence_that_names_the_writer_in_order_to_leave_it_is_refused() {
    // Naming `protocol artifact` used to exempt a sentence outright unless one of a list of
    // phrases — *yourself*, *by hand*, *skip* — appeared in it, and a regression that worded its
    // reason differently passed. Measured, that was not a marginal limit: four plainly-worded
    // routing sentences escaped at 2278 of 2278 line positions of this corpus. The exemption is
    // now a shape — the writer has to be the verb's own instrument — and the phrase list is what
    // is left for the sentences that punctuation cannot see.
    for case in [
        "When `protocol artifact body` fails, edit the frontmatter.",
        "If `protocol artifact` is broken, edit the frontmatter.",
        "`protocol artifact` is not on PATH here, so update the frontmatter.",
        "`protocol artifact body` is overkill for a typo; edit the story file.",
        "For a one-word change, forget `protocol artifact body` and edit the file.",
    ] {
        assert_eq!(
            refusals("fixture.md", case).len(),
            1,
            "naming the writer in order to leave it is not an exemption: {case:?}"
        );
    }

    // The three shapes the shipped corpus needs the exemption for, and needs it in sixteen places:
    // the writer as the verb's instrument, the writer as the verb's subject in a console
    // transcript, and a surface the verb does not reach at all.
    for case in [
        "Then write each story's complete body through `protocol artifact body <id> --from <path>`.",
        "$ protocol artifact body story:credential-store --from story-body.md \
         story:credential-store body replaced (revision 2) at \
         `.engineering/planning/story/credential-store.md`",
        "Every mutation crosses `protocol artifact`: `new` creates, `relate` changes relations, \
         `move` changes status, and `body <id> --from <path|->` replaces prose while preserving \
         frontmatter and revision rules.",
    ] {
        assert!(
            refusals("fixture.md", case).is_empty(),
            "the road through the CLI is not an instruction to write by hand: {case:?}\n{}",
            rendered(&refusals("fixture.md", case))
        );
    }
}

#[test]
fn every_sentence_of_the_regression_corpus_is_refused() {
    // What stops this file being satisfied by shortening its data. Cutting `WRITE_VERBS` from
    // forty-six entries to four, and `STORE_SURFACES` from eighteen to four, left every test in an
    // earlier version of this file green — so the lists were decoration and the scan's whole
    // sensitivity was unguarded. Then the test that fixed *that* generated its cases from the
    // entries, which fixed it only against a wholesale cut.
    let idle: Vec<&&str> = REGRESSION_CORPUS
        .iter()
        .filter(|case| refusals("fixture.md", case).is_empty())
        .collect();

    assert!(
        idle.is_empty(),
        "{} of {} instructions to write the store by hand are not refused:\n{}",
        idle.len(),
        REGRESSION_CORPUS.len(),
        idle.iter()
            .map(|case| format!("  - {case}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_sentence_of_the_lawful_corpus_passes() {
    let refused: Vec<String> = LAWFUL_CORPUS
        .iter()
        .filter(|case| !refusals("fixture.md", case).is_empty())
        .map(|case| format!("  - {case}\n{}", rendered(&refusals("fixture.md", case))))
        .collect();

    assert!(
        refused.is_empty(),
        "{} of {} lawful sentences are read as instructions to write the store by hand:\n{}",
        refused.len(),
        LAWFUL_CORPUS.len(),
        refused.join("\n")
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
        !EXEMPTIONS.is_empty() && !TRAILING_EXEMPTIONS.is_empty(),
        "with no exemptions the shipped prohibitions are read as the instructions they forbid"
    );
}

#[test]
fn reach_is_the_exact_distance_it_says_it_is() {
    // `REACH` was sixty, and setting it to eight, twenty or four hundred left every test in an
    // earlier version of this file green: the constant was guarded by nothing, which is why nobody
    // noticed that one ordinary qualifying clause is longer than sixty bytes. The version after
    // that bracketed it at 88 and 158 — which left 89, 99, 101 and 157 all green, so the constant
    // was still four values wide. This pins the boundary itself: ninety-nine bytes of filler is
    // crossed and a hundred is not, in both directions, which is one value of `REACH` and no other.
    let filler = |gap: usize| "x".repeat(gap - 8);
    let ahead = |gap: usize| format!("Edit, {}, the frontmatter.", filler(gap));
    let behind = |gap: usize| format!("The frontmatter {} you edit.", "x".repeat(gap - 6));

    assert_eq!(
        refusals("fixture.md", &ahead(99)).len(),
        1,
        "a surface ninety-nine bytes ahead of its verb is still its object, and REACH is {REACH}"
    );
    assert!(
        refusals("fixture.md", &ahead(100)).is_empty(),
        "a surface a hundred bytes ahead of its verb is out of reach, and REACH is {REACH}"
    );
    assert_eq!(
        refusals("fixture.md", &behind(99)).len(),
        1,
        "a surface ninety-nine bytes behind its verb is still its object, and REACH is {REACH}"
    );
    assert!(
        refusals("fixture.md", &behind(100)).is_empty(),
        "a surface a hundred bytes behind its verb is out of reach, and REACH is {REACH}"
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
    //
    // Every length here is an absolute number, not `Refusal::SHOWN`. Written in terms of the
    // constant, this test moved with it: `SHOWN = 100000` kept it green and put a five-hundred-byte
    // sentence in a failure message.
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

    // A sentence far longer than a message should carry is cut, and cut to a fixed size.
    let long = format!("Edit the frontmatter {} now.", "x".repeat(400));
    let rendered = refusals("fixture.md", &long)[0].render();
    let shown = rendered
        .lines()
        .last()
        .expect("the message carries the sentence");
    assert!(
        shown.contains('\u{2026}'),
        "a four-hundred-byte sentence is cut and says so: {shown}"
    );
    assert!(
        shown.trim().len() <= 190,
        "the cut sentence stays inside a readable message, and was {} bytes",
        shown.trim().len()
    );

    // And a sentence a message can carry whole is carried whole.
    let short = "Edit the frontmatter when the operator has asked and the CLI is unavailable.";
    let carried = refusals("fixture.md", short)[0].render();
    assert!(
        carried.ends_with(short.to_lowercase().as_str()),
        "a seventy-five-byte sentence is not cut: {carried}"
    );

    // And the cut itself, at every byte a multi-byte character could straddle.
    for at in 0..8 {
        let text = format!("{}\u{2014}{}", "x".repeat(180 - at), "y".repeat(20));
        assert!(
            clip(&text, 180).len() <= 180 + '\u{2026}'.len_utf8(),
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
