//! Adversary cases for `story:a-group-writable-snapshot-is-admitted-explicitly`: snapshots another
//! principal can write that load without `AEP_CACHE_GROUP_WRITABLE=1`, refusals that name the
//! wrong cause, and an ordering the unit's own tests do not pin.
//!
//! Each resolution runs in a child process of this test binary, as `tests/support` does.

#![cfg(unix)]

mod support;

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use support::{resolve_in_child, PinnedRemote};

/// The child entry `resolve_in_child` runs.
const ENTRY: &str = "child_resolves_the_source_its_parent_named";

#[test]
#[ignore = "a child process entry point, run by resolve_in_child"]
fn child_resolves_the_source_its_parent_named() {
    support::child_entry();
}

fn add_bits(path: &Path, bits: u32) {
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(support::mode_of(path) | bits),
    )
    .expect("the test owns the entry");
}

/// A group that can write the directory holding a sealed snapshot can rename the snapshot away
/// and put its own tree, with its own manifest, in its place: the threat the decision names ("a
/// snapshot that a different principal can write could be rewritten together with its
/// manifest"). Only the snapshot's own entries are checked, so it loads with no declaration. A
/// shared cache created under umask 002 is laid out this way.
#[test]
fn a_sealed_snapshot_in_a_directory_its_group_can_write_is_refused_without_the_declaration() {
    let root = support::scratch("adv-group-parent");
    let remote = PinnedRemote::create(&root);
    let locator = remote.locator();
    let cache = root.join("cache");
    let cache = ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path"));
    let snapshot =
        resolve_in_child(ENTRY, &locator, &[cache]).resolved("an empty cache fetches the pin");
    let parent = snapshot
        .parent()
        .expect("the snapshots directory")
        .to_path_buf();
    add_bits(&parent, 0o020);
    assert_eq!(
        support::mode_of(&snapshot) & 0o022,
        0,
        "the snapshot itself is sealed"
    );

    let outcome = resolve_in_child(ENTRY, &locator, &[cache]);
    support::remove(&root);
    let refusal = outcome.refused("its group can replace the snapshot through its parent");
    assert!(refusal.contains("AEP_CACHE_GROUP_WRITABLE"), "{refusal}");
    assert!(
        refusal.contains(&parent.display().to_string()),
        "the refusal names the directory the group can write: {refusal}"
    );
}

/// The writability check reads the snapshot root with `lstat`, so a root that is a symlink to a
/// sealed snapshot reports the link's own 0777 and is refused as "writable by others", which is
/// untrue and which `chmod -R o-w` cannot fix. Before this unit it loaded.
#[test]
fn a_snapshot_reached_through_a_symlink_is_not_refused_as_writable_by_others() {
    let root = support::scratch("adv-group-symlink-root");
    let remote = PinnedRemote::create(&root);
    let locator = remote.locator();
    let cache = root.join("cache");
    let cache = ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path"));
    let snapshot =
        resolve_in_child(ENTRY, &locator, &[cache]).resolved("an empty cache fetches the pin");

    let elsewhere = root.join("prefilled").join(&remote.revision);
    std::fs::create_dir_all(elsewhere.parent().expect("a parent")).expect("writable");
    add_bits(&snapshot, 0o200);
    std::fs::rename(&snapshot, &elsewhere).expect("the snapshot moves");
    std::fs::set_permissions(&elsewhere, std::fs::Permissions::from_mode(0o555)).expect("resealed");
    std::os::unix::fs::symlink(&elsewhere, &snapshot).expect("a symlink in its place");

    let outcome = resolve_in_child(ENTRY, &locator, &[cache]);
    support::remove(&root);
    if let support::Outcome::Refused(refusal) = outcome {
        assert!(
            !refusal.contains("writable by others"),
            "a sealed snapshot behind a symlink is refused for the link's own mode: {refusal}"
        );
    }
}

/// The unit's comment promises that a snapshot refused whatever the operator declares never names
/// `AEP_CACHE_GROUP_WRITABLE` as the way out. Its own tests set only one write bit at a time, so
/// looking for group write first would pass them; an `fsGroup` volume with one entry others can
/// write is the case that tells the two orders apart.
#[test]
fn on_an_fs_group_volume_an_entry_others_can_write_is_named_not_the_group_declaration() {
    let root = support::scratch("adv-group-others-first");
    let remote = PinnedRemote::create(&root);
    let locator = remote.locator();
    let cache = root.join("cache");
    let cache = ("AEP_CACHE_DIR", cache.to_str().expect("a UTF-8 path"));
    let snapshot =
        resolve_in_child(ENTRY, &locator, &[cache]).resolved("an empty cache fetches the pin");
    support::add_mode_bits(&snapshot, 0o660, 0o2770);
    let open = snapshot.join("profiles/standard.yaml");
    add_bits(&open, 0o002);

    let outcome = resolve_in_child(ENTRY, &locator, &[cache]);
    support::remove(&root);
    let refusal = outcome.refused("others can write one of its files");
    assert!(refusal.contains("writable by others"), "{refusal}");
    assert!(
        refusal.contains(&open.display().to_string()),
        "the refusal names the entry others can write: {refusal}"
    );
    assert!(
        !refusal.contains("AEP_CACHE_GROUP_WRITABLE"),
        "the declaration would not admit it, so it is not offered: {refusal}"
    );
}
