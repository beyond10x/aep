//! Adversary cases for `store_pack_end_to_end.rs`: what that test does not pin. It never checks that
//! the closed ids it compares are in a pack and gone from the working tree (only a count computed
//! from `pack`'s own report), never checks that an answer it compares was a real answer before
//! packing, compares text output only, and its fixture has no redacted review, no `review_outcome`
//! record and no packed artifact pointing at an open one. This store has each, and the answers are
//! compared in text and in JSON.

use std::collections::BTreeMap;
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

fn aep(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(project)
        .env("AEP_ACTOR", "human:tester")
        .output()
        .expect("the binary runs")
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(project: &Path, args: &[&str]) {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    let output = aep(project, &full);
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        said(&output)
    );
}

fn store_ok(project: &Path, args: &[&str]) -> String {
    let mut full = vec!["plan", "store"];
    full.extend_from_slice(args);
    let output = aep(project, &full);
    assert!(
        output.status.success(),
        "`store {}` failed: {}",
        args.join(" "),
        said(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn evidence(project: &Path, id: &str, at: &str) {
    ok(
        project,
        &[
            "evidence",
            id,
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            at,
        ],
    );
}

fn moves(project: &Path, id: &str, rungs: &[&str]) {
    for rung in rungs {
        ok(project, &["move", id, "--to", rung]);
    }
}

fn review(project: &Path, name: &str, subject: &str, message: &str) {
    let body = project.join("review-body.md");
    std::fs::write(
        &body,
        format!(
            "# Attack\n\n## Findings\n\n```findings\n- file: src/lib.rs\n  line: 10\n  \
             category: correctness\n  severity: blocker\n  verdict: CONFIRMED\n  \
             origin: introduced\n  message: {message}\n```\n"
        ),
    )
    .expect("writable");
    ok(
        project,
        &[
            "new",
            "review-result",
            name,
            "--title",
            name,
            "--relate",
            &format!("reviews:{subject}"),
            "--from",
            body.to_str().expect("a printable path"),
        ],
    );
    std::fs::remove_file(&body).expect("removable");
}

/// The closed artifacts this store holds, each with the loose files it has before packing.
const CLOSED: &[&str] = &[
    "planning/story/impl-a.md",
    "planning/story/impl-b.md",
    "planning/story/rej-a.md",
    "planning/story/arc-a.md",
    "planning/review-result/rev-1.md",
    "planning/review-result/rev-2.md",
    "planning/review-result/rev-open-1.md",
    "planning/review-result/rev-open-2.md",
];

/// The open artifacts, which stay loose.
const OPEN: &[&str] = &["planning/epic/live.md", "planning/story/open-a.md"];

/// A committed Git store: closed stories that decompose an open epic, one with a `review_outcome`
/// record and two archived reviews (one redacted after it was committed), a rejected and an
/// archived story with evidence, and an open story that depends on a closed one and has two
/// archived reviews of its own.
#[allow(clippy::too_many_lines)] // One store, built in the order its history needs.
fn store() -> PathBuf {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store-pack-end-to-end-adversary");
    if project.exists() {
        std::fs::remove_dir_all(&project).expect("the previous scratch project is removable");
    }
    let engineering = project.join(".engineering");
    std::fs::create_dir_all(&engineering).expect("writable");
    git(&project, &["init", "--quiet"]);
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            "version: aep.project/5\nplanning_scope: store-pack-end-to-end-adversary\n\
             protocol: adp/1\nprofile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
            relative(&engineering, &repository())
        ),
    )
    .expect("writable");

    ok(&project, &["new", "epic", "live", "--title", "Live epic"]);
    moves(&project, "epic:live", &["proposed", "active"]);
    for name in ["impl-a", "impl-b"] {
        ok(
            &project,
            &[
                "new",
                "story",
                name,
                "--title",
                name,
                "--relate",
                "decomposes:epic:live",
            ],
        );
        let id = format!("story:{name}");
        moves(&project, &id, &["proposed", "active"]);
        evidence(&project, &id, "2026-09-27T10:00:00Z");
        moves(&project, &id, &["implemented"]);
    }
    ok(&project, &["new", "story", "rej-a", "--title", "rej-a"]);
    moves(&project, "story:rej-a", &["proposed"]);
    evidence(&project, "story:rej-a", "2026-09-27T11:00:00Z");
    moves(&project, "story:rej-a", &["rejected"]);
    ok(&project, &["new", "story", "arc-a", "--title", "arc-a"]);
    moves(&project, "story:arc-a", &["proposed", "active"]);
    evidence(&project, "story:arc-a", "2026-09-27T12:00:00Z");
    moves(&project, "story:arc-a", &["archived"]);

    review(&project, "rev-1", "story:impl-a", "the loop never advances");
    review(
        &project,
        "rev-2",
        "story:impl-a",
        "the Quarkwidget token is logged",
    );
    ok(
        &project,
        &[
            "new",
            "story",
            "open-a",
            "--title",
            "open-a",
            "--relate",
            "decomposes:epic:live",
            "--relate",
            "depends_on:story:impl-a",
        ],
    );
    moves(&project, "story:open-a", &["proposed", "active"]);
    ok(
        &project,
        &["scope", "story:open-a", "--add", "crates/x/src/lib.rs"],
    );
    review(
        &project,
        "rev-open-1",
        "story:open-a",
        "the cache is never invalidated",
    );
    review(
        &project,
        "rev-open-2",
        "story:open-a",
        "the retry has no bound",
    );
    commit(&project, "a loose store");

    ok(
        &project,
        &[
            "redact",
            "review-result:rev-2",
            "--literal",
            "Quarkwidget",
            "--with",
            "[redacted]",
            "--reason",
            "a credential name",
        ],
    );
    ok(
        &project,
        &[
            "evidence",
            "story:impl-a",
            "--kind",
            "review_outcome",
            "--review",
            "review-result:rev-1",
            "--outcome",
            "fixed",
            "--at",
            "2026-09-29T10:00:00Z",
        ],
    );
    for name in ["rev-1", "rev-2", "rev-open-1", "rev-open-2"] {
        moves(&project, &format!("review-result:{name}"), &["archived"]);
    }
    commit(&project, "reviews closed");
    project
}

/// Tracked paths under `.engineering`, each with its blob id.
fn tracked(project: &Path) -> BTreeMap<String, String> {
    git(project, &["ls-files", "-s", ".engineering"])
        .lines()
        .map(|line| {
            let (meta, path) = line.split_once('\t').expect("a ls-files line");
            let oid = meta.split(' ').nth(1).expect("a blob id").to_owned();
            (path.to_owned(), oid)
        })
        .collect()
}

/// The evidence directory of a planning path: `planning/story/x.md` -> `evidence/story/x/`.
fn evidence_of(planning: &str) -> String {
    let rest = planning
        .strip_prefix("planning/")
        .and_then(|rest| rest.strip_suffix(".md"))
        .expect("a planning path");
    format!("evidence/{rest}/")
}

const IDS: &[&str] = &[
    "story:impl-a",
    "story:rej-a",
    "story:arc-a",
    "story:open-a",
    "review-result:rev-2",
    "epic:live",
];

fn calls() -> Vec<Vec<String>> {
    let mut calls: Vec<Vec<String>> = Vec::new();
    for json in [false, true] {
        let mut plain: Vec<Vec<String>> = vec![
            vec!["validate".into()],
            vec!["list".into()],
            vec!["list".into(), "--status".into(), "implemented".into()],
            vec!["graph".into()],
        ];
        for id in IDS {
            for verb in ["show", "history", "explain", "findings"] {
                plain.push(vec![verb.into(), (*id).into()]);
            }
        }
        for mut call in plain {
            if json {
                call.extend(["--format".to_owned(), "json".to_owned()]);
            }
            calls.push(call);
        }
    }
    calls
}

fn answers(project: &Path) -> Vec<(String, Option<i32>, String)> {
    calls()
        .into_iter()
        .map(|call| {
            let mut full = vec!["plan".to_owned(), "artifact".to_owned()];
            full.extend(call.iter().cloned());
            let arguments: Vec<&str> = full.iter().map(String::as_str).collect();
            let output = aep(project, &arguments);
            (
                call.join(" "),
                output.status.code(),
                String::from_utf8_lossy(&output.stdout).into_owned(),
            )
        })
        .collect()
}

#[test]
#[allow(clippy::too_many_lines)] // One store walked through its three points in order.
fn every_compared_answer_is_a_real_answer_and_the_closed_ids_it_names_are_really_packed() {
    let project = store();
    let engineering = project.join(".engineering");
    let loose = tracked(&project);
    let before = answers(&project);

    for (call, code, out) in &before {
        assert_eq!(
            *code,
            Some(0),
            "`{call}` exits 0 over the loose store: {out}"
        );
        assert!(!out.trim().is_empty(), "`{call}` prints something: {out}");
        if call.starts_with("show ")
            || call.starts_with("explain ")
            || call.starts_with("findings ")
        {
            let id = call.split(' ').nth(1).expect("an id");
            assert!(out.contains(id), "`{call}` names {id}: {out}");
        }
    }
    let graph = &before
        .iter()
        .find(|(call, _, _)| call == "graph")
        .expect("graph ran")
        .2;
    for name in ["impl-a", "open-a", "live", "rev-2"] {
        assert!(graph.contains(name), "the graph holds {name}: {graph}");
    }
    let findings = &before
        .iter()
        .find(|(call, _, _)| call == "findings story:open-a")
        .expect("findings ran")
        .2;
    assert!(
        findings.contains("no bound") || findings.contains("never invalidated"),
        "the open story's ledger reads its two archived reviews: {findings}"
    );

    store_ok(&project, &["pack"]);
    commit(&project, "packed");
    let packed = tracked(&project);
    let pack_paths: Vec<&String> = packed
        .keys()
        .filter(|path| {
            Path::new(path)
                .extension()
                .is_some_and(|extension| extension == "pack")
        })
        .collect();
    assert_eq!(pack_paths.len(), 1, "one pack: {packed:?}");
    let pack_text =
        std::fs::read_to_string(project.join(pack_paths[0])).expect("the pack reads as text");

    let mut gone = Vec::new();
    for planning in CLOSED {
        assert!(
            !engineering.join(planning).exists(),
            "{planning} left the working tree"
        );
        assert!(
            pack_text.contains(&format!("\n--- {planning} ")),
            "the pack holds {planning}"
        );
        gone.push((*planning).to_owned());
        let directory = evidence_of(planning);
        for path in loose
            .keys()
            .filter_map(|path| path.strip_prefix(".engineering/"))
            .filter(|path| path.starts_with(&directory))
        {
            assert!(
                !engineering.join(path).exists(),
                "{path} left the working tree"
            );
            assert!(
                pack_text.contains(&format!("\n--- {path} ")),
                "the pack holds {path}"
            );
            gone.push(path.to_owned());
        }
    }
    assert!(
        gone.iter()
            .any(|path| path.starts_with("evidence/review-result/rev-2/")),
        "the redaction record of rev-2 is packed: {gone:?}"
    );
    assert!(
        gone.iter()
            .filter(|path| path.starts_with("evidence/story/impl-a/"))
            .count()
            >= 2,
        "impl-a's test result and its review_outcome record are packed: {gone:?}"
    );
    for planning in OPEN {
        assert!(
            engineering.join(planning).is_file(),
            "{planning} stays loose"
        );
        assert!(
            !pack_text.contains(&format!("\n--- {planning} ")),
            "the pack does not hold {planning}"
        );
    }
    assert_eq!(
        packed.len(),
        loose.len() - gone.len() + 2,
        "after pack: the loose count minus the closed files, plus the pack and its attributes"
    );
    assert_eq!(
        std::fs::read_to_string(engineering.join("project.yaml"))
            .expect("reads")
            .lines()
            .next(),
        Some("version: aep.project/6")
    );
    let after_pack = answers(&project);

    store_ok(&project, &["unpack", "--all"]);
    commit(&project, "unpacked");
    assert_eq!(
        tracked(&project),
        loose,
        "after unpack --all every tracked file is back with the blob it had"
    );
    let after_unpack = answers(&project);

    for (label, found) in [("after pack", &after_pack), ("after unpack", &after_unpack)] {
        for ((call, code, out), (_, found_code, found_out)) in before.iter().zip(found.iter()) {
            assert_eq!(code, found_code, "{label}: exit of `{call}`");
            assert!(
                out == found_out,
                "{label}: stdout of `{call}` differs\n--- before\n{out}\n--- {label}\n{found_out}"
            );
        }
    }
}
