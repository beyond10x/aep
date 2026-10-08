//! `cargo xtask ess`: the ESS specification under `ess/` and the projections committed beside it.
//!
//! The specification describes the `aep plan reverse init` and `aep plan store migrate git`
//! surface. Its projections under `generated/ess/` are written only here, by the pinned `ess`, and
//! `--check` regenerates them in memory and fails, naming the file, when a committed byte differs.
//! The `ess-gate` task runs `--check` through `xtask/tests/ess_gate.rs`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// The specification directory, relative to the repository root.
pub(crate) const SPECIFICATION: &str = "ess";

/// One committed projection: where it is written and the `ess` arguments that write it.
struct Projection {
    /// The output directory, relative to the repository root; `ess` owns everything under it.
    out: &'static str,
    /// The arguments before `--path`, `--out` and `--check`.
    arguments: &'static [&'static str],
    /// Whether `ess` compares this projection itself. `ess generate types` (0.56.0) has no
    /// `--check`; its projection is written to a fresh directory and compared file by file.
    ess_checks: bool,
}

/// Every projection of the specification the repository commits.
///
/// JSON Schema for every declaration, and a standalone Rust data library of every type and event
/// payload. The library is its own Cargo workspace (`ess` writes the `[workspace]` table), so it is
/// not a member of this one and nothing here compiles it.
const PROJECTIONS: &[Projection] = &[
    Projection {
        out: "generated/ess/schema",
        arguments: &["generate", "--kind", "schema"],
        ess_checks: true,
    },
    Projection {
        out: "generated/ess/rust",
        arguments: &[
            "generate",
            "types",
            "--target",
            "rust",
            "--package",
            "aep-plan-ess",
            "--all-types",
            "--all-events",
            "--names",
            "qualified",
        ],
        ess_checks: false,
    },
];

/// Validates and compiles the specification under `root`, then writes its projections, or with
/// `check` compares them and fails on any difference.
///
/// # Errors
///
/// When `ess` is not on `PATH`, when the specification does not validate under its pinned release
/// (`--strict-requires`) or compile, or, with `check`, when a committed projection differs.
pub(crate) fn run(root: &Path, check: bool) -> Result<()> {
    ess(
        root,
        &[
            "specify",
            "validate",
            "--path",
            SPECIFICATION,
            "--strict-requires",
        ],
    )?;
    ess(
        root,
        &[
            "specify",
            "compile",
            "--path",
            SPECIFICATION,
            "--format",
            "json",
            "--strict-requires",
        ],
    )?;
    let mut drifted = Vec::new();
    for projection in PROJECTIONS {
        if check && !projection.ess_checks {
            match compare(root, projection) {
                Ok(()) => println!("{} matches {SPECIFICATION}/", projection.out),
                Err(error) => drifted.push(format!("{}: {error:#}", projection.out)),
            }
            continue;
        }
        let mut arguments: Vec<&str> = projection.arguments.to_vec();
        arguments.extend(["--path", SPECIFICATION, "--out", projection.out]);
        if check {
            arguments.push("--check");
        }
        match ess(root, &arguments) {
            Ok(stdout) if !check => print!("{stdout}"),
            Ok(_) => println!("{} matches {SPECIFICATION}/", projection.out),
            Err(error) if check => drifted.push(format!("{}: {error:#}", projection.out)),
            Err(error) => return Err(error),
        }
    }
    if !drifted.is_empty() {
        bail!(
            "{} projection(s) differ from {SPECIFICATION}/; `cargo xtask ess` rewrites them:\n{}",
            drifted.len(),
            drifted.join("\n")
        );
    }
    Ok(())
}

/// The output-ownership record `ess` keeps in each output directory. It carries a fresh anchor
/// identity per directory, so it is not compared; every file it records is.
const OUTPUT_RECORD: &str = ".ess-output";

/// Writes `projection` to a fresh directory and compares it, file by file, with the committed one.
fn compare(root: &Path, projection: &Projection) -> Result<()> {
    let fresh = std::env::temp_dir().join(format!(
        "aep-xtask-ess-{}-{}",
        std::process::id(),
        projection.out.replace('/', "-")
    ));
    if fresh.exists() {
        fs::remove_dir_all(&fresh).with_context(|| format!("removing {}", fresh.display()))?;
    }
    let out = fresh.to_string_lossy().into_owned();
    let mut arguments: Vec<&str> = projection.arguments.to_vec();
    arguments.extend(["--path", SPECIFICATION, "--out", &out]);
    let written = ess(root, &arguments);
    let differences = written.and_then(|_| {
        let committed = files(&root.join(projection.out))?;
        let generated = files(&fresh)?;
        let mut differences = Vec::new();
        for (path, bytes) in &generated {
            match committed.get(path) {
                None => differences.push(format!("{path}: generated, not committed")),
                Some(held) if held != bytes => differences.push(format!("{path}: differs")),
                Some(_) => {}
            }
        }
        for path in committed
            .keys()
            .filter(|path| !generated.contains_key(*path))
        {
            differences.push(format!("{path}: committed, no longer generated"));
        }
        Ok(differences)
    });
    let _ = fs::remove_dir_all(&fresh);
    let differences = differences?;
    if !differences.is_empty() {
        bail!("{}", differences.join("\n"));
    }
    Ok(())
}

/// Every file under `dir` except the output record, by `/`-separated relative path.
fn files(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, found: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
            let path = entry?.path();
            if path.file_name().is_some_and(|name| name == OUTPUT_RECORD) {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, found)?;
            } else {
                let relative = path
                    .strip_prefix(base)
                    .context("a walked file is under its base")?
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let bytes =
                    fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
                found.insert(relative, bytes);
            }
        }
        Ok(())
    }
    let mut found = BTreeMap::new();
    walk(dir, dir, &mut found)?;
    Ok(found)
}

/// Runs `ess` at `root` and returns its standard output; a non-zero exit fails with both streams.
fn ess(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = match Command::new("ess")
        .args(arguments)
        .current_dir(root)
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => bail!(
            "`ess` is not on PATH; install the release `{SPECIFICATION}/ess-inputs.yaml` pins \
             (`requires:`)"
        ),
        Err(error) => {
            return Err(error).with_context(|| format!("running `ess {}`", arguments.join(" ")))
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        bail!(
            "`ess {}` exited {}\n{stdout}{}",
            arguments.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(stdout)
}
