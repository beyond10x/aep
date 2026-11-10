//! Adversary cases for `aep.pack/1` and the overlay: what the parser admits must be what the readers
//! see, and what the parser admits must be what `render` writes.

use std::fs;
use std::path::PathBuf;

use aep_backend_markdown::pack::{self, Overlay, Pack, PackCode};
use sha2::{Digest as _, Sha256};

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").expect("a String takes a write");
        out
    })
}

fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("aep-pack-adversary-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("the scratch is writable");
    directory
}

/// A record the parser admits is a record some reader answers with: a pack that parses cleanly and
/// holds a record no listing reaches loses that record to every verb, with no error anywhere.
#[test]
fn every_record_a_pack_parses_with_is_listed_by_the_overlay() {
    let paths = [
        "planning/story/.hidden.md",
        "planning/.kind/a.md",
        "evidence/story/x/.record.json",
        "evidence/story/.x/record.json",
    ];
    for path in paths {
        let root = scratch("dot-names");
        let body = b"held\n";
        let bytes = format!(
            "aep.pack/1\n--- {path} {} sha256:{}\n{}\n",
            body.len(),
            sha(body),
            String::from_utf8_lossy(body)
        )
        .into_bytes();
        let parsed = match Pack::parse(&bytes) {
            Err(error) => {
                assert_eq!(error.code(), PackCode::PathInvalid, "`{path}`: {error}");
                continue;
            }
            Ok(parsed) => parsed,
        };
        assert_eq!(parsed.records().len(), 1);
        fs::create_dir_all(root.join("packs")).expect("writable");
        fs::write(
            root.join("packs")
                .join(pack::file_name(&pack::name(&bytes))),
            &bytes,
        )
        .expect("writable");
        let overlay = Overlay::new(root.join("planning"), root.join("evidence"));
        let (directory, file) = path.rsplit_once('/').expect("a directory and a file");
        let extension = file.rsplit_once('.').expect("an extension").1;
        // Every directory on the way down must be listed, or a walk never arrives.
        let segments: Vec<&str> = directory.split('/').collect();
        for depth in 1..segments.len() {
            let parent = segments[..depth].join("/");
            let child = segments[depth];
            assert!(
                overlay
                    .directories(&parent)
                    .expect("lists")
                    .iter()
                    .any(|name| name == child),
                "`{path}` parsed, and no walk reaches it: `{parent}` does not list `{child}`"
            );
        }
        assert!(
            overlay
                .files(directory, extension)
                .expect("lists")
                .iter()
                .any(|name| name == file),
            "`{path}` parsed, and `{directory}` does not list `{file}`: the record is lost to every \
             reader that walks the store"
        );
    }
}

/// The parser admits exactly the bytes `render` writes for the records it read: otherwise two packs
/// with different bytes and names hold one set of records, and "the pack is a function of the set"
/// no longer holds for what the overlay reads.
#[test]
fn a_pack_that_parses_is_the_bytes_render_writes_for_its_records() {
    let body = b"held\n";
    let digest = sha(body);
    let canonical = format!("aep.pack/1\n--- planning/story/a.md 5 sha256:{digest}\nheld\n\n");
    let leading_zero = format!("aep.pack/1\n--- planning/story/a.md 05 sha256:{digest}\nheld\n\n");
    assert_eq!(
        pack::render(&[("planning/story/a.md", body.as_slice())]).expect("renders"),
        canonical.as_bytes(),
        "the canonical form is what render writes"
    );
    let bytes = leading_zero.as_bytes();
    match Pack::parse(bytes) {
        Err(error) => assert_eq!(
            error.code(),
            PackCode::RecordLineMalformed,
            "a non-canonical length is a malformed record line: {error}"
        ),
        Ok(parsed) => {
            let records: Vec<(&str, &[u8])> = parsed
                .records()
                .iter()
                .map(|record| (record.path(), record.bytes()))
                .collect();
            assert_eq!(
                pack::render(&records).expect("renders"),
                bytes,
                "parse admitted `05` as a length, so pack {} holds the set pack {} holds",
                pack::name(bytes),
                pack::name(canonical.as_bytes())
            );
        }
    }
}
