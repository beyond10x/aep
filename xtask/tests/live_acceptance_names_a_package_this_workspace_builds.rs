//! A live planning artifact's `## Acceptance` names a package `cargo` can build.
//!
//! `story:profile-and-cli-crates-named-after-aep` renamed three packages, and its commit states
//! that *"live artifacts' scope entries and body citations of the three retired paths"* were
//! rewritten through the CLI. Paths were. **Package names were not** — a `## Acceptance` is a
//! command somebody runs to decide whether a story is finished, and `cargo test -p <package>`
//! against a package the workspace no longer has does not report a failing test, it exits with
//! *"package ID specification … did not match any packages"*. The story is then unverifiable by its
//! own predicate, and the reader has to work out that the acceptance is stale rather than the code.
//!
//! Nothing else in the tree decides this. `xtask/tests/crate_paths_are_area_qualified.rs` excludes
//! `.engineering/planning/` from its corpus, and the one store-side rule it keeps —
//! `no_live_planning_artifact_scopes_a_file_by_its_pre_move_path` — reads `scope:` and reads only
//! *path* spellings. A `-p <package>` in a body is neither.
//!
//! # Why `## Acceptance` and nothing wider
//!
//! Because a package name in a body is often a **record**: `story:protocol-drive-verb` says
//! "`cargo test -p protocol-cli --test drive_cli` → 47 passed, exit 0" under a dated heading, and
//! `task:w4-3-operator-resume-ux` quotes that sentence with "quoted unedited" beside it. Those are
//! observations of a run that happened, and rewriting one would falsify it. `## Acceptance` is the
//! opposite: it is the predicate the next person evaluates, in the present tense, and a stale one
//! is a defect rather than a record. The same distinction `crate_paths_are_area_qualified.rs` draws
//! between a live `scope:` and a finished story's.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The repository root, from this crate's manifest.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

/// The statuses at which an artifact is a record of finished work rather than work still to do.
///
/// The same list `crate_paths_are_area_qualified.rs` keeps, for the same reason: a finished story's
/// acceptance describes what was run, and rewriting it would be rewriting history.
const TERMINAL: &[&str] = &[
    "implemented",
    "archived",
    "superseded",
    "rejected",
    "cleared",
    "closed",
    "done",
];

/// The section whose commands are a predicate somebody still evaluates.
const LIVE_SECTION: &str = "Acceptance";

/// Every package name the workspace manifest's member list resolves to.
fn workspace_packages(root: &Path) -> BTreeSet<String> {
    let manifest =
        std::fs::read_to_string(root.join("Cargo.toml")).expect("reading the workspace manifest");
    let members = manifest
        .split_once("members = [")
        .map(|(_, rest)| rest.split_once(']').map_or(rest, |(list, _)| list))
        .expect("the workspace manifest declares a member list");
    let mut names = BTreeSet::new();
    for line in members.lines() {
        let Some(member) = line.trim().trim_end_matches(',').strip_prefix('"') else {
            continue;
        };
        let member = member.trim_end_matches('"');
        if member.is_empty() {
            continue;
        }
        let text = std::fs::read_to_string(root.join(member).join("Cargo.toml"))
            .unwrap_or_else(|error| panic!("reading {member}/Cargo.toml: {error}"));
        if let Some(name) = package_name(&text) {
            names.insert(name.to_owned());
        }
    }
    assert!(
        names.len() > 20,
        "the member list parse found only {names:?}, so it is reading the wrong block"
    );
    names
}

/// The `[package] name` a member manifest declares, when it declares one.
fn package_name(manifest: &str) -> Option<&str> {
    let (_, rest) = manifest.split_once("[package]")?;
    rest.lines()
        .take_while(|line| !line.trim_start().starts_with('['))
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "name").then(|| value.trim().trim_matches('"'))
        })
}

/// Every package `line` names with `-p` or `--package` in a `cargo` invocation.
///
/// Anchored on `cargo` so that `mkdir -p dist` and every other `-p` in prose is not a package
/// reference, and the token is taken as written: a value carrying a `/` is a path and not a name.
fn package_references(line: &str) -> Vec<&str> {
    let Some(start) = line.find("cargo") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for flag in [" -p ", " --package "] {
        let mut at = start;
        while let Some(index) = line[at..].find(flag) {
            let after = at + index + flag.len();
            at = after;
            let Some(token) = line[after..].split_whitespace().next() else {
                continue;
            };
            let token = token.trim_matches(|character| "`\"',;)".contains(character));
            if !token.is_empty() && !token.contains('/') && !token.starts_with('-') {
                found.push(token);
            }
        }
    }
    found
}

/// The `status:` a planning document's frontmatter declares.
fn status(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n")?;
    let block = &rest[..rest.find("\n---")?];
    block
        .lines()
        .find_map(|line| line.strip_prefix("status:").map(str::trim))
}

/// Every `*.md` under the planning store, sorted.
fn planning_documents(store: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![store.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|kind| kind == "md") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The predicate decides both directions, on strings rather than on what the store happens to hold.
///
/// `AGENTS.md` invariant 15: break the guarded condition and observe the named failure. Without
/// this, the rule below would pass identically if `package_references` returned nothing at all.
#[test]
fn a_cargo_package_reference_is_read_out_of_an_acceptance_line_and_prose_is_not() {
    assert_eq!(
        package_references("`cargo test -p aep-cli --test cli` passes with the flag"),
        vec!["aep-cli"],
        "a `-p` inside a `cargo` invocation names a package"
    );
    assert_eq!(
        package_references(
            "`cargo test -p aep-driver --test routing`; `cargo test --package xtask`"
        ),
        vec!["aep-driver", "xtask"],
        "both flags, and more than one on a line"
    );
    assert!(
        package_references("mkdir -p dist/aep && cd dist").is_empty(),
        "a `-p` that is not a cargo flag names no package"
    );
    assert!(
        package_references("run `cargo build` in the checkout").is_empty(),
        "an invocation naming no package is not a reference"
    );

    let packages = workspace_packages(&repo_root());
    assert!(
        packages.contains("aep-cli"),
        "the workspace builds `aep-cli`: {packages:?}"
    );
    assert!(
        !packages.contains("protocol-cli"),
        "and no longer builds `protocol-cli`, which is what makes the rule below bite"
    );
}

/// No live artifact's `## Acceptance` names a package this workspace does not build.
#[test]
fn no_live_acceptance_names_a_package_the_workspace_does_not_build() {
    let root = repo_root();
    let packages = workspace_packages(&root);
    let store = root.join(".engineering/planning");
    let mut live = 0usize;
    let mut checked = 0usize;
    let mut findings: Vec<String> = Vec::new();

    for path in planning_documents(&store) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let state = status(&text).unwrap_or("");
        if TERMINAL.contains(&state) {
            continue;
        }
        live += 1;
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let mut section = String::new();
        for (index, line) in text.lines().enumerate() {
            if let Some(heading) = line.strip_prefix("## ") {
                section = heading.trim().to_owned();
            }
            if section != LIVE_SECTION {
                continue;
            }
            for name in package_references(line) {
                checked += 1;
                if !packages.contains(name) {
                    findings.push(format!(
                        "  {relative}:{} [{state}] — `-p {name}`, which `cargo` answers \
                         \"package ID specification `{name}` did not match any packages\"",
                        index + 1
                    ));
                }
            }
        }
    }

    assert!(
        live > 30,
        "only {live} live artifact(s) were read, so this test is reading the wrong store"
    );
    assert!(
        checked > 0,
        "no `## Acceptance` names a package at all, so the scan has stopped looking at the right \
         section"
    );
    assert!(
        findings.is_empty(),
        "{} acceptance statement(s) name a package this workspace does not build, so the story \
         cannot be verified by the command it declares:\n{}",
        findings.len(),
        findings.join("\n")
    );
}
