//! What Git has committed about a Git-native plan, read for `validate` (git-native design § 5, § 6).
//!
//! Two questions only the history answers: what a file said when it was first committed, and
//! whether a committed evidence file has been changed since. Each is one `git` process for the
//! whole store — a log over the named paths plus one `cat-file --batch`, and one `diff` — so the
//! cost grows with the files asked about, not with a process per file.
//!
//! Outside a Git work tree there is no record to compare with: every answer here is `None` or
//! empty, and the check is skipped, not failed; a review there is undated, not dated now. A work
//! tree with no commit yet has an empty history. In a shallow clone a file the boundary commit
//! holds looks added by it, so [`first_committed`] says so rather than dating it by the clone.

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
///
/// A path in `packed` (spelled as in `paths`) is one a pack holds: a deletion that is its newest
/// event is the commit that moved it into the pack, not the end of the file, and the walk goes on
/// past it to the versions the file had while loose.
pub(super) fn committed_versions(
    root: &Path,
    paths: &[&str],
    packed: &std::collections::BTreeSet<&str>,
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
            if packed.contains(path.strip_prefix(prefix).unwrap_or(path)) {
                // Moved into a pack: the file goes on in the pack, and its loose history before
                // this commit is its history.
                continue;
            }
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
/// `HEAD` is a new file, and a new file is untracked and not listed here. A deletion whose bytes a
/// pack holds is a **move** into that pack, not a change, and is not listed: `pack` writes the
/// pack and removes the loose files before anything is committed.
pub(super) fn changed_evidence(evidence: &Path) -> Vec<String> {
    let (Some(parent), Some(name)) = (evidence.parent(), evidence.file_name()) else {
        return Vec::new();
    };
    let Some(name) = name.to_str() else {
        return Vec::new();
    };
    if !parent.is_dir() || unversioned(evidence) {
        return Vec::new();
    }
    let arguments = [
        "diff",
        "--no-renames",
        "--relative",
        "--name-status",
        "--diff-filter=MTD",
        "HEAD",
        "--",
        name,
    ];
    let Some(output) = git(parent, &arguments) else {
        return Vec::new();
    };
    let prefix = git(parent, &["rev-parse", "--show-prefix"])
        .and_then(|prefix| String::from_utf8(prefix).ok())
        .unwrap_or_default();
    let prefix = prefix.trim_end_matches('\n');
    let output = String::from_utf8_lossy(&output);
    let mut changed: Vec<String> = Vec::new();
    let mut deleted: Vec<&str> = Vec::new();
    for line in output.lines() {
        let Some((status, path)) = line.split_once('\t') else {
            continue;
        };
        if status.starts_with('D') {
            deleted.push(path);
        } else {
            changed.push(path.to_owned());
        }
    }
    if !deleted.is_empty() {
        let moved = moved_into_packs(evidence, parent, &deleted);
        changed.extend(
            deleted
                .iter()
                .zip(moved)
                .filter(|(_, moved)| !moved)
                .map(|(path, _)| (*path).to_owned()),
        );
    }
    let mut changed: Vec<String> = changed
        .into_iter()
        .map(|path| format!("{prefix}{path}"))
        .collect();
    changed.sort();
    changed
}

/// For each of `deleted` (relative to `parent`, the directory beside `evidence`), whether some
/// pack holds exactly the bytes `HEAD` committed there.
fn moved_into_packs(evidence: &Path, parent: &Path, deleted: &[&str]) -> Vec<bool> {
    use aep_backend_markdown::pack::{Overlay, Place};
    let overlay = Overlay::for_evidence(evidence);
    let mut requests = String::new();
    for path in deleted {
        requests.push_str("HEAD:./");
        requests.push_str(path);
        requests.push('\n');
    }
    let blobs = cat_file_batch(parent, &requests).unwrap_or_default();
    deleted
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let Some(Some(committed)) = blobs.get(index) else {
                return false;
            };
            overlay.holders(path).is_ok_and(|holders| {
                holders.iter().any(|holder| {
                    matches!(holder.place(), Place::Packed(_)) && holder.bytes() == committed
                })
            })
        })
        .collect()
}

/// What the history says a packed record's original file held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Committed {
    /// The last committed blob at the path, before the commit that added the pack.
    Blob(Vec<u8>),
    /// No commit before that one holds the path.
    Never,
}

/// For each of `paths` (relative to `base`, the directory the pack's `packs` directory is in), the
/// last committed blob at that path before the commit that added `pack` — `HEAD` when the pack is
/// not committed. `None` when there is no history to read: not a work tree, or one that ignores
/// the store.
///
/// A path deleted before that commit, as when the loose file was removed in one commit and the
/// pack added in a later one, answers with the last version it had.
pub(super) fn blobs_before_pack(
    base: &Path,
    pack: &Path,
    paths: &[&str],
) -> Option<Vec<Committed>> {
    if unversioned(base) {
        return None;
    }
    let (directory, file) = (pack.parent()?, pack.file_name()?.to_str()?);
    let added = String::from_utf8(git(
        directory,
        &[
            "log",
            "--no-renames",
            "--diff-filter=A",
            "--format=%H",
            "-1",
            "--",
            file,
        ],
    )?)
    .ok()?;
    let added = added.trim();
    let revision = if added.is_empty() {
        "HEAD".to_owned()
    } else {
        format!("{added}^")
    };
    let Some(commit) = git(base, &["rev-parse", "--verify", "--quiet", &revision])
        .and_then(|found| String::from_utf8(found).ok())
        .map(|found| found.trim().to_owned())
    else {
        // Nothing before the pack's commit (or no commit at all): nothing to equal.
        return Some(vec![Committed::Never; paths.len()]);
    };
    let mut requests = String::new();
    for path in paths {
        requests.push_str(&commit);
        requests.push_str(":./");
        requests.push_str(path);
        requests.push('\n');
    }
    let blobs = cat_file_batch(base, &requests)?;
    let mut found = Vec::new();
    for (path, blob) in paths.iter().zip(blobs) {
        if let Some(blob) = blob {
            found.push(Committed::Blob(blob));
            continue;
        }
        // Not at that commit: the last commit before it that wrote the path, if any.
        let last = git(
            base,
            &[
                "log",
                "--no-renames",
                "--diff-filter=AMT",
                "--format=%H",
                "-1",
                &commit,
                "--",
                path,
            ],
        )
        .and_then(|last| String::from_utf8(last).ok())
        .map(|last| last.trim().to_owned())
        .filter(|last| !last.is_empty());
        let blob = last.and_then(|last| {
            cat_file_batch(base, &format!("{last}:./{path}\n"))
                .and_then(|mut blobs| blobs.pop().flatten())
        });
        found.push(blob.map_or(Committed::Never, Committed::Blob));
    }
    Some(found)
}

/// What the history says about when one path was added in its current incarnation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Added {
    /// Added by a commit whose author date is this many milliseconds since the Unix epoch.
    At(u64),
    /// Added, as far as this history goes, by a shallow clone's boundary commit: every file the
    /// boundary holds looks added there, because the commits before it were not fetched. The
    /// boundary's date is when the clone starts, not when the file was added, so it dates nothing.
    ShallowBoundary,
}

/// When each of `paths` (relative to `root`) was added in its current incarnation, keyed by the
/// same relative path.
///
/// One `git log` for the whole set, plus two `rev-parse` calls to learn whether the repository is
/// shallow and which commits are its boundary. A path with no committed version is absent from the
/// map: not committed yet. `None` means there is no history to date anything by: not a Git work
/// tree, or one that ignores the store.
pub(super) fn first_committed(root: &Path, paths: &[&str]) -> Option<BTreeMap<String, Added>> {
    if unversioned(root) {
        return None;
    }
    let mut found = BTreeMap::new();
    if paths.is_empty() {
        return Some(found);
    }
    let prefix = String::from_utf8(git(root, &["rev-parse", "--show-prefix"])?).ok()?;
    let prefix = prefix.trim_end_matches('\n');
    let boundary = shallow_boundary(root)?;
    let mut args = vec![
        "log",
        "--no-renames",
        "--diff-filter=A",
        "--format=%x01%H %at",
        "--name-only",
        "--",
    ];
    args.extend_from_slice(paths);
    let Some(log) = git(root, &args) else {
        // A work tree with no commit yet has an empty history, which is an answer; any other
        // failure is not.
        return git(root, &["rev-parse", "--verify", "--quiet", "HEAD"])
            .is_none()
            .then_some(found);
    };
    let log = String::from_utf8(log).ok()?;
    // Newest first, and only the current incarnation counts: a file deleted and re-created is
    // dated from its re-creation, which is the first addition this walk meets.
    let mut added = None;
    for line in log.lines() {
        if let Some(header) = line.strip_prefix('\u{1}') {
            added = header.split_once(' ').and_then(|(commit, at)| {
                if boundary.contains(commit) {
                    Some(Added::ShallowBoundary)
                } else {
                    at.trim()
                        .parse::<u64>()
                        .ok()
                        .map(|seconds| Added::At(seconds.saturating_mul(1000)))
                }
            });
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if let Some(added) = added {
            let relative = line.strip_prefix(prefix).unwrap_or(line).to_owned();
            found.entry(relative).or_insert(added);
        }
    }
    Some(found)
}

/// The boundary commits of a shallow repository — the ones its `shallow` file lists, whose parents
/// were not fetched — or none when the repository is complete. `None` when Git cannot say.
fn shallow_boundary(root: &Path) -> Option<std::collections::BTreeSet<String>> {
    let shallow = git(root, &["rev-parse", "--is-shallow-repository"])?;
    if shallow.trim_ascii() != b"true" {
        return Some(std::collections::BTreeSet::new());
    }
    let path = String::from_utf8(git(root, &["rev-parse", "--git-path", "shallow"])?).ok()?;
    let path = Path::new(path.trim_end_matches('\n'));
    // `--git-path` answers relative to the directory Git ran in, which is `root`.
    let listed = std::fs::read_to_string(root.join(path)).ok()?;
    Some(
        listed
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
    )
}
