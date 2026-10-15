//! `aep plan artifact spend [--kind <k>] [--since <date>] [--baseline <n>]` prints spend per task
//! kind and figure against the baseline and target the project file declares under `spend`. It
//! writes nothing and reads no clock.
//!
//! The command is `aep.evidence.ReportSpend` in `ess/domains/evidence.yaml`, and the project key
//! `spend` with its refusal `aep.plan.SpendTargetNotBelowBaseline` is in `ess/domains/plan.yaml`
//! (story:spend-against-baseline).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A committed `eval.run-manifest/1`: it states tokens, cost and wall time, and no model calls.
const MANIFEST: &str =
    "crates/edge/aep-cli/fixtures/eval-matrix/runs/claude-raw-create-a-story.manifest.yaml";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn cli(args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(root())
        .args(args)
        .output()
        .unwrap()
}

fn text(output: &std::process::Output) -> String {
    format!(
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn success(output: &std::process::Output) {
    assert!(output.status.success(), "{}", text(output));
}

/// A scratch repository holding an `aep.project/5` project whose file ends with `spend`, and its
/// Git-native store. Returns `(repository, store)`.
struct Fixture {
    repository: PathBuf,
    store: PathBuf,
}

impl Fixture {
    fn new(name: &str, spend: &str) -> Self {
        let scratch = root()
            .join("target/artifact-spend/fixtures")
            .join(format!("{name}-{}", std::process::id()));
        if scratch.exists() {
            std::fs::remove_dir_all(&scratch).unwrap();
        }
        let repository = scratch.join("repository");
        let store = repository.join(".engineering/planning");
        std::fs::create_dir_all(&store).unwrap();
        let fixture = Self { repository, store };
        fixture.project(spend);
        fixture
    }

    /// Rewrites the project file with `spend` as its last lines.
    fn project(&self, spend: &str) {
        std::fs::write(
            self.repository.join(".engineering/project.yaml"),
            format!(
                "version: aep.project/5\nplanning_scope: spend\nprotocol: adp/1\n\
                 profile: development.standard\n{spend}"
            ),
        )
        .unwrap();
    }

    fn store(&self) -> &str {
        self.store.to_str().unwrap()
    }

    fn new_artifact(&self, kind: &str, name: &str) -> String {
        success(&cli(&[
            "plan",
            "artifact",
            "new",
            kind,
            name,
            "--title",
            name,
            "--store",
            self.store(),
        ]));
        format!("{kind}:{name}")
    }

    /// Writes a transcript at `relative` that made `calls` model calls, and records it as a
    /// `task_spend` of `id` at `at`.
    fn calls(&self, id: &str, relative: &str, calls: u64, at: &str) {
        let path = self.repository.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut lines = vec![
            r#"{"type":"system","subtype":"init","session_id":"5c0e7a1b-0000-4000-8000-000000000001","model":"claude-sonnet-5","cwd":"/work","tools":[],"permissionMode":"default"}"#.to_owned(),
        ];
        for call in 0..calls {
            lines.push(format!(
                r#"{{"type":"assistant","request_id":"req_{call}","session_id":"5c0e7a1b-0000-4000-8000-000000000001","message":{{"model":"claude-sonnet-5","id":"msg_{call}","type":"message","role":"assistant","content":[{{"type":"text","text":"step {call}"}}]}}}}"#
            ));
        }
        lines.push(
            r#"{"type":"result","subtype":"success","is_error":false,"num_turns":1,"session_id":"5c0e7a1b-0000-4000-8000-000000000001"}"#
                .to_owned(),
        );
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        self.record(id, &path, at);
    }

    /// Records the committed run manifest, which states no model calls, as a `task_spend` of `id`.
    fn manifest(&self, id: &str, at: &str) {
        let path = self.repository.join("runs/raw.manifest.yaml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(root().join(MANIFEST), &path).unwrap();
        self.record(id, &path, at);
    }

    fn record(&self, id: &str, from: &Path, at: &str) {
        success(&cli(&[
            "plan",
            "artifact",
            "evidence",
            id,
            "--kind",
            "task_spend",
            "--from",
            from.to_str().unwrap(),
            "--at",
            at,
            "--store",
            self.store(),
        ]));
    }

    fn spend(&self, extra: &[&str]) -> std::process::Output {
        let mut args = vec!["plan", "artifact", "spend", "--store", self.store()];
        args.extend_from_slice(extra);
        cli(&args)
    }

    /// Every row the text table prints, keyed by its kind and figure.
    fn rows(&self, extra: &[&str]) -> BTreeMap<(String, String), Row> {
        let output = self.spend(extra);
        success(&output);
        rows(&String::from_utf8_lossy(&output.stdout))
    }
}

/// One printed row, by column.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    tasks: String,
    skipped: String,
    median: String,
    baseline: String,
    target: String,
    standing: String,
}

/// The rows of the table, read by the header's columns.
fn rows(stdout: &str) -> BTreeMap<(String, String), Row> {
    let mut lines = stdout.lines();
    let header: Vec<&str> = lines
        .by_ref()
        .find(|line| line.split_whitespace().next() == Some("kind"))
        .unwrap_or_else(|| panic!("no header in {stdout}"))
        .split_whitespace()
        .collect();
    assert_eq!(
        header,
        ["kind", "figure", "tasks", "skipped", "median", "baseline", "target", "standing"],
        "{stdout}"
    );
    let mut found = BTreeMap::new();
    for line in lines {
        let cells: Vec<&str> = line.split_whitespace().collect();
        if cells.len() != header.len() {
            continue;
        }
        found.insert(
            (cells[0].to_owned(), cells[1].to_owned()),
            Row {
                tasks: cells[2].to_owned(),
                skipped: cells[3].to_owned(),
                median: cells[4].to_owned(),
                baseline: cells[5].to_owned(),
                target: cells[6].to_owned(),
                standing: cells[7].to_owned(),
            },
        );
    }
    found
}

fn row<'a>(rows: &'a BTreeMap<(String, String), Row>, kind: &str, figure: &str) -> &'a Row {
    rows.get(&(kind.to_owned(), figure.to_owned()))
        .unwrap_or_else(|| panic!("no {kind}/{figure} row in {rows:?}"))
}

/// Three stories stating 8, 12 and 20 model calls on 2026-01-01, -02 and -03, and one task whose
/// record states none.
fn three_stories_and_a_task(name: &str, spend: &str) -> Fixture {
    let fixture = Fixture::new(name, spend);
    for (story, calls, day) in [("one", 8, "01"), ("two", 12, "02"), ("three", 20, "03")] {
        let id = fixture.new_artifact("story", story);
        fixture.calls(
            &id,
            &format!("runs/{story}.jsonl"),
            calls,
            &format!("2026-01-{day}T10:00:00Z"),
        );
    }
    let task = fixture.new_artifact("task", "four");
    fixture.manifest(&task, "2026-01-02T12:00:00Z");
    fixture
}

const DECLARED: &str = "spend:\n  story:\n    model_calls: { baseline: 25, target: 10 }\n";

#[test]
fn story_model_calls_row_reports_tasks_median_baseline_target_and_standing() {
    let fixture = three_stories_and_a_task("row", DECLARED);

    let output = fixture.spend(&[]);
    success(&output);
    let rows = rows(&String::from_utf8_lossy(&output.stdout));

    assert_eq!(
        row(&rows, "story", "model_calls"),
        &Row {
            tasks: "3".into(),
            skipped: "0".into(),
            median: "12".into(),
            baseline: "25".into(),
            target: "10".into(),
            standing: "over".into(),
        },
        "the middle of 8, 12 and 20 is 12, over the target 10"
    );
    let task = row(&rows, "task", "model_calls");
    assert_eq!(
        (&*task.tasks, &*task.skipped, &*task.median, &*task.standing),
        ("1", "1", "unknown", "unknown"),
        "a task whose record states no model calls is skipped, and its median is unknown, never 0"
    );
    let figures: Vec<&str> = rows
        .keys()
        .filter(|(kind, _)| kind == "story")
        .map(|(_, figure)| figure.as_str())
        .collect();
    assert_eq!(figures.len(), 5, "one row per figure: {figures:?}");
}

#[test]
fn standing_follows_the_target_and_is_undeclared_without_one() {
    let fixture = three_stories_and_a_task("standing", DECLARED);

    fixture.project("spend:\n  story:\n    model_calls: { baseline: 25, target: 15 }\n");
    assert_eq!(
        row(&fixture.rows(&[]), "story", "model_calls").standing,
        "under"
    );

    fixture.project("spend:\n  story:\n    model_calls: { baseline: 25, target: 12 }\n");
    assert_eq!(
        row(&fixture.rows(&[]), "story", "model_calls").standing,
        "at"
    );

    fixture.project("");
    let rows = fixture.rows(&[]);
    let story = row(&rows, "story", "model_calls");
    assert_eq!(
        (&*story.baseline, &*story.target, &*story.standing),
        ("none", "none", "undeclared"),
        "{story:?}"
    );
}

#[test]
fn baseline_takes_the_median_over_the_first_n_tasks_of_each_kind() {
    let fixture = three_stories_and_a_task("baseline", DECLARED);

    let rows = fixture.rows(&["--baseline", "2"]);

    let story = row(&rows, "story", "model_calls");
    assert_eq!(
        story.median, "8",
        "the records of 01-01 and 01-02, and the lower middle of 8 and 12: {story:?}"
    );
}

#[test]
fn records_sharing_a_source_input_count_once_by_latest_at() {
    let fixture = Fixture::new("source-input", "");
    let id = fixture.new_artifact("story", "shared");
    fixture.calls(&id, "runs/a.jsonl", 3, "2026-01-01T10:00:00Z");
    fixture.calls(&id, "runs/b.jsonl", 4, "2026-01-02T10:00:00Z");

    let story = row(&fixture.rows(&[]), "story", "model_calls").clone();
    assert_eq!(
        (&*story.tasks, &*story.median),
        ("1", "7"),
        "two sources of one story are summed, and the story counts once: {story:?}"
    );

    fixture.calls(&id, "runs/c.jsonl", 5, "2026-01-04T10:00:00Z");
    fixture.calls(&id, "runs/c.jsonl", 9, "2026-01-05T10:00:00Z");

    let story = row(&fixture.rows(&[]), "story", "model_calls").clone();
    assert_eq!(
        (&*story.tasks, &*story.median),
        ("1", "16"),
        "two snapshots of one source count as the latest, 3 + 4 + 9, not 21: {story:?}"
    );
}

#[test]
fn kind_and_since_filter_rows_and_records() {
    let fixture = three_stories_and_a_task("filters", DECLARED);

    let only_stories = fixture.rows(&["--kind", "story"]);
    assert!(
        only_stories.keys().all(|(kind, _)| kind == "story"),
        "--kind story prints no task row: {only_stories:?}"
    );
    assert!(only_stories.contains_key(&("story".to_owned(), "model_calls".to_owned())));

    let since = fixture.rows(&["--since", "2026-01-02"]);
    let story = row(&since, "story", "model_calls");
    assert_eq!(
        (&*story.tasks, &*story.median),
        ("2", "12"),
        "the records of 01-02 and 01-03, and the lower middle of 12 and 20: {story:?}"
    );
}

#[test]
fn a_since_that_is_not_a_calendar_date_is_refused_naming_it_and_prints_no_table() {
    let fixture = three_stories_and_a_task("since-refused", DECLARED);

    let output = fixture.spend(&["--since", "2026-13-40"]);

    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("2026-13-40"),
        "the refusal names the value: {}",
        text(&output)
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("figure"),
        "no table is printed: {}",
        text(&output)
    );
}

#[test]
fn a_target_not_below_its_baseline_is_refused_by_validate_naming_kind_figure_and_values() {
    let fixture = three_stories_and_a_task("not-below", DECLARED);
    fixture.project("spend:\n  story:\n    model_calls: { baseline: 25, target: 25 }\n");

    let output = cli(&["plan", "artifact", "validate", "--store", fixture.store()]);

    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    let printed = text(&output);
    for named in ["story", "model_calls", "25"] {
        assert!(
            printed.contains(named),
            "the refusal names `{named}`: {printed}"
        );
    }
    assert!(
        printed.contains("target 25") && printed.contains("baseline 25"),
        "the refusal names both values: {printed}"
    );
}

#[test]
fn an_unknown_figure_name_is_refused_naming_it() {
    let fixture = three_stories_and_a_task("unknown-figure", DECLARED);
    fixture.project("spend:\n  story:\n    tokens: { baseline: 25, target: 10 }\n");

    let output = cli(&["plan", "artifact", "validate", "--store", fixture.store()]);

    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    assert!(
        text(&output).contains("`tokens`"),
        "the refusal names the figure: {}",
        text(&output)
    );
}

#[test]
fn two_runs_over_one_store_print_identical_bytes() {
    let fixture = three_stories_and_a_task("deterministic", DECLARED);
    let before = tree(&fixture.repository);

    let first = fixture.spend(&[]);
    let second = fixture.spend(&[]);

    success(&first);
    assert_eq!(first.stdout, second.stdout);
    let json_first = fixture.spend(&["--format", "json"]);
    let json_second = fixture.spend(&["--format", "json"]);
    success(&json_first);
    assert_eq!(json_first.stdout, json_second.stdout);
    let after = tree(&fixture.repository);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>(),
        "the verb creates and removes no path in the repository"
    );
    for (path, bytes) in &before {
        assert!(
            after.get(path) == Some(bytes),
            "the verb rewrote {}",
            path.display()
        );
    }
}

/// Every path under `directory`, directories included, with each file's bytes.
fn tree(directory: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(directory).unwrap().to_path_buf();
            if path.is_dir() {
                found.insert(relative, Vec::new());
                pending.push(path);
            } else {
                found.insert(relative, std::fs::read(&path).unwrap());
            }
        }
    }
    found
}

#[test]
fn a_baseline_of_zero_tasks_is_refused_naming_the_flag_and_prints_no_table() {
    let fixture = three_stories_and_a_task("baseline-zero", DECLARED);

    let output = fixture.spend(&["--baseline", "0"]);

    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--baseline 0"),
        "the refusal names the flag and its value: {}",
        text(&output)
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("figure"),
        "no table is printed: {}",
        text(&output)
    );
}

#[test]
fn a_since_with_a_signed_part_is_refused_naming_it() {
    let fixture = three_stories_and_a_task("since-signed", DECLARED);

    for written in ["2026-+1-+2", "+026-01-01"] {
        let output = fixture.spend(&["--since", written]);
        assert!(!output.status.success(), "{written}: {}", text(&output));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(written),
            "the refusal names the value: {}",
            text(&output)
        );
    }
}
