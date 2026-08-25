//! CLI shell for project-owned JSON Schema contracts.
//!
//! The project file names one registry directory. Every authored schema in that directory is
//! indexed by its own stable `$id`; paths locate contracts but never identify them. Validation and
//! TypeScript generation are both projections from those same JSON files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use schema_contract::validate::{self, JsonDocument};
use serde_json::{json, Value};

use crate::Format;

/// Operations supported by `protocol schema`.
#[derive(Debug, Subcommand)]
pub(crate) enum SchemaCommand {
    /// Validate JSON instances against the schemas selected by their `schema` property.
    Validate(ValidateArgs),
    /// Project a schema's structural types into a deterministic TypeScript module.
    Typescript(TypeScriptArgs),
    /// Print one of engineering-protocols' generated schemas by its historical short name.
    #[command(external_subcommand)]
    BuiltIn(Vec<String>),
}

/// Inputs for project contract validation.
#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// JSON files or directories to validate. Directories are searched recursively.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
    /// Override the registry declared by `.engineering/project.yaml`.
    #[arg(long, value_name = "DIR")]
    schemas: Option<PathBuf>,
    /// How to render the report.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

/// Inputs for deterministic TypeScript projection.
#[derive(Debug, Args)]
pub(crate) struct TypeScriptArgs {
    /// The schema's exact `$id`, not its filename.
    schema_id: String,
    /// The exported root type name.
    #[arg(long)]
    root: String,
    /// Write the generated module here. Omit to print it.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Refuse when `--out` differs from the generated module, without writing it.
    #[arg(long, requires = "out")]
    check: bool,
    /// Override the registry declared by `.engineering/project.yaml`.
    #[arg(long, value_name = "DIR")]
    schemas: Option<PathBuf>,
}

/// Runs one schema operation, or lists the built-in protocol document schemas.
pub(crate) fn run(command: Option<SchemaCommand>) -> Result<ExitCode> {
    match command {
        None => built_in(None),
        Some(SchemaCommand::BuiltIn(arguments)) => {
            if arguments.len() != 1 {
                bail!("a built-in schema expects exactly one name");
            }
            built_in(arguments.first().map(String::as_str))
        }
        Some(SchemaCommand::Validate(args)) => validate_instances(&args),
        Some(SchemaCommand::Typescript(args)) => typescript(&args),
    }
}

fn built_in(name: Option<&str>) -> Result<ExitCode> {
    let schemas = aep_schema::generated_schemas();
    match name {
        None => {
            for entry in schemas {
                outln!("{:<24} {}", entry.filename, entry.describes);
            }
        }
        Some(name) => {
            let wanted = format!("{name}.schema.json");
            let entry = schemas
                .into_iter()
                .find(|entry| entry.filename == wanted || entry.name == name)
                .with_context(|| format!("no schema is called `{name}`"))?;
            out!("{}", entry.to_json().context("serialising the schema")?);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn validate_instances(args: &ValidateArgs) -> Result<ExitCode> {
    let registry = schema_directory(args.schemas.as_deref())?;
    let schema_paths = json_files(&[registry], true)?;
    if schema_paths.is_empty() {
        bail!("the project schema registry contains no `*.schema.json` files");
    }
    let instance_paths = json_files(&args.paths, false)?;
    if instance_paths.is_empty() {
        bail!("the supplied paths contain no JSON instance files");
    }

    let schemas = read_json(&schema_paths)?;
    let instances = read_json(&instance_paths)?;
    let schema_documents = documents(&schema_paths, &schemas);
    let instance_documents = documents(&instance_paths, &instances);
    let report = validate::validate(&schema_documents, &instance_documents);

    match args.format {
        Format::Text => {
            for issue in &report.issues {
                outln!("{issue}");
            }
            if report.is_valid() {
                outln!(
                    "{} schema(s), {} instance(s): valid",
                    report.schema_count,
                    report.valid.len()
                );
            } else {
                outln!(
                    "{} schema(s), {} valid instance(s), {} issue(s): invalid",
                    report.schema_count,
                    report.valid.len(),
                    report.issues.len()
                );
            }
        }
        Format::Json | Format::Yaml => {
            let value = json!({
                "schema_count": report.schema_count,
                "valid": report.valid.iter().map(|valid| json!({
                    "instance": valid.instance,
                    "schema_id": valid.schema_id,
                    "schema": valid.schema,
                })).collect::<Vec<_>>(),
                "issues": report.issues.iter().map(|issue| json!({
                    "code": issue.code.as_str(),
                    "document": issue.document,
                    "instance_path": issue.instance_path,
                    "message": issue.message,
                })).collect::<Vec<_>>(),
            });
            crate::print_serialised(&value, args.format)?;
        }
    }

    Ok(if report.is_valid() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn typescript(args: &TypeScriptArgs) -> Result<ExitCode> {
    let registry = schema_directory(args.schemas.as_deref())?;
    let paths = json_files(&[registry], true)?;
    let values = read_json(&paths)?;
    let matching = paths
        .iter()
        .zip(&values)
        .filter(|(_, schema)| schema.get("$id").and_then(Value::as_str) == Some(&args.schema_id))
        .collect::<Vec<_>>();
    let [(source, schema)] = matching.as_slice() else {
        bail!(
            "expected exactly one schema with `$id: {}`, found {}",
            args.schema_id,
            matching.len()
        );
    };
    let generated = schema_contract::typescript::project(schema, &args.root)
        .with_context(|| format!("projecting {}", source.display()))?;

    let Some(output) = &args.out else {
        out!("{generated}");
        return Ok(ExitCode::SUCCESS);
    };
    if args.check {
        let existing = fs::read_to_string(output)
            .with_context(|| format!("reading generated projection {}", output.display()))?;
        if existing == generated {
            outln!("{}: current", output.display());
            return Ok(ExitCode::SUCCESS);
        }
        outln!("{}: stale", output.display());
        return Ok(ExitCode::from(1));
    }

    write_projection(output, &generated)?;
    outln!("wrote {} from {}", output.display(), source.display());
    Ok(ExitCode::SUCCESS)
}

fn schema_directory(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    let here = std::env::current_dir().context("reading the working directory")?;
    let directory = aep_engine::project::project_directory();
    let project = aep_engine::project::discover(&here).with_context(|| {
        format!(
            "no `--schemas` was given and no `{directory}/project.yaml` was found in {} or any parent",
            here.display()
        )
    })?;
    aep_engine::project::load_local_paths(&project)
        .map(|paths| project.join(directory).join(paths.schemas))
        .with_context(|| {
            format!(
                "reading the schema registry from {}/project.yaml",
                project.join(directory).display()
            )
        })
}

fn json_files(inputs: &[PathBuf], schemas: bool) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for input in inputs {
        collect_json(input, schemas, &mut paths)?;
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn collect_json(path: &Path, schemas: bool, output: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .with_context(|| format!("reading directory {}", path.display()))?
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            collect_json(&entry.path(), schemas, output)?;
        }
        return Ok(());
    }
    if !path.is_file() {
        bail!("{} is not a file or directory", path.display());
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(());
    };
    let selected = if schemas {
        name.ends_with(".schema.json")
    } else {
        name.strip_suffix(".json").is_some() && !name.ends_with(".schema.json")
    };
    if selected {
        output.push(path.to_path_buf());
    }
    Ok(())
}

fn read_json(paths: &[PathBuf]) -> Result<Vec<Value>> {
    paths
        .iter()
        .map(|path| {
            let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
        })
        .collect()
}

fn documents<'a>(paths: &'a [PathBuf], values: &'a [Value]) -> Vec<JsonDocument<'a>> {
    paths
        .iter()
        .zip(values)
        .map(|(path, value)| JsonDocument::new(path.to_str().unwrap_or("<non-UTF-8 path>"), value))
        .collect()
}

fn write_projection(path: &Path, contents: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("creating projection directory {}", parent.display()))?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("the projection output must have a UTF-8 filename")?;
    let temporary = parent.join(format!(".{filename}.protocol-schema.tmp"));
    fs::write(&temporary, contents)
        .with_context(|| format!("writing temporary projection {}", temporary.display()))?;
    fs::rename(&temporary, path).with_context(|| {
        format!(
            "installing generated projection {} from {}",
            path.display(),
            temporary.display()
        )
    })
}
