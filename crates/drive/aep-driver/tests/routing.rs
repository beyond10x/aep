//! The router and the lock refusal: the two pieces of the driver that decide without touching
//! anything.
//!
//! Both are pure functions over values somebody else observed, which is what makes them testable
//! without a store, a process or a second machine. That is the point of the placement, not a side
//! effect of it (review finding **F19**).

use std::collections::BTreeMap;

use aep_domain::ids::{ExecutionId, StateId, TaskId};
use aep_driver::lock::{Liveness, LockState};
use aep_driver::route::{next_step, steps_remaining, NextStep};
use aep_driver_spec::cursor::{DriverCursor, RunId, RunStatus};
use aep_driver_spec::map::{StepMapId, DEFAULT_VISIT_BUDGET};

/// A map with two steps in `implement`, a visit budget of two, and nothing said about `verify`.
const MAP: &str = r"
format: aep.driver-steps/1
id: test/routing
workflow: test/linear/1
states:
  implement:
    visit_budget: 2
    steps:
      - kind: llm
        prompt: write the code
      - kind: command
        run: [cargo, test]
        evidence:
          kind: test_result
          verifier: test-runner
          suite: unit
";

fn map() -> aep_driver_spec::map::StepMap {
    aep_schema::parse::step_map(MAP, Some("test/routing.yaml")).expect("the fixture map validates")
}

fn cursor(state: &str) -> DriverCursor {
    let task = TaskId::new("T-1").expect("a task id");
    DriverCursor {
        run: RunId::new(&task, 1).expect("a run id"),
        task: task.clone(),
        execution: ExecutionId::new("T-1.1").expect("an execution id"),
        workflow: "test/linear/1".to_owned(),
        map: StepMapId::new("test/routing").expect("a map id"),
        map_digest: "digest".to_owned(),
        engine_version: aep_driver::ENGINE_VERSION.to_owned(),
        state: StateId::new(state).expect("a state id"),
        step: 0,
        visits: BTreeMap::new(),
        attempts: BTreeMap::new(),
        in_flight: None,
        circuit_failures: BTreeMap::new(),
        iterations: 0,
        status: RunStatus::Running,
        reasons: Vec::new(),
        took_lock_from: None,
        owed: None,
        answers: Vec::new(),
    }
}

#[test]
fn the_router_walks_a_states_steps_in_order_and_then_asks_the_engine_to_move() {
    let map = map();
    let mut cursor = cursor("implement");
    cursor.record_visit(&cursor.state.clone());

    assert_eq!(next_step(&map, &cursor), NextStep::Run { index: 0 });
    assert_eq!(steps_remaining(&map, &cursor), 2);

    cursor.step = 1;
    assert_eq!(next_step(&map, &cursor), NextStep::Run { index: 1 });
    assert_eq!(steps_remaining(&map, &cursor), 1);

    cursor.step = 2;
    assert_eq!(
        next_step(&map, &cursor),
        NextStep::Transition,
        "the state's steps are done, so the next move is the workflow's"
    );
    assert_eq!(steps_remaining(&map, &cursor), 0);
}

#[test]
fn a_state_the_map_says_nothing_about_transitions_immediately() {
    let map = map();
    let mut cursor = cursor("verify");
    cursor.record_visit(&cursor.state.clone());

    assert_eq!(next_step(&map, &cursor), NextStep::Transition);
    assert_eq!(steps_remaining(&map, &cursor), 0);
    assert_eq!(
        map.visit_budget(&StateId::new("verify").expect("a state id")),
        DEFAULT_VISIT_BUDGET,
        "a state the map is silent about still has a budget, or a back-edge into it is unbounded"
    );
}

#[test]
fn a_state_entered_past_its_visit_budget_stops_the_run_rather_than_running_its_steps_again() {
    let map = map();
    let mut cursor = cursor("implement");
    let state = cursor.state.clone();

    for _ in 0..2 {
        cursor.record_visit(&state);
        assert_eq!(
            next_step(&map, &cursor),
            NextStep::Run { index: 0 },
            "a budget of two permits two entries"
        );
    }

    cursor.record_visit(&state);
    assert_eq!(
        next_step(&map, &cursor),
        NextStep::VisitBudgetExhausted {
            state: state.clone(),
            budget: 2
        },
        "the third entry exceeds the budget, and the run stops with the state named rather than \
         burning a token budget in silence"
    );
    assert_eq!(
        cursor.step, 0,
        "the budget is checked before the step list, so a spent state has no next step however \
         many steps it still holds"
    );
}

#[test]
fn a_cursor_pointing_past_the_end_of_a_shortened_list_reports_no_steps_left() {
    let map = map();
    let mut cursor = cursor("implement");
    cursor.record_visit(&cursor.state.clone());
    cursor.step = 9;

    assert_eq!(steps_remaining(&map, &cursor), 0);
    assert_eq!(next_step(&map, &cursor), NextStep::Transition);
}

#[test]
fn a_live_holder_is_refused_and_take_lock_is_refused_with_it() {
    // `state: None` here rather than a value: this case predates the field and asserts nothing
    // about it. Every construction site supplies it explicitly, because a `Default` or a builder
    // would make *forgetting the holder's state* the silent path — which is the defect the field
    // exists to close.
    let held = LockState {
        run: "AUTH-142/2".to_owned(),
        pid: 4711,
        host: "workbench".to_owned(),
        liveness: Liveness::Alive,
        state: None,
    };
    assert!(!held.is_stale());

    for taking in [false, true] {
        let refusal = held.refusal(taking);
        assert!(refusal.contains("AUTH-142/2"), "{refusal}");
        assert!(refusal.contains("4711"), "{refusal}");
        assert!(refusal.contains("workbench"), "{refusal}");
        assert!(
            refusal.contains("--resume"),
            "a refusal that does not name the way out is a puzzle: {refusal}"
        );
    }
    assert!(
        held.refusal(true)
            .contains("refused while the holder is alive"),
        "`--take-lock` is not a way past a running process"
    );
}

#[test]
fn a_dead_holder_is_stale_and_still_refused_until_a_person_says_take_it() {
    let stale = LockState {
        run: "AUTH-142/2".to_owned(),
        pid: 4711,
        host: "workbench".to_owned(),
        liveness: Liveness::Dead,
        state: None,
    };
    assert!(stale.is_stale());

    let refusal = stale.refusal(false);
    assert!(
        refusal.contains("--take-lock"),
        "a stale lock is refused *and* the route out is named: {refusal}"
    );
    assert!(
        stale.refusal(true).contains("supersedes"),
        "taking a lock supersedes rather than erases; the stolen lock goes into the new cursor"
    );
}

#[test]
fn a_lock_held_on_another_host_is_never_stale_whatever_the_local_pid_table_says() {
    let elsewhere = LockState {
        run: "AUTH-142/2".to_owned(),
        pid: 4711,
        host: "ci-runner-3".to_owned(),
        liveness: Liveness::OtherHost,
        state: None,
    };
    assert!(
        !elsewhere.is_stale(),
        "a pid on another machine is a number about a process this one cannot see"
    );
    let refusal = elsewhere.refusal(true);
    assert!(refusal.contains("ci-runner-3"), "{refusal}");
    assert!(
        refusal.contains("never stale"),
        "the reason has to travel with the refusal: {refusal}"
    );
}

/// Every liveness, both `taking` values.
const COMBINATIONS: [(Liveness, bool); 6] = [
    (Liveness::Alive, false),
    (Liveness::Alive, true),
    (Liveness::Dead, false),
    (Liveness::Dead, true),
    (Liveness::OtherHost, false),
    (Liveness::OtherHost, true),
];

/// A holder with the state the caller says it has, or none.
fn holder(liveness: Liveness, state: Option<&str>) -> LockState {
    LockState {
        run: "AUTH-142/2".to_owned(),
        pid: 4711,
        host: "workbench".to_owned(),
        liveness,
        state: state.map(ToOwned::to_owned),
    }
}

/// **R2, R4 of `specification:operator-resume-ux`.** The fifth fact is the one that decides what
/// the operator types.
///
/// Who holds the lock does not decide between `--resume` and waiting; *what that run is doing*
/// does. So the state travels in the same line as the run, the pid, the host and the liveness — in
/// **every** branch, which is what asserting over all six combinations is for. A clause written into
/// five arms instead of the one shared holder fragment passes today and loses the state the first
/// time somebody adds a sixth arm.
#[test]
fn every_refusal_names_the_holders_state_beside_its_run_pid_host_and_liveness() {
    for (liveness, taking) in COMBINATIONS {
        let refusal = holder(liveness, Some("awaiting-operator")).refusal(taking);
        for fact in [
            "AUTH-142/2",
            "4711",
            "workbench",
            liveness.as_str(),
            "state awaiting-operator",
        ] {
            assert!(
                refusal.contains(fact),
                "`{fact}` is missing from the {liveness}/taking={taking} line:\n{refusal}"
            );
        }
    }
}

/// **R3, R5.** A state nobody could read is said in words, and no message loses its route out.
///
/// A missing clause reads as *there is no state*. The true fact is *this machine could not read
/// one*, and only one of those is a reason to go and look — so the clause is the literal
/// `state unknown` rather than an omission.
///
/// The second half is a floor rather than a new claim: adding a field to a line must not cost that
/// line its answer. Every route the five branches name today is asserted here **with the state
/// absent**, which is the arrangement in which a rewrite of the shared fragment is most likely to
/// drop one.
#[test]
fn a_holder_whose_state_could_not_be_read_is_said_to_be_unknown_rather_than_left_out() {
    for (liveness, taking) in COMBINATIONS {
        let refusal = holder(liveness, None).refusal(taking);
        assert!(
            refusal.contains("state unknown"),
            "the {liveness}/taking={taking} line omits the clause instead of wording it:\n{refusal}"
        );
    }

    for (liveness, taking, route) in [
        (Liveness::Alive, true, "refused while the holder is alive"),
        (Liveness::Alive, false, "--resume"),
        (Liveness::Alive, false, "--take-lock"),
        (Liveness::Dead, true, "supersedes"),
        (Liveness::Dead, false, "--take-lock"),
        (Liveness::Dead, false, "--resume"),
        (Liveness::OtherHost, true, "never stale"),
        (Liveness::OtherHost, false, "never stale"),
    ] {
        let refusal = holder(liveness, None).refusal(taking);
        assert!(
            refusal.contains(route),
            "the {liveness}/taking={taking} line no longer names `{route}`, so a refusal that told \
             an operator what to do now tells them only no:\n{refusal}"
        );
    }
}

/// **R4.** The field is serde-optional in both directions.
///
/// A `LockState` serialised before this change still deserialises, and one with no state gains no
/// key — so a document written by either side of the change reads on the other.
#[test]
fn a_lock_state_written_before_the_state_field_still_reads_and_an_absent_state_writes_no_key() {
    let before = r#"{"run":"AUTH-142/2","pid":4711,"host":"workbench","liveness":"alive"}"#;
    let read: LockState = serde_json::from_str(before).expect("a lock state without a state key");
    assert_eq!(
        read.state, None,
        "an absent key is an undetermined state, not a failure to parse"
    );

    let written = serde_json::to_string(&read).expect("it serialises");
    assert!(
        !written.contains("state"),
        "an absent state writes no key, so `null` and absent are not two spellings of it: {written}"
    );

    let known = holder(Liveness::Alive, Some("implement"));
    let text = serde_json::to_string(&known).expect("it serialises");
    let back: LockState = serde_json::from_str(&text).expect("it deserialises");
    assert_eq!(back, known, "a known state round-trips: {text}");
}

/// Every line of `text` that is code rather than a comment, as `(line number, line)`.
///
/// The same shape `tests/determinism.rs` uses, and for the same reason: this crate's rules are
/// about what its **code** does, and a module documenting the rule it keeps would otherwise trip
/// its own scan.
fn code_lines(text: &str) -> Vec<(usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(number, line)| (number + 1, line))
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .collect()
}

/// **R1, and the placement rule L8 of `task:orx-lock-state-carries-state`.**
///
/// `crates/drive/aep-driver/src/lock.rs` is handed a [`LockState`] and probes nothing: no pid table, no
/// hostname, no clock, no filesystem. `tests/determinism.rs` cannot catch a probe — one reads
/// ambient OS state and uses none of that scan's banned tokens (review finding **F19**) — so
/// placement is the guard, and this is the scan that keeps it.
///
/// It matters most *now*: the obvious way to satisfy "the refusal names the holder's state" is to
/// read the holder's cursor right where the `LockState` is built, and the whole of R1 is that the
/// state is **supplied by the caller** instead.
#[test]
fn the_lock_module_still_reads_nothing_about_the_machine_it_runs_on() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lock.rs");
    let text = std::fs::read_to_string(&path).expect("the lock module is readable");
    let probes = [
        "std::fs",
        "fs::",
        "std::process",
        "std::env",
        "Path::",
        "PathBuf",
        "/proc",
        "hostname",
        "read_to_string",
        "SystemTime",
        "Command",
    ];
    let mut found = Vec::new();
    for (number, line) in code_lines(&text) {
        for probe in probes {
            if line.contains(probe) {
                found.push(format!("{}:{number}: `{probe}`", path.display()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "this crate is handed a `LockState` and probes nothing; these lines probe:\n{}",
        found.join("\n")
    );
}

/// **S1 of `task:orx-theft-in-the-record`.** The driver learns the stolen lock from its caller.
///
/// `lock.json` belongs to `aep-cli`, along with the run directory it grants. Threading the
/// superseded lock through `DriverOptions` or an argument satisfies R14; opening the lock file here
/// does not, and this is what says so — the crate never names the file at all outside its prose.
#[test]
fn the_driver_crate_never_opens_the_lock_file_it_is_told_about() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&directory).expect("the crate has sources") {
        let path = entry.expect("an entry").path();
        if path.extension().is_none_or(|it| it != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable source file");
        for (number, line) in code_lines(&text) {
            if line.contains("lock.json") {
                found.push(format!("{}:{number}", path.display()));
            }
        }
        checked += 1;
    }
    assert!(
        checked >= 6,
        "only {checked} source files were read; the scan is looking in the wrong place"
    );
    assert!(
        found.is_empty(),
        "the lock file belongs to `aep-cli`, and this crate is told about it rather than \
         reading it:\n{}",
        found.join("\n")
    );
}
