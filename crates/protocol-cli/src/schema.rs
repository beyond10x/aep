//! CLI shell for AEP's generated JSON Schemas.

use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::Subcommand;

/// Operations supported by `protocol schema`.
#[derive(Debug, Subcommand)]
pub(crate) enum SchemaCommand {
    /// Print one of AEP's generated schemas by its short name.
    #[command(external_subcommand)]
    BuiltIn(Vec<String>),
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
