//! Security review of the whole store-pack feature: integrity conformance checks for the four
//! invariants the operator named, across the units that built them.
//!
//! 1. No record lost.
//! 2. Digest verification: every pack read verifies each record's SHA-256 and the pack's
//!    content-addressed name, no reader serves bytes whose digest was not checked, and `validate`'s
//!    provenance check cannot be skipped by any environment.
//! 3. Path traversal: `pack` and `unpack` read and write only inside `.engineering` and never
//!    through a symbolic link.
//! 4. A redacted literal is never re-exposed, by any copy or any command output.
//!
//! Every fixture is a real store built by the real verbs in a scratch Git repository under the
//! test target directory; tampering is done to the bytes a pack file holds, as a hand edit or a
//! bad merge resolution would leave them.

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use aep_backend_markdown::pack;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn relative(from: &Path, to: &Path) -> String {
    let base: Vec<_> = from.components().collect();
    let target: Vec<_> = to.components().collect();
    let shared = base
        .iter()
        .zip(&target)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec![".."; base.len() - shared];
    parts.extend(
        target[shared..]
            .iter()
            .map(|component| component.as_os_str().to_str().expect("a printable path")),
    );
    parts.join("/")
}

fn scratch_root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&root).expect("the target scratch is writable");
    root.canonicalize().expect("the target scratch resolves")
}

fn project_file(project: &Path, version: &str) -> String {
    format!(
        "version: {version}\nplanning_scope: store-pack-security\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
        relative(&project.join(".engineering"), &repository())
    )
}

fn git(project: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=tester",
            "-c",
            "user.email=tester@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(project)
        .env_remove("GIT_CEILING_DIRECTORIES")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {}: {}{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn commit(project: &Path, message: &str) {
    git(project, &["add", "--all"]);
    git(
        project,
        &[
            "commit",
            "--quiet",
            "--no-verify",
            "--allow-empty",
            "-m",
            message,
        ],
    );
}

fn aep_with(project: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aep"));
    command
        .args(args)
        .current_dir(project)
        .env("AEP_ACTOR", "human:tester")
        .env_remove("GIT_CEILING_DIRECTORIES");
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("the binary runs")
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `aep plan artifact <args>`.
fn artifact(project: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    aep_with(project, &full, &[])
}

/// `aep plan artifact <args>`, required to succeed.
fn ok(project: &Path, args: &[&str]) -> String {
    let output = artifact(project, args);
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// `aep plan store <args>`.
fn store(project: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "store"];
    full.extend_from_slice(args);
    aep_with(project, &full, &[])
}

/// `aep plan store <args>`, required to succeed.
fn store_ok(project: &Path, args: &[&str]) -> String {
    let output = store(project, args);
    assert!(
        output.status.success(),
        "`store {}` failed: {}",
        args.join(" "),
        said(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn engineering(project: &Path) -> PathBuf {
    project.join(".engineering")
}

fn packs_directory(project: &Path) -> PathBuf {
    engineering(project).join("packs")
}

/// The pack file names, sorted.
#[allow(clippy::case_sensitive_file_extension_comparisons)] // A pack's extension is exactly `.pack`.
fn packs(project: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(packs_directory(project))
        .map(|entries| {
            entries
                .map(|entry| {
                    entry
                        .expect("an entry")
                        .file_name()
                        .into_string()
                        .expect("utf-8")
                })
                .filter(|name| name.ends_with(".pack"))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

fn read_pack(project: &Path, file: &str) -> pack::Pack {
    pack::Pack::parse(&std::fs::read(packs_directory(project).join(file)).expect("reads"))
        .expect("the pack parses")
}

/// The pack file holding `path`.
fn pack_holding(project: &Path, path: &str) -> String {
    packs(project)
        .into_iter()
        .find(|file| read_pack(project, file).get(path).is_some())
        .unwrap_or_else(|| panic!("a pack holds {path}"))
}

/// Every file under `directory` (`.git` excluded), with its bytes.
fn files(directory: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, directory: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.map(|entry| entry.expect("an entry")) {
            let path = entry.path();
            if entry.file_name() == ".git" {
                continue;
            }
            let kind = entry.file_type().expect("a type");
            if kind.is_dir() {
                walk(base, &path, found);
            } else if kind.is_symlink() {
                found.push((
                    path.strip_prefix(base)
                        .expect("below")
                        .display()
                        .to_string(),
                    std::fs::read_link(&path)
                        .expect("a link reads")
                        .display()
                        .to_string()
                        .into_bytes(),
                ));
            } else {
                found.push((
                    path.strip_prefix(base)
                        .expect("below")
                        .display()
                        .to_string(),
                    std::fs::read(&path).expect("reads"),
                ));
            }
        }
    }
    let mut found = Vec::new();
    walk(directory, directory, &mut found);
    found.sort();
    found
}

fn fresh(name: &str) -> PathBuf {
    let directory = scratch_root().join(format!("store-pack-security-{name}"));
    if directory.exists() || directory.is_symlink() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch is removable");
    }
    std::fs::create_dir_all(engineering(&directory)).expect("writable");
    git(&directory, &["init", "--quiet"]);
    std::fs::write(
        engineering(&directory).join("project.yaml"),
        project_file(&directory, "aep.project/5"),
    )
    .expect("writable");
    directory
}

fn build_story(project: &Path, name: &str, title: &str, rungs: &[&str]) {
    ok(project, &["new", "story", name, "--title", title]);
    for rung in rungs {
        ok(project, &["move", &format!("story:{name}"), "--to", rung]);
    }
}

/// A committed `aep.project/5` store: `closed-a` (implemented, one evidence record), `closed-b`
/// (archived) and `open-c` (active).
fn loose_store(name: &str) -> PathBuf {
    let project = fresh(name);
    build_story(
        &project,
        "closed-b",
        "Closed B",
        &["proposed", "active", "archived"],
    );
    build_story(&project, "closed-a", "Closed A", &["proposed", "active"]);
    ok(
        &project,
        &[
            "evidence",
            "story:closed-a",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-09-28T10:00:00Z",
        ],
    );
    ok(&project, &["move", "story:closed-a", "--to", "implemented"]);
    build_story(&project, "open-c", "Open C", &["proposed", "active"]);
    commit(&project, "a loose store");
    project
}

/// [`loose_store`], packed by `aep plan store pack` and committed.
fn packed_store(name: &str) -> PathBuf {
    let project = loose_store(name);
    store_ok(&project, &["pack"]);
    commit(&project, "pack the closed artifacts");
    project
}

/// `bytes` with the one occurrence of `from` replaced by `to`, which has the same length.
fn swap_same_length(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len(), "a same-length edit");
    let text = String::from_utf8(bytes.to_vec()).expect("utf-8");
    assert_eq!(
        text.matches(from).count(),
        1,
        "`{from}` occurs once in: {text}"
    );
    text.replacen(from, to, 1).into_bytes()
}

// ---------------------------------------------------------------------------------------------
// The store-pack invariant on digests: digest verification on every read.
// ---------------------------------------------------------------------------------------------

/// A record's bytes changed in place in its pack, its header digest and the pack's file name left
/// as they were: the record no longer matches the digest that names it. A read verb must verify
/// the digest before it serves the bytes, so `show` and `list` refuse (or at least never answer
/// with the changed text).
#[test]
fn readers_never_serve_a_packed_record_whose_bytes_fail_their_digest() {
    let project = packed_store("reader-digest");
    let file = pack_holding(&project, "planning/story/closed-a.md");
    let location = packs_directory(&project).join(&file);
    let original = std::fs::read(&location).expect("reads");
    let tampered = swap_same_length(&original, "title: Closed A", "title: Forged A");
    std::fs::write(&location, &tampered).expect("writable");
    let record = pack::Pack::parse(&tampered)
        .expect("a same-length edit still parses")
        .get("planning/story/closed-a.md")
        .expect("held")
        .clone();
    assert!(
        !record.digest_matches(),
        "precondition: the digest no longer matches"
    );

    for args in [
        vec!["show", "story:closed-a"],
        vec!["show", "story:closed-a", "--format", "json"],
        vec!["list"],
    ] {
        let output = artifact(&project, &args);
        let text = said(&output);
        assert!(
            !text.contains("Forged A"),
            "`{}` served bytes that fail their digest (exit {:?}): {text}",
            args.join(" "),
            output.status.code()
        );
    }
}

/// A pack whose records all match their digests, held under a file name that is not its content
/// digest (a record rewritten and the old name kept). A read must verify the content-addressed
/// name before it serves a record.
#[test]
fn readers_never_serve_a_record_from_a_pack_whose_name_is_not_its_content_digest() {
    let project = packed_store("reader-name");
    let file = pack_holding(&project, "planning/story/closed-a.md");
    let location = packs_directory(&project).join(&file);
    let parsed = pack::Pack::parse(&std::fs::read(&location).expect("reads")).expect("parses");
    let records: Vec<(String, Vec<u8>)> = parsed
        .records()
        .iter()
        .map(|record| {
            let bytes = if record.path() == "planning/story/closed-a.md" {
                swap_same_length(record.bytes(), "title: Closed A", "title: Forged A")
            } else {
                record.bytes().to_vec()
            };
            (record.path().to_owned(), bytes)
        })
        .collect();
    let rendered = pack::render(&records).expect("renders");
    std::fs::write(&location, &rendered).expect("writable");
    assert_ne!(
        format!("{}.pack", pack::name(&rendered)),
        file,
        "precondition: the file name is not the content digest"
    );

    for args in [vec!["show", "story:closed-a"], vec!["list"]] {
        let output = artifact(&project, &args);
        let text = said(&output);
        assert!(
            !text.contains("Forged A"),
            "`{}` served a record from a pack whose name is not its digest (exit {:?}): {text}",
            args.join(" "),
            output.status.code()
        );
    }
}

/// `unpack` rewrites every pack that held a restored record from the records it keeps. A kept
/// record whose bytes fail their digest must not come out of that rewrite under a fresh digest that
/// matches: the rewrite would turn a record every check refuses into one the digest check admits.
#[test]
fn unpack_never_rewrites_a_record_that_fails_its_digest_into_one_that_passes() {
    let project = packed_store("unpack-launder");
    let file = pack_holding(&project, "planning/story/closed-b.md");
    assert_eq!(
        file,
        pack_holding(&project, "planning/story/closed-a.md"),
        "precondition: one pack holds both closed stories"
    );
    let location = packs_directory(&project).join(&file);
    let original = std::fs::read(&location).expect("reads");
    let tampered = swap_same_length(&original, "title: Closed B", "title: Forged B");
    std::fs::write(&location, &tampered).expect("writable");

    let output = store(&project, &["unpack", "story:closed-a"]);
    let text = said(&output);
    for name in packs(&project) {
        for record in read_pack(&project, &name).records() {
            let forged = String::from_utf8_lossy(record.bytes()).contains("Forged B");
            assert!(
                !(forged && record.digest_matches()),
                "`unpack story:closed-a` (exit {:?}) re-signed the tampered closed-b in {name}: \
                 its digest now matches bytes no commit ever held: {text}",
                output.status.code()
            );
        }
    }
}

/// `validate`'s provenance check must not be skippable by the environment. With
/// `GIT_CEILING_DIRECTORIES` naming the project directory, Git does not look into the repository
/// from `.engineering`; `aep plan store pack` clears that variable and packs, and the readers serve
/// the packed records, so `validate` has to hold the packs to Git all the same.
#[test]
fn git_ceiling_directories_does_not_skip_pack_provenance() {
    let project = packed_store("ceiling");
    let file = pack_holding(&project, "planning/story/closed-a.md");
    let location = packs_directory(&project).join(&file);
    let parsed = pack::Pack::parse(&std::fs::read(&location).expect("reads")).expect("parses");
    let forged: Vec<(String, Vec<u8>)> = parsed
        .records()
        .iter()
        .map(|record| {
            let bytes = if record.path() == "planning/story/closed-a.md" {
                swap_same_length(record.bytes(), "title: Closed A", "title: Forged A")
            } else {
                record.bytes().to_vec()
            };
            (record.path().to_owned(), bytes)
        })
        .collect();
    let rendered = pack::render(&forged).expect("renders");
    std::fs::remove_file(&location).expect("removable");
    std::fs::write(
        packs_directory(&project).join(pack::file_name(&pack::name(&rendered))),
        &rendered,
    )
    .expect("writable");
    commit(&project, "a pack no loose file ever held");

    let control = artifact(&project, &["validate"]);
    assert_eq!(
        control.status.code(),
        Some(1),
        "precondition: the forged pack is refused: {}",
        said(&control)
    );
    assert!(said(&control).contains("[pack_record_not_committed_bytes]"));

    let ceiling = project.display().to_string();
    let env = [("GIT_CEILING_DIRECTORIES", ceiling.as_str())];
    let shown = aep_with(
        &project,
        &["plan", "artifact", "show", "story:closed-a"],
        &env,
    );
    assert!(
        said(&shown).contains("Forged A"),
        "precondition: the readers serve the forged record under the variable: {}",
        said(&shown)
    );
    let output = aep_with(&project, &["plan", "artifact", "validate"], &env);
    let text = said(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "GIT_CEILING_DIRECTORIES={ceiling} turns provenance off and the forged pack validates: \
         {text}"
    );
    assert!(text.contains("[pack_record_not_committed_bytes]"), "{text}");
}

// ---------------------------------------------------------------------------------------------
// The store-pack invariant on paths: no read or write outside `.engineering`, never through a symbolic link.
// ---------------------------------------------------------------------------------------------

/// `.engineering/planning` replaced in the working tree by a symbolic link to a directory outside
/// the repository that holds the same files. `pack` must not follow the link: it neither packs the
/// files behind it nor removes them.
#[test]
fn pack_never_follows_a_symbolically_linked_planning_directory() {
    let project = loose_store("linked-planning");
    let outside = scratch_root().join("store-pack-security-linked-planning-outside");
    if outside.exists() {
        std::fs::remove_dir_all(&outside).expect("removable");
    }
    std::fs::rename(engineering(&project).join("planning"), &outside).expect("movable");
    symlink(&outside, engineering(&project).join("planning")).expect("links");
    let before = files(&outside);
    assert!(
        !before.is_empty(),
        "precondition: the linked directory holds the stories"
    );

    let output = store(&project, &["pack"]);
    let text = said(&output);
    assert_eq!(
        files(&outside),
        before,
        "`pack` (exit {:?}) removed or changed files behind the symbolic link \
         .engineering/planning: {text}",
        output.status.code()
    );
    let packed: Vec<String> = packs(&project)
        .iter()
        .flat_map(|name| {
            read_pack(&project, name)
                .records()
                .iter()
                .filter(|record| record.path().starts_with("planning/"))
                .map(|record| record.path().to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        packed.is_empty(),
        "`pack` packed documents read through the symbolic link: {packed:?}: {text}"
    );
}

/// No id argument and no `--from` argument reaches outside the grammar: each is refused and the
/// tree is left as it was.
#[test]
fn unpack_refuses_ids_and_pack_names_outside_the_grammar_and_writes_nothing() {
    let project = packed_store("unpack-arguments");
    let before = files(&project);
    for args in [
        vec!["unpack", "story:../../outside"],
        vec!["unpack", "../planning:closed-a"],
        vec!["unpack", "story:closed-a/../../x"],
        vec!["unpack", "story:.hidden"],
        vec!["unpack", "story:closed-a", "--from", "../../../outside"],
        vec!["unpack", "story:closed-a", "--from", ""],
    ] {
        let output = store(&project, &args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "`{}` is refused: {}",
            args.join(" "),
            said(&output)
        );
        assert_eq!(
            files(&project),
            before,
            "`{}` wrote nothing",
            args.join(" ")
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The store-pack invariant on redaction: a redacted literal is never re-exposed.
// ---------------------------------------------------------------------------------------------

const LITERAL: &str = "Zorblax";

fn redact(project: &Path) {
    ok(
        project,
        &[
            "redact",
            "review-result:looked",
            "--literal",
            LITERAL,
            "--with",
            "customer A",
            "--reason",
            "names a customer",
        ],
    );
}

/// A committed store with `story:subject` (draft), an archived `review-result:looked` whose body
/// holds [`LITERAL`], and an archived `story:<filler>`.
fn review_store(name: &str, filler: &str) -> PathBuf {
    let project = fresh(name);
    let body = scratch_root().join(format!("store-pack-security-{name}-body.md"));
    std::fs::write(&body, format!("## Verdict\n\n{LITERAL} approved it.\n")).expect("writable");
    ok(&project, &["new", "story", "subject", "--title", "Subject"]);
    ok(
        &project,
        &[
            "new",
            "review-result",
            "looked",
            "--title",
            "Looked at it",
            "--relate",
            "reviews:story:subject",
            "--from",
            body.to_str().expect("printable"),
        ],
    );
    ok(
        &project,
        &["move", "review-result:looked", "--to", "archived"],
    );
    build_story(
        &project,
        filler,
        "Filler",
        &["proposed", "active", "archived"],
    );
    commit(&project, "a review naming a customer");
    project
}

/// Packing and unpacking a redacted review-result with the real verbs keeps the redaction replay
/// valid, and no command along the way prints the literal.
#[test]
fn a_redacted_review_result_packed_and_unpacked_keeps_the_replay_valid() {
    let project = review_store("redact-round-trip", "filler");
    redact(&project);
    commit(&project, "redact");
    for (step, args) in [
        ("pack --dry-run", vec!["pack", "--dry-run"]),
        ("pack", vec!["pack"]),
    ] {
        let output = store(&project, &args);
        assert!(output.status.success(), "{step}: {}", said(&output));
        assert!(
            !said(&output)
                .to_lowercase()
                .contains(&LITERAL.to_lowercase()),
            "{step} prints no literal: {}",
            said(&output)
        );
    }
    commit(&project, "pack");
    for pack_file in packs(&project) {
        let bytes = std::fs::read(packs_directory(&project).join(&pack_file)).expect("reads");
        assert!(
            !String::from_utf8_lossy(&bytes)
                .to_lowercase()
                .contains(&LITERAL.to_lowercase()),
            "{pack_file} holds no literal"
        );
    }
    let packed = artifact(&project, &["validate"]);
    assert!(packed.status.success(), "packed: {}", said(&packed));

    for (step, args) in [
        ("unpack --dry-run", vec!["unpack", "--all", "--dry-run"]),
        ("unpack", vec!["unpack", "--all"]),
    ] {
        let output = store(&project, &args);
        assert!(output.status.success(), "{step}: {}", said(&output));
        assert!(
            !said(&output)
                .to_lowercase()
                .contains(&LITERAL.to_lowercase()),
            "{step} prints no literal: {}",
            said(&output)
        );
    }
    commit(&project, "unpack");
    let unpacked = artifact(&project, &["validate"]);
    assert!(unpacked.status.success(), "unpacked: {}", said(&unpacked));
}

/// The design's own merge resolution for two branches that unpacked from one pack (merge with
/// `merge.renames=false`, keep both rewritten packs). Branch A unpacks the archived review,
/// redacts it and packs it again, as `pack` packs every closed artifact; branch B unpacks an
/// unrelated story. After the merge, B's rewrite of the old pack still holds the review with the
/// literal, A's new pack holds the redacted review, and no loose copy exists.
///
/// `validate` refuses that state. A read verb must not answer it with the literal: whichever copy
/// the overlay reads first, `show` and `history` must not print it. The fixture varies an
/// unrelated story's name until the copy holding the literal is the one the overlay reads first
/// (pack names are content digests), so the case is decided, not left to the hash.
#[test]
fn no_read_verb_prints_a_redacted_literal_a_two_branch_unpack_merge_leaves_packed() {
    let mut chosen = None;
    for variant in 0..24 {
        let filler = format!("filler-{variant}");
        let project = review_store(&format!("redact-merge-{variant}"), &filler);
        store_ok(&project, &["pack"]);
        commit(&project, "pack");
        let base = git(&project, &["branch", "--show-current"])
            .trim()
            .to_owned();

        git(&project, &["checkout", "--quiet", "-b", "redact", &base]);
        store_ok(&project, &["unpack", "review-result:looked"]);
        commit(&project, "unpack the review");
        redact(&project);
        commit(&project, "redact");
        store_ok(&project, &["pack"]);
        commit(&project, "pack the redacted review");
        let redacted = pack_holding(&project, "planning/review-result/looked.md");

        git(&project, &["checkout", "--quiet", "-b", "other", &base]);
        store_ok(&project, &["unpack", &format!("story:{filler}")]);
        commit(&project, "unpack the filler");
        let literal = pack_holding(&project, "planning/review-result/looked.md");

        git(&project, &["checkout", "--quiet", &base]);
        if literal < redacted {
            chosen = Some((project, literal, redacted));
            break;
        }
    }
    let (project, literal_pack, redacted_pack) =
        chosen.expect("a variant whose literal-holding pack sorts first");
    git(&project, &["merge", "--quiet", "--no-edit", "redact"]);
    git(
        &project,
        &[
            "-c",
            "merge.renames=false",
            "merge",
            "--quiet",
            "--no-edit",
            "other",
        ],
    );
    assert!(
        !engineering(&project)
            .join("planning/review-result/looked.md")
            .exists(),
        "precondition: no loose copy of the review"
    );
    let present = packs(&project);
    assert!(
        present.contains(&literal_pack) && present.contains(&redacted_pack),
        "precondition: both rewrites are kept: {present:?}"
    );
    let validate = artifact(&project, &["validate"]);
    assert_eq!(
        validate.status.code(),
        Some(1),
        "validate refuses the re-exposure: {}",
        said(&validate)
    );
    assert!(said(&validate).contains("pack_redacted_literal_exposed"));

    for args in [
        vec!["show", "review-result:looked"],
        vec!["show", "review-result:looked", "--format", "json"],
        vec!["history", "review-result:looked"],
    ] {
        let output = artifact(&project, &args);
        let text = said(&output);
        assert!(
            !text.to_lowercase().contains(&LITERAL.to_lowercase()),
            "`{}` (exit {:?}) printed the redacted literal from {literal_pack}: {text}",
            args.join(" "),
            output.status.code()
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Fixes of the review: the evidence link, a `/5` store that holds packs.
// ---------------------------------------------------------------------------------------------

/// `.engineering/evidence` replaced by a symbolic link to a directory outside the repository:
/// `pack` refuses, touches nothing behind the link and packs none of its records.
#[test]
fn pack_never_follows_a_symbolically_linked_evidence_directory() {
    let project = loose_store("linked-evidence");
    let outside = scratch_root().join("store-pack-security-linked-evidence-outside");
    if outside.exists() {
        std::fs::remove_dir_all(&outside).expect("removable");
    }
    std::fs::rename(engineering(&project).join("evidence"), &outside).expect("movable");
    symlink(&outside, engineering(&project).join("evidence")).expect("links");
    let before = files(&outside);
    assert!(
        !before.is_empty(),
        "precondition: the linked directory holds the evidence"
    );

    let output = store(&project, &["pack"]);
    let text = said(&output);
    assert_eq!(output.status.code(), Some(1), "`pack` is refused: {text}");
    assert!(text.contains("symbolic link"), "{text}");
    assert_eq!(
        files(&outside),
        before,
        "files behind the link are as they were: {text}"
    );
    assert!(
        packs(&project).is_empty(),
        "no pack was written: {:?}",
        packs(&project)
    );
}

/// `unpack` refuses the same links, and writes nothing.
#[test]
fn unpack_never_follows_a_symbolically_linked_planning_directory() {
    let project = packed_store("unpack-linked-planning");
    let outside = scratch_root().join("store-pack-security-unpack-linked-planning-outside");
    if outside.exists() {
        std::fs::remove_dir_all(&outside).expect("removable");
    }
    std::fs::rename(engineering(&project).join("planning"), &outside).expect("movable");
    symlink(&outside, engineering(&project).join("planning")).expect("links");
    let before = files(&outside);
    let packs_before = files(&packs_directory(&project));

    let output = store(&project, &["unpack", "--all"]);
    let text = said(&output);
    assert_eq!(output.status.code(), Some(1), "`unpack` is refused: {text}");
    assert!(text.contains("symbolic link"), "{text}");
    assert_eq!(files(&outside), before, "{text}");
    assert_eq!(files(&packs_directory(&project)), packs_before, "{text}");
}

/// The project file of a packed store, set back to `aep.project/5`, as a merge that took the other
/// side's `project.yaml` leaves it.
fn packed_store_selecting_v5(name: &str) -> PathBuf {
    let project = packed_store(name);
    let file = engineering(&project).join("project.yaml");
    let text = std::fs::read_to_string(&file).expect("reads");
    assert!(
        text.contains("version: aep.project/6"),
        "precondition: {text}"
    );
    std::fs::write(
        &file,
        text.replace("version: aep.project/6", "version: aep.project/5"),
    )
    .expect("writable");
    commit(&project, "a merge that took the /5 project file");
    project
}

fn selected_version(project: &Path) -> String {
    std::fs::read_to_string(engineering(project).join("project.yaml"))
        .expect("reads")
        .lines()
        .find(|line| line.starts_with("version:"))
        .expect("a version line")
        .to_owned()
}

/// A `/5` store that holds packs: the readers keep every record, `validate` refuses it as
/// `pack_version_disagrees`, and `pack` leaves the version that matches.
#[test]
fn pack_on_a_v5_store_that_holds_packs_leaves_v6() {
    let project = packed_store_selecting_v5("v5-pack");
    let shown = artifact(&project, &["show", "story:closed-a"]);
    assert!(
        shown.status.success(),
        "no record is lost: {}",
        said(&shown)
    );
    let validate = artifact(&project, &["validate"]);
    assert_eq!(validate.status.code(), Some(1), "{}", said(&validate));
    assert!(
        said(&validate).contains("pack_version_disagrees"),
        "{}",
        said(&validate)
    );

    let output = store(&project, &["pack"]);
    assert!(output.status.success(), "{}", said(&output));
    assert_eq!(selected_version(&project), "version: aep.project/6");
    let validate = artifact(&project, &["validate"]);
    assert!(validate.status.success(), "{}", said(&validate));
}

/// `unpack` of one artifact from a `/5` store that holds packs leaves `/6`: packs remain.
#[test]
fn unpack_on_a_v5_store_that_holds_packs_leaves_v6_while_packs_remain() {
    let project = packed_store_selecting_v5("v5-unpack-some");
    let output = store(&project, &["unpack", "story:closed-a"]);
    assert!(output.status.success(), "{}", said(&output));
    assert_eq!(selected_version(&project), "version: aep.project/6");
    assert!(!packs(&project).is_empty(), "a pack remains");
    let validate = artifact(&project, &["validate"]);
    assert!(validate.status.success(), "{}", said(&validate));
}

/// `unpack --all` of a `/5` store that holds packs leaves `/5`: none remain.
#[test]
fn unpack_all_on_a_v5_store_that_holds_packs_leaves_v5() {
    let project = packed_store_selecting_v5("v5-unpack-all");
    let output = store(&project, &["unpack", "--all"]);
    assert!(output.status.success(), "{}", said(&output));
    assert_eq!(selected_version(&project), "version: aep.project/5");
    assert!(packs(&project).is_empty(), "no pack remains");
    let validate = artifact(&project, &["validate"]);
    assert!(validate.status.success(), "{}", said(&validate));
}
