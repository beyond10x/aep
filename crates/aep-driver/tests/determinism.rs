//! Invariant 9's scan for this crate: no unordered map, no clock, no randomness, no ambient
//! environment, no spawned process.
//!
//! § 4.1 makes a purity claim for the driver — *clock-free and randomness-free, the same discipline
//! `aep-domain` holds under invariant 8* — and this is the half of it that can be checked
//! mechanically. It is worth more here than in most crates because the thing being claimed is
//! **replayability**: given the same snapshot and the same evidence, the same routing. A `HashMap`
//! in the router would make the order two builds walk a state's steps in a coin flip, and a clock
//! would make a run's routing depend on when it was started. An ambient environment read is the
//! same defect wearing a third face: it makes a run's routing depend on the shell it was launched
//! from, which no snapshot records and no replay can reproduce.
//!
//! What the scan cannot see is placed rather than banned: a pid-liveness probe reads ambient OS
//! state and uses none of these tokens, which is why the probe lives in `protocol-cli` and this
//! crate is handed a `LockState` (review finding **F19**). A scan is a floor, not the claim.
//!
//! The run directory is the one impurity Acceptance concedes, and the fourth bullet names the price
//! of the concession out loud: *the scan bans process spawning*. A spawned program reads a second
//! machine's world and hands back a pid, so it is the same defect as the clock with a fork in it.
//!
//! The scan *was* `aep-driver-spec`'s, which is `ess-gen`'s — comment-skipping and boundary-aware,
//! with both refinements asserted against synthetic samples, so a scan that has stopped seeing
//! violations fails on them instead of passing on everything. It is **extended here** and the two
//! have diverged: `aep-driver-spec/tests/determinism.rs` still bans seven tokens and reads one flat
//! `read_dir`, while this file bans twelve and takes its file list from the crate's own module
//! graph. Read this one for what the driver is held to; do not assume the sibling matches.
//!
//! # The file list is a `mod` question, not a filesystem question
//!
//! What the claim needs is *every file the crate compiles was read*. A directory walk answers a
//! different question and answers it wrongly in both directions: `#[path = "../ambient.rs"]` puts a
//! compiled module outside `src/` where no walk of `src/` will ever see it — a live idiom, at
//! `crates/ess-gen/src/schema.rs:48` — and a `src/fixtures/` of JSON is not a module at all. So the
//! list comes from [`compiled_sources`], which starts at `src/lib.rs` and follows `mod` the way the
//! compiler does, and [`rust_sources`] is kept only as a second opinion over the tree on disk.

use std::path::{Path, PathBuf};

/// What this crate must not mention in code.
///
/// Three groups. The collections, the clock and the RNG are invariant 9's original list.
///
/// The environment pair is Acceptance's third face of the same defect, and it is deliberately the
/// **module** rather than a function family: `std::env` catches `use std::env;` and the qualified
/// call, and `env::` catches every function on the module however the module was imported. A single
/// `env::var` token would have missed `use std::{env, ffi::OsString};` followed by `env::args_os()`
/// — ordinary Rust that a formatter produces and that holds neither `std::env` nor `var`. Widening
/// to the module costs the `env!("CARGO_PKG_VERSION")` exemption nothing, because `env!(` holds no
/// `::`: the compiler resolves it into the build rather than into something the run could observe
/// differently on a second machine.
///
/// The process tokens are the fourth Acceptance bullet's price for conceding the run directory.
/// Three spellings for one act, because the import decides which one a file uses.
const BANNED: &[&str] = &[
    "HashMap",
    "HashSet",
    "SystemTime",
    "Instant::now",
    "rand::",
    "getrandom",
    "thread_rng",
    "std::env",
    "env::",
    "std::process",
    "process::Command",
    "Command::new",
];

/// `path` with `.` and `..` resolved lexically, touching the filesystem not at all.
///
/// `#[path = "../ambient.rs"]` builds `src/../ambient.rs`, which is the same file as
/// `crates/aep-driver/ambient.rs` and not the same `PathBuf`, so the scan would report it twice and
/// the coverage assertion could not match it. Lexical rather than `canonicalize` on purpose: a path
/// holding no `..` comes back byte-identical, so nothing that does not need resolving is touched,
/// and no symlink under the build directory can move a fixture out from under a test.
fn normalize(path: &Path) -> PathBuf {
    let mut resolved = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            std::path::Component::CurDir => {}
            other => resolved.push(other),
        }
    }
    resolved
}

/// Every module `text` declares out of line, as `(name, the `#[path]` it was given)`.
///
/// A line-wise reader rather than a parser, and deliberately so — but it fails loudly rather than
/// quietly, which is the property that matters: [`compiled_sources`] panics on any declaration this
/// resolves to a file that is not there, so a spelling this reader gets wrong stops the suite
/// instead of dropping a file out of the scan. An inline `mod name { .. }` is skipped because its
/// contents are already in the file being read.
fn module_declarations(text: &str) -> Vec<(String, Option<String>)> {
    let mut found = Vec::new();
    let mut path_attribute: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#[path") {
            let quoted = rest.split('"').nth(1);
            path_attribute = quoted.map(str::to_owned);
            continue;
        }
        let declaration = trimmed
            .strip_prefix("pub(crate) ")
            .or_else(|| trimmed.strip_prefix("pub(super) "))
            .or_else(|| trimmed.strip_prefix("pub "))
            .unwrap_or(trimmed);
        let Some(rest) = declaration.strip_prefix("mod ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|it| it.is_alphanumeric() || *it == '_')
            .collect();
        if !name.is_empty() && rest.trim_end().ends_with(';') {
            found.push((name, path_attribute.take()));
        } else {
            // An inline module. Its `#[path]`, if it had one, belongs to nothing further.
            path_attribute = None;
        }
    }
    found
}

/// Every file the crate compiles, found the way the compiler finds them.
///
/// Starts at `entry` — `src/lib.rs` — and follows `mod` declarations. This is the invariant the
/// purity claim actually needs, and it is not the same as *every `.rs` file under `src/`*:
///
/// * `#[path = "../ambient.rs"] pub mod ambient;` compiles a module that sits **outside** `src/`,
///   and no walk of `src/` can see it. It is not hypothetical — `crates/ess-gen/src/schema.rs:48`
///   uses `#[path]`, and its own comment calls the layout temporary.
/// * a `src/fixtures/` of JSON is compiled by nothing, so it is not the scan's business.
///
/// Resolution follows the reference: a `#[path]` on an out-of-line module is relative to the
/// directory holding the declaring file, and a plain `mod name;` is `name.rs` or `name/mod.rs`
/// below the declaring file's own module directory. A declaration that resolves to no file is a
/// panic rather than a skip — a module walk that silently drops what it cannot follow is exactly
/// the weakness a directory walk already had.
fn compiled_sources(entry: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut pending = vec![normalize(entry)];
    while let Some(file) = pending.pop() {
        if found.contains(&file) {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap_or_else(|error| {
            panic!(
                "`{}` is compiled into the crate and could not be read: {error}",
                file.display()
            )
        });
        let beside = file
            .parent()
            .expect("a source file sits in a directory")
            .to_path_buf();
        let stem = file
            .file_stem()
            .and_then(|it| it.to_str())
            .expect("a source file has a name");
        let below = if matches!(stem, "lib" | "main" | "mod") {
            beside.clone()
        } else {
            beside.join(stem)
        };
        found.push(file.clone());
        for (name, path_attribute) in module_declarations(&text) {
            let target = if let Some(relative) = path_attribute {
                normalize(&beside.join(relative))
            } else {
                let flat = below.join(format!("{name}.rs"));
                if flat.is_file() {
                    flat
                } else {
                    below.join(&name).join("mod.rs")
                }
            };
            assert!(
                target.is_file(),
                "`mod {name};` in `{}` resolves to `{}`, which is not a file. This walk fails here \
                 rather than skipping, because a declaration it cannot follow is a compiled module \
                 the purity scan would then never read",
                file.display(),
                target.display()
            );
            pending.push(target);
        }
    }
    found.sort();
    found
}

/// Every `.rs` file at or below `root`, at any depth.
///
/// Recursive, and that is the whole point: `read_dir` alone does not descend, so a module moved one
/// directory down is scanned by nothing while a count floor over the top level still passes.
/// `src/run.rs` is over twelve hundred lines, and splitting it into `src/run/` is the ordinary
/// tidying that would have silently disarmed this file.
fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("a readable source directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|it| it == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every directory below `root` that holds Rust, at any depth, with `root` itself excluded.
///
/// Deliberately a *second* traversal rather than a by-product of the first: the crate scan asserts
/// that every directory this one finds contributed a file the other one read, so a walk that
/// stopped descending fails an assertion instead of quietly reporting a smaller tree — the failure
/// a `checked >= 6` floor cannot see.
///
/// Two things it is careful about.
///
/// *Only directories that hold a `.rs` file.* An earlier draft called every directory a module and
/// so failed on a correct tree — one `src/fixtures/protocol.json` reddened the purity scan with
/// *"a module directory the scan never enters"*, which was untrue twice: nothing under it is
/// enterable Rust, and `src/test-data/` could not be a module at all, because a hyphen is not legal
/// in an identifier. A guard that fires on a correct tree with a wrong explanation is one whose
/// next reader deletes the assertion rather than the file.
///
/// *It panics where the other walk panics.* This is the traversal whose whole job is to prove the
/// other one covered the tree, so it is the one that must never under-report: swallowing an
/// unreadable directory here would quietly shrink the set of things the coverage invariant is
/// stated over, which is the same silence in a louder place.
fn module_directories(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let mut holds_rust = false;
        for entry in std::fs::read_dir(&directory).expect("a readable source directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|it| it == "rs") {
                holds_rust = true;
            }
        }
        if holds_rust && directory != root {
            found.push(directory);
        }
    }
    found.sort();
    found
}

/// Every banned token `text` uses in code, as `(line number, token)`.
fn banned_uses(text: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        for token in BANNED {
            let mut from = 0;
            while let Some(at) = line[from..].find(token) {
                let start = from + at;
                let boundary = line[..start]
                    .chars()
                    .next_back()
                    .is_none_or(|before| !before.is_alphanumeric() && before != '_');
                if boundary {
                    found.push((number + 1, *token));
                }
                from = start + token.len();
            }
        }
    }
    found
}

#[test]
fn the_router_holds_no_unordered_map_and_reads_no_clock_or_environment_and_spawns_nothing() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // Every file the crate compiles, and every file lying in the tree — the union, because the two
    // answer different questions and each sees something the other cannot. `compiled_sources` is
    // the one the claim rests on: it follows `#[path]` out of `src/`, where no directory walk goes.
    let compiled = compiled_sources(&directory.join("lib.rs"));
    let on_disk = rust_sources(&directory);
    let mut sources = compiled.clone();
    sources.extend(on_disk.iter().cloned());
    sources.sort();
    sources.dedup();

    let mut violations = Vec::new();
    for path in &sources {
        let text = std::fs::read_to_string(path).expect("a readable source file");
        for (line, token) in banned_uses(&text) {
            violations.push(format!("{}:{line}: `{token}`", path.display()));
        }
    }

    let checked = sources.len();
    assert!(
        checked >= 6,
        "only {checked} source files were read; the scan is looking in the wrong place"
    );
    for module in module_directories(&directory) {
        assert!(
            on_disk.iter().any(|source| source.starts_with(&module)),
            "`{}` holds Rust and nothing under it was read. The count floor above is satisfied by \
             the top level alone, which is why it is this assertion that fails when the tree walk \
             stops descending",
            module.display()
        );
    }
    for file in &compiled {
        assert!(
            sources.contains(file),
            "`{}` is compiled into this crate and was not scanned. The invariant is *every file \
             the crate compiles was read*, and it is stated here rather than left to follow from \
             how the list above happens to be built",
            file.display()
        );
    }
    assert!(
        violations.is_empty(),
        "the replay claim is that the same snapshot and the same evidence yield the same routing, \
         and these lines can make two runs disagree:\n{}",
        violations.join("\n")
    );
}

/// The original three refinements, with the positive checks stated so that widening cannot break
/// them.
///
/// `contains` rather than `assert_eq!` on the whole match vector, and it is not a loosening: the
/// token and its line are still named exactly, so the sample stays load-bearing on `"HashMap"`
/// alone. What it stops doing is failing when somebody adds `"std::collections"` to `BANNED` — a
/// widening this file should welcome, and which the pinned vector would have reddened. The negative
/// checks stay exact, because that is where a scan's precision lives.
#[test]
fn the_scan_sees_a_real_violation_and_ignores_prose_and_substrings() {
    assert!(
        banned_uses("use std::collections::HashMap;").contains(&(1, "HashMap")),
        "a real use must trip the scan, and `\"HashMap\"` must be the token that does it"
    );
    assert!(
        banned_uses("// a HashMap here and two runs route differently").is_empty(),
        "a comment about the rule must not trip it"
    );
    assert!(
        banned_uses("let my_hash_map_like = MyHashMapLike::new();").is_empty(),
        "an identifier merely containing the token must not trip it"
    );
    assert!(
        banned_uses("        let now = SystemTime::now();").contains(&(1, "SystemTime")),
        "a clock read is the other half of what this scan is for, and `\"SystemTime\"` is the token \
         that has to see it"
    );
}

/// The two spellings the module doc names, each pinned to the token that has to see it.
///
/// Two drafts were wrong in opposite directions and both are worth stating, because the shape is
/// the point. The first pinned the whole match vector with `assert_eq!`, so widening `BANNED` to
/// close a hole reddened a test about purity. The second replaced it with *did any token containing
/// `env` fire* — which cannot tell `"std::env"` from `"env::"`, so deleting `"std::env"` outright
/// left this suite green.
///
/// The answer to a guard that punished strengthening was a second sample, not a looser assertion.
/// `contains(&(line, token))` names the token exactly and tolerates extra ones, and each sample
/// below is a line the *other* token cannot match — which is what makes both load-bearing.
#[test]
fn the_scan_sees_an_ambient_environment_read_in_both_spellings() {
    assert!(
        banned_uses("use std::env;").contains(&(1, "std::env")),
        "a bare import holds no `env::`, so `\"std::env\"` is the only token that can see it"
    );
    assert!(
        banned_uses("    let home = env::var(\"HOME\").ok();").contains(&(1, "env::")),
        "and an imported call holds no `std::env`, so `\"env::\"` is the only token that can see it"
    );
    assert!(
        banned_uses("    let root = std::env::args().next();").contains(&(1, "std::env")),
        "the fully qualified spelling is seen by both, which is why it cannot pin either"
    );
    assert!(
        banned_uses("// reading env::var here would make the same snapshot route two ways")
            .is_empty(),
        "prose about the rule is not a breach of it"
    );
    assert!(
        banned_uses("let candidate = shell_env::var_names();").is_empty(),
        "an identifier merely ending in the token must not trip it"
    );
}

/// **Acceptance, fourth bullet, verbatim: *"the scan bans process spawning"*.**
///
/// The story narrowed the purity claim to *no clock, no randomness, no ambient environment* and
/// paid for the narrowing with this clause — the run directory is conceded as the crate's one
/// filesystem impurity **and** the scan is supposed to close the door process spawning would open
/// instead. `BANNED` holds no process token, so it does not.
///
/// The only process scan in the crate is `tests/routing.rs:355-366`, and it reads exactly one file,
/// `src/lock.rs`. A `Command::new` in `src/route.rs` — the router itself — is seen by nothing: with
/// one planted, `cargo test -p aep-driver` was 69 passed, exit 0.
#[test]
fn the_scan_bans_process_spawning_anywhere_in_the_crate_not_only_in_the_lock_module() {
    for line in [
        "    let host = std::process::Command::new(\"hostname\").output();",
        "    let host = Command::new(\"hostname\").output();",
        "    let child = process::Command::new(\"sh\").spawn();",
    ] {
        assert!(
            !banned_uses(line).is_empty(),
            "a spawned program is ambient state with a pid: `{line}` must trip the scan, because \
             Acceptance pays for the filesystem concession with `the scan bans process spawning`"
        );
    }
    assert!(
        banned_uses("// spawning a program here would read a second machine's world").is_empty(),
        "prose about the rule is still not a breach of it"
    );
}

/// The environment half is two spellings of *one* function family, not of the module.
///
/// `std::env` catches the qualified path and `env::var` catches `var`, `var_os`, `vars` and
/// `vars_os` — because the boundary check looks only at the character *before* a match. Nothing
/// catches the module imported by group, which is ordinary Rust and what a formatter leaves alone:
///
/// ```text
/// use std::{env, ffi::OsString};          // no `std::env` substring
/// let _ = env::args_os();                  // no `env::var` substring
/// let _ = env::current_dir();              // no `env::var` substring
/// ```
///
/// Planted in `src/route.rs::next_step` — the deterministic router, the one function § 4.1's replay
/// claim is about — `cargo test -p aep-driver --test determinism` was 3 passed, 0 failed.
///
/// The fix is one token, `"env::"`, and it does **not** touch the `env!("CARGO_PKG_VERSION")` the
/// module doc says it is protecting: `env!(` holds no `::`. With `"env::"` in `BANNED`, the crate
/// scan stays green.
#[test]
fn the_scan_refuses_an_ambient_read_however_the_module_reached_the_file() {
    for line in [
        "    let args: Vec<String> = env::args().collect();",
        "    let here = env::current_dir().ok();",
        "    let mine = env::current_exe().ok();",
        "    let scratch = env::temp_dir();",
        "    env::set_var(\"AEP_ROUTE\", \"1\");",
    ] {
        assert!(
            !banned_uses(line).is_empty(),
            "`{line}` makes a run's routing depend on the shell it was launched from, which is the \
             defect this scan is named for; the import spelling above it is not the crate's choice \
             to make once the token list is the guard"
        );
    }
    assert!(
        banned_uses("pub const ENGINE_VERSION: &str = env!(\"CARGO_PKG_VERSION\");").is_empty(),
        "the compile-time read stays exempt: `env!(` is not `env::`, so closing the hole above \
         costs the exemption nothing"
    );
}

/// The recursion, proved on a tree rather than asserted in a comment.
///
/// The crate's `src/` is flat today, so the coverage assertion in the scan above is true and
/// vacuous — a walk that never descended would pass it. This builds the tree that walk would miss
/// and asks the two walkers about it directly, which is the same discipline the module doc claims
/// for the comment-skipping and boundary refinements: a scan that has stopped seeing violations
/// fails on a sample instead of passing on everything.
#[test]
fn the_walk_descends_into_a_module_directory_and_reads_what_it_finds() {
    // The pid is in the path because two gates in two worktrees run this at once, and a shared
    // fixture directory is how they delete each other's files mid-test.
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("determinism-walk-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let nested = root.join("ambient");
    std::fs::create_dir_all(&nested).expect("a scratch module tree");
    std::fs::write(root.join("lib.rs"), "pub mod ambient;\n").expect("a top level file");
    std::fs::write(
        nested.join("mod.rs"),
        "pub fn reached() { let _ = std::env::vars(); }\n",
    )
    .expect("a file one directory down");

    let sources = rust_sources(&root);
    assert!(
        sources.contains(&nested.join("mod.rs")),
        "a module one directory down must be read: `read_dir` does not descend, and a scan that \
         reads only the top level passes on a crate that moved its impurity into a subdirectory. \
         Read instead: {sources:?}"
    );
    assert!(
        module_directories(&root).contains(&nested),
        "and the directory traversal has to see the same subdirectory, or the coverage assertion \
         in the crate scan can never fail"
    );

    let text = std::fs::read_to_string(nested.join("mod.rs")).expect("the nested file");
    assert!(
        !banned_uses(&text).is_empty(),
        "and the ambient read planted inside it trips the token list, which is what makes the two \
         halves one guard rather than two"
    );

    std::fs::remove_dir_all(&root).expect("the scratch tree is this test's to remove");
}

/// The coverage invariant calls every directory a module, and a directory need not be one.
///
/// `the_router_holds_no_unordered_map_and_reads_no_clock_or_environment_and_spawns_nothing` asserts
/// that every entry `module_directories` returns has a file `rust_sources` read beneath it, and
/// fails with *"a module directory the scan never enters is a module the purity claim does not
/// cover"*. Neither half of that sentence is true of a directory holding data. `src/test-data/`
/// cannot be a module at all — a hyphen is not legal in a Rust identifier — and there is nothing
/// under it for a scan to enter.
///
/// Reproduced end to end rather than argued: one empty `crates/aep-driver/src/fixtures/protocol.json`
/// and `cargo test -p aep-driver --test determinism` was 5 passed, 1 failed, with that message
/// naming the fixtures directory. A guard that fails on a correct tree, with a message that
/// misdescribes what it found, is one whose next reader deletes the assertion rather than the file.
///
/// The predicate below is a copy of the scan's own, applied to a tree the scan would meet.
#[test]
fn a_data_directory_under_src_is_not_a_module_the_walk_failed_to_enter() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("determinism-data-dir-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let data = root.join("test-data");
    std::fs::create_dir_all(&data).expect("a scratch tree");
    std::fs::write(root.join("lib.rs"), "pub fn nothing() {}\n").expect("a top level file");
    std::fs::write(data.join("protocol.json"), "{}\n").expect("a data file");

    let sources = rust_sources(&root);
    let uncovered: Vec<String> = module_directories(&root)
        .into_iter()
        .filter(|module| !sources.iter().any(|source| source.starts_with(module)))
        .map(|module| module.display().to_string())
        .collect();

    std::fs::remove_dir_all(&root).ok();

    assert!(
        uncovered.is_empty(),
        "the crate scan would report these as modules the walk never entered, and every one of \
         them is a directory that holds no Rust: {uncovered:?}. The invariant wanted is about the \
         *walk*, not about the tree — it has to be stated over directories that contain a `.rs` \
         file, or a crate that keeps fixtures beside its sources fails the purity scan for keeping \
         fixtures"
    );
}

/// Two levels is not recursion, and the tree the walk is proved on has exactly two.
///
/// `the_walk_descends_into_a_module_directory_and_reads_what_it_finds` builds `root/ambient/mod.rs`
/// and stops there, so a walker that descends exactly one level passes it — and passes the crate
/// scan too, whose coverage assertion is vacuous while `src/` is flat. Both mutants were run
/// against the shipped file, and neither was caught by anything:
///
/// | mutant | determinism suite, as shipped |
/// |---|---|
/// | `rust_sources` pushes a directory only when its parent is `root` | 6 passed, 0 failed |
/// | `module_directories` pushes a directory only when its parent is `root` | 6 passed, 0 failed |
///
/// `src/run.rs` is over twelve hundred lines; the tidying the walk exists to survive is
/// `src/run/mod.rs` plus `src/run/persist/snapshot.rs`, which is already three levels.
///
/// Counted rather than sampled, and five deep rather than two, because a fixture pinned at depth
/// *k* is passed by a walker that stops at *k* — asserting on the deepest entry alone just moves
/// the surviving mutant one level down. The counts below are exact on both sides, so a walker that
/// stops early, descends twice or reports a directory it never entered all fail here.
///
/// Green against the shipped walkers, red against either mutant in the table.
#[test]
fn the_walk_descends_further_than_the_one_level_it_is_proved_on() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("determinism-deep-walk-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();

    // `src/` plus five module directories under it, one `.rs` file in each of the six.
    let names = ["run", "persist", "snapshot", "journal", "frame"];
    let mut directories = Vec::new();
    let mut files = vec![root.join("lib.rs")];
    let mut here = root.clone();
    for name in names {
        here = here.join(name);
        directories.push(here.clone());
        files.push(here.join("mod.rs"));
    }
    std::fs::create_dir_all(&here).expect("a five level module tree");
    for file in &files {
        std::fs::write(file, "pub fn reached() {}\n").expect("a module file");
    }

    let found_files = rust_sources(&root);
    let found_directories = module_directories(&root);
    std::fs::remove_dir_all(&root).ok();

    files.sort();
    directories.sort();
    assert_eq!(
        found_files, files,
        "every `.rs` file in the tree must be read, at every depth: `src/run/` alone is the split          that was coming and `src/run/persist/` is the one after it, so a walk proved on two levels          is proved on the shallowest tree that will ever exist"
    );
    assert_eq!(
        found_directories, directories,
        "and the directory traversal has to reach the same depth, or the coverage assertion in the          crate scan stops covering exactly where the tree stops being flat"
    );
}

/// Both environment spellings pinned individually, because the loosened self-check cannot.
///
/// `the_scan_sees_an_ambient_environment_read_in_both_spellings` asks only whether *an* `env` token
/// fired, so it cannot tell `"std::env"` from `"env::"`. Deleting `"std::env"` from `BANNED` leaves
/// the whole determinism suite green — run, not argued: `cargo test -p aep-driver --test
/// determinism` was 6 passed, 0 failed with the token removed, because every sample in the file
/// also matches `"env::"`.
///
/// `use std::env;` is the one line `"env::"` cannot see, so it is the sample that makes the token
/// load-bearing. The exact-vector `assert_eq!` that F4 removed was the wrong instrument — it broke
/// when `BANNED` was *widened* — but the answer to a guard that punished strengthening is a second
/// sample, not an assertion that stopped distinguishing the two things it names.
///
/// Green against the shipped list, red the moment either environment token is dropped.
#[test]
fn each_environment_spelling_is_load_bearing_on_a_line_the_other_cannot_see() {
    assert!(
        !banned_uses("use std::env;").is_empty(),
        "a bare import holds no `env::`, so `\"std::env\"` is the only token that can see it; \
         without a sample like this one the token can be deleted and nothing goes red"
    );
    assert!(
        !banned_uses("    let args: Vec<String> = env::args().collect();").is_empty(),
        "and a group-imported call holds no `std::env`, so `\"env::\"` is the only token that can \
         see it: the two spellings are pinned apart or they are pinned not at all"
    );
}

/// `#[path]` walks past any traversal of `src/`, so the file list comes from the module graph.
///
/// The two directory walkers are not independent — they are the same descent predicate written
/// twice, so they agree on everything they both miss, and `#[path]` is the thing they both miss.
/// With `#[path = "../ambient.rs"] pub mod ambient;` in `src/lib.rs` and the file beside `src/`
/// holding `HashMap`, `SystemTime::now`, `Command::new` and `std::env::vars`, the whole suite was
/// 72 passed, exit 0: `module_directories` found no directory under `src/`, so the coverage
/// assertion was satisfied by a tree with an entire impure module compiled into it.
///
/// It is a live idiom rather than an attack: `crates/ess-gen/src/schema.rs:48` uses `#[path]`, and
/// its own comment calls the layout temporary.
///
/// The tree below is also **three levels deep and counted exactly**, so a walk that stops at any
/// depth fails here rather than at whatever depth a fixture happened to be pinned to.
#[test]
fn the_module_walk_follows_a_path_attribute_out_of_the_directory_a_tree_walk_can_see() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("determinism-path-attribute-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let source = root.join("src");
    let deeper = source.join("deep").join("deeper");
    std::fs::create_dir_all(&deeper).expect("a scratch crate tree");

    std::fs::write(
        source.join("lib.rs"),
        "#[path = \"../ambient.rs\"]\npub mod ambient;\npub mod deep;\n",
    )
    .expect("a crate root");
    // Outside `src/` entirely, which is the whole point.
    std::fs::write(
        root.join("ambient.rs"),
        "pub fn ambient() { let _ = std::env::vars(); }\n",
    )
    .expect("a module beside the source directory");
    std::fs::write(source.join("deep").join("mod.rs"), "pub mod deeper;\n").expect("a mod.rs");
    std::fs::write(deeper.join("mod.rs"), "pub fn reached() {}\n").expect("a third level");

    let compiled = compiled_sources(&source.join("lib.rs"));
    let on_disk = rust_sources(&source);
    let outside = root.join("ambient.rs");
    std::fs::remove_dir_all(&root).ok();

    let mut expected = vec![
        source.join("lib.rs"),
        source.join("deep").join("mod.rs"),
        deeper.join("mod.rs"),
        outside.clone(),
    ];
    expected.sort();
    assert_eq!(
        compiled, expected,
        "the module walk has to find every file the crate compiles and no other: the `#[path]` \
         module outside `src/`, the `mod.rs` one level down and the `mod.rs` two levels down. \
         Counted rather than sampled, because a walk that stops at depth `k` passes any fixture \
         pinned at depth `k`"
    );
    assert!(
        !on_disk.contains(&outside),
        "and the tree walk must be shown blind to it — if `rust_sources` could see `{}` there \
         would be nothing here to fix, and the assertion above would be proving nothing",
        outside.display()
    );
}

/// One sample per banned token, each seen by that token and by no other.
///
/// The general form of the defect the environment pair showed twice. A token no sample can single
/// out is a token that can be **deleted with the suite still green** — `"std::env"` was, and
/// `"HashSet"`, `"Instant::now"`, `"rand::"`, `"getrandom"` and `"thread_rng"` all were, because
/// every check in this file was written about the two or three tokens somebody had just added.
///
/// Stated as a table over `BANNED` itself rather than as one test per token, which buys the
/// property that actually matters: a token added without a sample fails **here**, so the list
/// cannot grow past its own evidence.
const PINS: &[(&str, &str)] = &[
    ("HashMap", "    let seen: HashMap<u8, u8> = HashMap::new();"),
    ("HashSet", "    let seen: HashSet<u8> = HashSet::new();"),
    ("SystemTime", "    let now = SystemTime::now();"),
    ("Instant::now", "    let started = Instant::now();"),
    ("rand::", "    let value = rand::random::<u8>();"),
    ("getrandom", "    let _ = getrandom(&mut bytes);"),
    ("thread_rng", "    let mut source = thread_rng();"),
    ("std::env", "use std::env;"),
    ("env::", "    let home = env::var(\"HOME\").ok();"),
    ("std::process", "use std::process;"),
    ("process::Command", "use process::Command;"),
    (
        "Command::new",
        "    let child = Command::new(\"sh\").spawn();",
    ),
];

#[test]
fn every_banned_token_is_pinned_by_a_sample_no_other_token_matches() {
    for token in BANNED {
        assert!(
            PINS.iter().any(|(pinned, _)| pinned == token),
            "`{token}` is banned and no sample pins it, so deleting it from `BANNED` costs this \
             suite nothing and the ban is a comment. Add a line only this token can see"
        );
    }
    for (token, sample) in PINS {
        let hits = banned_uses(sample);
        assert!(
            hits.contains(&(1, token)),
            "`{sample}` is the sample for `{token}` and that token did not fire on it: {hits:?}"
        );
        let strays: Vec<&str> = hits
            .iter()
            .map(|(_, hit)| *hit)
            .filter(|hit| hit != token)
            .collect();
        assert!(
            strays.is_empty(),
            "`{sample}` is meant to pin `{token}` alone, and {strays:?} also matched it. A sample \
             two tokens can see pins neither: whichever of them is deleted, the other keeps this \
             assertion green"
        );
    }
}
