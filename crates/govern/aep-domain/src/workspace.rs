//! What a set of repositories says about itself.
//!
//! A project describes one repository ([`crate::project`]). A **workspace** describes several, so
//! one CLI can answer across all of them: `.engineering/workspace.yaml` names each member and where
//! its planning store is.
//!
//! ```yaml
//! version: aep.workspace/1
//! members:
//!   - name: aep
//!     source: ..
//!   - name: entity-runtime
//!     source: ../../entity-runtime
//!   - name: metaharness
//!     source: git+ssh://git@github.com/beyond10x/metaharness.git#0123456789abcdef0123456789abcdef01234567
//! ```
//!
//! # The consequence for a person
//!
//! A story here that is blocked by a story in another repository can say so, and one `board` shows
//! the work rather than three. Today each store is an island: a dependency that crosses a
//! repository boundary lives in somebody's head, and the first anybody hears of it is when the
//! blocked work is picked up.
//!
//! # Why a member is a source and not a path
//!
//! `source` is the same locator a project file's `protocols:` takes ([`ProtocolSource`]) — a
//! relative path, or a **pinned** `git+ssh://`, `git+https://` or `git+file://` revision. Reusing it
//! is not tidiness: that type already refuses an absolute path, because a path rooted somewhere only
//! one machine has is true on that machine and false in CI, and it already refuses an unpinned git
//! locator, because a tree that can move under you is a dependency whose meaning changes with no
//! commit in your repository.
//!
//! # What this type deliberately does not do
//!
//! It does not read a member, resolve one, or check that a member exists. A workspace is a
//! declaration; whether the tree it names is checked out on this machine is a question for the
//! shell, and **a member nobody has checked out is a normal condition rather than a broken
//! workspace**.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::artifact::ArtifactId;
use crate::error::{ParseError, ValidationCode, ValidationError, ValidationErrors};
use crate::project::ProtocolSource;

/// The file naming the members of a workspace.
pub const WORKSPACE_FILE: &str = "workspace.yaml";
/// The format version this build reads.
pub const WORKSPACE_VERSION: &str = "aep.workspace/1";
/// Where a member's planning store sits inside its tree, unless the member says otherwise.
pub const DEFAULT_MEMBER_STORE: &str = ".engineering/planning";

/// A member's short name, which is also the namespace its artifacts are addressed under.
///
/// Constrained to `[a-z0-9-]`, because it is about to be used as an identifier prefix and a
/// separator that can appear inside a name is a separator that cannot be relied on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct MemberName(String);

impl MemberName {
    /// Parses a member name, refusing anything that cannot be used as a namespace.
    pub fn parse(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        let refuse = |message: &str, hint: &str| {
            Err(ValidationError::new(
                ValidationCode::TypeMismatch,
                "workspace.members[].name",
                message,
            )
            .with_hint(hint))
        };

        if value.is_empty() {
            return refuse(
                "a member name is empty",
                "name the repository, such as `entity-runtime`",
            );
        }
        if !value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return refuse(
                &format!(
                    "`{value}` is not a member name: use lower-case letters, digits and hyphens"
                ),
                "a name becomes an identifier prefix, so it holds only what an identifier holds",
            );
        }
        if value.starts_with('-') || value.ends_with('-') {
            return refuse(
                &format!("`{value}` starts or ends with a hyphen"),
                "a hyphen separates parts of a name; it cannot be one of the parts",
            );
        }
        Ok(Self(value))
    }

    /// The name as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MemberName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One repository in a workspace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Member {
    /// The short name, which is also the namespace its artifacts are addressed under.
    pub name: MemberName,
    /// Where the tree comes from: a relative path, or a pinned Git revision.
    pub source: ProtocolSource,
    /// Where the planning store sits inside that tree.
    pub store: PathBuf,
}

impl Member {
    /// The store's path, given the directory the workspace file was read from.
    ///
    /// Only meaningful for a [`ProtocolSource::Path`] member; a Git member's tree has to be
    /// materialized first, which is the shell's job and not this type's.
    #[must_use]
    pub fn store_under(&self, engineering: &Path) -> Option<PathBuf> {
        match &self.source {
            ProtocolSource::Path(path) if path.is_absolute() => Some(path.join(&self.store)),
            ProtocolSource::Path(path) => Some(engineering.join(path).join(&self.store)),
            ProtocolSource::Git(_) => None,
        }
    }
}

/// The members one CLI answers across.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Workspace {
    /// Every member, in the order the file names them.
    pub members: Vec<Member>,
}

impl Workspace {
    /// The member of this name, if the workspace has one.
    #[must_use]
    pub fn member(&self, name: &str) -> Option<&Member> {
        self.members.iter().find(|m| m.name.as_str() == name)
    }

    /// Every member name, sorted.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.members.iter().map(|m| m.name.as_str()).collect();
        names.sort_unstable();
        names
    }
}

impl fmt::Display for Workspace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} member(s)", self.members.len())
    }
}

/// A workspace document, as parsed.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawWorkspace {
    /// The format version.
    #[serde(default = "default_version")]
    pub version: String,
    /// The repositories this workspace answers across.
    #[serde(default)]
    pub members: Vec<RawMember>,
}

/// One member, as parsed.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawMember {
    /// The short name, which becomes the namespace.
    pub name: String,
    /// A relative tree path, or a pinned `git+ssh://`, `git+https://` or `git+file://` revision.
    pub source: String,
    /// Where the planning store sits inside the tree.
    #[serde(default)]
    pub store: Option<PathBuf>,
}

/// Serde default for the format version.
fn default_version() -> String {
    WORKSPACE_VERSION.to_owned()
}

impl TryFrom<RawWorkspace> for Workspace {
    type Error = ValidationErrors;

    fn try_from(raw: RawWorkspace) -> Result<Self, Self::Error> {
        let mut errors = ValidationErrors::new();

        if raw.version != WORKSPACE_VERSION {
            errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedFormatVersion,
                    "workspace.version",
                    format!(
                        "this build reads `{WORKSPACE_VERSION}`, not `{}`",
                        raw.version
                    ),
                )
                .with_hint("a format version is a promise about the keys below it"),
            );
        }

        if raw.members.is_empty() {
            errors.push(
                ValidationError::new(
                    ValidationCode::EmptyDeclaration,
                    "workspace.members",
                    "a workspace names no members",
                )
                .with_hint("a workspace of nothing answers nothing; name at least one repository"),
            );
        }

        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut members = Vec::with_capacity(raw.members.len());
        for member in raw.members {
            let name = match MemberName::parse(member.name.clone()) {
                Ok(name) => Some(name),
                Err(error) => {
                    errors.push(error);
                    None
                }
            };

            if let Some(name) = &name {
                if !seen.insert(name.as_str().to_owned()) {
                    errors.push(
                        ValidationError::new(
                            ValidationCode::DuplicateDeclaration,
                            "workspace.members[].name",
                            format!("`{name}` is named twice"),
                        )
                        .with_hint(
                            "a name is the namespace its artifacts are addressed under, so two \
                             members cannot share one",
                        ),
                    );
                }
            }

            let source = match ProtocolSource::parse(member.source.clone()) {
                Ok(source) => Some(source),
                Err(error) => {
                    errors.push(error);
                    None
                }
            };

            let store = member
                .store
                .unwrap_or_else(|| PathBuf::from(DEFAULT_MEMBER_STORE));
            let store = match checked_store(&store) {
                Ok(store) => Some(store),
                Err(error) => {
                    errors.push(error);
                    None
                }
            };

            if let (Some(name), Some(source), Some(store)) = (name, source, store) {
                members.push(Member {
                    name,
                    source,
                    store,
                });
            }
        }

        errors.into_result(Workspace { members })
    }
}

/// Refuses a store path that names a place rather than a position inside the member's tree.
fn checked_store(store: &Path) -> Result<PathBuf, ValidationError> {
    let refuse = |message: String, hint: &str| {
        Err(ValidationError::new(
            ValidationCode::TypeMismatch,
            "workspace.members[].store",
            message,
        )
        .with_hint(hint))
    };

    let text = store.to_string_lossy();
    if text.is_empty() {
        return refuse(
            "a member's store path is empty".to_owned(),
            "leave `store` out to take the default, `.engineering/planning`",
        );
    }
    // An absolute store is true on one machine and false in CI, exactly as an absolute source is.
    if store.is_absolute() || text.starts_with('~') || text.contains(':') || text.starts_with('\\')
    {
        return refuse(
            format!("`{text}` is an absolute path"),
            "a store is a position inside the member's tree, not a place on one machine",
        );
    }
    // `..` would let a member's store address a tree the workspace never named, which is a member
    // nobody declared and nobody can pin.
    if store.components().any(|c| c.as_os_str() == "..") {
        return refuse(
            format!("`{text}` climbs out of the member's tree"),
            "name the member itself if you mean another tree; a store cannot reach outside its own",
        );
    }
    Ok(store.to_path_buf())
}

/// An artifact reference that may name the member it lives in.
///
/// Two spellings, and the difference is the whole point:
///
/// | written | means |
/// |---|---|
/// | `story:provider-spi` | *this* member's story — whichever store the reference was written in |
/// | `entity-runtime/story:provider-spi` | that member's story, wherever it is read from |
///
/// # Why unqualified is not the same as ambiguous
///
/// A story in one repository writing `story:passkey-login` means **its own**. That is never
/// ambiguous, however many members hold a story of that name, because the reference was written
/// somewhere and that somewhere is the answer. Ambiguity is a property of a *question asked of the
/// workspace* — `protocol workspace show story:passkey-login` — and [`Resolution::Ambiguous`] is
/// what that gets, listing the members rather than picking one.
///
/// # Why the separator is `/`
///
/// It is the one character an [`ArtifactId`] already permits and nothing has ever used: no shipped
/// kind contains a slash and no artifact in any store here is named with one. Choosing a character
/// already in the alphabet means an existing id keeps its meaning; choosing one already in *use*
/// would have silently re-read somebody's artifact as somebody else's.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceRef {
    /// The member, when the reference names one. `None` means the member it was written in.
    pub member: Option<MemberName>,
    /// The artifact, within whichever member resolves.
    pub artifact: ArtifactId,
}

impl WorkspaceRef {
    /// Parses `kind:name` or `member/kind:name`.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ParseError> {
        let value = value.as_ref();

        // Only a slash *before* the colon separates a member: `story:plans/q3` is one artifact
        // whose name contains a slash, and has been legal since before workspaces existed.
        let colon = value.find(':').unwrap_or(value.len());
        let Some(slash) = value[..colon].find('/') else {
            return Ok(Self {
                member: None,
                artifact: ArtifactId::new(value)?,
            });
        };

        let (member, rest) = value.split_at(slash);
        let rest = &rest[1..];
        if rest[..rest.find(':').unwrap_or(rest.len())].contains('/') {
            return Err(ParseError::identifier(
                "workspace reference",
                value,
                "names more than one member: write `member/kind:name`".to_owned(),
            ));
        }

        let member = MemberName::parse(member)
            .map_err(|error| ParseError::identifier("workspace reference", value, error.message))?;
        Ok(Self {
            member: Some(member),
            artifact: ArtifactId::new(rest)?,
        })
    }

    /// The same reference, read as belonging to `member` when it named none.
    #[must_use]
    pub fn within(&self, member: &MemberName) -> Self {
        Self {
            member: Some(self.member.clone().unwrap_or_else(|| member.clone())),
            artifact: self.artifact.clone(),
        }
    }

    /// Which member holds this reference, given who holds what.
    ///
    /// `holders` is every member holding an artifact of this id. A reference that named a member
    /// answers from that member alone; one that did not is [`Resolution::Ambiguous`] when more than
    /// one member holds it, because picking the nearest would be a guess presented as an answer.
    #[must_use]
    pub fn resolve(&self, holders: &BTreeSet<MemberName>) -> Resolution {
        match &self.member {
            Some(member) if holders.contains(member) => Resolution::Unique(member.clone()),
            Some(_) => Resolution::Absent,
            None => match holders.len() {
                0 => Resolution::Absent,
                1 => Resolution::Unique(holders.iter().next().expect("one holder").clone()),
                _ => Resolution::Ambiguous(holders.iter().cloned().collect()),
            },
        }
    }
}

impl fmt::Display for WorkspaceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.member {
            Some(member) => write!(f, "{member}/{}", self.artifact),
            None => write!(f, "{}", self.artifact),
        }
    }
}

/// What asking the workspace for a reference found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Exactly one member holds it.
    Unique(MemberName),
    /// More than one does, and the reference did not say which.
    Ambiguous(Vec<MemberName>),
    /// No member holds it. Not an error on its own: a member nobody checked out holds nothing.
    Absent,
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unique(member) => write!(f, "{member}"),
            Self::Ambiguous(members) => {
                let names: Vec<&str> = members.iter().map(MemberName::as_str).collect();
                write!(f, "ambiguous across {}", names.join(", "))
            }
            Self::Absent => f.write_str("absent"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(yaml: &str) -> Result<Workspace, ValidationErrors> {
        let raw: RawWorkspace = serde_yaml::from_str(yaml).expect("document parses");
        Workspace::try_from(raw)
    }

    #[test]
    fn a_member_takes_the_default_store_and_keeps_the_order_it_was_written_in() {
        let parsed = workspace(
            r"
members:
  - name: aep
    source: ..
  - name: entity-runtime
    source: ../../entity-runtime
",
        )
        .expect("validates");

        assert_eq!(parsed.members.len(), 2);
        assert_eq!(parsed.members[0].name.as_str(), "aep");
        assert_eq!(parsed.members[1].name.as_str(), "entity-runtime");
        assert_eq!(
            parsed.members[0].store,
            PathBuf::from(DEFAULT_MEMBER_STORE),
            "a member that says nothing about its store takes the conventional one"
        );
    }

    #[test]
    fn a_pinned_git_member_is_accepted_and_an_unpinned_one_is_not() {
        let pinned = workspace(
            r"
members:
  - name: metaharness
    source: git+ssh://git@github.com/beyond10x/metaharness.git#0123456789abcdef0123456789abcdef01234567
",
        )
        .expect("a pinned member validates");
        assert!(matches!(pinned.members[0].source, ProtocolSource::Git(_)));

        let errors = workspace(
            r"
members:
  - name: metaharness
    source: git+ssh://git@github.com/beyond10x/metaharness.git
",
        )
        .expect_err("an unpinned member is refused");
        assert!(errors.contains(ValidationCode::TypeMismatch));
    }

    #[test]
    fn two_members_cannot_share_a_name() {
        // The name is the namespace its artifacts are addressed under. Two members sharing one
        // would make `entity-runtime:story:x` name two different stories, and the resolver would
        // have to pick — which is the guess this whole type exists to refuse.
        let errors = workspace(
            r"
members:
  - name: entity-runtime
    source: ../../entity-runtime
  - name: entity-runtime
    source: ../../entity-runtime-fork
",
        )
        .expect_err("a duplicate name is refused");

        assert!(errors.contains(ValidationCode::DuplicateDeclaration));
    }

    #[test]
    fn a_workspace_with_no_members_is_refused_rather_than_read_as_empty() {
        // An empty workspace reads exactly like a workspace whose members failed to load, and one
        // of those is a mistake somebody wants told about.
        let errors = workspace("members: []").expect_err("an empty workspace is refused");
        assert!(errors.contains(ValidationCode::EmptyDeclaration));
    }

    #[test]
    fn a_name_that_could_not_be_a_namespace_is_refused() {
        for spelling in [
            "Entity-Runtime",
            "entity runtime",
            "entity:runtime",
            "-entity",
            "entity-",
            "",
        ] {
            let error =
                MemberName::parse(spelling).expect_err(&format!("`{spelling}` must be refused"));
            assert_eq!(error.code, ValidationCode::TypeMismatch, "{spelling}");
        }
    }

    #[test]
    fn a_store_that_leaves_the_members_tree_is_refused() {
        // `../other/.engineering/planning` would address a tree the workspace never named — a
        // member nobody declared, nobody pinned and nobody can check the revision of.
        for spelling in ["../elsewhere/planning", "/srv/planning", "~/planning", ""] {
            let errors = workspace(&format!(
                "members:\n  - name: m\n    source: ..\n    store: {spelling:?}\n"
            ))
            .expect_err(&format!("`{spelling}` must be refused"));
            assert!(errors.contains(ValidationCode::TypeMismatch), "{spelling}");
        }
    }

    #[test]
    fn a_relative_member_resolves_its_store_under_the_engineering_directory() {
        let parsed = workspace(
            r"
members:
  - name: entity-runtime
    source: ../../entity-runtime
",
        )
        .expect("validates");

        assert_eq!(
            parsed.members[0].store_under(Path::new("/w/aep/.engineering")),
            Some(PathBuf::from(
                "/w/aep/.engineering/../../entity-runtime/.engineering/planning"
            ))
        );
    }

    #[test]
    fn a_git_member_has_no_store_path_until_something_materializes_it() {
        // Deliberately `None` rather than a guess: this type performs no IO, and a path it invented
        // for a tree nobody has fetched would be a path that does not exist.
        let parsed = workspace(
            r"
members:
  - name: metaharness
    source: git+ssh://git@github.com/beyond10x/metaharness.git#0123456789abcdef0123456789abcdef01234567
",
        )
        .expect("validates");

        assert_eq!(
            parsed.members[0].store_under(Path::new("/w/.engineering")),
            None
        );
    }

    #[test]
    fn a_reference_may_name_a_member_or_leave_it_to_where_it_was_written() {
        let local = WorkspaceRef::parse("story:provider-spi").expect("parses");
        assert_eq!(local.member, None);
        assert_eq!(local.artifact.to_string(), "story:provider-spi");
        assert_eq!(local.to_string(), "story:provider-spi");

        let qualified = WorkspaceRef::parse("entity-runtime/story:provider-spi").expect("parses");
        assert_eq!(
            qualified.member.as_ref().map(MemberName::as_str),
            Some("entity-runtime")
        );
        assert_eq!(qualified.artifact.to_string(), "story:provider-spi");
        assert_eq!(
            qualified.to_string(),
            "entity-runtime/story:provider-spi",
            "a reference must round-trip, or the thing printed in a refusal is not the thing to retype"
        );
    }

    #[test]
    fn a_slash_after_the_colon_is_part_of_the_name_and_not_a_member() {
        // `story:plans/q3` has been a legal artifact id since before workspaces existed. Reading it
        // as a member would silently re-point every reference of that shape at a repository nobody
        // named.
        let parsed = WorkspaceRef::parse("story:plans/q3").expect("parses");
        assert_eq!(parsed.member, None);
        assert_eq!(parsed.artifact.to_string(), "story:plans/q3");
    }

    #[test]
    fn a_reference_naming_two_members_is_refused() {
        WorkspaceRef::parse("a/b/story:x").expect_err("two members is not a reference");
    }

    #[test]
    fn an_unqualified_reference_held_by_two_members_is_ambiguous_rather_than_guessed() {
        // The failure this refuses is the quiet one: picking the nearest match answers the question
        // somebody asked with a fact about a different repository, and nothing in the output says so.
        let reference = WorkspaceRef::parse("story:passkey-login").expect("parses");
        let holders: BTreeSet<MemberName> = ["one", "two"]
            .into_iter()
            .map(|n| MemberName::parse(n).expect("a name"))
            .collect();

        match reference.resolve(&holders) {
            Resolution::Ambiguous(members) => {
                assert_eq!(
                    members.len(),
                    2,
                    "the refusal lists every member, so it can be retyped"
                );
            }
            other => panic!("expected an ambiguous resolution, got {other:?}"),
        }
    }

    #[test]
    fn a_qualified_reference_is_never_ambiguous_however_many_members_hold_the_name() {
        let reference = WorkspaceRef::parse("two/story:passkey-login").expect("parses");
        let holders: BTreeSet<MemberName> = ["one", "two"]
            .into_iter()
            .map(|n| MemberName::parse(n).expect("a name"))
            .collect();

        assert_eq!(
            reference.resolve(&holders),
            Resolution::Unique(MemberName::parse("two").expect("a name"))
        );
    }

    #[test]
    fn a_reference_into_a_member_that_does_not_hold_it_is_absent_rather_than_falling_back() {
        // Falling back to another member would turn a typo into somebody else's artifact.
        let reference = WorkspaceRef::parse("three/story:passkey-login").expect("parses");
        let holders: BTreeSet<MemberName> = ["one", "two"]
            .into_iter()
            .map(|n| MemberName::parse(n).expect("a name"))
            .collect();

        assert_eq!(reference.resolve(&holders), Resolution::Absent);
    }

    #[test]
    fn an_unqualified_reference_takes_the_member_it_was_written_in() {
        let here = MemberName::parse("aep").expect("a name");
        let local = WorkspaceRef::parse("story:x")
            .expect("parses")
            .within(&here);
        assert_eq!(local.to_string(), "aep/story:x");

        // A reference that already named a member is not re-pointed by where it happens to be read.
        let elsewhere = WorkspaceRef::parse("entity-runtime/story:x")
            .expect("parses")
            .within(&here);
        assert_eq!(elsewhere.to_string(), "entity-runtime/story:x");
    }

    #[test]
    fn an_unreadable_format_version_is_refused_by_name() {
        let errors = workspace(
            r"
version: aep.workspace/2
members:
  - name: m
    source: ..
",
        )
        .expect_err("a future version is refused");
        assert!(errors.contains(ValidationCode::UnsupportedFormatVersion));
    }
}
