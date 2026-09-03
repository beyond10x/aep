//! One spelling for a crate path: `crates/<area>/<crate>`, everywhere the repository still writes.
//!
//! `story:crates-under-area-directories` moved twenty-two crates from `crates/<crate>` to
//! `crates/<area>/<crate>`. Nothing in the workspace decides whether the rest of the tree followed,
//! because every remaining path is prose or store data and no compiler reads it — so `task check`
//! was green with the claim false. These two tests are that claim, made checkable and kept.
//!
//! # What a *pre-move* path is, and why it is not "a path that does not exist"
//!
//! `crates/auth/src/passkey.rs` in `examples/` names a crate this repository never had; a check that
//! only asked *does it exist* would report it and be wrong. So the predicate here is narrower and
//! decides only what the move can be blamed for: the first component after `crates/` is the
//! directory name of a crate that **this tree still has**, one level deeper. `crates/aep-domain/…`
//! is a pre-move spelling of a file that is now at `crates/govern/aep-domain/…`;
//! `crates/govern/aep-domain/…` is not, because `govern` is not a crate name. Existence on disk is
//! not consulted: a pre-move spelling is a defect whether or not something else happens to sit there.
//!
//! The crate names are read off `crates/<area>/<crate>` rather than listed, so this file does not
//! have to be edited when an area gains a crate.
//!
//! # What is out of the corpus, and why each one is
//!
//! The story's amended `## Acceptance` names the exclusions and this list is that list, matched
//! root-relative, plus one the acceptance leaves to the sibling test below:
//!
//! | excluded | why |
//! |---|---|
//! | `CHANGELOG.md` | published sections are never rewritten; that rule predates this move |
//! | `.engineering/planning/journal.jsonl` | append-only: the record of what a command did, then |
//! | `docs/design/`, `docs/reviews/` | dated record. `docs/plan/` is **not** here: `AGENTS.md` § *Normative documents* makes accepted pages under it live, and `gap-register.md` cites current code by `file:line` |
//! | recorded `metaharness.event/1` streams | a session in a `/work/aep` sandbox, each `tool.result` carrying a `bytes` count over its own string; rewriting a path there falsifies the count, and one of the files they name never existed in this repository at all |
//! | the two lines that quote those streams | `conformance/eval/development-tests-after-the-code/case.yaml` narrates a transcript, and the blog post quotes a `churn` run from 2026-08-25 |
//! | `.engineering/planning/` | the store is written by `aep artifact`, never by an editor. Its machine-read half — `scope:`, which `aep artifact waves` compares — is asserted by `no_live_planning_artifact_scopes_a_file_by_its_pre_move_path` below. Its prose half is 298 citations across 111 documents, 190 of them in artifacts that are already implemented or archived, and rewriting those would be rewriting a finished record |

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The repository root, from this crate's manifest.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

/// Every crate directory name under `crates/<area>/`, which is the set of pre-move first components.
fn crate_directory_names(root: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let crates = root.join("crates");
    for area in std::fs::read_dir(&crates).expect("crates/ is readable") {
        let area = area.expect("a crates/ entry").path();
        if !area.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&area)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", area.display()))
        {
            let entry = entry.expect("an area entry").path();
            if entry.join("Cargo.toml").is_file() {
                names.insert(
                    entry
                        .file_name()
                        .expect("a directory has a name")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    assert!(
        names.len() >= 20,
        "only {} crate directories found, so the predicate below is reading the wrong tree",
        names.len()
    );
    names
}

/// The pre-move path `token` names, when it names one.
fn pre_move_path<'a>(token: &'a str, names: &BTreeSet<String>) -> Option<&'a str> {
    let rest = token.strip_prefix("crates/")?;
    let head = rest.split('/').next().unwrap_or(rest);
    names.contains(head).then_some(token)
}

/// Every `crates/…` token in `text`, in the order they appear.
///
/// A token runs to the first character no path uses. Trailing punctuation an English sentence adds
/// — a full stop, a comma, a closing bracket — is trimmed, so `(crates/aep-domain/src/lib.rs),`
/// yields the path and not the punctuation.
fn crate_tokens(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("crates/") {
        let candidate = &rest[at..];
        let end = candidate
            .find(|character: char| {
                !(character.is_ascii_alphanumeric()
                    || character == '/'
                    || character == '.'
                    || character == '_'
                    || character == '-')
            })
            .unwrap_or(candidate.len());
        let token = candidate[..end].trim_end_matches(['.', ',', '-']);
        if !token.is_empty() {
            found.push(token);
        }
        rest = &candidate[end.max(1)..];
    }
    found
}

/// The area directories, so a path that is already area-qualified can be told from a defect.
const AREAS: &[&str] = &["govern", "plan", "drive", "observe", "profile", "edge"];

/// Whether `token` is already `crates/<area>/…`.
fn area_qualified_path(token: &str) -> bool {
    token
        .strip_prefix("crates/")
        .and_then(|rest| rest.split('/').next())
        .is_some_and(|head| AREAS.contains(&head))
}

/// Files exact-matched out of the corpus, root-relative.
const EXCLUDED_FILES: &[&str] = &[
    // Published sections are never rewritten.
    "CHANGELOG.md",
    // Append-only: what a command did, when it did it.
    ".engineering/planning/journal.jsonl",
    // Narrates the transcript beside it, event for event.
    "conformance/eval/development-tests-after-the-code/case.yaml",
    // Quotes a `churn` run from 2026-08-25, output and all.
    "website/blog/2026-08-25-1605-a-repository-that-already-exists.md",
];

/// Directories whose contents are out of the corpus, root-relative.
const EXCLUDED_PREFIXES: &[&str] = &[
    // Dated record: a design or a review says what was true when it was written.
    "docs/design/",
    "docs/reviews/",
    // Recorded `metaharness.event/1` streams and the matrices assembled from them byte for byte.
    "crates/edge/protocol-cli/fixtures/eval-",
    // The store: written by `aep artifact`, never by an editor. See the module documentation.
    ".engineering/planning/",
];

/// Whether this tracked path is out of the corpus, and why is in the two lists above.
fn excluded(relative: &str) -> bool {
    if EXCLUDED_FILES.contains(&relative) || relative == file!() {
        return true;
    }
    if EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| relative.starts_with(prefix))
    {
        return true;
    }
    // Recorded streams, the two other places they are committed.
    let jsonl = Path::new(relative)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"));
    if relative.starts_with("crates/observe/trace-spec/tests/fixtures/") && jsonl {
        return true;
    }
    jsonl
        && relative.starts_with("conformance/eval/")
        && relative.ends_with("/transcript.jsonl")
        && relative.matches('/').count() == 3
}

/// Every tracked file the claim covers, root-relative and sorted.
///
/// `git ls-files` rather than a directory walk: tracked is what a reader, a reviewer and every tool
/// see, and it is the set `.gitignore` has already taken `target/` and `website/build/` out of.
/// Unlike `rg`'s default, dot-directories are **in** — `.engineering/checks/` and `.github/` name
/// paths too, and a rule that skipped them would have missed the gate's own checker.
fn acceptance_corpus(root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .expect("running git ls-files");
    assert!(output.status.success(), "git ls-files failed");
    let listing = String::from_utf8(output.stdout).expect("git ls-files prints UTF-8 paths");
    let mut files: Vec<String> = listing
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .filter(|entry| !excluded(entry))
        .map(str::to_owned)
        .collect();
    files.sort();
    files
}

/// The frontmatter of a planning document, when it has one.
fn frontmatter(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// The value of the first `key:` line in `block`.
fn field<'a>(block: &'a str, key: &str) -> Option<&'a str> {
    block.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?.strip_prefix(':')?;
        Some(rest.trim())
    })
}

/// The statuses at which an artifact is a record of finished work rather than work still to do.
///
/// A finished story's `scope:` describes where it landed, and rewriting it would be rewriting
/// history. Everything else is work somebody may still pick up, and its scope is read by
/// `aep artifact waves` before that happens.
const TERMINAL: &[&str] = &[
    "implemented",
    "archived",
    "superseded",
    "rejected",
    "cleared",
    "closed",
    "done",
];

/// A live planning artifact whose machine-read `scope:` still names a file by its pre-move path.
///
/// `aep artifact waves` decides which stories may be implemented at once by comparing `scope:` path
/// strings, and it says in its own documentation that "**Nothing is normalised**… nothing here
/// normalises `crates/x/src/lib.rs` to `crates/x`" (`crates/edge/protocol-cli/src/planning.rs`). So
/// a draft that still says `crates/aep-domain/src/artifact.rs` cannot collide with a story written
/// after the move that lands on `crates/govern/aep-domain/src/artifact.rs` — the same file, two
/// spellings, no collision reported, both placed in one wave. That is the exact failure the verb
/// exists to prevent, and the move introduced it into fifty-one artifacts at once.
#[test]
fn no_live_planning_artifact_scopes_a_file_by_its_pre_move_path() {
    let root = repo_root();
    let names = crate_directory_names(&root);
    let store = root.join(".engineering/planning");
    let mut findings: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut scanned = 0usize;

    let mut pending = vec![store.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", directory.display()))
        {
            let path = entry.expect("a store entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_none_or(|it| it != "md") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("a readable store document");
            let Some(block) = frontmatter(&text) else {
                continue;
            };
            scanned += 1;
            let status = field(block, "status").unwrap_or("");
            if TERMINAL.contains(&status) {
                continue;
            }
            let stale: Vec<String> = block
                .lines()
                .filter_map(|line| line.trim().trim_start_matches("- ").strip_prefix("path:"))
                .map(str::trim)
                .filter_map(|value| pre_move_path(value, &names).map(str::to_owned))
                .collect();
            if !stale.is_empty() {
                let id = field(block, "id").unwrap_or("<no id>");
                findings.insert(format!("{id} ({status})"), stale);
            }
        }
    }

    assert!(
        scanned > 100,
        "only {scanned} planning documents were read, so this test is asserting nothing"
    );
    let entries: usize = findings.values().map(Vec::len).sum();
    let report: Vec<String> = findings
        .iter()
        .map(|(artifact, paths)| format!("  {artifact}: {}", paths.join(", ")))
        .collect();
    assert!(
        findings.is_empty(),
        "{} live planning artifact(s) carry {entries} `scope:` path(s) that name a crate by the \
         directory it no longer sits in, so `aep artifact waves` cannot collide them with any \
         story scoped after the move:\n{}",
        findings.len(),
        report.join("\n")
    );
}

/// No tracked file spells a crate path the way the tree spelled it before the move.
///
/// The corpus is `git ls-files` — what is tracked, which is what a reader and every tool see —
/// minus the exclusions the module documentation lists and argues for. A finding is a spelling, not
/// a broken link: `crates/aep-domain/src/artifact.rs` names a file that is now one directory
/// deeper, and a reader who follows it lands nowhere.
#[test]
fn no_tracked_file_spells_a_crate_path_the_pre_move_way() {
    let root = repo_root();
    let names = crate_directory_names(&root);
    let mut findings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut scanned = 0usize;
    let mut area_qualified = 0usize;

    for relative in acceptance_corpus(&root) {
        let Ok(text) = std::fs::read_to_string(root.join(&relative)) else {
            continue;
        };
        scanned += 1;
        for token in crate_tokens(&text) {
            if let Some(stale) = pre_move_path(token, &names) {
                findings
                    .entry(relative.clone())
                    .or_default()
                    .insert(stale.to_owned());
            } else if area_qualified_path(token) {
                area_qualified += 1;
            }
        }
    }

    assert!(
        scanned > 500,
        "only {scanned} files were read, so this test is asserting nothing"
    );
    // Anti-vacuity from the other side: the corpus has to contain the paths this rule is about. A
    // tokeniser that stopped matching, or a corpus filter that excluded the tree, would leave the
    // findings empty and the test green while checking nothing.
    assert!(
        area_qualified > 0,
        "the scan collected no `crates/<area>/<crate>` path at all, so it has stopped looking at \
         the right files"
    );

    let total: usize = findings.values().map(BTreeSet::len).sum();
    let report: Vec<String> = findings
        .iter()
        .map(|(file, paths)| {
            format!(
                "  {file}: {}",
                paths.iter().cloned().collect::<Vec<_>>().join(", ")
            )
        })
        .collect();
    assert!(
        findings.is_empty(),
        "{total} path(s) in {} tracked file(s) still name a crate by the directory it no longer \
         sits in:\n{}",
        findings.len(),
        report.join("\n")
    );
}
