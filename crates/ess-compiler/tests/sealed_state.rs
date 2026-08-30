//! The two validation boundaries stay one-way doors.

use std::path::Path;

/// Reads a workspace source file.
fn source(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

/// Extracts one named struct body without relying on line numbers.
fn struct_body<'a>(text: &'a str, declaration: &str) -> &'a str {
    let start = text
        .find(declaration)
        .unwrap_or_else(|| panic!("{declaration} exists"));
    let body = &text[start + declaration.len()..];
    let end = body.find("\n}").expect("the struct closes");
    &body[..end]
}

#[test]
fn validated_and_resolved_state_have_no_public_fields() {
    for (path, declaration) in [
        (
            "crates/ess-domain/src/spec.rs",
            "pub struct Specification {",
        ),
        ("crates/ess-compiler/src/ir.rs", "pub struct EssIr {"),
    ] {
        let text = source(path);
        let body = struct_body(&text, declaration);
        assert!(
            !body.lines().any(|line| line.trim_start().starts_with("pub ")),
            "{declaration} exposes a field, reopening construction outside its validation boundary:\n{body}"
        );
    }
}

#[test]
fn every_compiler_entrance_validates_before_resolution() {
    let text = source("crates/ess-compiler/src/resolve.rs");
    let entrance = text
        .split("pub fn compile_locating")
        .nth(1)
        .expect("the locating entrance exists")
        .split("/// What looking a reference up found.")
        .next()
        .expect("the entrance ends before the next item");
    let validation = entrance
        .find("specification.validate()")
        .expect("the sealed specification is revalidated");
    let resolution = entrance
        .find("Resolver::new")
        .expect("resolution still follows validation");
    assert!(
        validation < resolution,
        "resolution began before complete validation:\n{entrance}"
    );
}

#[test]
fn provenance_never_hashes_an_empty_serialization_fallback() {
    let text = source("crates/ess-gen/src/provenance.rs");
    assert!(
        !text.contains("serde_json::to_vec(ir).unwrap_or_default()"),
        "a serialization failure must be named, never turned into the digest of empty bytes"
    );
    assert!(
        text.contains("cannot digest an IR that does not serialize"),
        "the explicit failure should say which integrity boundary failed"
    );
}
