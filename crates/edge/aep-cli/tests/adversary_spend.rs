//! Adversarial cases against `aep plan artifact spend` and the project file's `spend` key
//! (story:spend-against-baseline). Each case asserts what the story, the ESS specification or the
//! `findings_required_since` precedent says, against the behaviour of the tree.

use std::path::{Path, PathBuf};

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
        "status={:?} stdout={} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A scratch repository with an `aep.project/5` project file ending in `spend`, one story and one
/// `task_spend` record of it.
struct Fixture {
    repository: PathBuf,
    store: PathBuf,
}

impl Fixture {
    fn new(name: &str, spend: &str) -> Self {
        let scratch = root()
            .join("target/adversary-spend/fixtures")
            .join(format!("{name}-{}", std::process::id()));
        if scratch.exists() {
            std::fs::remove_dir_all(&scratch).unwrap();
        }
        let repository = scratch.join("repository");
        let store = repository.join(".engineering/planning");
        std::fs::create_dir_all(&store).unwrap();
        let fixture = Self { repository, store };
        fixture.project("");
        let new = cli(&[
            "plan",
            "artifact",
            "new",
            "story",
            "one",
            "--title",
            "one",
            "--store",
            fixture.store(),
        ]);
        assert!(new.status.success(), "{}", text(&new));
        let transcript = fixture.repository.join("runs/one.jsonl");
        std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        std::fs::write(
            &transcript,
            [
                r#"{"type":"system","subtype":"init","session_id":"5c0e7a1b-0000-4000-8000-000000000001","model":"claude-sonnet-5","cwd":"/work","tools":[],"permissionMode":"default"}"#,
                r#"{"type":"assistant","request_id":"req_0","session_id":"5c0e7a1b-0000-4000-8000-000000000001","message":{"model":"claude-sonnet-5","id":"msg_0","type":"message","role":"assistant","content":[{"type":"text","text":"step"}]}}"#,
                r#"{"type":"result","subtype":"success","is_error":false,"num_turns":1,"session_id":"5c0e7a1b-0000-4000-8000-000000000001"}"#,
            ]
            .join("\n")
                + "\n",
        )
        .unwrap();
        let recorded = cli(&[
            "plan",
            "artifact",
            "evidence",
            "story:one",
            "--kind",
            "task_spend",
            "--from",
            transcript.to_str().unwrap(),
            "--at",
            "2026-01-02T10:00:00Z",
            "--store",
            fixture.store(),
        ]);
        assert!(recorded.status.success(), "{}", text(&recorded));
        fixture.project(spend);
        fixture
    }

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

    fn validate(&self) -> std::process::Output {
        cli(&["plan", "artifact", "validate", "--store", self.store()])
    }
}

/// The story: "a value that is not a calendar date is refused, naming it, and no table is
/// printed", and the flag's own help says `YYYY-MM-DD`. `2026-+1-+2` is not written that way, and
/// is read as 2026-01-02.
#[test]
fn adversary_since_with_signed_month_and_day_is_refused() {
    let fixture = Fixture::new("since-signed", "");

    let output = cli(&[
        "plan",
        "artifact",
        "spend",
        "--store",
        fixture.store(),
        "--since",
        "2026-+1-+2",
    ]);

    assert!(
        !output.status.success(),
        "`--since 2026-+1-+2` is not YYYY-MM-DD and must be refused: {}",
        text(&output)
    );
}

/// The precedent the story names, `findings_required_since`, refuses the key written empty or
/// null "naming the key, never read as absent". `spend:` with no value is read as absent.
#[test]
fn adversary_spend_written_empty_is_refused_naming_the_key() {
    let fixture = Fixture::new("spend-empty", "spend:\n");

    let output = fixture.validate();

    assert_eq!(
        output.status.code(),
        Some(1),
        "`spend:` written with no value declares nothing and must be refused, as \
         `findings_required_since:` is: {}",
        text(&output)
    );
    assert!(text(&output).contains("spend"), "{}", text(&output));
}

/// `spend.story:` with no figures under it is read as no declaration at all, silently.
#[test]
fn adversary_spend_kind_written_empty_is_refused_naming_it() {
    let fixture = Fixture::new("spend-kind-empty", "spend:\n  story:\n");

    let output = fixture.validate();

    assert_eq!(
        output.status.code(),
        Some(1),
        "`spend.story:` written with no figure declares nothing and must be refused: {}",
        text(&output)
    );
}

fn raw_spend_bound_schema() -> serde_json::Value {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("schemas/generated/project.schema.json")).unwrap(),
    )
    .unwrap();
    schema["definitions"]["RawSpendBound"].clone()
}

/// The story: `baseline` and `target` are "both non-negative integers and both required". The
/// published schema of the project file makes both optional and nullable.
#[test]
fn adversary_project_schema_requires_baseline_and_target() {
    let bound = raw_spend_bound_schema();
    let required: Vec<&str> = bound["required"]
        .as_array()
        .map(|values| values.iter().filter_map(|value| value.as_str()).collect())
        .unwrap_or_default();

    assert!(
        required.contains(&"baseline") && required.contains(&"target"),
        "schemas/generated/project.schema.json accepts `{{}}` for a spend entry the loader \
         refuses: {bound}"
    );
}

/// The story: a figure is "one of the five above", and any other name refuses the project file.
/// The published schema admits any figure name under a kind.
#[test]
fn adversary_project_schema_admits_only_the_five_figures() {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("schemas/generated/project.schema.json")).unwrap(),
    )
    .unwrap();
    let per_kind = &schema["properties"]["spend"]["additionalProperties"];

    let closed_by_names = per_kind["propertyNames"]["enum"].is_array();
    let closed_by_properties = per_kind["additionalProperties"] == serde_json::Value::Bool(false)
        && per_kind["properties"].get("model_calls").is_some();
    assert!(
        closed_by_names || closed_by_properties,
        "a figure `tokens` validates against the schema and is refused by the loader: {per_kind}"
    );
}
