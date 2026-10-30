//! `AEP_CACHE_GROUP_WRITABLE`, read from the environment of the process that resolves a pinned
//! protocol source (`story:a-group-writable-snapshot-is-admitted-explicitly`).
//!
//! The mode rules themselves are tested in `project.rs` through the value this variable becomes;
//! this file proves the variable is what decides it, across processes, as `aep` runs.

mod support;

use support::{resolve_in_child, PinnedRemote};

/// The child entry `resolve_in_child` runs.
const ENTRY: &str = "child_resolves_the_source_its_parent_named";

#[test]
#[ignore = "a child process entry point, run by resolve_in_child"]
fn child_resolves_the_source_its_parent_named() {
    support::child_entry();
}

#[cfg(unix)]
#[test]
fn a_cache_on_an_fs_group_volume_resolves_only_in_a_process_that_admits_the_group() {
    let root = support::scratch("group-writable-env");
    let remote = PinnedRemote::create(&root);
    let locator = remote.locator();
    let cache = root.join("cache");
    let cache = (
        "AEP_CACHE_DIR",
        cache.to_str().expect("a UTF-8 scratch path"),
    );
    let admitted = [cache, ("AEP_CACHE_GROUP_WRITABLE", "1")];

    let snapshot =
        resolve_in_child(ENTRY, &locator, &[cache]).resolved("an empty cache fetches the pin");
    let stamp = snapshot.with_file_name(format!("{}.verified.json", remote.revision));
    // What the kubelet does on each mount of a volume with `fsGroup`.
    support::add_mode_bits(&snapshot, 0o660, 0o2770);
    assert_eq!(support::mode_of(&snapshot.join("protocol.yaml")), 0o664);
    assert_eq!(support::mode_of(&snapshot.join("profiles")), 0o2775);

    // Acceptance 2: no variable, or any value but `1`.
    for value in [
        None,
        Some(""),
        Some("0"),
        Some("true"),
        Some("yes"),
        Some(" 1"),
    ] {
        let mut environment = vec![cache];
        environment.extend(value.map(|value| ("AEP_CACHE_GROUP_WRITABLE", value)));
        let refusal = resolve_in_child(ENTRY, &locator, &environment).refused(&format!(
            "AEP_CACHE_GROUP_WRITABLE={value:?} does not admit the group"
        ));
        assert!(
            refusal.contains("AEP_CACHE_GROUP_WRITABLE"),
            "{value:?}: {refusal}"
        );
        assert!(
            refusal.contains("writable by its group"),
            "{value:?}: {refusal}"
        );
    }

    // Past the two-second racy window, so the admitted verification below leaves a stamp.
    std::thread::sleep(std::time::Duration::from_millis(2_100));
    let before = std::fs::read(&stamp).ok();
    let resolved = resolve_in_child(ENTRY, &locator, &admitted)
        .resolved("AEP_CACHE_GROUP_WRITABLE=1 admits the volume's group");
    assert_eq!(resolved, snapshot);
    let after = std::fs::read(&stamp).ok();
    assert!(
        after.is_some() && after != before,
        "the admitted process verified in full and stamped the snapshot"
    );

    // Acceptance 6: that stamp does not carry the admission to a later process without it.
    let refusal = resolve_in_child(ENTRY, &locator, &[cache])
        .refused("a later process that did not declare the group trusted");
    assert!(refusal.contains("AEP_CACHE_GROUP_WRITABLE"), "{refusal}");
    resolve_in_child(ENTRY, &locator, &admitted)
        .resolved("the stamp still serves a process that admits the group");

    // Acceptance 1: the admission trusts the group, not the bytes.
    std::fs::write(snapshot.join("protocol.yaml"), "seaLed\n").expect("the group can write it");
    let refusal = resolve_in_child(ENTRY, &locator, &admitted)
        .refused("a changed byte under an admitted group");
    assert!(refusal.contains("does not match"), "{refusal}");
    support::remove(&root);
}
