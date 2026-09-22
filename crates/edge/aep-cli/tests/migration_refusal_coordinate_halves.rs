//! Independent verification of `story:migration-mapper-reads-the-declared-workspace`, clause 3.
//!
//! > Red first: a mapper refusal reaches the receipt with the mapper's coordinate (`markdown/graph`
//! > or the document path), in `--format json` as a field and in plain output as text; **the
//! > selector coordinate is used only for selector failures.**
//!
//! The unit's own cases cover the first half. Nothing in the repository asserts the second: at
//! `d6268b91` `rg 'DiagnosticCoordinateV1::Selector'` matches one line, the constructor in
//! `store_command.rs`, and no test at all. A change that routed every refusal through a source
//! coordinate would leave the suite green, so these run both halves through the built binary.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "aep-refusal-coordinate-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A v1 Markdown store whose one story points at `target`.
fn store_at(project: &Path, target: &str, workspace: Option<&str>) {
    let engineering = project.join(".engineering");
    let planning = engineering.join("planning");
    std::fs::create_dir_all(planning.join("story")).expect("planning source");
    std::fs::create_dir_all(project.join("protocols")).expect("protocol source");
    std::fs::write(
        engineering.join("project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
    )
    .expect("legacy selector");
    if let Some(members) = workspace {
        std::fs::write(engineering.join("workspace.yaml"), members).expect("workspace declaration");
    }
    std::fs::write(
        planning.join("story/crossing.md"),
        format!(
            "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\nstatus: draft\n\
             title: A story that names another repository\nrelations:\n\
             - informed_by: {target}\nrevision: 1\n---\n# Story\n\nBody.\n"
        ),
    )
    .expect("crossing planning source");
}

fn dry_run(selector: &Path, format: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args([
            "plan",
            "store",
            "migrate",
            "dry-run",
            "--project",
            &selector.display().to_string(),
            "--authority-scope",
            "planning-review-1",
            "--authority-tenant",
            "tenant-review-1",
            "--authority-new",
            "--format",
            format,
        ])
        .output()
        .expect("the built `aep` binary runs")
}

fn refusal_kind(json: &serde_json::Value) -> &str {
    json["outcome"]["value"]["refusals"][0]["at"]["kind"]
        .as_str()
        .unwrap_or("<absent>")
}

/// Half one, end to end: a refusal the *mapper* produced names the document the mapper read.
///
/// The unit asserts this in-process; this is the same claim measured at the command's own JSON,
/// which is the surface a cutover operator actually reads.
#[test]
fn a_mapper_refusal_reaches_the_receipt_as_a_source_coordinate() {
    let project = scratch("mapper");
    // No workspace declaration, so `other/story:theirs` is a genuine dangling edge.
    store_at(&project, "other/story:theirs", None);
    let output = dry_run(&project.join(".engineering/project.yaml"), "json");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--format json is JSON");

    assert_eq!(json["outcome"]["kind"], "refused", "{json}");
    assert_eq!(
        refusal_kind(&json),
        "source",
        "a mapper refusal names the source, not the selector: {json}"
    );
    assert_eq!(
        json["outcome"]["value"]["refusals"][0]["at"]["value"]["coordinate"]["kind"],
        "markdown_path",
        "{json}"
    );
    let _ = std::fs::remove_dir_all(&project);
}

/// Half two, the converse, which nothing in the repository measures.
///
/// A failure of the **selector** — `--project` naming a file that is not there — must still be
/// reported at the selector coordinate, carrying the path the operator passed. This is the half a
/// fix that re-routes refusals usually breaks, and the suite would not have noticed.
#[test]
fn a_selector_failure_is_still_reported_at_the_selector_coordinate() {
    let project = scratch("selector");
    let absent = project.join(".engineering/project.yaml");
    let output = dry_run(&absent, "json");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--format json is JSON");

    assert_eq!(json["outcome"]["kind"], "refused", "{json}");
    assert_eq!(
        refusal_kind(&json),
        "selector",
        "the selector coordinate is used for selector failures: {json}"
    );
    let rendered = json["outcome"]["value"]["refusals"][0]["at"]["value"]["path"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    // `fold` rather than `map(format!).collect()`: `clippy::format_collect` is denied by the gate.
    let expected = absent.display().to_string().as_bytes().iter().fold(
        "hex:".to_owned(),
        |mut encoded, byte| {
            use std::fmt::Write as _;
            let _ = write!(encoded, "{byte:02x}");
            encoded
        },
    );
    assert_eq!(
        rendered, expected,
        "the selector coordinate carries the path the operator passed: {json}"
    );
    let _ = std::fs::remove_dir_all(&project);
}

/// The same two halves in `--format text`, which is what the cutover runbooks read.
#[test]
fn both_coordinate_halves_survive_the_text_renderer() {
    let mapper = scratch("mapper-text");
    store_at(&mapper, "other/story:theirs", None);
    let text = String::from_utf8(dry_run(&mapper.join(".engineering/project.yaml"), "text").stdout)
        .expect("text output is UTF-8");
    assert!(
        text.lines()
            .any(|line| line.starts_with("outcome.value.refusals[0].at.kind")
                && line.contains("source")),
        "a mapper refusal names the source in plain output: {text}"
    );
    let _ = std::fs::remove_dir_all(&mapper);

    let selector = scratch("selector-text");
    let text =
        String::from_utf8(dry_run(&selector.join(".engineering/project.yaml"), "text").stdout)
            .expect("text output is UTF-8");
    assert!(
        text.lines()
            .any(|line| line.starts_with("outcome.value.refusals[0].at.kind")
                && line.contains("selector")),
        "a selector failure names the selector in plain output: {text}"
    );
    let _ = std::fs::remove_dir_all(&selector);
}
