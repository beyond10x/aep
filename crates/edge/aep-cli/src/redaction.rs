//! Taking the operator out of a stream before it is written, digested or committed.
//!
//! Its own module for the reason [`crate::money`] is: the concern is one value read once and
//! applied at two unrelated boundaries. `eval run --redact` scrubs as it writes and
//! `trace redact` scrubs a file already on disk, and before this module existed the second reached
//! into the first — which put a dependency between two verb families whose module docs both say
//! they share no state with the rest of the binary.
//!
//! Everything here answers one question: **who is the operator**. There is one answer, so there is
//! one place to read it.

use std::path::{Path, PathBuf};

/// What replaces the operator's home directory in a redacted stream.
const HOME_PLACEHOLDER: &str = "~";

/// What replaces the operator's user name.
const USER_PLACEHOLDER: &str = "<user>";

/// The operator's own identity, as `--redact` removes it from a stream before anything is digested.
///
/// # Why this exists beside the report's redaction and not inside it
///
/// `--redact` on the *report* replaces every quoted command and path with a digest, which is what
/// makes a **record** publishable. It does nothing for the **stream** beside it, and the stream is
/// the document a case's `recorded/` directory is for. Eight streams recorded on 2026-09-03 carried
/// `/home/<operator>` between 18 and 49 times each and the operator's user name between 20 and 51
/// times, so none of them could be committed anywhere public.
///
/// # What is replaced, and what is deliberately not
///
/// * the home directory — `$HOME` **and** its realpath, because a home reached through a symlink
///   is the same home and only one of the two spellings is in the environment — becomes `~`;
/// * the user name — `$USER`, `$LOGNAME`, and the last segment of `$HOME`, because a machine where
///   those differ still has every one of those strings in its transcripts — becomes `<user>`;
/// * the operator's **git identity**, `user.name` and `user.email`, which is neither of the above:
///   a run that commits inside its fixture puts a real name and an address into `git log` output,
///   and a redaction that removed only the shell's idea of who is running would leave both. See
///   [`git_identity`].
///
/// Repository names, commit ids and branch names are **not** touched: they are the run's subject,
/// and a redaction that removed them would leave a stream nobody can check anything against.
///
/// # Two rules that keep this from corrupting a stream
///
/// * **Longest first.** `/home/ada` is replaced before `ada` can be, so a path never degrades to
///   `/home/<user>`.
/// * **The user name is replaced at word boundaries only**, where a boundary is anything outside
///   `[A-Za-z0-9_]`. A user named `tim` must not rewrite `multimodal`'s neighbours or a base64
///   payload that happens to contain the three letters, and a substring replacement of a short name
///   would do exactly that.
///
/// The substitution is over the **bytes of the whole stream** rather than over parsed fields, which
/// is the same decision for the same reason: an event text, a tool argument, a `cwd`, a transcript
/// path and a shell command are all places a home path appears, and a reader that enumerated the
/// fields it knew about would miss the one added next release. Neither replacement can produce
/// invalid JSON — `~`, `<` and `>` need no escaping inside a JSON string — so a redacted stream is
/// still a stream this runner reads.
#[derive(Debug, Default)]
struct Operator {
    /// Absolute paths that are this operator's home, longest first.
    homes: Vec<String>,
    /// Spellings of this operator's user name, longest first.
    names: Vec<String>,
}

impl Operator {
    /// Reads the operator's identity out of the environment.
    ///
    /// The environment and not a flag: the thing being removed is *whoever is running this*, and an
    /// operator who had to name their own home would forget on the run that mattered.
    fn from_environment(at: &[&Path]) -> Self {
        let mut homes = Vec::new();
        let mut names = Vec::new();
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            if let Some(segment) = home.file_name().and_then(|name| name.to_str()) {
                names.push(segment.to_owned());
            }
            // The realpath as well as the spelling, because `$HOME` reached through a symlink and
            // the directory it lands on are the same home and a stream carries whichever one the
            // tool that wrote the line happened to resolve.
            if let Ok(real) = std::fs::canonicalize(&home) {
                homes.push(real.display().to_string());
            }
            homes.push(home.display().to_string());
        }
        for key in ["USER", "LOGNAME"] {
            if let Some(name) = std::env::var_os(key).and_then(|value| value.into_string().ok()) {
                names.push(name);
            }
        }
        for directory in at {
            names.extend(git_identity(directory));
        }
        Self::new(homes, names)
    }

    /// Builds one from explicit values. Ordered and deduplicated here, so `scrub` cannot be wrong
    /// about precedence whatever order a caller supplied.
    fn new(homes: Vec<String>, names: Vec<String>) -> Self {
        let order = |values: Vec<String>| {
            let mut values: Vec<String> = values.into_iter().filter(|v| !v.is_empty()).collect();
            values.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
            values.dedup();
            values
        };
        Self {
            homes: order(homes),
            names: order(names),
        }
    }

    /// The stream with this operator taken out of it.
    ///
    /// Returns the bytes unchanged when there is nothing to remove, so a machine with no `HOME` in
    /// the environment produces a stream identical to the one it recorded rather than a mangled
    /// one.
    fn scrub(&self, bytes: &[u8]) -> Vec<u8> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            // Not text; there are no paths in it this could find, and rewriting bytes it cannot
            // read would corrupt whatever it is.
            return bytes.to_vec();
        };
        let mut scrubbed = text.to_owned();
        for home in &self.homes {
            scrubbed = scrubbed.replace(home.as_str(), HOME_PLACEHOLDER);
        }
        for name in &self.names {
            // Never let a placeholder match itself. `<` and `>` are both outside `[A-Za-z0-9_]`,
            // so an operator literally named `user` would turn `<user>` into `<<user>>` — and the
            // second pass `trace redact` exists to perform would do it again, and change the
            // digest each time. Idempotence is a promise this module makes in three places.
            if USER_PLACEHOLDER.contains(name.as_str()) {
                continue;
            }
            scrubbed = replace_word(&scrubbed, name, USER_PLACEHOLDER);
        }
        scrubbed.into_bytes()
    }

    /// Whether this operator has anything to remove.
    fn is_empty(&self) -> bool {
        self.homes.is_empty() && self.names.is_empty()
    }
}

/// The operator's git identity: the `user.name` and `user.email` git would author a commit with.
///
/// Read at a **named directory**, and by both callers at every directory that could hold one. A
/// git identity is per repository: the runner's own checkout may carry a bot override while the
/// fixture the run committed in carries the operator's real name, and reading only the first
/// scrubs the bot and leaves the person — silently, because a redaction that removed something
/// looks exactly like one that removed the right thing. A recorded run commits inside its fixture, so the stream carries `git log` output, and
/// `git log` prints the author — which is a person's real name and their address, neither of which
/// is `$USER`. The golden-path recording of 2026-09-03 went to disk redacted and still carried the
/// operator's own name four times: twice as a commit author, and twice inside a
/// `git -c user.name=...` call the agent made after reading the value out of `git config`.
///
/// The name itself is not written here. A doc comment naming the operator this function exists to
/// remove would be the same leak in a file nothing redacts.
///
/// A failure to read git is silence, not a refusal: a machine with no git has no identity to
/// remove. A directory that is not a repository is **not** that case — `git config --get` falls
/// back to the global file, which is the operator's own identity and exactly what should be
/// removed. What a directory changes is only whether a **local** override wins, which is why both
/// callers pass every directory that could carry one.
fn git_identity(at: &Path) -> Vec<String> {
    ["user.name", "user.email"]
        .into_iter()
        .filter_map(|key| {
            let output = std::process::Command::new("git")
                .current_dir(at)
                .args(["config", "--get", key])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
            (!value.is_empty()).then_some(value)
        })
        .filter(|value| removable(value))
        .collect()
}

/// Whether a free-text identity is safe to replace everywhere it stands as a word.
///
/// `$USER` is a login name and is constrained by the system. `user.name` is a text field somebody
/// typed, and in a container or a CI job it is routinely `root`, `CI`, `Bot` or `Test`. Replacing
/// one of those everywhere would rewrite `root cause` to `<user> cause` and digest the result into
/// a manifest — corrupting the stream to hide a name that identifies nobody.
///
/// So a single token is only removed when it is long enough to be a name rather than a word: an
/// address always is, and so is anything with a space in it. This is deliberately not a blocklist
/// of common words, because the failure it prevents is corruption and a blocklist is never
/// complete.
fn removable(value: &str) -> bool {
    value.contains('@') || value.contains(' ') || value.chars().count() >= MIN_SINGLE_TOKEN
}

/// How long a one-word identity has to be before it is removed rather than left alone.
///
/// Eight, because the four that actually occur — `root`, `CI`, `Bot`, `Test` — are at most four,
/// and a name shorter than this is one whose removal would be more likely to corrupt a stream than
/// to protect anybody. A shorter real name is not redacted, and that is the trade this states
/// rather than hides.
const MIN_SINGLE_TOKEN: usize = 8;

/// Replaces `needle` with `replacement` where it stands as a word.
///
/// A word boundary is anything outside `[A-Za-z0-9_]`, so `ada` in `/home/ada`, `"user":"ada"` and
/// `ada-scratch` is replaced and `ada` inside `adapter` is not. Written out rather than reached for
/// with a regular expression because the pattern is one literal and a dependency here would be a
/// dependency for the whole binary.
pub(crate) fn replace_word(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_owned();
    }
    let word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let bytes = haystack.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0;
    while let Some(hit) = haystack[cursor..].find(needle) {
        let start = cursor + hit;
        let end = start + needle.len();
        let bounded = (start == 0 || !word(bytes[start - 1]))
            && (end == bytes.len() || !word(bytes[end]));
        out.push_str(&haystack[cursor..start]);
        out.push_str(if bounded { replacement } else { &haystack[start..end] });
        cursor = end;
    }
    out.push_str(&haystack[cursor..]);
    out
}

/// This operator taken out of a stream, wherever the bytes came from.
///
/// Both callers reach it here: `eval run --redact` as it writes, `trace redact` for a file already
/// on disk. Idempotent — a stream redacted by an older build carries whatever that build did not
/// know to remove, and running this over it removes the rest without disturbing what is already a
/// placeholder.
pub(crate) fn redacted(events: Vec<u8>, at: &[&Path]) -> Vec<u8> {
    let operator = Operator::from_environment(at);
    if operator.is_empty() {
        return events;
    }
    operator.scrub(&events)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_removes_both_spellings_of_the_home_and_then_the_name() {
        // Longest first, which is the rule that keeps `/home/ada` from degrading to `/home/<user>`,
        // and both spellings of the home because a symlinked `$HOME` and its realpath are the same
        // home and a stream carries whichever one the tool that wrote the line resolved.
        let operator = Operator::new(
            vec!["/home/ada".to_owned(), "/mnt/users/ada".to_owned()],
            vec!["ada".to_owned()],
        );
        let stream = concat!(
            r#"{"cwd":"/home/ada/work","transcript":"/mnt/users/ada/.cache/t.jsonl","#,
            r#""text":"ada ran it from /home/ada as ada-scratch","user":"ada"}"#
        );
        let scrubbed = String::from_utf8(operator.scrub(stream.as_bytes())).expect("still text");
        assert!(!scrubbed.contains("/home/ada") && !scrubbed.contains("/mnt/users/ada"));
        assert!(!scrubbed.contains("ada\""), "{scrubbed}");
        assert!(
            scrubbed.contains(r#""cwd":"~/work""#)
                && scrubbed.contains(r#""transcript":"~/.cache/t.jsonl""#)
                && scrubbed.contains("<user> ran it from ~ as <user>-scratch")
                && scrubbed.contains(r#""user":"<user>""#),
            "{scrubbed}"
        );
        // Still JSON: neither placeholder needs escaping inside a JSON string.
        serde_json::from_str::<serde_json::Value>(&scrubbed).expect("a redacted line is JSON");
    }

    #[test]
    fn a_git_author_is_removed_where_the_user_name_would_not_have_been() {
        // What the golden-path recording of 2026-09-03 carried after `--redact`: a real name and an
        // address, four times, none of them `$USER`. Two came out of `git log` inside the fixture
        // the run committed in, two out of an argument the agent read from `git config` and typed
        // back. `$HOME` and `$USER` cannot reach either.
        let operator = Operator::new(
            vec!["/home/ada".to_owned()],
            vec![
                "ada".to_owned(),
                "Ada Lovelace".to_owned(),
                "17+ada@users.noreply.github.com".to_owned(),
            ],
        );
        let stream = concat!(
            r#"{"content":"Author: Ada Lovelace <17+ada@users.noreply.github.com>","#,
            r#""command":"git -c user.name=\"Ada Lovelace\" commit -q -m x"}"#
        );
        let scrubbed = String::from_utf8(operator.scrub(stream.as_bytes())).expect("still text");
        assert!(!scrubbed.contains("Ada Lovelace"), "{scrubbed}");
        assert!(!scrubbed.contains("noreply.github.com"), "{scrubbed}");
        assert!(scrubbed.contains("Author: <user> <<user>>"), "{scrubbed}");
        serde_json::from_str::<serde_json::Value>(&scrubbed).expect("a redacted line is JSON");
    }

    #[test]
    fn a_common_one_word_git_name_is_left_alone_rather_than_corrupting_the_stream() {
        // `user.name` is free text somebody typed, and in a container it is routinely `root`. A
        // list that took it would rewrite `root cause` and digest the result into a manifest —
        // corrupting a stream to hide a name that identifies nobody.
        assert!(!removable("root"));
        assert!(!removable("CI"));
        assert!(!removable("Bot"));
        assert!(removable("Ada Lovelace"), "a space makes it a name");
        assert!(removable("17+ada@users.noreply.github.com"), "so does an address");
        assert!(removable("alexandra"), "and so does length");

        // The guard is on this field and not on `$USER`, which is deliberate and is the whole of
        // where it applies. `$USER` is a login name the system constrains; `user.name` is a text
        // field with no constraint at all, and it is the one this release started reading. A
        // machine whose `$USER` is genuinely `root` keeps the behaviour it had, because widening
        // the guard would silently stop redacting an operator who has always been redacted.
        //
        // Whatever this machine's git says, every value that gets through is one `scrub` can apply
        // without rewriting ordinary words.
        for value in git_identity(std::path::Path::new(".")) {
            assert!(removable(&value), "git_identity let through `{value}`");
        }
    }

    #[test]
    fn an_operator_named_user_does_not_make_the_placeholder_match_itself() {
        // `<` and `>` are both outside `[A-Za-z0-9_]`, so `<user>` is a word-bounded `user`. A
        // second pass would yield `<<user>>`, and `trace redact` exists to be a second pass.
        let operator = Operator::new(vec!["/home/user".to_owned()], vec!["user".to_owned()]);
        let once = operator.scrub(r#"{"cwd":"/home/user","who":"user"}"#.as_bytes());
        let twice = operator.scrub(&once);
        assert_eq!(once, twice, "a second pass changes nothing");
        let text = String::from_utf8(once).expect("still text");
        assert!(!text.contains("<<user>>"), "{text}");
    }

    #[test]
    fn redaction_is_idempotent_so_a_stream_can_be_cleaned_twice() {
        // `protocol trace redact` exists to finish a stream an older `--redact` wrote. Applying the
        // removal to its own output must be a no-op, or the verb would eat placeholders.
        let operator = Operator::new(vec!["/home/ada".to_owned()], vec!["ada".to_owned()]);
        let once = operator.scrub(r#"{"cwd":"/home/ada/work","who":"ada"}"#.as_bytes());
        let twice = operator.scrub(&once);
        assert_eq!(once, twice, "a second pass changes nothing");
    }

    #[test]
    fn a_user_name_is_replaced_as_a_word_and_never_inside_another_one() {
        // A short user name is the case that would corrupt a stream: `tim` as a substring occurs in
        // `runtime`, `estimate` and any base64 payload that happens to contain the three letters,
        // and a checker reading a stream with those rewritten would be reading a different run.
        let operator = Operator::new(Vec::new(), vec!["tim".to_owned()]);
        let stream = r#"{"text":"tim measured the runtime estimate; tim-2 did not","who":"tim"}"#;
        let scrubbed = String::from_utf8(operator.scrub(stream.as_bytes())).expect("still text");
        assert_eq!(
            scrubbed,
            r#"{"text":"<user> measured the runtime estimate; <user>-2 did not","who":"<user>"}"#
        );
    }

    #[test]
    fn an_operator_with_nothing_to_remove_leaves_the_stream_byte_identical() {
        // A machine with no `HOME` in the environment records the stream it recorded, rather than a
        // mangled one — and the digest in the manifest is over exactly those bytes.
        let stream = b"{\"text\":\"nothing here\"}";
        assert!(Operator::default().is_empty());
        assert_eq!(Operator::default().scrub(stream), stream.to_vec());
        // And `redacted` over a directory that is not a repository finds no identity to remove,
        // which is the same answer by the other road.
        let nowhere = std::path::Path::new("/");
        assert_eq!(redacted(stream.to_vec(), &[nowhere]), stream.to_vec());
    }
}
