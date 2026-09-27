//! Content blobs: every large value an `aep.project/4` store records, kept once and named by digest.
//!
//! Entity Runtime records a decision's instance several times over — in the command, the result,
//! the change and the committed instance — and an AEP document repeats an artifact's body in its
//! fields and in each event that set it. An `aep.project/3` store therefore wrote one body a dozen
//! times per batch. An `aep.project/4` store hands Entity Runtime a reference in place of each large
//! value, and keeps the value itself once in a content-addressed directory beside the authority:
//!
//! | reference | stands for | the blob holds |
//! |---|---|---|
//! | `aep-blob:text:sha256:<hex>` | a string of [`INLINE_LIMIT`] bytes or more | its UTF-8 bytes |
//! | `aep-blob:hex:sha256:<hex>` | a canonical `hex:` byte string of that size | the decoded bytes |
//! | `aep-blob:lines:sha256:<hex>` | such a byte string holding JSON lines | the JSON list of its lines' digests |
//! | `aep-blob:json:sha256:<hex>` | the value of a top-level `document` field | its JSON, references kept |
//!
//! A byte string of JSON lines — two or more lines, each a JSON object, the last one ended — is
//! kept line by line, each line a blob of its own. The legacy journal a migration captured is such
//! a string, and each of its lines is also the exact bytes of one imported legacy record: kept this
//! way, the capture names those records' blobs rather than holding a second copy of them. Any other
//! byte string, a captured Markdown file among them, is one blob.
//!
//! The digest is SHA-256 over the blob's bytes and the file is `<root>/<hex[..2]>/<hex>`, so
//! `sha256sum` checks a blob and two branches that stored one value wrote one identical file.
//!
//! A string that already begins with [`PREFIX`] is always stored as a text blob, whatever its
//! size, so every string a reader finds with that prefix is a reference and none is ambiguous.
//!
//! Reading resolves every reference before AEP sees a value, so everything above the provider —
//! artifacts, statuses, history, audit and the projection — reads exactly what it read from an
//! `aep.project/3` store.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value};
use sha2::Digest as _;

/// What every reference begins with.
pub const PREFIX: &str = "aep-blob:";
/// A string shorter than this many UTF-8 bytes stays inline: a reference is 83 bytes long.
pub const INLINE_LIMIT: usize = 256;
/// The field whose whole value is stored as one JSON blob.
const DOCUMENT: &str = "document";

/// How a blob's bytes are read back as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Text,
    Hex,
    Lines,
    Json,
}

impl Form {
    const fn word(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Hex => "hex",
            Self::Lines => "lines",
            Self::Json => "json",
        }
    }
}

/// A parsed reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    form: Form,
    digest: String,
}

impl Reference {
    /// Parses `text` as a reference. `None` when it does not begin with [`PREFIX`].
    ///
    /// # Errors
    /// A string with the prefix that is not a well-formed reference.
    pub fn parse(text: &str) -> Option<Result<Self, String>> {
        let rest = text.strip_prefix(PREFIX)?;
        let parsed = (|| {
            let (word, digest) = rest.split_once(":sha256:")?;
            let form = match word {
                "text" => Form::Text,
                "hex" => Form::Hex,
                "lines" => Form::Lines,
                "json" => Form::Json,
                _ => return None,
            };
            is_digest(digest).then(|| Self {
                form,
                digest: digest.to_owned(),
            })
        })();
        Some(parsed.ok_or_else(|| format!("`{text}` is not a content-blob reference")))
    }

    /// The hexadecimal SHA-256 digest the reference names.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    fn wire(&self) -> String {
        format!("{PREFIX}{}:sha256:{}", self.form.word(), self.digest)
    }
}

fn is_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn lower_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// The bytes a `hex:` string names, when it is in the one spelling [`lower_hex`] writes back.
fn canonical_hex(text: &str) -> Option<Vec<u8>> {
    let digits = text.strip_prefix("hex:")?;
    if digits.len() % 2 != 0
        || !digits
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    (0..digits.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&digits[at..at + 2], 16).ok())
        .collect()
}

/// Whether `bytes` are JSON lines: at least two lines, each `{…}`, and a final newline.
fn is_json_lines(bytes: &[u8]) -> bool {
    let Some(body) = bytes.strip_suffix(b"\n") else {
        return false;
    };
    let mut lines = 0;
    for line in body.split(|byte| *byte == b'\n') {
        if !(line.starts_with(b"{") && line.ends_with(b"}")) {
            return false;
        }
        lines += 1;
    }
    lines >= 2
}

/// The SHA-256 digest of `bytes`, as lower-case hexadecimal.
#[must_use]
pub fn digest_of(bytes: &[u8]) -> String {
    lower_hex(&sha2::Sha256::digest(bytes))
}

/// A content-addressed blob directory.
#[derive(Debug)]
pub struct ContentStore {
    root: PathBuf,
    /// Blobs already read or written by this process, checked once.
    held: Mutex<BTreeMap<String, Arc<Vec<u8>>>>,
}

impl ContentStore {
    /// The store at `root`. Nothing is created until a blob is written.
    #[must_use]
    pub fn at(root: PathBuf) -> Self {
        Self {
            root,
            held: Mutex::new(BTreeMap::new()),
        }
    }

    /// Where the blobs live.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the blob named `digest` is kept.
    #[must_use]
    pub fn path_of(&self, digest: &str) -> PathBuf {
        self.root.join(&digest[..2]).join(digest)
    }

    /// Writes `bytes` once and returns their digest.
    ///
    /// A blob already present is read and compared rather than trusted: a file under a digest
    /// that holds other bytes is corruption, and is refused instead of being overwritten.
    ///
    /// # Errors
    /// The directory cannot be written, or holds different bytes under the digest.
    pub fn put(&self, bytes: &[u8]) -> Result<String, String> {
        let digest = digest_of(bytes);
        if self
            .held
            .lock()
            .expect("content cache")
            .contains_key(&digest)
        {
            return Ok(digest);
        }
        let path = self.path_of(&digest);
        match std::fs::read(&path) {
            Ok(held) if held == bytes => {}
            Ok(_) => {
                return Err(format!(
                    "content blob {} holds bytes other than its digest names",
                    path.display()
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let directory = path.parent().expect("a blob path has a shard");
                std::fs::create_dir_all(directory)
                    .map_err(|error| format!("creating {}: {error}", directory.display()))?;
                // Written beside the blob and renamed over it, so a reader never sees half a blob.
                let partial = directory.join(format!(".{digest}.{}.partial", std::process::id()));
                std::fs::write(&partial, bytes)
                    .map_err(|error| format!("writing {}: {error}", partial.display()))?;
                std::fs::rename(&partial, &path)
                    .map_err(|error| format!("publishing {}: {error}", path.display()))?;
            }
            Err(error) => return Err(format!("reading {}: {error}", path.display())),
        }
        self.held
            .lock()
            .expect("content cache")
            .insert(digest.clone(), Arc::new(bytes.to_vec()));
        Ok(digest)
    }

    /// The bytes of the blob named `digest`, checked against it.
    ///
    /// # Errors
    /// The blob is missing, unreadable, or holds bytes of another digest.
    pub fn get(&self, digest: &str) -> Result<Arc<Vec<u8>>, String> {
        if let Some(held) = self.held.lock().expect("content cache").get(digest) {
            return Ok(Arc::clone(held));
        }
        let path = self.path_of(digest);
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("reading content blob {}: {error}", path.display()))?;
        if digest_of(&bytes) != digest {
            return Err(format!(
                "content blob {} holds bytes other than its digest names",
                path.display()
            ));
        }
        let bytes = Arc::new(bytes);
        self.held
            .lock()
            .expect("content cache")
            .insert(digest.to_owned(), Arc::clone(&bytes));
        Ok(bytes)
    }

    /// The fields or arguments of a command as they are recorded: the `document` member as one
    /// JSON blob, and every large string anywhere as a text or byte blob.
    ///
    /// # Errors
    /// A blob cannot be written.
    pub fn store_fields(&self, value: Value) -> Result<Value, String> {
        let Value::Object(map) = value else {
            return self.store_value(value);
        };
        let mut out = Map::new();
        for (key, item) in map {
            let item = if key == DOCUMENT {
                let inner = self.store_value(item)?;
                let bytes = serde_json::to_vec(&inner)
                    .map_err(|error| format!("encoding a document blob: {error}"))?;
                if bytes.len() >= INLINE_LIMIT {
                    Value::String(
                        Reference {
                            form: Form::Json,
                            digest: self.put(&bytes)?,
                        }
                        .wire(),
                    )
                } else {
                    inner
                }
            } else {
                self.store_value(item)?
            };
            out.insert(key, item);
        }
        Ok(Value::Object(out))
    }

    /// `value` with every large string stored as a blob and replaced by its reference.
    ///
    /// # Errors
    /// A blob cannot be written.
    pub fn store_value(&self, value: Value) -> Result<Value, String> {
        Ok(match value {
            Value::String(text) => Value::String(self.store_string(text)?),
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .map(|item| self.store_value(item))
                    .collect::<Result<_, _>>()?,
            ),
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .map(|(key, item)| Ok((key, self.store_value(item)?)))
                    .collect::<Result<_, String>>()?,
            ),
            other => other,
        })
    }

    fn store_string(&self, text: String) -> Result<String, String> {
        if text.starts_with(PREFIX) {
            return Ok(Reference {
                form: Form::Text,
                digest: self.put(text.as_bytes())?,
            }
            .wire());
        }
        if text.len() < INLINE_LIMIT {
            return Ok(text);
        }
        if let Some(bytes) = canonical_hex(&text) {
            if !is_json_lines(&bytes) {
                return Ok(Reference {
                    form: Form::Hex,
                    digest: self.put(&bytes)?,
                }
                .wire());
            }
            let lines = bytes
                .split(|byte| *byte == b'\n')
                .map(|line| self.put(line).map(Value::String))
                .collect::<Result<Vec<_>, _>>()?;
            let manifest = serde_json::to_vec(&Value::Array(lines))
                .map_err(|error| format!("encoding a line manifest: {error}"))?;
            return Ok(Reference {
                form: Form::Lines,
                digest: self.put(&manifest)?,
            }
            .wire());
        }
        Ok(Reference {
            form: Form::Text,
            digest: self.put(text.as_bytes())?,
        }
        .wire())
    }

    /// `value` with every reference replaced by the value it stands for.
    ///
    /// # Errors
    /// A reference is malformed, or names a blob that is missing or does not match its digest.
    pub fn resolve(&self, value: Value) -> Result<Value, String> {
        Ok(match value {
            Value::String(text) => match Reference::parse(&text) {
                None => Value::String(text),
                Some(reference) => self.resolve_reference(&reference?)?,
            },
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .map(|item| self.resolve(item))
                    .collect::<Result<_, _>>()?,
            ),
            Value::Object(map) => Value::Object(self.resolve_map(map)?),
            other => other,
        })
    }

    /// [`Self::resolve`] over an object's members.
    ///
    /// # Errors
    /// As [`Self::resolve`].
    pub fn resolve_map(&self, map: Map<String, Value>) -> Result<Map<String, Value>, String> {
        map.into_iter()
            .map(|(key, item)| Ok((key, self.resolve(item)?)))
            .collect()
    }

    fn resolve_reference(&self, reference: &Reference) -> Result<Value, String> {
        let bytes = self.get(&reference.digest)?;
        match reference.form {
            Form::Text => String::from_utf8(bytes.to_vec())
                .map(Value::String)
                .map_err(|_| format!("text blob {} is not UTF-8", reference.digest)),
            Form::Hex => Ok(Value::String(format!("hex:{}", lower_hex(&bytes)))),
            Form::Lines => {
                let digests: Vec<String> = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("line manifest {}: {error}", reference.digest))?;
                let mut joined = Vec::new();
                for (index, digest) in digests.iter().enumerate() {
                    if !is_digest(digest) {
                        return Err(format!(
                            "line manifest {} names `{digest}`",
                            reference.digest
                        ));
                    }
                    if index > 0 {
                        joined.push(b'\n');
                    }
                    joined.extend_from_slice(&self.get(digest)?);
                }
                Ok(Value::String(format!("hex:{}", lower_hex(&joined))))
            }
            Form::Json => {
                let inner: Value = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("json blob {}: {error}", reference.digest))?;
                self.resolve(inner)
            }
        }
    }

    /// A serializable value with every reference in it resolved.
    ///
    /// # Errors
    /// As [`Self::resolve`], or the value does not survive its own JSON form.
    pub fn resolve_typed<T>(&self, value: &T) -> Result<T, String>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let json = serde_json::to_value(value).map_err(|error| error.to_string())?;
        serde_json::from_value(self.resolve(json)?).map_err(|error| error.to_string())
    }

    /// A serializable value with every large string in it stored.
    ///
    /// # Errors
    /// As [`Self::store_value`], or the value does not survive its own JSON form.
    pub fn store_typed<T>(&self, value: &T) -> Result<T, String>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let json = serde_json::to_value(value).map_err(|error| error.to_string())?;
        serde_json::from_value(self.store_value(json)?).map_err(|error| error.to_string())
    }

    /// Every problem with the directory: a file that is not a blob, a blob whose bytes are not
    /// its name's digest, or a left-over partial write. Sorted, so two runs print the same.
    #[must_use]
    pub fn verify(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let Ok(shards) = std::fs::read_dir(&self.root) else {
            return problems;
        };
        for shard in shards.flatten() {
            let shard_path = shard.path();
            let shard_name = shard.file_name().to_string_lossy().into_owned();
            if !shard_path.is_dir() {
                problems.push(format!("{}: not a blob shard", shard_path.display()));
                continue;
            }
            let Ok(files) = std::fs::read_dir(&shard_path) else {
                problems.push(format!("{}: unreadable", shard_path.display()));
                continue;
            };
            for file in files.flatten() {
                let path = file.path();
                let name = file.file_name().to_string_lossy().into_owned();
                if !is_digest(&name) || !name.starts_with(&shard_name) || shard_name.len() != 2 {
                    problems.push(format!("{}: not a content blob", path.display()));
                    continue;
                }
                match std::fs::read(&path) {
                    Ok(bytes) if digest_of(&bytes) == name => {}
                    Ok(_) => problems.push(format!(
                        "{}: holds bytes other than its digest names",
                        path.display()
                    )),
                    Err(error) => problems.push(format!("{}: {error}", path.display())),
                }
            }
        }
        problems.sort();
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store() -> (tempfile::TempDir, ContentStore) {
        let directory = tempfile::tempdir().expect("scratch");
        let store = ContentStore::at(directory.path().join("blobs"));
        (directory, store)
    }

    #[test]
    fn a_body_repeated_in_fields_document_and_events_is_one_blob_named_everywhere() {
        let (_directory, store) = store();
        let body = "x".repeat(4096);
        let fields = json!({
            "body": body,
            "title": "short stays inline",
            "document": {
                "fields": { "body": body },
                "events": [{ "changed": { "body": body }, "args": { "changes": { "body": body } } }]
            }
        });
        let stored = store.store_fields(fields.clone()).expect("stored");
        assert_eq!(stored["title"], "short stays inline");
        let reference = stored["body"].as_str().expect("a reference").to_owned();
        assert!(
            reference.starts_with("aep-blob:text:sha256:"),
            "{reference}"
        );
        assert!(stored["document"]
            .as_str()
            .expect("the document is one reference")
            .starts_with("aep-blob:json:sha256:"));
        let blobs: Vec<_> = walkdir(store.root());
        assert_eq!(
            blobs.len(),
            2,
            "one body blob and one document blob: {blobs:?}"
        );
        assert_eq!(store.resolve(stored).expect("resolved"), fields);
    }

    #[test]
    fn a_canonical_hex_string_is_stored_as_its_bytes_and_read_back_in_the_same_spelling() {
        let (_directory, store) = store();
        let hexed = format!("hex:{}", lower_hex(&[0xab; 300]));
        let upper = format!("hex:{}", "AB".repeat(300));
        let stored = store.store_value(json!([hexed, upper])).expect("stored");
        assert!(stored[0]
            .as_str()
            .expect("ref")
            .starts_with("aep-blob:hex:"));
        assert!(stored[1]
            .as_str()
            .expect("ref")
            .starts_with("aep-blob:text:"));
        let digest = Reference::parse(stored[0].as_str().expect("ref"))
            .expect("prefixed")
            .expect("valid")
            .digest
            .clone();
        assert_eq!(store.get(&digest).expect("blob").len(), 300);
        assert_eq!(
            store.resolve(stored).expect("resolved"),
            json!([hexed, upper])
        );
    }

    #[test]
    fn a_captured_journal_names_each_line_by_the_blob_its_legacy_record_already_has() {
        let (_directory, store) = store();
        let line = |n: usize| format!("{{\"line\":{n},\"pad\":\"{}\"}}", "p".repeat(300));
        let journal = format!("{}\n{}\n", line(1), line(2));
        let as_hex = |text: &str| format!("hex:{}", lower_hex(text.as_bytes()));
        let stored = store
            .store_value(json!([as_hex(&journal), as_hex(&line(1))]))
            .expect("stored");
        assert!(stored[0]
            .as_str()
            .expect("ref")
            .starts_with("aep-blob:lines:"));
        assert!(stored[1]
            .as_str()
            .expect("ref")
            .starts_with("aep-blob:hex:"));
        // Two lines, the empty remainder after the final newline, and the manifest: the first
        // line is the record's own blob.
        assert_eq!(walkdir(store.root()).len(), 4);
        assert_eq!(
            store.resolve(stored).expect("resolved"),
            json!([as_hex(&journal), as_hex(&line(1))])
        );
        let markdown = format!("# A captured file\n\n{}\n", "text ".repeat(80));
        let stored = store.store_value(json!(as_hex(&markdown))).expect("stored");
        assert!(
            stored.as_str().expect("ref").starts_with("aep-blob:hex:"),
            "a file that is not JSON lines is one blob: {stored}"
        );
    }

    #[test]
    fn a_short_string_that_looks_like_a_reference_is_stored_so_it_reads_back_as_itself() {
        let (_directory, store) = store();
        let lookalike = format!("{PREFIX}text:sha256:{}", "0".repeat(64));
        let stored = store.store_value(json!(lookalike)).expect("stored");
        assert_ne!(stored, json!(lookalike));
        assert_eq!(store.resolve(stored).expect("resolved"), json!(lookalike));
    }

    #[test]
    fn a_blob_whose_bytes_do_not_match_its_name_is_refused_on_read_and_reported_by_verify() {
        let (_directory, store) = store();
        let stored = store.store_value(json!("y".repeat(512))).expect("stored");
        let reference = Reference::parse(stored.as_str().expect("ref"))
            .expect("prefixed")
            .expect("valid");
        std::fs::write(store.path_of(reference.digest()), b"tampered").expect("tamper");
        let fresh = ContentStore::at(store.root().to_owned());
        let error = fresh.resolve(stored).expect_err("refused");
        assert!(error.contains("other than its digest"), "{error}");
        assert_eq!(fresh.verify().len(), 1);
    }

    #[test]
    fn a_malformed_reference_is_refused_rather_than_read_as_text() {
        let (_directory, store) = store();
        let error = store
            .resolve(json!(format!("{PREFIX}text:sha256:nothex")))
            .expect_err("refused");
        assert!(error.contains("not a content-blob reference"), "{error}");
    }

    fn walkdir(root: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for shard in std::fs::read_dir(root).expect("root").flatten() {
            for file in std::fs::read_dir(shard.path()).expect("shard").flatten() {
                found.push(file.path());
            }
        }
        found
    }
}
