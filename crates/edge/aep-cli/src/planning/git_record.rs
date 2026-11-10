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
    // Named by its absolute path, so `.` is the directory it is and has a name to ask about.
    let directory = &std::path::absolute(directory).unwrap_or_else(|_| directory.to_path_buf());
    let (Some(parent), Some(name)) = (directory.parent(), directory.file_name()) else {
        return true;
    };
    let Some(name) = name.to_str() else {
        return true;
    };
    git(parent, &["rev-parse", "--is-inside-work-tree"]).is_none()
        || git(parent, &["check-ignore", "--quiet", name]).is_some()
}

/// Whether the history of a directory can be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Versioning {
    /// No repository holds the directory, or the one that does ignores it: there is no history to
    /// compare with, and a check that needs one is skipped.
    Unversioned,
    /// A repository holds it and answers.
    Versioned,
    /// A repository holds it and Git cannot answer — an ownership refusal, no `git` program, any
    /// error. Never the same as [`Self::Unversioned`]: a check that needs the history is refused.
    Failed(String),
}

/// Whether some directory from `start` upwards has a `.git`, stopping where
/// `GIT_CEILING_DIRECTORIES` says Git stops. `start` is absolute.
fn repository_above(start: &Path) -> bool {
    let ceilings: Vec<std::path::PathBuf> = std::env::var_os("GIT_CEILING_DIRECTORIES")
        .map(|listed| {
            std::env::split_paths(&listed)
                .filter(|path| !path.as_os_str().is_empty())
                .collect()
        })
        .unwrap_or_default();
    let mut current = Some(start);
    while let Some(directory) = current {
        if ceilings.iter().any(|ceiling| ceiling == directory) {
            return false;
        }
        // What Git itself takes for a repository: a `.git` file, or a directory that is one. A
        // stub `.git` (an empty directory) is stepped over, as Git steps over it.
        let dot_git = directory.join(".git");
        if dot_git.is_file()
            || (dot_git.join("HEAD").exists()
                && dot_git.join("objects").is_dir()
                && dot_git.join("refs").is_dir())
        {
            return true;
        }
        current = directory.parent();
    }
    false
}

/// [`Versioning`] of `directory`, decided by where a repository exists and not by whether Git
/// could be asked: only the absence of a repository, or its ignoring the directory, skips.
pub(super) fn versioning(directory: &Path) -> Versioning {
    let Ok(directory) = std::path::absolute(directory) else {
        return Versioning::Failed(format!("{} has no absolute path", directory.display()));
    };
    if !repository_above(&directory) {
        return Versioning::Unversioned;
    }
    let (Some(parent), Some(name)) = (directory.parent(), directory.file_name()) else {
        return Versioning::Unversioned;
    };
    let Some(name) = name.to_str() else {
        return Versioning::Failed(format!("{} is not UTF-8", directory.display()));
    };
    let probe = Command::new("git")
        .arg("-C")
        .arg(&directory)
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match probe {
        Err(error) => return Versioning::Failed(format!("`git` cannot be run: {error}")),
        Ok(output) if !output.status.success() => {
            return Versioning::Failed(
                "a repository holds the store and `git` cannot read it (an ownership refusal, or \
                 a damaged repository)"
                    .to_owned(),
            )
        }
        Ok(output) if output.stdout.trim_ascii() != b"true" => return Versioning::Unversioned,
        Ok(_) => {}
    }
    match Command::new("git")
        .arg("-C")
        .arg(parent)
        .args(["check-ignore", "--quiet", name])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) if status.code() == Some(0) => Versioning::Unversioned,
        Ok(status) if status.code() == Some(1) => Versioning::Versioned,
        Ok(_) => Versioning::Failed("`git check-ignore` could not answer".to_owned()),
        Err(error) => Versioning::Failed(format!("`git` cannot be run: {error}")),
    }
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

/// What [`blobs_before_pack`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Provenance {
    /// There is no history to read at all: not a work tree, or one that ignores the store. The
    /// only case the check is skipped in, as the other history checks are.
    Unversioned,
    /// The commit that added the pack is a shallow clone's boundary: the history that would say
    /// what the records replaced was not fetched.
    Shallow,
    /// Git could not answer; the check is refused, never skipped.
    Failed(String),
    /// One answer per path asked about, in order.
    Blobs(Vec<Committed>),
}

/// For each of `paths` (relative to `base`, the directory the pack's `packs` directory is in), the
/// last committed blob at that path before the commit that added `pack` — `HEAD` when the pack is
/// not committed.
///
/// Every `git` call runs in `base` with pathspecs relative to it, so the answer is the enclosing
/// repository's and never that of a repository nested in the packs directory.
///
/// A path deleted before that commit, as when the loose file was removed in one commit and the
/// pack added in a later one, answers with the last version it had.
pub(super) fn blobs_before_pack(base: &Path, pack: &Path, paths: &[&str]) -> Provenance {
    match versioning(base) {
        Versioning::Unversioned => return Provenance::Unversioned,
        Versioning::Failed(reason) => return Provenance::Failed(reason),
        Versioning::Versioned => {}
    }
    let Some(boundary) = shallow_boundary(base) else {
        return Provenance::Failed("git could not say whether the clone is shallow".to_owned());
    };
    let shallow = !boundary.is_empty();
    let Some(file) = pack.strip_prefix(base).ok().and_then(|relative| {
        relative
            .components()
            .map(|part| part.as_os_str().to_str())
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.join("/"))
    }) else {
        return Provenance::Failed(format!(
            "the pack is not inside {}, where its history is read",
            base.display()
        ));
    };
    let Some(added) = git(
        base,
        &[
            "log",
            "--no-renames",
            "--diff-filter=A",
            "--format=%H",
            "-1",
            "--",
            &file,
        ],
    ) else {
        // A work tree with no commit yet has an empty history, which is an answer.
        return if git(base, &["rev-parse", "--verify", "--quiet", "HEAD"]).is_none() {
            Provenance::Blobs(vec![Committed::Never; paths.len()])
        } else {
            Provenance::Failed("`git log` could not read the history of the pack".to_owned())
        };
    };
    let added = String::from_utf8_lossy(&added).trim().to_owned();
    if boundary.contains(&added) {
        return Provenance::Shallow;
    }
    let revision = if added.is_empty() {
        "HEAD".to_owned()
    } else {
        format!("{added}^")
    };
    let Some(commit) = git(base, &["rev-parse", "--verify", "--quiet", &revision])
        .and_then(|found| String::from_utf8(found).ok())
        .map(|found| found.trim().to_owned())
    else {
        // Nothing before the pack's commit (or no commit at all): nothing to equal — unless the
        // clone is shallow, where it is the history that was not fetched.
        return if shallow && !paths.is_empty() {
            Provenance::Shallow
        } else {
            Provenance::Blobs(vec![Committed::Never; paths.len()])
        };
    };
    let mut requests = String::new();
    for path in paths {
        requests.push_str(&commit);
        requests.push_str(":./");
        requests.push_str(path);
        requests.push('\n');
    }
    let Some(blobs) = cat_file_batch(base, &requests) else {
        return Provenance::Failed("`git cat-file` could not read the committed blobs".to_owned());
    };
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
    if shallow && found.contains(&Committed::Never) {
        // The walk reached the shallow boundary without finding the blob.
        return Provenance::Shallow;
    }
    Provenance::Blobs(found)
}

/// Every pack `HEAD` commits under `packs` (relative to `base`), as the file name and the bytes
/// committed. `None` when there is no history to read, or no commit yet.
#[allow(clippy::case_sensitive_file_extension_comparisons)] // A pack's extension is exactly `.pack`.
pub(super) fn committed_packs(base: &Path, packs: &Path) -> Option<Vec<(String, Vec<u8>)>> {
    if versioning(base) != Versioning::Versioned {
        return None;
    }
    git(base, &["rev-parse", "--verify", "--quiet", "HEAD"])?;
    let directory = packs
        .strip_prefix(base)
        .ok()?
        .components()
        .map(|part| part.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()?
        .join("/");
    let listed = git(base, &["ls-tree", "--name-only", "HEAD", &format!("{directory}/")])?;
    let listed = String::from_utf8_lossy(&listed);
    let files: Vec<&str> = listed
        .lines()
        .filter(|line| line.ends_with(".pack"))
        .collect();
    let mut requests = String::new();
    for file in &files {
        requests.push_str("HEAD:./");
        requests.push_str(file);
        requests.push('\n');
    }
    let blobs = cat_file_batch(base, &requests)?;
    Some(
        files
            .iter()
            .zip(blobs)
            .filter_map(|(file, blob)| {
                let name = file.rsplit('/').next()?.to_owned();
                Some((name, blob?))
            })
            .collect(),
    )
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
