//! Cooperative exclusion shared by new-build planning writers and migration commands.
//!
//! This lock is deliberately only the first layer of migration writer control. It stops two AEP
//! processes that implement this protocol from opening and writing the selected legacy store at
//! once. It says nothing about an older binary or an external SQL client that ignores the lock;
//! migration apply still requires its separate [`aep_planning_migration::WriterControl`].
//!
//! # Where the fence lives
//!
//! Exclusion is an advisory `flock` on a fence file. The file itself carries nothing: a file left
//! by a process that died is unlocked, so the kernel, not the file's existence, says whether a
//! writer is live.
//!
//! Inside a Git repository the fence files live under the repository's common directory, in
//! `<git-common-dir>/aep/planning-writer-project-<sha256>.lock` and
//! `planning-writer-store-<sha256>.lock`, keyed by the canonical project or store path. They are
//! outside every working tree, so they never show up in `git status`, are never committed and
//! never keep a linked worktree from being retired. The common directory is the one every linked
//! worktree of a repository shares, so two writers to one store reach one file wherever they were
//! started from. The directory is found by walking up from the store to the first `.git` entry and
//! following a `.git` file's `gitdir:` and that directory's `commondir`, the layout `git
//! worktree` writes, without running `git`: the answer depends on the store path alone, so every
//! process derives the same file for the same store.
//!
//! Outside a Git repository there is no working tree to keep clean, and the fence stays where it
//! always was, beside the store: `<project>/.aep-planning-writer.lock` and
//! `<store parent>/.aep-planning-writer-<sha256>.lock`.
//!
//! In both places the file persists after release, and that is what keeps two writers from both
//! holding the fence: every writer locks the one inode that name has held since it was created.
//! Unlinking on release would let a writer that opened the old inode before the unlink lock it
//! after, while a later writer creates and locks a new one.
//!
//! An `aep` built before the fence left the working tree locks the in-tree names. Inside a
//! repository this build therefore also locks those names, but only when they already exist, and
//! never creates them: an earlier build leaves its file behind after its first write, so the peer
//! is excluded wherever that file exists. Each fence is taken in a fixed order — the Git common
//! directory file first, then the in-tree name — and the project fence before the store fence;
//! every lock is held for the fence's lifetime. The one window left is an earlier build's very
//! first write in a tree that has no in-tree file yet, which this build cannot see without
//! creating the file it exists to keep out of the working tree.
//!
//! A Git common directory this process cannot write refuses the write, naming the fence: the
//! refusal fails closed and nothing reaches the store.

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs2::FileExt as _;
use sha2::{Digest as _, Sha256};

const LOCK_FILE: &str = ".aep-planning-writer.lock";

/// The directory under the Git common directory that holds the fence files.
const COMMON_FENCE_DIRECTORY: &str = "aep";

/// The Git common directory of the repository whose working tree holds `path`, or `None` when no
/// ancestor of `path` carries a `.git` entry that names a repository. `path` need not exist; the
/// walk starts at its nearest existing ancestor.
///
/// A `.git` entry counts only if it leads to a Git directory holding `HEAD` whose common directory
/// holds `objects` and `refs`, the shape Git itself requires before it accepts a repository. A
/// stray `.git` directory without them is walked past, as Git walks past it; otherwise one left
/// in a home or cache directory would capture every store beneath it.
fn git_common_directory(path: &Path) -> Result<Option<PathBuf>> {
    for directory in path.ancestors() {
        let dot_git = directory.join(".git");
        let metadata = match std::fs::symlink_metadata(&dot_git) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("reading {}", dot_git.display()));
            }
        };
        let git_directory = if metadata.is_file() {
            // A linked worktree or a submodule: `.git` is a file naming the real Git directory.
            let text = std::fs::read_to_string(&dot_git)
                .with_context(|| format!("reading {}", dot_git.display()))?;
            let Some(target) = text
                .lines()
                .find_map(|line| line.strip_prefix("gitdir:"))
                .map(str::trim)
            else {
                continue;
            };
            directory.join(target)
        } else {
            dot_git
        };
        // A linked worktree's Git directory names the shared one in `commondir`; a main worktree's
        // and a submodule's have none and are their own common directory.
        let common = match std::fs::read_to_string(git_directory.join("commondir")) {
            Ok(text) => git_directory.join(text.trim()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => git_directory.clone(),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("reading {}", git_directory.display()));
            }
        };
        if !(git_directory.join("HEAD").is_file()
            && common.join("objects").is_dir()
            && common.join("refs").is_dir())
        {
            continue;
        }
        let common = common
            .canonicalize()
            .with_context(|| format!("resolving Git common directory {}", common.display()))?;
        return Ok(Some(common));
    }
    Ok(None)
}

fn digest_text(path: &Path) -> String {
    let digest = Sha256::digest(path.as_os_str().as_encoded_bytes());
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut text, "{byte:02x}").expect("writing to a String cannot fail");
    }
    text
}

/// The fence directory under the Git common directory that holds `path`, created if missing; or
/// `None` outside a Git repository. Only this directory is ever created: a missing store and its
/// missing parents stay missing.
fn common_fence_directory(path: &Path) -> Result<Option<PathBuf>> {
    let Some(common) = git_common_directory(path)? else {
        return Ok(None);
    };
    let directory = common.join(COMMON_FENCE_DIRECTORY);
    std::fs::create_dir_all(&directory).with_context(|| {
        format!(
            "creating planning writer fence directory {}",
            directory.display()
        )
    })?;
    Ok(Some(directory))
}

/// Where one fence is taken: the file this build creates and locks, and, inside a repository, the
/// in-tree name an earlier build locks, which is locked too when it already exists.
struct FencePaths {
    primary: PathBuf,
    legacy: Option<PathBuf>,
}

/// The fence for a canonical project directory.
fn project_fence_paths(engineering: &Path) -> Result<FencePaths> {
    let in_tree = engineering.join(LOCK_FILE);
    Ok(match common_fence_directory(engineering)? {
        Some(directory) => FencePaths {
            primary: directory.join(format!(
                "planning-writer-project-{}.lock",
                digest_text(engineering)
            )),
            legacy: Some(in_tree),
        },
        None => FencePaths {
            primary: in_tree,
            legacy: None,
        },
    })
}

/// The fence for a canonical store path, which need not exist yet.
fn store_fence_paths(store: &Path, parent: &Path) -> Result<FencePaths> {
    let in_tree = parent.join(format!(".aep-planning-writer-{}.lock", digest_text(store)));
    Ok(match common_fence_directory(store)? {
        Some(directory) => FencePaths {
            primary: directory.join(format!("planning-writer-store-{}.lock", digest_text(store))),
            legacy: Some(in_tree),
        },
        None => FencePaths {
            primary: in_tree,
            legacy: None,
        },
    })
}

/// Resolve aliases without creating a missing store. Canonicalizing the existing parent keeps a
/// deleted projection reached through a directory symlink bound to its original project.
pub(crate) fn canonical_store_path(store: &Path) -> Result<PathBuf> {
    let mut existing = std::path::absolute(store)
        .with_context(|| format!("resolving explicit planning store {}", store.display()))?;
    let mut missing = Vec::new();
    loop {
        match existing.canonicalize() {
            Ok(mut resolved) => {
                for name in missing.into_iter().rev() {
                    resolved.push(name);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(
                    existing
                        .file_name()
                        .context("a missing planning path must name a directory")?
                        .to_os_string(),
                );
                existing.pop();
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("resolving planning store {}", store.display()));
            }
        }
    }
}

/// One process-local handle to the shared new-build writer protocol.
pub(crate) struct PlanningWriterFence {
    files: Vec<File>,
}

impl PlanningWriterFence {
    /// Acquires the project writer lock before any backend is opened.
    pub(crate) fn acquire(engineering: &Path) -> Result<Self> {
        let engineering = engineering
            .canonicalize()
            .with_context(|| format!("resolving planning project {}", engineering.display()))?;
        let mut fence = Self { files: Vec::new() };
        fence.take(&project_fence_paths(&engineering)?)?;
        // Keep the project fence for selector/backend coordination, and also hold the canonical
        // Markdown path fence used by --store. Otherwise two new-build writers can enter the
        // same planning directory under different lock names. This also covers the Markdown half
        // of a hybrid and a planning directory reached through a symlink.
        let mut store_fence = Self::acquire_store(&engineering.join("planning"), false)?;
        fence.files.append(&mut store_fence.files);
        Ok(fence)
    }

    /// Acquires the same protocol for an explicit legacy Markdown path outside project discovery.
    /// The resolved path digest avoids one unrelated explicit store blocking another, while the
    /// lock remains outside the captured store namespace.
    pub(crate) fn acquire_explicit(store: &Path) -> Result<Self> {
        Self::acquire_store(store, true)
    }

    fn acquire_store(store: &Path, include_project: bool) -> Result<Self> {
        // A command can reject an input before it opens the selected store. Keep that ordering for
        // a missing path: deriving the cooperative lock must not turn a semantic refusal into an
        // incidental `canonicalize` error or create the store. Existing paths are canonicalized so
        // two spellings of the same store still share one fence.
        let store = canonical_store_path(store)?;
        let parent = store
            .parent()
            .context("an explicit planning store has no containing directory")?;
        let mut fence = Self { files: Vec::new() };
        // Explicit selection of the conventional project store must also contend with a peer
        // holding only the existing project lock. Always acquire project before path; on a
        // subsequent failure, dropping the guard releases every lock already acquired.
        if include_project
            && store.file_name().is_some_and(|name| name == "planning")
            && parent
                .file_name()
                .is_some_and(|name| name == aep_project::project::project_directory())
        {
            fence.take(&project_fence_paths(parent)?)?;
        }
        fence.take(&store_fence_paths(&store, parent)?)?;
        Ok(fence)
    }

    /// Locks one fence: the primary file first, then the legacy in-tree name if it exists. On a
    /// failure the guard's `Drop` releases whatever was already taken.
    fn take(&mut self, paths: &FencePaths) -> Result<()> {
        self.files.push(Self::lock(&paths.primary)?);
        if let Some(legacy) = &paths.legacy {
            if let Some(file) = Self::lock_existing(legacy)? {
                self.files.push(file);
            }
        }
        Ok(())
    }

    /// Locks `path` only if it exists; it is never created, so a repository that never saw an
    /// earlier build gains no file in its working tree.
    fn lock_existing(path: &Path) -> Result<Option<File>> {
        let file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("opening legacy planning writer fence {}", path.display())
                });
            }
        };
        file.try_lock_exclusive().with_context(|| {
            format!(
                "another admitted planning writer or migration holds {}",
                path.display()
            )
        })?;
        Ok(Some(file))
    }

    fn lock(path: &Path) -> Result<File> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("opening planning writer fence {}", path.display()))?;
        file.try_lock_exclusive().with_context(|| {
            format!(
                "another admitted planning writer or migration holds {}",
                path.display()
            )
        })?;
        Ok(file)
    }
}

impl Drop for PlanningWriterFence {
    fn drop(&mut self) {
        for file in &self.files {
            let _ = fs2::FileExt::unlock(file);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paused_new_build_writer_excludes_an_apply_peer_until_release() {
        let engineering =
            std::env::temp_dir().join(format!("aep-planning-writer-fence-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&engineering);
        std::fs::create_dir_all(&engineering).expect("fixture directory");

        let writer = PlanningWriterFence::acquire(&engineering).expect("first writer enters");
        let refused = PlanningWriterFence::acquire(&engineering)
            .err()
            .expect("a racing migration is refused while the writer is paused");
        assert!(refused
            .to_string()
            .contains("another admitted planning writer"));

        drop(writer);
        PlanningWriterFence::acquire(&engineering)
            .expect("the migration can enter after the writer completes");
        let _ = std::fs::remove_dir_all(engineering);
    }

    #[test]
    fn project_writer_excludes_explicit_store_without_blocking_another_store() {
        let engineering = std::env::temp_dir().join(format!(
            "aep-project-explicit-writer-fence-{}",
            std::process::id()
        ));
        let store = engineering.join("planning");
        let unrelated = engineering.join("other-planning");
        std::fs::create_dir_all(&store).expect("planning fixture");
        std::fs::create_dir_all(&unrelated).expect("unrelated fixture");

        let project = PlanningWriterFence::acquire(&engineering).expect("project writer enters");
        let refused = PlanningWriterFence::acquire_explicit(&store)
            .err()
            .expect("explicit writer must contend with the project writer");
        assert!(refused
            .to_string()
            .contains("another admitted planning writer"));
        let other = PlanningWriterFence::acquire_explicit(&unrelated)
            .expect("a different explicit store remains independent");
        drop(other);
        drop(project);

        let explicit = PlanningWriterFence::acquire_explicit(&store)
            .expect("explicit writer enters after the project writer releases");
        drop(explicit);
        std::fs::remove_dir_all(engineering).expect("retire fixture");
    }

    #[test]
    fn explicit_writer_excludes_project_and_failed_acquisition_releases_project_lock() {
        let engineering = std::env::temp_dir().join(format!(
            "aep-explicit-project-writer-fence-{}",
            std::process::id()
        ));
        let store = engineering.join("planning");
        std::fs::create_dir_all(&store).expect("planning fixture");

        let explicit =
            PlanningWriterFence::acquire_explicit(&store).expect("explicit writer enters");
        let refused = PlanningWriterFence::acquire(&engineering)
            .err()
            .expect("project writer must contend with the explicit writer");
        assert!(refused
            .to_string()
            .contains("another admitted planning writer"));
        drop(explicit);

        let project = PlanningWriterFence::acquire(&engineering)
            .expect("failed acquisition must not leak the project lock");
        drop(project);
        std::fs::remove_dir_all(engineering).expect("retire fixture");
    }
}
