//! The `roadmap` kind: an ordered list of epics with an approval rung, checked by `validate`.
//!
//! Every case runs the built binary over a fresh Git-native project whose document tree is this
//! repository, so the ladder and the template under test are the ones in `artifacts/`.

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

/// A fresh Git-native project named `name`, under `profile`.
fn project_with_profile(name: &str, profile: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("roadmap-kind-{name}"));
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
            "version: aep.project/5\nplanning_scope: roadmap-kind-test\nprotocol: adp/1\n\
             profile: {profile}\nprotocols: {}\nstore:\n  git: {{}}\n",
            relative(&engineering, &repository())
        ),
    )
    .expect("the project file is writable");
    directory
}

fn project(name: &str) -> PathBuf {
    project_with_profile(name, "development.standard")
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

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(directory: &Path, args: &[&str]) -> String {
    let output = aep(directory, args);
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        text(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn refused(directory: &Path, args: &[&str]) -> String {
    let output = aep(directory, args);
    assert!(
        !output.status.success(),
        "`{}` was expected to be refused and succeeded: {}",
        args.join(" "),
        text(&output)
    );
    text(&output)
}

/// A file in the project holding `content`, for a `--from`.
fn file(project: &Path, name: &str, content: &str) -> String {
    let path = project.join(name);
    std::fs::write(&path, content).expect("the input file is writable");
    path.display().to_string()
}

/// A roadmap body whose block lists `entries`, each `(epic, horizon)`.
fn roadmap_body(entries: &[(&str, &str)]) -> String {
    let mut block = String::new();
    for (epic, horizon) in entries {
        block.push_str(&format!(
            "- epic: {epic}\n  horizon: {horizon}\n  rationale: because\n"
        ));
    }
    format!("# Roadmap\n\n```roadmap\n{block}```\n")
}

fn epic(project: &Path, name: &str, status: &str) {
    ok(project, &["new", "epic", name, "--title", "An epic"]);
    if status != "draft" {
        ok(
            project,
            &["move", &format!("epic:{name}"), "--to", status, "--via"],
        );
    }
}

fn roadmap(project: &Path, name: &str, entries: &[(&str, &str)]) {
    let body = file(project, &format!("{name}.md"), &roadmap_body(entries));
    ok(
        project,
        &[
            "new",
            "roadmap",
            name,
            "--title",
            "A roadmap",
            "--from",
            &body,
        ],
    );
}

/// Takes `roadmap:<name>` to `approved` the way the ladder asks: review, an approval record, and
/// the approval-record artifact that says who decided.
fn approve(project: &Path, name: &str) {
    let id = format!("roadmap:{name}");
    ok(project, &["move", &id, "--to", "in_review"]);
    ok(
        project,
        &[
            "evidence",
            &id,
            "--kind",
            "approval",
            "--source",
            &format!("approval-record:{name}-approved"),
        ],
    );
    ok(
        project,
        &[
            "new",
            "approval-record",
            &format!("{name}-approved"),
            "--title",
            "Approved",
            "--relate",
            &format!("reviews:{id}"),
        ],
    );
    ok(project, &["move", &id, "--to", "approved"]);
}

fn validate(project: &Path, extra: &[&str]) -> (bool, String) {
    let mut args = vec!["validate"];
    args.extend_from_slice(extra);
    let output = aep(project, &args);
    (output.status.success(), text(&output))
}

#[test]
fn a_new_roadmap_is_a_draft_seeded_from_its_template_and_works_with_the_common_verbs() {
    let project = project("common-verbs");
    ok(
        &project,
        &["new", "vision", "direction", "--title", "Direction"],
    );
    ok(&project, &["new", "roadmap", "plan", "--title", "Plan"]);

    let shown = ok(&project, &["show", "roadmap:plan"]);
    assert!(
        shown.contains("draft"),
        "a roadmap starts at draft: {shown}"
    );
    assert!(
        shown.contains("```roadmap"),
        "the template seeds a roadmap block: {shown}"
    );

    ok(
        &project,
        &["relate", "roadmap:plan", "derived_from:vision:direction"],
    );
    assert!(ok(&project, &["show", "roadmap:plan"]).contains("vision:direction"));

    epic(&project, "first", "draft");
    let body = file(&project, "plan.md", &roadmap_body(&[("epic:first", "now")]));
    ok(&project, &["body", "roadmap:plan", "--from", &body]);
    assert!(ok(&project, &["show", "roadmap:plan"]).contains("epic:first"));

    let listed = ok(&project, &["list", "--kind", "roadmap"]);
    assert!(listed.contains("roadmap:plan"), "listed: {listed}");
    assert!(!listed.contains("epic:first"), "only roadmaps: {listed}");

    let kinds = ok(&project, &["kinds"]);
    assert!(
        kinds.contains("roadmap"),
        "the vocabulary names it: {kinds}"
    );

    let (valid, said) = validate(&project, &[]);
    assert!(valid, "a draft roadmap over a draft epic is valid: {said}");
}

#[test]
fn approved_is_refused_until_an_approval_is_recorded() {
    let project = project("approval-rung");
    epic(&project, "first", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    ok(&project, &["move", "roadmap:plan", "--to", "in_review"]);

    let said = refused(&project, &["move", "roadmap:plan", "--to", "approved"]);
    assert!(
        said.contains("approval"),
        "the refusal names the missing approval: {said}"
    );

    ok(
        &project,
        &[
            "evidence",
            "roadmap:plan",
            "--kind",
            "approval",
            "--source",
            "approval-record:plan-approved",
        ],
    );
    ok(&project, &["move", "roadmap:plan", "--to", "approved"]);
}

#[test]
fn an_approved_roadmap_that_lists_every_live_epic_once_is_valid() {
    let project = project("valid");
    epic(&project, "first", "active");
    epic(&project, "second", "proposed");
    roadmap(
        &project,
        "plan",
        &[("epic:first", "now"), ("epic:second", "next")],
    );
    approve(&project, "plan");

    let (valid, said) = validate(&project, &["--strict"]);
    assert!(valid, "{said}");
    assert!(!said.contains("on no approved roadmap"), "{said}");
}

#[test]
fn an_entry_naming_no_artifact_is_a_problem() {
    let project = project("missing-epic");
    roadmap(&project, "plan", &[("epic:nowhere", "now")]);

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("roadmap:plan") && said.contains("epic:nowhere") && said.contains("no such"),
        "the problem names the roadmap and the missing epic: {said}"
    );
}

#[test]
fn an_entry_naming_an_artifact_that_is_not_an_epic_is_a_problem() {
    let project = project("not-an-epic");
    ok(&project, &["new", "story", "small", "--title", "Small"]);
    roadmap(&project, "plan", &[("story:small", "now")]);

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("story:small") && said.contains("not an epic"),
        "{said}"
    );
}

#[test]
fn an_epic_listed_twice_is_a_problem() {
    let project = project("twice");
    epic(&project, "first", "draft");
    roadmap(
        &project,
        "plan",
        &[("epic:first", "now"), ("epic:first", "later")],
    );

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("epic:first") && said.contains("twice"),
        "{said}"
    );
}

#[test]
fn a_horizon_that_goes_back_down_the_list_is_a_problem() {
    let project = project("horizon-order");
    epic(&project, "first", "draft");
    epic(&project, "second", "draft");
    roadmap(
        &project,
        "plan",
        &[("epic:first", "later"), ("epic:second", "now")],
    );

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("epic:second") && said.contains("after") && said.contains("later"),
        "the problem says which entry goes back and behind what: {said}"
    );
}

#[test]
fn every_defect_of_one_roadmap_is_reported_together() {
    let project = project("accumulates");
    ok(&project, &["new", "story", "small", "--title", "Small"]);
    epic(&project, "first", "draft");
    roadmap(
        &project,
        "plan",
        &[
            ("epic:first", "now"),
            ("epic:first", "next"),
            ("epic:nowhere", "next"),
            ("story:small", "later"),
        ],
    );

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("3 problem(s)"),
        "three independent defects: {said}"
    );
}

#[test]
fn an_unreadable_block_is_a_problem() {
    let project = project("unreadable");
    let body = file(
        &project,
        "plan.md",
        "# Roadmap\n\n```roadmap\n- epic: epic:first\n  horizon: soon\n```\n",
    );
    ok(
        &project,
        &["new", "roadmap", "plan", "--title", "Plan", "--from", &body],
    );

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("roadmap:plan") && said.contains("soon"),
        "the problem quotes the value outside the vocabulary: {said}"
    );
}

#[test]
fn an_approved_roadmap_that_no_approval_record_reviews_is_a_problem() {
    let project = project("no-approval-record");
    epic(&project, "first", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    ok(&project, &["move", "roadmap:plan", "--to", "in_review"]);
    ok(
        &project,
        &[
            "evidence",
            "roadmap:plan",
            "--kind",
            "approval",
            "--source",
            "operator, in chat",
        ],
    );
    ok(&project, &["move", "roadmap:plan", "--to", "approved"]);

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("roadmap:plan") && said.contains("approval-record"),
        "{said}"
    );
}

#[test]
fn a_second_approved_roadmap_is_a_problem() {
    let project = project("two-approved");
    epic(&project, "first", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    roadmap(&project, "other", &[("epic:first", "now")]);
    approve(&project, "plan");
    approve(&project, "other");

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("roadmap:other")
            && said.contains("roadmap:plan")
            && said.contains("roadmap.approved.limit"),
        "the problem names both and the fact that would allow it: {said}"
    );
}

#[test]
fn a_superseded_roadmap_leaves_one_approved() {
    let project = project("superseded");
    epic(&project, "first", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    approve(&project, "plan");
    let body = file(
        &project,
        "next.md",
        &roadmap_body(&[("epic:first", "next")]),
    );
    ok(
        &project,
        &[
            "new",
            "roadmap",
            "next",
            "--title",
            "Next",
            "--from",
            &body,
            "--relate",
            "supersedes:roadmap:plan",
        ],
    );
    approve(&project, "next");
    ok(&project, &["move", "roadmap:plan", "--to", "superseded"]);

    let (valid, said) = validate(&project, &[]);
    assert!(valid, "{said}");
}

#[test]
fn a_profile_that_declares_the_limit_allows_two_approved_roadmaps() {
    let project = project_with_profile("profile-limit", "roadmaps.two");
    let profiles = project.join(".engineering/profiles");
    std::fs::create_dir_all(&profiles).expect("the profile directory is writable");
    std::fs::write(
        profiles.join("roadmaps-two.yaml"),
        "id: roadmaps.two\nversion: 1\ntitle: Two roadmaps\nprotocol: adp/1\n\
         extends: development.standard\nfacts:\n  roadmap.approved.limit: 2\n",
    )
    .expect("the profile is writable");
    epic(&project, "first", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    roadmap(&project, "other", &[("epic:first", "now")]);
    approve(&project, "plan");
    approve(&project, "other");

    let (valid, said) = validate(&project, &[]);
    assert!(valid, "the profile allows two: {said}");
}

#[test]
fn a_live_epic_on_no_approved_roadmap_is_reported_and_refused_only_under_strict() {
    let project = project("off-roadmap");
    epic(&project, "first", "active");
    epic(&project, "second", "proposed");
    epic(&project, "third", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);

    // No approved roadmap yet: the rule waits for the first one.
    let (valid, said) = validate(&project, &["--strict"]);
    assert!(
        valid,
        "nothing is reported before a roadmap is approved: {said}"
    );
    assert!(!said.contains("on no approved roadmap"), "{said}");

    approve(&project, "plan");
    let (valid, said) = validate(&project, &[]);
    assert!(valid, "reported, not a problem: {said}");
    assert!(
        said.contains("1 epic(s) on no approved roadmap") && said.contains("epic:second"),
        "the proposed epic is listed: {said}"
    );
    assert!(
        !said.contains("epic:third is"),
        "a draft epic is not listed: {said}"
    );

    let (valid, said) = validate(&project, &["--strict"]);
    assert!(!valid, "--strict refuses it: {said}");
    assert!(said.contains("on no approved roadmap"), "{said}");
}

#[test]
fn body_refuses_to_rewrite_an_approved_roadmap() {
    let project = project("frozen");
    epic(&project, "first", "draft");
    epic(&project, "second", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    approve(&project, "plan");
    let path = project.join(".engineering/planning/roadmap/plan.md");
    let before = std::fs::read(&path).expect("the roadmap is on disk");

    let body = file(
        &project,
        "changed.md",
        &roadmap_body(&[("epic:first", "now"), ("epic:second", "next")]),
    );
    let said = refused(&project, &["body", "roadmap:plan", "--from", &body]);
    assert!(
        said.contains("approved") && said.contains("supersedes"),
        "the refusal names the route: a successor that supersedes it: {said}"
    );
    assert_eq!(
        before,
        std::fs::read(&path).expect("the roadmap is on disk"),
        "a refusal changes nothing"
    );
}

#[test]
fn waves_are_the_same_with_and_without_a_roadmap() {
    let project = project("waves");
    ok(&project, &["new", "story", "small", "--title", "Small"]);
    ok(&project, &["move", "story:small", "--to", "proposed"]);
    ok(&project, &["scope", "story:small", "--add", "src/lib.rs"]);
    let before = ok(&project, &["waves"]);

    epic(&project, "first", "active");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    approve(&project, "plan");

    assert_eq!(before, ok(&project, &["waves"]));
}
