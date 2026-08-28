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

    /// Asserts a verb answers alike in every store, and exits with `code` — because two stores
    /// that fail the same way are alike too, and that is not what this test is for.
    fn alike_with(&self, args: &[&str], code: i32) {
        let markdown = self.alike(args);
        assert_eq!(
            markdown.code,
            Some(code),
            "`protocol {}`: {}{}",
            args.join(" "),
            markdown.stdout,
            markdown.stderr
        );
    }

    /// Asserts a verb answers alike in every store, running it once in each; the markdown answer.
    fn alike(&self, args: &[&str]) -> Answer {
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
        markdown
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

/// The store's location as `<store>`, in the two ways the verbs spell it, and wall-clock instants
/// as `<instant>`. Every number is compared — revisions included, since
/// `story:relation-bumps-a-document-revision-but-not-an-entity` made every store count an edge the
/// same way.
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
///
/// `show` is here and not in [`READS_BEFORE`] on purpose: the artifact it names is the one whose
/// prose arrived **through the contract**, and prose is the one thing seeding from the example's
/// manifest does not carry — so before the writes the markdown project would print a body the
/// other two have never been told about, and the difference would be the seed's, not the verb's.
const READS_AFTER: &[&[&str]] = &[
    &["artifact", "list"],
    &["artifact", "list", "--format", "json"],
    &["artifact", "board"],
    &["artifact", "graph"],
    &["artifact", "show", "story:passkey-audit-trail"],
    &[
        "artifact",
        "show",
        "story:passkey-audit-trail",
        "--format",
        "json",
    ],
    &["artifact", "history", "story:passkey-audit-trail"],
    &[
        "artifact",
        "history",
        "story:passkey-audit-trail",
        "--format",
        "json",
    ],
    &["artifact", "history", "story:passkey-login"],
    &["artifact", "explain", "story:passkey-login"],
    &[
        "artifact",
        "explain",
        "story:passkey-login",
        "--format",
        "json",
    ],
    &["artifact", "explain", "story:passkey-audit-trail"],
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
            "evidence",
            "story:passkey-login",
            "--kind",
            "review",
            "--source",
            "alice",
            "--ref",
            "https://example.invalid/review/9",
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
        pair.alike_with(args, 0);
    }
    let writes = writes(body);
    let (accepted, refused) = writes.split_at(writes.len() - 1);
    for args in accepted {
        pair.alike_with(args, 0);
    }
    for args in refused {
        pair.alike_with(args, 1);
    }
    for args in READS_AFTER {
        pair.alike_with(args, 0);
    }
    let (markdown, _) = pair.both(&["artifact", "validate"]);
    assert_eq!(markdown.code, Some(0), "{}", markdown.stdout);
}

/// The prose `show` is asked to print back, spacing and blank line included.
///
/// Deliberately not tidy: two spaces inside a line and a trailing blank one are exactly what a
/// verb that "cleaned up" the body would lose, and losing them is the failure this asserts against.
const SHOWN_BODY: &str = "# Audit trail\n\nEvery ceremony,  verbatim.\n\n";

/// `protocol artifact show <id>` prints one artifact, and the same one in every store.
///
/// The gap it closes: with an id in hand there was no verb at all. `list` prints the whole plan,
/// `explain` answers what made a status happen, `history` prints the event log and `body` *writes* —
/// so a driven session asked for `show` five times in one run and got `unrecognized subcommand`
/// every time. The artifact is created and given its prose here rather than taken from the example,
/// because a body seeded from the manifest exists only in the markdown copy.
/// `story:passkey-audit-trail`, with a title, a summary, a tag, an edge and [`SHOWN_BODY`], written
/// into all three stores through the contract so that what `show` prints back is comparable.
fn a_story_carrying_prose(pair: &Pair) {
    let body = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store-selection-show-body.md");
    std::fs::write(&body, SHOWN_BODY).expect("body written");
    let body = body.to_str().expect("a printable path");

    for args in [
        &[
            "artifact",
            "new",
            "story",
            "passkey-audit-trail",
            "--title",
            "Every passkey ceremony is audited",
            "--summary",
            "One artifact, printed.",
            "--tag",
            "webauthn",
        ][..],
        &[
            "artifact",
            "relate",
            "story:passkey-audit-trail",
            "decomposes",
            "epic:passkey-sign-in",
        ][..],
        &[
            "artifact",
            "body",
            "story:passkey-audit-trail",
            "--from",
            body,
        ][..],
    ] {
        pair.alike_with(args, 0);
    }
}

#[test]
fn show_prints_one_artifact_with_its_body_verbatim_in_every_store() {
    let pair = Pair::new("show");
    a_story_carrying_prose(&pair);

    // Text: the frontmatter fields the reader asked for, then the body, unaltered and last.
    let shown = pair.alike(&["artifact", "show", "story:passkey-audit-trail"]);
    assert_eq!(shown.code, Some(0), "{}{}", shown.stdout, shown.stderr);
    for expected in [
        "story:passkey-audit-trail",
        "story",
        "draft",
        "Every passkey ceremony is audited",
        "One artifact, printed.",
        "webauthn",
        "decomposes epic:passkey-sign-in",
    ] {
        assert!(
            shown.stdout.contains(expected),
            "`artifact show` does not print {expected:?}:\n{}",
            shown.stdout
        );
    }
    assert!(
        shown.stdout.ends_with(SHOWN_BODY),
        "the body is not printed verbatim, last and whole:\n{}",
        shown.stdout
    );

    // YAML and JSON carry the same artifact, and the body byte for byte.
    let yaml = pair.alike(&[
        "artifact",
        "show",
        "story:passkey-audit-trail",
        "--format",
        "yaml",
    ]);
    assert_eq!(yaml.code, Some(0), "{}{}", yaml.stdout, yaml.stderr);
    let json = pair.alike(&[
        "artifact",
        "show",
        "story:passkey-audit-trail",
        "--format",
        "json",
    ]);
    assert_eq!(json.code, Some(0), "{}{}", json.stdout, json.stderr);
    let document: serde_json::Value =
        serde_json::from_str(&json.stdout).expect("`show --format json` is JSON");
    assert_eq!(document["id"], "story:passkey-audit-trail");
    assert_eq!(document["kind"], "story");
    assert_eq!(document["status"], "draft");
    assert_eq!(document["title"], "Every passkey ceremony is audited");
    assert_eq!(document["summary"], "One artifact, printed.");
    assert_eq!(document["tags"][0], "webauthn");
    assert_eq!(document["relations"][0]["relation"], "decomposes");
    assert_eq!(document["relations"][0]["target"], "epic:passkey-sign-in");
    assert_eq!(
        document["body"], SHOWN_BODY,
        "`show --format json` altered the body"
    );

    // An id the plan does not hold is refused, naming it, in every store — the way `explain` and
    // `history` refuse one. The wording differs by store because the stores name themselves
    // differently; that the id is in it does not.
    for (label, project) in [
        ("markdown", &pair.markdown),
        ("sqlite", &pair.sqlite),
        ("hybrid", &pair.hybrid),
    ] {
        let output = protocol_in(project, &["artifact", "show", "story:not-in-this-plan"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "the {label} plan did not refuse an unknown id: {}{}",
            text(&output.stdout),
            text(&output.stderr)
        );
        let said = format!("{}{}", text(&output.stdout), text(&output.stderr));
        assert!(
            said.contains("story:not-in-this-plan"),
            "the {label} plan's refusal does not name the id: {said}"
        );
    }
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

#[test]
fn evidence_without_at_is_recorded_at_the_instant_the_edge_read() {
    // `story:evidence-verb-refuses-its-own-default-instant`: the default was produced to the second
    // and refused by the reader that only knew a date — every recording had to type `--at`.
    let pair = Pair::new("evidence-now");
    for project in [&pair.markdown, &pair.sqlite, &pair.hybrid] {
        let output = protocol_in(
            project,
            &[
                "artifact",
                "evidence",
                "story:passkey-login",
                "--kind",
                "test_result",
                "--source",
                "task check",
            ],
        );
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        let history = protocol_in(project, &["artifact", "history", "story:passkey-login"]);
        assert!(
            text(&history.stdout).contains("test_result"),
            "{}",
            text(&history.stdout)
        );
    }

    // And an instant nothing can read is still refused, naming it.
    let output = protocol_in(
        &pair.markdown,
        &[
            "artifact",
            "evidence",
            "story:passkey-login",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "yesterday-ish",
        ],
    );
    assert_ne!(output.status.code(), Some(0));
    assert!(
        text(&output.stderr).contains("`yesterday-ish` is not an instant this build can read"),
        "{}",
        text(&output.stderr)
    );
}

/// Records two observations about `story:passkey-login` and moves it, in `project`.
///
/// Its own function because three tests need the same fixture and the difference between them is
/// what they then ask of it, not how it was built.
fn closed_on_two_records(project: &Path, reference: &str) {
    for (kind, source) in [("test_result", "task check"), ("review", "alice")] {
        let output = protocol_in(
            project,
            &[
                "artifact",
                "evidence",
                "story:passkey-login",
                "--kind",
                kind,
                "--source",
                source,
                "--ref",
                reference,
                "--at",
                AT,
            ],
        );
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    }
    let output = protocol_in(
        project,
        &[
            "artifact",
            "move",
            "--to",
            "implemented",
            "story:passkey-login",
            "--at",
            AT,
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
}

#[test]
fn what_made_a_story_done_names_the_revision_each_record_was_admitted_at() {
    // `story:completion-audit-join`. The join is worth nothing unless it is pinned to the text the
    // record was about: an answer that named the artifact's *current* revision would read correctly
    // on the day it was written and be a lie every day after, because a later edit would silently
    // re-date every old record onto the new body.
    let pair = Pair::new("explain-revisions");
    for project in [&pair.markdown, &pair.sqlite, &pair.hybrid] {
        closed_on_two_records(project, "run-4711");

        let output = protocol_in(
            project,
            &[
                "artifact",
                "explain",
                "story:passkey-login",
                "--format",
                "json",
            ],
        );
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        let report: serde_json::Value =
            serde_json::from_str(&text(&output.stdout)).expect("a JSON explanation");

        let reached = report["reached"]
            .as_array()
            .expect("the statuses it reached");
        let moved = reached
            .iter()
            .find(|step| step["to"] == "implemented")
            .unwrap_or_else(|| panic!("the move to implemented: {reached:#?}"));
        let records = moved["rested_on"]
            .as_array()
            .expect("the records the move rested on");

        // One-to-many, which is the story's own default: a suite and a review are two records and
        // forcing a choice between them would lose one.
        assert_eq!(records.len(), 2, "{records:#?}");
        assert_eq!(records[0]["kind"], "test_result");
        assert_eq!(records[1]["kind"], "review");
        assert_eq!(records[0]["reference"], "run-4711");

        // The fixture reached the state where the rule is load-bearing: the move left the artifact
        // at a later revision than the records were admitted at, so naming the wrong one is a
        // difference this test can see at all.
        let admitted = records[0]["revision"].as_u64().expect("a revision");
        let after_the_move = moved["revision"].as_u64().expect("a revision");
        assert!(
            after_the_move > admitted,
            "the move must leave the artifact past the revision the records were admitted at, or \
             the two revisions are indistinguishable: {admitted} -> {after_the_move}"
        );
        assert_eq!(
            report["revision"].as_u64(),
            Some(after_the_move),
            "the artifact stands at the revision the move left it at"
        );
        for record in records {
            assert_eq!(
                record["revision"].as_u64(),
                Some(admitted),
                "a record is named against the revision the artifact was at when it was admitted, \
                 never the one it is at now: {record:#?}"
            );
        }
    }
}

#[test]
fn a_joined_record_outlives_the_file_its_reference_names() {
    // The join is a stored fact, not a path. A CI log rotates away and a scratch file is cleaned
    // up; the record of what closed the story must not go with it, which is the difference between
    // an audit trail and a directory listing.
    let pair = Pair::new("explain-deleted-reference");
    let log = Path::new(env!("CARGO_TARGET_TMPDIR")).join("explain-deleted-reference.log");
    std::fs::write(&log, "1 suite, 0 failures\n").expect("the run's log is written");
    let reference = log.to_str().expect("a printable path").to_owned();

    for project in [&pair.markdown, &pair.sqlite, &pair.hybrid] {
        closed_on_two_records(project, &reference);
    }

    // The fixture reached the state the rule is about: the reference named a file that was there,
    // and now names one that is not.
    assert!(log.is_file(), "the reference named a file that existed");
    std::fs::remove_file(&log).expect("the log goes");
    assert!(!log.exists(), "and now names one that does not");

    for project in [&pair.markdown, &pair.sqlite, &pair.hybrid] {
        let output = protocol_in(project, &["artifact", "explain", "story:passkey-login"]);
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        let said = text(&output.stdout);
        assert!(
            said.contains("test_result") && said.contains("review"),
            "both records are still joined to the story: {said}"
        );
        assert!(
            said.contains(&reference),
            "and the reference is still named though nothing is there to read: {said}"
        );
    }
}

#[test]
fn a_status_reached_without_a_record_says_which_kind_of_claim_it_rested_on() {
    // Mirroring `protocol artifact validate`: a status reached on somebody's word is legal — a
    // runner is down on the day it matters most — and what it must not be is indistinguishable
    // from one the store holds a record for.
    let pair = Pair::new("explain-assertions");
    for project in [&pair.markdown, &pair.sqlite, &pair.hybrid] {
        // A rung that asks for nothing: nothing was recorded about how it was decided.
        let output = protocol_in(
            project,
            &[
                "artifact",
                "move",
                "--to",
                "active",
                "story:passkey-recovery",
                "--at",
                AT,
            ],
        );
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        // A rung that asks for a test result, answered by a number nobody can go and check.
        let output = protocol_in(
            project,
            &[
                "artifact",
                "move",
                "--to",
                "implemented",
                "story:passkey-login",
                "--evidence",
                "test_result=1",
                "--at",
                AT,
            ],
        );
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

        let output = protocol_in(project, &["artifact", "explain", "story:passkey-login"]);
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        let said = text(&output.stdout);
        assert!(
            said.contains("asserted — no record"),
            "a move on a bare count is marked as one: {said}"
        );

        let output = protocol_in(project, &["artifact", "explain", "story:passkey-recovery"]);
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        let said = text(&output.stdout);
        assert!(
            said.contains("no record"),
            "a move that rested on nothing says so: {said}"
        );
        assert!(
            !said.contains("asserted"),
            "and does not claim somebody asserted something they did not: {said}"
        );
    }
}

#[test]
fn explaining_an_artifact_no_store_holds_is_refused_naming_it() {
    let pair = Pair::new("explain-unknown");
    for project in [&pair.markdown, &pair.sqlite, &pair.hybrid] {
        let output = protocol_in(project, &["artifact", "explain", "story:no-such-story"]);
        assert_ne!(output.status.code(), Some(0), "{}", text(&output.stdout));
        assert!(
            text(&output.stderr).contains("story:no-such-story"),
            "{}",
            text(&output.stderr)
        );
    }
}
