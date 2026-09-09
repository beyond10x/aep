#[cfg(test)]
mod tests {
    use crate::money::micro_usd;
use super::*;
use clap::ValueEnum;
/// A manifest that passes every rule, as a starting point for one-line mutations.
    const HONEST: &str = "\
format: eval.run-manifest/1
arm: plugin
harness: claude
workflow: adp/default
case: case:create-a-story
plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332
model: claude-sonnet-5
harness_version: claude 2.1.239
transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b
observed_at: 2026-08-23
";
/// Reads a manifest, returning the refusals rather than a message.
    fn read(text: &str) -> Result<RunManifest, Vec<Refusal>> {
        let raw: RawRunManifest = serde_yaml::from_str(text).expect("the fixture is YAML");
        RunManifest::try_from(raw)
    }
/// The codes a refusal set carries, which is what a test matches on.
    fn codes(refusals: &[Refusal]) -> Vec<&'static str> {
        refusals.iter().map(Refusal::code).collect()
    }
#[test]
    fn the_honest_manifest_is_read_so_every_mutation_below_reaches_its_rule() {
        // The control. Without it a mutation test could be passing because the fixture was broken
        // for some other reason entirely.
        let manifest = read(HONEST).expect("the fixture states every field");
        assert_eq!(manifest.arm, Arm::Plugin);
        assert_eq!(manifest.harness, "claude");
        assert!(manifest.plugin_digest.is_some());
        assert_eq!(manifest.cost_micro_usd, None, "a quantity nobody stated");
    }
#[test]
    fn an_arm_this_evaluation_does_not_have_is_refused_by_name() {
        let refusals = read(&HONEST.replace("arm: plugin", "arm: hybrid"))
            .expect_err("another arm is a change to the programme");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-002"]);
        let sentence = refusals[0].to_string();
        assert!(
            sentence.contains("`raw`, `plugin`, `driven`, `native`"),
            "the refusal lists the arms there are, in programme order: {sentence}"
        );
    }
#[test]
    fn the_arms_the_refusal_lists_are_every_arm_the_type_has() {
        // `Arm::ALL` is the whole content of `EVAL-MANIFEST-002`, so an arm missing from it is an
        // arm the refusal tells a reader does not exist — which is what `native` was between
        // `dce6db5`, the commit that added it to the enum, to `parse` and to `as_str`, and this
        // one. The second list is `ValueEnum`'s, derived from the variants themselves, so this
        // cannot be satisfied by editing a second hand-written array to match the first.
        assert_eq!(
            Arm::ALL.as_slice(),
            <Arm as ValueEnum>::value_variants(),
            "every variant, in declaration order, which is the programme's order"
        );
        for arm in Arm::ALL {
            assert_eq!(
                Arm::parse(arm.as_str()),
                Some(arm),
                "and each word the refusal offers round-trips through `parse`"
            );
        }

        let sentence = Refusal::ArmUnknown {
            written: "b10x".to_owned(),
        }
        .to_string();
        assert!(
            sentence.contains("`native`"),
            "the fourth arm is named where a reader is told what the arms are: {sentence}"
        );
        assert!(
            !sentence.contains("three arms") && !sentence.contains("A fourth arm"),
            "and the sentence states no count, so the next arm does not make it wrong: {sentence}"
        );
    }
#[test]
    fn a_missing_field_is_refused_by_its_own_name_and_every_other_refusal_is_reported_beside_it() {
        // Invariant 3: a document with four broken fields reports four refusals. A reader who has
        // to run the verb four times to find four typos stops running it.
        //
        // `model` stays in this set deliberately after it became a `Written`: an *absent* model is
        // still refused, and keeping it here proves `written_or_null` accumulates beside `required`
        // rather than short-circuiting the pass.
        let stripped = HONEST
            .lines()
            .filter(|line| {
                !line.starts_with("model:")
                    && !line.starts_with("harness_version:")
                    && !line.starts_with("case:")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let refusals = read(&stripped).expect_err("three fields are missing");
        assert_eq!(
            codes(&refusals),
            [
                "EVAL-MANIFEST-003",
                "EVAL-MANIFEST-003",
                "EVAL-MANIFEST-003"
            ]
        );
        let named: Vec<String> = refusals.iter().map(ToString::to_string).collect();
        for field in ["case", "model", "harness_version"] {
            assert!(
                named
                    .iter()
                    .any(|line| line.contains(&format!("`{field}`"))),
                "each refusal names its own field: {named:?}"
            );
        }
    }
#[test]
    fn an_omitted_plugin_digest_is_refused_and_an_explicit_null_is_not() {
        // The rule the shape exists for. Serde maps `null` and *absent* onto the same `None`, so
        // without `written_down` a manifest that forgot the key would read as a run that had no
        // plugin — and arm `raw`'s whole claim is that it had none.
        let omitted = HONEST
            .lines()
            .filter(|line| !line.starts_with("plugin_digest:"))
            .collect::<Vec<_>>()
            .join("\n");
        let refusals = read(&omitted).expect_err("the key must be written");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-003"]);

        let raw_arm = HONEST.replace("arm: plugin", "arm: raw").replace(
            "plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332",
            "plugin_digest: null",
        );
        let manifest = read(&raw_arm).expect("an explicit null on arm raw is the honest form");
        assert_eq!(manifest.arm, Arm::Raw);
        assert_eq!(manifest.plugin_digest, None);
    }
#[test]
    fn a_model_that_is_written_and_empty_is_refused_rather_than_read_as_unstated() {
        // The boundary between the two answers above. Without this an empty string would slip
        // through as `Some("")` and print as a blank model column, which reads like a rendering
        // bug rather than a fact.
        let refusals = read(&HONEST.replace("model: claude-sonnet-5", "model: \"\""))
            .expect_err("an empty model names nothing");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-004"]);
    }
#[test]
    fn a_plugin_digest_on_arm_raw_is_refused_because_arm_raw_is_the_arm_without_one() {
        let refusals = read(&HONEST.replace("arm: plugin", "arm: raw"))
            .expect_err("arm raw carries no plugin");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-005"]);
    }
#[test]
    fn a_null_plugin_digest_on_arm_plugin_is_refused_because_the_plugin_is_the_subject() {
        let refusals = read(&HONEST.replace(
            "plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332",
            "plugin_digest: null",
        ))
        .expect_err("arm plugin must say which plugin");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-006"]);
    }
/// The same fixture with the directory treatment removed and a marketplace plugin named.
    ///
    /// Written out rather than patched with `replace`, because the two treatments differ by two
    /// keys at once and a patch that changed one of them would be testing a manifest nothing
    /// writes.
    const MARKETPLACE: &str = "\
format: eval.run-manifest/1
arm: plugin
harness: claude
workflow: adp/default
case: case:create-a-story
plugin_digest: null
plugins:
  - plugin: bdfinst/agentic-dev-team@dev-team@1.4.0
    digest: c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3
model: claude-sonnet-5
harness_version: claude 2.1.239
transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b
observed_at: 2026-08-23
";
#[test]
    fn a_marketplace_plugin_is_a_treatment_and_a_run_with_one_needs_no_directory_digest() {
        // The arm `plugin` rule is about the **treatment**, not about one of the two mechanisms
        // that deliver it. A run whose plugin came from a marketplace has no directory to digest,
        // and `plugin_digest: null` there is a stated absence rather than a hole.
        let manifest = read(MARKETPLACE).expect("a marketplace plugin is a plugin");
        assert_eq!(manifest.plugin_digest, None);
        assert_eq!(
            manifest.plugins.first().map(|plugin| plugin.plugin.as_str()),
            Some("bdfinst/agentic-dev-team@dev-team@1.4.0")
        );
    }
#[test]
    fn arm_plugin_that_names_neither_mechanism_is_still_refused() {
        // The boundary of the rule above: it widened what counts as a plugin, and it must not have
        // widened it to nothing.
        let refusals = read(&MARKETPLACE.replace(
            "plugins:\n  - plugin: bdfinst/agentic-dev-team@dev-team@1.4.0\n    digest: \
             c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3\n",
            "",
        ))
        .expect_err("arm plugin must say which plugin");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-006"]);
    }
#[test]
    fn a_marketplace_plugin_on_arm_raw_is_refused_for_the_reason_a_digest_is() {
        let refusals = read(&MARKETPLACE.replace("arm: plugin", "arm: raw"))
            .expect_err("arm raw is the arm with no plugin in it");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-008"]);
    }
#[test]
    fn a_marketplace_plugin_whose_digest_is_not_one_is_refused_by_the_field_that_is_wrong() {
        let refusals = read(
            &MARKETPLACE.replace(
                "c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3",
                "0.4.0",
            ),
        )
        .expect_err("a version is not a digest");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-007"]);
    }
#[test]
    fn arm_driven_may_answer_either_way_because_the_enforcer_is_not_the_plugin() {
        // The boundary of the two rules above. Without this test they could have been written as
        // one rule — *a digest exactly when the arm is not raw* — and every other test would still
        // pass.
        let with = read(&HONEST.replace("arm: plugin", "arm: driven"))
            .expect("a driven run may also have had the plugin installed");
        assert!(with.plugin_digest.is_some());

        let without = read(&HONEST.replace("arm: plugin", "arm: driven").replace(
            "plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332",
            "plugin_digest: null",
        ))
        .expect("and it may not have");
        assert_eq!(without.plugin_digest, None);
    }
#[test]
    fn a_document_that_does_not_claim_the_format_is_refused_before_its_fields_are_believed() {
        let refusals = read(&HONEST.replace("eval.run-manifest/1", "eval.run-manifest/2"))
            .expect_err("a version this build does not know");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-001"]);
        assert!(
            refusals[0].to_string().contains("eval.run-manifest/2"),
            "the refusal quotes what it was handed: {}",
            refusals[0]
        );
    }
#[test]
    fn a_digest_that_is_not_a_digest_is_refused() {
        let refusals = read(&HONEST.replace(
            "transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b",
            "transcript_digest: sha256:6522e1",
        ))
        .expect_err("a digest is 64 hex characters");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-007"]);
    }
/// The honest manifest, as a value, with the arm and transcript given.
    fn manifest_of(arm: Arm, transcript: &str) -> RunManifest {
        RunManifest {
            arm,
            harness: "claude".to_owned(),
            workflow: "adp/default".to_owned(),
            case: "case:create-a-story".to_owned(),
            plugin_digest: None,
            plugins: Vec::new(),
            model: Some("claude-sonnet-5".to_owned()),
            model_requested: None,
            harness_version: "claude 2.1.239".to_owned(),
            transcript_digest: transcript.to_owned(),
            observed_at: "2026-08-23".to_owned(),
            cost_micro_usd: Some(1_500_000),
            tokens: Some(10),
            wall_time_ms: Some(20),
        }
    }
/// A record value with one row.
    fn record_value(transcript: &str, digest: &str, outcome: Outcome) -> Record {
        Record {
            specification: "eval/development-story".to_owned(),
            spec_digest: digest.to_owned(),
            transcript_digest: transcript.to_owned(),
            rows: vec![("only".to_owned(), outcome)],
        }
    }
#[test]
    fn a_manifest_that_describes_another_run_than_its_record_is_refused() {
        let pairs = vec![(
            manifest_of(Arm::Raw, &"a".repeat(DIGEST_WIDTH)),
            record_value(&"b".repeat(DIGEST_WIDTH), "d", Outcome::Held),
        )];
        let refusals = assemble(pairs).expect_err("the two documents are about different runs");
        assert_eq!(codes(&refusals), ["EVAL-PAIR-003"]);
    }
#[test]
    fn one_transcript_cannot_arrive_twice_because_one_run_would_be_counted_twice() {
        let digest = "c".repeat(DIGEST_WIDTH);
        let pairs = vec![
            (
                manifest_of(Arm::Raw, &digest),
                record_value(&digest, "d", Outcome::Held),
            ),
            (
                manifest_of(Arm::Plugin, &digest),
                record_value(&digest, "d", Outcome::Held),
            ),
        ];
        let refusals = assemble(pairs).expect_err("two runs are two transcripts");
        assert_eq!(codes(&refusals), ["EVAL-PAIR-004"]);
    }
#[test]
    fn one_specification_at_two_digests_is_refused_because_the_rows_share_a_name_only() {
        let pairs = vec![
            (
                manifest_of(Arm::Raw, &"e".repeat(DIGEST_WIDTH)),
                record_value(&"e".repeat(DIGEST_WIDTH), "before", Outcome::Held),
            ),
            (
                manifest_of(Arm::Plugin, &"f".repeat(DIGEST_WIDTH)),
                record_value(&"f".repeat(DIGEST_WIDTH), "after", Outcome::Violated),
            ),
        ];
        let refusals = assemble(pairs).expect_err("the document moved between the two runs");
        assert_eq!(codes(&refusals), ["EVAL-PAIR-005"]);
    }
#[test]
    fn a_cells_resource_total_says_how_many_runs_it_covers() {
        // The reason a column is a pair and not a number: a total over two of three runs read as a
        // total over three would understate the arm it describes.
        let mut quiet = manifest_of(Arm::Driven, &"1".repeat(DIGEST_WIDTH));
        quiet.cost_micro_usd = None;
        quiet.tokens = None;
        quiet.wall_time_ms = None;
        let pairs = vec![
            (
                manifest_of(Arm::Driven, &"0".repeat(DIGEST_WIDTH)),
                record_value(&"0".repeat(DIGEST_WIDTH), "d", Outcome::Held),
            ),
            (
                quiet,
                record_value(&"1".repeat(DIGEST_WIDTH), "d", Outcome::Unobservable),
            ),
        ];
        let matrix = assemble(pairs).expect("two distinct runs of one cell");
        assert_eq!(matrix.cells.len(), 1);
        let cell = &matrix.cells[0];
        assert_eq!(cell.runs, 2);
        assert_eq!(
            cell.resources.cost_micro_usd,
            Some(Reported {
                runs: 1,
                total: 1_500_000
            })
        );
        assert_eq!(cell.counts.held, 1);
        assert_eq!(cell.counts.unobservable, 1);
        assert_eq!(
            cost(cell.resources.cost_micro_usd, cell.runs),
            "$1.500000 (1/2)",
            "the rendering says what the total covers"
        );
    }
#[test]
    fn the_arms_sort_in_the_order_the_experiment_runs_them() {
        // Alphabetically this is `driven`, `plugin`, `raw`, which reads the experiment backwards.
        let mut arms = vec![Arm::Driven, Arm::Raw, Arm::Plugin];
        arms.sort_unstable();
        assert_eq!(arms, vec![Arm::Raw, Arm::Plugin, Arm::Driven]);
    }
#[test]
    fn no_rendering_of_a_matrix_contains_a_score() {
        // The programme's one prohibition, asserted on the bytes rather than trusted to review.
        let pairs = vec![(
            manifest_of(Arm::Raw, &"2".repeat(DIGEST_WIDTH)),
            record_value(&"2".repeat(DIGEST_WIDTH), "d", Outcome::Held),
        )];
        let matrix = assemble(pairs).expect("one run");
        let text = to_text(&matrix);
        let json = serde_json::to_string(&matrix).expect("the matrix serialises");
        for rendering in [&text, &json] {
            assert!(
                !rendering.contains('%'),
                "no percentage reaches an output: {rendering}"
            );
            assert_eq!(
                rendering.matches("score").count(),
                rendering.matches("no score is computed").count(),
                "and the only occurrence of the word is the sentence saying there is none: \
                 {rendering}"
            );
        }
        assert!(
            text.contains("No arm is ranked and no score is computed"),
            "and the text rendering says so where a reader will look for one: {text}"
        );
    }
#[test]
    fn a_table_holding_a_native_cell_says_how_to_read_it_and_one_without_stays_silent() {
        // The reading rule of `docs/design/native-arm-store-integrity-design-v0.1.md` § 6 O1: the
        // arm word is the only enforcement label a cell gets, and on this arm a clean row is
        // compliance rather than a refusal. Printed as a line under the table it qualifies, not as
        // a column — § 8 OQ4 leaves the column to the operator.
        let native = to_text(
            &assemble(vec![(
                manifest_of(Arm::Native, &"7".repeat(DIGEST_WIDTH)),
                record_value(&"7".repeat(DIGEST_WIDTH), "d", Outcome::Held),
            )])
            .expect("one native run"),
        );
        assert!(
            native.contains("reading a `native` cell")
                && native.contains("never enforced")
                && native.contains("nobody asked"),
            "a native cell carries the rule for reading it: {native}"
        );

        let driven = to_text(
            &assemble(vec![(
                manifest_of(Arm::Driven, &"8".repeat(DIGEST_WIDTH)),
                record_value(&"8".repeat(DIGEST_WIDTH), "d", Outcome::Held),
            )])
            .expect("one driven run"),
        );
        assert!(
            !driven.contains("reading a `native` cell"),
            "and a table with no native cell in it says nothing about one: {driven}"
        );
    }
#[test]
    fn an_amount_becomes_millionths_by_integer_arithmetic_and_never_by_a_float() {
        // The one-line mutation this guards is `(value * 1_000_000.0) as u64`, which turns the
        // cost this repository's own fixtures carry — `0.0714` — into `71399`. A cent lost per run
        // is a budget that overspends, and a manifest that will not reproduce.
        assert_eq!(micro_usd("0.0714").expect("a cost off the wire"), 71_400);
        assert_eq!(micro_usd("0.4137").expect("another"), 413_700);
        assert_eq!(
            micro_usd("10").expect("a whole number of dollars"),
            10_000_000
        );
        assert_eq!(micro_usd("0.25").expect("the assumed rate"), 250_000);
        assert_eq!(
            micro_usd("$1.00").expect("a dollar sign is tolerated"),
            1_000_000
        );
        assert_eq!(micro_usd("0").expect("nothing at all"), 0);
    }
#[test]
    fn an_amount_this_reader_cannot_convert_exactly_is_refused_rather_than_rounded() {
        // Scientific notation and a seventh decimal place are both *nearly* readable, which is
        // what makes silently approximating them tempting. A cost that cannot be converted exactly
        // does not belong in a document somebody commits.
        for written in ["1e-7", "0.1234567", "", "ten", "1.2.3", "-3"] {
            assert!(
                micro_usd(written).is_err(),
                "`{written}` is not an amount this reader will convert"
            );
        }
    }
#[test]
    fn the_terminal_events_cost_is_read_from_its_own_decimal_text() {
        let ended = serde_json::json!({ "total_cost_usd": 0.0714 });
        assert_eq!(cost_of(&ended), Ok(Some(71_400)));
        // Written `null` and absent are the same answer, and neither is zero: the manifest states
        // no cost at all, and the matrix's total then says how many runs it covers.
        assert_eq!(
            cost_of(&serde_json::json!({ "total_cost_usd": null })),
            Ok(None)
        );
        assert_eq!(cost_of(&serde_json::json!({})), Ok(None));
    }
#[test]
    fn a_stated_cost_this_reader_cannot_convert_is_refused_rather_than_read_as_no_cost() {
        // The half of the defect that made it silent: `.ok()` collapsed *there is a number here I
        // cannot convert* into *there is no number*, and the second is the one the ledger is
        // allowed to charge an estimate for. Unreadable is not unstated.
        assert!(micro_usd_stated("1e-7").is_err(), "an exponent is refused");
        let reason = cost_of(&serde_json::json!({ "total_cost_usd": "0.80" }))
            .expect_err("a cost written as a string is not a number");
        assert!(
            reason.contains("neither a number nor `null`"),
            "and the refusal says which two answers there are: {reason}"
        );
    }
#[test]
    fn a_person_typing_an_amount_is_still_held_to_an_exact_one() {
        // The reason there are two readers rather than one loosened one. A wire computes and may
        // hand over float noise; a person types, and `--budget-usd 1e-7` is a mistake worth naming
        // rather than a cap silently rounded to nothing.
        assert!(micro_usd("1e-7").is_err());
        assert!(micro_usd("0.1234567").is_err());
        assert_eq!(
            micro_usd_stated("0.1234567"),
            Ok(123_457),
            "while the same text off a wire is rounded to the nearest millionth"
        );
    }
#[test]
    fn a_usage_key_written_null_contributes_nothing_and_does_not_erase_the_total() {
        let usage = serde_json::json!({
            "usage": { "input_tokens": 14, "output_tokens": 1128,
                       "cache_read_input_tokens": null, "cache_creation_input_tokens": 20168 }
        });
        assert_eq!(tokens_of(&usage), Some(14 + 1128 + 20168));
        assert_eq!(
            tokens_of(&serde_json::json!({ "usage": {} })),
            None,
            "and a usage object that states none of the four states no total"
        );
    }
#[test]
    fn a_case_whose_subject_names_the_ess_skill_says_it_needs_ess() {
        let directory = std::env::temp_dir().join("aep-eval-case-needs-ess");
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("scratch");
        std::fs::write(
            directory.join("case.yaml"),
            "format: eval-case/1\nid: aep-eval-case-needs-ess\nworkflow: adp/default\n\
             task: draft the domain\nexpectations: expectations.trace.yaml\n\
             subject:\n  skills: [aep-plan:planning, ess-specify:specify]\n",
        )
        .expect("written");
        let case = read_case(&directory).expect("a case");
        assert!(case.needs_ess);

        std::fs::write(
            directory.join("case.yaml"),
            "format: eval-case/1\nid: aep-eval-case-needs-ess\nworkflow: adp/default\n\
             task: draft the domain\nexpectations: expectations.trace.yaml\n",
        )
        .expect("written");
        assert!(!read_case(&directory).expect("a case").needs_ess);
    }
#[test]
    fn both_spellings_of_the_ess_plugin_trip_the_preflight() {
        // The plugin was renamed `ess-schema` → `ess-specify` in `agentplugins@a2077d2`. A
        // preflight keyed on one spelling is a preflight that stops firing the day a case is
        // written under the other, and the case that stops firing is the one that spawns and pays
        // on a runner with no `ess` — which is the hazard the agentplugins adversary recorded
        // against this exact line on 2026-09-03. Both spellings, one loop, so a third never gets
        // added to the matcher without a case here.
        let directory = std::env::temp_dir().join("aep-eval-case-needs-ess-both-spellings");
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("scratch");
        assert!(
            ESS_SKILL_PREFIXES.contains(&"ess-specify:")
                && ESS_SKILL_PREFIXES.contains(&"ess-schema:"),
            "the current id and the one the corpus was authored under, both: {ESS_SKILL_PREFIXES:?}"
        );
        for prefix in ESS_SKILL_PREFIXES {
            let skill = format!("{prefix}a-skill");
            std::fs::write(
                directory.join("case.yaml"),
                format!(
                    "format: eval-case/1\nid: aep-eval-case-needs-ess\nworkflow: adp/default\n\
                     task: draft the domain\nexpectations: expectations.trace.yaml\n\
                     subject:\n  skills: [aep-plan:planning, {skill}]\n"
                ),
            )
            .expect("written");
            assert!(
                read_case(&directory).expect("a case").needs_ess,
                "`{skill}` names the plugin whose step runs `ess`, so the case needs `ess`"
            );
        }

        // A skill whose plugin merely starts with the same letters is not that plugin.
        std::fs::write(
            directory.join("case.yaml"),
            "format: eval-case/1\nid: aep-eval-case-needs-ess\nworkflow: adp/default\n\
             task: draft the domain\nexpectations: expectations.trace.yaml\n\
             subject:\n  skills: [ess-specifier:specify]\n",
        )
        .expect("written");
        assert!(
            !read_case(&directory).expect("a case").needs_ess,
            "the prefix is `<plugin>:`, not a bare stem"
        );

        // The person who reads the refusal has to be able to tell which spelling tripped it, so
        // the message names every spelling the matcher accepts.
        let refusal = RunRefusal::ChildEssMissing {
            case: "a-case".to_owned(),
            child_path: "/usr/bin".to_owned(),
        }
        .to_string();
        assert!(refusal.starts_with("EVAL-RUN-018"), "{refusal}");
        for spelling in ESS_SKILL_PREFIXES {
            assert!(
                refusal.contains(spelling),
                "the refusal names `{spelling}`, which is one of the spellings that trips it: \
                 {refusal}"
            );
        }
    }
fn plan_of(arm: Arm, harness: Harness) -> Plan {
        Plan {
            case: Case {
                id: "development-honest".to_owned(),
                workflow: "adp/default".to_owned(),
                task: "Add a `--json` flag.".to_owned(),
                expectations: PathBuf::from(
                    "conformance/eval/development-honest/expectations.trace.yaml",
                ),
                needs_ess: false,
            },
            arm,
            harness,
            plugins: Vec::new(),
            model_requested: None,
        }
    }
#[test]
    fn the_manifest_the_runner_assembles_is_one_the_matrixs_own_reader_reads() {
        // The round trip, at the level of the two functions rather than through the binary. The
        // failure it exists to catch is a quoting or key-order mistake that would otherwise sit
        // undetected until a sweep had been paid for.
        let session = Session {
            harness_version: "claude 2.1.239".to_owned(),
            model: Some("claude-sonnet-5".to_owned()),
            plugin_digest: Some("a".repeat(DIGEST_WIDTH)),
            plugins: Vec::new(),
            cost_micro_usd: Some(521_600),
            tokens: Some(116_546),
            wall_time_ms: Some(22_320),
        };
        let text = manifest_text(
            &plan_of(Arm::Plugin, Harness::Claude),
            &session,
            &"b".repeat(DIGEST_WIDTH),
            "2026-08-23",
        );
        let manifest = read(&text).expect("the runner writes manifests its own reader reads");
        assert_eq!(manifest.arm, Arm::Plugin);
        assert_eq!(manifest.case, "case:development-honest");
        assert_eq!(manifest.plugin_digest, Some("a".repeat(DIGEST_WIDTH)));
        assert_eq!(manifest.cost_micro_usd, Some(521_600));
    }
#[test]
    fn one_session_that_wrote_two_terminal_records_is_charged_once() {
        // The budget-kill shape, and the other half of the fold. `--max-budget-usd` stops a session
        // *after* it has written its `result`, so the stream carries a second terminal record
        // saying `error_max_budget_usd` — and both restate the same running counters. Summing them
        // reported `$30.002816` for the golden-path run of 2026-09-03, which spent `$15.00140784`.
        //
        // The second record also zeroes its `usage` while restating the cost and a shorter wall
        // clock, which is why the fold takes the larger of the two rather than the last.
        let one = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/eval-run/claude-driven-attested.jsonl"),
        )
        .expect("the committed driven fixture");
        let mut killed = one.clone();
        killed.extend_from_slice(
            concat!(
                r#"{"format":"metaharness.event/1","seq":47,"run":"R3-4/claude-driven","#,
                r#""event":"session.ended","is_error":true,"subtype":"error_max_budget_usd","#,
                r#""terminal_reason":"budget_exhausted","num_turns":1,"duration_ms":12532,"#,
                r#""total_cost_usd":0.5216,"usage":{"input_tokens":0,"output_tokens":0,"#,
                r#""cache_read_input_tokens":0,"cache_creation_input_tokens":0}}"#,
                "\n"
            )
            .as_bytes(),
        );

        let single =
            Session::read(&one, Arm::Driven, Harness::Claude, &[]).expect("a readable stream");
        let stopped = Session::read(&killed, Arm::Driven, Harness::Claude, &[])
            .expect("the same session, stopped at its cap");

        assert_eq!(
            stopped.cost_micro_usd, single.cost_micro_usd,
            "one session spent one figure, however many times it stated it"
        );
        assert_eq!(
            stopped.tokens, single.tokens,
            "and used what it used: the stopping record's zeroed usage is not the session's"
        );
        assert_eq!(
            stopped.wall_time_ms, single.wall_time_ms,
            "and ran as long as its longest statement, not the sum of two"
        );
    }
}
#[cfg(test)]
mod native_arm_tests {
use super::*;
#[test]
    fn the_fourth_arm_is_a_word_the_manifest_reads_and_writes() {
        assert_eq!(Arm::parse("native"), Some(Arm::Native));
        assert_eq!(Arm::Native.as_str(), "native");
        assert_eq!(Harness::B10x.as_str(), "b10x");
    }
#[test]
    fn the_arms_still_sort_in_the_order_the_experiment_runs_them() {
        // `native` last, because it is the arm that removes the vendor loop entirely and every
        // other arm is a treatment applied to one.
        let mut arms = vec![Arm::Native, Arm::Driven, Arm::Raw, Arm::Plugin];
        arms.sort();
        assert_eq!(arms, vec![Arm::Raw, Arm::Plugin, Arm::Driven, Arm::Native]);
    }
#[test]
    fn a_native_run_is_refused_a_spawn_and_told_what_does_launch_it() {
        // Same position as `driven` and a different reason, which the message has to carry: a
        // driven run is launched by `protocol drive run` because there must be one policy; a
        // native run is launched by `b10x-harness` because it *is* the loop and there is no vendor
        // harness here to drive.
        let refusal = RunRefusal::NativeIsNotLaunchedHere.to_string();
        assert!(refusal.starts_with("EVAL-RUN-011"), "{refusal}");
        assert!(refusal.contains("b10x-harness"), "{refusal}");
        assert!(refusal.contains("--arm native --stream"), "{refusal}");
        assert!(
            refusal.contains("no vendor harness in it"),
            "and says why it differs from driven: {refusal}"
        );
    }
#[test]
    fn every_arm_has_a_code_of_its_own_and_none_is_reused() {
        let codes = [
            RunRefusal::NotLive.code(),
            RunRefusal::NoBudget.code(),
            RunRefusal::DrivenIsNotLaunchedHere.code(),
            RunRefusal::NativeIsNotLaunchedHere.code(),
        ];
        let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();
        assert_eq!(unique.len(), codes.len(), "{codes:?}");
    }
}
