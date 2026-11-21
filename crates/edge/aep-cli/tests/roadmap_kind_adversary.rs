//! Adversary cases for `story:roadmap-kind`: what `roadmap_kind.rs` does not say.
//!
//! Same project setup as `roadmap_kind.rs`: the built binary over a fresh Git-native project whose
//! document tree is this repository.

use std::fmt::Write as _;
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

fn project_with_profile(name: &str, profile: &str) -> PathBuf {
    let directory =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("roadmap-kind-adversary-{name}"));
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
            "version: aep.project/5\nplanning_scope: roadmap-kind-adversary\nprotocol: adp/1\n\
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

/// A project under a local profile that declares `roadmap.approved.limit: <value>`.
fn project_with_limit(name: &str, value: &str) -> PathBuf {
    let project = project_with_profile(name, "roadmaps.limited");
    let profiles = project.join(".engineering/profiles");
    std::fs::create_dir_all(&profiles).expect("the profile directory is writable");
    std::fs::write(
        profiles.join("roadmaps-limited.yaml"),
        format!(
            "id: roadmaps.limited\nversion: 1\ntitle: Limited\nprotocol: adp/1\n\
             extends: development.standard\nfacts:\n  roadmap.approved.limit: {value}\n"
        ),
    )
    .expect("the profile is writable");
    project
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

fn file(project: &Path, name: &str, content: &str) -> String {
    let path = project.join(name);
    std::fs::write(&path, content).expect("the input file is writable");
    path.display().to_string()
}

fn roadmap_body(entries: &[(&str, &str)]) -> String {
    let mut block = String::new();
    for (epic, horizon) in entries {
        let _ = writeln!(block, "- epic: {epic}\n  horizon: {horizon}");
    }
    format!("# Roadmap\n\n```roadmap\n{block}```\n")
}

fn artifact(project: &Path, kind: &str, name: &str, status_path: &[&str]) {
    ok(project, &["new", kind, name, "--title", "An artifact"]);
    for status in status_path {
        ok(
            project,
            &["move", &format!("{kind}:{name}"), "--to", status, "--via"],
        );
    }
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

fn roadmap_of_kind(project: &Path, kind: &str, name: &str, body: &str) {
    let body = file(project, &format!("{kind}-{name}.md"), body);
    ok(
        project,
        &["new", kind, name, "--title", "A roadmap", "--from", &body],
    );
}

fn roadmap_with(project: &Path, name: &str, body: &str) {
    roadmap_of_kind(project, "roadmap", name, body);
}

fn roadmap(project: &Path, name: &str, entries: &[(&str, &str)]) {
    roadmap_with(project, name, &roadmap_body(entries));
}

/// `<kind>:<name>` to `approved`: review, an approval record, then an artifact of `recorder_kind`
/// that relates to it with `relation`.
fn approve_with(project: &Path, kind: &str, name: &str, recorder_kind: &str, relation: &str) {
    let id = format!("{kind}:{name}");
    ok(project, &["move", &id, "--to", "in_review"]);
    ok(
        project,
        &[
            "evidence",
            &id,
            "--kind",
            "approval",
            "--source",
            &format!("{recorder_kind}:{name}-approved"),
        ],
    );
    ok(
        project,
        &[
            "new",
            recorder_kind,
            &format!("{name}-approved"),
            "--title",
            "Approved",
            "--relate",
            &format!("{relation}:{id}"),
        ],
    );
    ok(project, &["move", &id, "--to", "approved"]);
}

fn approve(project: &Path, name: &str) {
    approve_with(project, "roadmap", name, "approval-record", "reviews");
}

fn validate(project: &Path, extra: &[&str]) -> (bool, String) {
    let mut args = vec!["validate"];
    args.extend_from_slice(extra);
    let output = aep(project, &args);
    (output.status.success(), text(&output))
}

// ---------------------------------------------------------------------------------------------
// Red at 9dc5ea225: the kind hierarchy. `ArtifactKind::parent` makes `platform-epic` an epic and
// `team-roadmap` a roadmap (the CLI accepts both, and `team-roadmap` gets the roadmap ladder with
// its approval rung). `roadmaps.rs` asked `is_a(Epic)` in one place and `namespace() == "epic"` /
// `kind == Roadmap` in the others.
// ---------------------------------------------------------------------------------------------

/// `off_roadmap` lists a live `platform-epic` (it asks `is_a(Epic)`), and the only repair it names
/// — a roadmap that lists it — is refused as "not an epic". `--strict` can never pass.
#[test]
fn an_epic_subkind_the_off_roadmap_rule_lists_can_be_placed_on_a_roadmap() {
    let project = project("epic-subkind");
    artifact(&project, "platform-epic", "big", &["proposed"]);
    roadmap(&project, "plan", &[("platform-epic:big", "now")]);
    approve(&project, "plan");

    let (valid, said) = validate(&project, &["--strict"]);
    assert!(
        valid,
        "a roadmap listing the one live epic (a platform-epic) is the repair off_roadmap names; \
         it must validate: {said}"
    );
}

/// A `team-roadmap` climbs the roadmap ladder (approval rung included) and is then outside every
/// roadmap rule: `body` rewrites it once approved.
#[test]
fn body_refuses_to_rewrite_an_approved_roadmap_subkind() {
    let project = project("roadmap-subkind-frozen");
    epic(&project, "first", "draft");
    epic(&project, "second", "draft");
    roadmap_of_kind(
        &project,
        "team-roadmap",
        "plan",
        &roadmap_body(&[("epic:first", "now")]),
    );
    approve_with(
        &project,
        "team-roadmap",
        "plan",
        "approval-record",
        "reviews",
    );
    let path = project.join(".engineering/planning/team-roadmap/plan.md");
    let before = std::fs::read(&path).expect("the roadmap is on disk");

    let body = file(
        &project,
        "changed.md",
        &roadmap_body(&[("epic:first", "now"), ("epic:second", "next")]),
    );
    let output = aep(&project, &["body", "team-roadmap:plan", "--from", &body]);
    assert!(
        !output.status.success(),
        "an approved team-roadmap (a roadmap by `ArtifactKind::parent`, on the roadmap ladder) was \
         rewritten in place: {}",
        text(&output)
    );
    assert_eq!(before, std::fs::read(&path).expect("on disk"));
}

/// The same subkind is not validated at all: an entry naming a missing epic passes.
#[test]
fn a_roadmap_subkind_s_entries_are_validated() {
    let project = project("roadmap-subkind-validated");
    roadmap_of_kind(
        &project,
        "team-roadmap",
        "plan",
        &roadmap_body(&[("epic:nowhere", "now")]),
    );

    let (valid, said) = validate(&project, &[]);
    assert!(
        !valid && said.contains("epic:nowhere"),
        "a team-roadmap listing an epic the store does not hold: {said}"
    );
}

// ---------------------------------------------------------------------------------------------
// Red at 9dc5ea225: an id spelled with this store's own member name is the same artifact
// (`ArtifactId::local_to`: "the spelling an author chose must not decide anything").
// ---------------------------------------------------------------------------------------------

#[test]
fn an_epic_spelled_with_this_store_s_own_member_name_is_that_epic() {
    let project = project("own-member");
    std::fs::write(
        project.join(".engineering/workspace.yaml"),
        "version: aep.workspace/1\nmembers:\n  - name: mine\n    source: ..\n",
    )
    .expect("the workspace declaration is writable");
    epic(&project, "first", "active");
    roadmap(&project, "plan", &[("mine/epic:first", "now")]);
    approve(&project, "plan");

    let (valid, said) = validate(&project, &["--strict"]);
    assert!(
        valid,
        "`mine/epic:first` read inside member `mine` is `epic:first`: {said}"
    );
}

// ---------------------------------------------------------------------------------------------
// Red at 9dc5ea225: what a renderer shows as the roadmap and validate does not read.
// ---------------------------------------------------------------------------------------------

/// Two `roadmap` blocks: only the first is read, so the second (which a reader sees as part of the
/// approved order) can name an epic that does not exist and validate passes.
#[test]
fn a_second_roadmap_block_is_not_silently_ignored() {
    let project = project("two-blocks");
    epic(&project, "first", "draft");
    roadmap_with(
        &project,
        "plan",
        "# Roadmap\n\n```roadmap\n- epic: epic:first\n  horizon: now\n```\n\n\
         ## Later\n\n```roadmap\n- epic: epic:nowhere\n  horizon: later\n```\n",
    );

    let (valid, said) = validate(&project, &[]);
    assert!(
        !valid,
        "a body with two roadmap blocks, the second naming a missing epic, validated: {said}"
    );
}

/// A `~~~roadmap` block renders exactly as a backtick one and is not read at all.
#[test]
fn a_tilde_fenced_roadmap_block_is_read_or_refused() {
    let project = project("tilde");
    roadmap_with(
        &project,
        "plan",
        "# Roadmap\n\n~~~roadmap\n- epic: epic:nowhere\n  horizon: now\n~~~\n",
    );

    let (valid, said) = validate(&project, &[]);
    assert!(
        !valid,
        "a tilde-fenced roadmap block naming a missing epic validated: {said}"
    );
}

// ---------------------------------------------------------------------------------------------
// Red at 9dc5ea225: validation accumulates (AGENTS.md invariant *Validation accumulates*).
// ---------------------------------------------------------------------------------------------

#[test]
fn every_unreadable_entry_of_a_block_is_reported_together() {
    let project = project("accumulates-unreadable");
    roadmap_with(
        &project,
        "plan",
        "# Roadmap\n\n```roadmap\n- epic: epic:a\n  horizon: soon\n\
         - epic: epic:b\n  horizon: eventually\n```\n",
    );

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("soon") && said.contains("eventually"),
        "two independent bad horizons, both named: {said}"
    );
}

// ---------------------------------------------------------------------------------------------
// Red at 9dc5ea225: an approved roadmap's order changed without `body`.
// ---------------------------------------------------------------------------------------------

/// `redact` rewrites the block of an approved roadmap: the approved order now names a different
/// epic and the approval stands.
#[test]
fn redact_does_not_change_what_an_approved_roadmap_orders() {
    let project = project("redact");
    epic(&project, "first", "draft");
    epic(&project, "second", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    approve(&project, "plan");
    let path = project.join(".engineering/planning/roadmap/plan.md");
    let before = std::fs::read_to_string(&path).expect("on disk");

    let output = aep(
        &project,
        &[
            "redact",
            "roadmap:plan",
            "--literal",
            "epic:first",
            "--with",
            "epic:second",
            "--reason",
            "not a secret",
        ],
    );
    let after = std::fs::read_to_string(&path).expect("on disk");
    assert!(
        !output.status.success() && before == after,
        "redact rewrote the approved order (exit {:?}): {}\n--- after ---\n{after}",
        output.status.code(),
        text(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// Green at 9dc5ea225: mutant killers for guards the unit's suite did not exercise.
// ---------------------------------------------------------------------------------------------

/// Kills dropping `entry.epic.namespace() == "epic"` from the existence check: a missing story is
/// one defect (not an epic), not two.
#[test]
fn a_missing_non_epic_is_one_problem() {
    let project = project("missing-story");
    roadmap(&project, "plan", &[("story:nowhere", "now")]);

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(said.contains("1 problem(s)"), "{said}");
    assert!(said.contains("not an epic"), "{said}");
}

/// Kills dropping `named.insert`: a missing epic listed twice is "twice" once and "no such" once.
#[test]
fn a_missing_epic_listed_twice_is_missing_once() {
    let project = project("missing-twice");
    roadmap(
        &project,
        "plan",
        &[("epic:nowhere", "now"), ("epic:nowhere", "now")],
    );

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(said.contains("2 problem(s)"), "{said}");
    assert_eq!(said.matches("no such artifact").count(), 1, "{said}");
}

/// Kills dropping the `ApprovalRecord` kind filter: a story that `reviews` the roadmap is not who
/// approved it.
#[test]
fn only_an_approval_record_satisfies_the_approval_record_rule() {
    let project = project("story-reviews");
    epic(&project, "first", "draft");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    approve_with(&project, "roadmap", "plan", "story", "reviews");

    let (valid, said) = validate(&project, &[]);
    assert!(!valid, "{said}");
    assert!(
        said.contains("no approval-record reviews or decides"),
        "{said}"
    );
}

/// Kills dropping `.chain(decides)`: an approval-record that decides the roadmap satisfies it.
#[test]
fn an_approval_record_that_decides_the_roadmap_satisfies_the_rule() {
    let project = project("decides");
    epic(&project, "first", "active");
    roadmap(&project, "plan", &[("epic:first", "now")]);
    approve_with(&project, "roadmap", "plan", "approval-record", "decides");

    let (valid, said) = validate(&project, &["--strict"]);
    assert!(valid, "{said}");
}

/// Kills widening `live`: an implemented epic is not reported off the roadmap.
#[test]
fn a_closed_epic_is_not_off_the_roadmap() {
    let project = project("closed-epic");
    epic(&project, "first", "active");
    epic(&project, "done", "active");
    ok(
        &project,
        &["move", "epic:done", "--to", "implemented", "--via"],
    );
    roadmap(&project, "plan", &[("epic:first", "now")]);
    approve(&project, "plan");

    let (valid, said) = validate(&project, &["--strict"]);
    assert!(valid, "{said}");
    assert!(!said.contains("epic:done"), "{said}");
}

/// Kills dropping `is_integral()` / `>= 1.0`: each value is refused, naming the fact.
#[test]
fn a_limit_that_is_not_a_whole_number_of_at_least_one_is_a_problem() {
    for (tag, value) in [
        ("zero", "0"),
        ("fraction", "1.5"),
        ("negative", "-2"),
        ("text", "two"),
    ] {
        let project = project_with_limit(&format!("limit-{tag}"), value);
        let (valid, said) = validate(&project, &[]);
        assert!(!valid, "limit {value}: {said}");
        assert!(
            said.contains("roadmap.approved.limit") && said.contains("whole number"),
            "limit {value}: {said}"
        );
    }
}

/// Kills dropping `front.kind == ArtifactKind::Roadmap` from the `body` refusal: an approved
/// vision's body is still writable.
#[test]
fn body_still_rewrites_an_approved_vision() {
    let project = project("vision-body");
    artifact(&project, "vision", "direction", &["in_review", "approved"]);
    let body = file(&project, "vision.md", "# Vision\n\nRewritten.\n");
    ok(&project, &["body", "vision:direction", "--from", &body]);
}

/// The JSON form and a null rationale read as YAML does.
#[test]
fn a_json_block_with_a_null_rationale_is_read() {
    let project = project("json");
    epic(&project, "first", "active");
    roadmap_with(
        &project,
        "plan",
        "# Roadmap\n\n```roadmap\n[{\"epic\": \"epic:first\", \"horizon\": \"now\", \
         \"rationale\": null}]\n```\n",
    );
    approve(&project, "plan");

    let (valid, said) = validate(&project, &["--strict"]);
    assert!(valid, "{said}");
}
