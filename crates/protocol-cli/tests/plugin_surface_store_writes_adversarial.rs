//! Adversarial cases against `tests/plugin_surface_store_writes.rs`.
//!
//! The guard under attack is a tripwire whose only value is its false-negative rate: it exists so
//! that a *future* skill saying "edit the frontmatter directly" cannot ship. Every case here is a
//! sentence somebody rewriting a shipped skill would plausibly write, run through the guard's own
//! decision procedure.
//!
//! The procedure is reached by compiling the guard's source at test time. Its items are private to
//! its own integration-test crate and its header is a `//!` block, so neither `#[path] mod` nor
//! `include!` can reach them (`include!` reports E0753 on the first inner doc comment). Rewriting
//! the guard to expose them would make this file a fork of the thing it is testing; compiling the
//! committed bytes keeps it a test *of* the guard, and a change to the guard is picked up on the
//! next run.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The tree of harness surfaces the guard walks.
const SURFACE_TREE: &str = "integrations";

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The guard's committed source.
fn guard_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/plugin_surface_store_writes.rs")
}

/// The probe binary: the guard's own `refusals`, driven from stdin.
///
/// Built once per test binary. Chunks of stdin separated by a `\u{1}` line are read as documents;
/// one line of `PASS` or `REFUSED line=… verb=… surface=…` is printed per chunk.
fn probe() -> &'static Path {
    static PROBE: OnceLock<PathBuf> = OnceLock::new();
    PROBE.get_or_init(|| {
        let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store-write-adversary");
        std::fs::create_dir_all(&scratch).expect("the scratch directory is creatable");

        // The guard verbatim, with its header demoted from an inner doc comment to a comment. No
        // other byte is changed, so what runs below is the committed decision procedure.
        let source = std::fs::read_to_string(guard_path()).expect("the guard's source is readable");
        let demoted: String = source
            .lines()
            .map(|line| {
                line.strip_prefix("//!")
                    .map_or_else(|| line.to_owned(), |rest| format!("//{rest}"))
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(scratch.join("guard.rs"), demoted).expect("the copy is writable");
        std::fs::write(
            scratch.join("driver.rs"),
            r#"include!("guard.rs");

fn main() {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).expect("stdin");
    for chunk in input.split("\u{1}\n") {
        let found = refusals("probe.md", chunk);
        match found.first() {
            None => println!("PASS"),
            // `render` on purpose: the message is the guard's product, and it is the path that only
            // runs once the guard has caught something.
            Some(first) => println!("REFUSED {}", first.render().replace('\n', " ")),
        }
    }
}
"#,
        )
        .expect("the driver is writable");

        let binary = scratch.join("probe");
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
        let built = Command::new(rustc)
            .current_dir(&scratch)
            .args(["--edition", "2021", "-A", "warnings", "-o"])
            .arg(&binary)
            .arg("driver.rs")
            .env("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("rustc runs");
        assert!(
            built.status.success(),
            "the guard's own source must compile as a driver:\n{}",
            String::from_utf8_lossy(&built.stderr)
        );
        binary
    })
}

/// Whether the guard refuses each document, in order.
fn refused(documents: &[String]) -> Vec<bool> {
    use std::io::Write as _;

    let mut child = Command::new(probe())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the probe runs");
    let payload = documents.join("\u{1}\n");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(payload.as_bytes())
        .expect("the payload is writable");
    let out = child.wait_with_output().expect("the probe exits");
    assert!(
        out.status.success(),
        "the guard's own decision procedure aborted ({}) while reading these documents:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr).trim()
    );

    let verdicts: Vec<bool> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.starts_with("REFUSED"))
        .collect();
    assert_eq!(
        verdicts.len(),
        documents.len(),
        "the probe answered {} of {} documents",
        verdicts.len(),
        documents.len()
    );
    verdicts
}

/// The documents the guard refuses, rendered as a list for a failure message.
fn escaped(cases: &[&str]) -> Vec<String> {
    let documents: Vec<String> = cases.iter().map(|case| (*case).to_owned()).collect();
    refused(&documents)
        .into_iter()
        .zip(cases.iter())
        .filter(|(refused, _)| !refused)
        .map(|(_, case)| format!("  - {case}"))
        .collect()
}

/// Every markdown document under `integrations/`, repo-relative and sorted.
fn shipped_documents() -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut directories = vec![root().join(SURFACE_TREE)];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory).expect("the surface tree is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                directories.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                found.push(
                    path.strip_prefix(root())
                        .expect("inside the repository")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    found.sort();
    found
}

/// The frontmatter-ownership reference the planning skill points its reader at.
const STORE_CONVENTIONS: &str =
    "integrations/claude-code/skills/planning/references/store-conventions.md";

#[test]
fn a_sentence_a_regressing_skill_would_contain_is_refused() {
    // The acceptance, asserted directly: *a test … refuses one whose text instructs a direct write
    // to a planning-store file — editing frontmatter, patching a body, or writing `status:` by
    // hand.* Each line below is one of those three shapes in wording a person would actually use,
    // and none of them needs a verb or a noun the guard's own header calls out as out of scope.
    let cases: &[&str] = &[
        "Amend the frontmatter yourself.",
        "Adjust the `status:` field by hand.",
        "Correct the frontmatter yourself.",
        "Tweak the frontmatter yourself when the operator asks.",
        "Bump the `revision:` field yourself.",
        "Add a `relations:` entry to the file by hand.",
        "Delete the `status:` line from the file.",
        "Save the corrected `status:` into the file.",
        "Edit the artifact's YAML header.",
        "Update the YAML at the top of the document.",
        "Patch the artifact directly.",
        "Modify the artifact yourself.",
        "Use the `Edit` tool on the artifact's YAML header.",
        "Write the new status into the planning document.",
        "Amend the document in `.engineering/planning/` when the operator asks.",
    ];
    let missed = escaped(cases);

    assert!(
        missed.is_empty(),
        "{} of {} instructions to write the store by hand are not refused:\n{}",
        missed.len(),
        cases.len(),
        missed.join("\n")
    );
}

#[test]
fn naming_the_sanctioned_writer_does_not_licence_a_hand_edit() {
    // The exemption the guard's header names as a limit, measured: a sentence that mentions
    // `protocol artifact` is exempt *anywhere* in the sentence, and the sentence a regression
    // actually takes is the one that names the CLI in order to route around it. This is not a
    // hypothetical shape — it is the shape of the defect the story was written about, which was a
    // skill telling agents to patch bodies rather than call the writer.
    let cases: &[&str] = &[
        "Skip `protocol artifact move` and write `status: active` into the file.",
        "Instead of `protocol artifact body`, edit the file.",
        "When `protocol artifact body` is unavailable, edit the story file with your editor.",
        "`protocol artifact body` is the slow route; edit the frontmatter yourself.",
    ];
    let missed = escaped(cases);

    assert!(
        missed.is_empty(),
        "{} of {} sentences route around the sanctioned writer and are exempted by naming it:\n{}",
        missed.len(),
        cases.len(),
        missed.join("\n")
    );
}

#[test]
fn a_prohibition_does_not_exempt_a_later_row_of_the_same_table() {
    // Sentences are split on `. `, and a markdown table has no sentence-ending punctuation, so a
    // whole table is one sentence and one `never` in its first data row exempts every row after it.
    // The shipped frontmatter-ownership table is exactly that shape: its `id` row reads *never
    // edited*, and it is the table a person would add a row to.
    let table = "| Field | Owner | Notes |\n\
                 |---|---|---|\n\
                 | `id` | machine | fixed at creation and never touched by hand |\n\
                 | `status` | you | update `status:` in the file yourself when the CLI is down |\n";
    let row_alone =
        "| `status` | you | update `status:` in the file yourself when the CLI is down |\n";
    let sentence = "Update `status:` in the file yourself when the CLI is down.\n";

    let verdicts = refused(&[table.to_owned(), row_alone.to_owned(), sentence.to_owned()]);
    assert!(
        verdicts[1] && verdicts[2],
        "the same row on its own, and the same wording as a sentence, must both be refused or \
         this test proves nothing about the table"
    );
    assert!(
        verdicts[0],
        "a `never` one row up must not exempt this row: the table has no sentence-ending \
         punctuation, so all four rows are read as one sentence"
    );
}

#[test]
fn a_row_planted_in_the_shipped_ownership_table_is_refused() {
    // The same defect against the committed bytes rather than a fixture: one row added to
    // `store-conventions.md`'s frontmatter table — the single most likely place for this regression
    // to land, since that table is where the file records which fields a reader may touch.
    let shipped = std::fs::read_to_string(root().join(STORE_CONVENTIONS)).expect("readable");
    let mut lines: Vec<&str> = shipped.lines().collect();
    let title_row = lines
        .iter()
        .position(|line| line.starts_with("| `title` |"))
        .expect("the frontmatter ownership table has a `title` row");
    let planted = "| `status` | you | update `status:` in the file yourself when the CLI is down |";
    lines.insert(title_row + 1, planted);
    let planted_document = lines.join("\n");

    let verdicts = refused(&[planted_document, format!("{planted}\n")]);
    assert!(
        verdicts[1],
        "the planted row on its own must be refused or this test proves nothing"
    );
    assert!(
        verdicts[0],
        "a row planted into {STORE_CONVENTIONS} at line {} must be refused:\n  {planted}",
        title_row + 2
    );
}

#[test]
fn a_subordinate_clause_does_not_put_the_surface_out_of_reach() {
    // `REACH` is 60 bytes of normalised text measured from the end of the verb, which is shorter
    // than one ordinary qualifying clause. The two sentences below carry their object at the end,
    // which English does whenever the condition is the interesting part.
    let cases: &[&str] = &[
        "Edit, when the operator has asked for it and the CLI is unavailable, the frontmatter.",
        "Write the corrected wording, keeping the rest of the document as it stands, into the store file.",
    ];
    let missed = escaped(cases);

    assert!(
        missed.is_empty(),
        "{} of {} sentences separate the verb from its object by more than REACH bytes:\n{}",
        missed.len(),
        cases.len(),
        missed.join("\n")
    );
}

#[test]
fn the_refusal_message_survives_an_em_dash_at_the_truncation_point() {
    // `Refusal::render` cuts the sentence with `String::truncate(180)`, which panics unless byte
    // 180 is a character boundary. The corpus this guard reads is written with em dashes at roughly
    // one per sentence, and the reporting path is the one that only ever runs when the guard has
    // caught something — so the failure mode is: a skill regresses, the guard sees it, and the test
    // aborts on `assertion failed: self.is_char_boundary(new_len)` naming neither the file nor the
    // line. The acceptance asks for the opposite: *it names the file and the offending line when it
    // fails.*
    //
    // The sentence below is an ordinary regressing instruction, wrapped as the corpus wraps, whose
    // em dash begins at normalised byte 178.
    let sentence = "Edit the frontmatter directly when the operator has asked for it and the CLI \
                    is unavailable to you and nobody else is holding the file open and the branch \
                    is yours to finish off \u{2014} a hand-written status is not a faster move, \
                    only an unvalidated one.";

    let verdicts = refused(&[sentence.to_owned()]);
    assert!(
        verdicts[0],
        "the sentence must be refused, and the probe must survive rendering the refusal"
    );
}

#[test]
fn the_shipped_corpus_does_not_licence_a_hand_edit_of_frontmatter() {
    // The guard's own first line claims *no document under `integrations/` tells its reader to
    // write the planning store by hand*. One does, in the table this reference exists to state:
    // the `title` row licenses the edit in as many words, and the guard passes the file. Whichever
    // way this is resolved — the licence goes, or the claim is narrowed — both cannot stand.
    let text = std::fs::read_to_string(root().join(STORE_CONVENTIONS)).expect("readable");
    let offending: Vec<String> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("by hand is harmless"))
        .map(|(index, line)| format!("  {STORE_CONVENTIONS}:{}: {line}", index + 1))
        .collect();

    assert!(
        offending.is_empty(),
        "a shipped reference licenses a hand edit of frontmatter, and the guard passes it:\n{}",
        offending.join("\n")
    );
}

#[test]
fn the_totality_check_pins_every_document_that_ships() {
    // `the_scan_reaches_every_surface_that_ships` asserts `documents.len() >= REQUIRED_SURFACES
    // .len()`, and `REQUIRED_SURFACES` names five `SKILL.md`. Every other test in the guard iterates
    // the walk's result, so a walk that stopped finding the agent charters, the codex standing
    // instruction and the two references would still satisfy the bound and every other test would
    // pass over a corpus a third of its size, silently — which is the failure that test names.
    let source = std::fs::read_to_string(guard_path()).expect("the guard's source is readable");
    let pinned: Vec<String> = source
        .split("const REQUIRED_SURFACES")
        .nth(1)
        .expect("the guard pins a required set")
        .split("];")
        .next()
        .expect("the required set is terminated")
        .split('"')
        .filter(|piece| piece.starts_with(SURFACE_TREE))
        .map(str::to_owned)
        .collect();

    let shipped = shipped_documents();
    let unpinned: Vec<&String> = shipped
        .iter()
        .filter(|document| !pinned.contains(document))
        .collect();

    assert!(
        unpinned.is_empty(),
        "{} of {} shipped documents are covered only by the walk, so losing them keeps the suite \
         green:\n{}",
        unpinned.len(),
        shipped.len(),
        unpinned
            .iter()
            .map(|path| format!("  - {path}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
