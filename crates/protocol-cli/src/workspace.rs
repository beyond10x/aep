//! CLI shell for the repositories one command answers across.
//!
//! `.engineering/workspace.yaml` names members; this module reads it, resolves each member's tree,
//! and reports what it found. It resolves and reports — nothing here decides anything, and nothing
//! here writes.
//!
//! # A member that is not there is a fact, not a failure
//!
//! `members` exits `0` with an unresolved member listed as unresolved. A workspace is read on
//! machines that have checked out different subsets of it, and a command that failed because a
//! colleague's repository is missing from your disk would be a command nobody could use.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};

use aep_backend_markdown::assembly::Assembly;
use aep_domain::project::ProtocolSource;
use aep_domain::workspace::{Member, Resolution, WorkspaceRef};
use aep_engine::project::{load_workspace, project_directory, resolve_member};

use crate::Format;

/// Operations supported by `protocol workspace`.
#[derive(Debug, Subcommand)]
pub(crate) enum WorkspaceCommand {
    /// The plan across every member, one line per artifact.
    List {
        /// The repository holding the workspace file. The current directory when absent.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Only artifacts of this kind.
        #[arg(long)]
        kind: Option<String>,
        /// Only artifacts at this status.
        #[arg(long)]
        status: Option<String>,
        /// Only this member.
        #[arg(long)]
        member: Option<String>,
        /// How to render the report.
        #[arg(long, default_value = "text")]
        format: Format,
    },
    /// Every relation that crosses a member boundary, and whether its target is there.
    Crossings {
        /// The repository holding the workspace file. The current directory when absent.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Exit 1 if any crossing relation does not resolve.
        #[arg(long)]
        strict: bool,
        /// How to render the report.
        #[arg(long, default_value = "text")]
        format: Format,
    },
    /// Where one reference points, and what to type when more than one member holds it.
    Show {
        /// `kind:name`, or `member/kind:name` to say which member.
        reference: String,
        /// The repository holding the workspace file. The current directory when absent.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// How to render the report.
        #[arg(long, default_value = "text")]
        format: Format,
    },
    /// List the members, where each one resolves to, and whether its store is there.
    Members {
        /// The repository holding the workspace file. The current directory when absent.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// How to render the report.
        #[arg(long, default_value = "text")]
        format: Format,
        /// Materialize a pinned Git member instead of reporting it unresolved.
        ///
        /// Off by default: fetching is the one thing in this verb that reaches a network, and a
        /// read-only report should not do that because somebody typed `members`.
        #[arg(long)]
        fetch: bool,
    },
}

/// Runs one `protocol workspace` subcommand.
pub(crate) fn run(command: WorkspaceCommand) -> Result<ExitCode> {
    match command {
        WorkspaceCommand::Members {
            root,
            format,
            fetch,
        } => members(&root, format, fetch),
        WorkspaceCommand::List {
            root,
            kind,
            status,
            member,
            format,
        } => list(
            &root,
            kind.as_deref(),
            status.as_deref(),
            member.as_deref(),
            format,
        ),
        WorkspaceCommand::Crossings {
            root,
            strict,
            format,
        } => crossings(&root, strict, format),
        WorkspaceCommand::Show {
            reference,
            root,
            format,
        } => show(&root, &reference, format),
    }
}

/// Reads every member's store into one graph.
fn assemble(root: &Path) -> Result<(Assembly, Vec<String>)> {
    let workspace = load_workspace(root)
        .map_err(|errors| anyhow::anyhow!("{errors}"))
        .with_context(|| format!("reading the workspace in {}", root.display()))?;
    let workspace = workspace.ok_or_else(|| {
        anyhow::anyhow!(
            "{} has no workspace.yaml, so there is no set of repositories to answer across",
            root.join(project_directory()).display()
        )
    })?;

    let engineering = root.join(project_directory());
    let mut roots = Vec::new();
    let mut unresolved = Vec::new();
    for member in &workspace.members {
        match resolve(member, &engineering, false) {
            Resolved {
                tree: Some(tree), ..
            } => roots.push((member.name.clone(), tree.join(&member.store))),
            Resolved { name, detail, .. } => {
                unresolved.push(format!(
                    "{name}: {}",
                    detail.unwrap_or_else(|| "unresolved".to_owned())
                ));
            }
        }
    }

    let assembly = Assembly::read(
        roots
            .iter()
            .map(|(name, path)| (name.clone(), path.as_path())),
    );
    Ok((assembly, unresolved))
}

/// The plan across every member.
fn list(
    root: &Path,
    kind: Option<&str>,
    status: Option<&str>,
    member: Option<&str>,
    format: Format,
) -> Result<ExitCode> {
    let (assembly, unresolved) = assemble(root)?;

    let rows: Vec<Value> = assembly
        .documents()
        .filter(|(holder, id, document)| {
            member.is_none_or(|want| holder.as_str() == want)
                && kind.is_none_or(|want| id.namespace() == want)
                && status.is_none_or(|want| document.document.frontmatter.status.as_str() == want)
        })
        .map(|(holder, id, document)| {
            json!({
                "member": holder.to_string(),
                "id": id.to_string(),
                "reference": format!("{holder}/{id}"),
                "status": document.document.frontmatter.status.as_str(),
                "title": document.document.frontmatter.title,
            })
        })
        .collect();

    match format {
        Format::Text => {
            for row in &rows {
                println!(
                    "{:<52} {:<12} {}",
                    row["reference"].as_str().unwrap_or_default(),
                    row["status"].as_str().unwrap_or_default(),
                    row["title"].as_str().unwrap_or_default()
                );
            }
            println!(
                "{} artifact(s) across {} member(s)",
                rows.len(),
                assembly.members().len()
            );
            report_unresolved(&unresolved);
        }
        Format::Yaml | Format::Json => render(
            format,
            &json!({ "artifacts": rows, "unresolved": unresolved }),
        )?,
    }
    Ok(ExitCode::SUCCESS)
}

/// Every relation that crosses a member boundary.
///
/// Exits `0` by default even when a crossing does not resolve, because a member nobody has checked
/// out holds nothing and that is a normal machine rather than a broken plan. `--strict` is for CI,
/// where every member *is* checked out and an unresolved crossing is a real dangling edge.
fn crossings(root: &Path, strict: bool, format: Format) -> Result<ExitCode> {
    let (assembly, unresolved) = assemble(root)?;
    let crossings = assembly.crossing_relations();
    let unresolved_count = crossings.iter().filter(|c| !c.is_resolved()).count();

    match format {
        Format::Text => {
            for crossing in &crossings {
                println!(
                    "{}/{} {} {}  [{}]",
                    crossing.from_member,
                    crossing.from,
                    crossing.kind,
                    crossing.to,
                    crossing.resolution
                );
            }
            println!(
                "{} crossing relation(s), {unresolved_count} unresolved",
                crossings.len()
            );
            report_unresolved(&unresolved);
        }
        Format::Yaml | Format::Json => render(
            format,
            &json!({
                "crossings": crossings.iter().map(|c| json!({
                    "from": format!("{}/{}", c.from_member, c.from),
                    "relation": c.kind,
                    "to": c.to.to_string(),
                    "resolution": c.resolution.to_string(),
                    "resolved": c.is_resolved(),
                })).collect::<Vec<_>>(),
                "unresolved": unresolved_count,
                "members_not_read": unresolved,
            }),
        )?,
    }

    Ok(if strict && unresolved_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Where one reference points.
///
/// Exit `1` on an ambiguous or absent reference: unlike `members`, this one was asked a question
/// with a single right answer, and reporting success without giving one would let a script carry
/// on as though it had.
fn show(root: &Path, reference: &str, format: Format) -> Result<ExitCode> {
    let parsed = WorkspaceRef::parse(reference)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("reading the reference `{reference}`"))?;
    let (assembly, _) = assemble(root)?;
    let resolution = assembly.resolve(&parsed);

    let (code, payload) = match &resolution {
        Resolution::Unique(member) => {
            let (_, document) = assembly
                .get(&parsed)
                .expect("a unique resolution has a document");
            (
                ExitCode::SUCCESS,
                json!({
                    "reference": format!("{member}/{}", parsed.artifact),
                    "member": member.to_string(),
                    "status": document.document.frontmatter.status.as_str(),
                    "title": document.document.frontmatter.title,
                    "path": document.relative_path,
                }),
            )
        }
        Resolution::Ambiguous(members) => (
            ExitCode::FAILURE,
            json!({
                "reference": parsed.to_string(),
                "ambiguous": members.iter().map(ToString::to_string).collect::<Vec<_>>(),
                // The retypeable answer, which is the only part of a refusal that saves anybody time.
                "try": members
                    .iter()
                    .map(|m| format!("{m}/{}", parsed.artifact))
                    .collect::<Vec<_>>(),
            }),
        ),
        Resolution::Absent => (
            ExitCode::FAILURE,
            json!({ "reference": parsed.to_string(), "absent": true }),
        ),
    };

    match format {
        Format::Text => match &resolution {
            Resolution::Unique(member) => println!(
                "{member}/{}  {}  {}",
                parsed.artifact,
                payload["status"].as_str().unwrap_or_default(),
                payload["title"].as_str().unwrap_or_default()
            ),
            Resolution::Ambiguous(members) => {
                println!("{parsed} is held by {} members; say which:", members.len());
                for member in members {
                    println!("  {member}/{}", parsed.artifact);
                }
            }
            Resolution::Absent => println!("{parsed} is held by no member of this workspace"),
        },
        Format::Yaml | Format::Json => render(format, &payload)?,
    }
    Ok(code)
}

/// Names every member the assembly could not read from, so a short answer is not read as a full one.
fn report_unresolved(unresolved: &[String]) {
    if unresolved.is_empty() {
        return;
    }
    println!("{} member(s) not read:", unresolved.len());
    for detail in unresolved {
        println!("  {detail}");
    }
}

/// What resolving one member found.
struct Resolved {
    name: String,
    source: String,
    tree: Option<PathBuf>,
    store: Option<PathBuf>,
    detail: Option<String>,
}

impl Resolved {
    /// `ok` when the store is on disk, `absent` when it is not, `unresolved` when the tree is not.
    fn state(&self) -> &'static str {
        match (&self.tree, &self.store) {
            (Some(_), Some(store)) if store.is_dir() => "ok",
            (Some(_), Some(_)) => "absent",
            _ => "unresolved",
        }
    }
}

/// Reports every member of the workspace this repository declares.
fn members(root: &Path, format: Format, fetch: bool) -> Result<ExitCode> {
    let workspace = load_workspace(root)
        .map_err(|errors| anyhow::anyhow!("{errors}"))
        .with_context(|| format!("reading the workspace in {}", root.display()))?;

    let Some(workspace) = workspace else {
        // Not an error: most repositories answer only for themselves.
        let path = root.join(project_directory()).join("workspace.yaml");
        match format {
            Format::Text => println!("no workspace: {} does not exist", path.display()),
            Format::Yaml | Format::Json => {
                render(format, &json!({ "workspace": Value::Null, "members": [] }))?;
            }
        }
        return Ok(ExitCode::SUCCESS);
    };

    let engineering = root.join(project_directory());
    let resolved: Vec<Resolved> = workspace
        .members
        .iter()
        .map(|member| resolve(member, &engineering, fetch))
        .collect();

    match format {
        Format::Text => {
            println!("{} in {}", workspace, root.display());
            for member in &resolved {
                println!(
                    "  {:<24} {:<11} {}",
                    member.name,
                    member.state(),
                    member
                        .store
                        .as_ref()
                        .map_or_else(|| member.source.clone(), |p| p.display().to_string())
                );
                if let Some(detail) = &member.detail {
                    println!("    {detail}");
                }
            }
        }
        Format::Yaml | Format::Json => render(
            format,
            &json!({
                "workspace": { "members": workspace.members.len() },
                "members": resolved.iter().map(|m| json!({
                    "name": m.name,
                    "source": m.source,
                    "state": m.state(),
                    "tree": m.tree.as_ref().map(|p| p.display().to_string()),
                    "store": m.store.as_ref().map(|p| p.display().to_string()),
                    "detail": m.detail,
                })).collect::<Vec<_>>(),
            }),
        )?,
    }

    Ok(ExitCode::SUCCESS)
}

/// Resolves one member's tree, fetching a pinned Git source only when asked to.
fn resolve(member: &Member, engineering: &Path, fetch: bool) -> Resolved {
    let source = member.source.to_string();
    let name = member.name.to_string();

    if matches!(member.source, ProtocolSource::Git(_)) && !fetch {
        return Resolved {
            name,
            source,
            tree: None,
            store: None,
            detail: Some("pinned Git member; pass --fetch to materialize it".to_owned()),
        };
    }

    match resolve_member(member, engineering) {
        Ok(tree) => {
            let store = tidy(&tree.join(&member.store));
            Resolved {
                name,
                source,
                tree: Some(tidy(&tree)),
                store: Some(store),
                detail: None,
            }
        }
        Err(detail) => Resolved {
            name,
            source,
            tree: None,
            store: None,
            detail: Some(detail),
        },
    }
}

/// Removes `.` and resolvable `..` components, for a path a person is going to read.
///
/// Lexical, not canonical: `std::fs::canonicalize` touches the filesystem and fails on a path that
/// is not there, and the paths most worth printing clearly are exactly the ones that are not there.
fn tidy(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir if matches!(out.last(), Some(Component::Normal(_))) => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    let tidied: PathBuf = out.iter().collect();
    // Everything cancelled out, which means "the directory we started from" and not "no path".
    if tidied.as_os_str().is_empty() {
        return PathBuf::from(".");
    }
    tidied
}

/// Prints a report in the requested machine-readable format.
fn render(format: Format, value: &Value) -> Result<()> {
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(value)?),
        Format::Yaml | Format::Text => println!("{}", serde_yaml::to_string(value)?),
    }
    Ok(())
}
