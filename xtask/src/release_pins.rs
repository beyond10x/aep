//! Release pins: the install tag and the pinned protocol commit that the README and the website
//! tell a reader to use.
//!
//! A pin lives in a marked region whose begin marker records what the region pins:
//!
//! ```text
//! {/* generated:release-pin:begin version=0.64.0 commit=58433bd8… — kept by `cargo xtask status` */}
//! …any Markdown, code fences included…
//! {/* generated:release-pin:end */}
//! ```
//!
//! (`<!-- … -->` in plain Markdown, which GitHub renders and MDX refuses.) The region's text is
//! written by hand; only the recorded values are the command's. `cargo xtask status` replaces each
//! recorded value inside the region with the current release's, and rewrites the marker; `--check`
//! fails while any region lags. A region that records only `commit=` leaves every version string
//! in it alone, which is how a pasted output block keeps saying which binary produced it.
//!
//! The 0.63.1 README went on pinning `88836a30` after 0.64.0 shipped: nothing rewrote it and nothing
//! checked it.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::git_at;

/// Every page that carries a release pin. Each must hold at least one region, and a region in a
/// file not listed here is refused by the status-region inventory.
pub(crate) const RELEASE_PIN_FILES: &[&str] = &[
    "README.md",
    "website/docs/getting-started.md",
    "website/docs/guides/gate-a-move-on-evidence.md",
    "website/docs/guides/migrate-an-older-store.md",
    "website/docs/guides/validate-in-ci.md",
    "website/docs/reference/project-file.md",
];

/// The two spellings of a marker: `(open, close)` around its text.
const SPELLINGS: &[(&str, &str)] = &[("{/* ", " */}"), ("<!-- ", " -->")];

const BEGIN: &str = "generated:release-pin:begin ";
const END: &str = "generated:release-pin:end";
const OWNER: &str = " — kept by `cargo xtask status`";

/// One release: its bare version and the commit its tag points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Release {
    pub(crate) version: String,
    pub(crate) commit: String,
}

/// What each kind of region has to pin.
#[derive(Clone, Debug)]
pub(crate) struct Targets {
    /// The newest bare-version tag reachable from `HEAD`.
    pub(crate) newest: Release,
    /// What a region that records a commit pins.
    ///
    /// The newest release, except while `HEAD` is that release's own commit (or a merge that
    /// adds nothing to it): a commit cannot contain its own id, so the release commit and the
    /// merge that lands it keep pinning the release before. The first commit after that has to
    /// bring the pin forward.
    pub(crate) commit_pins: Release,
}

/// Bare-version tags reachable from `HEAD`, newest first.
fn release_tags(root: &Path) -> Result<Vec<String>> {
    let output = git_at(
        root,
        &["tag", "--list", "--merged", "HEAD", "--sort=-v:refname"],
        "release pins are derived from the tags",
    )?;
    let listed = String::from_utf8(output.stdout).context("reading the tag list as UTF-8")?;
    Ok(listed
        .lines()
        .map(str::trim)
        .filter(|tag| is_version(tag))
        .map(str::to_owned)
        .collect())
}

fn release(root: &Path, tag: &str) -> Result<Release> {
    let output = git_at(
        root,
        &["rev-list", "-n1", tag],
        "a release pin names its tag's commit",
    )?;
    let commit = String::from_utf8(output.stdout)
        .context("reading the commit id as UTF-8")?
        .trim()
        .to_owned();
    if !is_commit(&commit) {
        bail!("git rev-list -n1 {tag} printed `{commit}`, which is not a full commit id");
    }
    Ok(Release {
        version: tag.to_owned(),
        commit,
    })
}

/// Derives what the pins have to say from the tags reachable from `HEAD`.
pub(crate) fn targets(root: &Path) -> Result<Targets> {
    let tags = release_tags(root)?;
    let newest_tag = tags.first().context(
        "no bare-version tag is reachable from HEAD, so there is no release for the pins to name \
         — fetch them first (`git fetch --tags`)",
    )?;
    let newest = release(root, newest_tag)?;
    let since = git_at(
        root,
        &[
            "rev-list",
            "--count",
            "--no-merges",
            &format!("{newest_tag}..HEAD"),
        ],
        "a release commit cannot pin itself",
    )?;
    let since = String::from_utf8(since.stdout).context("reading the commit count as UTF-8")?;
    let commit_pins = match (since.trim(), tags.get(1)) {
        ("0", Some(previous)) => release(root, previous)?,
        _ => newest.clone(),
    };
    Ok(Targets {
        newest,
        commit_pins,
    })
}

/// Writes or checks every release pin, and returns how many files changed.
pub(crate) fn hold_release_pins(
    root: &Path,
    files: &[&str],
    targets: &Targets,
    check: bool,
) -> Result<usize> {
    let mut lagging = Vec::new();
    let mut written = 0;
    for relative in files {
        let path = root.join(relative);
        let current =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let (updated, regions) =
            rewrite(&current, targets).with_context(|| format!("in {relative}"))?;
        if regions == 0 {
            bail!(
                "{relative} is listed as carrying a release pin and has no \
                 `generated:release-pin` region"
            );
        }
        if updated == current {
            continue;
        }
        if check {
            lagging.push((*relative).to_owned());
            continue;
        }
        fs::write(&path, updated).with_context(|| format!("writing {}", path.display()))?;
        written += 1;
    }
    if !lagging.is_empty() {
        bail!(
            "release pins lag the newest release {} (commit pins: {} at {}) in: {}; run `cargo \
             xtask status` and commit the result",
            targets.newest.version,
            targets.commit_pins.version,
            targets.commit_pins.commit,
            lagging.join(", ")
        );
    }
    Ok(written)
}

/// The values a begin marker records.
#[derive(Debug, Default)]
struct Recorded {
    version: Option<String>,
    commit: Option<String>,
}

fn parse_begin(line: &str) -> Result<Option<(usize, Recorded)>> {
    let trimmed = line.trim();
    for (index, (open, close)) in SPELLINGS.iter().enumerate() {
        let Some(inner) = trimmed
            .strip_prefix(open)
            .and_then(|rest| rest.strip_suffix(close))
        else {
            continue;
        };
        let Some(rest) = inner.strip_prefix(BEGIN) else {
            continue;
        };
        let attributes = rest.strip_suffix(OWNER).with_context(|| {
            format!("a release-pin begin marker must end with `{OWNER}`: `{trimmed}`")
        })?;
        let mut recorded = Recorded::default();
        for attribute in attributes.split_whitespace() {
            match attribute.split_once('=') {
                Some(("version", value)) if is_version(value) && recorded.version.is_none() => {
                    recorded.version = Some(value.to_owned());
                }
                Some(("commit", value)) if is_commit(value) && recorded.commit.is_none() => {
                    recorded.commit = Some(value.to_owned());
                }
                _ => bail!(
                    "`{attribute}` is not `version=<x.y.z>` or `commit=<40 hex>`, or repeats one: \
                     `{trimmed}`"
                ),
            }
        }
        if recorded.version.is_none() && recorded.commit.is_none() {
            bail!("a release-pin region records neither a version nor a commit: `{trimmed}`");
        }
        return Ok(Some((index, recorded)));
    }
    if trimmed.contains(BEGIN.trim_end()) {
        bail!("a malformed release-pin begin marker: `{trimmed}`");
    }
    Ok(None)
}

fn is_end(line: &str, spelling: usize) -> bool {
    let (open, close) = SPELLINGS[spelling];
    line.trim() == format!("{open}{END}{close}")
}

fn render_begin(
    indent: &str,
    spelling: usize,
    version: Option<&str>,
    commit: Option<&str>,
) -> String {
    let (open, close) = SPELLINGS[spelling];
    let mut attributes = Vec::new();
    if let Some(version) = version {
        attributes.push(format!("version={version}"));
    }
    if let Some(commit) = commit {
        attributes.push(format!("commit={commit}"));
    }
    format!(
        "{indent}{open}{BEGIN}{}{OWNER}{close}",
        attributes.join(" ")
    )
}

/// Rewrites every region in `text` to `targets`, and returns the new text and the region count.
pub(crate) fn rewrite(text: &str, targets: &Targets) -> Result<(String, usize)> {
    let mut output = String::with_capacity(text.len());
    let mut regions = 0;
    let mut open: Option<(usize, usize, Recorded, String, String)> = None;
    for (number, line) in text.split_inclusive('\n').enumerate() {
        let number = number + 1;
        if let Some((spelling, begun, recorded, body, indent)) = open.as_mut() {
            if is_end(line, *spelling) {
                let target = if recorded.commit.is_some() {
                    &targets.commit_pins
                } else {
                    &targets.newest
                };
                let (begin, rewritten) = rewrite_region(indent, *spelling, recorded, target, body)
                    .with_context(|| format!("the release-pin region begun on line {begun}"))?;
                output.push_str(&begin);
                output.push_str(&rewritten);
                output.push_str(line);
                regions += 1;
                open = None;
            } else if parse_begin(line)?.is_some() {
                bail!("line {number} begins a release-pin region inside the one begun on line {begun}");
            } else {
                body.push_str(line);
            }
            continue;
        }
        if let Some((spelling, recorded)) = parse_begin(line)? {
            let indent = line[..line.len() - line.trim_start().len()].to_owned();
            open = Some((spelling, number, recorded, String::new(), indent));
        } else if line.contains(END) {
            bail!("line {number} ends a release-pin region that was never begun");
        } else {
            output.push_str(line);
        }
    }
    if let Some((_, begun, ..)) = open {
        bail!("the release-pin region begun on line {begun} has no end marker");
    }
    Ok((output, regions))
}

/// Checks one region against what its marker records, then moves it to `target`.
fn rewrite_region(
    indent: &str,
    spelling: usize,
    recorded: &Recorded,
    target: &Release,
    body: &str,
) -> Result<(String, String)> {
    let mut rewritten = body.to_owned();
    if let Some(commit) = &recorded.commit {
        let commits = hex_runs(body);
        if !commits.iter().any(|found| found == commit) {
            bail!("it records commit {commit} and does not contain it");
        }
        if let Some(other) = commits.iter().find(|found| *found != commit) {
            bail!("it records commit {commit} and also contains {other}, which nothing keeps");
        }
        rewritten = rewritten.replace(commit.as_str(), &target.commit);
    } else if let Some(stray) = hex_runs(body).first() {
        bail!("it contains commit {stray} and records no `commit=`, so nothing keeps it");
    }
    if let Some(version) = &recorded.version {
        let versions = version_spans(body);
        if !versions
            .iter()
            .any(|(start, end)| &body[*start..*end] == version)
        {
            bail!("it records version {version} and does not contain it");
        }
        if let Some((start, end)) = versions
            .iter()
            .find(|(start, end)| &body[*start..*end] != version)
        {
            bail!(
                "it records version {version} and also contains {}, which nothing keeps; record \
                 only `commit=` in a region whose other versions are history",
                &body[*start..*end]
            );
        }
        // Spans are byte offsets in `body`; the commit replacement keeps lengths, so they still
        // hold in `rewritten`.
        let mut moved = String::with_capacity(rewritten.len());
        let mut at = 0;
        for (start, end) in versions {
            moved.push_str(&rewritten[at..start]);
            moved.push_str(&target.version);
            at = end;
        }
        moved.push_str(&rewritten[at..]);
        rewritten = moved;
    }
    let begin = render_begin(
        indent,
        spelling,
        recorded.version.as_ref().map(|_| target.version.as_str()),
        recorded.commit.as_ref().map(|_| target.commit.as_str()),
    ) + "\n";
    Ok((begin, rewritten))
}

/// Maximal runs of exactly forty lowercase hexadecimal characters.
fn hex_runs(text: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut current = String::new();
    for character in text.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_digit() || ('a'..='f').contains(&character) {
            current.push(character);
        } else {
            if current.len() == 40 && !character.is_ascii_alphanumeric() {
                runs.push(current.clone());
            }
            current.clear();
        }
    }
    runs
}

/// Byte spans of every `x.y.z` version in `text` not embedded in a longer number.
fn version_spans(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let preceded = index > 0 && (bytes[index - 1].is_ascii_digit() || bytes[index - 1] == b'.');
        if !bytes[index].is_ascii_digit() || preceded {
            index += 1;
            continue;
        }
        let mut end = index;
        while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
            end += 1;
        }
        let mut token_end = end;
        while token_end > index && bytes[token_end - 1] == b'.' {
            token_end -= 1;
        }
        if is_version(&text[index..token_end]) {
            spans.push((index, token_end));
        }
        index = end;
    }
    spans
}

fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn is_commit(text: &str) -> bool {
    text.len() == 40
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{hold_release_pins, rewrite, targets, Release, Targets};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const OLD: &str = "88836a30f28ab2fddc3ab63d1ac54956973fa25e";
    const NEW: &str = "58433bd85a1ccf939566c53d5543df86c3852b19";

    fn to_new() -> Targets {
        let newest = Release {
            version: "0.64.0".to_owned(),
            commit: NEW.to_owned(),
        };
        Targets {
            commit_pins: newest.clone(),
            newest,
        }
    }

    fn stale_page() -> String {
        format!(
            "Outputs are from a real run of `aep 0.63.1`.\n\n\
             {{/* generated:release-pin:begin version=0.63.1 — kept by `cargo xtask status` */}}\n\
             ```bash\nVERSION=0.63.1\ncurl …/aep-0.63.1/x.tar.gz\n```\n\
             {{/* generated:release-pin:end */}}\n\n\
             <!-- generated:release-pin:begin commit={OLD} — kept by `cargo xtask status` -->\n\
             ```\n$ aep plan reverse init --protocols git+https://example#{OLD}\n\
             ok    binary-version: 0.63.1\n```\n\
             <!-- generated:release-pin:end -->\n"
        )
    }

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("xtask-release-pins-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("creating the fixture directory");
        directory
    }

    #[test]
    fn a_stale_pin_is_rewritten_and_history_is_left_alone() {
        let (rewritten, regions) = rewrite(&stale_page(), &to_new()).expect("rewriting");
        assert_eq!(regions, 2);
        assert!(rewritten.contains("VERSION=0.64.0\ncurl …/aep-0.64.0/x.tar.gz"));
        assert!(rewritten.contains(&format!("init --protocols git+https://example#{NEW}")));
        assert!(rewritten.contains("begin version=0.64.0 — kept"));
        assert!(rewritten.contains(&format!("begin commit={NEW} — kept")));
        assert!(!rewritten.contains(OLD));
        // Outside every region, and in a commit-only region, the producing version stays.
        assert!(rewritten.contains("real run of `aep 0.63.1`"));
        assert!(rewritten.contains("binary-version: 0.63.1"));
        let (again, _) = rewrite(&rewritten, &to_new()).expect("rewriting twice");
        assert_eq!(again, rewritten, "the rewrite is a fixed point");
    }

    #[test]
    fn check_fails_on_a_stale_pin_and_passes_after_the_rewrite() {
        let root = scratch("check");
        fs::write(root.join("page.md"), stale_page()).expect("writing the fixture");
        let error = hold_release_pins(&root, &["page.md"], &to_new(), true)
            .expect_err("a stale pin must fail --check");
        assert!(error.to_string().contains("page.md"), "{error}");
        assert_eq!(
            hold_release_pins(&root, &["page.md"], &to_new(), false).expect("rewriting"),
            1
        );
        hold_release_pins(&root, &["page.md"], &to_new(), true)
            .expect("--check passes once the pins name the release");
        fs::remove_dir_all(&root).expect("removing the fixture");
    }

    #[test]
    fn a_region_that_does_not_hold_what_it_records_is_refused() {
        let cases = [
            // The recorded version is not in the region.
            "<!-- generated:release-pin:begin version=0.63.1 — kept by `cargo xtask status` -->\n\
             VERSION=0.62.0\n<!-- generated:release-pin:end -->\n"
                .to_owned(),
            // A commit nothing records.
            format!(
                "<!-- generated:release-pin:begin version=0.63.1 — kept by `cargo xtask status` \
                 -->\n0.63.1 #{OLD}\n<!-- generated:release-pin:end -->\n"
            ),
            // Unterminated.
            "<!-- generated:release-pin:begin version=0.63.1 — kept by `cargo xtask status` -->\n\
             0.63.1\n"
                .to_owned(),
            // An end with no begin.
            "0.63.1\n<!-- generated:release-pin:end -->\n".to_owned(),
        ];
        for case in cases {
            assert!(rewrite(&case, &to_new()).is_err(), "accepted:\n{case}");
        }
    }

    #[test]
    fn a_page_listed_as_pinning_with_no_region_is_refused() {
        let root = scratch("empty");
        fs::write(root.join("page.md"), "VERSION=0.63.1\n").expect("writing the fixture");
        assert!(hold_release_pins(&root, &["page.md"], &to_new(), true).is_err());
        fs::remove_dir_all(&root).expect("removing the fixture");
    }

    fn git(root: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "tag.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(arguments)
            .current_dir(root)
            .output()
            .expect("running git");
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout)
            .expect("UTF-8")
            .trim()
            .to_owned()
    }

    #[test]
    fn a_release_commit_pins_the_release_before_and_the_next_commit_pins_it() {
        let root = scratch("git");
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["commit", "-q", "--allow-empty", "-m", "one"]);
        git(&root, &["tag", "0.1.0"]);
        let first = git(&root, &["rev-parse", "HEAD"]);
        git(&root, &["commit", "-q", "--allow-empty", "-m", "release"]);
        git(&root, &["tag", "0.2.0"]);
        git(&root, &["tag", "0.2.0-slug"]);
        let second = git(&root, &["rev-parse", "HEAD"]);

        let at_release = targets(&root).expect("deriving at the release commit");
        assert_eq!(at_release.newest.version, "0.2.0");
        assert_eq!(at_release.newest.commit, second);
        assert_eq!(at_release.commit_pins.version, "0.1.0");
        assert_eq!(at_release.commit_pins.commit, first);

        git(&root, &["commit", "-q", "--allow-empty", "-m", "after"]);
        let after = targets(&root).expect("deriving after the release");
        assert_eq!(after.commit_pins.version, "0.2.0");
        assert_eq!(after.commit_pins.commit, second);
        fs::remove_dir_all(&root).expect("removing the fixture");
    }
}
