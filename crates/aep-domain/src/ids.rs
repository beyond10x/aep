//! Identifier newtypes.
//!
//! Every identifier in AEP is a validated newtype rather than a bare [`String`], so a
//! principle id cannot be passed where a state id is expected, and malformed ids are
//! rejected at the parser boundary.
//!
//! Four charset rules are used:
//!
//! | rule | shape | example |
//! |---|---|---|
//! | kebab | `[a-z][a-z0-9]*(-[a-z0-9]+)*` | `test-driven` |
//! | dotted | kebab segments separated by `.` or `/` | `development.standard`, `adp/default` |
//! | dotted-snake | as dotted, with `_` allowed inside a segment | `adversarial_verify` |
//! | loose | alphanumeric segments separated by `.`, `-`, `_` or `/`, upper case allowed | `AUTH-142` |
//!
//! A trailing segment that is a bare integer is rejected for dotted ids, which keeps the
//! `<id>/<major>` reference syntax unambiguous (see [`crate::version`]).
//!
//! Each rule is written twice — once as the crate-private `validate`, which the constructor
//! runs, and once as a
//! JSON Schema `pattern`, which an editor runs — and the two spellings must agree on every input
//! or a document is valid to one and invalid to the other. [`identifier_pattern!`](crate::identifier_pattern) is the second
//! spelling, in one place per rule, and every identifier and reference in the workspace composes
//! its published pattern from it.
//!
//! The length bound is the part of `validate` a `pattern` cannot carry: bounding
//! `[a-z][a-z0-9]*(-[a-z0-9]+)*` at [`MAX_LENGTH`] characters is a constraint on the sum of its
//! parts, and a regular expression has no way to write one. It is published in the keyword that
//! can, `maxLength`, beside every pattern here — without it the schema calls a 201-character
//! identifier valid and the loader refuses it, which is the same defect one keyword along.

use std::fmt;

use crate::error::ParseError;

/// The longest an identifier may be.
///
/// Published as `maxLength` beside every pattern these rules generate, because a `pattern` cannot
/// express a length bound and `validate` does: without it the schema calls a 201-character
/// identifier valid and the loader refuses it, which is the same one-rule-two-definitions defect
/// [`identifier_pattern!`](crate::identifier_pattern) exists to remove, in a second keyword.
///
/// `validate` counts bytes and JSON Schema counts characters. Every charset here is ASCII-only, so
/// a string the pattern accepts has one byte per character and the two counts are the same number.
pub const MAX_LENGTH: u32 = 200;

/// Charset rule applied to an identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Charset {
    /// Lower-case kebab-case: `test-driven`.
    Kebab,
    /// Lower-case kebab-case segments separated by `.` or `/`: `development.standard`.
    Dotted,
    /// As [`Charset::Dotted`], additionally allowing `_` inside a segment: `adversarial_verify`.
    DottedSnake,
    /// Mixed-case alphanumeric segments separated by `.`, `/`, `-` or `_`: `AUTH-142`.
    Loose,
}

impl Charset {
    /// Characters that separate segments, for charset validation.
    fn separators(self) -> &'static [char] {
        match self {
            Self::Kebab => &['-'],
            Self::Dotted => &['.', '/', '-'],
            Self::DottedSnake | Self::Loose => &['.', '/', '-', '_'],
        }
    }

    /// Characters that separate namespace components, for the numeric-tail rule.
    fn path_separators(self) -> &'static [char] {
        match self {
            Self::Kebab | Self::Loose => &[],
            Self::Dotted | Self::DottedSnake => &['.', '/'],
        }
    }

    fn allows_upper(self) -> bool {
        self == Self::Loose
    }
}

/// Validates `value` against `charset`, returning a human-readable reason on failure.
fn validate(value: &str, charset: Charset, kind: &'static str) -> Result<(), ParseError> {
    let reject = |reason: String| Err(ParseError::identifier(kind, value, reason));

    if value.is_empty() {
        return reject("must not be empty".to_owned());
    }
    if value.len() > MAX_LENGTH as usize {
        return reject(format!(
            "must be at most {MAX_LENGTH} characters, got {}",
            value.len()
        ));
    }

    let separators = charset.separators();
    let segments: Vec<&str> = value.split(|c| separators.contains(&c)).collect();

    for segment in &segments {
        if segment.is_empty() {
            return reject(format!(
                "has an empty segment; separators ({}) must not lead, trail or repeat",
                separators.iter().collect::<String>()
            ));
        }
        for ch in segment.chars() {
            let ok = ch.is_ascii_lowercase()
                || ch.is_ascii_digit()
                || (charset.allows_upper() && ch.is_ascii_uppercase());
            if !ok {
                return reject(format!("contains disallowed character {ch:?}"));
            }
        }
    }

    let first = value.chars().next().unwrap_or('_');
    if !(first.is_ascii_lowercase() || (charset.allows_upper() && first.is_ascii_alphanumeric())) {
        return reject(format!("must start with a letter, got {first:?}"));
    }

    let path_separators = charset.path_separators();
    if !path_separators.is_empty() {
        let components: Vec<&str> = value.split(|c| path_separators.contains(&c)).collect();
        if let Some(last) = components.last() {
            if last.chars().all(|c| c.is_ascii_digit()) {
                return reject(format!(
                    "must not end in a numeric segment ({last:?}); that form is reserved for \
                     version references such as `{value}/1`"
                ));
            }
        }
    }

    Ok(())
}

/// The JSON Schema pattern for one charset rule, wrapped in `$prefix` and `$suffix`.
///
/// # Why this is a macro and not four `const`s
///
/// A published pattern is the *second* definition of a rule whose first definition is the
/// crate-private `validate`, and the two have drifted every time they were written apart. `WorkflowId`'s
/// pattern accepted `adp/2`, which `WorkflowId::new` refuses, so
/// `schemas/generated/driver-steps.schema.json` called 183 step-map pins valid that the loader
/// would not load; before that, `PinnedWorkflowRef` paraphrased the same rule with `-` inside the
/// character class and accepted `adp-/1`. Both were found by evaluating the constant against the
/// constructor, and both are the same defect: one identifier, two definitions.
///
/// So there is one definition per charset, here, and every identifier and every reference in the
/// workspace composes its pattern from it — including `aep-driver-spec`'s `StepMapId` and
/// `PinnedWorkflowRef`, which is why this is exported. `concat!` is what makes composition
/// possible in a `const`:
/// stable Rust cannot concatenate two `&'static str` constants, so the pieces have to still be
/// literals when they meet.
///
/// `crates/aep-driver-spec/tests/published_pattern_evaluated.rs` evaluates every published pattern
/// against its own constructor over a corpus, which is what keeps this honest — a pattern that is
/// merely *shared* is still only as good as the one place it is written.
///
/// ```
/// use aep_domain::identifier_pattern;
/// use aep_domain::ids::WorkflowId;
///
/// assert_eq!(identifier_pattern!(Dotted, "^", "$"), WorkflowId::PATTERN);
/// ```
#[macro_export]
macro_rules! identifier_pattern {
    // Lower-case kebab: `test-driven`. Hyphens separate non-empty segments; they do not lead,
    // trail or repeat, which is why `-` is between the groups and not inside the class.
    (Kebab, $prefix:literal, $suffix:literal) => {
        concat!($prefix, "[a-z][a-z0-9]*(-[a-z0-9]+)*", $suffix)
    };
    // Kebab segments joined by `.` or `/`: `adp/default`. The last `.`/`/` component may not be a
    // bare integer, because `<id>/<major>` is a version reference — so the tail is spelled as an
    // alternation of *has a letter in its first hyphen segment* and *is digits followed by at
    // least one more hyphen segment*, which is `validate`'s rule and not an approximation of it.
    (Dotted, $prefix:literal, $suffix:literal) => {
        concat!(
            $prefix,
            "[a-z][a-z0-9]*(-[a-z0-9]+)*",
            "(([./][a-z0-9]+(-[a-z0-9]+)*)*",
            "[./]([a-z0-9]*[a-z][a-z0-9]*(-[a-z0-9]+)*|[0-9]+(-[a-z0-9]+)+))?",
            $suffix
        )
    };
    // As `Dotted`, with `_` allowed beside `-` inside a component: `adversarial_verify`.
    (DottedSnake, $prefix:literal, $suffix:literal) => {
        concat!(
            $prefix,
            "[a-z][a-z0-9]*([-_][a-z0-9]+)*",
            "(([./][a-z0-9]+([-_][a-z0-9]+)*)*",
            "[./]([a-z0-9]*[a-z][a-z0-9]*([-_][a-z0-9]+)*|[0-9]+([-_][a-z0-9]+)+))?",
            $suffix
        )
    };
    // Mixed-case alphanumeric segments joined by `.`, `/`, `-` or `_`: `AUTH-142`. No numeric-tail
    // rule, because a loose id is not a namespace path and carries no version reference syntax.
    (Loose, $prefix:literal, $suffix:literal) => {
        concat!(
            $prefix,
            "[A-Za-z0-9]([A-Za-z0-9]|[./_-][A-Za-z0-9])*",
            $suffix
        )
    };
}

/// Declares an identifier newtype with a charset rule, whose JSON Schema pattern the
/// charset decides.
macro_rules! identifier {
    ($(#[$meta:meta])* $name:ident, $charset:ident, $kind:literal) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Parses and validates an identifier.
            pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
                let value = value.into();
                validate(&value, Charset::$charset, $kind)?;
                Ok(Self(value))
            }

            /// The identifier as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// The regular expression this identifier is validated against.
            ///
            /// Published in generated JSON Schema so that non-Rust consumers can apply the
            /// same rule.
            ///
            /// Derived from the charset by
            /// [`identifier_pattern!`](crate::identifier_pattern), never written beside it: the
            /// pattern and [`Self::new`] are two definitions of one rule, and they drift.
            pub const PATTERN: &'static str = $crate::identifier_pattern!($charset, "^", "$");
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:?})", stringify!($name), self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = ParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = ParseError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(d)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }

        impl schemars::JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).to_owned()
            }

            fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                let mut schema = schemars::schema::SchemaObject {
                    instance_type: Some(schemars::schema::InstanceType::String.into()),
                    ..Default::default()
                };
                schema.string().pattern = Some(Self::PATTERN.to_owned());
                schema.string().max_length = Some($crate::ids::MAX_LENGTH);
                schema.metadata().description = Some(format!("{} identifier.", $kind));
                schema.into()
            }
        }
    };
}

identifier!(
    /// Identifier of a protocol, such as `aep`, `adp` or `aop`.
    ProtocolId,
    Kebab,
    "protocol"
);

identifier!(
    /// Identifier of a principle, such as `test-driven`.
    PrincipleId,
    Kebab,
    "principle"
);

identifier!(
    /// Identifier of a profile, such as `development.standard`.
    ProfileId,
    Dotted,
    "profile"
);

identifier!(
    /// Identifier of a workflow, such as `adp/default`.
    WorkflowId,
    Dotted,
    "workflow"
);

identifier!(
    /// Identifier of a workflow state, such as `implement`.
    StateId,
    DottedSnake,
    "state"
);

identifier!(
    /// Identifier of a workflow phase, such as `implementation`.
    ///
    /// Phases are the join between principles and workflows: a principle's obligations are
    /// timed against phases (`before_implementation`), and states declare which phases they
    /// belong to.
    PhaseId,
    Kebab,
    "phase"
);

identifier!(
    /// Identifier of an obligation within a principle.
    ObligationId,
    DottedSnake,
    "obligation"
);

identifier!(
    /// Identifier of an approval, such as `security-review`.
    ApprovalId,
    Kebab,
    "approval"
);

identifier!(
    /// Identifier of a verified claim, such as `recovery` in `recovery_verified`.
    ClaimId,
    Kebab,
    "claim"
);

identifier!(
    /// Reference to an external tool, such as `cargo-nextest`.
    ToolRef,
    Dotted,
    "tool"
);

identifier!(
    /// Identifier of a service, such as `auth-api`.
    ServiceId,
    Dotted,
    "service"
);

identifier!(
    /// Identifier of an external provider holding artifacts, such as `linear` or `github`.
    ProviderId,
    Kebab,
    "provider"
);

identifier!(
    /// Reference to a repository, such as `acme/payments`.
    RepositoryRef,
    Loose,
    "repository"
);

identifier!(
    /// Identifier of one logical command.
    ///
    /// A retry of the same logical command reuses its command id; a new attempt at the transport
    /// level gets a new [`RequestId`]. Keeping the two apart is what lets an audit trail show three
    /// attempts at one intended change rather than three changes.
    CommandId,
    Loose,
    "command"
);

identifier!(
    /// Identifier of one transport attempt.
    RequestId,
    Loose,
    "request"
);

identifier!(
    /// Identifier shared by everything belonging to one activity.
    ///
    /// Correlation answers "what belongs together"; causation answers "what directly caused this".
    /// A design command, the review event it triggers, the protocol decision that follows and the
    /// implementation command after that share one correlation id and form a causation chain.
    CorrelationId,
    Loose,
    "correlation"
);

identifier!(
    /// A client-chosen key that makes a mutation safe to retry.
    IdempotencyKey,
    Loose,
    "idempotency key"
);

identifier!(
    /// Identifier of one audit record.
    AuditId,
    Loose,
    "audit"
);

identifier!(
    /// Identifier of one domain event.
    EventId,
    Loose,
    "event"
);

identifier!(
    /// Identifier of one relation in the entity graph.
    RelationId,
    Loose,
    "relation"
);

identifier!(
    /// Identifier of a task, such as `AUTH-142`.
    TaskId,
    Loose,
    "task"
);

identifier!(
    /// Identifier of a single unit of evidence.
    EvidenceId,
    Loose,
    "evidence"
);

identifier!(
    /// Identifier of a protocol execution.
    ExecutionId,
    Loose,
    "execution"
);

/// What a piece of evidence or an approval is about, written `<kind>:<id>`.
///
/// Examples: `task:AUTH-142`, `service:auth-api`, `suite:unit`, `deployment:rev-4711`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(into = "String")]
pub struct SubjectRef {
    kind: String,
    id: String,
}

impl SubjectRef {
    /// Parses a `<kind>:<id>` subject reference.
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let value: String = value.into();
        let Some((kind, id)) = value.split_once(':') else {
            return Err(ParseError::identifier(
                "subject",
                &value,
                "must be written `<kind>:<id>`, for example `service:auth-api`".to_owned(),
            ));
        };
        validate(kind, Charset::Kebab, "subject kind")?;
        validate(id, Charset::Loose, "subject id")?;
        Ok(Self {
            kind: kind.to_owned(),
            id: id.to_owned(),
        })
    }

    /// Builds a subject reference for a task.
    pub fn task(task: &TaskId) -> Self {
        Self {
            kind: "task".to_owned(),
            id: task.as_str().to_owned(),
        }
    }

    /// The subject kind, such as `service`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// The subject id, such as `auth-api`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The pattern published in generated JSON Schema.
    ///
    /// Composed from the two charsets [`Self::new`] validates the halves against, rather than
    /// paraphrased: the paraphrase it replaces took `[a-z][a-z0-9-]*` for the kind, so it accepted
    /// `a-:x` and `a--b:x` that this constructor refuses, and `[A-Za-z0-9._/-]*` for the id, so it
    /// refused `task:AUTH_142` that this constructor takes.
    pub const PATTERN: &'static str = concat!(
        identifier_pattern!(Kebab, "^", ":"),
        identifier_pattern!(Loose, "", "$")
    );
}

impl fmt::Display for SubjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.id)
    }
}

impl fmt::Debug for SubjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubjectRef({}:{})", self.kind, self.id)
    }
}

impl std::str::FromStr for SubjectRef {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl From<SubjectRef> for String {
    fn from(value: SubjectRef) -> Self {
        value.to_string()
    }
}

impl<'de> serde::Deserialize<'de> for SubjectRef {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for SubjectRef {
    fn schema_name() -> String {
        "SubjectRef".to_owned()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            ..Default::default()
        };
        schema.string().pattern = Some(Self::PATTERN.to_owned());
        // A kind, a `:` and an id, each half bounded by `validate`. The bound `Self::new` really
        // applies is *per half*, and JSON Schema has no keyword for that: `maxLength` bounds the
        // whole string, so `a:` followed by 201 characters stays inside 401 and is refused by the
        // constructor. That residue is pinned by
        // `crates/aep-driver-spec/tests/published_pattern_evaluated.rs`, which names it rather than
        // leaving it to be rediscovered.
        schema.string().max_length = Some(2 * MAX_LENGTH + 1);
        schema.metadata().description =
            Some("Subject of evidence, written `<kind>:<id>`.".to_owned());
        schema.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_well_formed_identifiers() {
        assert!(PrincipleId::new("test-driven").is_ok());
        assert!(ProfileId::new("development.standard").is_ok());
        assert!(WorkflowId::new("adp/default").is_ok());
        assert!(StateId::new("adversarial_verify").is_ok());
        assert!(TaskId::new("AUTH-142").is_ok());
    }

    #[test]
    fn rejects_malformed_identifiers() {
        assert!(PrincipleId::new("Test-Driven").is_err(), "upper case");
        assert!(
            PrincipleId::new("test--driven").is_err(),
            "repeated separator"
        );
        assert!(PrincipleId::new("-test").is_err(), "leading separator");
        assert!(PrincipleId::new("test.driven").is_err(), "dot in kebab id");
        assert!(PrincipleId::new("").is_err(), "empty");
        assert!(TaskId::new("AUTH 142").is_err(), "space");
    }

    #[test]
    fn rejects_numeric_tail_on_dotted_ids_to_keep_version_refs_unambiguous() {
        let err = WorkflowId::new("incident-standard/2").expect_err("numeric tail");
        assert!(err.to_string().contains("version references"), "{err}");
        assert!(WorkflowId::new("adp/default").is_ok());
    }

    #[test]
    fn subject_refs_round_trip() {
        let subject: SubjectRef = "service:auth-api".parse().expect("parses");
        assert_eq!(subject.kind(), "service");
        assert_eq!(subject.id(), "auth-api");
        assert_eq!(subject.to_string(), "service:auth-api");
        assert!(SubjectRef::new("auth-api").is_err(), "missing kind");
    }
}
