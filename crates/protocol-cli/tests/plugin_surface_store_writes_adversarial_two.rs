//! Second adversarial pass against `tests/plugin_surface_store_writes.rs`.
//!
//! Same method as `plugin_surface_store_writes_adversarial.rs`: the guard's committed source is
//! compiled at test time, with its `//!` header demoted to `//`, and its own `refusals` is driven
//! from stdin. Nothing here forks the guard's decision procedure; a change to the guard is picked
//! up on the next run.
//!
//! Where the first pass attacked the guard's coverage, this one attacks its **grammar**. Every case
//! below is one English sentence a person rewriting a shipped skill could plausibly write, and the
//! acceptance the story is held to does not carve any of them out:
//!
//! > A test enumerates every installed skill under `integrations/` and refuses one whose text
//! > instructs a direct write to a planning-store file — editing frontmatter, patching a body, or
//! > writing `status:` by hand.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The guard's committed source.
fn guard_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/plugin_surface_store_writes.rs")
}

/// The probe binary: the guard's own `refusals`, driven from stdin.
///
/// Its own scratch directory, so this binary and the first pass's never race on one build.
fn probe() -> &'static Path {
    static PROBE: OnceLock<PathBuf> = OnceLock::new();
    PROBE.get_or_init(|| {
        let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store-write-adversary-two");
        std::fs::create_dir_all(&scratch).expect("the scratch directory is creatable");

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
        match refusals("probe.md", chunk).first() {
            None => println!("PASS"),
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

/// The cases the guard let through, rendered for a failure message.
fn escaped(cases: &[&str]) -> Vec<String> {
    let documents: Vec<String> = cases.iter().map(|case| (*case).to_owned()).collect();
    refused(&documents)
        .into_iter()
        .zip(cases.iter())
        .filter(|(refused, _)| !refused)
        .map(|(_, case)| format!("  - {case}"))
        .collect()
}

#[test]
fn the_comma_the_guard_s_own_header_quotes_still_swallows_the_instruction() {
    // `plugin_surface_store_writes.rs:42` names the sentence this whole clause-window design was
    // built for, verbatim and with a comma:
    //
    //   > A sentence-wide exemption is a sentence-wide hole: *do not ask the operator, edit the
    //   > frontmatter* was excused by its own first clause for as long as the window was the whole
    //   > sentence.
    //
    // It is still excused by its own first clause. `CLAUSE_BREAKS`
    // (`plugin_surface_store_writes.rs:402`) deliberately omits the comma, so the clause holding
    // `edit` opens at byte zero and the `not` in *do not ask* is inside it. The guard's own case for
    // this, `a_prohibition_does_not_exempt_the_clause_after_it` (`:917`), silently substitutes a
    // colon — `"Do not ask the operator: edit the frontmatter."` — which is the one punctuation mark
    // that makes the sentence work.
    //
    // The comma is not an exotic way to write this. It is how the header wrote it.
    let cases: &[&str] = &[
        "Do not ask the operator, edit the frontmatter.",
        "Never open a pull request for this, edit the frontmatter directly.",
        "The operator has not asked, patch the body in place.",
        "You cannot reach the CLI, set `status: active` in the file yourself.",
        "There is no need to stop, update the `status:` field yourself.",
    ];
    let missed = escaped(cases);

    assert!(
        missed.is_empty(),
        "{} of {} sentences are excused by a prohibition of something else, one comma away:\n{}",
        missed.len(),
        cases.len(),
        missed.join("\n")
    );
}

#[test]
fn one_word_between_instead_of_and_the_cli_restores_the_exemption() {
    // `governs_sanctioned` (`plugin_surface_store_writes.rs:671`) voids an exemption whose object is
    // the CLI, and it decides that by requiring **zero** intervening words:
    //
    //   `sentence[after..found].split_whitespace().count() == 0`
    //
    // So *instead of `protocol artifact body`, edit the file* is refused — the guard tests exactly
    // that string at `:1078` — and *instead of **using** `protocol artifact body`, edit the file* is
    // not. The gerund is the ordinary way to write the sentence; the guard's own corpus writes
    // *rather than calling*, *instead of asking* elsewhere in the repository. This is not the named
    // `ROUTING` blacklist limit: the marker is present, matched, and then discarded on a
    // whitespace count.
    let cases: &[&str] = &[
        "Instead of using `protocol artifact body`, edit the file.",
        "Instead of using `protocol artifact body`, edit the frontmatter.",
        "Rather than calling `protocol artifact move`, set `status: active` in the frontmatter.",
        "Instead of shelling out to `protocol artifact body`, patch the body in place.",
    ];
    let missed = escaped(cases);

    // The zero-word forms are refused, so the difference below is the intervening word and nothing
    // else.
    let baseline = [
        "Instead of `protocol artifact body`, edit the file.".to_owned(),
        "Instead of `protocol artifact body`, edit the frontmatter.".to_owned(),
    ];
    assert!(
        refused(&baseline).into_iter().all(|refused| refused),
        "the adjacent forms must be refused or this test proves nothing"
    );

    assert!(
        missed.is_empty(),
        "{} of {} sentences route around the CLI and are exempted because one word sits between \
         the marker and the CLI's name:\n{}",
        missed.len(),
        cases.len(),
        missed.join("\n")
    );
}

#[test]
fn an_instruction_that_names_its_object_before_its_verb_is_refused() {
    // `instruction` (`plugin_surface_store_writes.rs:757`) only ever looks for a store surface
    // **after** the verb: `find_word(sentence, surface, end)`. Every case below carries the surface
    // in front of the verb, which is what English does in the copular and cleft moods a
    // documentation writer reaches for constantly — *the frontmatter is yours to edit* is the same
    // instruction as *edit the frontmatter*, and one of them is invisible.
    //
    // Nothing in the guard's header names this as a limit. The four limits it does name are
    // non-markdown surfaces, the plural `bodies`, the `ROUTING` blacklist and paraphrase; the
    // surface here is `frontmatter` and `status:`, which are in `STORE_SURFACES`, spelled out.
    let cases: &[&str] = &[
        "The frontmatter is yours to edit.",
        "The `status:` field is the one you set by hand.",
        "It is the frontmatter you edit, and nothing else.",
        "The frontmatter is what you change when a review lands.",
    ];
    let missed = escaped(cases);

    // The same instructions with the object behind the verb are refused, so the difference is word
    // order and nothing else.
    let baseline = [
        "Edit the frontmatter.".to_owned(),
        "Set the `status:` field by hand.".to_owned(),
    ];
    assert!(
        refused(&baseline).into_iter().all(|refused| refused),
        "the verb-first forms must be refused or this test proves nothing"
    );

    assert!(
        missed.is_empty(),
        "{} of {} instructions put their object in front of their verb and are invisible:\n{}",
        missed.len(),
        cases.len(),
        missed.join("\n")
    );
}

#[test]
fn a_horizontal_rule_ends_a_block_as_starts_block_says_it_does() {
    // `starts_block`'s own doc comment (`plugin_surface_store_writes.rs:536`) claims it recognises
    // "a heading, a list item, a table row, a fence, a blockquote **or a rule**". It recognises no
    // rule: `---`, `___` and a setext underline all fall through, so a prohibition on one side of a
    // horizontal rule reaches across it and excuses the instruction on the other.
    //
    // `---` is not hypothetical in this corpus — it opens and closes the YAML frontmatter of every
    // one of the five shipped `SKILL.md`, and `a_prohibition_does_not_exempt_the_block_after_it`
    // (`:996`) tests four block kinds and not this one.
    // `- - -` and `* * *` are caught, but by accident rather than by rule: the first is read as a
    // bullet, and the second vanishes when `normalize` strips `*`, leaving a line that is empty.
    let cases: [(&str, &str); 3] = [
        ("---", "a horizontal rule"),
        ("___", "an underscore rule"),
        ("===", "a setext heading underline"),
    ];
    let mut leaked: Vec<String> = Vec::new();
    for (rule, what) in cases {
        let document = format!("Never edit a store file directly\n{rule}\nedit the frontmatter\n");
        if !refused(&[document])[0] {
            leaked.push(format!("  - {what} ({rule:?}) does not end the block"));
        }
    }

    // The block kinds the guard does test are refused, so the difference is the rule and nothing
    // else.
    let baseline = ["Never edit a store file directly\n# edit the frontmatter\n".to_owned()];
    assert!(
        refused(&baseline).into_iter().all(|refused| refused),
        "a heading must end the block or this test proves nothing"
    );

    assert!(
        leaked.is_empty(),
        "{} of {} rules let a prohibition reach across them, though `starts_block` documents \
         itself as handling a rule:\n{}",
        leaked.len(),
        cases.len(),
        leaked.join("\n")
    );
}

#[test]
fn where_a_hard_wrap_falls_does_not_change_the_verdict() {
    // The acceptance says the check is **over content**. It is not: `opens_a_sentence`
    // (`plugin_surface_store_writes.rs:570`) makes the verdict depend on where the line breaks fall,
    // and a capitalised proper noun at the head of a wrapped line splits a prohibition away from the
    // clause it was protecting. Each pair below is the same sentence, byte for byte, wrapped two
    // ways; the guard answers differently.
    //
    // The corpus this guard reads wraps at 100 columns and is full of capitalised proper nouns that
    // land at a line head — `Edit`, `Write`, `NotebookEdit`, `YAML`, `CLI`, `Codex`. A reflow by a
    // markdown formatter is enough to redden a green tree, which is the shape of test somebody
    // eventually deletes.
    let pairs: [(&str, &str); 3] = [
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
    ];

    let documents: Vec<String> = pairs
        .iter()
        .flat_map(|(one_line, wrapped)| [(*one_line).to_owned(), (*wrapped).to_owned()])
        .collect();
    let verdicts = refused(&documents);

    let disagreed: Vec<String> = pairs
        .iter()
        .enumerate()
        .filter(|(index, _)| verdicts[index * 2] != verdicts[index * 2 + 1])
        .map(|(index, (one_line, _))| {
            format!(
                "  - unwrapped: {}, wrapped: {} — {one_line}",
                if verdicts[index * 2] {
                    "refused"
                } else {
                    "passed"
                },
                if verdicts[index * 2 + 1] {
                    "refused"
                } else {
                    "passed"
                },
            )
        })
        .collect();

    assert!(
        disagreed.is_empty(),
        "{} of {} sentences get a different verdict depending only on where the line breaks:\n{}",
        disagreed.len(),
        pairs.len(),
        disagreed.join("\n")
    );
}

#[test]
fn routing_around_the_writer_is_refused_however_the_reason_is_worded() {
    // The guard's header calls `ROUTING` its one blacklist and says a regression that invents a new
    // phrase passes. That is a statement about the implementation; the acceptance makes no such
    // reservation, and none of the sentences below invents anything — each gives an ordinary reason
    // to go round the CLI, in the plainest words available, and each is exempted by the very fact
    // that it names the CLI it is telling the reader to avoid.
    //
    // Measured over the whole corpus rather than as fixtures: planted at each of the 2278 line
    // positions of the seventeen shipped documents, every one of these escapes at every position.
    let cases: &[&str] = &[
        "When `protocol artifact body` fails, edit the frontmatter.",
        "If `protocol artifact` is broken, edit the frontmatter.",
        "`protocol artifact` is not on PATH here, so update the frontmatter.",
        "`protocol artifact body` is overkill for a typo; edit the story file.",
        "For a one-word change, forget `protocol artifact body` and edit the file.",
    ];
    let missed = escaped(cases);

    assert!(
        missed.is_empty(),
        "{} of {} sentences name the CLI in order to route around it and are exempted for naming \
         it:\n{}",
        missed.len(),
        cases.len(),
        missed.join("\n")
    );
}
