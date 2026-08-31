//! Reading a document tree.
//!
//! A document tree is the conventional layout — `protocols/`, `principles/`, `workflows/`,
//! `profiles/`, `artifacts/lifecycles/`, `drivers/` — but the convention is only about *where to
//! look*: what a file is called has no bearing on what it declares.
//!
//! Loading reports **every** bad file with its path rather than stopping at the first, because
//! fixing a document set one error per run is how a validation step becomes something people avoid
//! running.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use aep_domain::error::ValidationErrors;
use aep_domain::error::{ValidationCode, ValidationError};
use aep_driver_spec::map::{StepMap, StepMapId};
use aep_schema::parse::{self, DocumentKind};

use aep_engine::registry::Registry;
use sha2::{Digest, Sha256};

/// Which directory holds which kind of document.
const TREE: &[(&str, DocumentKind)] = &[
    ("protocols", DocumentKind::Protocol),
    ("principles", DocumentKind::Principle),
    ("workflows", DocumentKind::Workflow),
    ("profiles", DocumentKind::Profile),
    ("artifacts/lifecycles", DocumentKind::Lifecycle),
    // Last, and the order is load-bearing rather than aesthetic: a step map is cross-validated
    // against the workflow it pins, and the workflows are filled in by the row above this one.
    // `Registry::validate` is what runs that check, after the whole tree has been read, so a map
    // read before its workflow is still checked against it — but the reading order is kept honest
    // here so nobody has to know that to see why this row is last.
    ("drivers", DocumentKind::StepMap),
];

/// File extensions treated as documents.
const EXTENSIONS: &[&str] = &["yaml", "yml", "json"];

/// One file that could not be loaded.
#[derive(Debug)]
pub struct LoadFailure {
    /// The file, or `None` for a failure of the document set as a whole.
    pub path: Option<PathBuf>,
    /// What went wrong.
    pub detail: String,
}

impl fmt::Display for LoadFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}: {}", path.display(), self.detail),
            None => f.write_str(&self.detail),
        }
    }
}

/// What a load produced: whatever was readable, plus every failure.
#[derive(Debug)]
pub struct LoadOutcome {
    /// The documents that loaded.
    pub registry: Registry,
    /// Harness projections loaded beside, but never stored in, the semantic registry.
    pub drivers: DriverRegistry,
    /// How many files were read.
    pub files_read: usize,
    /// What failed.
    pub failures: Vec<LoadFailure>,
}

/// One validated immutable document tree and the digest that identifies its source bytes.
#[derive(Debug)]
pub struct PinnedBundle {
    /// Validated semantic definitions.
    pub registry: Registry,
    /// Validated driver projections shipped beside them.
    pub drivers: DriverRegistry,
    /// Lowercase SHA-256 over sorted length-prefixed relative paths and bytes.
    pub digest: String,
    /// Number of definition documents included in the digest.
    pub files_read: usize,
}

/// Driver step maps acquired with a tree but kept outside the semantic engine registry.
#[derive(Debug, Clone, Default)]
pub struct DriverRegistry {
    maps: BTreeMap<StepMapId, StepMap>,
}

impl DriverRegistry {
    /// The map registered under `id`.
    pub fn get(&self, id: &StepMapId) -> Option<&StepMap> {
        self.maps.get(id)
    }

    /// Every registered map in declared-id order.
    pub fn iter(&self) -> impl Iterator<Item = &StepMap> {
        self.maps.values()
    }

    /// How many maps were acquired.
    pub fn len(&self) -> usize {
        self.maps.len()
    }

    /// Whether no maps were acquired.
    pub fn is_empty(&self) -> bool {
        self.maps.is_empty()
    }

    /// Adds one map, refusing duplicate declared identities.
    fn insert(&mut self, map: StepMap) -> Result<(), ValidationError> {
        if self.maps.contains_key(&map.id) {
            return Err(ValidationError::new(
                ValidationCode::DuplicatePrinciple,
                format!("step map {}", map.id),
                format!("a second step map document declares the id `{}`", map.id),
            ));
        }
        self.maps.insert(map.id.clone(), map);
        Ok(())
    }

    /// Cross-validates every map against the semantic workflows it projects.
    fn validate(&self, registry: &Registry) -> ValidationErrors {
        let mut errors = ValidationErrors::new();
        for map in self.maps.values() {
            let location = format!("step map {}", map.id);
            let Some(workflow) = registry.workflow(map.workflow.reference()) else {
                let present = registry
                    .workflows()
                    .find(|workflow| &workflow.id == map.workflow.id());
                errors.push(match present {
                    Some(workflow) => ValidationError::new(
                        ValidationCode::VersionMismatch,
                        location,
                        format!(
                            "the map pins `{}` and the tree holds `{}` at version {}",
                            map.workflow, workflow.id, workflow.version
                        ),
                    )
                    .with_hint(
                        "a major version exists because the change could not be expressed additively, so the map is rewritten against the new state graph rather than migrated",
                    ),
                    None => ValidationError::new(
                        ValidationCode::UnknownWorkflow,
                        location,
                        format!("no workflow `{}` is in the tree", map.workflow.id()),
                    ),
                });
                continue;
            };
            errors.extend(map.cross_validate(workflow));
        }
        errors
    }
}

impl LoadOutcome {
    /// `true` when everything loaded and the document set is consistent.
    pub fn is_clean(&self) -> bool {
        self.failures.is_empty()
    }

    /// The registry, or every failure.
    pub fn into_result(self) -> Result<Registry, LoadErrors> {
        if self.failures.is_empty() {
            Ok(self.registry)
        } else {
            Err(LoadErrors(self.failures))
        }
    }
}

/// Every failure from one load.
#[derive(Debug)]
pub struct LoadErrors(Vec<LoadFailure>);

impl LoadErrors {
    /// Builds an error set from failures collected elsewhere.
    pub fn from_failures(failures: Vec<LoadFailure>) -> Self {
        Self(failures)
    }

    /// The failures.
    pub fn as_slice(&self) -> &[LoadFailure] {
        &self.0
    }

    /// How many failures there are.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` when there are none, which a constructed value never is.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for LoadErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} document problem(s):", self.0.len())?;
        for failure in &self.0 {
            writeln!(f, "  - {failure}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LoadErrors {}

/// Loads a document tree rooted at `root`, reporting everything readable and everything broken.
///
/// Missing directories are not an error: a project with principles of its own but no workflows is
/// perfectly ordinary.
pub fn load_tree_report(root: &Path) -> LoadOutcome {
    let mut registry = Registry::new();
    let mut drivers = DriverRegistry::default();
    let mut failures = Vec::new();
    let mut files_read = 0_usize;

    for (directory, kind) in TREE {
        let path = root.join(directory);
        if !path.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        if let Err(error) = collect_documents(&path, &mut files) {
            failures.push(LoadFailure {
                path: Some(path.clone()),
                detail: format!("cannot be read: {error}"),
            });
            continue;
        }
        files.sort();
        for file in files {
            files_read += 1;
            if let Err(failure) = load_file(&mut registry, &mut drivers, &file, *kind) {
                failures.push(failure);
            }
        }
    }

    let consistency = registry.validate();
    failures.extend(validation_failures(&consistency));
    failures.extend(validation_failures(&drivers.validate(&registry)));

    LoadOutcome {
        registry,
        drivers,
        files_read,
        failures,
    }
}

/// Loads a document tree, or fails with every problem found.
pub fn load_tree(root: &Path) -> Result<Registry, LoadErrors> {
    load_tree_report(root).into_result()
}

/// Loads and validates one tree and verifies the exact source-byte digest expected by deployment.
///
/// # Errors
///
/// Every document failure, unreadable bundle file, invalid expected digest or digest mismatch.
pub fn load_pinned_bundle(root: &Path, expected: &str) -> Result<PinnedBundle, LoadErrors> {
    let outcome = load_tree_report(root);
    if !outcome.failures.is_empty() {
        return Err(LoadErrors(outcome.failures));
    }
    let mut files = Vec::new();
    for (directory, _) in TREE {
        let path = root.join(directory);
        if path.is_dir() {
            collect_documents(&path, &mut files).map_err(|error| {
                LoadErrors(vec![LoadFailure {
                    path: Some(path.clone()),
                    detail: format!("cannot be read for bundle digest: {error}"),
                }])
            })?;
        }
    }
    files.sort();
    let mut hasher = Sha256::new();
    for path in &files {
        let relative = path.strip_prefix(root).map_err(|error| {
            LoadErrors(vec![LoadFailure {
                path: Some(path.clone()),
                detail: format!("is outside the bundle root: {error}"),
            }])
        })?;
        let relative = relative.to_string_lossy();
        let bytes = fs::read(path).map_err(|error| {
            LoadErrors(vec![LoadFailure {
                path: Some(path.clone()),
                detail: format!("cannot be read for bundle digest: {error}"),
            }])
        })?;
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
    }
    let digest = format!("{:x}", hasher.finalize());
    let expected_valid = expected.len() == 64
        && expected
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !expected_valid {
        return Err(LoadErrors(vec![LoadFailure {
            path: None,
            detail: "expected bundle digest must be 64 lowercase hexadecimal characters".to_owned(),
        }]));
    }
    if digest != expected {
        return Err(LoadErrors(vec![LoadFailure {
            path: None,
            detail: format!(
                "definition bundle digest mismatch: expected {expected}, found {digest}"
            ),
        }]));
    }
    Ok(PinnedBundle {
        registry: outcome.registry,
        drivers: outcome.drivers,
        digest,
        files_read: files.len(),
    })
}

/// Loads one file into `registry`.
fn load_file(
    registry: &mut Registry,
    drivers: &mut DriverRegistry,
    path: &Path,
    kind: DocumentKind,
) -> Result<(), LoadFailure> {
    let text = fs::read_to_string(path).map_err(|error| LoadFailure {
        path: Some(path.to_path_buf()),
        detail: format!("cannot be read: {error}"),
    })?;
    let origin = path.display().to_string();

    let outcome = match kind {
        DocumentKind::Protocol => parse::protocol(&text, Some(&origin))
            .map_err(|error| error.to_string())
            .and_then(|document| {
                registry
                    .insert_protocol(document)
                    .map_err(|error| error.to_string())
            }),
        DocumentKind::Principle => parse::principle(&text, Some(&origin))
            .map_err(|error| error.to_string())
            .and_then(|document| {
                registry
                    .insert_principle(document)
                    .map_err(|error| error.to_string())
            }),
        DocumentKind::Workflow => parse::workflow(&text, Some(&origin))
            .map_err(|error| error.to_string())
            .and_then(|document| {
                registry
                    .insert_workflow(document)
                    .map_err(|error| error.to_string())
            }),
        DocumentKind::Profile => parse::profile(&text, Some(&origin))
            .map_err(|error| error.to_string())
            .and_then(|document| {
                registry
                    .insert_profile(document)
                    .map_err(|error| error.to_string())
            }),
        DocumentKind::Lifecycle => parse::lifecycle(&text, Some(&origin))
            .map_err(|error| error.to_string())
            .and_then(|document| {
                registry
                    .insert_lifecycle(document)
                    .map_err(|error| error.to_string())
            }),
        DocumentKind::StepMap => parse::step_map(&text, Some(&origin))
            .map_err(|error| error.to_string())
            .and_then(|document| drivers.insert(document).map_err(|error| error.to_string())),
        DocumentKind::Task
        | DocumentKind::ArtifactManifest
        | DocumentKind::Evidence
        | DocumentKind::Project
        | DocumentKind::Workspace => {
            Err(format!("{kind} documents do not belong in a protocol tree"))
        }
    };

    outcome.map_err(|detail| LoadFailure {
        path: Some(path.to_path_buf()),
        detail,
    })
}

/// Turns cross-document validation errors into load failures.
fn validation_failures(errors: &ValidationErrors) -> Vec<LoadFailure> {
    errors
        .as_slice()
        .iter()
        .map(|error| LoadFailure {
            path: None,
            detail: error.to_string(),
        })
        .collect()
}

/// Collects document files under `directory`, recursively.
fn collect_documents(directory: &Path, into: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_documents(&path, into)?;
            continue;
        }
        let is_document = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| EXTENSIONS.contains(&extension));
        if is_document {
            into.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_driver_spec::map::RawStepMap;

    /// A scratch document tree that deletes itself, however the test ends.
    ///
    /// Rooted at the system temporary directory and not `CARGO_TARGET_TMPDIR`, which cargo defines
    /// for integration tests and benches only — this is a unit test, beside the code it tests, as
    /// the repository's convention asks. `crates/aep-engine/tests/project_directory_env.rs` roots
    /// its trees the same way. The process id keeps two gates in two worktrees from writing the
    /// same path, which is the collision `CARGO_TARGET_TMPDIR` exists to avoid.
    ///
    /// The cleanup is a `Drop` and not a line at the end of the test on purpose: a failing
    /// assertion unwinds past that line, and a scratch tree named after a process id is never
    /// reclaimed by the next run. Measured — the first mutation run of this file left one behind.
    struct Tree(PathBuf);

    /// Where `tree` writes, as a function so a case can name the path a failed `tree` never
    /// returned.
    fn scratch_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("aep-load-{name}-{}", std::process::id()))
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Writes a document tree holding exactly the files given.
    ///
    /// The guard is constructed **before** the first write, not returned after the last one. Every
    /// line below it can fail — a parent that is a file, a full disk, a read-only temporary
    /// directory — and a `.expect` between the first `create_dir_all` and the `Tree(root)` at the
    /// end unwinds past the constructor, leaving behind exactly the directory the guard exists to
    /// reclaim. Measured with a probe that makes a parent path a file: the earlier shape leaked,
    /// this one does not.
    fn tree(name: &str, files: &[(&str, &str)]) -> Tree {
        let root = scratch_root(name);
        let _ = fs::remove_dir_all(&root);
        let guard = Tree(root);
        for (path, contents) in files {
            let file = guard.0.join(path);
            fs::create_dir_all(file.parent().expect("a document has a directory"))
                .expect("the tree is writable");
            fs::write(&file, contents).expect("the document is writable");
        }
        guard
    }

    const WORKFLOW_AT_2: &str = r#"{"id":"adp/default","version":2,"title":"t",
        "initial":"implement",
        "states":{"implement":{"title":"Implement","terminal":true}},"transitions":[]}"#;

    const MAP_PINNED_TO_1: &str = r#"{"format":"aep.driver-steps/1","id":"development/default",
        "workflow":"adp/default/1","states":{"implement":{"steps":[]}}}"#;

    fn workflow_at(major: u32) -> aep_domain::workflow::Workflow {
        let raw: aep_domain::raw::RawWorkflow = serde_json::from_str(&format!(
            r#"{{"id":"adp/default","version":{major},"title":"t","initial":"implement",
                "states":{{"implement":{{"title":"Implement","terminal":true}}}},
                "transitions":[]}}"#
        ))
        .expect("the fixture deserializes");
        aep_domain::workflow::Workflow::try_from(raw).expect("the fixture validates")
    }

    fn map_named(id: &str, workflow: &str, states: &str) -> StepMap {
        let raw: RawStepMap = serde_json::from_str(&format!(
            r#"{{"format":"aep.driver-steps/1","id":"{id}",
                "workflow":"{workflow}","states":{states}}}"#
        ))
        .expect("the fixture deserializes");
        StepMap::try_from(raw).expect("the fixture validates")
    }

    fn map_pinned_to(workflow: &str) -> StepMap {
        map_named(
            "development/default",
            workflow,
            r#"{"implement":{"steps":[]}}"#,
        )
    }

    fn locations(errors: &ValidationErrors) -> Vec<String> {
        errors
            .as_slice()
            .iter()
            .map(|error| error.location.clone())
            .collect()
    }

    #[test]
    fn a_map_pinned_to_a_major_the_tree_no_longer_has_is_refused() {
        let mut registry = Registry::new();
        registry
            .insert_workflow(workflow_at(2))
            .expect("the workflow registers");
        let mut drivers = DriverRegistry::default();
        drivers
            .insert(map_pinned_to("adp/default/1"))
            .expect("the map registers");

        let id = aep_domain::ids::WorkflowId::new("adp/default").expect("an id");
        assert!(registry
            .workflow(&aep_domain::version::WorkflowRef::unpinned(id.clone()))
            .is_some());
        assert!(registry
            .workflow(&aep_domain::version::WorkflowRef::new(
                id,
                Some(aep_domain::version::MajorVersion::V1),
            ))
            .is_none());

        let errors = drivers.validate(&registry);
        assert_eq!(errors.len(), 1, "{errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(refusal.code, ValidationCode::VersionMismatch);
        assert_eq!(refusal.location, "step map development/default");
        assert!(refusal.message.contains("adp/default/1"));
        assert!(refusal.message.contains("`adp/default` at version 2"));
    }

    #[test]
    fn a_map_pinned_to_the_major_the_tree_holds_is_accepted() {
        let mut registry = Registry::new();
        registry
            .insert_workflow(workflow_at(1))
            .expect("the workflow registers");
        let mut drivers = DriverRegistry::default();
        drivers
            .insert(map_pinned_to("adp/default/1"))
            .expect("the map registers");
        let errors = drivers.validate(&registry);
        assert!(errors.is_empty(), "{errors}");
    }

    #[test]
    fn a_map_whose_pin_resolves_is_still_checked_against_the_workflow_it_resolved_to() {
        let mut registry = Registry::new();
        registry
            .insert_workflow(workflow_at(1))
            .expect("the workflow registers");
        let mut drivers = DriverRegistry::default();
        drivers
            .insert(map_named(
                "development/default",
                "adp/default/1",
                r#"{"implement":{"steps":[]},"polish":{"steps":[]}}"#,
            ))
            .expect("the map registers");

        let errors = drivers.validate(&registry);
        assert_eq!(errors.len(), 1, "{errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(refusal.code, ValidationCode::UnknownState);
        assert_eq!(
            refusal.location,
            "driver-steps[development/default].states.polish"
        );
    }

    #[test]
    fn two_maps_orphaned_by_the_same_bump_are_both_named() {
        let mut registry = Registry::new();
        registry
            .insert_workflow(workflow_at(2))
            .expect("the workflow registers");
        let mut drivers = DriverRegistry::default();
        for id in ["development/default", "development/wave"] {
            drivers
                .insert(map_named(
                    id,
                    "adp/default/1",
                    r#"{"implement":{"steps":[]}}"#,
                ))
                .expect("the map registers");
        }

        let errors = drivers.validate(&registry);
        assert_eq!(errors.len(), 2, "one refusal per orphaned map: {errors}");
        assert_eq!(
            locations(&errors),
            vec!["step map development/default", "step map development/wave"]
        );
    }

    #[test]
    fn a_map_pinned_to_a_workflow_the_tree_does_not_hold_names_the_workflow_that_is_missing() {
        let registry = Registry::new();
        let mut drivers = DriverRegistry::default();
        drivers
            .insert(map_pinned_to("adp/default/1"))
            .expect("the map registers");

        let errors = drivers.validate(&registry);
        assert_eq!(errors.len(), 1, "{errors}");
        let refusal = &errors.as_slice()[0];
        assert_eq!(refusal.code, ValidationCode::UnknownWorkflow);
        assert_eq!(refusal.location, "step map development/default");
        assert!(refusal.message.contains("adp/default"));
        assert!(!refusal.message.contains("development/default"));
    }

    #[test]
    fn the_orphan_refusal_says_what_to_do_about_it() {
        let mut registry = Registry::new();
        registry
            .insert_workflow(workflow_at(2))
            .expect("the workflow registers");
        let mut drivers = DriverRegistry::default();
        drivers
            .insert(map_pinned_to("adp/default/1"))
            .expect("the map registers");

        let errors = drivers.validate(&registry);
        let refusal = &errors.as_slice()[0];
        let hint = refusal
            .hint
            .as_deref()
            .unwrap_or_else(|| panic!("the orphan refusal carries a hint: {refusal}"));
        assert!(hint.contains("rewritten"));
    }

    #[test]
    fn two_maps_with_one_declared_identity_are_refused_at_insertion() {
        let mut drivers = DriverRegistry::default();
        drivers
            .insert(map_pinned_to("adp/default/1"))
            .expect("the first map registers");
        let refusal = drivers
            .insert(map_pinned_to("adp/default/1"))
            .expect_err("the duplicate identity is refused");
        assert_eq!(refusal.location, "step map development/default");
    }

    /// Loading a tree is what runs the cross-document checks — not a step a caller adds afterwards.
    ///
    /// The story's acceptance says an orphaned pin is refused **at load**, and every other case for
    /// it calls `Registry::validate` directly, which is the one thing a caller of `load_tree` never
    /// does. Deleting the `registry.validate()` call from `load_tree_report` leaves all of them
    /// green while `load_tree` starts returning a registry it has not checked.
    #[test]
    fn a_tree_whose_step_map_pins_a_major_the_workflows_no_longer_have_does_not_load() {
        let root = tree(
            "orphan-pin",
            &[
                ("workflows/adp-default.json", WORKFLOW_AT_2),
                ("drivers/development-default.json", MAP_PINNED_TO_1),
            ],
        );

        let root = &root.0;
        let outcome = load_tree_report(root);

        // The fixture has to reach the consistency check: both documents parsed and registered, so
        // the failure below is the cross-document rule and not a file that would not read.
        assert_eq!(outcome.files_read, 2, "both documents were read");
        assert_eq!(
            outcome.drivers.len(),
            1,
            "the map itself is well formed and did register"
        );

        assert_eq!(
            outcome.failures.len(),
            1,
            "{}",
            outcome
                .failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        );
        let said = outcome.failures[0].to_string();
        assert!(
            said.contains("step map development/default"),
            "the refusal reaches the caller of `load_tree`, naming the map: {said}"
        );
        assert!(
            said.contains("adp/default/1") && said.contains("version 2"),
            "with the pin and what the tree holds: {said}"
        );
        assert!(
            !outcome.is_clean(),
            "and the outcome is not clean, so `load_tree` refuses it"
        );
        assert!(
            load_tree(root).is_err(),
            "`load_tree` is the entry point a caller uses, and it must not hand back a registry \
             whose step map is orphaned"
        );
    }

    /// And the same tree with the pin the workflows do hold loads, so this is a filter.
    #[test]
    fn the_same_tree_pinned_to_the_major_the_workflows_hold_loads_clean() {
        let root = tree(
            "matching-pin",
            &[
                (
                    "workflows/adp-default.json",
                    &WORKFLOW_AT_2.replace(r#""version":2"#, r#""version":1"#),
                ),
                ("drivers/development-default.json", MAP_PINNED_TO_1),
            ],
        );

        let root = &root.0;
        let outcome = load_tree_report(root);
        assert!(
            outcome.is_clean(),
            "{}",
            outcome
                .failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        );
        assert!(load_tree(root).is_ok());
    }
    // --- adversarial cases -------------------------------------------------------------------
    //
    // Added by the second adversarial pass over `story:driver-spec-crate`. `Tree`'s doc comment
    // makes two claims — the cleanup survives a failing case, and two runs cannot collide — and
    // neither had a case. A guard nobody has watched fire is the shape this repository refuses to
    // call done, and it is the shape that left a directory behind here once already.

    /// The scratch tree is reclaimed when the case that made it **fails**.
    ///
    /// That is the whole reason the cleanup is a `Drop` rather than a line at the end of the test,
    /// and it is the path no case exercises: both cases above pass, so they would be just as clean
    /// with the cleanup written as their last statement. Panicking on purpose is the only way to
    /// ask the question. It also fails if the panic strategy is ever set to `abort`, under which a
    /// `Drop` does not run and this guard silently stops being one.
    ///
    /// The panic printed by this case is deliberate.
    #[test]
    fn a_scratch_tree_is_reclaimed_when_the_case_that_made_it_panics() {
        let recorded = std::sync::Arc::new(std::sync::Mutex::new(None::<PathBuf>));
        let sink = std::sync::Arc::clone(&recorded);

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let scratch = tree(
                "drop-guard",
                &[("workflows/adp-default.json", WORKFLOW_AT_2)],
            );
            *sink.lock().expect("the sink is not poisoned") = Some(scratch.0.clone());
            assert!(
                scratch.0.join("workflows/adp-default.json").is_file(),
                "the tree is on disk while the case is running"
            );
            panic!("deliberate: this is the unwind a failing assertion would start");
        }));
        assert!(outcome.is_err(), "the closure was supposed to panic");

        let path = recorded
            .lock()
            .expect("the sink is not poisoned")
            .clone()
            .expect("the closure recorded where it wrote");
        assert!(
            !path.exists(),
            "a case that panicked left its scratch tree behind at {}",
            path.display()
        );
    }

    /// Two runs of this file cannot delete each other's tree, because the path carries the pid.
    ///
    /// `tree` removes its root before writing it, so a name two processes could both produce would
    /// make one run delete the other's fixture mid-test. The process id is what stops that, and
    /// this is what says so — dropping it from the format string fails here rather than showing up
    /// as a test that fails only when two gates happen to overlap.
    #[test]
    fn a_scratch_tree_is_named_after_the_process_and_the_case_that_made_it() {
        let first = tree("naming-a", &[("workflows/adp-default.json", WORKFLOW_AT_2)]);
        let second = tree("naming-b", &[("workflows/adp-default.json", WORKFLOW_AT_2)]);
        assert_ne!(
            first.0, second.0,
            "two cases in one run must not share a root, or they delete each other's fixture"
        );
        for root in [&first.0, &second.0] {
            let name = root
                .file_name()
                .expect("the root has a name")
                .to_string_lossy()
                .into_owned();
            assert!(
                name.ends_with(&format!("-{}", std::process::id())),
                "the root must carry the process id, or two gates in two worktrees collide: {name}"
            );
            assert!(
                root.is_dir(),
                "and the tree was written: {}",
                root.display()
            );
        }
    }

    /// The tree is reclaimed when the **helper itself** fails, not only when the case does.
    ///
    /// `a_scratch_tree_is_reclaimed_when_the_case_that_made_it_panics` starts its unwind after
    /// `tree` has returned, so it is satisfied by a helper that constructs the guard as its last
    /// statement — which is what this one did. Every line of `tree` between the first
    /// `create_dir_all` and that constructor can fail, and a failure there unwinds past it: the
    /// directory is on disk and nothing owns it. Measured before the fix with this exact probe,
    /// `leaked = true`.
    ///
    /// The probe writes a *file* where the next entry needs a *directory*, which is the cheapest
    /// way to make `create_dir_all` fail for a reason that is nothing to do with the environment.
    ///
    /// The panic printed by this case is deliberate.
    #[test]
    fn a_scratch_tree_is_reclaimed_when_the_helper_that_writes_it_fails_partway() {
        let root = scratch_root("helper-failure");
        let _ = fs::remove_dir_all(&root);

        let outcome = std::panic::catch_unwind(|| {
            tree(
                "helper-failure",
                &[
                    // Written first, as a file.
                    ("workflows", "this is a file, not a directory"),
                    // Whose `create_dir_all` for a parent that is now a file must fail.
                    ("workflows/adp-default.json", WORKFLOW_AT_2),
                ],
            )
        });
        assert!(
            outcome.is_err(),
            "the probe must make `tree` fail, or it proves nothing about the failing path"
        );
        assert!(
            !root.exists(),
            "`tree` failed partway and left its scratch tree behind at {} — the guard has to be \
             constructed before the first write, not returned after the last one",
            root.display()
        );
    }
}
