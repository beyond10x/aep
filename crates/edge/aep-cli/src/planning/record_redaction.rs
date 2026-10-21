//! The text half of `aep plan artifact redact`: case-folded matching, the digest a redaction record
//! keeps instead of the literal, and the replay `validate` runs over a review's first version.
//!
//! One matcher for both. The verb compares a window's folded text with the folded literal, and
//! `validate` compares the window's folded digest with the one the record kept; both walk the text
//! the same way — left to right, non-overlapping, each window the run of whole characters whose
//! folded form is as long as the folded literal — so a record replays to exactly the text the verb
//! wrote.
//!
//! A window is measured folded, not as written, because folding can lengthen a character: `İ`
//! (U+0130) folds to `i` and U+0307, so `ACMEİ` is five characters and its folded form, the folded
//! form of `acmei̇`, is six. Measured as written, one of the two spellings would never be seen.

use sha2::{Digest as _, Sha256};

/// `text` folded to lowercase, character by character.
pub(crate) fn fold(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// How many characters `literal` folds to: the window width a redaction record keeps.
pub(crate) fn folded_chars(literal: &str) -> u64 {
    literal
        .chars()
        .map(|character| character.to_lowercase().count() as u64)
        .sum()
}

/// Whether `text` holds `literal` in some letter case anywhere, the inside of a folded character
/// included.
pub(crate) fn holds(text: &str, literal: &str) -> bool {
    fold(text).contains(&fold(literal))
}

/// The lowercase hex SHA-256 of `literal` folded, as a redaction record keeps it.
pub(crate) fn folded_digest(literal: &str) -> String {
    Sha256::digest(fold(literal).as_bytes())
        .iter()
        .fold(String::new(), |mut output, byte| {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

/// `text` with every window whose folded form spans `width` characters and that `matches` replaced
/// by `replacement`, left to right and non-overlapping, and how many windows matched.
///
/// `width` comes from a record a person can write by hand, so no value of it is trusted: the walk
/// adds only widths of characters the text holds, and stops once the folded text left is shorter
/// than `width`.
pub(crate) fn replaced(
    text: &str,
    width: u64,
    matches: impl Fn(&str) -> bool,
    replacement: &str,
) -> (String, u64) {
    if width == 0 {
        return (text.to_owned(), 0);
    }
    // Byte offsets of every character, plus the end, so a window is a slice.
    let mut offsets: Vec<usize> = text.char_indices().map(|(offset, _)| offset).collect();
    offsets.push(text.len());
    let widths: Vec<u64> = text
        .chars()
        .map(|character| character.to_lowercase().count() as u64)
        .collect();
    let characters = widths.len();
    let mut left: u64 = widths.iter().sum();
    let mut output = String::with_capacity(text.len());
    let mut count = 0;
    let mut index = 0;
    while index < characters && left >= width {
        let mut spanned: u64 = 0;
        let mut end = index;
        while end < characters && spanned < width {
            spanned = spanned.saturating_add(widths[end]);
            end += 1;
        }
        if spanned == width && matches(&text[offsets[index]..offsets[end]]) {
            output.push_str(replacement);
            count += 1;
            left -= spanned;
            index = end;
            continue;
        }
        output.push_str(&text[offsets[index]..offsets[index + 1]]);
        left -= widths[index];
        index += 1;
    }
    output.push_str(&text[offsets[index]..]);
    (output, count)
}

/// `text` with every case-insensitive occurrence of `literal` replaced, and how many there were.
pub(crate) fn replace_literal(text: &str, literal: &str, replacement: &str) -> (String, u64) {
    let folded = fold(literal);
    replaced(
        text,
        folded_chars(literal),
        |window| fold(window) == folded,
        replacement,
    )
}

/// `text` with every window whose folded digest is `digest` replaced, and how many there were.
pub(crate) fn replace_digest(
    text: &str,
    digest: &str,
    width: u64,
    replacement: &str,
) -> (String, u64) {
    replaced(
        text,
        width,
        |window| folded_digest(window) == digest,
        replacement,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_letter_cases_are_three_occurrences() {
        let (text, count) = replace_literal("Acme, ACME and acme.", "acme", "customer A");
        assert_eq!(count, 3);
        assert_eq!(text, "customer A, customer A and customer A.");
    }

    #[test]
    fn the_digest_replay_writes_what_the_literal_wrote() {
        let original = "Für Acme: ACMEacme — done";
        let (by_literal, literal_count) = replace_literal(original, "Acme", "X");
        let (by_digest, digest_count) =
            replace_digest(original, &folded_digest("aCmE"), 4, "X");
        assert_eq!((by_literal.as_str(), literal_count), ("Für X: XX — done", 3));
        assert_eq!((by_digest, digest_count), (by_literal, literal_count));
    }

    #[test]
    fn occurrences_do_not_overlap_and_are_taken_left_to_right() {
        assert_eq!(replace_literal("aaaa", "aa", "b"), ("bb".to_owned(), 2));
        assert_eq!(replace_literal("aaa", "aa", "b"), ("ba".to_owned(), 1));
    }

    #[test]
    fn the_digest_is_of_the_folded_literal() {
        assert_eq!(folded_digest("ACME"), folded_digest("acme"));
        assert_eq!(folded_digest("").len(), 64);
    }

    #[test]
    fn a_character_that_folds_longer_is_matched_in_both_spellings() {
        let text = "ACME\u{130} and acmei\u{307}.";
        assert_eq!(folded_chars("Acme\u{130}"), 6);
        let (by_literal, count) = replace_literal(text, "Acme\u{130}", "X");
        assert_eq!((by_literal.as_str(), count), ("X and X.", 2));
        let replayed = replace_digest(text, &folded_digest("Acme\u{130}"), 6, "X");
        assert_eq!(replayed, (by_literal, count));
    }

    #[test]
    fn a_width_no_text_can_hold_matches_nothing() {
        let text = "Acme approved it.";
        for width in [u64::MAX, u64::MAX - 1, 1 << 40] {
            assert_eq!(
                replace_digest(text, &folded_digest("acme"), width, "X"),
                (text.to_owned(), 0)
            );
        }
    }
}
