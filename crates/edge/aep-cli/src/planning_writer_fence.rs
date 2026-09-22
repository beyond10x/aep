//! Cooperative exclusion shared by new-build planning writers and migration commands.
//!
//! This lock is deliberately only the first layer of migration writer control. It stops two AEP
//! processes that implement this protocol from opening and writing the selected legacy store at
//! once. It says nothing about an older binary or an external SQL client that ignores the lock;
//! migration apply still requires its separate [`aep_planning_migration::WriterControl`].

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs2::FileExt as _;
use sha2::{Digest as _, Sha256};

const LOCK_FILE: &str = ".aep-planning-writer.lock";

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
        let file = Self::lock(&engineering.join(LOCK_FILE))?;
        let mut fence = Self { files: vec![file] };
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
            fence.files.push(Self::lock(&parent.join(LOCK_FILE))?);
        }
        let digest = Sha256::digest(store.as_os_str().as_encoded_bytes());
        let mut digest_text = String::with_capacity(digest.len() * 2);
        for byte in digest {
            write!(&mut digest_text, "{byte:02x}").expect("writing to a String cannot fail");
        }
        let name = format!(".aep-planning-writer-{digest_text}.lock");
        let path = parent.join(name);
        fence.files.push(Self::lock(&path)?);
        Ok(fence)
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
