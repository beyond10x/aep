#[cfg(test)]
mod tests {
    use aep_driver_spec::tool::ToolConfig;
use super::*;
use aep_domain::ids::StateId;
use aep_driver::executor::StepAttempt;
fn config(capabilities: &[Capability]) -> ToolConfig {
        ToolConfig::new(capabilities.iter().cloned().collect())
    }
/// Every long flag a `drive` refusal names is a flag some `drive` verb parses.
    ///
    /// The defect this retires: a refused resume advised `--restart`, which no verb accepts, so the
    /// reader typed it and got a second refusal — a bare clap usage error that reads as their
    /// mistake rather than the message's. One test over the whole surface, because the failure is
    /// a sentence drifting from a command and that can happen to any of them.
    #[test]
    fn a_flag_a_refusal_names_is_a_flag_a_drive_verb_parses() {
        use clap::CommandFactory as _;

        let mut parsed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut collect = |command: &clap::Command| {
            for argument in command.get_arguments() {
                if let Some(long) = argument.get_long() {
                    parsed.insert(format!("--{long}"));
                }
            }
        };
        let root = crate::Cli::command();
        for command in root.get_subcommands() {
            collect(command);
            for nested in command.get_subcommands() {
                collect(nested);
                for deeper in nested.get_subcommands() {
                    collect(deeper);
                }
            }
        }
        assert!(
            parsed.contains("--root"),
            "the walk found no flags at all, so it is not testing anything: {parsed:?}"
        );

        let named: std::collections::BTreeSet<String> = aep_driver::run::routes_out()
            .split_whitespace()
            .map(|word| word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-'))
            .filter(|word| word.starts_with("--"))
            .map(str::to_owned)
            .collect();
        for flag in &named {
            assert!(
                parsed.contains(flag),
                "the refusal names `{flag}`, which no `drive` verb parses"
            );
        }
    }
/// The execution every fixture below belongs to: the first run of task `T-1`.
    ///
    /// `'static` because two of the helpers here *return* a [`StepContext`], and a context borrows
    /// its execution for as long as it lives.
    fn driven_execution() -> &'static ExecutionId {
        static EXECUTION: std::sync::OnceLock<ExecutionId> = std::sync::OnceLock::new();
        EXECUTION.get_or_init(|| ExecutionId::new("T-1.1").expect("an execution id"))
    }
/// One `llm` step, as a step map that names the second harness would produce it.
    ///
    /// The harness is spelled as a literal rather than through a constant, deliberately: this is
    /// the string a step map author writes, and a test that read it out of the same constant the
    /// selector reads would pass whatever that constant said.
    /// A one-step map whose `llm` step names the native harness.
    fn b10x_map() -> StepMap {
        aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/b10x\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: llm\n        prompt: do it\n\
             \x20       harness: b10x\n",
            None,
        )
        .expect("the map validates")
    }
/// The task a prompt test's run is driving.
    ///
    /// `derived_from` is populated because the identity line names the artifacts, and a fixture
    /// without one would let the line pass by saying nothing.
    fn driven_task() -> aep_domain::task::Task {
        aep_schema::parse::task(
            "id: T-1\nkind: feature\nobjective: drive something\nprotocol: aep/1\n\
             profile: test.standard\nderived_from: [story:the-one-being-driven]\n",
            None,
        )
        .expect("the fixture task parses")
    }
/// A metacharacter inside quotes is an argument; outside them it composes.
    ///
    /// **Found by run `A3` within minutes of admitting the readers, and it was my own defect.**
    /// `grep -n "StolenLock\|took_lock_from" crates/` is one invocation whose `|` belongs to grep,
    /// and the bare-character scan refused it three times in one state. Admitting a tool and then
    /// refusing the natural way to use it is worse than not admitting it: the session is told two
    /// things and cannot tell which to believe.
    #[test]
    fn a_metacharacter_inside_quotes_is_an_argument_and_outside_them_it_composes() {
        for one_invocation in [
            r#"grep -n "StolenLock\|took_lock_from" crates/"#,
            r"grep -n 'a;b' file",
            r#"grep -E "fn (drive|resume)" src/run.rs"#,
            r"rg 'x > y' crates",
            r"grep -n '$(whoami)' file",
            r"grep -n '`date`' file",
            "protocol artifact list",
        ] {
            assert_eq!(
                composes(one_invocation),
                None,
                "`{one_invocation}` is one invocation: its metacharacters are quoted"
            );
        }

        for composed in [
            "protocol artifact list && protocol artifact graph",
            "protocol artifact list | head",
            "grep -rn x . > out.txt",
            "cat a; rm b",
            r#"echo "$(whoami)""#,
            r#"echo "`date`""#,
            r#"grep -n "a\|b" file | wc -l"#,
        ] {
            assert!(
                composes(composed).is_some(),
                "`{composed}` composes and must be refused"
            );
        }
    }
/// A scratch directory under this crate's target directory, named for the test that asked.
    fn scratch(name: &str) -> PathBuf {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/drive-records")
            .join(name);
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("the scratch directory is writable");
        directory
    }
/// A `trace_conformance` document of the shape `protocol trace evidence` writes.
    const TRACE_RECORD: &str = "\
- kind: trace_conformance
  specification: driven-eval/honest-step
  spec_digest: c2114acdc5782176f7149da41bf1baab6266305ce77d31f813da9de8f93e7aeb
  transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b
  status: passed
  expectations_total: 12
  expectations_gapped: 0
  expectations_unknown: 0
  observed_at: 1787355862391
  producer:
    producer: verifier
    verifier: trace-checker
";
/// The record a verifier wrote is submitted as the verifier's, with nothing minted here.
    ///
    /// `trace_conformance` is not in `EvidenceMapping::MINTABLE` and must never be: its record
    /// carries a specification digest, a transcript digest and three counts, and an exit status
    /// carries none of them. So the check writes the document and the driver reads it — and the
    /// producer that arrives at the engine is the checker's, not this binary's, which is what makes
    /// the record admissible at all.
    #[test]
    fn a_record_a_verifier_wrote_is_submitted_as_that_verifiers_and_never_minted_here() {
        let directory = scratch("trace");
        let record = directory.join("trace-implement.yaml");
        std::fs::write(&record, TRACE_RECORD).expect("the record is writable");
        let mapping = EvidenceMapping {
            kind: EvidenceKind::TraceConformance,
            verifier: Verifier::TraceChecker,
            suite: None,
            subject: None,
            tool: None,
            record: Some("{run_directory}/trace-implement.yaml".to_owned()),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 1,
            attempt: 1,
            tools: &tools,
            run_directory: &directory,
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: Some(StepAttempt {
                index: 0,
                attempt: 1,
            }),
        };

        let outcome = read_record(
            mapping.record.as_deref().expect("a declared record"),
            &mapping,
            "protocol trace evidence",
            &context,
        );
        let StepOutcome::Observed(submission) = outcome else {
            panic!("a record that reads is a verdict: {outcome:?}");
        };
        assert_eq!(
            submission.evidence.kind(),
            EvidenceKind::TraceConformance,
            "what the document says it is, is what is submitted"
        );
        assert!(
            matches!(
                submission.producer,
                Producer::Verifier {
                    verifier: Verifier::TraceChecker
                }
            ),
            "the producer is the checker's own: {:?}",
            submission.producer
        );

        // `{transcript}` is a run-time fact, so a step that names one in a run where no `llm` step
        // has run is D5's `Unknown` rather than a verdict about a file that is not there.
        let empty: Vec<String> = Vec::new();
        let unrun = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 1,
            attempt: 1,
            tools: &tools,
            run_directory: &directory,
            requirements: &empty,
            reaching: &empty,
            preceding_llm: None,
        };
        let outcome = expand("{transcript}", &unrun).expect_err("there is no transcript to name");
        assert!(outcome.contains("transcript"), "{outcome}");
    }
/// `{task}` is the document **this run** was started from, and a run started from none says so.
    ///
    /// The two halves are the two things the placeholder has to get right. A driven run reaches
    /// `protocol specification evidence --task {task}` holding the document the operator named —
    /// not the one the project names, which is the discovery this closes: run `W4-3/1` bound that
    /// verb to `task.yaml` while the engine's cursor said something else. And a run whose task was
    /// never read out of a file produces D5's `Unknown`, rather than a command line carrying the
    /// literal characters `{task}` into a verb that would then bind by discovery anyway — the
    /// failure this whole placeholder exists to remove, reintroduced one layer down.
    #[test]
    fn the_task_placeholder_is_the_document_this_run_was_started_from() {
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "verify".parse().expect("a state id");
        let task = driven_task();
        let empty: Vec<String> = Vec::new();
        // Not `.engineering/task.yaml`: the whole point is a document the project does not name,
        // so a test whose fixture used the project's own would pass under discovery too.
        let named = Path::new("/projects/repo/.engineering/task-native-1.yaml");
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(named),
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &empty,
            reaching: &empty,
            preceding_llm: None,
        };
        assert_eq!(
            expand("{task}", &context).expect("a run started from a document expands it"),
            named.display().to_string(),
            "the document the driver resolved, not the one the project names"
        );
        // Inside a word as well as alone, because `--task={task}` is a line a map may write.
        assert_eq!(
            expand("--task={task}", &context).expect("a placeholder is expanded where it sits"),
            format!("--task={}", named.display())
        );

        let unread = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: None,
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &empty,
            reaching: &empty,
            preceding_llm: None,
        };
        let refusal = expand("{task}", &unread).expect_err("there is no document to name");
        assert!(
            refusal.contains("task document"),
            "the refusal says what the placeholder is: {refusal}"
        );
        assert!(
            refusal.contains(&task.id.to_string()),
            "and which task had none: {refusal}"
        );
    }
/// Invariant 7 at the layer a `record:` path opens: a run cannot submit a person's approval.
    ///
    /// The path a step writes to is a path a step can also write *to*, and an approval read out of
    /// a file would unlock a capability gate with a document the run itself could have authored.
    /// The engine's capability check matches on the decision and not on who granted it, so the
    /// refusal has to be here.
    #[test]
    fn an_approval_read_out_of_a_file_is_refused_however_well_formed_it_is() {
        let directory = scratch("approval");
        let record = directory.join("approval.yaml");
        std::fs::write(
            &record,
            "- kind: approval\n  approval: release\n  decision: granted\n  \
             observed_at: 1787355862391\n  producer:\n    producer: human\n    id: a-person\n",
        )
        .expect("the record is writable");
        let mapping = EvidenceMapping {
            kind: EvidenceKind::Approval,
            verifier: Verifier::HumanApproval,
            suite: None,
            subject: None,
            tool: None,
            record: Some("{run_directory}/approval.yaml".to_owned()),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "review".parse().expect("a state id");
        let empty: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: &directory,
            requirements: &empty,
            reaching: &empty,
            preceding_llm: None,
        };

        let outcome = read_record(
            mapping.record.as_deref().expect("a declared record"),
            &mapping,
            "cat approval.yaml",
            &context,
        );
        let StepOutcome::NoVerdict { reason } = outcome else {
            panic!("an approval read out of a file is refused: {outcome:?}");
        };
        assert!(
            reason.contains("approval"),
            "the refusal says what it refused: {reason}"
        );
    }
/// A record the verifier was to write and did not is D5's `Unknown`, and so is one that does
    /// not read.
    ///
    /// The case `story:evidence-producers-for-the-driven-map` made load-bearing. Three of the four
    /// kinds that map now produces arrive through `record:`, and the failure mode a producer has
    /// that a `cargo test` step does not is *the verb ran and wrote nothing usable*: a store the
    /// checker refused to choose from, a path a rename broke, a half-written file. None of those is
    /// a failing verdict — the run has observed nothing — and submitting a `failed` record for one
    /// would be the driver inventing an observation, which is invariant 7 a layer above the engine.
    ///
    /// Both roads are checked because they fail at different depths: a missing file never reaches
    /// the parser, and a malformed one fails inside it.
    #[test]
    fn a_record_that_is_missing_or_does_not_read_submits_nothing_and_says_why() {
        let directory = scratch("absent-record");
        let mapping = EvidenceMapping {
            kind: EvidenceKind::Specification,
            verifier: Verifier::ExternalTool("protocol".parse().expect("a tool reference")),
            suite: None,
            subject: None,
            tool: None,
            record: Some("{run_directory}/specification.yaml".to_owned()),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "adversarial_verify".parse().expect("a state id");
        let empty: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 3,
            attempt: 1,
            tools: &tools,
            run_directory: &directory,
            requirements: &empty,
            reaching: &empty,
            preceding_llm: None,
        };
        let read = || {
            read_record(
                mapping.record.as_deref().expect("a declared record"),
                &mapping,
                "protocol specification evidence",
                &context,
            )
        };

        // Nothing was written: the verb refused to choose between two specifications in force, or
        // the path in the map no longer names what the verb writes.
        let StepOutcome::NoVerdict { reason } = read() else {
            panic!("a record that is not there is not a verdict");
        };
        assert!(
            reason.contains("specification") && reason.contains("nothing was observed"),
            "the refusal names the kind that is owed and says nothing was observed, so a person \
             reading the run knows the step did not fail — it did not run: {reason}"
        );

        // Written, and not a document. Half a file is the shape a killed verb leaves behind.
        std::fs::write(
            directory.join("specification.yaml"),
            "- kind: specification\n  satisfied: ",
        )
        .expect("the record is writable");
        let StepOutcome::NoVerdict { reason } = read() else {
            panic!("a record that does not parse is not a verdict");
        };
        assert!(
            reason.contains("does not read"),
            "the refusal says the document is unreadable rather than reporting a failed \
             specification: {reason}"
        );
    }
/// Both arms' sessions are launched as the run, so a store write from inside one says so.
    ///
    /// **The defect this is about is one variable wide.** `command_actor()` stamped
    /// `human:<$USER>` on every store write, so a driven session running
    /// `protocol artifact move <spec> approved` was journalled as the operator's own move and
    /// nothing in the record could tell an agent's write from a person's. The launch declares who
    /// the session is instead.
    ///
    /// The second assertion is the one that has to hold for the first to be worth anything: the
    /// actor a session *writes* under is the same actor `admit` refuses an approval *from*. Two
    /// spellings of `agent:<execution>` would let a run approve its own specification under the
    /// name it wrote it with, which is the case the `operator` step exists to prevent — so the
    /// fixture reaches that state, naming the session itself as the approver, before asserting the
    /// refusal.
    #[test]
    fn an_llm_sessions_launch_declares_the_run_as_its_actor_and_that_actor_cannot_approve_the_run()
    {
        let execution = ExecutionId::new("W4-3.1").expect("an execution id");
        assert_eq!(
            session_env(&execution),
            vec![("AEP_ACTOR".to_owned(), "agent:W4-3.1".to_owned())],
            "the variable and its value are what `command_actor()` reads on the other side"
        );

        let declared = ActorRef::parse(&session_env(&execution)[0].1).expect("a parseable actor");
        let own = [aep_driver::attest::session_actor(&execution).expect("the run's own actor")];
        assert_eq!(declared, own[0], "one spelling, not two");
        let refusal = aep_driver::attest::admit(
            &Producer::Agent {
                id: declared.name().to_owned(),
            },
            Some(&declared),
            &own,
        );
        assert!(
            !refusal.is_admitted(),
            "the actor a session writes under may not approve that session's work: {refusal:?}"
        );

        // An execution id an actor name cannot hold declares nothing rather than a mangled name:
        // the session then writes as the operator did before, which is honest, and never as
        // somebody else.
        let slashed = ExecutionId::new("W4-3/1").expect("an execution id may carry a slash");
        assert!(session_env(&slashed).is_empty());
    }

    #[test]
    fn a_driven_action_keeps_one_command_identity_across_attempts() {
        let task = driven_task();
        let state = StateId::new("implement").expect("state");
        let tools = config(&[]);
        let directory = scratch("command-identity");
        let requirements = Vec::new();
        let reaching = Vec::new();
        let context = |index, attempt| StepContext {
            task: &task,
            task_document: None,
            execution: driven_execution(),
            state: &state,
            index,
            attempt,
            tools: &tools,
            run_directory: &directory,
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        let first = driver_command_identity(&context(3, 1));
        let retried = driver_command_identity(&context(3, 2));
        let next = driver_command_identity(&context(4, 1));
        assert_eq!(first, retried, "attempt is not part of the persisted action identity");
        assert_ne!(first, next, "a different action index gets a different reservation");
        assert!(aep_contract::migration::MigrationIdV1::new(first).is_ok());
    }
/// The `--write-scope` words are the words a step map is written in.
    ///
    /// Two spellings of one rule is one spelling that drifts, and the drift here is silent: a rule
    /// rendered as an unknown word is a rule metaharness refuses at launch, or worse, one it reads
    /// as a different rule.
    #[test]
    fn the_write_scope_words_are_the_ones_the_step_map_is_written_in() {
        for scope in [
            WriteScope::Allowed,
            WriteScope::PartialOnly,
            WriteScope::Denied,
        ] {
            let written = serde_json::to_value(scope).expect("a scope serialises");
            assert_eq!(
                written.as_str().expect("a string"),
                write_scope_word(scope),
                "the argv word and the document word are one word"
            );
        }
    }
#[test]
    fn a_failing_command_mints_a_record_that_says_so_and_a_failed_diff_mints_nothing() {
        let mapping = EvidenceMapping {
            kind: EvidenceKind::TestResult,
            verifier: Verifier::TestRunner,
            suite: Some(TestSuite::Unit),
            subject: None,
            tool: None,
            record: None,
        };
        let failed = mint(&mapping, false, "cargo test", observed_now()).expect("a verdict");
        match &failed.evidence {
            Evidence::TestResult(result) => assert_eq!(result.failed, 1),
            other => panic!("expected a test result, got {other:?}"),
        }
        assert_eq!(
            failed.producer,
            Producer::Verifier {
                verifier: Verifier::TestRunner
            }
        );

        let diff = EvidenceMapping {
            kind: EvidenceKind::Diff,
            verifier: Verifier::parse("git").expect("a verifier"),
            suite: None,
            subject: None,
            tool: None,
            record: None,
        };
        assert!(
            mint(&diff, false, "git diff", observed_now()).is_none(),
            "a ChangeSet has no form that says no change happened, so the honest answer is to \
             submit nothing"
        );
    }
/// `RunArgs` as `protocol drive run` parses them, so a refusal here is clap's and not ours.
    #[derive(Debug, clap::Parser)]
    struct RunProbe {
        #[command(flatten)]
        run: RunArgs,
    }
#[test]
    fn an_approver_is_parsed_as_an_actor_and_needs_a_run_that_can_stop() {
        use clap::Parser as _;
        let parsed = RunProbe::try_parse_from([
            "probe",
            "--pause-on-approval",
            "--approver",
            "agent:orchestrator",
        ])
        .expect("a named agent beside the pause flag parses");
        assert_eq!(
            parsed.run.approver,
            Some(ActorRef::parse("agent:orchestrator").expect("an actor"))
        );

        let error = RunProbe::try_parse_from(["probe", "--approver", "agent:orchestrator"])
            .expect_err(
                "an approver answers while the run is stopped, so the run must be able to stop",
            );
        assert!(
            error.to_string().contains("--pause-on-approval"),
            "the refusal names the flag it needs: {error}"
        );

        let error = RunProbe::try_parse_from(["probe", "--pause-on-approval", "--approver", "bob"])
            .expect_err("an actor is `<kind>:<name>`");
        assert!(
            error.to_string().contains("human:alice"),
            "the refusal shows the shape: {error}"
        );
    }
#[test]
    fn a_person_the_system_a_service_and_the_run_itself_are_refused_as_approvers_before_the_run() {
        let task = TaskId::new("T-1").expect("a task id");
        let map = b10x_map();
        for (named, why) in [
            ("human:alice", "needs no naming"),
            ("system", "nobody"),
            ("service:release-controller", "never answer"),
            ("agent:T-1", "own actor"),
            ("agent:T-1.2", "own actor"),
            ("agent:b10x", "own actor"),
        ] {
            let refusal = approver_refusal(&ActorRef::parse(named).expect("an actor"), &task, &map)
                .unwrap_or_else(|| panic!("`{named}` is refused"));
            assert!(refusal.contains(why), "`{named}`: {refusal}");
            assert!(refusal.contains("--approver"), "names the flag: {refusal}");
        }
        assert_eq!(
            approver_refusal(
                &ActorRef::parse("agent:orchestrator").expect("an actor"),
                &task,
                &map
            ),
            None,
            "an agent that is not this run may be named"
        );
        assert_eq!(
            approver_refusal(
                &ActorRef::parse("agent:T-1.x").expect("an actor"),
                &task,
                &map
            ),
            None,
            "only the execution family `<task>.<ordinal>` is the run's own"
        );
    }
/// A `command` step that says `protocol` is spawned as the binary this process **is**.
    ///
    /// The unit half of the rule: keyed on the file name and on nothing else, so a path spelling
    /// of the same request is the same request, and every other program a map can name is left
    /// exactly where it was. The end-to-end half — that the substituted binary really is the one
    /// that answers, proved by a version string only this build prints — is
    /// `a_command_step_that_says_protocol_runs_the_build_that_is_driving_it` in
    /// `tests/drive_cli.rs`.
    #[test]
    fn a_command_step_naming_this_cli_is_resolved_to_the_binary_this_process_is() {
        let executable = std::env::current_exe().expect("a running process can name itself");
        let expected = executable.display().to_string();

        for spelling in [
            "protocol",
            "/usr/local/bin/protocol",
            "./target/debug/protocol",
        ] {
            let resolved = resolve_program(spelling);
            assert_eq!(
                resolved.resolution,
                Resolution::Driver,
                "`{spelling}` names this CLI and was left to PATH"
            );
            assert_eq!(resolved.program, expected);
            let note = resolved
                .note
                .expect("substituting a binary is never done silently");
            assert!(
                note.contains(&expected) && note.contains(env!("CARGO_PKG_VERSION")),
                "the note names neither the binary nor the build: {note}"
            );
        }

        for other in ["cargo", "bash", "git", "/bin/sh", "protocolol", "sh"] {
            let untouched = resolve_program(other);
            assert_eq!(
                untouched.resolution,
                Resolution::AsWritten,
                "`{other}` is not this CLI and was rewritten anyway"
            );
            assert_eq!(untouched.program, other);
            assert!(
                untouched.note.is_none(),
                "`{other}` resolved as written and still carried a note"
            );
        }
    }
/// The third pre-flight: which maps it looks at, and what it says when it fires.
    ///
    /// The two lookups it sits behind are unreachable from a test — `current_exe()` does not fail
    /// on a machine a suite runs on — so the scan and the message are checked directly. That is
    /// also the honest scope of this test, and it is why the `PathFallback` note above exists: on
    /// the machine where this refusal is wrong to fire, the step still says what it did.
    #[test]
    fn a_driver_that_cannot_name_itself_refuses_a_map_whose_commands_say_protocol() {
        let elsewhere = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/elsewhere\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: command\n        run: [cargo, test]\n",
            None,
        )
        .expect("the map validates");
        assert_eq!(
            protocol_command_steps(&elsewhere),
            0,
            "a map that names no `protocol` is not this check's business"
        );
        assert!(protocol_command_preflight(&elsewhere).is_none());

        let ours = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/ours\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: command\n        run: [cargo, test]\n\
             \x20     - kind: command\n        run: [protocol, artifact, validate]\n\
             \x20     - kind: command\n        run: [/usr/local/bin/protocol, property, evidence]\n",
            None,
        )
        .expect("the map validates");
        assert_eq!(
            protocol_command_steps(&ours),
            2,
            "both spellings of this CLI count and `cargo` does not"
        );
        assert!(
            protocol_command_preflight(&ours).is_none(),
            "this process can name its own binary, so there is nothing to refuse"
        );

        let version = env!("CARGO_PKG_VERSION");
        assert!(
            protocol_command_refusal(2, Some(version)).is_none(),
            "the PATH binary is this build, so the fallback would spawn it and nothing is at stake"
        );

        let stale = protocol_command_refusal(4, Some("0.28.0"))
            .expect("a PATH binary of another version is refused");
        assert!(
            stale.contains("0.28.0") && stale.contains(version),
            "a refusal over two versions names both: {stale}"
        );
        assert!(
            stale.contains("4 `command` step(s)"),
            "and how much of the map is at stake: {stale}"
        );
        assert!(
            stale.contains("cargo install --path crates/edge/aep-cli --root ~/.local")
                && stale.contains("export PATH="),
            "the fix is named, and named correctly: an install alone puts the binary where a \
             *session* looks, and a driver-side PATH is the operator's own shell: {stale}"
        );

        let absent = protocol_command_refusal(1, None)
            .expect("nothing to fall back to is refused for the same reason");
        assert!(
            absent.contains("no `protocol` on that `PATH` at all"),
            "and says that is what it found rather than quoting a version it does not have: \
             {absent}"
        );
    }
}
