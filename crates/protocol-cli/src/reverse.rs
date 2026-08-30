//! `protocol reverse` — reading a repository that already exists into the protocol's own terms.
//!
//! Every other verb in this binary starts from a document somebody wrote. These three start from a
//! repository somebody built, which is the state almost every adopter is actually in: the rules
//! exist, in a `CONTRIBUTING.md` or one engineer's head, and the governed tree does not.
//!
//! # The seam, and why it runs through the middle of adoption
//!
//! Turning a codebase into a plan needs judgement — which of four roadmap stages is one initiative
//! and which is four, whether a disabled CI variable is a story or an accepted cost. Judgement is
//! the model's job and it is not this binary's. What *is* this binary's job is the half underneath:
//! finding what the repository mechanically says about itself, and saying it the same way twice.
//!
//! So `reverse scan` reads and does not interpret. It emits a bundle of located facts — every entry
//! carries the path and line it was read from — and writes nothing. An agent then decides what those
//! facts mean and records the decision through `protocol artifact`, citing entries it did not
//! author. That is the same asymmetry `independent: true` draws on an evidence requirement, one
//! layer up: the thing being judged did not produce the record it is judged against.
//!
//! A verb that called a model to do the interpreting would collapse the seam, and it would put a
//! network call and a credential in a repository whose whole claim is that it holds neither.
//!
//! # Same tree, same bytes
//!
//! `reverse scan` has no clock, no network, no randomness and no `read_dir` order dependence: the
//! walk sorts every directory's entries by name before descending, and the bundle's collections are
//! built in that one order. Two runs over one tree produce identical stdout, which is what lets a
//! bundle be committed, diffed, and cited by an artifact that outlives the session that wrote it.
//!
//! The rule is borrowed from the precedent this repository already set for a scanner of its own,
//! `.engineering/checks/scan-declarations.sh`: **stdout is data and nothing else**, diagnostics go
//! to stderr, and the enumeration order is fixed rather than inherited from the filesystem.
//!
//! # What a scan cannot do, said out loud
//!
//! It reports what is *written down*. A convention that lives only in review comments, a rule
//! everybody follows and nobody typed, an intent behind a module — none of that is here, and a plan
//! built from a bundle alone will be missing exactly those things. The bundle is where an adopting
//! session *starts*, not the set of things worth planning.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use serde::Serialize;
use serde_yaml::Value as Yaml;

use aep_domain::project::{ProtocolSource, PROJECT_FILE, PROJECT_VERSION};
use aep_project::project::project_directory;

use crate::Format;

/// The bundle format `reverse scan` emits, versioned like every other document here.
const BUNDLE_VERSION: &str = "aep.reverse-scan/1";

/// How deep the walk descends before it stops.
///
/// A bound rather than a guess: a repository nested deeper than this is one whose top-level shape is
/// already past what a first plan can usefully cover, and an unbounded walk over a symlinked tree is
/// a hang rather than an error.
const MAX_DEPTH: usize = 12;

/// The largest file the scan will read.
///
/// A checked-in fixture, a minified bundle or a captured audio sample has nothing to say about how
/// the work is organised, and reading one costs more than everything else in the walk together.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// How much of a marked line travels into the bundle.
///
/// Enough to tell two TODOs apart; not so much that a bundle grows a copy of the source. The `path`
/// and `line` are the citation, and a reader who needs the whole sentence opens the file.
const MAX_EXCERPT: usize = 200;

/// Directories the walk never descends into.
///
/// Build output and vendored dependencies, which say what a toolchain did rather than what this
/// repository decided. Dot-directories are skipped by a separate rule, with one exception.
const SKIP_DIRECTORIES: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    "dist",
    "build",
    "out",
    "coverage",
    "__pycache__",
    "site-packages",
    "Pods",
    "DerivedData",
];

/// The dot-directory the walk does descend into.
///
/// Everything else dot-prefixed is machine state — the loader that reads a document tree skips them
/// on the same reasoning. `.github` is the exception because a workflow file there is one of the few
/// places a repository writes down what it considers a gate.
const KEPT_DOT_DIRECTORIES: &[&str] = &[".github"];

/// The words a scan treats as an unfinished-work marker.
const MARKERS: &[&str] = &["TODO", "FIXME", "HACK", "XXX"];

/// The ways a test says it is not going to run.
///
/// One entry per spelling rather than a regex, because the interesting half is the *reason string*
/// beside it and each language puts that in a different place. A framework nobody here listed is a
/// miss and not a wrong answer — `todo_sites` will usually still catch the comment beside it.
const SKIP_MARKERS: &[&str] = &[
    "#[ignore]",
    "@Disabled",
    "@Ignore",
    "@pytest.mark.skip",
    "@unittest.skip",
    "describe.skip(",
    "it.skip(",
    "pytest.skip(",
    "t.Skip(",
    "t.SkipNow(",
    "t.Skipf(",
    "test.skip(",
    "xdescribe(",
    "xit(",
];

/// `ABC-123`-shaped things that are not tracker identifiers.
///
/// Standards and algorithms are spelt exactly like a project key, and a SIP or crypto codebase
/// mentions them constantly: `RFC-3261`, `UTF-8`, `SHA-256`. Listing them is cruder than asking the
/// adopter to configure their key, and it is the right way round — a repository that names its
/// tracker nowhere still mentions it in every second commit, and a tool that must be configured
/// before it can say what it found is a tool nobody gets a first answer out of.
const NOT_TRACKERS: &[&str] = &[
    "AES", "ARM", "ASCII", "CRC", "DES", "GPG", "HTTP", "IEEE", "IPV", "ISO", "MD", "PGP", "RFC",
    "RSA", "SHA", "SSL", "TLS", "UTC", "UTF", "X",
];

/// What makes a skip conditional rather than permanent.
///
/// A skip inside a condition is an opt-in — *run these when the environment has a SIP stack* — and is
/// ordinary. A skip with no condition above it is a test that never runs on any machine, and it is
/// the only one of the two worth a plan item, so the two are told apart rather than counted together.
const GUARDS: &[&str] = &["if ", "if(", "unless ", "when ", "elif ", "else "];

/// Extension to language name, and the set of files the scan will read at all.
///
/// A closed list rather than *everything that is not binary*: an unknown extension is more often a
/// data blob than a language, and counting one as source makes `package_tree` say a fixture
/// directory is the largest component in the system.
const LANGUAGES: &[(&str, &str)] = &[
    ("c", "C"),
    ("cc", "C++"),
    ("clj", "Clojure"),
    ("cpp", "C++"),
    ("cs", "C#"),
    ("css", "CSS"),
    ("dart", "Dart"),
    ("ex", "Elixir"),
    ("exs", "Elixir"),
    ("go", "Go"),
    ("h", "C"),
    ("hpp", "C++"),
    ("hs", "Haskell"),
    ("java", "Java"),
    ("js", "JavaScript"),
    ("jsx", "JavaScript"),
    ("kt", "Kotlin"),
    ("lua", "Lua"),
    ("m", "Objective-C"),
    ("mjs", "JavaScript"),
    ("php", "PHP"),
    ("proto", "Protocol Buffers"),
    ("py", "Python"),
    ("rb", "Ruby"),
    ("rs", "Rust"),
    ("scala", "Scala"),
    ("sh", "Shell"),
    ("sql", "SQL"),
    ("swift", "Swift"),
    ("tf", "Terraform"),
    ("ts", "TypeScript"),
    ("tsx", "TypeScript"),
    ("vue", "Vue"),
    ("zig", "Zig"),
];

/// Operations supported by `protocol reverse`.
#[derive(Debug, Subcommand)]
pub(crate) enum ReverseCommand {
    /// Read a repository and report what it says about itself, without writing anything.
    Scan(ScanArgs),
    /// Write the `project.yaml` that makes a repository an adopting project.
    Init(InitArgs),
    /// Draft an `ess/1` domain from an `OpenAPI` document that already exists.
    Openapi(OpenapiArgs),
    /// Read what the repository's own history says, without writing anything.
    History(HistoryArgs),
}

/// Inputs for a scan.
#[derive(Debug, Args)]
pub(crate) struct ScanArgs {
    /// The repository to read. The current directory when absent.
    #[arg(default_value = ".")]
    root: PathBuf,
    /// How to render the bundle.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

/// Inputs for writing a project file.
#[derive(Debug, Args)]
pub(crate) struct InitArgs {
    /// The repository to adopt. The current directory when absent.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Where the governing documents come from: a path, or a pinned `git+…#<40-hex>` locator.
    #[arg(long)]
    protocols: String,
    /// The profile whose rules apply, such as `development.standard`.
    #[arg(long)]
    profile: String,
    /// The protocol the project runs under.
    #[arg(long, default_value = "adp/1")]
    protocol: String,
    /// One line on what this project is. Written into the file for a human, read by nothing.
    #[arg(long)]
    summary: Option<String>,
    /// Write the file without resolving the protocol source first.
    ///
    /// For an offline machine, or a source that is not reachable yet. The file is written unverified
    /// and the first command that needs the tree is where the failure surfaces instead.
    #[arg(long)]
    no_verify: bool,
}

/// Inputs for reading a repository's history.
#[derive(Debug, Args)]
pub(crate) struct HistoryArgs {
    /// The repository to read. The current directory when absent.
    #[arg(default_value = ".")]
    root: PathBuf,
    /// How many of the most recent commits count as *recent* for dormancy.
    ///
    /// A count and not a duration, because a repository's tempo is its own: 500 commits is six weeks
    /// in one tree and four years in another, and the question being asked — *has anybody been here
    /// lately* — is about the work, not the calendar.
    #[arg(long, default_value_t = 500)]
    recent: usize,
    /// How many entries each ranked section reports.
    #[arg(long, default_value_t = 15)]
    top: usize,
    /// How to render the report.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

/// Inputs for drafting a domain from an `OpenAPI` document.
#[derive(Debug, Args)]
pub(crate) struct OpenapiArgs {
    /// The `OpenAPI` document, YAML or JSON.
    path: PathBuf,
    /// The domain name to declare, such as `acme.billing`.
    #[arg(long)]
    domain: String,
    /// Write the draft here. Standard output when absent.
    #[arg(long)]
    out: Option<PathBuf>,
}

/// Runs one `protocol reverse` operation.
pub(crate) fn run(command: ReverseCommand) -> Result<ExitCode> {
    match command {
        ReverseCommand::Scan(args) => scan(&args),
        ReverseCommand::Init(args) => init(&args),
        ReverseCommand::Openapi(args) => openapi(&args),
        ReverseCommand::History(args) => history(&args),
    }
}

// ---------------------------------------------------------------------------------------------
// The walk
// ---------------------------------------------------------------------------------------------

/// One readable file the walk found.
#[derive(Debug)]
struct FileEntry {
    /// Path relative to the scanned root, always `/`-separated so a bundle reads the same on any
    /// platform and a citation can be pasted into an editor.
    rel: String,
    /// Where to actually read it.
    abs: PathBuf,
    /// Size in bytes, used to decide whether the file is read at all.
    size: u64,
}

impl FileEntry {
    /// The lowercased extension, or the empty string.
    fn extension(&self) -> String {
        Path::new(&self.rel)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
    }

    /// The file name, or the empty string.
    fn name(&self) -> &str {
        self.rel.rsplit('/').next().unwrap_or_default()
    }

    /// How deep below the root it sits: `0` for a file in the root itself.
    fn depth(&self) -> usize {
        self.rel.matches('/').count()
    }

    /// The top-level directory it lives under, or `None` for a file in the root.
    fn top_level(&self) -> Option<&str> {
        self.rel.split_once('/').map(|(head, _)| head)
    }

    /// The file's text, or `None` when it is too large or is not UTF-8.
    ///
    /// Not being UTF-8 is an ordinary outcome for a repository — an audio fixture, a compiled
    /// artifact somebody committed — so it is a skip and not a diagnostic.
    fn text(&self) -> Option<String> {
        if self.size > MAX_FILE_BYTES {
            return None;
        }
        fs::read(&self.abs)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }
}

/// Every readable file under a root, in one fixed order.
#[derive(Debug)]
struct Tree {
    /// The files, sorted by their relative path.
    files: Vec<FileEntry>,
    /// How many files were passed over for being larger than [`MAX_FILE_BYTES`].
    oversized: usize,
}

/// Walks `root`, collecting files in a deterministic order.
///
/// Directory entries are sorted by name before the walk descends, so the resulting order is a
/// property of the tree rather than of the filesystem that stored it. Symlinks are not followed:
/// a link out of the repository would put another project's paths in the bundle, and a link back
/// into it would be a cycle.
fn walk(root: &Path) -> Result<Tree> {
    let mut files = Vec::new();
    let mut oversized = 0;
    walk_into(root, root, 0, &mut files, &mut oversized)?;
    files.sort_by(|left, right| left.rel.cmp(&right.rel));
    Ok(Tree { files, oversized })
}

/// One directory's worth of the walk.
fn walk_into(
    root: &Path,
    directory: &Path,
    depth: usize,
    files: &mut Vec<FileEntry>,
    oversized: &mut usize,
) -> Result<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }

    let mut entries: Vec<PathBuf> = match fs::read_dir(directory) {
        Ok(reader) => reader
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect(),
        // An unreadable directory is a fact about permissions, not about the repository's plan.
        // Reporting it on stderr keeps stdout a clean bundle, which is the property callers pipe.
        Err(error) => {
            eprintln!("reverse scan: cannot read {}: {error}", directory.display());
            return Ok(());
        }
    };
    entries.sort();

    for path in entries {
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            if skip_directory(name) {
                continue;
            }
            walk_into(root, &path, depth + 1, files, oversized)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        let Some(rel) = relative(root, &path) else {
            continue;
        };
        let size = metadata.len();
        if size > MAX_FILE_BYTES {
            *oversized += 1;
        }
        files.push(FileEntry {
            rel,
            abs: path,
            size,
        });
    }

    Ok(())
}

/// Whether the walk refuses to descend into a directory of this name.
fn skip_directory(name: &str) -> bool {
    if SKIP_DIRECTORIES.contains(&name) {
        return true;
    }
    name.starts_with('.') && !KEPT_DOT_DIRECTORIES.contains(&name)
}

/// `path` relative to `root`, `/`-separated.
fn relative(root: &Path, path: &Path) -> Option<String> {
    let stripped = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in stripped.components() {
        parts.push(component.as_os_str().to_str()?.to_owned());
    }
    Some(parts.join("/"))
}

// ---------------------------------------------------------------------------------------------
// The bundle
// ---------------------------------------------------------------------------------------------

/// What one repository mechanically says about itself.
///
/// Every collection is sorted, and every entry that could carry a citation does. An entry without a
/// `path` is one nothing in the tree could be pointed at for, and there are none.
#[derive(Debug, Serialize)]
struct ScanBundle {
    /// The bundle format, so a consumer can refuse a shape it does not know.
    version: &'static str,
    /// Headings of every `README` in the tree: the closest thing a repository has to a stated intent.
    readme_outline: Vec<Heading>,
    /// Unfinished work the code admits to, one entry per marked line.
    todo_sites: Vec<TodoSite>,
    /// Tests that say they are not going to run, and whether anything can turn them back on.
    disabled_tests: Vec<DisabledTest>,
    /// Jobs a CI definition declares, with the variables set on each.
    ci_jobs: Vec<CiJob>,
    /// Targets a task runner declares: what somebody can actually run.
    task_targets: Vec<TaskTarget>,
    /// Top-level source directories, with how much is in each.
    package_tree: Vec<PackageDirectory>,
    /// Interface documents that already exist, and how much surface each describes.
    api_surfaces: Vec<ApiSurface>,
    /// Markdown at the repository root that is not a README — where loose plans and notes land.
    root_docs: Vec<RootDoc>,
}

/// One heading in a README.
#[derive(Debug, Serialize)]
struct Heading {
    /// Where it was read.
    path: String,
    /// Which line it is on, 1-based.
    line: usize,
    /// How many `#` characters opened it.
    level: usize,
    /// The heading text, with the `#` characters and surrounding space removed.
    text: String,
}

/// One line that admits to unfinished work.
#[derive(Debug, Serialize)]
struct TodoSite {
    /// Where it was read.
    path: String,
    /// Which line it is on, 1-based.
    line: usize,
    /// Which of [`MARKERS`] appeared.
    marker: String,
    /// The line, trimmed and truncated to [`MAX_EXCERPT`].
    text: String,
}

/// One test that declares it will not run.
#[derive(Debug, Serialize)]
struct DisabledTest {
    /// Where it was read.
    path: String,
    /// Which line it is on, 1-based.
    line: usize,
    /// The spelling that was matched, such as `t.Skip(`.
    marker: String,
    /// Whether a condition above it can still turn the test on.
    ///
    /// `false` is the finding. A guarded skip is an opt-in; an unguarded one is a test that runs on
    /// no machine, and no pipeline reports it as anything other than a pass.
    guarded: bool,
    /// The reason the skip states, when it states one in a quoted string.
    reason: Option<String>,
}

/// One job a CI definition declares.
#[derive(Debug, Serialize)]
struct CiJob {
    /// Where it was read.
    path: String,
    /// The line the job's key is on, 1-based.
    line: usize,
    /// The job's name.
    name: String,
    /// The variables set on the job, sorted by key.
    ///
    /// The interesting ones are usually the switches: a suite turned off here is a decision nobody
    /// wrote a ticket for, and it is the single most productive thing a scan finds.
    variables: BTreeMap<String, String>,
}

/// One target a task runner declares.
#[derive(Debug, Serialize)]
struct TaskTarget {
    /// Where it was read.
    path: String,
    /// The line the target's key is on, 1-based.
    line: usize,
    /// The target's name.
    name: String,
    /// Its stated purpose, when the runner has a field for one.
    summary: Option<String>,
}

/// One top-level directory that holds source.
#[derive(Debug, Serialize)]
struct PackageDirectory {
    /// The directory, relative to the root.
    path: String,
    /// How many source files are under it.
    files: usize,
    /// How many lines those files hold.
    lines: usize,
    /// The languages present, largest first, then by name.
    languages: Vec<LanguageCount>,
}

/// How much of one language sits in one directory.
#[derive(Debug, Serialize)]
struct LanguageCount {
    /// The language's name.
    language: String,
    /// How many files.
    files: usize,
    /// How many lines.
    lines: usize,
}

/// One interface document that already exists.
#[derive(Debug, Serialize)]
struct ApiSurface {
    /// Where it was read.
    path: String,
    /// The line the surface's own header is on, 1-based, or 1 when it has none.
    line: usize,
    /// What kind of document it is: `openapi`, `asyncapi` or `protobuf`.
    kind: String,
    /// The title it gives itself, when it gives one.
    title: Option<String>,
    /// The version it gives itself, when it gives one.
    version: Option<String>,
    /// How many operations, channels or RPCs it describes.
    operations: usize,
}

/// One markdown document at the repository root that is not a README.
#[derive(Debug, Serialize)]
struct RootDoc {
    /// Where it was read.
    path: String,
    /// How many lines it holds.
    lines: usize,
    /// Its first level-one heading, when it has one.
    title: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// `protocol reverse scan`
// ---------------------------------------------------------------------------------------------

/// Reads a repository and prints what it says about itself.
fn scan(args: &ScanArgs) -> Result<ExitCode> {
    let root = args
        .root
        .canonicalize()
        .with_context(|| format!("cannot read {}", args.root.display()))?;
    if !root.is_dir() {
        bail!("{} is not a directory", args.root.display());
    }

    let tree = walk(&root)?;
    if tree.oversized > 0 {
        eprintln!(
            "reverse scan: {} file(s) larger than {MAX_FILE_BYTES} bytes were not read",
            tree.oversized
        );
    }

    let bundle = ScanBundle {
        version: BUNDLE_VERSION,
        readme_outline: readme_outline(&tree),
        todo_sites: todo_sites(&tree),
        disabled_tests: disabled_tests(&tree),
        ci_jobs: ci_jobs(&tree),
        task_targets: task_targets(&tree),
        package_tree: package_tree(&tree),
        api_surfaces: api_surfaces(&tree),
        root_docs: root_docs(&tree),
    };

    match args.format {
        Format::Json => outln!("{}", serde_json::to_string_pretty(&bundle)?),
        Format::Yaml => out!("{}", serde_yaml::to_string(&bundle)?),
        Format::Text => render_bundle(&bundle),
    }
    Ok(ExitCode::SUCCESS)
}

/// Prints a bundle for a person rather than for a program.
fn render_bundle(bundle: &ScanBundle) {
    outln!("{BUNDLE_VERSION}");
    outln!();

    section("readme headings", bundle.readme_outline.len());
    for heading in &bundle.readme_outline {
        outln!(
            "  {}:{}  {}{}",
            heading.path,
            heading.line,
            "  ".repeat(heading.level.saturating_sub(1)),
            heading.text
        );
    }

    section("unfinished work", bundle.todo_sites.len());
    for site in &bundle.todo_sites {
        outln!(
            "  {}:{}  {} {}",
            site.path,
            site.line,
            site.marker,
            site.text
        );
    }

    section("disabled tests", bundle.disabled_tests.len());
    for test in &bundle.disabled_tests {
        outln!(
            "  {}:{}  {}{}{}",
            test.path,
            test.line,
            test.marker,
            if test.guarded {
                "  [guarded]"
            } else {
                "  [NEVER RUNS]"
            },
            test.reason
                .as_ref()
                .map(|reason| format!("  {reason}"))
                .unwrap_or_default()
        );
    }

    section("ci jobs", bundle.ci_jobs.len());
    for job in &bundle.ci_jobs {
        outln!("  {}:{}  {}", job.path, job.line, job.name);
        for (key, value) in &job.variables {
            outln!("      {key} = {value}");
        }
    }

    section("task targets", bundle.task_targets.len());
    for target in &bundle.task_targets {
        match &target.summary {
            Some(summary) => outln!(
                "  {}:{}  {}  — {summary}",
                target.path,
                target.line,
                target.name
            ),
            None => outln!("  {}:{}  {}", target.path, target.line, target.name),
        }
    }

    section("packages", bundle.package_tree.len());
    for package in &bundle.package_tree {
        let languages: Vec<String> = package
            .languages
            .iter()
            .map(|count| format!("{} {}", count.language, count.lines))
            .collect();
        outln!(
            "  {}  {} file(s), {} line(s)  [{}]",
            package.path,
            package.files,
            package.lines,
            languages.join(", ")
        );
    }

    section("api surfaces", bundle.api_surfaces.len());
    for surface in &bundle.api_surfaces {
        outln!(
            "  {}:{}  {} — {} operation(s){}",
            surface.path,
            surface.line,
            surface.kind,
            surface.operations,
            surface
                .title
                .as_ref()
                .map(|title| format!("  ({title})"))
                .unwrap_or_default()
        );
    }

    section("root documents", bundle.root_docs.len());
    for doc in &bundle.root_docs {
        match &doc.title {
            Some(title) => outln!("  {}  {} line(s)  {title}", doc.path, doc.lines),
            None => outln!("  {}  {} line(s)", doc.path, doc.lines),
        }
    }
}

/// One heading in the text rendering, printed whether or not the section found anything.
///
/// An empty section is a finding: a repository with no task targets and no CI jobs has nothing a
/// plan can call a gate, and hiding the zero would make that read as *not checked*.
fn section(name: &str, count: usize) {
    outln!("{name}: {count}");
}

// ---------------------------------------------------------------------------------------------
// Extractors
// ---------------------------------------------------------------------------------------------

/// Headings from every README in the tree.
///
/// Fenced blocks are skipped, because a `# comment` in a shell example is not a section of the
/// document and a plan built from one would invent a heading nobody wrote.
fn readme_outline(tree: &Tree) -> Vec<Heading> {
    let mut headings = Vec::new();
    for file in &tree.files {
        let name = file.name().to_ascii_lowercase();
        if !(name.starts_with("readme") && file.extension() == "md") {
            continue;
        }
        let Some(text) = file.text() else { continue };
        for (line, content) in outside_fences(&text) {
            let trimmed = content.trim_start();
            if !trimmed.starts_with('#') {
                continue;
            }
            let level = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            if level > 6 {
                continue;
            }
            let rest = trimmed[level..].trim();
            if rest.is_empty() {
                continue;
            }
            headings.push(Heading {
                path: file.rel.clone(),
                line,
                level,
                text: truncate(rest),
            });
        }
    }
    headings
}

/// Every line in a source file that carries one of [`MARKERS`].
///
/// The marker must stand on its own: the character before it may not be alphanumeric, `_` or `.`,
/// and the character after may not be alphanumeric or `_`. Without that rule Go's own
/// `context.TODO()` — a legitimate call, used hundreds of times in a large service — reads as
/// hundreds of pieces of unfinished work, and the one bundle entry that matters is buried.
fn todo_sites(tree: &Tree) -> Vec<TodoSite> {
    let mut sites = Vec::new();
    for file in &tree.files {
        if language_for(&file.extension()).is_none() {
            continue;
        }
        let Some(text) = file.text() else { continue };
        for (line, content) in text.lines().enumerate() {
            for marker in MARKERS {
                if let Some(column) = standalone_marker(content, marker) {
                    let _ = column;
                    sites.push(TodoSite {
                        path: file.rel.clone(),
                        line: line + 1,
                        marker: (*marker).to_owned(),
                        text: truncate(content.trim()),
                    });
                    break;
                }
            }
        }
    }
    sites
}

/// Where `marker` appears in `content` as a word of its own, if it does.
fn standalone_marker(content: &str, marker: &str) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut from = 0;
    while let Some(offset) = content[from..].find(marker) {
        let start = from + offset;
        let end = start + marker.len();
        let before_ok = start == 0 || {
            let previous = bytes[start - 1];
            !(previous.is_ascii_alphanumeric() || previous == b'_' || previous == b'.')
        };
        // The trailing boundary applies only when the marker *ends* in an identifier character.
        // `TODO` must not match inside `TODOS`, so it is checked; `t.Skip(` ends at an open paren
        // and its next character is the first argument, so checking it would reject
        // `t.Skip(fmt.Sprintf(…))` — a real disabled test — while accepting the bare `t.Skip()`
        // beside it. The leading boundary is what keeps `exit(` from matching `xit(`, and it always
        // applies.
        let marker_ends_in_word = marker
            .as_bytes()
            .last()
            .is_some_and(|last| last.is_ascii_alphanumeric() || *last == b'_');
        let after_ok = !marker_ends_in_word || end >= bytes.len() || {
            let next = bytes[end];
            !(next.is_ascii_alphanumeric() || next == b'_')
        };
        if before_ok && after_ok {
            return Some(start);
        }
        from = end;
    }
    None
}

/// Every test in the tree that declares it will not run.
///
/// The guarded/unguarded split is the whole point. A test skipped behind an environment check is an
/// opt-in and appears in every healthy repository; a test skipped unconditionally is one nobody has
/// run since the day it was switched off, and a green pipeline reports it exactly the same way. The
/// second is a plan item and the first is not, so a scan that returned one list would bury it.
fn disabled_tests(tree: &Tree) -> Vec<DisabledTest> {
    let mut found = Vec::new();
    for file in &tree.files {
        if language_for(&file.extension()).is_none() {
            continue;
        }
        let Some(text) = file.text() else { continue };
        let lines: Vec<&str> = text.lines().collect();
        for (index, content) in lines.iter().enumerate() {
            // `standalone_marker` and not `contains`, for the reason it was written for one line
            // down: `exit(` contains `xit(`, and a scan that matched the substring reported every
            // process exit in the tree as a disabled test. The boundary rule is the same one that
            // keeps `context.TODO()` out of `todo_sites`.
            let Some(marker) = SKIP_MARKERS
                .iter()
                .find(|marker| standalone_marker(content, marker).is_some())
            else {
                continue;
            };
            found.push(DisabledTest {
                path: file.rel.clone(),
                line: index + 1,
                marker: (*marker).to_owned(),
                guarded: guarded_here(&lines, index),
                reason: quoted_reason(content),
            });
        }
    }
    found
}

/// Whether a condition stands between this line and the test running.
///
/// The line itself first — `if testing.Short() { t.Skip(…) }` is one line — then the two above it,
/// which covers the ordinary block form. Deliberately shallow: a guard six lines up is a guard this
/// misses, and the cost of that miss is one entry marked as never running when something could still
/// turn it on. That is the direction to be wrong in — the alternative is calling a dead test alive.
fn guarded_here(lines: &[&str], index: usize) -> bool {
    let start = index.saturating_sub(2);
    lines[start..=index].iter().any(|line| {
        let trimmed = line.trim_start();
        GUARDS.iter().any(|guard| trimmed.starts_with(guard))
            || (line.contains("if ") && line.contains('{'))
    })
}

/// The first double-quoted string on the line, which is where every listed spelling puts its reason.
fn quoted_reason(content: &str) -> Option<String> {
    let opening = content.find('"')?;
    let rest = &content[opening + 1..];
    let closing = rest.find('"')?;
    let reason = &rest[..closing];
    if reason.is_empty() {
        return None;
    }
    Some(truncate(reason))
}

/// Jobs from every CI definition the tree holds.
fn ci_jobs(tree: &Tree) -> Vec<CiJob> {
    /// Top-level GitLab keys that configure the pipeline rather than declaring a job.
    const GITLAB_RESERVED: &[&str] = &[
        "stages",
        "variables",
        "include",
        "default",
        "workflow",
        "image",
        "services",
        "before_script",
        "after_script",
        "cache",
        "pages",
    ];

    let mut jobs = Vec::new();
    for file in &tree.files {
        let extension = file.extension();
        if extension != "yml" && extension != "yaml" {
            continue;
        }
        let name = file.name();
        let Some(text) = file.text() else { continue };
        let Ok(document) = serde_yaml::from_str::<Yaml>(&text) else {
            continue;
        };
        let Some(mapping) = document.as_mapping() else {
            continue;
        };

        if name == ".gitlab-ci.yml" || name == ".gitlab-ci.yaml" {
            for (key, value) in mapping {
                let Some(key) = key.as_str() else { continue };
                if key.starts_with('.') || GITLAB_RESERVED.contains(&key) {
                    continue;
                }
                if !value.is_mapping() {
                    continue;
                }
                jobs.push(CiJob {
                    path: file.rel.clone(),
                    line: key_line(&text, key, 0),
                    name: key.to_owned(),
                    variables: scalar_map(value.get("variables")),
                });
            }
        } else if file.rel.starts_with(".github/workflows/") {
            let Some(declared) = mapping.get(Yaml::from("jobs")).and_then(Yaml::as_mapping) else {
                continue;
            };
            for (key, value) in declared {
                let Some(key) = key.as_str() else { continue };
                jobs.push(CiJob {
                    path: file.rel.clone(),
                    line: key_line(&text, key, 2),
                    name: key.to_owned(),
                    variables: scalar_map(value.get("env")),
                });
            }
        }
    }
    jobs.sort_by(|left, right| (&left.path, &left.name).cmp(&(&right.path, &right.name)));
    jobs
}

/// Targets from every task runner definition the tree holds.
fn task_targets(tree: &Tree) -> Vec<TaskTarget> {
    let mut targets = Vec::new();
    for file in &tree.files {
        let name = file.name();
        let Some(text) = file.text() else { continue };

        if name == "Taskfile.yml" || name == "Taskfile.yaml" {
            let Ok(document) = serde_yaml::from_str::<Yaml>(&text) else {
                continue;
            };
            let Some(declared) = document.get("tasks").and_then(Yaml::as_mapping) else {
                continue;
            };
            for (key, value) in declared {
                let Some(key) = key.as_str() else { continue };
                targets.push(TaskTarget {
                    path: file.rel.clone(),
                    line: key_line(&text, key, 2),
                    name: key.to_owned(),
                    summary: value
                        .get("desc")
                        .and_then(Yaml::as_str)
                        .map(|summary| truncate(summary.trim())),
                });
            }
        } else if name == "Makefile" || name == "makefile" || name == "GNUmakefile" {
            for (index, content) in text.lines().enumerate() {
                let Some((head, _)) = content.split_once(':') else {
                    continue;
                };
                if head.is_empty() || head.starts_with('.') || head.starts_with(['\t', ' ', '#']) {
                    continue;
                }
                if !head.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "._-/$()%".contains(character)
                }) {
                    continue;
                }
                targets.push(TaskTarget {
                    path: file.rel.clone(),
                    line: index + 1,
                    name: head.trim().to_owned(),
                    summary: None,
                });
            }
        }
    }
    targets.sort_by(|left, right| (&left.path, &left.name).cmp(&(&right.path, &right.name)));
    targets
}

/// Top-level directories that hold source, with per-language counts.
fn package_tree(tree: &Tree) -> Vec<PackageDirectory> {
    let mut totals: BTreeMap<String, BTreeMap<String, (usize, usize)>> = BTreeMap::new();
    for file in &tree.files {
        let Some(language) = language_for(&file.extension()) else {
            continue;
        };
        let Some(directory) = file.top_level() else {
            continue;
        };
        let Some(text) = file.text() else { continue };
        let entry = totals
            .entry(directory.to_owned())
            .or_default()
            .entry(language.to_owned())
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 += text.lines().count();
    }

    let mut packages: Vec<PackageDirectory> = totals
        .into_iter()
        .map(|(path, languages)| {
            let mut counts: Vec<LanguageCount> = languages
                .into_iter()
                .map(|(language, (files, lines))| LanguageCount {
                    language,
                    files,
                    lines,
                })
                .collect();
            counts.sort_by(|left, right| {
                right
                    .lines
                    .cmp(&left.lines)
                    .then_with(|| left.language.cmp(&right.language))
            });
            PackageDirectory {
                files: counts.iter().map(|count| count.files).sum(),
                lines: counts.iter().map(|count| count.lines).sum(),
                languages: counts,
                path,
            }
        })
        .collect();
    packages.sort_by(|left, right| {
        right
            .lines
            .cmp(&left.lines)
            .then_with(|| left.path.cmp(&right.path))
    });
    packages
}

/// Interface documents the repository already publishes.
fn api_surfaces(tree: &Tree) -> Vec<ApiSurface> {
    let mut surfaces = Vec::new();
    for file in &tree.files {
        let extension = file.extension();

        if extension == "proto" {
            let Some(text) = file.text() else { continue };
            let rpcs = text
                .lines()
                .filter(|line| line.trim_start().starts_with("rpc "))
                .count();
            surfaces.push(ApiSurface {
                path: file.rel.clone(),
                line: 1,
                kind: "protobuf".to_owned(),
                title: None,
                version: None,
                operations: rpcs,
            });
            continue;
        }

        let structured = matches!(extension.as_str(), "yaml" | "yml" | "json");
        if !structured {
            continue;
        }
        let Some(text) = file.text() else { continue };
        // Recognised by what the document declares about itself, not by what somebody named the
        // file. The convention that puts the kind in the *directory* — `generated/openapi/
        // invoice-service.yaml` — is at least as common as the one that puts it in the name, and a
        // name-matching scan reports zero surfaces for a repository that publishes four.
        let Some(kind) = declared_surface_kind(&text) else {
            continue;
        };
        let Ok(document) = parse_structured(&text, &extension) else {
            continue;
        };

        let openapi = kind == "openapi";
        let operations = if openapi {
            count_openapi_operations(&document)
        } else {
            document
                .get("channels")
                .and_then(Yaml::as_mapping)
                .map_or(0, serde_yaml::Mapping::len)
        };
        surfaces.push(ApiSurface {
            path: file.rel.clone(),
            line: key_line(&text, "info", 0),
            kind: kind.to_owned(),
            title: document
                .get("info")
                .and_then(|info| info.get("title"))
                .and_then(Yaml::as_str)
                .map(str::to_owned),
            version: document
                .get("info")
                .and_then(|info| info.get("version"))
                .and_then(Yaml::as_str)
                .map(str::to_owned),
            operations,
        });
    }
    surfaces.sort_by(|left, right| left.path.cmp(&right.path));
    surfaces
}

/// Which interface document a text declares itself to be, if it declares one.
///
/// A top-level `openapi:` or `asyncapi:` key, found by a line scan before anything is parsed: the
/// check has to be cheap because it runs against every structured file in the tree, and a document
/// that declares neither is not one of these no matter what it is called.
fn declared_surface_kind(text: &str) -> Option<&'static str> {
    for content in text.lines() {
        if content.starts_with("openapi:") {
            return Some("openapi");
        }
        if content.starts_with("asyncapi:") {
            return Some("asyncapi");
        }
    }
    None
}

/// Markdown at the root that is not a README.
fn root_docs(tree: &Tree) -> Vec<RootDoc> {
    let mut docs = Vec::new();
    for file in &tree.files {
        if file.depth() != 0 || file.extension() != "md" {
            continue;
        }
        if file.name().to_ascii_lowercase().starts_with("readme") {
            continue;
        }
        let Some(text) = file.text() else { continue };
        let title = outside_fences(&text)
            .into_iter()
            .find_map(|(_, content)| content.strip_prefix("# ").map(|rest| truncate(rest.trim())));
        docs.push(RootDoc {
            path: file.rel.clone(),
            lines: text.lines().count(),
            title,
        });
    }
    docs
}

// ---------------------------------------------------------------------------------------------
// Reading helpers
// ---------------------------------------------------------------------------------------------

/// The document's lines, 1-based, with fenced blocks removed.
fn outside_fences(text: &str) -> Vec<(usize, &str)> {
    let mut lines = Vec::new();
    let mut fence: Option<&str> = None;
    for (index, content) in text.lines().enumerate() {
        let trimmed = content.trim_start();
        let opener = if trimmed.starts_with("```") {
            Some("```")
        } else if trimmed.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        match (fence, opener) {
            (None, Some(marker)) => fence = Some(marker),
            (Some(open), Some(marker)) if open == marker => fence = None,
            (None, None) => lines.push((index + 1, content)),
            _ => {}
        }
    }
    lines
}

/// `text`, cut to [`MAX_EXCERPT`] characters on a character boundary.
fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_EXCERPT {
        return text.to_owned();
    }
    text.chars().take(MAX_EXCERPT).collect::<String>() + "…"
}

/// The language an extension names, if the scan knows it.
fn language_for(extension: &str) -> Option<&'static str> {
    LANGUAGES
        .binary_search_by(|(candidate, _)| (*candidate).cmp(extension))
        .ok()
        .map(|index| LANGUAGES[index].1)
}

/// The 1-based line a YAML key sits on at a given indent, or `1` when nothing matches.
///
/// A line scan and not a parser position, because `serde_yaml` discards spans: the value is a
/// citation for a person to follow, and pointing at the top of the file would be a citation that
/// says nothing. Ambiguity is resolved by taking the first match, which is also what an editor's
/// jump-to-line does.
fn key_line(text: &str, key: &str, indent: usize) -> usize {
    let prefix = " ".repeat(indent);
    let quoted = [
        format!("{prefix}{key}:"),
        format!("{prefix}\"{key}\":"),
        format!("{prefix}'{key}':"),
    ];
    for (index, content) in text.lines().enumerate() {
        if quoted
            .iter()
            .any(|candidate| content.starts_with(candidate))
        {
            return index + 1;
        }
    }
    1
}

/// A YAML mapping of scalars, rendered as strings.
///
/// Anything that is not a scalar is dropped rather than stringified: a nested block rendered into
/// one line would be a value nobody can act on, and the citation already says where to read it.
fn scalar_map(value: Option<&Yaml>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(mapping) = value.and_then(Yaml::as_mapping) else {
        return map;
    };
    for (key, entry) in mapping {
        let Some(key) = key.as_str() else { continue };
        let rendered = match entry {
            Yaml::String(text) => text.clone(),
            Yaml::Bool(flag) => flag.to_string(),
            Yaml::Number(number) => number.to_string(),
            _ => continue,
        };
        map.insert(key.to_owned(), truncate(&rendered));
    }
    map
}

/// Parses a document that may be YAML or JSON into one value type.
///
/// Every JSON document is a YAML document, so the YAML parser reads both and the extension only
/// decides which error message is worth printing.
fn parse_structured(text: &str, extension: &str) -> Result<Yaml> {
    serde_yaml::from_str(text).with_context(|| format!("cannot parse this {extension} document"))
}

/// How many operations an `OpenAPI` document describes.
fn count_openapi_operations(document: &Yaml) -> usize {
    /// The keys under a path item that are operations rather than metadata.
    const METHODS: &[&str] = &[
        "get", "put", "post", "delete", "options", "head", "patch", "trace",
    ];

    document
        .get("paths")
        .and_then(Yaml::as_mapping)
        .map_or(0, |paths| {
            paths
                .values()
                .filter_map(Yaml::as_mapping)
                .map(|item| {
                    item.keys()
                        .filter_map(Yaml::as_str)
                        .filter(|key| METHODS.contains(key))
                        .count()
                })
                .sum()
        })
}

// ---------------------------------------------------------------------------------------------
// `protocol reverse init`
// ---------------------------------------------------------------------------------------------

/// Writes the project file that makes a repository an adopting project.
///
/// The one refusal worth having is the pinning rule, and it is not implemented here:
/// [`ProtocolSource::parse`] already rejects an unpinned `git+` locator and says what would fix it,
/// so this verb calls it and prints what it said. A second copy of that rule in the CLI is a second
/// place for it to drift from the loader that actually enforces it.
fn init(args: &InitArgs) -> Result<ExitCode> {
    let root = args
        .root
        .canonicalize()
        .with_context(|| format!("cannot read {}", args.root.display()))?;
    let directory = root.join(project_directory());
    let file = directory.join(PROJECT_FILE);

    if file.exists() {
        bail!(
            "{} already exists; this project is already adopted",
            file.display()
        );
    }

    let source = ProtocolSource::parse(args.protocols.clone())
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    // The directory is created before the source is checked, and not after, because a relative
    // source is resolved *through* it: `../../tree` from a `.engineering` that does not exist yet
    // fails `is_dir` on the missing component rather than on the tree, and reports a path the
    // adopter did not write. `existed` is what lets a refusal leave the repository as it found it.
    let existed = directory.is_dir();
    fs::create_dir_all(&directory)
        .with_context(|| format!("cannot create {}", directory.display()))?;
    let undo = || {
        if !existed {
            let _ = fs::remove_dir(&directory);
        }
    };

    if let ProtocolSource::Path(path) = &source {
        let resolved = directory.join(path);
        if !resolved.is_dir() {
            undo();
            bail!(
                "the protocol source `{}` resolves to {}, which is not a directory (paths are \
                 resolved from {})",
                path.display(),
                resolved.display(),
                directory.display()
            );
        }
    }

    if let Err(error) = fs::write(&file, project_file(args, &source)) {
        undo();
        return Err(anyhow::Error::from(error).context(format!("cannot write {}", file.display())));
    }

    if args.no_verify {
        outln!("{} written, unverified", file.display());
        outln!("  the protocol source was not resolved; --no-verify was given");
        return Ok(ExitCode::SUCCESS);
    }

    // Written first and checked second, so the check runs against the same file every later command
    // will read rather than against a copy of it. A source that does not resolve leaves no project
    // behind: a half-adopted repository whose every subsequent command fails on the same unreadable
    // tree is worse than one that was never adopted, because it looks adopted.
    match aep_project::project::load_paths(&root) {
        Ok(paths) => {
            outln!("{} written", file.display());
            outln!(
                "  protocol source resolves to {}",
                paths.protocols.display()
            );
            outln!("  profile {}", args.profile);
            Ok(ExitCode::SUCCESS)
        }
        Err(errors) => {
            let _ = fs::remove_file(&file);
            undo();
            bail!(
                "the protocol source did not resolve, so nothing was written:\n{errors}\n\
                 pass --no-verify to write the file anyway"
            )
        }
    }
}

/// The bytes of a new project file.
fn project_file(args: &InitArgs, source: &ProtocolSource) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "version: {PROJECT_VERSION}");
    let _ = writeln!(text);
    let _ = writeln!(
        text,
        "# Written by `protocol reverse init`. It points; it does not duplicate — a rule restated"
    );
    let _ = writeln!(
        text,
        "# here would be a second copy with no way to say which one is in force. Principles and"
    );
    let _ = writeln!(
        text,
        "# profiles of this project's own go under `principles/` and `profiles/` beside this file."
    );
    let _ = writeln!(text, "protocol: {}", args.protocol);
    let _ = writeln!(text, "profile: {}", args.profile);
    let _ = writeln!(text, "protocols: {source}");
    if let Some(summary) = &args.summary {
        let _ = writeln!(text);
        let _ = writeln!(text, "summary: >-");
        for line in wrap(summary, 96) {
            let _ = writeln!(text, "  {line}");
        }
    }
    text
}

/// Wraps `text` at `width` on word boundaries, so a summary reads as a paragraph in the file.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

// ---------------------------------------------------------------------------------------------
// `protocol reverse openapi`
// ---------------------------------------------------------------------------------------------

/// Drafts an `ess/1` domain from an `OpenAPI` document.
///
/// # A draft, and the word is load-bearing
///
/// An `OpenAPI` document says what a service *accepts*. A specification says what the system *is* —
/// entities, their lifecycles, the invariants that hold, and which outcomes a command may refuse
/// with. The first does not contain the second, so nothing here can produce a finished domain and a
/// verb that claimed to would be inviting somebody to commit a specification that describes a wire
/// format rather than a system.
///
/// What it does instead is remove the typing. Types, command names and their inputs are mechanical
/// and are emitted; everything else it can see but cannot decide is emitted as an `UNMAPPED:`
/// comment naming the construct and the choice it is waiting for. A silent omission would be the
/// one failure mode worth ruling out — the reader cannot tell an absent lifecycle from an absent
/// decision about one.
fn openapi(args: &OpenapiArgs) -> Result<ExitCode> {
    let text = fs::read_to_string(&args.path)
        .with_context(|| format!("cannot read {}", args.path.display()))?;
    let extension = args
        .path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("yaml");
    let document = parse_structured(&text, extension)?;

    if document.get("openapi").is_none() {
        bail!(
            "{} declares no `openapi:` version, so it is not an OpenAPI document",
            args.path.display()
        );
    }

    let draft = draft_domain(&args.domain, &document, &args.path);
    match &args.out {
        Some(path) => {
            if path.exists() {
                bail!("{} already exists", path.display());
            }
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("cannot create {}", parent.display()))?;
            }
            fs::write(path, &draft).with_context(|| format!("cannot write {}", path.display()))?;
            outln!("{} written", path.display());
        }
        None => out!("{draft}"),
    }
    Ok(ExitCode::SUCCESS)
}

/// The draft's bytes.
fn draft_domain(domain: &str, document: &Yaml, source: &Path) -> String {
    let mut text = String::new();
    let title = document
        .get("info")
        .and_then(|info| info.get("title"))
        .and_then(Yaml::as_str)
        .unwrap_or(domain);

    let _ = writeln!(
        text,
        "# Drafted by `protocol reverse openapi` from {}.",
        source.display()
    );
    let _ = writeln!(text, "#");
    let _ = writeln!(
        text,
        "# This is a draft and not a specification. An OpenAPI document says what a service accepts;"
    );
    let _ = writeln!(
        text,
        "# a specification says what the system is. Every entity, lifecycle, invariant and refusal"
    );
    let _ = writeln!(
        text,
        "# below is a decision nobody has taken yet — they are marked UNMAPPED, and every one of them"
    );
    let _ = writeln!(
        text,
        "# has to be answered before `protocol ess validate` will accept this file."
    );
    let _ = writeln!(text, "domain: {domain}");
    let _ = writeln!(text);
    let _ = writeln!(text, "summary: {}", yaml_scalar(title));
    let _ = writeln!(text);

    write_types(&mut text, domain, document);
    write_errors(&mut text, domain, document);
    write_commands(&mut text, domain, document);

    let _ = writeln!(text);
    let _ = writeln!(
        text,
        "# UNMAPPED: entities. An OpenAPI schema is a payload shape, and an"
    );
    let _ = writeln!(
        text,
        "# entity is a thing with an identity and a life. Which of the types"
    );
    let _ = writeln!(
        text,
        "# above is an entity, and what its identity field is, is the first"
    );
    let _ = writeln!(text, "# question this draft cannot answer.");
    let _ = writeln!(text, "#");
    let _ = writeln!(
        text,
        "# UNMAPPED: lifecycles. Nothing in an HTTP contract says which states"
    );
    let _ = writeln!(
        text,
        "# an entity holds or which command moves it between them."
    );
    let _ = writeln!(text, "#");
    let _ = writeln!(
        text,
        "# UNMAPPED: invariants. A schema constrains one payload; an invariant"
    );
    let _ = writeln!(
        text,
        "# holds over an entity for its whole life. None can be read off here."
    );
    let _ = writeln!(text, "#");
    let _ = writeln!(
        text,
        "# UNMAPPED: actors. Who may issue each command is a fact about the"
    );
    let _ = writeln!(text, "# organisation, not about the transport.");
    text
}

/// Types projected from `components.schemas`.
fn write_types(text: &mut String, domain: &str, document: &Yaml) {
    let schemas = document
        .get("components")
        .and_then(|components| components.get("schemas"))
        .and_then(Yaml::as_mapping);
    let Some(schemas) = schemas else {
        let _ = writeln!(
            text,
            "# The document declares no `components.schemas`, so there are no types to project."
        );
        return;
    };

    let _ = writeln!(text, "types:");
    for (name, schema) in schemas {
        let Some(name) = name.as_str() else { continue };
        let qualified = format!("{domain}.{}", pascal(name));

        if let Some(variants) = schema.get("enum").and_then(Yaml::as_sequence) {
            let rendered: Vec<String> = variants
                .iter()
                .filter_map(Yaml::as_str)
                .map(pascal)
                .collect();
            let _ = writeln!(text, "  - name: {qualified}");
            let _ = writeln!(text, "    kind: enum");
            let _ = writeln!(text, "    variants: [{}]", rendered.join(", "));
            let _ = writeln!(text);
            continue;
        }

        let declared = schema
            .get("type")
            .and_then(Yaml::as_str)
            .unwrap_or("object");
        if declared == "object" {
            let _ = writeln!(text, "  - name: {qualified}");
            let _ = writeln!(text, "    kind: struct");
            let _ = writeln!(text, "    fields:");
            let properties = schema.get("properties").and_then(Yaml::as_mapping);
            match properties {
                Some(properties) if !properties.is_empty() => {
                    let required = required_set(schema);
                    for (field, definition) in properties {
                        let Some(field) = field.as_str() else {
                            continue;
                        };
                        let mut rendered = ess_type(definition, domain);
                        if !required.contains(&field.to_owned()) {
                            rendered = format!("Optional<{rendered}>");
                        }
                        let _ = writeln!(text, "      - name: {field}");
                        let _ = writeln!(text, "        type: {rendered}");
                    }
                }
                _ => {
                    let _ = writeln!(text, "      # UNMAPPED: the schema declares no properties.");
                }
            }
            let _ = writeln!(text);
        } else {
            let _ = writeln!(text, "  - name: {qualified}");
            let _ = writeln!(text, "    kind: newtype");
            let _ = writeln!(text, "    of: {}", ess_type(schema, domain));
            let _ = writeln!(text);
        }
    }
}

/// One error declaration per refusal code the document uses anywhere.
///
/// Emitted because a `commands:` block that names an error type nothing declares is a draft with a
/// dangling reference, and the first thing `protocol ess validate` says about it would be about
/// this file's own inconsistency rather than about the decisions it is waiting for. The name is the
/// status code, which is the wrong name — an error is a domain fact and `Refused409` is a transport
/// one — and that is exactly why it is marked.
fn write_errors(text: &mut String, domain: &str, document: &Yaml) {
    let codes = refusal_codes(document);
    if codes.is_empty() {
        return;
    }
    let _ = writeln!(text, "errors:");
    let _ = writeln!(
        text,
        "  # UNMAPPED: every name here is a status code, and an error is a domain fact rather than"
    );
    let _ = writeln!(
        text,
        "  # a transport one. Rename each to what actually went wrong, and give it the fields a"
    );
    let _ = writeln!(
        text,
        "  # caller needs in order to do something other than retry."
    );
    for code in codes {
        let name = outcome_name(&code, false);
        let _ = writeln!(text, "  - name: {domain}.{}", pascal(&name));
        let _ = writeln!(
            text,
            "    summary: {}",
            yaml_scalar(&format!("The request was refused with HTTP {code}."))
        );
    }
    let _ = writeln!(text);
}

/// Every non-2xx response code the document declares, sorted and deduplicated.
fn refusal_codes(document: &Yaml) -> Vec<String> {
    /// The keys under a path item that are operations rather than metadata.
    const METHODS: &[&str] = &[
        "get", "put", "post", "delete", "options", "head", "patch", "trace",
    ];

    let mut codes: BTreeSet<String> = BTreeSet::new();
    let Some(paths) = document.get("paths").and_then(Yaml::as_mapping) else {
        return Vec::new();
    };
    for item in paths.values() {
        let Some(item) = item.as_mapping() else {
            continue;
        };
        for method in METHODS {
            let Some(operation) = item.get(Yaml::from(*method)) else {
                continue;
            };
            let Some(responses) = operation.get("responses").and_then(Yaml::as_mapping) else {
                continue;
            };
            for status in responses.keys() {
                let code = match status {
                    Yaml::String(text) => text.clone(),
                    Yaml::Number(number) => number.to_string(),
                    _ => continue,
                };
                if !code.starts_with('2') {
                    codes.insert(code);
                }
            }
        }
    }
    codes.into_iter().collect()
}

/// Commands projected from the document's operations.
fn write_commands(text: &mut String, domain: &str, document: &Yaml) {
    /// The keys under a path item that are operations rather than metadata.
    const METHODS: &[&str] = &[
        "get", "put", "post", "delete", "options", "head", "patch", "trace",
    ];

    let Some(paths) = document.get("paths").and_then(Yaml::as_mapping) else {
        return;
    };

    let _ = writeln!(text, "commands:");
    for (route, item) in paths {
        let Some(route) = route.as_str() else {
            continue;
        };
        let Some(item) = item.as_mapping() else {
            continue;
        };
        for method in METHODS {
            let Some(operation) = item.get(Yaml::from(*method)) else {
                continue;
            };
            let operation_id = operation
                .get("operationId")
                .and_then(Yaml::as_str)
                .map_or_else(|| synthetic_name(method, route), pascal);
            let _ = writeln!(text, "  - name: {domain}.{operation_id}");
            let _ = writeln!(text, "    # {} {route}", method.to_uppercase());
            if let Some(summary) = operation.get("summary").and_then(Yaml::as_str) {
                let _ = writeln!(text, "    summary: {}", yaml_scalar(summary));
            }
            let _ = writeln!(text, "    naming:");
            let _ = writeln!(text, "      wire: {}", kebab(&operation_id));
            let _ = writeln!(text);
            let _ = writeln!(text, "    input:");

            let mut wrote_input = false;
            if let Some(parameters) = operation.get("parameters").and_then(Yaml::as_sequence) {
                for parameter in parameters {
                    let Some(name) = parameter.get("name").and_then(Yaml::as_str) else {
                        continue;
                    };
                    let schema = parameter.get("schema").unwrap_or(&Yaml::Null);
                    let mut rendered = ess_type(schema, domain);
                    if parameter.get("required").and_then(Yaml::as_bool) != Some(true) {
                        rendered = format!("Optional<{rendered}>");
                    }
                    let _ = writeln!(text, "      - name: {name}");
                    let _ = writeln!(text, "        type: {rendered}");
                    wrote_input = true;
                }
            }
            if let Some(reference) = request_body_reference(operation) {
                let _ = writeln!(text, "      - name: body");
                let _ = writeln!(text, "        type: {domain}.{}", pascal(&reference));
                wrote_input = true;
            }
            if !wrote_input {
                let _ = writeln!(
                    text,
                    "      # UNMAPPED: the operation declares no parameters and no body schema."
                );
            }

            let _ = writeln!(text);
            let _ = writeln!(text, "    outcomes:");
            write_outcomes(text, domain, operation);
            let _ = writeln!(text);
        }
    }
}

/// Outcomes projected from an operation's declared responses.
fn write_outcomes(text: &mut String, domain: &str, operation: &Yaml) {
    let responses = operation.get("responses").and_then(Yaml::as_mapping);
    let Some(responses) = responses else {
        let _ = writeln!(
            text,
            "      # UNMAPPED: the operation declares no responses."
        );
        return;
    };

    let mut wrote = false;
    for (status, response) in responses {
        let code = match status {
            Yaml::String(text) => text.clone(),
            Yaml::Number(number) => number.to_string(),
            _ => continue,
        };
        let summary = response
            .get("description")
            .and_then(Yaml::as_str)
            .unwrap_or("");
        let successful = code.starts_with('2');
        if successful {
            let _ = writeln!(text, "      - name: {}", outcome_name(&code, true));
            let _ = writeln!(text, "        # HTTP {code}");
            let _ = writeln!(
                text,
                "        # UNMAPPED: `when:`, `creates:` and `emits:` — which entity this"
            );
            let _ = writeln!(
                text,
                "        # produces, and under what condition, is not in the contract."
            );
            if !summary.is_empty() {
                let _ = writeln!(text, "        summary: {}", yaml_scalar(summary));
            }
        } else {
            let _ = writeln!(text, "      - name: {}", outcome_name(&code, false));
            let _ = writeln!(
                text,
                "        error: {domain}.{}",
                pascal(&outcome_name(&code, false))
            );
            if !summary.is_empty() {
                let _ = writeln!(text, "        summary: {}", yaml_scalar(summary));
            }
        }
        wrote = true;
    }
    if !wrote {
        let _ = writeln!(
            text,
            "      # UNMAPPED: no response could be read as an outcome."
        );
    }
}

/// The set of property names a schema marks required.
fn required_set(schema: &Yaml) -> Vec<String> {
    schema
        .get("required")
        .and_then(Yaml::as_sequence)
        .map(|values| {
            values
                .iter()
                .filter_map(Yaml::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The component name a request body refers to, when it refers to one.
fn request_body_reference(operation: &Yaml) -> Option<String> {
    let content = operation
        .get("requestBody")
        .and_then(|body| body.get("content"))
        .and_then(Yaml::as_mapping)?;
    let media = content.values().next()?;
    let reference = media.get("schema").and_then(|schema| schema.get("$ref"))?;
    reference
        .as_str()
        .and_then(|value| value.rsplit('/').next())
        .map(str::to_owned)
}

/// The `ess/1` type an `OpenAPI` schema projects to.
///
/// A `$ref` becomes the referenced type; a primitive becomes its nearest `ess/1` scalar; anything
/// else becomes `String` with the loss named beside it, because a draft that silently widened a
/// structured field to text would be a draft that lies about what it read.
fn ess_type(schema: &Yaml, domain: &str) -> String {
    if let Some(reference) = schema.get("$ref").and_then(Yaml::as_str) {
        if let Some(name) = reference.rsplit('/').next() {
            return format!("{domain}.{}", pascal(name));
        }
    }
    let declared = schema
        .get("type")
        .and_then(Yaml::as_str)
        .unwrap_or("string");
    let format = schema.get("format").and_then(Yaml::as_str).unwrap_or("");
    match (declared, format) {
        ("string", "date-time") => "Timestamp".to_owned(),
        ("string", "uuid") => "Uuid".to_owned(),
        ("string", "byte" | "binary") => "Bytes".to_owned(),
        ("integer", _) => "Integer".to_owned(),
        ("number", _) => "Decimal".to_owned(),
        ("boolean", _) => "Boolean".to_owned(),
        ("array", _) => {
            let inner = schema
                .get("items")
                .map_or_else(|| "String".to_owned(), |items| ess_type(items, domain));
            format!("List<{inner}>")
        }
        ("object", _) => "Map<String, String>".to_owned(),
        // `string` and everything unrecognised land here together, and deliberately: an unknown
        // `type:` is a construct this projection did not understand, and widening it to text is the
        // same loss as widening a plain string. The `UNMAPPED` notes at the foot of the draft are
        // where that loss is stated; a separate arm here would only repeat the same expression.
        _ => "String".to_owned(),
    }
}

/// A name for an operation the document did not give an `operationId`.
fn synthetic_name(method: &str, route: &str) -> String {
    let mut name = String::from(method);
    for segment in route.split('/') {
        let cleaned: String = segment
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        if !cleaned.is_empty() {
            name.push('-');
            name.push_str(&cleaned);
        }
    }
    pascal(&name)
}

/// A name for one response code.
fn outcome_name(code: &str, successful: bool) -> String {
    if successful {
        format!("accepted-{code}")
    } else {
        format!("refused-{code}")
    }
}

/// `text` in `PascalCase`, splitting on anything that is not alphanumeric.
fn pascal(text: &str) -> String {
    let mut out = String::new();
    for word in text.split(|character: char| !character.is_ascii_alphanumeric()) {
        let mut characters = word.chars();
        if let Some(first) = characters.next() {
            out.extend(first.to_uppercase());
            out.push_str(characters.as_str());
        }
    }
    out
}

/// `text` in `kebab-case`, splitting on case boundaries and non-alphanumerics.
fn kebab(text: &str) -> String {
    let mut out = String::new();
    for (index, character) in text.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 && !out.ends_with('-') {
                out.push('-');
            }
            out.extend(character.to_lowercase());
        } else if character.is_ascii_alphanumeric() {
            out.push(character);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

/// `text` as a YAML scalar that survives a round trip.
fn yaml_scalar(text: &str) -> String {
    let flattened = text.replace(['\n', '\r'], " ");
    let trimmed = flattened.trim();
    serde_yaml::to_string(&trimmed)
        .unwrap_or_else(|_| format!("{trimmed:?}"))
        .trim_start_matches("---")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{kebab, language_for, standalone_marker, truncate, LANGUAGES, MAX_EXCERPT};

    #[test]
    fn the_language_table_is_sorted() {
        // `language_for` reaches it by binary search, so an entry added in the wrong place does not
        // fail loudly — it makes one extension invisible, and the directory written in it silently
        // stops counting as source. The guard is here rather than in a review checklist.
        let mut sorted: Vec<&str> = LANGUAGES.iter().map(|(extension, _)| *extension).collect();
        let given = sorted.clone();
        sorted.sort_unstable();
        assert_eq!(given, sorted, "LANGUAGES must be sorted by extension");
    }

    #[test]
    fn every_listed_extension_is_findable() {
        for (extension, language) in LANGUAGES {
            assert_eq!(language_for(extension), Some(*language));
        }
        assert_eq!(language_for("not-a-language"), None);
    }

    #[test]
    fn a_marker_has_to_stand_on_its_own() {
        assert!(standalone_marker("// TODO: fix this", "TODO").is_some());
        assert!(standalone_marker("# TODO", "TODO").is_some());
        assert!(standalone_marker("TODO at the start", "TODO").is_some());
        assert!(standalone_marker("ctx := context.TODO()", "TODO").is_none());
        assert!(standalone_marker("var TODOS = 3", "TODO").is_none());
        assert!(standalone_marker("myTODO := 1", "TODO").is_none());
        // The awkward one: a qualified call and a real comment on one line. The comment wins,
        // because the second occurrence is found after the first is rejected.
        assert!(standalone_marker("context.TODO() // TODO: give it a real one", "TODO").is_some());
    }

    #[test]
    fn a_call_shaped_marker_is_bounded_only_at_its_front() {
        // `t.Skip(fmt.Sprintf(...))` is a disabled test and its next character is `f`. A trailing
        // boundary would reject it and keep only the argument-free spelling, which is the rarer one.
        assert!(standalone_marker("\t\tt.Skip(fmt.Sprintf(\"off\"))", "t.Skip(").is_some());
        assert!(standalone_marker("\tt.Skip(\"handle later\")", "t.Skip(").is_some());
        // The front boundary still does the work that matters: `exit(` is not `xit(`.
        assert!(standalone_marker("os.exit(1)", "xit(").is_none());
        assert!(standalone_marker("\txit(\"pending\")", "xit(").is_some());
    }

    #[test]
    fn truncation_cuts_on_a_character_boundary() {
        let long = "ä".repeat(MAX_EXCERPT + 10);
        let cut = truncate(&long);
        assert_eq!(
            cut.chars().count(),
            MAX_EXCERPT + 1,
            "the ellipsis is the extra one"
        );
        assert!(cut.ends_with('…'));
        assert_eq!(truncate("short"), "short");
    }

    #[test]
    fn a_timestamp_becomes_the_day_it_names() {
        // Written out rather than taken from a crate, so it is worth pinning against dates a person
        // can check: the epoch, both sides of a leap day, and a turn of the century that is a leap
        // year only because of the 400 rule.
        assert_eq!(super::day(0), "1970-01-01");
        assert_eq!(super::day(86_399), "1970-01-01");
        assert_eq!(super::day(86_400), "1970-01-02");
        assert_eq!(super::day(951_782_400), "2000-02-29");
        assert_eq!(super::day(951_868_800), "2000-03-01");
        assert_eq!(super::day(1_709_164_800), "2024-02-29");
        assert_eq!(super::day(1_751_929_042), "2025-07-07");
    }

    #[test]
    fn a_tracker_key_may_carry_digits_but_a_standard_is_not_a_ticket() {
        use super::tracker_ids;

        // Real project keys carry digits after the first letter. Requiring letters only found none
        // of one repository's 34 identifiers, because every one of them carried a digit.
        assert!(tracker_ids("fix(read): handle empty input WID2-426").contains("WID2-426"));
        assert!(tracker_ids("Refs: ACME-638").contains("ACME-638"));
        assert!(tracker_ids("B2B-7 and API2-1200").contains("B2B-7"));

        // And a codebase that speaks SIP or crypto mentions these constantly.
        for standard in ["RFC-3261", "UTF-8", "SHA-256", "ISO-8601"] {
            assert!(
                tracker_ids(standard).is_empty(),
                "`{standard}` is not a ticket"
            );
        }

        // A key glued to a word is not a key.
        assert!(tracker_ids("xABC-12").is_empty());
        assert!(tracker_ids("ABC-12x").is_empty());
        assert!(tracker_ids("ABC-").is_empty());
    }

    #[test]
    fn a_wire_name_is_kebab_case() {
        assert_eq!(kebab("CreateWidget"), "create-widget");
        assert_eq!(kebab("listWidgets"), "list-widgets");
        assert_eq!(kebab("HTTPServer"), "h-t-t-p-server");
        assert_eq!(kebab("already-kebab"), "already-kebab");
    }
}

// ---------------------------------------------------------------------------------------------
// `protocol reverse history`
// ---------------------------------------------------------------------------------------------

/// The bundle format `reverse history` emits.
const HISTORY_VERSION: &str = "aep.reverse-history/1";

/// Words a commit message uses when it is describing something it expects to undo.
///
/// Each one is a decision with an implied expiry and no mechanism to enforce it. *For now* is the
/// most common and the most durable: `reverse history` exists in large part to put a date beside it.
const EXPIRY_WORDS: &[&str] = &[
    "for now",
    "temporar",
    "workaround",
    "until we",
    "until the",
    "revisit",
    "quick fix",
    "stopgap",
    "band-aid",
];

/// What a repository's own history says about itself.
///
/// The half of a repository `reverse scan` structurally cannot see. A scan reads the tree as it
/// stands, so it can report that a suite is switched off and never that it has been off since
/// February 2024 — and the second is what turns an observation into something anybody acts on.
///
/// Everything here is derived from the commits reachable at `HEAD`. Nothing reads a clock: dates are
/// quoted from the commits that carry them, never compared against today, so a fixed `HEAD` gives
/// fixed output and a bundle stays true after it is committed.
#[derive(Debug, Serialize)]
struct HistoryBundle {
    /// The bundle format, so a consumer can refuse a shape it does not know.
    version: &'static str,
    /// How much history there is, and how it is released.
    span: Span,
    /// The conventional-commit prefixes in use, by count.
    commit_types: Vec<Counted>,
    /// External tracker identifiers the messages mention.
    tickets: Vec<Ticket>,
    /// Commits that undid something, and what they undid.
    reverted: Vec<Commit>,
    /// Commits whose message states an expiry nothing enforces.
    stated_expiry: Vec<Commit>,
    /// The files the work keeps returning to.
    churn: Vec<Churn>,
    /// Source files no recent commit has touched.
    dormant: Vec<String>,
    /// When each line the scan would cite was last written.
    line_ages: Vec<LineAge>,
}

/// How much history there is.
#[derive(Debug, Serialize)]
struct Span {
    /// The first commit's date, `YYYY-MM-DD`.
    first_commit: String,
    /// The last commit's date, `YYYY-MM-DD`.
    last_commit: String,
    /// How many commits are reachable from `HEAD`.
    commits: usize,
    /// How many distinct people authored them.
    ///
    /// A count and not a list. The number answers the question a plan asks — how many people know
    /// this — and a bundle that named them would carry personal data into a file meant to be
    /// committed, diffed and pasted into a ticket.
    authors: usize,
    /// How many tags the repository carries.
    tags: usize,
    /// The most recent tags, newest first.
    latest_tags: Vec<String>,
}

/// One name and how often it occurred.
#[derive(Debug, Serialize)]
struct Counted {
    /// What was counted.
    name: String,
    /// How many times.
    count: usize,
}

/// One external tracker identifier.
#[derive(Debug, Serialize)]
struct Ticket {
    /// The identifier, such as `ABC-123`.
    id: String,
    /// How many commits mention it.
    commits: usize,
    /// The date of the most recent commit that does.
    last_seen: String,
}

/// One commit worth reading.
#[derive(Debug, Serialize)]
struct Commit {
    /// The abbreviated hash.
    hash: String,
    /// The author date, `YYYY-MM-DD`.
    date: String,
    /// The subject line.
    subject: String,
}

/// One file the work keeps returning to.
#[derive(Debug, Serialize)]
struct Churn {
    /// The file, relative to the repository root.
    path: String,
    /// How many commits touched it.
    commits: usize,
    /// How many distinct people authored those commits.
    authors: usize,
    /// The date it was last touched.
    last_touched: String,
}

/// When one cited line was last written.
#[derive(Debug, Serialize)]
struct LineAge {
    /// The file, relative to the repository root.
    path: String,
    /// The line, 1-based. With `path`, this is the join key back into a scan bundle.
    line: usize,
    /// The author date of the commit that last wrote it, `YYYY-MM-DD`.
    ///
    /// Last written, not first introduced — those differ only if somebody edited the line since,
    /// which for a marked comment is uncommon and, when it happens, means the marker was reaffirmed
    /// rather than left. Either way the date is a floor on how long this has been true.
    last_written: String,
    /// The subject of that commit.
    subject: String,
}

/// Reads a repository's history and prints what it says.
fn history(args: &HistoryArgs) -> Result<ExitCode> {
    let root = args
        .root
        .canonicalize()
        .with_context(|| format!("cannot read {}", args.root.display()))?;

    // Asked before anything else, so a tarball, an export or a fresh directory gets one sentence
    // naming what is missing rather than nine empty sections that read like nine findings.
    if git(&root, &["rev-parse", "--is-inside-work-tree"]).is_none() {
        bail!(
            "{} is not a Git working tree, so it has no history to read (`reverse scan` needs none)",
            root.display()
        );
    }
    if git(&root, &["rev-parse", "HEAD"]).is_none() {
        bail!("{} has no commits yet", root.display());
    }
    if git(&root, &["rev-parse", "--is-shallow-repository"])
        .is_some_and(|value| value.trim() == "true")
    {
        eprintln!(
            "reverse history: this is a shallow clone, so every date below is a floor and the \
             oldest is probably wrong — fetch with --unshallow for the real ones"
        );
    }

    let tree = walk(&root)?;
    let bundle = HistoryBundle {
        version: HISTORY_VERSION,
        span: span(&root),
        commit_types: commit_types(&root),
        tickets: tickets(&root, args.top),
        reverted: reverted(&root, args.top),
        stated_expiry: stated_expiry(&root, args.top),
        churn: churn(&root, args.top),
        dormant: dormant(&root, &tree, args.recent),
        line_ages: line_ages(&root, &tree),
    };

    match args.format {
        Format::Json => outln!("{}", serde_json::to_string_pretty(&bundle)?),
        Format::Yaml => out!("{}", serde_yaml::to_string(&bundle)?),
        Format::Text => render_history(&bundle),
    }
    Ok(ExitCode::SUCCESS)
}

/// Runs `git` in `root` and returns its standard output, or `None` when it fails.
///
/// A failure is not an error here. Every caller is asking a question the repository may simply have
/// no answer to — no tags, no commits touching a path, not a work tree at all — and the shape of
/// that answer is an empty section, not a stopped command.
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// The same, split into lines, with the empties dropped.
fn git_lines(root: &Path, args: &[&str]) -> Vec<String> {
    git(root, args)
        .map(|text| {
            text.lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// How much history there is, and how it is released.
fn span(root: &Path) -> Span {
    let dates = git_lines(root, &["log", "--format=%ad", "--date=short"]);
    let mut tags = git_lines(root, &["tag", "--sort=-creatordate"]);
    let authors: BTreeSet<String> = git_lines(root, &["log", "--format=%aE"])
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect();
    let latest = tags.drain(..).take(5).collect();
    Span {
        first_commit: dates.last().cloned().unwrap_or_default(),
        last_commit: dates.first().cloned().unwrap_or_default(),
        commits: dates.len(),
        authors: authors.len(),
        tags: git_lines(root, &["tag"]).len(),
        latest_tags: latest,
    }
}

/// The conventional-commit prefixes in use.
fn commit_types(root: &Path) -> Vec<Counted> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for subject in git_lines(root, &["log", "--format=%s"]) {
        let Some((head, _)) = subject.split_once(':') else {
            continue;
        };
        // `feat(dial)!` and `feat` are the same type; the scope and the breaking marker are not part
        // of what is being counted.
        let kind = head
            .split_once('(')
            .map_or(head, |(before, _)| before)
            .trim_end_matches('!')
            .trim();
        if kind.is_empty() || !kind.chars().all(|c| c.is_ascii_lowercase()) || kind.len() > 12 {
            continue;
        }
        *counts.entry(kind.to_owned()).or_default() += 1;
    }
    ranked(counts)
}

/// External tracker identifiers the messages mention.
fn tickets(root: &Path, top: usize) -> Vec<Ticket> {
    let mut counts: BTreeMap<String, (usize, String)> = BTreeMap::new();

    // NUL-delimited records, not lines. `%B` is the whole message and a message is multi-line, so a
    // line-oriented read splits one commit into several and loses the date from all but the first —
    // and the identifiers that matter most live in a trailer (`Refs: ABC-123`) on the last line. An
    // earlier version of this read lines and reported 2 tickets in a repository that has 34.
    let log = git(root, &["log", "--format=%x00%ad%x09%B", "--date=short"]).unwrap_or_default();
    for record in log.split('\0').skip(1) {
        let Some((date, text)) = record.split_once('\t') else {
            continue;
        };
        for id in tracker_ids(text) {
            // `git log` walks newest first, so the first date seen for an id is its latest.
            let entry = counts.entry(id).or_insert_with(|| (0, date.to_owned()));
            entry.0 += 1;
        }
    }
    let mut found: Vec<Ticket> = counts
        .into_iter()
        .map(|(id, (commits, last_seen))| Ticket {
            id,
            commits,
            last_seen,
        })
        .collect();
    found.sort_by(|left, right| {
        right
            .commits
            .cmp(&left.commits)
            .then_with(|| left.id.cmp(&right.id))
    });
    found.truncate(top);
    found
}

/// Every `ABC-123`-shaped identifier in a message.
///
/// Deliberately shape-based and not a configured project key: a repository that names its tracker
/// nowhere still mentions it in every second commit, and asking an adopter to configure the pattern
/// before the tool can tell them what it found is the wrong way round.
fn tracker_ids(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_uppercase() {
            index += 1;
            continue;
        }
        if index > 0 && (bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_') {
            index += 1;
            continue;
        }
        // The key may carry digits after its first letter — `PV2`, `B2B` and `API2` are all real
        // project keys, and requiring letters only reported none of a repository's 34 identifiers
        // because every one of them carried a digit in the key.
        let start = index;
        while index < bytes.len()
            && (bytes[index].is_ascii_uppercase() || bytes[index].is_ascii_digit())
        {
            index += 1;
        }
        let key = &text[start..index];
        if !(2..=6).contains(&key.len())
            || index >= bytes.len()
            || bytes[index] != b'-'
            || NOT_TRACKERS.contains(&key)
        {
            continue;
        }
        let digits_at = index + 1;
        let mut end = digits_at;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end > digits_at && !(end < bytes.len() && bytes[end].is_ascii_alphanumeric()) {
            found.insert(text[start..end].to_owned());
        }
        index = end.max(index + 1);
    }
    found
}

/// Commits that undid something.
fn reverted(root: &Path, top: usize) -> Vec<Commit> {
    commits_matching(root, top, |subject| {
        let lowered = subject.to_ascii_lowercase();
        lowered.starts_with("revert") || lowered.contains(" revert ")
    })
}

/// Commits whose message states an expiry nothing enforces.
fn stated_expiry(root: &Path, top: usize) -> Vec<Commit> {
    commits_matching(root, top, |subject| {
        let lowered = subject.to_ascii_lowercase();
        EXPIRY_WORDS.iter().any(|word| lowered.contains(word))
    })
}

/// Commits whose subject satisfies a predicate, newest first.
fn commits_matching(root: &Path, top: usize, wanted: impl Fn(&str) -> bool) -> Vec<Commit> {
    let mut found = Vec::new();
    for line in git_lines(root, &["log", "--format=%h%x09%ad%x09%s", "--date=short"]) {
        let mut fields = line.splitn(3, '\t');
        let (Some(hash), Some(date), Some(subject)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if wanted(subject) {
            found.push(Commit {
                hash: hash.to_owned(),
                date: date.to_owned(),
                subject: truncate(subject),
            });
        }
        if found.len() >= top {
            break;
        }
    }
    found
}

/// The files the work keeps returning to.
fn churn(root: &Path, top: usize) -> Vec<Churn> {
    let mut counts: BTreeMap<String, (usize, BTreeSet<String>, String)> = BTreeMap::new();
    let mut date = String::new();
    let mut author = String::new();
    for line in git(
        root,
        &[
            "log",
            "--format=%x00%ad%x09%aE",
            "--date=short",
            "--name-only",
        ],
    )
    .unwrap_or_default()
    .lines()
    {
        if let Some(header) = line.strip_prefix('\0') {
            let mut fields = header.splitn(2, '\t');
            fields.next().unwrap_or_default().clone_into(&mut date);
            author = fields.next().unwrap_or_default().to_ascii_lowercase();
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let entry = counts
            .entry(line.to_owned())
            .or_insert_with(|| (0, BTreeSet::new(), date.clone()));
        entry.0 += 1;
        entry.1.insert(author.clone());
    }
    let mut found: Vec<Churn> = counts
        .into_iter()
        .map(|(path, (commits, authors, last_touched))| Churn {
            path,
            commits,
            authors: authors.len(),
            last_touched,
        })
        .collect();
    found.sort_by(|left, right| {
        right
            .commits
            .cmp(&left.commits)
            .then_with(|| left.path.cmp(&right.path))
    });
    found.truncate(top);
    found
}

/// Source files no recent commit has touched.
fn dormant(root: &Path, tree: &Tree, recent: usize) -> Vec<String> {
    let touched: BTreeSet<String> = git_lines(
        root,
        &["log", &format!("-{recent}"), "--format=", "--name-only"],
    )
    .into_iter()
    .collect();
    if touched.is_empty() {
        return Vec::new();
    }
    tree.files
        .iter()
        .filter(|file| language_for(&file.extension()).is_some())
        .map(|file| file.rel.clone())
        .filter(|rel| !touched.contains(rel))
        .collect()
}

/// When each line a scan would cite was last written.
///
/// The join that makes the two bundles worth having together: `reverse scan` says *there is a marked
/// line here* and this says *and it has said that since 2023*. Keyed by `path` and `line`, which is
/// the citation an artifact carries anyway, so nothing has to be threaded between the two commands.
fn line_ages(root: &Path, tree: &Tree) -> Vec<LineAge> {
    let mut wanted: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for site in todo_sites(tree) {
        wanted.entry(site.path).or_default().insert(site.line);
    }
    for test in disabled_tests(tree) {
        wanted.entry(test.path).or_default().insert(test.line);
    }

    let mut found = Vec::new();
    for (path, lines) in wanted {
        let mut arguments = vec!["blame".to_owned(), "--line-porcelain".to_owned()];
        for line in &lines {
            arguments.push("-L".to_owned());
            arguments.push(format!("{line},{line}"));
        }
        arguments.push("--".to_owned());
        arguments.push(path.clone());
        let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let Some(output) = git(root, &borrowed) else {
            // An uncommitted file has no blame and is not a failure: it is the newest possible
            // answer, and saying nothing about it is more honest than dating it today.
            continue;
        };
        found.extend(parse_blame(&path, &output));
    }
    found.sort_by(|left, right| {
        left.last_written
            .cmp(&right.last_written)
            .then_with(|| (&left.path, left.line).cmp(&(&right.path, right.line)))
    });
    found
}

/// One `git blame --line-porcelain` run, as line ages.
///
/// Porcelain rather than a format string because it is the one blame output with a stated contract:
/// a header naming the line, then `author-time`, `summary` and the rest as their own keys. A parser
/// over `--date=short` would be reading a locale.
fn parse_blame(path: &str, output: &str) -> Vec<LineAge> {
    let mut found = Vec::new();
    let mut line = None;
    let mut time = None;
    let mut subject = None;
    for entry in output.lines() {
        if let Some(rest) = entry.strip_prefix("author-time ") {
            time = rest.trim().parse::<i64>().ok();
        } else if let Some(rest) = entry.strip_prefix("summary ") {
            subject = Some(rest.trim().to_owned());
        } else if entry.starts_with('\t') {
            // The content line closes a record: everything before it described this one.
            if let (Some(number), Some(seconds), Some(text)) = (line, time, subject.take()) {
                found.push(LineAge {
                    path: path.to_owned(),
                    line: number,
                    last_written: day(seconds),
                    subject: truncate(&text),
                });
            }
            line = None;
            time = None;
        } else if let Some(number) = blame_header_line(entry) {
            line = Some(number);
        }
    }
    found
}

/// The final line number in a porcelain header, `<sha> <orig> <final> [count]`.
fn blame_header_line(entry: &str) -> Option<usize> {
    let mut fields = entry.split(' ');
    let sha = fields.next()?;
    if sha.len() < 20 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    fields.next()?;
    fields.next()?.parse().ok()
}

/// A Unix timestamp as `YYYY-MM-DD`, UTC.
///
/// Written out rather than reached for from a date crate, and computed rather than read from a
/// clock: this converts a number a commit already carries, so it is the same on every machine and in
/// every timezone. A bundle whose dates moved with the reader's offset could not be diffed.
fn day(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let mut year = 1970;
    let mut remaining = days;
    loop {
        let length = if leap(year) { 366 } else { 365 };
        if remaining < length {
            break;
        }
        remaining -= length;
        year += 1;
    }
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while month < 12 && remaining >= months[month] {
        remaining -= months[month];
        month += 1;
    }
    format!("{year:04}-{:02}-{:02}", month + 1, remaining + 1)
}

/// Whether a year has a 29th of February.
fn leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// A count map as a ranked list, largest first then by name.
fn ranked(counts: BTreeMap<String, usize>) -> Vec<Counted> {
    let mut found: Vec<Counted> = counts
        .into_iter()
        .map(|(name, count)| Counted { name, count })
        .collect();
    found.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    found
}

/// Prints a history bundle for a person rather than for a program.
fn render_history(bundle: &HistoryBundle) {
    let span = &bundle.span;
    outln!("{HISTORY_VERSION}");
    outln!();
    outln!(
        "span: {} commits, {} author(s), {} -> {}",
        span.commits,
        span.authors,
        span.first_commit,
        span.last_commit
    );
    outln!("tags: {}  [{}]", span.tags, span.latest_tags.join(", "));
    outln!();

    let types: Vec<String> = bundle
        .commit_types
        .iter()
        .map(|entry| format!("{} {}", entry.name, entry.count))
        .collect();
    outln!("commit types: {}", types.join("  "));

    section("tickets", bundle.tickets.len());
    for ticket in &bundle.tickets {
        outln!(
            "  {}  {} commit(s), last {}",
            ticket.id,
            ticket.commits,
            ticket.last_seen
        );
    }

    section("reverted", bundle.reverted.len());
    for commit in &bundle.reverted {
        outln!("  {} {}  {}", commit.date, commit.hash, commit.subject);
    }

    section("stated expiry", bundle.stated_expiry.len());
    for commit in &bundle.stated_expiry {
        outln!("  {} {}  {}", commit.date, commit.hash, commit.subject);
    }

    section("churn", bundle.churn.len());
    for entry in &bundle.churn {
        outln!(
            "  {}  {} commit(s), {} author(s), last {}",
            entry.path,
            entry.commits,
            entry.authors,
            entry.last_touched
        );
    }

    section("dormant", bundle.dormant.len());
    for path in &bundle.dormant {
        outln!("  {path}");
    }

    section("line ages", bundle.line_ages.len());
    for age in &bundle.line_ages {
        outln!(
            "  {}  {}:{}  {}",
            age.last_written,
            age.path,
            age.line,
            age.subject
        );
    }
}
