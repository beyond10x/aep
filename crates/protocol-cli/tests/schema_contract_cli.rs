//! `protocol schema validate|typescript` integration tests.
//!
//! The library owns the contract semantics; these tests prove the command discovers the registry
//! from a project, indexes schemas by `$id`, and keeps generated TypeScript drift-checkable.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the temporary tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

fn protocol(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(directory)
        .output()
        .expect("the protocol binary runs")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:example:record:1",
  "type": "object",
  "required": ["schema", "name"],
  "properties": {
    "schema": {"const": "urn:example:record:1"},
    "name": {"type": "string", "minLength": 1}
  },
  "additionalProperties": false
}"#;

#[test]
fn validation_discovers_the_registry_declared_by_the_project() {
    let project = scratch("aep-schema-project-registry");
    write(
        &project.join(".engineering/project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: git+file:///path/that/does/not/exist#0123456789abcdef0123456789abcdef01234567\nschemas: contracts\n",
    );
    write(
        &project.join(".engineering/contracts/record.schema.json"),
        SCHEMA,
    );
    write(
        &project.join("evidence/record.json"),
        r#"{"schema":"urn:example:record:1","name":"Ada"}"#,
    );

    let output = protocol(&project, &["schema", "validate", "evidence"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(stdout(&output).contains("1 schema(s), 1 instance(s): valid"));
}

#[test]
fn invalid_instances_accumulate_machine_readable_diagnostics() {
    let project = scratch("aep-schema-invalid-instances");
    write(
        &project.join(".engineering/project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ..\n",
    );
    write(
        &project.join(".engineering/schemas/record.schema.json"),
        SCHEMA,
    );
    write(
        &project.join("evidence/a.json"),
        r#"{"schema":"urn:example:record:1","extra":true}"#,
    );
    write(
        &project.join("evidence/b.json"),
        r#"{"schema":"urn:example:missing:1"}"#,
    );

    let output = protocol(
        &project,
        &["schema", "validate", "evidence", "--format", "json"],
    );
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the report is JSON");
    assert_eq!(report["issues"].as_array().map(Vec::len), Some(3));
    assert_eq!(report["issues"][2]["code"], "unknown_schema");
}

#[test]
fn typescript_is_generated_by_schema_id_and_can_be_drift_checked() {
    let project = scratch("aep-schema-typescript");
    write(
        &project.join(".engineering/project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ..\n",
    );
    write(
        &project.join(".engineering/schemas/record.schema.json"),
        SCHEMA,
    );

    let generated = protocol(
        &project,
        &[
            "schema",
            "typescript",
            "urn:example:record:1",
            "--root",
            "Record",
            "--out",
            "generated/record.ts",
        ],
    );
    assert_eq!(code(&generated), 0, "{}", stderr(&generated));
    let module = std::fs::read_to_string(project.join("generated/record.ts"))
        .expect("the projection was written");
    assert!(module.contains("export type Record = {"), "{module}");
    assert!(
        module.contains("schema: \"urn:example:record:1\";"),
        "{module}"
    );

    let current = protocol(
        &project,
        &[
            "schema",
            "typescript",
            "urn:example:record:1",
            "--root",
            "Record",
            "--out",
            "generated/record.ts",
            "--check",
        ],
    );
    assert_eq!(code(&current), 0, "{}", stderr(&current));

    write(&project.join("generated/record.ts"), "stale\n");
    let stale = protocol(
        &project,
        &[
            "schema",
            "typescript",
            "urn:example:record:1",
            "--root",
            "Record",
            "--out",
            "generated/record.ts",
            "--check",
        ],
    );
    assert_eq!(code(&stale), 1, "{}", stderr(&stale));
    assert_eq!(
        std::fs::read_to_string(project.join("generated/record.ts")).expect("still readable"),
        "stale\n",
        "a drift check must not rewrite its target"
    );
}
