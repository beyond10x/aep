//! A tree store's projection is rendered as `aep.planning-md/2`, and holds no store-wide file.
//!
//! Design § 3.2 and § 13 A5. S5 holds each document to the render of its **own** `format:` tag
//! (§ 8), so a projection written before the renderer moved — every document still `/1` — still
//! validates until it is re-rendered, and a `/1` document that is not its render is still refused.

use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// `to` written relative to `from`, which is how a project names its protocol tree.
fn relative(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<Component<'_>> = from.components().collect();
    let to: Vec<Component<'_>> = to.components().collect();
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut path = PathBuf::new();
    for _ in shared..from.len() {
        path.push("..");
    }
    for component in &to[shared..] {
        path.push(component);
    }
    path
}

fn aep(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(project)
        .output()
        .expect("the aep binary runs")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A tree store with one story, created by this binary.
fn tree_with_one_story(name: &str) -> PathBuf {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tree-format-{name}"));
    let _ = std::fs::remove_dir_all(&project);
    let engineering = project.join(".engineering");
    std::fs::create_dir_all(&engineering).expect("the .engineering directory");
    let protocols = relative(&engineering, &workspace());
    let init = aep(
        &project,
        &[
            "plan",
            "store",
            "init-tree",
            "--engineering",
            ".engineering",
            "--scope",
            "fixture",
            "--protocols",
            protocols.to_str().expect("a UTF-8 path"),
        ],
    );
    assert!(init.status.success(), "{}", text(&init));
    let created = aep(
        &project,
        &[
            "plan",
            "artifact",
            "new",
            "story",
            "fixture-one",
            "--title",
            "Fixture one",
        ],
    );
    assert!(created.status.success(), "{}", text(&created));
    project
}

fn story(project: &Path) -> PathBuf {
    project.join(".engineering/planning/story/fixture-one.md")
}

/// Every file under `root`, relative to it.
fn files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("readable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push(
                    path.strip_prefix(root)
                        .expect("under root")
                        .display()
                        .to_string(),
                );
            }
        }
    }
    found.sort();
    found
}

#[test]
fn a_tree_store_renders_its_projection_in_the_second_format_with_no_store_wide_file() {
    let project = tree_with_one_story("second");
    let document = std::fs::read_to_string(story(&project)).expect("the story is rendered");
    assert!(
        document.starts_with("---\nformat: aep.planning-md/2\n"),
        "a tree store's projection is `aep.planning-md/2`:\n{document}"
    );
    assert_eq!(
        files(&project.join(".engineering/planning")),
        vec!["story/fixture-one.md".to_owned()],
        "the projection holds one document per artifact and no store-wide file"
    );

    let validated = aep(&project, &["plan", "artifact", "validate"]);
    assert!(validated.status.success(), "{}", text(&validated));
}

#[test]
fn a_projection_still_in_the_first_format_validates_until_it_is_re_rendered() {
    let project = tree_with_one_story("first");
    let path = story(&project);
    let second = std::fs::read_to_string(&path).expect("the story is rendered");
    let first = second.replacen(
        "format: aep.planning-md/2\n",
        "format: aep.planning-md/1\n",
        1,
    );
    assert_ne!(first, second, "the fixture rewrote the tag");
    std::fs::write(&path, &first).expect("the /1 document written");

    let validated = aep(&project, &["plan", "artifact", "validate"]);
    assert!(
        validated.status.success(),
        "S5 holds a document to the render of its own tag, and this is the /1 render:\n{}",
        text(&validated)
    );

    std::fs::write(&path, first.replace("Fixture one", "Edited by hand")).expect("edited");
    let validated = aep(&project, &["plan", "artifact", "validate"]);
    let out = text(&validated);
    assert!(
        !validated.status.success(),
        "a /1 document is still held to S5:\n{out}"
    );
    assert!(
        out.contains("S5: story/fixture-one.md is not the render of its artifact"),
        "{out}"
    );

    let rendered = aep(&project, &["plan", "artifact", "render"]);
    assert!(rendered.status.success(), "{}", text(&rendered));
    assert_eq!(
        std::fs::read_to_string(&path).expect("re-rendered"),
        second,
        "the re-render step writes the second format"
    );
    let validated = aep(&project, &["plan", "artifact", "validate"]);
    assert!(validated.status.success(), "{}", text(&validated));
}

/// Copy `from` into `to`, every file and directory, for a fixture a test may write to.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the copy's directory");
    for entry in std::fs::read_dir(from).expect("the fixture directory reads") {
        let entry = entry.expect("a fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a fixture entry's type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a fixture file copies");
        }
    }
}

/// A store whose last record is the projection watermark an older release wrote, rendered in
/// the first format, is re-rendered by `render` in the second. The watermark's inventory is the
/// older renderer's, so re-staging it can never match; nothing was recorded after it, so the
/// only difference is the renderer, and `render` publishes the current state instead of refusing
/// it as drift.
#[test]
fn render_upgrades_a_projection_an_older_release_published_last() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tree-rendered-by-0-58-0");
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("tree-format-older-release");
    let _ = std::fs::remove_dir_all(&project);
    copy_tree(&fixture, &project);
    let engineering = project.join(".engineering");
    let selector = engineering.join("project.yaml");
    let mut document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&selector).expect("the selector reads"))
            .expect("the selector is JSON");
    document["protocols"] =
        serde_json::Value::String(relative(&engineering, &workspace()).display().to_string());
    std::fs::write(&selector, document.to_string()).expect("the selector writes");
    let story = engineering.join("planning/story/rendered-by-an-older-release.md");
    assert!(
        std::fs::read_to_string(&story)
            .expect("the fixture's story")
            .contains("format: aep.planning-md/1"),
        "the fixture no longer holds a first-format projection, so this case tests nothing"
    );

    let render = aep(&project, &["plan", "artifact", "render"]);
    assert!(
        render.status.success(),
        "render refused a projection an older release published: {}",
        text(&render)
    );
    assert!(
        std::fs::read_to_string(&story)
            .expect("the re-rendered story")
            .contains("format: aep.planning-md/2"),
        "render left the story in the first format"
    );
    let validate = aep(&project, &["plan", "artifact", "validate"]);
    assert!(
        validate.status.success(),
        "the re-rendered store does not validate: {}",
        text(&validate)
    );
    let again = aep(&project, &["plan", "artifact", "render"]);
    assert!(
        again.status.success(),
        "a second render refused the projection the first one published: {}",
        text(&again)
    );
    let left: Vec<String> = std::fs::read_dir(&engineering)
        .expect("the .engineering directory reads")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("aep-stage"))
        .collect();
    assert!(
        left.is_empty(),
        "render left a stage directory beside the projection: {left:?}"
    );
}
