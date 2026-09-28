//! What Git has committed about a Git-native plan, read for `validate` (git-native design § 5, § 6).
//!
//! Two questions only the history answers: what a file said when it was first committed, and
//! whether a committed evidence file has been changed since. Each is one `git` process for the
//! whole store — a log over the named paths plus one `cat-file --batch`, and one `diff` — so the
//! cost grows with the files asked about, not with a process per file.
//!
//! Outside a Git work tree there is no record to compare with: every answer here is `None` or
//! empty, and the check is skipped, not failed. A work tree with no commit yet has an empty history.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

/// Runs `git -C <directory> <args>` and returns its standard output, or `None` when it fails.
fn git(directory: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

/// Whether `directory` is excluded from its enclosing work tree by `.gitignore` (or not in one).
///
/// A store the enclosing repository ignores — a test fixture under `target/`, a scratch copy — is
/// not versioned by that repository, so its history there says nothing about it: every question
/// below answers as it does outside a work tree. Without this, the same store validated
/// differently depending on whether the build directory happened to sit inside a checkout.
fn unversioned(directory: &Path) -> bool {
    let (Some(parent), Some(name)) = (directory.parent(), directory.file_name()) else {
        return true;
    };
    let Some(name) = name.to_str() else {
        return true;
    };
    git(parent, &["rev-parse", "--is-inside-work-tree"]).is_none()
        || git(parent, &["check-ignore", "--quiet", name]).is_some()
}

/// Every committed version of each of `paths` (relative to `root`), oldest first, keyed by the
/// same relative path.
///
/// Only the current incarnation counts: the walk back through a path's history stops at the
/// commit that added it, so a file deleted and later re-created is judged from its re-creation. A
/// path with no committed version is absent from the map. `None` means there is no history to
/// read at all: not a work tree.
pub(super) fn committed_versions(
    root: &Path,
    paths: &[&str],
) -> Option<BTreeMap<String, Vec<Vec<u8>>>> {
    if unversioned(root) {
        return None;
    }
    let prefix = String::from_utf8(git(root, &["rev-parse", "--show-prefix"])?).ok()?;
    let prefix = prefix.trim_end_matches('\n');
    if paths.is_empty() {
        return Some(BTreeMap::new());
    }
    let mut args = vec![
        "log",
        "--no-renames",
        "--format=%x01%H",
        "--name-status",
        "--",
    ];
    args.extend_from_slice(paths);
    let Some(log) = git(root, &args) else {
        // A work tree with no commit yet has an empty history, which is an answer; any other
        // failure is not.
        return git(root, &["rev-parse", "--verify", "--quiet", "HEAD"])
            .is_none()
            .then(BTreeMap::new);
    };
    let log = String::from_utf8(log).ok()?;

    // Newest first: collect each path's commits until the one that added it.
    let mut commits: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut finished: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut commit = "";
    for line in log.lines() {
        if let Some(hash) = line.strip_prefix('\u{1}') {
            commit = hash;
            continue;
        }
        let Some((status, path)) = line.split_once('\t') else {
            continue;
        };
        if commit.is_empty() || finished.contains(path) {
            continue;
        }
        if status.starts_with('D') {
            // The newest event is a deletion: whatever is on disk now was never committed.
            finished.insert(path);
            continue;
        }
        commits.entry(path).or_default().push(commit);
        if status.starts_with('A') {
            finished.insert(path);
        }
    }
    if commits.is_empty() {
        return Some(BTreeMap::new());
    }

    let mut requests = String::new();
    let mut order: Vec<(&str, usize)> = Vec::new();
    for (path, newest_first) in &mut commits {
        newest_first.reverse();
        for commit in newest_first.iter() {
            requests.push_str(commit);
            requests.push(':');
            requests.push_str(path);
            requests.push('\n');
        }
        order.push((path, newest_first.len()));
    }
    let blobs = cat_file_batch(root, &requests)?;
    let mut blobs = blobs.into_iter();
    let mut versions = BTreeMap::new();
    for (path, count) in order {
        let relative = path.strip_prefix(prefix).unwrap_or(path).to_owned();
        let found: Vec<Vec<u8>> = blobs.by_ref().take(count).flatten().collect();
        if !found.is_empty() {
            versions.insert(relative, found);
        }
    }
    Some(versions)
}

/// The contents `git cat-file --batch` answers for each request line, in order; a missing object
/// is `None`.
fn cat_file_batch(root: &Path, requests: &str) -> Option<Vec<Option<Vec<u8>>>> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Written from a thread so a large answer cannot fill the pipe while the request is still
    // being written.
    let mut stdin = child.stdin.take()?;
    let written = requests.to_owned();
    let writer = std::thread::spawn(move || stdin.write_all(written.as_bytes()));
    let output = child.wait_with_output().ok()?;
    writer.join().ok()?.ok()?;
    if !output.status.success() {
        return None;
    }

    let data = output.stdout;
    let mut answers = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let end = at + data[at..].iter().position(|byte| *byte == b'\n')?;
        let header = std::str::from_utf8(&data[at..end]).ok()?;
        at = end + 1;
        if header.ends_with(" missing") || header.ends_with(" ambiguous") {
            answers.push(None);
            continue;
        }
        let size: usize = header.rsplit(' ').next()?.parse().ok()?;
        let content = data.get(at..at + size)?.to_vec();
        at += size + 1;
        answers.push(Some(content));
    }
    Some(answers)
}

/// Every committed file under `evidence` whose working copy is not its committed blob — changed,
/// retyped or deleted — as a path relative to the repository root.
///
/// An evidence file is written once and never again (§ 5), so the only honest difference from
/// `HEAD` is a new file, and a new file is untracked and not listed here.
pub(super) fn changed_evidence(evidence: &Path) -> Vec<String> {
    let (Some(parent), Some(pathspec)) = (evidence.parent(), evidence.to_str()) else {
        return Vec::new();
    };
    if !evidence.is_dir() || unversioned(evidence) {
        return Vec::new();
    }
    let arguments = [
        "diff",
        "--no-renames",
        "--name-only",
        "--diff-filter=MTD",
        "HEAD",
        "--",
        pathspec,
    ];
    let Some(output) = git(parent, &arguments) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output)
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// When each of `paths` (relative to `root`) was added in its current incarnation, as milliseconds
/// since the Unix epoch from the adding commit's author date, keyed by the same relative path.
///
/// One `git log` for the whole set. A path with no committed version is absent from the map, and
/// so is every path outside a work tree.
pub(super) fn first_committed(root: &Path, paths: &[&str]) -> BTreeMap<String, u64> {
    let mut found = BTreeMap::new();
    if paths.is_empty() || unversioned(root) {
        return found;
    }
    let Some(prefix) = git(root, &["rev-parse", "--show-prefix"])
        .and_then(|prefix| String::from_utf8(prefix).ok())
    else {
        return found;
    };
    let prefix = prefix.trim_end_matches('\n');
    let mut args = vec![
        "log",
        "--no-renames",
        "--diff-filter=A",
        "--format=%x01%at",
        "--name-only",
        "--",
    ];
    args.extend_from_slice(paths);
    let Some(log) = git(root, &args).and_then(|log| String::from_utf8(log).ok()) else {
        return found;
    };
    // Newest first, and only the current incarnation counts: a file deleted and re-created is
    // dated from its re-creation, which is the first addition this walk meets.
    let mut seconds = None;
    for line in log.lines() {
        if let Some(at) = line.strip_prefix('\u{1}') {
            seconds = at.trim().parse::<u64>().ok();
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if let Some(seconds) = seconds {
            let relative = line.strip_prefix(prefix).unwrap_or(line).to_owned();
            found.entry(relative).or_insert(seconds.saturating_mul(1000));
        }
    }
    found
}
