//! Cooperative exclusion shared by new-build planning writers and migration commands.
//!
//! This lock is deliberately only the first layer of migration writer control. It stops two AEP
//! processes that implement this protocol from opening and writing the selected legacy store at
//! once. It says nothing about an older binary or an external SQL client that ignores the lock;
//! migration apply still requires its separate [`aep_planning_migration::WriterControl`].

use std::fs::{File, OpenOptions};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use fs2::FileExt as _;
use sha2::{Digest as _, Sha256};

const LOCK_FILE: &str = ".aep-planning-writer.lock";

/// One process-local handle to the shared new-build writer protocol.
pub(crate) struct PlanningWriterFence {
    file: File,
}

impl PlanningWriterFence {
    /// Acquires the project writer lock before any backend is opened.
    pub(crate) fn acquire(engineering: &Path) -> Result<Self> {
        let engineering = engineering
            .canonicalize()
            .with_context(|| format!("resolving planning project {}", engineering.display()))?;
        let path = engineering.join(LOCK_FILE);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("opening planning writer fence {}", path.display()))?;
        file.try_lock_exclusive().with_context(|| {
            format!(
                "another admitted planning writer or migration holds {}",
                path.display()
            )
        })?;
        Ok(Self { file })
    }

    /// Acquires the same protocol for an explicit legacy Markdown path outside project discovery.
    /// The resolved path digest avoids one unrelated explicit store blocking another, while the
    /// lock remains outside the captured store namespace.
    pub(crate) fn acquire_explicit(store: &Path) -> Result<Self> {
        // A command can reject an input before it opens the selected store. Keep that ordering for
        // a missing path: deriving the cooperative lock must not turn a semantic refusal into an
        // incidental `canonicalize` error or create the store. Existing paths are canonicalized so
        // two spellings of the same store still share one fence.
        let store = match store.canonicalize() {
            Ok(store) => store,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::path::absolute(store).with_context(|| {
                    format!("resolving absolute explicit store path {}", store.display())
                })?
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("resolving explicit planning store {}", store.display())
                });
            }
        };
        let parent = store
            .parent()
            .context("an explicit planning store has no containing directory")?;
        let digest = Sha256::digest(store.as_os_str().as_encoded_bytes());
        let mut digest_text = String::with_capacity(digest.len() * 2);
        for byte in digest {
            write!(&mut digest_text, "{byte:02x}").expect("writing to a String cannot fail");
        }
        let name = format!(".aep-planning-writer-{digest_text}.lock");
        let path = parent.join(name);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("opening planning writer fence {}", path.display()))?;
        file.try_lock_exclusive().with_context(|| {
            format!(
                "another admitted planning writer or migration holds {}",
                path.display()
            )
        })?;
        Ok(Self { file })
    }
}

impl Drop for PlanningWriterFence {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paused_new_build_writer_excludes_an_apply_peer_until_release() {
        let engineering = std::env::temp_dir().join(format!(
            "aep-planning-writer-fence-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&engineering);
        std::fs::create_dir_all(&engineering).expect("fixture directory");

        let writer = PlanningWriterFence::acquire(&engineering).expect("first writer enters");
        let refused = PlanningWriterFence::acquire(&engineering)
            .err()
            .expect("a racing migration is refused while the writer is paused");
        assert!(refused.to_string().contains("another admitted planning writer"));

        drop(writer);
        PlanningWriterFence::acquire(&engineering)
            .expect("the migration can enter after the writer completes");
        let _ = std::fs::remove_dir_all(engineering);
    }
}
