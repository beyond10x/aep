//! The service wire emits pinned bytes, so its sources admit no unordered collection, clock or RNG.

use std::path::Path;

const BANNED: &[&str] = &[
    "HashMap",
    "HashSet",
    "SystemTime",
    "Instant::now",
    "rand::",
    "getrandom",
    "thread_rng",
];

fn banned_uses(text: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        for token in BANNED {
            let mut from = 0;
            while let Some(at) = line[from..].find(token) {
                let start = from + at;
                let boundary = line[..start]
                    .chars()
                    .next_back()
                    .is_none_or(|before| !before.is_alphanumeric() && before != '_');
                if boundary {
                    found.push((number + 1, *token));
                }
                from = start + token.len();
            }
        }
    }
    found
}

#[test]
fn canonical_wire_sources_hold_no_unordered_collection_clock_or_random_source() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    let mut violations = Vec::new();
    for entry in std::fs::read_dir(&directory).expect("the crate has sources") {
        let path = entry.expect("an entry").path();
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable source file");
        for (line, token) in banned_uses(&text) {
            violations.push(format!("{}:{line}: `{token}`", path.display()));
        }
        checked += 1;
    }
    assert_eq!(checked, 4, "the scan must read every source module");
    assert!(
        violations.is_empty(),
        "canonical client or wire bytes can vary between runs:\n{}",
        violations.join("\n")
    );
}

#[test]
fn the_scan_detects_code_and_ignores_prose_and_substrings() {
    assert_eq!(
        banned_uses("use std::collections::HashMap;"),
        vec![(1, "HashMap")]
    );
    assert!(banned_uses("// HashMap is forbidden").is_empty());
    assert!(banned_uses("let hashmap_like = DeterministicMap::new();").is_empty());
}
