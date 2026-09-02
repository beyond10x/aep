//! AEP repository automation.
//!
//! The task binary owns deterministic AEP schema and status generation plus the repository's
//! governance, formatting, dependency, and release checks. ESS generation, synthesis,
//! infrastructure, and agent-plugin automation live in their respective repositories.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

/// The index of a generated directory, written from the same list the directory is.
const INDEX: &str = "README.md";

/// Repository automation for AEP.
#[derive(Debug, Parser)]
#[command(name = "xtask", about, version)]
struct Cli {
    /// What to do.
    #[command(subcommand)]
    command: Command,
}

/// The available tasks.
#[derive(Debug, Subcommand)]
enum Command {
    /// Print one release's notes: its CHANGELOG section, reflowed for GitHub.
    Notes {
        /// The version, without a leading `v`. Omitted with `--self-test`.
        version: Option<String>,
        /// Check the reflow against the shapes it must not damage, and print nothing else.
        #[arg(long, conflicts_with = "version")]
        self_test: bool,
    },
    /// Regenerate the published JSON Schemas.
    Schema {
        /// Verify the committed files match instead of writing them.
        #[arg(long)]
        check: bool,
    },
    /// Format the source workspace's members — and only them.
    Fmt {
        /// Verify formatting instead of rewriting it.
        #[arg(long)]
        check: bool,
    },
    /// Regenerate annotated-tag and gate-derived status regions and verify CI's gate delegation.
    Status {
        /// Verify the committed record matches instead of writing it.
        #[arg(long)]
        check: bool,
    },
    /// Check the workspace version against the newest release tag.
    Version,
    /// Check the Entity Runtime pin and refuse any compiled ESS modeling crate.
    Deps,
    /// Check that every uniqueness claim in a test name has a guard beside it.
    Guards,
    /// Check that each released `### Fixed` entry names something that existed to be broken.
    Claims,
    /// Report whether the newest release was cut completely: version, changelog heading, pushed
    /// tag, GitHub Release, and a gate record in the planning store. Reaches the network, so it
    /// is not a gate step.
    Release,
}

/// A release's notes: the tag's own `CHANGELOG.md` section, reflowed so GitHub does not break it
/// mid-sentence.
///
/// # Why the reflow
///
/// Release bodies render as **GFM**, and GFM turns a single newline into a `<br>`. Confirmed against
/// GitHub's own `/markdown` endpoint rather than assumed: the same text posted with `mode: markdown`
/// reflows and with `mode: gfm` does not.
///
/// `CHANGELOG.md` is hard-wrapped at 100 columns, so fed to a release verbatim every one of those
/// wraps arrives as a literal break — text snapping after "added" and before "the", in spots no
/// author chose. Across `entity-runtime`'s seven published releases that was 228 stray breaks.
///
/// **The file stays wrapped.** That is the right shape for something reviewed in a diff, and
/// writing the changelog in one-line paragraphs would make every edit a whole-paragraph diff to
/// please a renderer. Only the notes are joined.
fn notes(root: &Path, version: &str) -> Result<()> {
    let text = fs::read_to_string(root.join("CHANGELOG.md")).context("reading CHANGELOG.md")?;
    let section = changelog_section(&text, version);
    if section.trim().is_empty() {
        bail!("CHANGELOG.md has no section for {version}");
    }
    print!("{}", reflow(&section));
    Ok(())
}

/// Checks the reflow against the shapes a release note must survive.
///
/// This runs in the release workflow *before* the notes are generated, because the failure it
/// catches is silent: a reflow that eats a table or joins two list items produces a release page
/// that is wrong rather than a job that is red, and nobody re-reads a release they already cut.
///
/// Each case is `(name, input, expected)`. The rule under test is the same one everywhere — a line
/// ending is typography inside a paragraph and **content** everywhere else.
fn notes_self_test() -> Result<()> {
    let cases: &[(&str, &str, &str)] = &[
        (
            "a paragraph joins",
            "one line\nand its continuation\n",
            "one line and its continuation\n",
        ),
        (
            "a blank line separates paragraphs",
            "first\npara\n\nsecond\npara\n",
            "first para\n\nsecond para\n",
        ),
        (
            "a fence is copied byte for byte",
            "before\n```console\n$ one\n$ two\n```\nafter\n",
            "before\n```console\n$ one\n$ two\n```\nafter\n",
        ),
        (
            "a table keeps one row per line",
            "| a | b |\n|---|---|\n| 1 | 2 |\n",
            "| a | b |\n|---|---|\n| 1 | 2 |\n",
        ),
        (
            "a heading stands alone",
            "### Fixed\nthe body\n",
            "### Fixed\nthe body\n",
        ),
        (
            "a blockquote is left as written",
            "> quoted\n> lines\n",
            "> quoted\n> lines\n",
        ),
        (
            "list items do not merge, but a wrapped item does",
            "- first item\n  wrapped on\n- second item\n",
            "- first item wrapped on\n- second item\n",
        ),
        (
            "two trailing spaces are the author asking for a break",
            "hard break here  \nnext line\n",
            "hard break here  \nnext line\n",
        ),
    ];

    let mut failed = 0;
    for (name, input, expected) in cases {
        let got = reflow(input);
        if got == *expected {
            println!("ok    {name}");
        } else {
            failed += 1;
            println!("FAIL  {name}\n  expected: {expected:?}\n  got:      {got:?}");
        }
    }
    if failed > 0 {
        bail!("{failed} of {} reflow shapes damaged", cases.len());
    }
    println!("{} reflow shapes hold", cases.len());
    Ok(())
}

/// The lines under `## [version]`, up to the next `## [`.
fn changelog_section(text: &str, version: &str) -> String {
    let mut out = Vec::new();
    let mut found = false;
    for line in text.lines() {
        if line.starts_with("## [") {
            if found {
                break;
            }
            found = line.starts_with(&format!("## [{version}]"));
            continue;
        }
        if found {
            out.push(line);
        }
    }
    out.join("\n")
}

/// `true` when this line carries a list item's own marker: `-`, `*`, `+`, `1.` or `1)`.
fn is_list_item(line: &str) -> bool {
    let rest = line.trim_start();
    if let Some(after) = rest.strip_prefix(['-', '*', '+']) {
        return after.starts_with(' ');
    }
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    !digits.is_empty()
        && rest[digits.len()..].starts_with(['.', ')'])
        && rest[digits.len() + 1..].starts_with(' ')
}

/// Joins the continuation lines of each paragraph, leaving everything else exactly as written.
///
/// Untouched are the places a line ending is **content** rather than typography: fenced code,
/// tables, headings, blockquotes, list-item boundaries, and a line ending in two spaces — which is
/// Markdown's own way of asking for a break, and honouring it is the difference between reflowing
/// and overriding the author.
fn reflow(text: &str) -> String {
    fn flush(out: &mut Vec<String>, pending: &mut Vec<String>) {
        if !pending.is_empty() {
            out.push(pending.join(" "));
            pending.clear();
        }
    }

    let mut out: Vec<String> = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut in_fence = false;

    for line in text.lines() {
        let stripped = line.trim();

        if stripped.starts_with("```") || stripped.starts_with("~~~") {
            flush(&mut out, &mut pending);
            out.push(line.to_owned());
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            out.push(line.to_owned());
            continue;
        }
        if stripped.is_empty() || stripped.starts_with(['|', '#', '>']) {
            flush(&mut out, &mut pending);
            out.push(line.to_owned());
            continue;
        }
        if is_list_item(line) {
            flush(&mut out, &mut pending);
            pending.push(line.trim_end().to_owned());
            continue;
        }
        if line.ends_with("  ") {
            // Markdown's own request for a break. The two spaces are kept, not just the line
            // ending: a bare newline renders as `<br>` under GFM today, so dropping them would
            // leave the author's deliberate break standing on the very quirk this reflow exists
            // to remove.
            let kept = if pending.is_empty() {
                line.to_owned()
            } else {
                format!("{stripped}  ")
            };
            pending.push(kept);
            flush(&mut out, &mut pending);
            continue;
        }
        if pending.is_empty() {
            pending.push(line.trim_end().to_owned());
        } else {
            pending.push(stripped.to_owned());
        }
    }
    flush(&mut out, &mut pending);
    format!("{}\n", out.join("\n").trim())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Notes { version, self_test } => {
            if self_test {
                notes_self_test()
            } else {
                let version = version.context("a version is required without --self-test")?;
                notes(&workspace_root(), &version)
            }
        }
        Command::Schema { check } => schema(&workspace_root(), check),
        Command::Fmt { check } => fmt(check),
        // Both surfaces of one derivation, under one verb. Two verbs is how one of them stops
        // being run, and the surface nobody re-runs is the one strangers read first.
        Command::Status { check } => {
            let root = workspace_root();
            authoritative_gate_contract(&root)?;
            generated_status_region_inventory(&root)?;
            status(&root, check)?;
            agents_gate_steps(&root, check)?;
            website_currency_from_tags(&root, check)
        }
        Command::Version => version_check(&workspace_root()),
        Command::Release => release_check(&workspace_root()),
        Command::Deps => deps(&workspace_root()),
        Command::Guards => guards(&workspace_root()),
        Command::Claims => claims(&workspace_root()),
    }
}

/// The workspace version must equal the newest release tag.
///
/// # The consequence for a person
///
/// `protocol --version` prints `CARGO_PKG_VERSION`, which is the workspace version. If that number
/// does not move with the releases, every build of this tool reports the same string for ever — and
/// somebody running an installed `protocol` from three weeks ago has no way to find out. That is
/// not hypothetical: on 2026-08-26 an installed binary predating the store journal silently wrote
/// **no journal entries** for six status moves, while printing the same `0.1.0` the current build
/// printed. The moves happened, the record did not, and nothing said so.
///
/// So the version is checked against the tags, in the same spirit as the delivered-waves record:
/// derived from what actually shipped rather than maintained by hand beside it.
///
/// # Errors
///
/// If git cannot be run, if there are no tags, or if the two numbers disagree.
/// Runs `git` at `root` and returns its output, failing on a non-zero exit with git's own words.
fn git_at(root: &Path, arguments: &[&str], doing: &str) -> Result<std::process::Output> {
    let output = std::process::Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .with_context(|| format!("running git — {doing}"))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            arguments.first().copied().unwrap_or_default(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output)
}

/// The newest bare-version tag reachable from `HEAD`.
///
/// Bare-version tags only. The pre-0.12.0 slugged form is left behind by convention, and a slug
/// sorted by `-v:refname` would win over a plain number that is actually newer.
fn newest_release_tag(root: &Path) -> Result<String> {
    let output = git_at(
        root,
        // Reachable from HEAD only — see `TAGS_REACHABLE_FROM_HEAD`.
        &["tag", "--list", "--merged", "HEAD", "--sort=-v:refname"],
        "the version is checked against the tags",
    )?;
    let listed = String::from_utf8(output.stdout).context("reading the tag list as UTF-8")?;
    listed
        .lines()
        .map(str::trim)
        .find(|tag| {
            !tag.is_empty()
                && tag
                    .split('.')
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
        })
        .map(str::to_owned)
        .with_context(|| {
            format!(
                "no bare-version tag is visible, so there is nothing to check the workspace \
                 version against — fetch them first (`git fetch --tags`) \
                 ({TAGS_REACHABLE_FROM_HEAD})"
            )
        })
}

/// The `[workspace.package] version` the manifest declares — what `protocol --version` prints.
fn workspace_version(root: &Path) -> Result<String> {
    let manifest =
        fs::read_to_string(root.join("Cargo.toml")).context("reading the workspace manifest")?;
    manifest
        .lines()
        .skip_while(|line| line.trim() != "[workspace.package]")
        .find_map(|line| line.trim().strip_prefix("version = "))
        .map(|value| value.trim().trim_matches('"').to_owned())
        .context("the workspace manifest declares no `[workspace.package] version`")
}

fn version_check(root: &Path) -> Result<()> {
    let newest = newest_release_tag(root)?;
    let declared = workspace_version(root)?;

    if declared != newest {
        bail!(
            "the workspace version is `{declared}` and the newest release tag is `{newest}`.\n\
             `protocol --version` prints the workspace version, so while these disagree the binary \
             cannot say which build it is — which is how a stale install writes nothing and looks \
             like it worked.\n\
             Set `[workspace.package] version` to `{newest}`, or cut the tag the version expects."
        );
    }
    println!("version {declared} matches the newest release tag");
    Ok(())
}

fn changelog_top_version(root: &Path) -> Result<String> {
    let changelog =
        fs::read_to_string(root.join("CHANGELOG.md")).context("reading CHANGELOG.md")?;
    changelog
        .lines()
        .filter_map(|line| line.strip_prefix("## ["))
        .filter_map(|rest| rest.split_once(']'))
        .map(|(version, _)| version.trim())
        .find(|version| !version.eq_ignore_ascii_case("Unreleased"))
        .map(str::to_owned)
        .context("CHANGELOG.md has no `## [<version>]` heading")
}

/// `cargo xtask release`: was the newest release cut completely?
///
/// A release is the procedure in `AGENTS.md` § *Releases*, and until this existed nothing said
/// whether it had been followed: eleven tags shipped with red CI, two tags were never pushed,
/// one GitHub Release was made by hand, and on 2026-08-30 the operator had to define the word to
/// a coordinator that had conflated it with a merge. Five checks, one line each, every one a
/// fact somebody would otherwise look up in three places. Two of them reach the network, which
/// is why this is `task release-check` and not a step of `task check`.
fn release_check(root: &Path) -> Result<()> {
    let tag = newest_release_tag(root)?;
    let version = workspace_version(root)?;
    let heading = changelog_top_version(root)?;
    let commit = String::from_utf8(
        git_at(
            root,
            &["rev-list", "-n", "1", &tag],
            "the tag's commit is what the gate record must name",
        )?
        .stdout,
    )
    .context("reading the tag's commit as UTF-8")?
    .trim()
    .to_owned();

    let pushed = std::process::Command::new("git")
        .args(["ls-remote", "--tags", "origin", &format!("refs/tags/{tag}")])
        .current_dir(root)
        .output()
        .is_ok_and(|output| output.status.success() && !output.stdout.is_empty());
    let released = std::process::Command::new("gh")
        .args(["release", "view", &tag, "--json", "tagName"])
        .current_dir(root)
        .output()
        .is_ok_and(|output| output.status.success());
    let journal =
        fs::read_to_string(root.join(".engineering/planning/journal.jsonl")).unwrap_or_default();
    let short = &commit[..commit.len().min(7)];
    let gated = journal.lines().any(|line| {
        line.contains("\"test_result\"") && (line.contains(&commit) || line.contains(short))
    });

    let checks: [(&str, bool, String); 5] = [
        (
            "workspace version matches the tag",
            version == tag,
            format!("Cargo.toml `{version}`, tag `{tag}`"),
        ),
        (
            "CHANGELOG.md heading matches the tag",
            heading == tag,
            format!("heading `{heading}`, tag `{tag}`"),
        ),
        (
            "tag is pushed to origin",
            pushed,
            format!("refs/tags/{tag}"),
        ),
        (
            "GitHub Release exists",
            released,
            format!("gh release view {tag}"),
        ),
        (
            "planning store holds a test_result naming the tag's commit",
            gated,
            format!("journal.jsonl, commit {short}"),
        ),
    ];
    let mut missing = 0;
    for (what, held, detail) in &checks {
        println!(
            "{}  {what} ({detail})",
            if *held { "ok     " } else { "MISSING" }
        );
        if !held {
            missing += 1;
        }
    }
    if missing > 0 {
        bail!(
            "{missing} of {} release steps are not done for `{tag}`",
            checks.len()
        );
    }
    println!("release {tag} is complete");
    Ok(())
}

/// The generated region in `AGENTS.md` § *Gate* that names the gate's steps.
///
/// The count was hand-written and drifted twice: "eighteen" against "Nineteen" inside one file on
/// 2026-08-28, and "Twenty" against a `Taskfile.yml` with twenty-one on 2026-08-30. The steps are
/// read from the Taskfile's own `check:` block, as the website's currency line already does.
const AGENTS_PAGE: &str = "AGENTS.md";

/// The first line of that region.
const AGENTS_GATE_BEGIN: &str =
    "<!-- generated:gate-steps:begin — do not edit; run `cargo xtask status` -->";

/// The last line of that region.
const AGENTS_GATE_END: &str = "<!-- generated:gate-steps:end -->";

/// The CI and release workflows whose only verification authority is [`GATE_DEFINITION`].
const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const RELEASE_WORKFLOW: &str = ".github/workflows/release.yml";

/// Holds CI to one invocation of the repository gate and release to that same reusable workflow.
fn authoritative_gate_contract(root: &Path) -> Result<()> {
    let ci_path = root.join(CI_WORKFLOW);
    let release_path = root.join(RELEASE_WORKFLOW);
    let ci =
        fs::read_to_string(&ci_path).with_context(|| format!("reading {}", ci_path.display()))?;
    let release = fs::read_to_string(&release_path)
        .with_context(|| format!("reading {}", release_path.display()))?;
    authoritative_gate_text(&ci, &release)?;
    release_asset_text(&release)
}

/// The text-level contract is intentionally exact: a renamed or wrapped command is a reviewable
/// change to the one gate, not something this check guesses is equivalent.
fn authoritative_gate_text(ci: &str, release: &str) -> Result<()> {
    let invocations = ci
        .lines()
        .filter(|line| line.trim() == "run: task check")
        .count();
    if invocations != 1 {
        bail!(
            "{CI_WORKFLOW} must invoke the authoritative `task check` gate exactly once; found \
             {invocations} invocations"
        );
    }
    if !release
        .lines()
        .any(|line| line.trim() == "uses: ./.github/workflows/ci.yml")
    {
        bail!(
            "{RELEASE_WORKFLOW} must reuse {CI_WORKFLOW}; release verification cannot carry a \
             second gate"
        );
    }
    Ok(())
}

/// Holds every cut release to the binary platforms and verification record promised to adopters.
fn release_asset_text(release: &str) -> Result<()> {
    const TARGETS: [&str; 4] = [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
    ];
    for target in TARGETS {
        let matrix_entry = format!("- target: {target}");
        if !release.lines().any(|line| line.trim() == matrix_entry) {
            bail!("{RELEASE_WORKFLOW} must build release target `{target}`");
        }
    }
    if release.to_ascii_lowercase().contains("windows") {
        bail!("{RELEASE_WORKFLOW} may publish only the declared Linux and macOS targets");
    }
    for required in [
        "workflow_dispatch:",
        "always() && needs.provenance.result == 'success' && needs.build.result == 'success'",
        "cargo build --release --locked -p protocol-cli --target ${{ matrix.target }}",
        "target/${TARGET}/release/aep",
        "target/${TARGET}/release/protocol",
        "actions/upload-artifact@",
        "actions/download-artifact@",
        "SHA256SUMS",
        "sha256sum --check SHA256SUMS",
    ] {
        if !release.contains(required) {
            bail!("{RELEASE_WORKFLOW} is missing release-asset control `{required}`");
        }
    }
    for line in release.lines().map(str::trim) {
        let Some(action) = line.strip_prefix("- uses: ") else {
            continue;
        };
        if action.starts_with("./") {
            continue;
        }
        let Some((_, revision_with_comment)) = action.split_once('@') else {
            bail!("{RELEASE_WORKFLOW} has an unversioned action `{action}`");
        };
        let revision = revision_with_comment
            .split_whitespace()
            .next()
            .unwrap_or_default();
        if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("{RELEASE_WORKFLOW} action `{action}` is not pinned to a commit");
        }
    }
    Ok(())
}

/// Writes or checks the gate-step list in `AGENTS.md`.
fn agents_gate_steps(root: &Path, check: bool) -> Result<()> {
    let steps = gate_steps(root)?;
    let listed = steps
        .iter()
        .map(|step| format!("`{step}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let region = format!(
        "`task check` runs **{} steps**, in this order: {listed}.\n",
        steps.len()
    );
    if hold_region(
        root,
        AGENTS_PAGE,
        AGENTS_GATE_BEGIN,
        AGENTS_GATE_END,
        &region,
        check,
    )? {
        println!(
            "wrote the gate-step list into {AGENTS_PAGE} ({} steps)",
            steps.len()
        );
    } else {
        println!(
            "{AGENTS_PAGE} gate-step list is up to date ({} steps)",
            steps.len()
        );
    }
    Ok(())
}

/// Every `entity-*` crate in the lockfile resolves to one version, and all of them to one pin.
///
/// # The consequence for a person
///
/// Two kernels compiled into one workspace means a story that says *"the runtime does X"* is about
/// one of them and silent about the other. That was the state on 2026-08-28: `aep-backend-markdown`
/// pinned `entity-core` at `0.5.2` — the kernel that decides every `protocol artifact move` — and
/// `aep-backend-sqlite` pinned `0.8.0`, four releases on, with fixes to the store the markdown
/// side's kernel predated. `cargo tree -i entity-core` answered *"specification is ambiguous"*, and
/// nothing in the gate noticed for two releases.
///
/// # What is checked, and from where
///
/// `Cargo.lock`, not the manifests. The lockfile is what is actually compiled, and a manifest that
/// names a tag is only a request. Two rules, each with the offending lines in the message:
///
/// 1. no `entity-*` package appears at two versions;
/// 2. every `entity-*` package comes from the same source — one tag, one commit.
///
/// The prefix is `entity-` because that is what `entity-runtime` publishes; the check would fire
/// for any crate somebody added under that name, which is the right answer — a crate that is not
/// theirs under a name that looks like theirs is a question for a reviewer either way.
///
/// # Errors
///
/// If `Cargo.lock` cannot be read, or if either rule is broken.
fn deps(root: &Path) -> Result<()> {
    let lock = fs::read_to_string(root.join("Cargo.lock")).context("reading Cargo.lock")?;
    let ess_packages = locked_packages(&lock, ESS_MODEL_PREFIX);
    if !ess_packages.is_empty() {
        let names = ess_packages
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "AEP compiles against ESS modeling crates: {names}. The optional adapter transcribes \
             the closed standalone report wire and must not depend on an `ess-*` package."
        );
    }

    let packages = locked_packages(&lock, ENTITY_RUNTIME_PREFIX);
    if packages.is_empty() {
        bail!(
            "no `{ENTITY_RUNTIME_PREFIX}*` package is in Cargo.lock, so there is no pin to check — \
             the backends depend on `entity-runtime` and the lockfile should say so"
        );
    }

    let mut by_name: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut sources: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for package in &packages {
        by_name
            .entry(package.name.as_str())
            .or_default()
            .insert(package.version.as_str());
        sources
            .entry(package.source.as_str())
            .or_default()
            .insert(package.name.as_str());
    }

    let duplicated: Vec<String> = by_name
        .iter()
        .filter(|(_, versions)| versions.len() > 1)
        .map(|(name, versions)| {
            format!(
                "  {name} at {}",
                versions.iter().copied().collect::<Vec<_>>().join(" and ")
            )
        })
        .collect();
    if !duplicated.is_empty() {
        bail!(
            "`entity-runtime` is compiled into this workspace at two versions:\n{}\n\
             One kernel decides every `protocol artifact move` and another sits under the SQLite \
             backend, so a claim about \"the runtime\" is about one of them and silent about the \
             other. Every `entity-*` dependency must name the same tag \
             (`crates/aep-backend-markdown/Cargo.toml`, `crates/aep-backend-sqlite/Cargo.toml`), \
             then `cargo update -p <crate>` until `cargo tree -i entity-core` is unambiguous.",
            duplicated.join("\n")
        );
    }

    if sources.len() > 1 {
        let listed: Vec<String> = sources
            .iter()
            .map(|(source, names)| {
                format!(
                    "  {} from {source}",
                    names.iter().copied().collect::<Vec<_>>().join(", ")
                )
            })
            .collect();
        bail!(
            "the `entity-*` crates come from more than one `entity-runtime` pin:\n{}\n\
             One tag for all of them, so \"the runtime at this commit\" is one sentence a review can \
             check.",
            listed.join("\n")
        );
    }

    let names: Vec<&str> = by_name.keys().copied().collect();
    let (source, _) = sources.iter().next().expect("at least one source");
    println!(
        "entity-runtime is pinned once: {} at {} ({source})",
        names.join(", "),
        packages[0].version
    );
    Ok(())
}

/// The lockfile packages whose name starts with `prefix`.
///
/// A `[[package]]` block in `Cargo.lock` is flat `key = "value"` lines up to the next block; three of
/// them are read. A package with no `source` line is a workspace member, and its source is recorded
/// as `path`, so a local crate under the prefix would be reported as a second pin rather than
/// slipping past as "no source".
fn locked_packages(lock: &str, prefix: &str) -> Vec<LockedPackage> {
    let mut packages = Vec::new();
    let mut current: Option<LockedPackage> = None;
    let mut push = |current: &mut Option<LockedPackage>| {
        if let Some(package) = current.take() {
            if package.name.starts_with(prefix) {
                packages.push(package);
            }
        }
    };
    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            push(&mut current);
            current = Some(LockedPackage {
                name: String::new(),
                version: String::new(),
                source: "path".to_owned(),
            });
            continue;
        }
        let Some(package) = current.as_mut() else {
            continue;
        };
        if let Some(value) = line.strip_prefix("name = ") {
            value.trim_matches('"').clone_into(&mut package.name);
        } else if let Some(value) = line.strip_prefix("version = ") {
            value.trim_matches('"').clone_into(&mut package.version);
        } else if let Some(value) = line.strip_prefix("source = ") {
            value.trim_matches('"').clone_into(&mut package.source);
        } else if line.starts_with('[') {
            // A new table that is not a package — `[metadata]`, say — ends the current block.
            push(&mut current);
        }
    }
    push(&mut current);
    packages
}

/// One resolved package, as `Cargo.lock` records it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LockedPackage {
    name: String,
    version: String,
    /// The `source` line, or `path` for a workspace member.
    source: String,
}

/// What `entity-runtime` publishes its crates as.
const ENTITY_RUNTIME_PREFIX: &str = "entity-";
const ESS_MODEL_PREFIX: &str = "ess-";

/// A test asserting the same thing as one in another crate is not evidence of a difference.
///
/// # The defect this catches, and why the gate could not
///
/// On 2026-08-26 a reviewer found that `entity-sqlite`'s `a_refused_commit_rolls_back_both_halves`
/// asserted a refusal that happens at the **pre-check**, before either write — so there were no
/// halves to roll back. It was byte-for-byte the same assertion as
/// `every_provider_leaves_a_refused_commit_with_no_trace`, which runs against the providers that
/// document they *cannot* keep that promise. It had been cited as evidence for a requirement across
/// two releases.
///
/// The gate could not tell. It checks that a cited test **passes**, and a test that asserts nothing
/// distinguishing passes beautifully.
///
/// So: a test body that appears in more than one crate is reported. Either the two crates are
/// testing one shared behaviour — in which case the assertion belongs in a shared suite, run
/// against both, which is what `aep-conformance` is — or one of them is named for a property it
/// does not assert.
///
/// # Why bodies and not names
///
/// A name heuristic was tried first and reported 92 findings against correct code, because ordinary
/// test names contain "only" and "cannot". A check that fires that often on working code is one
/// somebody switches off, which makes it worse than nothing. A duplicated body is a fact.
///
/// # Errors
///
/// If a source file cannot be read, or if a body is duplicated across crates.
fn guards(root: &Path) -> Result<()> {
    /// A body shorter than this says too little to be worth comparing.
    const FLOOR: usize = 120;

    // The three parallel command vocabularies. `adp`, `aep` and `aop` each define their own
    // `Command` enum and each carries the same structural tests over it — round-trips through JSON,
    // the sample set covering every kind. The bodies are identical because the *shape* is identical
    // and the type is reached through a `use` at the top of each file; they are three tests of
    // three different types, not one test cited twice.
    //
    // Allowlisted rather than silently skipped: a reader of this list can see what was excused and
    // argue with it, which is the difference between an exception and a blind spot.
    const PARALLEL_VOCABULARIES: &[&str] = &["adp-domain", "aep-domain", "aop-domain"];

    let mut bodies: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

    for entry in walk(&root.join("crates")) {
        if entry.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text =
            fs::read_to_string(&entry).with_context(|| format!("reading {}", entry.display()))?;
        if !text.contains("#[test]") {
            continue;
        }
        let relative = entry.strip_prefix(root).unwrap_or(&entry);
        let crate_name = relative.components().nth(1).map_or_else(
            || "?".to_owned(),
            |c| c.as_os_str().to_string_lossy().into_owned(),
        );

        for (name, body) in test_bodies(&text) {
            // Comments and whitespace out: two tests that differ only in what they say about
            // themselves are the same test.
            let normalised: String = body
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            if normalised.len() < FLOOR {
                continue;
            }
            bodies
                .entry(normalised)
                .or_default()
                .push((crate_name.clone(), name));
        }
    }

    let mut findings = Vec::new();
    let mut excused = 0usize;
    for sites in bodies.values() {
        let crates: BTreeSet<&str> = sites.iter().map(|(krate, _)| krate.as_str()).collect();
        if crates.len() > 1
            && crates
                .iter()
                .all(|krate| PARALLEL_VOCABULARIES.contains(krate))
        {
            excused += 1;
            continue;
        }
        // **Two tests with one body, wherever they live.** Requiring different crates missed a
        // real pair inside one, and a duplicate is a duplicate: either the assertion belongs in
        // one place run against both, or one of the two is named for something it does not assert.
        if sites.len() > 1 {
            findings.push(
                sites
                    .iter()
                    .map(|(krate, name)| format!("{krate}::{name}"))
                    .collect::<Vec<_>>()
                    .join("  ==  "),
            );
        }
    }

    for finding in &findings {
        println!("  - {finding}");
    }
    println!(
        "{} test body/bodies compared, {} duplicated across crates, {excused} excused (the three \
         parallel command vocabularies)",
        bodies.len(),
        findings.len()
    );
    if findings.is_empty() {
        Ok(())
    } else {
        bail!(
            "a test asserting exactly what a test in another crate asserts is not evidence that \
             the two behave differently. Either move the assertion into a shared suite run against \
             both — which is what `aep-conformance` is for — or make the test assert the thing its \
             name claims."
        )
    }
}

/// Every `#[test]` function's name and body text.
fn test_bodies(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (at, _) in text.match_indices("#[test]") {
        let rest = &text[at..];
        let Some(fn_at) = rest.find("fn ") else {
            continue;
        };
        let after = &rest[fn_at + 3..];
        let Some(paren) = after.find('(') else {
            continue;
        };
        let name = after[..paren].trim().to_owned();
        let Some(open) = after.find('{') else {
            continue;
        };

        // Braces inside a string literal are not braces. Counting them swallowed the `#[test]`
        // that followed two real bodies, so those two were never compared at all.
        let mut depth = 0i32;
        let mut end = None;
        let mut in_string = false;
        let mut escaped = false;
        for (index, character) in after[open..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match character {
                '\\' if in_string => escaped = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(open + index);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            found.push((name, after[open + 1..end].to_owned()));
        }
    }
    found
}

/// Every file under `root`, recursively.
fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some("target") {
                    continue;
                }
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// A released `### Fixed` entry must name something that existed at the previous release.
///
/// # The defect this catches
///
/// `entity-runtime` 0.6.0 shipped three `### Fixed` entries. **Two described defects that never
/// existed in a release.** One named a requirement-numbering scheme, `R-90b`, that appears nowhere
/// in that repository's history except in the changelog entry claiming it was wrong; the other
/// described a `serde` defect in a file that did not exist before the release it was said to be
/// fixed in.
///
/// Both were real — caught while the wave was being built. The tag message said so correctly:
/// *"three defects the wave's own tests caught on first run"*. `### Fixed` did not, and `### Fixed`
/// is a promise about what a user of the **previous** version experienced.
///
/// # What is checked, and what is not
///
/// Backticked identifiers only — file paths, symbol names, requirement ids. Each must appear
/// somewhere in the tree at the previous release tag. Prose is not read and fairness is not judged:
/// this checks that the things a bullet names were there to be broken.
///
/// A bullet naming nothing checkable is counted as **unverifiable** rather than passed, so the
/// number of claims actually checked is honest.
///
/// # Errors
///
/// If git cannot be run, or if a released bullet names something that did not yet exist.
fn claims(root: &Path) -> Result<()> {
    // Published sections are not rewritten — that rule has held all through this repository's
    // history and is why corrections live in a *later* section. So an entry already released that
    // names only its own fix is excused **by name, with its reason**, rather than quietly skipped
    // or fixed by editing what shipped.
    const EXCUSED: &[(&str, &str)] = &[(
        "0.26.0",
        "the dangling-check bullet names `ArtifactGraph::build_in_workspace` and \
         `StoreReport::graph_in_workspace`, both introduced by the fix. What was broken lived in \
         `artifact.rs`, which the bullet describes in prose and does not backtick. Released; a \
         published section stays as published.",
    )];

    let text = fs::read_to_string(root.join("CHANGELOG.md")).context("reading CHANGELOG.md")?;

    let sections = fixed_sections(&text);

    let mut checked = 0usize;
    let mut unverifiable = 0usize;
    let mut excused = 0usize;
    let mut findings = Vec::new();

    for (release, bullets) in &sections {
        let Some(before) = previous_tag(root, release)? else {
            continue;
        };
        for bullet in bullets {
            let named: Vec<&str> = bullet
                .split('`')
                .skip(1)
                .step_by(2)
                .filter(|token| {
                    // A token worth checking: a path, a symbol, or a requirement id. Prose in
                    // backticks — a shell line, a sentence — is not, and neither is a literal:
                    // `1..=3650` contains a dot and named nothing anybody could grep for.
                    let starts_like_a_name = token
                        .chars()
                        .next()
                        .is_some_and(|first| first.is_ascii_alphabetic());
                    !token.contains(' ')
                        && token.len() > 3
                        && (starts_like_a_name || token.starts_with("R-"))
                        && (token.contains('.') || token.contains('_') || token.starts_with("R-"))
                })
                .collect();
            if named.is_empty() {
                unverifiable += 1;
                continue;
            }
            // **At least one named thing must have existed**, not all of them. A `### Fixed`
            // bullet names the defect *and* the fix, and the fix's identifiers are new by
            // definition — requiring every one to pre-date the release flagged twenty-four entries
            // that were describing themselves honestly.
            //
            // What the rule still catches is the case it was written for: a bullet where *nothing*
            // it names existed at the previous release describes a defect no user of that release
            // could have met. `R-90b` was exactly that — its only token appeared nowhere but in the
            // changelog entry claiming it was wrong.
            checked += 1;
            let mut any_existed = false;
            for token in &named {
                if existed_at(root, &before, token)? {
                    any_existed = true;
                    break;
                }
            }
            if !any_existed && EXCUSED.iter().any(|(tag, _)| *tag == release.as_str()) {
                excused += 1;
                continue;
            }
            if !any_existed {
                findings.push(format!(
                    "{release} `### Fixed` names {} — and none of them appears anywhere at \
                     {before}, so a user of {before} cannot have hit this. If it was caught while \
                     the wave was built, it belongs in `### Changed` or the tag message.",
                    named
                        .iter()
                        .map(|token| format!("`{token}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }

    for finding in &findings {
        println!("  - {finding}");
    }
    println!(
        "{checked} claim(s) checked against the previous release, {unverifiable} unverifiable, \
         {excused} excused, {} finding(s)",
        findings.len()
    );
    if findings.is_empty() {
        Ok(())
    } else {
        bail!("`### Fixed` is a promise about what a user of the previous version experienced")
    }
}

/// Every released `### Fixed` section, as (version, bullets).
///
/// `[Unreleased]` is skipped: a bullet there describes work that has not shipped, so there is no
/// previous release to check it against. Its bullets are still **drained** at the header, because
/// leaving them in the buffer attributed them to the next released heading — a correctly-filed
/// unreleased fix checked against the wrong tag.
///
/// A bullet's continuation lines are joined onto it. Dropping them read only the first line of every
/// multi-line entry, which in this repository is almost all of them.
fn fixed_sections(text: &str) -> Vec<(String, Vec<String>)> {
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut version: Option<String> = None;
    let mut fixed = false;
    let mut bullets: Vec<String> = Vec::new();

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("## [") {
            let carried = std::mem::take(&mut bullets);
            if let Some(previous) = version.take() {
                sections.push((previous, carried));
            }
            let name = rest.split(']').next().unwrap_or_default().to_owned();
            version = (name != "Unreleased").then_some(name);
            fixed = false;
        } else if line.starts_with("### ") {
            fixed = line.trim() == "### Fixed";
        } else if fixed && (line.starts_with("* ") || line.starts_with("- ")) {
            bullets.push(line.to_owned());
        } else if fixed && !line.trim().is_empty() && line.starts_with("  ") {
            if let Some(last) = bullets.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
            }
        }
    }
    if let Some(previous) = version {
        sections.push((previous, bullets));
    }
    sections
}

/// The release tag immediately before `version`, if this repository has one.
///
/// **Ordered by when the tag was made, not by version name.** Tags before `0.12.0` carry a slugged
/// form (`0.11.0-ground-truth-and-docs`), and a bare-version-only ordering skipped straight past
/// them — so `0.12.0` was checked against `0.2.1`, thirty releases earlier, and every identifier
/// introduced in between read as one that "did not exist". Twenty-five false findings from one
/// wrong comparison.
fn previous_tag(root: &Path, version: &str) -> Result<Option<String>> {
    let output = std::process::Command::new("git")
        // Reachable from HEAD only — see `TAGS_REACHABLE_FROM_HEAD`.
        .args(["tag", "--list", "--merged", "HEAD", "--sort=-creatordate"])
        .current_dir(root)
        .output()
        .context("running git — released claims are checked against the tags")?;
    // A git that failed is not a repository with no tags. Ignoring the status let a clone without
    // tags skip every section and report green having checked nothing.
    if !output.status.success() {
        bail!(
            "git tag failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let listed = String::from_utf8(output.stdout).context("reading the tag list as UTF-8")?;
    let ordered: Vec<&str> = listed
        .lines()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect();
    Ok(ordered
        .iter()
        .position(|tag| *tag == version)
        .and_then(|at| ordered.get(at + 1))
        .map(|tag| (*tag).to_owned()))
}

/// Why every tag lookup in this file asks for `--merged HEAD` rather than the whole namespace.
///
/// A gate step runs *at a commit*, but `git tag` answers with the tags the **clone** happens to
/// hold — not the tags that existed when that commit was made. Push two release tags in one
/// `git push` and the older tag's Release run checks out the older commit with both tags fetched:
/// `docs/status.md` there records 38 tags, the clone shows 39, and the drift check fails a release
/// that was correct when it was cut. That is exactly what happened to 0.27.1 — the tag shipped,
/// its own gate refused it, and no GitHub Release was ever published for it.
///
/// Reachability makes each release's gate answer the question it means to ask: what had shipped
/// *as of this commit*. At the tip of `main` the two sets are identical, so nothing about the
/// everyday check changes.
const TAGS_REACHABLE_FROM_HEAD: &str =
    "only tags reachable from HEAD are counted, so a later release cannot retroactively fail an \
     earlier one";

/// Whether `token` appears anywhere in the tree at `tag`.
fn existed_at(root: &Path, tag: &str, token: &str) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["grep", "--fixed-strings", "--quiet", token, tag])
        .current_dir(root)
        .output()
        .context("running git grep")?;
    Ok(output.status.success())
}

/// The repository root, derived from this crate's manifest directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

/// The page holding the delivered-waves record.
const STATUS_PAGE: &str = "docs/status.md";

/// The first line of the generated region `cargo xtask status` owns inside that page.
///
/// The rest of the page is hand-written; this pair of markers bounds the one part of it that is
/// derived. In place rather than a file of its own, so the reader gets one status page and not a
/// stub pointing at a fragment.
const STATUS_BEGIN: &str =
    "<!-- generated:delivered-waves:begin — do not edit; run `cargo xtask status` -->";

/// The last line of that region.
const STATUS_END: &str = "<!-- generated:delivered-waves:end -->";

/// The website page carrying the same claim for a reader who is not in the repository.
const SITE_STATUS_PAGE: &str = "website/docs/status/where-this-stands.md";

/// The landing page, whose status panel names the release in a chip.
const SITE_LANDING_PAGE: &str = "website/src/pages/index.tsx";

/// The generated region on the website's status page.
///
/// An MDX comment, not the HTML one `docs/status.md` uses: Docusaurus compiles a page under
/// `website/docs/` as MDX 3, which refuses `<!--` outright — *to create a comment in MDX, use
/// `{/* text */}`*. The two pages therefore carry the same region under two spellings, and the
/// spelling is the renderer's to dictate.
const SITE_STATUS_BEGIN: &str =
    "{/* generated:currency:begin — do not edit; run `cargo xtask status` */}";

/// The end of that region.
const SITE_STATUS_END: &str = "{/* generated:currency:end */}";

/// The generated region around the landing page's release chip.
///
/// A JSX block comment rather than an HTML one: this is a `.tsx` file, and an HTML comment inside a
/// JSX expression is text on the page.
const SITE_CHIP_BEGIN: &str =
    "/* generated:release-chip:begin — do not edit; run `cargo xtask status` */";

/// The end of that region.
const SITE_CHIP_END: &str = "/* generated:release-chip:end */";

/// Every reader-facing volatile status region owned by `cargo xtask status`.
const STATUS_REGION_FILES: &[&str] = &[
    AGENTS_PAGE,
    STATUS_PAGE,
    SITE_STATUS_PAGE,
    SITE_LANDING_PAGE,
];

/// Refuses a generated status marker the command does not know how to refresh.
fn generated_status_region_inventory(root: &Path) -> Result<()> {
    let mut found = BTreeSet::new();
    collect_status_region_files(root, root, &mut found)?;
    let expected: BTreeSet<String> = STATUS_REGION_FILES
        .iter()
        .map(|path| (*path).to_owned())
        .collect();
    if found != expected {
        let missing = expected.difference(&found).cloned().collect::<Vec<_>>();
        let unowned = found.difference(&expected).cloned().collect::<Vec<_>>();
        bail!(
            "the `cargo xtask status` region inventory drifted; missing: [{}]; unowned: [{}]",
            missing.join(", "),
            unowned.join(", ")
        );
    }
    Ok(())
}

/// Finds status-generator begin markers in reader-facing source files.
fn collect_status_region_files(
    root: &Path,
    directory: &Path,
    found: &mut BTreeSet<String>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("reading status-region directory {}", directory.display()))?
    {
        let entry = entry
            .with_context(|| format!("reading status-region entry in {}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            if matches!(
                name.to_str(),
                Some(".git" | "target" | "node_modules" | "build")
            ) {
                continue;
            }
            collect_status_region_files(root, &path, found)?;
            continue;
        }
        let extension = path.extension().and_then(|value| value.to_str());
        if !matches!(extension, Some("md" | "mdx" | "tsx")) {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading status-region candidate {}", path.display()))?;
        if text
            .lines()
            .any(|line| line.contains("generated:") && line.contains("cargo xtask status"))
        {
            found.insert(
                path.strip_prefix(root)
                    .expect("the walk stays below the repository root")
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}

/// The gate's own definition, which is where its step list is derived from.
const GATE_DEFINITION: &str = "Taskfile.yml";

/// The steps `task check` runs, in order, read from the Taskfile.
///
/// Derived rather than transcribed for the reason the delivered-waves table is: the website told
/// readers the gate had **twenty** steps for as long as it had twenty, and would have gone on
/// saying so. A step list is a claim about this repository that this repository can answer.
fn gate_steps(root: &Path) -> Result<Vec<String>> {
    let path = root.join(GATE_DEFINITION);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut steps = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line == "  check:" {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(name) = line.strip_prefix("      - task: ") {
            steps.push(name.trim().to_owned());
        } else if line.starts_with("  ")
            && line.trim_end().ends_with(':')
            && !line.starts_with("   ")
        {
            // The next task's definition: the `check:` block has ended.
            break;
        }
    }
    if steps.is_empty() {
        bail!(
            "{GATE_DEFINITION} has no `check:` block with `- task:` steps in it, so the gate's own \
             step list cannot be derived — failing rather than publishing an empty gate"
        );
    }
    Ok(steps)
}

/// The website's currency claim: which tag this is, and what the gate runs at it.
fn currency(tag: &str, dated: &str, steps: &[String]) -> String {
    let listed = match steps.split_last() {
        Some((last, rest)) => format!(
            "{} and `{last}`",
            rest.iter()
                .map(|step| format!("`{step}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        None => String::new(),
    };
    format!(
        "Current as of the tag `{tag}` ({dated}).\n\nThe repository's gate, `task check`, runs \
         **{} steps** — {listed}.\n",
        steps.len()
    )
}

/// The day the newest reachable tag was created, as the tag itself records it.
fn tag_date(root: &Path, tag: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args([
            "for-each-ref",
            "--format=%(creatordate:short)",
            &format!("refs/tags/{tag}"),
        ])
        .current_dir(root)
        .output()
        .context("running git — the currency stamp is dated from the tag")?;
    if !output.status.success() {
        bail!(
            "git for-each-ref failed for {tag}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let dated = String::from_utf8(output.stdout)
        .context("reading the tag date as UTF-8")?
        .trim()
        .to_owned();
    if dated.is_empty() {
        bail!("{tag} has no creation date, so the currency stamp cannot be dated from it");
    }
    Ok(dated)
}

/// Splices one generated region, and says whether the file changed.
///
/// Shared by the three regions so that a page added later cannot get a subtly different rule about
/// what `--check` means.
fn hold_region(
    root: &Path,
    relative: &str,
    begin: &str,
    end: &str,
    replacement: &str,
    check: bool,
) -> Result<bool> {
    let path = root.join(relative);
    let current =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let updated = splice_generated(&current, begin, end, replacement)?;
    if updated == current {
        return Ok(false);
    }
    if check {
        bail!(
            "{relative}'s generated region no longer matches the tags; run `cargo xtask status` \
             and commit the result"
        );
    }
    fs::write(&path, updated).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

/// The newest tag reachable from `HEAD`, which is the release this checkout *is*.
fn newest_reachable_tag(root: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args([
            "for-each-ref",
            "refs/tags",
            // Reachable from HEAD only — see `TAGS_REACHABLE_FROM_HEAD`.
            "--merged",
            "HEAD",
            "--sort=creatordate",
            "--format=%(refname:short)",
        ])
        .current_dir(root)
        .output()
        .context("running git — the currency stamp is derived from the tags")?;
    if !output.status.success() {
        bail!(
            "git for-each-ref failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout)
        .context("reading the tag list as UTF-8")?
        .lines()
        .next_back()
        .map(str::to_owned)
        .context(
            "no tags are reachable from HEAD, so there is no release for the website to be current \
             as of — fetch them first (`git fetch --tags`)",
        )
}

/// Writes or checks the website's currency stamps against the newest reachable tag, and reports.
fn website_currency_from_tags(root: &Path, check: bool) -> Result<()> {
    let newest = newest_reachable_tag(root)?;
    let written = website_currency(root, &newest, check)?;
    if check {
        println!("the website's currency stamps are up to date ({newest})");
    } else {
        println!("{written} website currency stamp(s) rewritten to {newest}");
    }
    Ok(())
}

/// Writes or checks the three derived claims the website makes about this repository.
///
/// The website is not a component gate of its own: `npm run build` resolves links and reads no
/// claim, so a page describing a repository that moved underneath it builds green for ever. The
/// landing page's chip said `0.7.1-infra-waves-1-4` on 2026-08-30, twenty-six tags after that was
/// true, and it was the first version number any visitor saw.
fn website_currency(root: &Path, tag: &str, check: bool) -> Result<usize> {
    let dated = tag_date(root, tag)?;
    let steps = gate_steps(root)?;
    let mut written = 0;
    if hold_region(
        root,
        SITE_STATUS_PAGE,
        SITE_STATUS_BEGIN,
        SITE_STATUS_END,
        &currency(tag, &dated, &steps),
        check,
    )? {
        written += 1;
    }
    if hold_region(
        root,
        SITE_LANDING_PAGE,
        SITE_CHIP_BEGIN,
        SITE_CHIP_END,
        &format!("        <code>{tag}</code>\n        "),
        check,
    )? {
        written += 1;
    }
    Ok(written)
}

/// Writes or checks the delivered-waves record in `docs/status.md`.
///
/// The table is derived from the repository's annotated tags, oldest first, because `git tag -n99`
/// is the per-wave record of what actually shipped — and because the delivered-waves list was the
/// one status surface still maintained by hand. Four hand-written gate counts drifted apart within
/// the repository's first 48 hours; the fix is the rule invariant 1 already applies to the
/// schemas: derive, then drift-check.
fn status(root: &Path, check: bool) -> Result<()> {
    let output = std::process::Command::new("git")
        .args([
            "for-each-ref",
            "refs/tags",
            // Reachable from HEAD only — see `TAGS_REACHABLE_FROM_HEAD`.
            "--merged",
            "HEAD",
            "--sort=creatordate",
            "--format=%(refname:short)\t%(subject)",
        ])
        .current_dir(root)
        .output()
        .context("running git — the delivered-waves record is derived from the tags")?;
    if !output.status.success() {
        bail!(
            "git for-each-ref failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8(output.stdout).context("reading the tag list as UTF-8")?;
    let tags: Vec<(&str, &str)> = stdout
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    if tags.is_empty() {
        bail!(
            "no tags are visible, so there is nothing to derive the delivered-waves record from — \
             fetch them first (`git fetch --tags`). Failing here rather than writing an empty \
             table, because an empty record reads exactly like a project that shipped nothing \
             ({TAGS_REACHABLE_FROM_HEAD})"
        );
    }

    let path = root.join(STATUS_PAGE);
    let current =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let updated = splice_generated(&current, STATUS_BEGIN, STATUS_END, &delivered_waves(&tags))?;

    if check {
        if updated != current {
            bail!(
                "{STATUS_PAGE}'s delivered-waves record no longer matches the tags; run \
                 `cargo xtask status` and commit the result"
            );
        }
        println!("delivered-waves record is up to date ({} tags)", tags.len());
    } else if updated == current {
        println!(
            "delivered-waves record already matches the tags ({} tags)",
            tags.len()
        );
    } else {
        fs::write(&path, updated).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote the delivered-waves record ({} tags)", tags.len());
    }

    Ok(())
}

/// Renders the delivered-waves table from `(tag, subject)` pairs, oldest first.
///
/// The subject is an annotated tag message's first line, which in this repository already names
/// the wave and what it delivered. A `|` in either column would silently break the table, so it is
/// escaped rather than trusted absent.
fn delivered_waves(tags: &[(&str, &str)]) -> String {
    let mut table = String::from("| tag | delivered |\n|---|---|\n");
    for (tag, subject) in tags {
        let tag = tag.replace('|', "\\|");
        // Annotated tags are immutable history, but these legacy release subjects carried the
        // repository's retired name. The public status surface speaks the current name instead.
        let subject = if matches!(
            tag.as_str(),
            "0.36.0" | "0.36.1" | "0.36.2" | "0.36.3" | "0.36.4" | "0.37.0" | "0.37.1"
        ) {
            format!("AEP {tag}")
        } else {
            (*subject).to_owned()
        }
        .replace('|', "\\|");
        let _ = writeln!(table, "| `{tag}` | {subject} |");
    }
    table
}

/// Replaces everything between `begin` and `end` with `replacement`, keeping both marker lines.
///
/// A missing or reversed marker is an error rather than a no-op, because a page that has lost its
/// markers would otherwise be reported clean while the generator had silently stopped maintaining
/// it — the same defect class as a source scan that stopped seeing constructions.
fn splice_generated(content: &str, begin: &str, end: &str, replacement: &str) -> Result<String> {
    let begin_at = content
        .find(begin)
        .with_context(|| format!("the begin marker `{begin}` is missing from the page"))?;
    let after_begin = begin_at + begin.len();
    let end_offset = content[after_begin..].find(end).with_context(|| {
        format!("the end marker `{end}` is missing from the page, or precedes the begin marker")
    })?;
    let end_at = after_begin + end_offset;
    let mut updated = String::with_capacity(content.len() + replacement.len());
    updated.push_str(&content[..after_begin]);
    updated.push('\n');
    updated.push_str(replacement);
    updated.push_str(&content[end_at..]);
    Ok(updated)
}

/// Writes or checks `schemas/generated/`.
fn schema(root: &Path, check: bool) -> Result<()> {
    let directory = root.join("schemas/generated");
    if !check {
        fs::create_dir_all(&directory)
            .with_context(|| format!("creating {}", directory.display()))?;
    }

    let mut differing = Vec::new();
    let mut expected = BTreeSet::new();
    let mut written = 0_usize;
    let mut removed = 0_usize;

    for entry in aep_schema::generated_schemas() {
        expected.insert(entry.filename.clone());
        let path = directory.join(&entry.filename);
        let generated = entry
            .to_json()
            .with_context(|| format!("serialising the {} schema", entry.name))?;

        if check {
            let committed =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            if committed != generated {
                differing.push(entry.filename.clone());
            }
        } else {
            let unchanged = fs::read_to_string(&path).is_ok_and(|committed| committed == generated);
            if !unchanged {
                fs::write(&path, &generated)
                    .with_context(|| format!("writing {}", path.display()))?;
                written += 1;
            }
        }
    }

    // The index is generated from the same list, so a schema cannot be added without appearing in
    // the documentation that tells a reader it exists.
    let index_path = directory.join(INDEX);
    let index = schema_index();
    expected.insert(INDEX.to_owned());
    if check {
        let committed = fs::read_to_string(&index_path)
            .with_context(|| format!("reading {}", index_path.display()))?;
        if committed != index {
            differing.push(INDEX.to_owned());
        }
    } else if !fs::read_to_string(&index_path).is_ok_and(|committed| committed == index) {
        fs::write(&index_path, &index)
            .with_context(|| format!("writing {}", index_path.display()))?;
        written += 1;
    }

    // Every file here is an output, so one that nothing generates is drift the other direction: a
    // schema that was renamed or withdrawn leaves its file behind, and a consumer validating
    // against that file goes on passing against a contract this repository no longer publishes.
    let mut orphaned = Vec::new();
    for entry in
        fs::read_dir(&directory).with_context(|| format!("reading {}", directory.display()))?
    {
        let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if expected.contains(&name) || !entry.path().is_file() {
            continue;
        }
        if check {
            orphaned.push(name);
        } else {
            fs::remove_file(entry.path())
                .with_context(|| format!("removing {}", entry.path().display()))?;
            removed += 1;
        }
    }
    orphaned.sort();

    if check {
        if differing.is_empty() && orphaned.is_empty() {
            println!("schemas are up to date");
            return Ok(());
        }
        let mut detail = String::new();
        if !differing.is_empty() {
            let _ = writeln!(
                detail,
                "{} file(s) differ from the Rust types: {}",
                differing.len(),
                differing.join(", ")
            );
        }
        if !orphaned.is_empty() {
            let _ = writeln!(
                detail,
                "{} file(s) are generated by nothing any more: {}",
                orphaned.len(),
                orphaned.join(", ")
            );
        }
        bail!("{detail}run `cargo xtask schema` and commit the result");
    }

    println!("schemas written: {written} changed, {removed} no longer generated");
    Ok(())
}

/// The index of `schemas/generated/`.
fn schema_index() -> String {
    let mut out = String::from(
        "# Generated schemas\n\n**Do not edit these files.** They are generated from the Rust \
         types by `cargo xtask schema`, and CI\nfails if they differ from what the types \
         produce.\n\nThey are the interoperability contract: anything that produces or consumes \
         these documents can\nvalidate them without linking the Rust crates.\n\n| file | Rust type \
         | describes |\n| --- | --- | --- |\n",
    );
    for entry in aep_schema::generated_schemas() {
        let _ = writeln!(
            out,
            "| [`{}`]({}) | `{}` | {} |",
            entry.filename, entry.filename, entry.name, entry.describes
        );
    }
    out
}

/// Formats (or checks) exactly the source workspace's members.
///
/// The member list comes from Cargo metadata so the check has one explicit workspace boundary.
fn fmt(check: bool) -> Result<()> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = std::process::Command::new(&cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .with_context(|| format!("running {cargo:?} metadata"))?;
    if !output.status.success() {
        bail!(
            "reading the workspace members failed:
{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parsing `cargo metadata` output")?;
    let mut arguments: Vec<String> = vec!["fmt".to_owned()];
    let packages = metadata["packages"]
        .as_array()
        .context("cargo metadata returned no packages array")?;
    for package in packages {
        arguments.push("--package".to_owned());
        arguments.push(
            package["name"]
                .as_str()
                .context("a cargo metadata package has no name")?
                .to_owned(),
        );
    }
    if check {
        arguments.push("--".to_owned());
        arguments.push("--check".to_owned());
    }
    let status = std::process::Command::new(&cargo)
        .args(&arguments)
        .current_dir(workspace_root())
        .status()
        .with_context(|| format!("running {cargo:?} fmt over the workspace members"))?;
    if !status.success() {
        bail!("formatting {}", if check { "differs" } else { "failed" });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{delivered_waves, schema, splice_generated};

    #[test]
    fn the_delivered_waves_table_keeps_the_tags_in_the_order_given() {
        let table = delivered_waves(&[("0.1.0", "domain model"), ("0.2.0", "execution core")]);
        assert_eq!(
            table,
            "| tag | delivered |\n|---|---|\n| `0.1.0` | domain model |\n| `0.2.0` | execution core |\n"
        );
    }

    #[test]
    fn a_pipe_in_a_tag_subject_cannot_break_the_table() {
        let table = delivered_waves(&[("0.1.0", "left | right")]);
        assert!(table.contains("left \\| right"), "{table}");
    }

    #[test]
    fn the_delivered_waves_table_uses_the_current_repository_name() {
        let table = delivered_waves(&[("0.36.0", "legacy repository 0.36.0")]);
        assert!(table.contains("AEP 0.36.0"), "{table}");
        assert!(!table.contains("legacy repository"), "{table}");
    }

    #[test]
    fn splicing_replaces_only_the_generated_region_and_twice_changes_nothing() {
        let page = "before\n<begin>\nstale\n<end>\nafter\n";
        let once = splice_generated(page, "<begin>", "<end>", "fresh\n")
            .expect("both markers are present");
        assert_eq!(once, "before\n<begin>\nfresh\n<end>\nafter\n");
        let twice = splice_generated(&once, "<begin>", "<end>", "fresh\n")
            .expect("both markers survive a splice");
        assert_eq!(twice, once);
    }

    #[test]
    fn missing_or_reversed_markers_are_refused() {
        assert!(splice_generated("no markers\n", "<begin>", "<end>", "fresh\n").is_err());
        assert!(splice_generated("<end>\n<begin>\n", "<begin>", "<end>", "fresh\n").is_err());
    }

    fn generated(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(name);
        std::fs::remove_dir_all(&root).ok();
        schema(&root, false).expect("the schemas are written");
        root
    }

    #[test]
    fn schema_check_refuses_orphans_and_accepts_the_generated_index() {
        let root = generated("aep-xtask-generated-schema");
        assert!(root.join("schemas/generated/README.md").is_file());
        schema(&root, true).expect("a freshly generated tree is current");

        let orphan = root.join("schemas/generated/obsolete.schema.json");
        std::fs::write(&orphan, "{}\n").expect("the fixture is writable");
        let refusal = schema(&root, true).expect_err("an orphaned schema is drift");
        assert!(format!("{refusal:#}").contains("obsolete.schema.json"));

        schema(&root, false).expect("regeneration removes the orphan");
        assert!(!orphan.exists());
        std::fs::remove_dir_all(&root).ok();
    }
}

#[cfg(test)]
mod currency_tests {
    use super::{
        authoritative_gate_contract, authoritative_gate_text, currency, gate_steps,
        generated_status_region_inventory, hold_region, release_asset_text, splice_generated,
        workspace_root, RELEASE_WORKFLOW, SITE_STATUS_BEGIN, SITE_STATUS_END,
    };

    #[test]
    fn ci_and_release_delegate_to_the_one_gate() {
        let root = workspace_root();
        authoritative_gate_contract(&root).expect("CI invokes the gate and release reuses CI");
        generated_status_region_inventory(&root)
            .expect("every generated volatile status region has this command as its owner");
    }

    #[test]
    fn removing_the_gate_or_its_release_reuse_is_drift() {
        let ci = "steps:\n  - name: Gate\n    run: task check\n";
        let release = "jobs:\n  gate:\n    uses: ./.github/workflows/ci.yml\n";
        authoritative_gate_text(ci, release).expect("the authoritative shape holds");
        let no_gate = ci.replace("task check", "cargo test --workspace");
        assert!(
            authoritative_gate_text(&no_gate, release).is_err(),
            "CI cannot replace the gate with a hand-picked command"
        );
        let copied_release =
            release.replace("uses: ./.github/workflows/ci.yml", "runs-on: ubuntu-latest");
        assert!(
            authoritative_gate_text(ci, &copied_release).is_err(),
            "release verification cannot stop reusing CI's gate"
        );
    }

    #[test]
    fn removing_a_release_target_or_checksum_is_drift() {
        let path = workspace_root().join(RELEASE_WORKFLOW);
        let release = std::fs::read_to_string(path).expect("reading the release workflow");
        release_asset_text(&release).expect("the release asset contract holds");

        let missing_target = release.replace(
            "- target: aarch64-apple-darwin",
            "- target: aarch64-unknown-linux-musl",
        );
        assert!(
            release_asset_text(&missing_target).is_err(),
            "removing Apple Silicon must be detected"
        );

        let missing_checksum = release.replace("sha256sum --check SHA256SUMS", "ls -l");
        assert!(
            release_asset_text(&missing_checksum).is_err(),
            "a checksum file that is never verified must be detected"
        );

        let skipped_backfill = release.replace(
            "always() && needs.provenance.result == 'success' && needs.build.result == 'success'",
            "success()",
        );
        assert!(
            release_asset_text(&skipped_backfill).is_err(),
            "a manual backfill cannot inherit the skipped tag-only gate"
        );
    }

    #[test]
    fn check_mode_refuses_a_changed_generated_region() {
        let root = workspace_root().join("target/xtask-tests/status-region-drift");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("creating the scratch root");
        std::fs::write(
            root.join("page.md"),
            "before\n<begin>\nstale\n<end>\nafter\n",
        )
        .expect("writing the stale page");
        let refusal = hold_region(&root, "page.md", "<begin>", "<end>", "derived\n", true)
            .expect_err("check mode refuses a hand-edited generated region");
        assert!(refusal.to_string().contains("cargo xtask status"));
        std::fs::remove_dir_all(root).ok();
    }

    /// The gate's step list is read from the Taskfile, so a step added there reaches the website
    /// without anybody remembering the website exists.
    ///
    /// Asserted against the real `Taskfile.yml` rather than a fixture: a parser that agrees with a
    /// fixture it was written beside proves nothing about the file it actually reads.
    #[test]
    fn the_gate_step_list_is_read_from_the_taskfile() {
        let steps = gate_steps(&workspace_root()).expect("reading the gate's steps");
        assert!(
            steps.len() > 10,
            "the Taskfile parse found only {steps:?}, so it is reading the wrong block"
        );
        assert_eq!(
            steps.first().map(String::as_str),
            Some("fmt-check"),
            "the first step of `task check` is `fmt-check`; got {steps:?}"
        );
        assert!(
            steps.contains(&"docs-check".to_owned()),
            "`docs-check` is a step of the gate and the list has to carry it: {steps:?}"
        );
        assert!(
            steps.iter().all(|step| !step.contains(':')),
            "a step name carried a colon, so the parse is picking up a YAML key: {steps:?}"
        );
    }

    /// A stamp edited by hand is drift, and `--check` has to see it.
    ///
    /// The rule is load-bearing only when the region's bytes differ, so the fixture edits them —
    /// a test that spliced the same text back would pass whether or not the comparison ran.
    #[test]
    fn a_hand_edited_currency_stamp_no_longer_matches_what_the_tags_derive() {
        let steps = vec!["fmt-check".to_owned(), "website".to_owned()];
        let derived = currency("0.33.0", "2026-08-30", &steps);
        let page =
            format!("# Where this stands\n\n{SITE_STATUS_BEGIN}\n{derived}{SITE_STATUS_END}\n");

        let unchanged = splice_generated(&page, SITE_STATUS_BEGIN, SITE_STATUS_END, &derived)
            .expect("splicing the derived stamp back");
        assert_eq!(
            unchanged, page,
            "re-splicing what is already there must be a no-op"
        );

        let tampered = page.replace("0.33.0", "0.99.0");
        assert_ne!(
            tampered, page,
            "the fixture did not actually change the stamp"
        );
        let repaired = splice_generated(&tampered, SITE_STATUS_BEGIN, SITE_STATUS_END, &derived)
            .expect("splicing over a hand edit");
        assert_ne!(
            repaired, tampered,
            "a hand-edited stamp must differ from what the tags derive, which is what `--check` \
             reports"
        );
        assert_eq!(
            repaired, page,
            "and the repair must land back on the derived text"
        );
    }

    /// The rendered stamp says the count and the list, and they agree with each other.
    #[test]
    fn the_stamp_states_the_step_count_it_lists() {
        let steps = vec![
            "fmt-check".to_owned(),
            "test".to_owned(),
            "website".to_owned(),
        ];
        let rendered = currency("0.33.0", "2026-08-30", &steps);
        assert!(rendered.contains("`0.33.0` (2026-08-30)"), "{rendered}");
        assert!(rendered.contains("**3 steps**"), "{rendered}");
        assert!(
            rendered.contains("`fmt-check`, `test` and `website`"),
            "the list joins with a final `and`: {rendered}"
        );
    }
}
#[cfg(test)]
mod dep_tests {
    use super::{deps, locked_packages, ENTITY_RUNTIME_PREFIX};

    const ONE_PIN: &str = r#"
[[package]]
name = "anyhow"
version = "1.0.99"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "entity-core"
version = "0.9.1"
source = "git+https://github.com/beyond10x/entity-runtime?tag=0.9.1#dc5b25a"
dependencies = [
 "serde",
]

[[package]]
name = "entity-sqlite"
version = "0.9.1"
source = "git+https://github.com/beyond10x/entity-runtime?tag=0.9.1#dc5b25a"

[[package]]
name = "entity-store"
version = "0.9.1"
source = "git+https://github.com/beyond10x/entity-runtime?tag=0.9.1#dc5b25a"

[[package]]
name = "xtask"
version = "0.27.3"
"#;

    /// The lockfile as it stood on 2026-08-28, reduced to the lines that matter.
    const TWO_KERNELS: &str = r#"
[[package]]
name = "entity-core"
version = "0.5.2"
source = "git+https://github.com/beyond10x/entity-runtime?tag=0.5.2#1bfad9f"

[[package]]
name = "entity-core"
version = "0.8.0"
source = "git+https://github.com/beyond10x/entity-runtime?tag=0.8.0#6aa3c59"

[[package]]
name = "entity-store"
version = "0.8.0"
source = "git+https://github.com/beyond10x/entity-runtime?tag=0.8.0#6aa3c59"
"#;

    fn lockfile_in_a_root(lock: &str) -> tempdir::Root {
        let root = tempdir::Root::new("xtask-deps");
        std::fs::write(root.path().join("Cargo.lock"), lock).expect("writing the lockfile");
        root
    }

    #[test]
    fn the_entity_crates_are_read_out_of_the_lockfile_and_nothing_else_is() {
        let packages = locked_packages(ONE_PIN, ENTITY_RUNTIME_PREFIX);
        let names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["entity-core", "entity-sqlite", "entity-store"]);
        assert!(packages.iter().all(|p| p.version == "0.9.1"));
        assert!(packages.iter().all(|p| p.source.contains("tag=0.9.1")));
    }

    #[test]
    fn a_workspace_member_under_the_prefix_reads_as_its_own_source() {
        let lock = "[[package]]\nname = \"entity-local\"\nversion = \"0.1.0\"\n";
        let packages = locked_packages(lock, ENTITY_RUNTIME_PREFIX);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].source, "path");
    }

    #[test]
    fn one_version_from_one_tag_passes() {
        let root = lockfile_in_a_root(ONE_PIN);
        deps(root.path()).expect("one pin is what the rule asks for");
    }

    #[test]
    fn two_versions_of_one_entity_crate_are_refused_and_both_are_named() {
        let root = lockfile_in_a_root(TWO_KERNELS);
        let error = deps(root.path())
            .expect_err("two kernels must not pass")
            .to_string();
        assert!(
            error.contains("entity-core at 0.5.2 and 0.8.0"),
            "the message names both versions: {error}"
        );
        assert!(
            error.contains("two versions"),
            "and says what the finding is: {error}"
        );
    }

    #[test]
    fn two_tags_across_the_entity_crates_are_refused_even_when_no_crate_is_duplicated() {
        let lock = r#"
[[package]]
name = "entity-core"
version = "0.9.1"
source = "git+https://github.com/beyond10x/entity-runtime?tag=0.9.1#dc5b25a"

[[package]]
name = "entity-store"
version = "0.8.0"
source = "git+https://github.com/beyond10x/entity-runtime?tag=0.8.0#6aa3c59"
"#;
        let root = lockfile_in_a_root(lock);
        let error = deps(root.path())
            .expect_err("two pins must not pass")
            .to_string();
        assert!(
            error.contains("more than one `entity-runtime` pin"),
            "the message names the finding: {error}"
        );
        assert!(
            error.contains("tag=0.9.1") && error.contains("tag=0.8.0"),
            "and both pins: {error}"
        );
    }

    #[test]
    fn a_lockfile_with_no_entity_crate_is_a_finding_not_a_pass() {
        let root = lockfile_in_a_root("[[package]]\nname = \"anyhow\"\nversion = \"1\"\n");
        let error = deps(root.path())
            .expect_err("nothing to check is not a pass")
            .to_string();
        assert!(error.contains("no pin to check"), "{error}");
    }

    #[test]
    fn an_ess_modeling_crate_is_refused_even_beside_one_entity_runtime_pin() {
        let lock = format!("{ONE_PIN}\n[[package]]\nname = \"ess-domain\"\nversion = \"0.2.0\"\n");
        let root = lockfile_in_a_root(&lock);
        let error = deps(root.path())
            .expect_err("AEP must not compile against ESS modeling crates")
            .to_string();
        assert!(error.contains("ess-domain"), "{error}");
        assert!(error.contains("standalone report wire"), "{error}");
    }

    #[test]
    fn the_committed_lockfile_passes() {
        // The gate step itself, so a pin that drifts fails `cargo test` as well as `task check`.
        deps(&super::workspace_root()).expect("the committed Cargo.lock names one entity-runtime");
    }

    /// A scratch directory that is removed on drop. Under this repository's `target/`, as the
    /// tag-reachability test does, rather than the system temporary directory.
    mod tempdir {
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        pub struct Root(PathBuf);

        impl Root {
            pub fn new(label: &str) -> Self {
                let n = COUNTER.fetch_add(1, Ordering::Relaxed);
                let path = super::super::workspace_root()
                    .join("target/xtask-tests")
                    .join(format!("{label}-{}-{n}", std::process::id()));
                std::fs::create_dir_all(&path).expect("creating the scratch directory");
                Self(path)
            }

            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}
