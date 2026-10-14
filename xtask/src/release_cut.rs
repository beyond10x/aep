//! Cutting a release: `cargo xtask release prepare | tag | pin | record`.
//!
//! The release in `AGENTS.md` § *Releases and commits* was eight steps an agent performed by hand.
//! These four are the ones that change the repository; push, pull request and merge stay the bot
//! route's. Each is idempotent, and each refuses, naming why, before it changes anything.

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::{
    changelog_top_version, git_at, newest_release_tag, release_recorded, tag_is_on_main,
    workspace_version,
};

/// Runs the git commands that write history: `commit` and `tag`.
///
/// The repository's writes are the bot's (`b10x-gates bot -- <git command>`); a test drives a
/// fixture checkout with plain `git` instead, so it never reaches the bot.
pub(crate) struct GitWriter {
    program: OsString,
    prefix: Vec<OsString>,
}

impl GitWriter {
    /// The bot route every commit and tag in this repository takes.
    pub(crate) fn bot() -> Self {
        Self {
            program: "b10x-gates".into(),
            prefix: vec!["bot".into(), "--".into()],
        }
    }

    /// Plain `git`, for a fixture checkout.
    #[cfg(test)]
    pub(crate) fn plain() -> Self {
        Self {
            program: "git".into(),
            prefix: Vec::new(),
        }
    }
}

/// The derived regions `cargo xtask status` owns.
pub(crate) trait StatusOutput {
    /// Every file the derivation may rewrite, relative to the root.
    fn paths(&self) -> Vec<String>;
    /// Rewrites them from the tags.
    ///
    /// # Errors
    ///
    /// When a derivation fails.
    fn write(&self, root: &Path) -> Result<()>;
}

/// One run of the `Release` workflow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReleaseRun {
    /// The run's id.
    pub(crate) id: u64,
    /// The run's page.
    pub(crate) url: String,
    /// `success`, `failure`, … or empty while it runs.
    pub(crate) conclusion: String,
}

/// The read-only view of the forge `record` needs.
pub(crate) trait Forge {
    /// The `Release` workflow run for `tag` at `commit`, if there is one.
    ///
    /// # Errors
    ///
    /// When the forge cannot be asked.
    fn release_run(&self, root: &Path, tag: &str, commit: &str) -> Result<Option<ReleaseRun>>;
}

/// Writes one evidence record into the planning store.
pub(crate) trait Recorder {
    /// Runs `aep` with `arguments` at `root`.
    ///
    /// # Errors
    ///
    /// When `aep` cannot be run or refuses.
    fn aep(&self, root: &Path, arguments: &[String]) -> Result<()>;
}

/// The forge as `gh` reads it. Read-only: `gh run list`.
pub(crate) struct GhForge;

impl Forge for GhForge {
    fn release_run(&self, root: &Path, tag: &str, commit: &str) -> Result<Option<ReleaseRun>> {
        let output = Command::new("gh")
            .args([
                "run",
                "list",
                "--workflow",
                RELEASE_WORKFLOW_FILE,
                "--branch",
                tag,
                "--limit",
                "20",
                "--json",
                "databaseId,url,conclusion,headSha",
            ])
            .current_dir(root)
            .output()
            .context("running gh — the Release workflow run is read from the forge")?;
        if !output.status.success() {
            bail!(
                "gh run list failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let runs: serde_json::Value =
            serde_json::from_slice(&output.stdout).context("reading gh run list's JSON")?;
        let runs: Vec<ReleaseRun> = runs
            .as_array()
            .context("gh run list printed something other than a JSON array")?
            .iter()
            .filter(|run| run["headSha"].as_str() == Some(commit))
            .filter_map(|run| {
                Some(ReleaseRun {
                    id: run["databaseId"].as_u64()?,
                    url: run["url"].as_str()?.to_owned(),
                    conclusion: run["conclusion"].as_str().unwrap_or_default().to_owned(),
                })
            })
            .collect();
        // A re-run after a failure is a second run at the same commit; the one that succeeded is
        // the release.
        Ok(runs
            .iter()
            .find(|run| run.conclusion == "success")
            .or_else(|| runs.first())
            .cloned())
    }
}

/// The `aep` this release is recorded with: `AEP_BIN` when set, else `aep` on the path.
pub(crate) struct AepCli(OsString);

impl AepCli {
    /// Reads `AEP_BIN`.
    pub(crate) fn from_env() -> Self {
        Self(std::env::var_os("AEP_BIN").unwrap_or_else(|| "aep".into()))
    }
}

impl Recorder for AepCli {
    fn aep(&self, root: &Path, arguments: &[String]) -> Result<()> {
        let status = Command::new(&self.0)
            .args(arguments)
            .current_dir(root)
            .status()
            .with_context(|| format!("running {}", self.0.to_string_lossy()))?;
        if !status.success() {
            bail!(
                "{} {} refused ({status})",
                self.0.to_string_lossy(),
                arguments.join(" ")
            );
        }
        Ok(())
    }
}

/// The workflow file of the `Release` workflow.
const RELEASE_WORKFLOW_FILE: &str = "release.yml";

/// The epic every release is recorded on (`AGENTS.md` § *Releases and commits*, step 7).
const RELEASE_EPIC: &str = "epic:planning-on-entity-runtime";

impl GitWriter {
    fn run(&self, root: &Path, arguments: &[&str]) -> Result<()> {
        let output = Command::new(&self.program)
            .args(&self.prefix)
            .args(arguments)
            .current_dir(root)
            .output()
            .with_context(|| format!("running {}", self.program.to_string_lossy()))?;
        if !output.status.success() {
            bail!(
                "{} git {} failed: {}{}",
                self.program.to_string_lossy(),
                arguments.first().copied().unwrap_or_default(),
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }
}

/// A bare `major.minor.patch`, as a comparable triple.
fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let triple = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    if parts.next().is_some() || !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(triple)
}

fn is_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
}

fn text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The commit `rev` names, or `None` when it names nothing.
fn commit_of(root: &Path, rev: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "-q", "--verify", &format!("{rev}^{{commit}}")])
        .current_dir(root)
        .output()
        .ok()?;
    output.status.success().then(|| text(&output))
}

fn head(root: &Path) -> Result<String> {
    commit_of(root, "HEAD").context("HEAD names no commit")
}

/// Tracked paths whose working-tree or index content differs from `HEAD`. Untracked files are
/// not part of any commit these steps make, so they are not counted.
fn changed_paths(root: &Path) -> Result<Vec<String>> {
    let output = git_at(
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=no"],
        "a release step reads which files changed",
    )?;
    let listed = String::from_utf8(output.stdout).context("reading git status as UTF-8")?;
    let mut entries = listed.split('\0').filter(|entry| !entry.is_empty());
    let mut paths = Vec::new();
    while let Some(entry) = entries.next() {
        let (status, path) = entry.split_at(entry.len().min(3));
        paths.push(path.to_owned());
        // A rename or copy is followed by its source path.
        if status.starts_with('R') || status.starts_with('C') {
            if let Some(source) = entries.next() {
                paths.push(source.to_owned());
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn tag_commit(root: &Path, version: &str) -> Option<String> {
    commit_of(root, &format!("refs/tags/{version}"))
}

fn tag_message(version: &str) -> String {
    format!("aep {version}")
}

fn pin_subject(version: &str) -> String {
    format!("docs: move the release pins and status to {version}")
}

/// The workspace version, refused unless it is a bare version.
fn release_version(root: &Path) -> Result<String> {
    let version = workspace_version(root)?;
    if parse_version(&version).is_none() {
        bail!("the workspace version `{version}` is not a bare `major.minor.patch` version");
    }
    Ok(version)
}

/// `prepare <version> --date <date>`: the workspace version, the lockfile's workspace members and
/// the changelog heading.
///
/// # Errors
///
/// A version that is not bare or not above the newest tag, a date that is not `YYYY-MM-DD`, a
/// changelog with no Unreleased entries, or one that already dates this version differently.
pub(crate) fn prepare(root: &Path, version: &str, date: &str) -> Result<()> {
    let wanted = parse_version(version)
        .with_context(|| format!("`{version}` is not a bare `major.minor.patch` version"))?;
    if !is_date(date) {
        bail!("`{date}` is not a `YYYY-MM-DD` date");
    }
    let newest = newest_release_tag(root)?;
    let current = workspace_version(root)?;
    let heading = format!("## [{version}] — {date}");
    let changelog_path = root.join("CHANGELOG.md");
    let changelog = fs::read_to_string(&changelog_path).context("reading CHANGELOG.md")?;
    let dated = changelog
        .lines()
        .find(|line| line.starts_with(&format!("## [{version}]")));

    if parse_version(&newest).is_some_and(|tagged| wanted <= tagged) {
        // Run again after `tag`: the release is this tag, already prepared.
        if version == newest && current == version && dated == Some(heading.as_str()) {
            println!("release {version} is already prepared");
            return Ok(());
        }
        bail!("version {version} is not above the newest tag {newest}");
    }

    let changelog = match dated {
        Some(line) if line == heading => changelog,
        Some(line) => bail!("CHANGELOG.md already heads {version} as `{line}`, not `{heading}`"),
        None => dated_changelog(&changelog, &heading)?,
    };
    let manifest_path = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).context("reading Cargo.toml")?;
    let manifest = versioned_manifest(&manifest, &current, version);
    let lock_path = root.join("Cargo.lock");
    let lock = fs::read_to_string(&lock_path).context("reading Cargo.lock")?;
    let lock = versioned_lock(&lock, &current, version);

    let mut written = Vec::new();
    for (path, updated, name) in [
        (&manifest_path, manifest, "Cargo.toml"),
        (&lock_path, lock, "Cargo.lock"),
        (&changelog_path, changelog, "CHANGELOG.md"),
    ] {
        if fs::read_to_string(path)? != updated {
            fs::write(path, updated).with_context(|| format!("writing {name}"))?;
            written.push(name);
        }
    }
    if written.is_empty() {
        println!("release {version} is already prepared");
    } else {
        println!("prepared release {version}: {}", written.join(", "));
    }
    Ok(())
}

/// Inserts the release heading under `## [Unreleased]`, which stays, empty, for the next change.
fn dated_changelog(changelog: &str, heading: &str) -> Result<String> {
    let lines: Vec<&str> = changelog.lines().collect();
    let unreleased = lines
        .iter()
        .position(|line| line.trim() == "## [Unreleased]")
        .context("CHANGELOG.md has no `## [Unreleased]` heading")?;
    let first_entry = lines
        .iter()
        .enumerate()
        .skip(unreleased + 1)
        .find(|(_, line)| !line.trim().is_empty())
        .map(|(index, _)| index);
    let Some(first_entry) = first_entry.filter(|index| !lines[*index].starts_with("## [")) else {
        bail!("the Unreleased section of CHANGELOG.md is empty, so there is nothing to release");
    };
    let mut out: Vec<&str> = Vec::with_capacity(lines.len() + 2);
    out.extend(&lines[..=unreleased]);
    out.push("");
    out.push(heading);
    out.push("");
    out.extend(&lines[first_entry..]);
    let mut text = out.join("\n");
    if changelog.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// The manifest with `[workspace.package] version` and every path dependency pinned at `from`
/// moved to `to`.
fn versioned_manifest(manifest: &str, from: &str, to: &str) -> String {
    let old_pin = format!("version = \"{from}\"");
    let new_pin = format!("version = \"{to}\"");
    let mut section = "";
    let mut out = String::with_capacity(manifest.len());
    for line in manifest.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = trimmed;
        }
        let moved = match section {
            "[workspace.package]" if trimmed == old_pin => line.replace(&old_pin, &new_pin),
            "[workspace.dependencies]" if line.contains("path = ") && line.contains(&old_pin) => {
                line.replace(&old_pin, &new_pin)
            }
            _ => line.to_owned(),
        };
        out.push_str(&moved);
    }
    out
}

/// The lockfile with every workspace member (a package with no `source`) at `from` moved to `to`.
fn versioned_lock(lock: &str, from: &str, to: &str) -> String {
    let old_pin = format!("version = \"{from}\"");
    let new_pin = format!("version = \"{to}\"");
    lock.split("\n[[package]]\n")
        .enumerate()
        .map(|(index, block)| {
            let member = index > 0 && !block.lines().any(|line| line.starts_with("source = "));
            if member {
                block
                    .split_inclusive('\n')
                    .map(|line| {
                        if line.trim() == old_pin {
                            line.replace(&old_pin, &new_pin)
                        } else {
                            line.to_owned()
                        }
                    })
                    .collect()
            } else {
                block.to_owned()
            }
        })
        .collect::<Vec<String>>()
        .join("\n[[package]]\n")
}

/// `tag`: the annotated tag `<version>` at `HEAD`, through the writer.
///
/// # Errors
///
/// A changelog that does not head this version, a tag at another commit, or tracked changes.
pub(crate) fn tag(root: &Path, writer: &GitWriter) -> Result<()> {
    let version = release_version(root)?;
    let heading = changelog_top_version(root)?;
    if heading != version {
        bail!(
            "CHANGELOG.md heads `{heading}` and the workspace version is `{version}`; run \
             `cargo xtask release prepare {version} --date <date>` first"
        );
    }
    let head = head(root)?;
    match tag_commit(root, &version) {
        Some(commit) if commit == head => {
            println!("tag {version} is already at HEAD {head}");
            return Ok(());
        }
        Some(commit) => bail!("tag {version} already exists at commit {commit}, not HEAD {head}"),
        None => {}
    }
    let changed = changed_paths(root)?;
    if !changed.is_empty() {
        bail!(
            "the tree has uncommitted changes, so HEAD is not the release: {}",
            changed.join(", ")
        );
    }
    writer.run(root, &["tag", "-a", &version, "-m", &tag_message(&version)])?;
    println!("tagged {version} at {head}");
    Ok(())
}

/// `pin`: commits the `cargo xtask status` output and moves the tag to that commit.
///
/// # Errors
///
/// No tag, a tag that is neither at `HEAD` nor under its pin commit, or changes outside the status
/// output.
pub(crate) fn pin(root: &Path, writer: &GitWriter, status: &dyn StatusOutput) -> Result<()> {
    let version = release_version(root)?;
    let tagged = tag_commit(root, &version).with_context(|| {
        format!("there is no tag {version}; run `cargo xtask release tag` first")
    })?;
    let head = head(root)?;
    let retag = [
        "tag",
        "-f",
        "-a",
        version.as_str(),
        "-m",
        &tag_message(&version),
    ];
    if tagged != head {
        // Resume a pin whose commit was made and whose tag move was not.
        let subject = text(&git_at(
            root,
            &["log", "-1", "--format=%s", "HEAD"],
            "a pin resumes after its own commit",
        )?);
        if commit_of(root, "HEAD~1").as_deref() == Some(tagged.as_str())
            && subject == pin_subject(&version)
            && changed_paths(root)?.is_empty()
        {
            writer.run(root, &retag)?;
            println!("moved tag {version} to the pin commit {head}");
            return Ok(());
        }
        bail!("tag {version} is at commit {tagged}, not HEAD {head}; pin runs right after tag");
    }

    let owned = status.paths();
    let foreign: Vec<String> = changed_paths(root)?
        .into_iter()
        .filter(|path| !owned.contains(path))
        .collect();
    if !foreign.is_empty() {
        bail!(
            "the tree has changes other than the status output: {}",
            foreign.join(", ")
        );
    }
    status.write(root)?;
    let changed = changed_paths(root)?;
    if changed.is_empty() {
        println!("the status output for {version} is already committed at {head}");
        return Ok(());
    }
    let subject = pin_subject(&version);
    let body = format!(
        "- cargo xtask status output for the {version} tag: the release pins, the currency \
         stamps and docs/status.md."
    );
    let mut commit = vec!["commit", "-m", subject.as_str(), "-m", body.as_str(), "--"];
    commit.extend(changed.iter().map(String::as_str));
    writer.run(root, &commit)?;
    writer.run(root, &retag)?;
    println!(
        "committed the status output for {version} ({}) and moved the tag to it",
        changed.join(", ")
    );
    Ok(())
}

/// `record`: one `test_result` on the release epic naming the tag's commit and its `Release`
/// workflow run.
///
/// # Errors
///
/// No tag, a tag not on `origin/main`, no `Release` run for the tag or one that did not succeed,
/// or an `aep` refusal.
pub(crate) fn record(root: &Path, forge: &dyn Forge, recorder: &dyn Recorder) -> Result<()> {
    let version = release_version(root)?;
    let commit = tag_commit(root, &version)
        .with_context(|| format!("there is no tag {version} to record"))?;
    let (on_main, detail) = tag_is_on_main(root, &commit);
    if !on_main {
        bail!("tag {version} (commit {commit}) is not on origin/main: {detail}");
    }
    if release_recorded(root, &commit) {
        println!("release {version} is already recorded for commit {commit}");
        return Ok(());
    }
    let run = forge
        .release_run(root, &version, &commit)?
        .with_context(|| format!("no Release run exists for tag {version} (commit {commit})"))?;
    if run.conclusion != "success" {
        let state = if run.conclusion.is_empty() {
            "still running"
        } else {
            run.conclusion.as_str()
        };
        bail!(
            "Release run {} for tag {version} is {state}, not success: {}",
            run.id,
            run.url
        );
    }
    let arguments: Vec<String> = [
        "plan",
        "artifact",
        "evidence",
        RELEASE_EPIC,
        "--kind",
        "test_result",
        "--source",
        &format!(
            "release {version}: tag commit {commit}, Release run {}; task check exit 0",
            run.id
        ),
        "--ref",
        &run.url,
    ]
    .iter()
    .map(|argument| (*argument).to_owned())
    .collect();
    recorder.aep(root, &arguments)?;
    println!(
        "recorded release {version}: commit {commit}, Release run {}",
        run.url
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use anyhow::Result;

    use super::{pin, prepare, record, tag, Forge, GitWriter, Recorder, ReleaseRun, StatusOutput};
    use crate::dep_tests::tempdir::Root;

    const MANIFEST: &str = "[workspace]\nmembers = [\"a\"]\n\n[workspace.package]\n\
        version = \"1.0.0\"\nedition = \"2021\"\n\n[workspace.dependencies]\n\
        a = { path = \"a\", version = \"1.0.0\" }\nserde = \"1.0.0\"\n";

    const LOCK: &str = "version = 4\n\n[[package]]\nname = \"a\"\nversion = \"1.0.0\"\n\
        dependencies = [\n \"serde\",\n]\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n\
        source = \"registry+https://github.com/rust-lang/crates.io-index\"\n";

    const CHANGELOG: &str = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- a thing\n\n\
        ## [1.0.0] — 2026-01-01\n\n- the first\n";

    fn git(root: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(root)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("utf-8")
            .trim()
            .to_owned()
    }

    fn read(root: &Path, relative: &str) -> String {
        fs::read_to_string(root.join(relative)).expect("the fixture file exists")
    }

    /// A checkout at release `1.0.0` with an Unreleased section, and a bare `origin` holding `main`.
    fn checkout(label: &str) -> (Root, std::path::PathBuf) {
        let scratch = Root::new(label);
        let origin = scratch.path().join("origin.git");
        let root = scratch.path().join("work");
        fs::create_dir_all(&root).expect("mkdir");
        fs::create_dir_all(scratch.path().join("no-hooks")).expect("mkdir");
        git(scratch.path(), &["init", "-q", "--bare", "origin.git"]);
        git(&root, &["init", "-q", "-b", "main"]);
        for (key, value) in [
            ("user.name", "fixture"),
            ("user.email", "fixture@example.invalid"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
            ("core.hooksPath", "../no-hooks"),
        ] {
            git(&root, &["config", key, value]);
        }
        git(
            &root,
            &["remote", "add", "origin", &origin.to_string_lossy()],
        );
        fs::create_dir_all(root.join("docs")).expect("mkdir");
        fs::write(root.join("Cargo.toml"), MANIFEST).expect("write");
        fs::write(root.join("Cargo.lock"), LOCK).expect("write");
        fs::write(root.join("CHANGELOG.md"), CHANGELOG).expect("write");
        fs::write(root.join("docs/status.md"), "newest: 1.0.0\n").expect("write");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "chore: release 1.0.0"]);
        git(&root, &["tag", "-a", "1.0.0", "-m", "aep 1.0.0"]);
        git(&root, &["push", "-q", "origin", "main"]);
        (scratch, root)
    }

    /// The working tree and every ref, so "changes nothing" is one comparison.
    fn snapshot(root: &Path) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}",
            git(root, &["status", "--porcelain"]),
            git(root, &["show-ref", "--head", "--dereference"]),
            read(root, "Cargo.toml"),
            read(root, "Cargo.lock"),
            read(root, "CHANGELOG.md"),
        )
    }

    /// `prepare`, committed the way the release commit is.
    fn prepared(root: &Path) {
        prepare(root, "9.9.9", "2026-01-02").expect("prepare");
        git(root, &["commit", "-q", "-am", "chore: release 9.9.9"]);
    }

    /// Writes the newest tag reachable from HEAD, as the real derivation does.
    struct NewestTag;

    impl StatusOutput for NewestTag {
        fn paths(&self) -> Vec<String> {
            vec!["docs/status.md".to_owned()]
        }

        fn write(&self, root: &Path) -> Result<()> {
            let newest = git(root, &["describe", "--tags", "--abbrev=0"]);
            fs::write(root.join("docs/status.md"), format!("newest: {newest}\n"))?;
            Ok(())
        }
    }

    // prepare

    #[test]
    fn prepare_sets_the_workspace_version_in_the_manifest_and_the_lockfile() {
        let (_scratch, root) = checkout("prepare-version");
        prepare(&root, "9.9.9", "2026-01-02").expect("prepare");
        let manifest = read(&root, "Cargo.toml");
        assert!(
            manifest.contains("[workspace.package]\nversion = \"9.9.9\""),
            "{manifest}"
        );
        assert!(
            manifest.contains("a = { path = \"a\", version = \"9.9.9\" }"),
            "{manifest}"
        );
        assert!(manifest.contains("serde = \"1.0.0\""), "{manifest}");
        let lock = read(&root, "Cargo.lock");
        assert!(lock.contains("name = \"a\"\nversion = \"9.9.9\""), "{lock}");
        assert!(
            lock.contains("name = \"serde\"\nversion = \"1.0.0\""),
            "a registry package keeps its version: {lock}"
        );
    }

    #[test]
    fn prepare_turns_the_unreleased_section_into_the_dated_release() {
        let (_scratch, root) = checkout("prepare-changelog");
        prepare(&root, "9.9.9", "2026-01-02").expect("prepare");
        let changelog = read(&root, "CHANGELOG.md");
        assert!(
            changelog
                .contains("## [Unreleased]\n\n## [9.9.9] — 2026-01-02\n\n### Added\n\n- a thing\n"),
            "{changelog}"
        );
        assert_eq!(
            crate::changelog_top_version(&root).expect("heading"),
            "9.9.9"
        );
    }

    #[test]
    fn prepare_twice_changes_nothing_the_second_time() {
        let (_scratch, root) = checkout("prepare-twice");
        prepare(&root, "9.9.9", "2026-01-02").expect("first");
        let once = snapshot(&root);
        prepare(&root, "9.9.9", "2026-01-02").expect("second");
        assert_eq!(snapshot(&root), once);
    }

    #[test]
    fn prepare_refuses_a_version_not_above_the_newest_tag_naming_both() {
        let (_scratch, root) = checkout("prepare-below");
        let before = snapshot(&root);
        for version in ["0.9.0", "1.0.0"] {
            let refusal = prepare(&root, version, "2026-01-02").expect_err("not above 1.0.0");
            let text = format!("{refusal:#}");
            assert!(text.contains(version) && text.contains("1.0.0"), "{text}");
            assert!(text.contains("not above"), "{text}");
        }
        assert_eq!(snapshot(&root), before);
    }

    // tag

    #[test]
    fn tag_creates_the_annotated_tag_at_head_with_the_release_message() {
        let (_scratch, root) = checkout("tag-create");
        prepared(&root);
        tag(&root, &GitWriter::plain()).expect("tag");
        assert_eq!(git(&root, &["cat-file", "-t", "9.9.9"]), "tag");
        assert_eq!(
            git(&root, &["rev-list", "-n1", "9.9.9"]),
            git(&root, &["rev-parse", "HEAD"])
        );
        assert_eq!(
            git(
                &root,
                &["tag", "-l", "--format=%(contents:subject)", "9.9.9"]
            ),
            "aep 9.9.9"
        );
    }

    #[test]
    fn tag_twice_at_the_same_head_changes_nothing() {
        let (_scratch, root) = checkout("tag-twice");
        prepared(&root);
        tag(&root, &GitWriter::plain()).expect("first");
        let once = snapshot(&root);
        tag(&root, &GitWriter::plain()).expect("second");
        assert_eq!(snapshot(&root), once);
    }

    #[test]
    fn tag_refuses_when_the_tag_exists_at_another_commit_naming_it() {
        let (_scratch, root) = checkout("tag-elsewhere");
        prepared(&root);
        git(&root, &["tag", "-a", "9.9.9", "-m", "aep 9.9.9", "HEAD~1"]);
        let elsewhere = git(&root, &["rev-parse", "HEAD~1"]);
        let before = snapshot(&root);
        let refusal = tag(&root, &GitWriter::plain()).expect_err("tag elsewhere");
        let text = format!("{refusal:#}");
        assert!(text.contains(&elsewhere), "{text}");
        assert_eq!(snapshot(&root), before);
    }

    // pin

    #[test]
    fn pin_commits_the_status_output_and_moves_the_tag_to_that_commit() {
        let (_scratch, root) = checkout("pin-fold");
        prepared(&root);
        tag(&root, &GitWriter::plain()).expect("tag");
        let release_commit = git(&root, &["rev-parse", "HEAD"]);
        pin(&root, &GitWriter::plain(), &NewestTag).expect("pin");
        assert_eq!(read(&root, "docs/status.md"), "newest: 9.9.9\n");
        assert_eq!(git(&root, &["status", "--porcelain"]), "");
        let head = git(&root, &["rev-parse", "HEAD"]);
        assert_ne!(head, release_commit);
        assert_eq!(git(&root, &["rev-parse", "HEAD~1"]), release_commit);
        assert_eq!(git(&root, &["rev-list", "-n1", "9.9.9"]), head);
        assert_eq!(git(&root, &["cat-file", "-t", "9.9.9"]), "tag");
        assert_eq!(
            git(&root, &["show", "--format=", "--name-only", "HEAD"]),
            "docs/status.md"
        );
    }

    #[test]
    fn pin_twice_changes_nothing_the_second_time() {
        let (_scratch, root) = checkout("pin-twice");
        prepared(&root);
        tag(&root, &GitWriter::plain()).expect("tag");
        pin(&root, &GitWriter::plain(), &NewestTag).expect("first");
        let once = snapshot(&root);
        pin(&root, &GitWriter::plain(), &NewestTag).expect("second");
        assert_eq!(snapshot(&root), once);
    }

    #[test]
    fn pin_refuses_changes_other_than_the_status_output_naming_the_paths() {
        let (_scratch, root) = checkout("pin-dirty");
        prepared(&root);
        tag(&root, &GitWriter::plain()).expect("tag");
        fs::write(root.join("CHANGELOG.md"), "edited by hand\n").expect("write");
        let before = snapshot(&root);
        let refusal = pin(&root, &GitWriter::plain(), &NewestTag).expect_err("dirty");
        let text = format!("{refusal:#}");
        assert!(text.contains("CHANGELOG.md"), "{text}");
        assert!(!text.contains("docs/status.md"), "{text}");
        assert_eq!(snapshot(&root), before);
    }

    // record

    struct StubForge(Option<ReleaseRun>);

    impl Forge for StubForge {
        fn release_run(
            &self,
            _root: &Path,
            _tag: &str,
            _commit: &str,
        ) -> Result<Option<ReleaseRun>> {
            Ok(self.0.clone())
        }
    }

    /// Records every `aep` invocation and writes the evidence file the real one would.
    #[derive(Default)]
    struct StubAep(RefCell<Vec<Vec<String>>>);

    impl Recorder for StubAep {
        fn aep(&self, root: &Path, arguments: &[String]) -> Result<()> {
            self.0.borrow_mut().push(arguments.to_vec());
            let directory = root.join(".engineering/evidence/epic/planning-on-entity-runtime");
            fs::create_dir_all(&directory)?;
            let source = &arguments[arguments
                .iter()
                .position(|a| a == "--source")
                .expect("source")
                + 1];
            let reference =
                &arguments[arguments.iter().position(|a| a == "--ref").expect("ref") + 1];
            let record = serde_json::json!({
                "change": {"change": "evidence", "kind": "test_result", "source": source, "reference": reference}
            });
            fs::write(directory.join("record.json"), record.to_string())?;
            Ok(())
        }
    }

    fn run() -> ReleaseRun {
        ReleaseRun {
            id: 42,
            url: "https://github.com/beyond10x/aep/actions/runs/42".to_owned(),
            conclusion: "success".to_owned(),
        }
    }

    /// A release cut with the four commands and its branch on `origin/main`.
    fn released(label: &str) -> (Root, std::path::PathBuf, String) {
        let (scratch, root) = checkout(label);
        prepared(&root);
        tag(&root, &GitWriter::plain()).expect("tag");
        pin(&root, &GitWriter::plain(), &NewestTag).expect("pin");
        let commit = git(&root, &["rev-list", "-n1", "9.9.9"]);
        (scratch, root, commit)
    }

    #[test]
    fn record_writes_one_test_result_naming_the_tag_commit_and_the_run_url() {
        let (_scratch, root, commit) = released("record-write");
        git(&root, &["push", "-q", "origin", "main"]);
        let aep = StubAep::default();
        record(&root, &StubForge(Some(run())), &aep).expect("record");
        record(&root, &StubForge(Some(run())), &aep).expect("a second run");
        let calls = aep.0.borrow();
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(
            calls[0],
            [
                "plan",
                "artifact",
                "evidence",
                "epic:planning-on-entity-runtime",
                "--kind",
                "test_result",
                "--source",
                &format!("release 9.9.9: tag commit {commit}, Release run 42; task check exit 0"),
                "--ref",
                "https://github.com/beyond10x/aep/actions/runs/42",
            ]
        );
    }

    #[test]
    fn record_refuses_when_the_tag_is_not_on_origin_main() {
        let (_scratch, root, _commit) = released("record-off-main");
        let aep = StubAep::default();
        let refusal = record(&root, &StubForge(Some(run())), &aep).expect_err("not on main");
        let text = format!("{refusal:#}");
        assert!(text.contains("origin/main"), "{text}");
        assert!(aep.0.borrow().is_empty());
    }

    #[test]
    fn record_refuses_when_no_release_run_exists_naming_the_tag() {
        let (_scratch, root, _commit) = released("record-no-run");
        git(&root, &["push", "-q", "origin", "main"]);
        let aep = StubAep::default();
        let refusal = record(&root, &StubForge(None), &aep).expect_err("no run");
        let text = format!("{refusal:#}");
        assert!(
            text.contains("no Release run") && text.contains("9.9.9"),
            "{text}"
        );
        assert!(aep.0.borrow().is_empty());
    }

    #[test]
    fn record_refuses_a_release_run_that_did_not_succeed() {
        let (_scratch, root, _commit) = released("record-failed-run");
        git(&root, &["push", "-q", "origin", "main"]);
        let aep = StubAep::default();
        let failed = ReleaseRun {
            conclusion: "failure".to_owned(),
            ..run()
        };
        let refusal = record(&root, &StubForge(Some(failed)), &aep).expect_err("failed run");
        let text = format!("{refusal:#}");
        assert!(text.contains("failure") && text.contains("42"), "{text}");
        assert!(aep.0.borrow().is_empty());
    }
}
