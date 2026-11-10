//! The `aep.pack/1` format and the read overlay over loose files and packs.

use std::fs;
use std::path::PathBuf;

use aep_backend_markdown::pack::{self, Holder, Overlay, Pack, PackCode, Place};
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

fn record_line(path: &str, bytes: &[u8]) -> String {
    format!("--- {path} {} sha256:{}\n", bytes.len(), sha(bytes))
}

fn code_of(bytes: &[u8]) -> PackCode {
    Pack::parse(bytes).expect_err("the pack is refused").code()
}

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("aep-pack-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("the scratch is writable");
    directory
}

#[test]
fn render_sorts_by_path_and_is_a_function_of_the_set() {
    let forward = pack::render(&[
        ("planning/story/b.md", b"B".as_slice()),
        ("evidence/story/a/1.json", b"{}\n".as_slice()),
        ("planning/story/a.md", b"A\n".as_slice()),
    ])
    .expect("renders");
    let backward = pack::render(&[
        ("planning/story/a.md", b"A\n".as_slice()),
        ("planning/story/b.md", b"B".as_slice()),
        ("evidence/story/a/1.json", b"{}\n".as_slice()),
    ])
    .expect("renders");
    assert_eq!(forward, backward);

    let mut expected = String::from("aep.pack/1\n");
    expected.push_str(&record_line("evidence/story/a/1.json", b"{}\n"));
    expected.push_str("{}\n\n");
    expected.push_str(&record_line("planning/story/a.md", b"A\n"));
    expected.push_str("A\n\n");
    expected.push_str(&record_line("planning/story/b.md", b"B"));
    expected.push_str("B\n");
    assert_eq!(String::from_utf8(forward).expect("text"), expected);
}

#[test]
fn render_refuses_a_repeated_path() {
    let error = pack::render(&[
        ("planning/story/a.md", b"one".as_slice()),
        ("planning/story/a.md", b"two".as_slice()),
    ])
    .expect_err("a path is held once");
    assert_eq!(error.code(), PackCode::PathRepeated);
}

#[test]
fn the_name_is_the_first_sixteen_hex_of_the_sha256_of_the_packs_bytes() {
    let bytes = pack::render(&[("planning/story/a.md", b"A\n".as_slice())]).expect("renders");
    let name = pack::name(&bytes);
    assert_eq!(name, sha(&bytes)[..16]);
    assert_eq!(name.len(), 16);
    assert_eq!(pack::file_name(&name), format!("{name}.pack"));
    assert_eq!(Pack::parse(&bytes).expect("parses").name(), name);
}

#[test]
fn parse_reads_back_what_render_wrote_byte_for_byte() {
    let body = b"---\nid: story:a\n---\n\nbody without a final newline";
    let bytes = pack::render(&[
        ("planning/story/a.md", body.as_slice()),
        (
            "evidence/story/a/20261110T000000Z-000-aaaaaaaaaaaa.json",
            b"{}\n".as_slice(),
        ),
    ])
    .expect("renders");
    let parsed = Pack::parse(&bytes).expect("parses");
    let records = parsed.records();
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0].path(),
        "evidence/story/a/20261110T000000Z-000-aaaaaaaaaaaa.json"
    );
    assert_eq!(records[1].path(), "planning/story/a.md");
    assert_eq!(records[1].bytes(), body);
    assert!(records[1].digest_matches());
    assert_eq!(records[1].sha256(), sha(body));
}

#[test]
fn parse_refuses_a_missing_or_wrong_header() {
    assert_eq!(code_of(b""), PackCode::HeaderInvalid);
    assert_eq!(code_of(b"aep.pack/2\n"), PackCode::HeaderInvalid);
    assert_eq!(code_of(b"aep.pack/1"), PackCode::HeaderInvalid);
}

#[test]
fn parse_refuses_a_truncated_record() {
    let good = pack::render(&[("planning/story/a.md", b"hello".as_slice())]).expect("renders");

    // Cut inside the record line: no line end.
    let line_end = good
        .iter()
        .rposition(|byte| *byte == b'\n')
        .expect("a line end");
    let header_len = "aep.pack/1\n".len();
    assert_eq!(code_of(&good[..header_len + 10]), PackCode::RecordTruncated);

    // Cut after the bytes, before the closing newline.
    assert_eq!(code_of(&good[..line_end]), PackCode::RecordTruncated);
}

#[test]
fn parse_refuses_a_length_past_the_end() {
    let mut bytes = b"aep.pack/1\n".to_vec();
    bytes.extend_from_slice(
        format!(
            "--- planning/story/a.md 99 sha256:{}\nshort\n",
            sha(b"short")
        )
        .as_bytes(),
    );
    assert_eq!(code_of(&bytes), PackCode::LengthPastEnd);

    let cut = pack::render(&[("planning/story/a.md", b"hello world".as_slice())]).expect("renders");
    assert_eq!(code_of(&cut[..cut.len() - 5]), PackCode::LengthPastEnd);
}

#[test]
fn parse_refuses_bytes_not_followed_by_a_line_end() {
    let mut bytes = b"aep.pack/1\n".to_vec();
    bytes.extend_from_slice(record_line("planning/story/a.md", b"hello").as_bytes());
    bytes.extend_from_slice(b"helloXtrailing\n");
    assert_eq!(code_of(&bytes), PackCode::RecordUnterminated);
}

#[test]
fn parse_refuses_an_unsorted_and_a_repeated_path() {
    let mut unsorted = b"aep.pack/1\n".to_vec();
    for path in ["planning/story/b.md", "planning/story/a.md"] {
        unsorted.extend_from_slice(record_line(path, b"x").as_bytes());
        unsorted.extend_from_slice(b"x\n");
    }
    assert_eq!(code_of(&unsorted), PackCode::PathUnsorted);

    let mut repeated = b"aep.pack/1\n".to_vec();
    for _ in 0..2 {
        repeated.extend_from_slice(record_line("planning/story/a.md", b"x").as_bytes());
        repeated.extend_from_slice(b"x\n");
    }
    assert_eq!(code_of(&repeated), PackCode::PathRepeated);
}

#[test]
fn parse_refuses_a_path_outside_the_grammar() {
    for path in [
        "planning/story/../x.md",
        "planning/../story/x.md",
        "/planning/story/x.md",
        "planning\\story\\x.md",
        "planning//x.md",
        "planning/story//x.md",
        "planning/./x.md",
        "planning/story/./x.md",
        "planning/story/x.md/",
        "planning/story/\u{1}x.md",
        "planning/story/x\u{7f}.md",
        "planning/x.md",
        "planning/story/x.txt",
        "planning/story/.md",
        "planning/story/deep/x.md",
        "evidence/story/x.json",
        "evidence/story/x/y.md",
        "evidence/story/x/.json",
        "evidence/story/x/y/z.json",
        "notes/story/x.md",
        "story/x.md",
        "",
    ] {
        let mut bytes = b"aep.pack/1\n".to_vec();
        bytes.extend_from_slice(record_line(path, b"x").as_bytes());
        bytes.extend_from_slice(b"x\n");
        let error = Pack::parse(&bytes).expect_err(path);
        assert!(
            matches!(
                error.code(),
                PackCode::PathInvalid | PackCode::RecordLineMalformed
            ),
            "{path:?}: {error}"
        );
        if !path.is_empty() {
            assert_eq!(error.code(), PackCode::PathInvalid, "{path:?}: {error}");
        }
        let refused = pack::render(&[(path, b"x".as_slice())]);
        assert_eq!(
            refused.expect_err(path).code(),
            PackCode::PathInvalid,
            "{path:?}"
        );
    }
}

#[test]
fn parse_refuses_a_nul_in_a_path() {
    let mut bytes = b"aep.pack/1\n".to_vec();
    bytes.extend_from_slice(b"--- planning/story/a\0b.md 1 sha256:");
    bytes.extend_from_slice(sha(b"x").as_bytes());
    bytes.extend_from_slice(b"\nx\n");
    assert_eq!(code_of(&bytes), PackCode::PathInvalid);
}

#[test]
fn parse_refuses_a_malformed_record_line_and_digest() {
    let mut bytes = b"aep.pack/1\n".to_vec();
    bytes.extend_from_slice(b"--- planning/story/a.md twelve sha256:00\nx\n");
    assert_eq!(code_of(&bytes), PackCode::RecordLineMalformed);

    let mut bytes = b"aep.pack/1\n".to_vec();
    bytes.extend_from_slice(b"--- planning/story/a.md 1 sha256:xyz\nx\n");
    assert_eq!(code_of(&bytes), PackCode::DigestMalformed);
}

#[test]
fn a_record_whose_bytes_differ_from_its_digest_parses_and_says_so() {
    let mut bytes = b"aep.pack/1\n".to_vec();
    bytes.extend_from_slice(record_line("planning/story/a.md", b"x").as_bytes());
    bytes.extend_from_slice(b"y\n");
    let parsed = Pack::parse(&bytes).expect("the structure is sound; the digest is validate's");
    assert!(!parsed.records()[0].digest_matches());
}

fn engineering(name: &str) -> (PathBuf, Overlay) {
    let root = scratch(name);
    let overlay = Overlay::new(root.join("planning"), root.join("evidence"));
    (root, overlay)
}

fn write_pack(root: &std::path::Path, records: &[(&str, &[u8])]) -> String {
    let bytes = pack::render(records).expect("renders");
    let name = pack::name(&bytes);
    let directory = root.join("packs");
    fs::create_dir_all(&directory).expect("writable");
    fs::write(directory.join(pack::file_name(&name)), &bytes).expect("writable");
    name
}

#[test]
fn the_overlay_lists_every_place_that_holds_a_path_with_the_bytes_each_holds() {
    let (root, overlay) = engineering("holders");
    fs::create_dir_all(root.join("planning/story")).expect("writable");
    fs::write(root.join("planning/story/a.md"), b"loose").expect("writable");
    let first = write_pack(&root, &[("planning/story/a.md", b"packed one".as_slice())]);
    let second = write_pack(
        &root,
        &[
            ("planning/story/a.md", b"packed two".as_slice()),
            ("planning/story/b.md", b"only b".as_slice()),
        ],
    );

    let holders = overlay.holders("planning/story/a.md").expect("lists");
    let mut expected = vec![
        Holder::new(Place::Loose, b"loose".to_vec()),
        Holder::new(Place::Packed(first.clone()), b"packed one".to_vec()),
        Holder::new(Place::Packed(second.clone()), b"packed two".to_vec()),
    ];
    expected[1..].sort_by(|left, right| left.place().cmp(right.place()));
    assert_eq!(holders, expected);

    let only_b = overlay.holders("planning/story/b.md").expect("lists");
    assert_eq!(
        only_b,
        vec![Holder::new(Place::Packed(second), b"only b".to_vec())]
    );
    assert_eq!(
        overlay.holders("planning/story/none.md").expect("lists"),
        Vec::<Holder>::new()
    );
}

#[test]
fn a_store_with_two_packs_reads_both_as_one_tree() {
    let (root, overlay) = engineering("two-packs");
    fs::create_dir_all(root.join("planning/story")).expect("writable");
    fs::write(root.join("planning/story/loose.md"), b"loose").expect("writable");
    write_pack(&root, &[("planning/story/one.md", b"1".as_slice())]);
    write_pack(
        &root,
        &[
            ("planning/story/two.md", b"2".as_slice()),
            ("evidence/story/two/e.json", b"{}\n".as_slice()),
        ],
    );

    assert_eq!(
        overlay.read("planning/story/one.md").expect("reads"),
        Some(b"1".to_vec())
    );
    assert_eq!(
        overlay.read("planning/story/two.md").expect("reads"),
        Some(b"2".to_vec())
    );
    assert_eq!(
        overlay.read("planning/story/loose.md").expect("reads"),
        Some(b"loose".to_vec())
    );
    assert_eq!(overlay.read("planning/story/none.md").expect("reads"), None);
    assert_eq!(
        overlay.files("planning/story", "md").expect("lists"),
        ["loose.md", "one.md", "two.md"]
    );
    assert_eq!(overlay.directories("planning").expect("lists"), ["story"]);
    assert_eq!(overlay.directories("evidence").expect("lists"), ["story"]);
    assert_eq!(
        overlay.directories("evidence/story").expect("lists"),
        ["two"]
    );
    assert_eq!(
        overlay.files("evidence/story/two", "json").expect("lists"),
        ["e.json"]
    );
}

#[test]
fn a_loose_copy_answers_a_read_before_a_packed_one() {
    let (root, overlay) = engineering("loose-first");
    fs::create_dir_all(root.join("planning/story")).expect("writable");
    fs::write(root.join("planning/story/a.md"), b"loose").expect("writable");
    write_pack(&root, &[("planning/story/a.md", b"packed".as_slice())]);
    assert_eq!(
        overlay.read("planning/story/a.md").expect("reads"),
        Some(b"loose".to_vec())
    );
}

#[test]
fn a_pack_that_does_not_parse_is_reported_by_name_not_skipped() {
    let (root, overlay) = engineering("bad-pack");
    fs::create_dir_all(root.join("packs")).expect("writable");
    fs::write(root.join("packs/0123456789abcdef.pack"), b"not a pack").expect("writable");
    let packs = overlay.packs();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].name(), "0123456789abcdef");
    assert_eq!(
        packs[0].pack().as_ref().expect_err("refused").code(),
        PackCode::HeaderInvalid
    );
    let error = overlay
        .read("planning/story/a.md")
        .expect_err("a corrupt pack is not silence");
    assert!(error.to_string().contains("0123456789abcdef"), "{error}");
}
