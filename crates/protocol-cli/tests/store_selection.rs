//! One line in `project.yaml` decides where the plan is kept, and no verb can tell the difference.
//!
//! `story:store-selection-in-project-yaml` and `story:hybrid-backend`. Three copies of
//! `examples/planning-passkeys/` — on `project.yaml` (markdown, the default), on
//! `project.sqlite.yaml`, and on `project.hybrid.yaml` (markdown with a SQLite replica) — each with
//! the same seven artifacts seeded through the contract, and every `protocol artifact` verb run in
//! all three, each as its own process, with its output compared after the one thing that
//! legitimately differs (where the store is) is written as `<store>`. A `hybrid` missing a policy
//! word is refused by name; `protocol conformance --backend project` holds the configured kind of
//! store to the suites without writing into the plan; and a hybrid whose replica refuses a write
//! records the divergence for the next process to list and catch up.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use aep_backend_hybrid::HybridBackend;
use aep_backend_markdown::backend::{MarkdownBackend, ORGANISATION, SPACE};
use aep_backend_markdown::store::MarkdownStore;
use aep_backend_memory::seed;
use aep_backend_sqlite::SqliteBackend;
use aep_domain::artifact::LifecycleRegistry;
use aep_domain::entity::ActorRef;
use aep_domain::time::Timestamp;
use entity_sqlite::SqliteStore;

/// The repository root: the protocol tree both projects point at.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn example() -> PathBuf {
    root().join("examples/planning-passkeys")
}

/// Runs `protocol` from inside `project`, which is how an adopting team runs it: no `--store`, no
/// `--root`, everything from `project.yaml`.
fn protocol_in(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(project)
        .output()
        .expect("the protocol binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the target exists");
    for entry in std::fs::read_dir(from).expect("readable") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copied");
        }
    }
}

/// A scratch copy of the example, its `protocols:` re-pointed at this repository (the example's
/// `../../..` is relative to where the example sits, not to where a copy under `target/` does; a
/// project file refuses an absolute path, so the copy's own relative one is computed).
fn scratch_project(name: &str, variant: &str) -> PathBuf {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("store-selection-{name}"));
    let _ = std::fs::remove_dir_all(&project);
    copy_tree(&example(), &project);
    let engineering = project.join(".engineering");
    let depth = engineering
        .canonicalize()
        .expect("the copy exists")
        .strip_prefix(root())
        .expect("the scratch tree is under the repository")
        .components()
        .count();
    let up = vec![".."; depth].join("/");
    let config = std::fs::read_to_string(engineering.join(variant)).expect("the variant exists");
    let config = config.replace("protocols: ../../..", &format!("protocols: {up}"));
    std::fs::write(engineering.join("project.yaml"), config).expect("project.yaml written");
    let _ = std::fs::remove_file(engineering.join("project.sqlite.yaml"));
    project
}

/// The four words the hybrid example declares, as the runtime spells them.
const HYBRID_POLICY: aep_backend_hybrid::Policy = aep_backend_hybrid::Policy::new(
    aep_backend_hybrid::Authority::Local,
    aep_backend_hybrid::ReadPath::LocalFirst,
    aep_backend_hybrid::WhenUnreachable::Refuse,
    aep_backend_hybrid::OnDivergence::RecordDivergence,
);

/// The three projects, each seeded with the example's seven artifacts through the contract — the
/// markdown one by `MarkdownBackend`, the SQLite one by `SqliteBackend`, the hybrid one by
/// `HybridBackend`, same instant, same actor, same commands in the same order — so that what differs
/// afterwards is the store and nothing else. The example's own files are the source and are read
/// once; the SQLite project keeps none of them, so nothing could answer from a file by mistake.
struct Pair {
    markdown: PathBuf,
    sqlite: PathBuf,
    hybrid: PathBuf,
}

impl Pair {
    fn new(name: &str) -> Self {
        let markdown = scratch_project(&format!("{name}-markdown"), "project.yaml");
        let sqlite = scratch_project(&format!("{name}-sqlite"), "project.sqlite.yaml");
        let hybrid = scratch_project(&format!("{name}-hybrid"), "project.hybrid.yaml");

        let report = MarkdownStore::open(example().join(".engineering/planning")).load();
        assert!(report.is_clean(), "the example reads cleanly");
        let graph = report.graph().expect("the example is a graph");
        let at = Timestamp::from_epoch_millis(1_700_000_000_000);
        let actor = ActorRef::parse("human:seed").expect("an actor");

        let planning = markdown.join(".engineering/planning");
        std::fs::remove_dir_all(&planning).expect("the copied files go");
        std::fs::create_dir_all(&planning).expect("an empty store");
        let files = MarkdownBackend::open(
            &planning,
            std::iter::empty(),
            at,
            actor.clone(),
            LifecycleRegistry::default(),
        )
        .expect("the markdown backend opens");
        let seeded = seed::from_manifest(&files, &graph, ORGANISATION, SPACE, at, &actor)
            .expect("the plan seeds into files");
        assert_eq!(seeded.entities, graph.len());
        drop(files);

        std::fs::remove_dir_all(sqlite.join(".engineering/planning")).expect("no files here");
        let database = SqliteBackend::open(sqlite.join(".engineering/plan.sqlite3"))
            .expect("the database opens");
        let seeded = seed::from_manifest(&database, &graph, ORGANISATION, SPACE, at, &actor)
            .expect("the plan seeds into SQLite");
        assert_eq!(seeded.entities, graph.len());
        drop(database);

        let planning = hybrid.join(".engineering/planning");
        std::fs::remove_dir_all(&planning).expect("the copied files go");
        std::fs::create_dir_all(&planning).expect("an empty store");
        let replica =
            SqliteStore::open(hybrid.join(".engineering/replica.sqlite3")).expect("a replica");
        let both = HybridBackend::open(
            &planning,
            replica,
            HYBRID_POLICY,
            std::iter::empty(),
            at,
            actor.clone(),
            LifecycleRegistry::default(),
        )
        .expect("the hybrid backend opens");
        let seeded = seed::from_manifest(&both, &graph, ORGANISATION, SPACE, at, &actor)
            .expect("the plan seeds into both");
        assert_eq!(seeded.entities, graph.len());
        assert!(
            both.divergences().is_empty(),
            "the replica took every write"
        );
        drop(both);

        Self {
            markdown,
            sqlite,
            hybrid,
        }
    }

    /// One verb in the markdown and the SQLite project, its output made comparable.
    fn both(&self, args: &[&str]) -> (Answer, Answer) {
        (
            Answer::of(&self.markdown, args),
            Answer::of(&self.sqlite, args),
        )
    }

    /// Asserts a verb answers alike in every store.
    fn alike(&self, args: &[&str]) {
        let (markdown, sqlite) = self.both(args);
        assert_eq!(
            markdown,
            sqlite,
            "`protocol {}` differs between markdown and SQLite",
            args.join(" ")
        );
        let hybrid = Answer::of(&self.hybrid, args);
        assert_eq!(
            markdown,
            hybrid,
            "`protocol {}` differs between markdown and the hybrid",
            args.join(" ")
        );
    }
}

/// What one invocation answered, with the store's location and the seed's import written out.
#[derive(Debug, PartialEq, Eq)]
struct Answer {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Answer {
    fn of(project: &Path, args: &[&str]) -> Self {
        let output = protocol_in(project, args);
        Self {
            code: output.status.code(),
            stdout: normalise(project, &text(&output.stdout)),
            stderr: normalise(project, &text(&output.stderr)),
        }
    }
}

/// The store's location as `<store>`, in the two ways the verbs spell it; and revision numbers as
/// `#`, because the two stores count them differently for one reason that is recorded rather than
/// hidden: a markdown document's revision moves when an edge is written into its frontmatter, an
/// entity's does not move when a relation is created beside it
/// (`story:relation-bumps-a-document-revision-but-not-an-entity`). Every other number is compared.
fn normalise(project: &Path, text: &str) -> String {
    let engineering = project.join(".engineering");
    let markdown_root = engineering.join("planning").display().to_string();
    let sqlite = format!(
        "the SQLite store {}",
        engineering.join("plan.sqlite3").display()
    );
    let replica = format!(
        " with its replica in the SQLite store {}",
        engineering.join("replica.sqlite3").display()
    );
    let mut out = String::new();
    for line in text.lines() {
        let line = line.replace(&replica, "");
        let line = line.replace(&format!("{markdown_root}/"), "<store>/");
        let line = line.replace(&markdown_root, "<store>");
        let line = line.replace(&sqlite, "<store>");
        // `created story:x (draft) at <store>/story/x.md` on files; `… at story/x.md in <store>`
        // on a plan without files. Same fact, one spelling.
        let line = match line.split_once(" in <store>") {
            Some((before, after)) if after.is_empty() || after.starts_with('"') => {
                match before.rsplit_once(' ') {
                    Some((head, relative)) if relative.contains('/') => {
                        format!("{head} <store>/{relative}{after}")
                    }
                    _ => format!("{before} in <store>{after}"),
                }
            }
            _ => line,
        };
        let line = blank_number_after(&line, "(revision ");
        let line = blank_number_after(&line, "\"revision\": ");
        let line = blank_number_after(&line, "\"version\": \"");
        let line = blank_instants(&line);
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// `line` with every ISO-8601 instant to the second replaced by `<instant>`: the verbs that take no
/// `--at` read the clock, and three processes run one after another cross a second boundary.
fn blank_instants(line: &str) -> String {
    let bytes = line.as_bytes();
    let is_instant = |at: usize| {
        at + 20 <= bytes.len()
            && bytes[at..at + 20].iter().enumerate().all(|(i, b)| match i {
                4 | 7 => *b == b'-',
                10 => *b == b'T',
                13 | 16 => *b == b':',
                19 => *b == b'Z',
                _ => b.is_ascii_digit(),
            })
    };
    let mut out = String::new();
    let mut at = 0;
    while at < bytes.len() {
        if is_instant(at) {
            out.push_str("<instant>");
            at += 20;
        } else {
            let next = line[at..].chars().next().expect("within the line");
            out.push(next);
            at += next.len_utf8();
        }
    }
    out
}

/// `line` with the digits following every `prefix` replaced by `#`.
fn blank_number_after(line: &str, prefix: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(index) = rest.find(prefix) {
        let after = index + prefix.len();
        out.push_str(&rest[..after]);
        let digits = rest[after..]
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len() - after);
        if digits > 0 {
            out.push('#');
        }
        rest = &rest[after + digits..];
    }
    out.push_str(rest);
    out
}

/// The instant every dated verb is given, so two processes decide against one clock.
const AT: &str = "2026-08-28T12:00:00Z";

/// The reads, before anything was written.
const READS_BEFORE: &[&[&str]] = &[
    &["artifact", "list"],
    &["artifact", "list", "--format", "json"],
    &["artifact", "list", "--kind", "story"],
    &["artifact", "board"],
    &["artifact", "graph"],
    &["artifact", "graph", "--format", "json"],
    &["artifact", "validate", "--format", "json"],
    &["artifact", "lifecycle", "story"],
    &["artifact", "kinds"],
    &["artifact", "relations"],
];

/// The reads, after: the writes landed the same way, and the history says the same things.
const READS_AFTER: &[&[&str]] = &[
    &["artifact", "list"],
    &["artifact", "list", "--format", "json"],
    &["artifact", "board"],
    &["artifact", "graph"],
    &["artifact", "history", "story:passkey-audit-trail"],
    &[
        "artifact",
        "history",
        "story:passkey-audit-trail",
        "--format",
        "json",
    ],
    &["artifact", "history", "story:passkey-login"],
    &["artifact", "validate"],
];

/// Every verb that changes the plan, each once, each its own process. `body` is the file the
/// replacement prose is read from.
fn writes(body: &str) -> Vec<Vec<&str>> {
    vec![
        vec![
            "artifact",
            "new",
            "story",
            "passkey-audit-trail",
            "--title",
            "Every passkey ceremony is audited",
            "--summary",
            "Written through the contract into whichever store the project names.",
            "--tag",
            "webauthn",
        ],
        vec![
            "artifact",
            "relate",
            "story:passkey-audit-trail",
            "decomposes",
            "epic:passkey-sign-in",
        ],
        vec![
            "artifact",
            "body",
            "story:passkey-audit-trail",
            "--from",
            body,
        ],
        vec![
            "artifact",
            "evidence",
            "story:passkey-login",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--ref",
            "run-4711",
            "--at",
            AT,
        ],
        vec![
            "artifact",
            "move",
            "--to",
            "implemented",
            "story:passkey-login",
            "--at",
            AT,
        ],
        vec![
            "artifact",
            "move",
            "--to",
            "proposed",
            "story:passkey-audit-trail",
            "--at",
            AT,
        ],
        // A move the ladder refuses is refused the same way.
        vec![
            "artifact",
            "move",
            "--to",
            "implemented",
            "story:passkey-audit-trail",
            "--at",
            AT,
        ],
    ]
}

#[test]
fn every_verb_answers_alike_over_markdown_and_sqlite() {
    let pair = Pair::new("verbs");
    let body = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store-selection-body.md");
    std::fs::write(&body, "# Story\n\nReplaced through the contract.\n").expect("body written");
    let body = body.to_str().expect("a printable path");

    for args in READS_BEFORE {
        pair.alike(args);
    }
    for args in writes(body) {
        pair.alike(&args);
    }
    for args in READS_AFTER {
        pair.alike(args);
    }
    let (markdown, _) = pair.both(&["artifact", "validate"]);
    assert_eq!(markdown.code, Some(0), "{}", markdown.stdout);
}

#[test]
fn the_sqlite_plan_is_read_from_the_database_and_not_from_files() {
    let pair = Pair::new("no-files");
    assert!(
        !pair.sqlite.join(".engineering/planning").exists(),
        "the SQLite project keeps no markdown"
    );
    let output = protocol_in(&pair.sqlite, &["artifact", "list"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout).lines().count(),
        7,
        "{}",
        text(&output.stdout)
    );
    // The database is where the project said, relative to `.engineering/`.
    assert!(pair.sqlite.join(".engineering/plan.sqlite3").is_file());
}

#[test]
fn a_hybrid_missing_a_policy_word_is_refused_naming_the_word() {
    let project = scratch_project("hybrid-missing-word", "project.yaml");
    let engineering = project.join(".engineering");
    let config = std::fs::read_to_string(engineering.join("project.yaml")).expect("readable");
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            "{config}\nstore:\n  hybrid:\n    authority: local\n    read: local-first\n    \
             on_unreachable: fail\n    local: markdown\n    replica: {{ sqlite: replica.sqlite3 }}\n"
        ),
    )
    .expect("project.yaml written");

    for args in [
        &["artifact", "list"][..],
        &["validate", "--root", root().to_str().expect("printable")],
    ] {
        let output = protocol_in(&project, args);
        let (stdout, stderr) = (text(&output.stdout), text(&output.stderr));
        assert_ne!(
            output.status.code(),
            Some(0),
            "`protocol {}` accepted a hybrid with no `on_divergence`: {stdout}",
            args.join(" ")
        );
        assert!(
            format!("{stdout}{stderr}").contains("on_divergence"),
            "`protocol {}` refused without naming the missing word:\n{stdout}{stderr}",
            args.join(" ")
        );
    }
}

#[test]
fn conformance_against_the_project_holds_the_configured_kind_of_store_to_the_suites() {
    let pair = Pair::new("conformance");
    for (project, expected) in [
        (&pair.markdown, "markdown ("),
        (&pair.sqlite, "sqlite (in-memory database)"),
        (&pair.hybrid, "hybrid ("),
    ] {
        let output = protocol_in(
            project,
            &[
                "conformance",
                "--backend",
                "project",
                "--level",
                "core",
                "--format",
                "json",
            ],
        );
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        let report: serde_json::Value =
            serde_json::from_str(&text(&output.stdout)).expect("a JSON report");
        let ran_against = report["ran_against"].as_str().unwrap_or_default();
        assert!(
            ran_against.starts_with(expected),
            "ran against {ran_against:?}, expected {expected:?}"
        );
    }
    // The plan itself was not written into: the same seven artifacts, no suite entity among them.
    let output = protocol_in(&pair.sqlite, &["artifact", "list"]);
    assert_eq!(text(&output.stdout).lines().count(), 7);

    // `--store` beside `project` is two answers to one question.
    let output = protocol_in(
        &pair.sqlite,
        &[
            "conformance",
            "--backend",
            "project",
            "--store",
            "x.sqlite3",
        ],
    );
    assert_ne!(output.status.code(), Some(0));
    assert!(text(&output.stderr).contains("project.yaml"));
}

#[test]
fn a_hybrid_records_a_write_its_replica_refused_and_the_next_process_catches_it_up() {
    use std::os::unix::fs::PermissionsExt as _;

    let pair = Pair::new("divergence");
    let project = &pair.hybrid;
    let replica = project.join(".engineering/replica.sqlite3");

    // Nothing outstanding to begin with, and the verb says so with exit 0.
    let output = protocol_in(project, &["artifact", "divergences"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("no divergences recorded; authority: local"));

    // The replica stops taking writes: its file is read-only. The authority takes the story.
    let writable = std::fs::metadata(&replica).expect("metadata").permissions();
    let mut read_only = writable.clone();
    read_only.set_mode(0o444);
    std::fs::set_permissions(&replica, read_only).expect("read-only");
    let output = protocol_in(
        project,
        &[
            "artifact",
            "new",
            "story",
            "passkey-attestation",
            "--title",
            "Attestation is verified",
        ],
    );
    std::fs::set_permissions(&replica, writable).expect("writable again");
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(
        project
            .join(".engineering/planning/story/passkey-attestation.md")
            .is_file(),
        "the authority holds the document"
    );

    // The next process lists what diverged — and says which side is authoritative.
    let output = protocol_in(project, &["artifact", "divergences", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1), "a divergence is a problem");
    let report: serde_json::Value =
        serde_json::from_str(&text(&output.stdout)).expect("a JSON report");
    assert_eq!(report["authority"], "local");
    assert_eq!(report["divergences"].as_array().map(Vec::len), Some(1));
    assert_eq!(report["divergences"][0]["entity"], "story");
    assert_eq!(report["divergences"][0]["id"], "passkey-attestation");
    assert!(
        project
            .join(".engineering/planning/divergences.jsonl")
            .is_file(),
        "written beside the plan"
    );

    // Every other verb still works over the plan while it is diverged.
    let output = protocol_in(project, &["artifact", "list"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).lines().count(), 8);

    // Catch-up replays it at the replica; the replica then holds the story.
    let output = protocol_in(project, &["artifact", "catch-up"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains("1 divergence(s) found, 1 replayed, 0 outstanding"),
        "{}",
        text(&output.stdout)
    );
    assert!(!project
        .join(".engineering/planning/divergences.jsonl")
        .exists());
    let output = protocol_in(project, &["artifact", "divergences"]);
    assert_eq!(output.status.code(), Some(0));
    let held = {
        use entity_store::StateProvider as _;
        SqliteStore::open(&replica)
            .expect("the replica opens")
            .load("story", "passkey-attestation")
            .expect("answers")
    };
    assert!(held.is_some(), "the replica now holds the story");

    // A plan that is not a hybrid has no divergences to speak of.
    let output = protocol_in(&pair.sqlite, &["artifact", "divergences"]);
    assert_ne!(output.status.code(), Some(0));
    assert!(text(&output.stderr).contains("not a hybrid plan"));
}
