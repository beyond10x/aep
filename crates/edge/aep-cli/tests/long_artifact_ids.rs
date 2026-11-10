//! Write verbs on artifact ids near the 200-character identifier limit.
//!
//! Every envelope identifier a verb builds comes from the artifact id, so a long id used to push
//! the request id, command id or idempotency key over the identifier cap and the verb refused.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

/// A fresh Git-native project named `name`.
fn project(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("long-ids-{name}"));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch project is removable");
    }
    let engineering = directory.join(".engineering");
    std::fs::create_dir_all(&engineering).expect("the scratch project is writable");
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&directory)
        .status()
        .expect("git runs");
    assert!(status.success(), "git init");
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            "version: aep.project/5\nplanning_scope: long-ids-test\nprotocol: adp/1\n\
             profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
            relative(&engineering, &repository())
        ),
    )
    .expect("the project file is writable");
    directory
}

fn aep(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "artifact"])
        .args(args)
        .current_dir(directory)
        .env("AEP_ACTOR", "human:tester")
        .output()
        .expect("the binary runs")
}

fn ok(directory: &Path, args: &[&str]) -> String {
    let output = aep(directory, args);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "`{}` failed: {stdout}{stderr}",
        args.join(" ")
    );
    stdout
}

/// An artifact id `story:<name>` of exactly `length` characters.
fn story_id(length: usize, letter: char) -> String {
    let name_length = length - "story:".len();
    format!("story:{}", letter.to_string().repeat(name_length))
}

fn create(project: &Path, id: &str) {
    let name = id.strip_prefix("story:").expect("a story id");
    ok(project, &["new", "story", name, "--title", "Long"]);
}

#[test]
fn relate_joins_a_120_character_id_to_a_100_character_id() {
    let project = project("relate");
    let source = story_id(120, 'a');
    let target = story_id(100, 'b');
    create(&project, &source);
    create(&project, &target);

    ok(&project, &["relate", &source, &format!("reviews:{target}")]);

    let shown = ok(&project, &["show", &source]);
    assert!(
        shown.contains(&target),
        "the relation is in the store: {shown}"
    );
}

#[test]
fn unrelate_removes_an_edge_between_long_ids() {
    let project = project("unrelate");
    let source = story_id(120, 'a');
    let target = story_id(100, 'b');
    create(&project, &source);
    create(&project, &target);
    ok(&project, &["relate", &source, &format!("reviews:{target}")]);

    ok(
        &project,
        &["unrelate", &source, &format!("reviews:{target}")],
    );

    let shown = ok(&project, &["show", &source]);
    assert!(!shown.contains(&target), "the relation is gone: {shown}");
}

#[test]
fn new_move_and_evidence_succeed_on_an_id_near_the_limit() {
    let project = project("near-limit");
    let id = story_id(198, 'c');

    create(&project, &id);
    assert!(
        ok(&project, &["list"]).contains(&id),
        "`new` filed the artifact"
    );

    ok(&project, &["move", &id, "--to", "proposed"]);
    assert!(
        ok(&project, &["show", &id]).contains("proposed"),
        "`move` took effect"
    );

    ok(
        &project,
        &[
            "evidence",
            &id,
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-09-28T10:00:00Z",
        ],
    );
    let evidence = project.join(".engineering/evidence/story");
    assert!(evidence.is_dir(), "`evidence` filed a record");
}

/// A fresh SQLite-backed project named `name`: the store that keeps every envelope identifier a
/// verb built, in its applied-command and audit records.
fn sqlite_project(name: &str) -> PathBuf {
    let directory = project(&format!("sqlite-{name}"));
    let file = directory.join(".engineering/project.yaml");
    let declared = std::fs::read_to_string(&file).expect("the project file is readable");
    std::fs::write(
        &file,
        declared.replace(
            "store:\n  git: {}\n",
            "store:\n  sqlite:\n    path: plan.sqlite3\n",
        ),
    )
    .expect("the project file is writable");
    directory
}

/// `text` with every run of exactly 13 digits — an epoch-millisecond clock reading — as `<ms>`.
fn without_clock(text: &str) -> String {
    let mut masked = String::new();
    let mut digits = String::new();
    for character in text.chars().map(Some).chain(std::iter::once(None)) {
        if let Some(digit) = character.filter(char::is_ascii_digit) {
            digits.push(digit);
            continue;
        }
        if digits.len() == 13 {
            masked.push_str("<ms>");
        } else {
            masked.push_str(&digits);
        }
        digits.clear();
        if let Some(other) = character {
            masked.push(other);
        }
    }
    masked
}

/// One envelope a SQLite store applied, as it kept it.
#[derive(Debug)]
struct Recorded {
    key: String,
    command: String,
    request: String,
    correlation: String,
}

impl Recorded {
    /// The four identifiers on one line, the clock masked.
    fn line(&self) -> String {
        without_clock(&format!(
            "{} {} {} {}",
            self.key, self.command, self.request, self.correlation
        ))
    }
}

/// Every applied command the SQLite store under `project` kept, with the request id its audit
/// record carries.
fn recorded(project: &Path) -> Vec<Recorded> {
    let connection = rusqlite::Connection::open(project.join(".engineering/plan.sqlite3"))
        .expect("the store opens");
    let documents = |entity: &str| -> Vec<serde_json::Value> {
        let mut statement = connection
            .prepare("SELECT document FROM instances WHERE entity = ?1 ORDER BY id")
            .expect("a query");
        statement
            .query_map([entity], |row| row.get::<_, String>(0))
            .expect("rows")
            .map(|row| serde_json::from_str(&row.expect("a row")).expect("a JSON document"))
            .collect()
    };
    let text = |value: &serde_json::Value| value.as_str().expect("a string").to_owned();
    let requests: std::collections::BTreeMap<String, String> = documents("aep.audit")
        .iter()
        .map(|audit| {
            let record = &audit["fields"]["record"];
            (text(&record["command_id"]), text(&record["request_id"]))
        })
        .collect();
    documents("aep.applied")
        .iter()
        .map(|applied| {
            let intent = &applied["fields"]["intent"];
            let command = text(&intent["command_id"]);
            Recorded {
                key: text(&applied["id"]),
                request: requests.get(&command).cloned().unwrap_or_default(),
                command,
                correlation: text(&intent["correlation_id"]),
            }
        })
        .collect()
}

#[test]
fn every_write_verb_on_short_ids_records_the_identifiers_it_recorded_before_the_bound() {
    // Replay of an existing journal: every identifier at or under the cap is the one the code
    // before the bound built. The expected lines are what 5b1a7b9e2 wrote for this sequence.
    let project = sqlite_project("replay");
    let body = project.join("body.md");
    std::fs::write(&body, "A new body.\n").expect("the body file is writable");
    let body = body.to_str().expect("a printable path");

    ok(&project, &["new", "story", "alpha-one", "--title", "Alpha"]);
    ok(&project, &["new", "story", "beta-two", "--title", "Beta"]);
    ok(&project, &["move", "story:alpha-one", "--to", "proposed"]);
    ok(
        &project,
        &["relate", "story:alpha-one", "reviews:story:beta-two"],
    );
    ok(
        &project,
        &["unrelate", "story:alpha-one", "reviews:story:beta-two"],
    );
    ok(
        &project,
        &["relate", "story:alpha-one", "reviews:story:beta-two"],
    );
    ok(
        &project,
        &[
            "evidence",
            "story:alpha-one",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-09-28T10:00:00Z",
        ],
    );
    ok(&project, &["body", "story:alpha-one", "--from", body]);
    ok(&project, &["set", "story:alpha-one", "--title", "Renamed"]);
    ok(
        &project,
        &["scope", "story:alpha-one", "--add", "crates/x/src/lib.rs"],
    );

    let mut lines: Vec<String> = recorded(&project).iter().map(Recorded::line).collect();
    lines.sort();
    let relate = "rel-story-alpha-one-reviews-story-beta-two-<ms> \
                  cmd-story-alpha-one-reviews-story-beta-two \
                  req-story-alpha-one-reviews-story-beta-two protocol-artifact-relate";
    assert_eq!(
        lines,
        [
            "evidence-story-alpha-one-test_result-<ms> \
             cmd-evidence-story-alpha-one-test_result-<ms> \
             req-evidence-story-alpha-one-test_result-<ms> protocol-artifact-evidence",
            "move-story-alpha-one-proposed-<ms> cmd-move-story-alpha-one-<ms> \
             req-move-story-alpha-one protocol-artifact-move",
            "new-story-alpha-one cmd-new-story-alpha-one req-new-story-alpha-one \
             protocol-artifact-new",
            "new-story-beta-two cmd-new-story-beta-two req-new-story-beta-two \
             protocol-artifact-new",
            relate,
            relate,
            "unrel-story-alpha-one-reviews-story-beta-two-<ms> \
             cmd-unrel-story-alpha-one-reviews-story-beta-two \
             req-unrel-story-alpha-one-reviews-story-beta-two protocol-artifact-unrelate",
            "upd-protocol-artifact-body-story-alpha-one-<ms> \
             cmd-protocol-artifact-body-story-alpha-one-<ms> \
             req-protocol-artifact-body-story-alpha-one protocol-artifact-body",
            "upd-protocol-artifact-scope-story-alpha-one-<ms> \
             cmd-protocol-artifact-scope-story-alpha-one-<ms> \
             req-protocol-artifact-scope-story-alpha-one protocol-artifact-scope",
            "upd-protocol-artifact-set-story-alpha-one-<ms> \
             cmd-protocol-artifact-set-story-alpha-one-<ms> \
             req-protocol-artifact-set-story-alpha-one protocol-artifact-set",
        ]
    );
}

#[test]
fn an_identifier_exactly_at_the_cap_is_the_one_built_before_and_one_over_is_bounded() {
    // `new-story-<186>` is 196 characters: `req-` and `cmd-` take it to exactly 200, which the code
    // before the bound accepted, so it must be unchanged. One character more and those two are
    // bounded while the 197-character idempotency key, under its own cap, is not.
    let project = sqlite_project("at-the-cap");
    let at = "e".repeat(186);
    let over = "f".repeat(187);
    ok(&project, &["new", "story", &at, "--title", "At"]);
    ok(&project, &["new", "story", &over, "--title", "Over"]);

    let kept = recorded(&project);
    let of = |name: &str| {
        kept.iter()
            .find(|record| record.key == format!("new-story-{name}"))
            .unwrap_or_else(|| panic!("an applied record for `{name}`: {kept:?}"))
    };
    let exact = of(&at);
    assert_eq!(exact.command, format!("cmd-new-story-{at}"));
    assert_eq!(exact.request, format!("req-new-story-{at}"));
    assert_eq!(exact.command.len(), 200);

    let bounded = of(&over);
    assert_ne!(bounded.command, format!("cmd-new-story-{over}"));
    assert!(bounded.command.len() <= 200, "{}", bounded.command);
    assert!(bounded.request.len() <= 200, "{}", bounded.request);
    assert!(bounded.command.starts_with("cmd-new-story-f"));
    assert!(bounded.request.starts_with("req-new-story-f"));
}

#[test]
fn body_set_scope_and_redact_succeed_on_an_id_near_the_limit() {
    // The structural update door builds `upd-`, `req-` and `cmd-` from `<correlation>-<id>`, a
    // second site beside `envelope_for`; `redact` goes through `envelope_for` with its own name.
    let project = project("update-verbs");
    let id = story_id(198, 'd');
    let name = id.strip_prefix("story:").expect("a story id");
    ok(
        &project,
        &["new", "story", name, "--title", "Long secretword"],
    );
    let body = project.join("body.md");
    std::fs::write(&body, "A replaced body.\n").expect("the body file is writable");
    let body = body.to_str().expect("a printable path");

    ok(
        &project,
        &[
            "redact",
            &id,
            "--literal",
            "secretword",
            "--with",
            "REDACTED",
            "--reason",
            "a probe",
        ],
    );
    assert!(ok(&project, &["show", &id]).contains("Long REDACTED"));

    ok(&project, &["body", &id, "--from", body]);
    ok(
        &project,
        &["body", &id, "--from", body, "--section", "Outcome"],
    );
    ok(&project, &["set", &id, "--title", "Renamed long"]);
    ok(&project, &["scope", &id, "--add", "crates/x/src/lib.rs"]);

    let shown = ok(&project, &["show", &id]);
    assert!(shown.contains("A replaced body."), "{shown}");
    assert!(shown.contains("## Outcome"), "{shown}");
    assert!(shown.contains("Renamed long"), "{shown}");
    assert!(shown.contains("crates/x/src/lib.rs"), "{shown}");
}

#[test]
fn every_identifier_a_sqlite_store_keeps_for_over_cap_ids_is_valid_and_distinct() {
    let project = sqlite_project("over-cap");
    let id = story_id(198, 'c');
    let body = project.join("body.md");
    std::fs::write(&body, "A new body.\n").expect("the body file is writable");
    let body = body.to_str().expect("a printable path");
    let source = story_id(200, 'a');
    let target = story_id(200, 'b');
    // Two 200-character ids that differ only in their last character, far past any cut.
    let one = format!("story:{}y", "x".repeat(193));
    let two = format!("story:{}z", "x".repeat(193));
    for created in [&id, &source, &target, &one, &two] {
        create(&project, created);
    }

    ok(&project, &["move", &id, "--to", "proposed"]);
    ok(
        &project,
        &[
            "evidence",
            &id,
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-09-28T10:00:00Z",
        ],
    );
    ok(&project, &["body", &id, "--from", body]);
    ok(&project, &["set", &id, "--title", "Renamed"]);
    ok(&project, &["scope", &id, "--add", "crates/x/src/lib.rs"]);
    ok(&project, &["relate", &source, &format!("reviews:{target}")]);
    ok(
        &project,
        &["unrelate", &source, &format!("reviews:{target}")],
    );
    ok(&project, &["relate", &source, &format!("reviews:{one}")]);
    ok(&project, &["relate", &source, &format!("reviews:{two}")]);

    let shown = ok(&project, &["show", &source]);
    assert!(
        shown.contains(&one) && shown.contains(&two),
        "both edges: {shown}"
    );

    let kept = recorded(&project);
    assert_eq!(kept.len(), 14, "{kept:?}");
    for record in &kept {
        assert!(
            aep_domain::ids::IdempotencyKey::new(record.key.as_str()).is_ok(),
            "{record:?}"
        );
        assert!(
            aep_domain::ids::CommandId::new(record.command.as_str()).is_ok(),
            "{record:?}"
        );
        assert!(
            aep_domain::ids::RequestId::new(record.request.as_str()).is_ok(),
            "{record:?}"
        );
        assert!(record.command.starts_with("cmd-"), "{record:?}");
        assert!(record.request.starts_with("req-"), "{record:?}");
    }
    let commands: std::collections::BTreeSet<&str> =
        kept.iter().map(|record| record.command.as_str()).collect();
    assert_eq!(commands.len(), kept.len(), "every command id is its own");
    let keys: std::collections::BTreeSet<&str> =
        kept.iter().map(|record| record.key.as_str()).collect();
    assert_eq!(keys.len(), kept.len(), "every idempotency key is its own");
    for (prefix, count) in [("rel-", 3), ("unrel-", 1), ("upd-", 3), ("new-", 5)] {
        assert_eq!(
            kept.iter()
                .filter(|record| record.key.starts_with(prefix))
                .count(),
            count,
            "{prefix}: {kept:?}"
        );
    }
}

#[test]
fn an_over_cap_command_id_is_the_same_in_every_process_and_across_releases() {
    // A relate's command id carries no clock: the same edge is the same command in every process.
    // The literal is what this fix writes, so a change to the digest or the cut is caught here —
    // and every store that already holds such a record would stop recognising its replays.
    let source = story_id(200, 'a');
    let target = story_id(200, 'b');
    let mut seen = Vec::new();
    for run in ["first", "second"] {
        let project = sqlite_project(&format!("same-command-{run}"));
        create(&project, &source);
        create(&project, &target);
        ok(&project, &["relate", &source, &format!("reviews:{target}")]);
        let kept = recorded(&project);
        let relate = kept
            .iter()
            .find(|record| record.key.starts_with("rel-"))
            .expect("a relate record");
        seen.push(relate.command.clone());
    }
    assert_eq!(seen[0], seen[1]);
    assert_eq!(
        seen[0],
        format!("cmd-story-{}-640cdeaeca912bc6", "a".repeat(173))
    );
}

#[test]
fn relating_a_long_edge_twice_is_nothing_to_do_as_it_is_for_a_short_one() {
    let project = project("relate-twice");
    for (source, target) in [
        (story_id(14, 'g'), story_id(14, 'h')),
        (story_id(200, 'g'), story_id(200, 'h')),
    ] {
        create(&project, &source);
        create(&project, &target);
        let edge = format!("reviews:{target}");
        ok(&project, &["relate", &source, &edge]);
        let again = ok(&project, &["relate", &source, &edge]);
        assert!(again.contains("nothing to do"), "{again}");
    }
}

/// Two Git-native members, `alpha` and `beta`, each declaring both.
fn workspace(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("long-ids-ws-{name}"));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("the previous workspace is removable");
    }
    for (member, other) in [("alpha", "beta"), ("beta", "alpha")] {
        let directory = root.join(member);
        let engineering = directory.join(".engineering");
        std::fs::create_dir_all(engineering.join("planning")).expect("a member store");
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&directory)
            .status()
            .expect("git runs");
        assert!(status.success(), "git init");
        std::fs::write(
            engineering.join("project.yaml"),
            format!(
                "version: aep.project/5\nplanning_scope: {member}\nprotocol: adp/1\n\
                 profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
                relative(&engineering, &repository())
            ),
        )
        .expect("the project file is writable");
        std::fs::write(
            engineering.join("workspace.yaml"),
            format!(
                "version: aep.workspace/1\nmembers:\n  - name: {member}\n    source: ..\n  \
                 - name: {other}\n    source: ../../{other}\n"
            ),
        )
        .expect("the workspace file is writable");
    }
    root
}

#[test]
fn a_crossing_between_two_200_character_ids_is_written_and_taken_back() {
    // A cross-member edge travels as an `UpdateEntity` named `crossings-<id>-<n>-<ms>`.
    let root = workspace("crossing");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    let source = story_id(200, 'a');
    let target = story_id(200, 'b');
    create(&beta, &target);
    create(&alpha, &source);
    let edge = format!("reviews:beta/{target}");

    ok(&alpha, &["relate", &source, &edge]);
    let shown = ok(&alpha, &["show", &source]);
    assert!(shown.contains(&format!("beta/{target}")), "{shown}");

    ok(&alpha, &["unrelate", &source, &edge]);
    let shown = ok(&alpha, &["show", &source]);
    assert!(!shown.contains(&format!("beta/{target}")), "{shown}");
}
