//! Adversary pass 1 for `story:ess-grant-scenario-ids`: `ScenarioId::new` against what ESS 0.55.0's
//! own `ScenarioId::parse` decided for the same strings.
//!
//! Each verdict was measured, not transcribed from ESS's source: every id was given alone, as a
//! one-element array, to `ess 0.55.0 verify conform select --ids`, which parses the array as
//! scenario ids before it looks any of them up. ESS answered an id it parsed with "explicit ID
//! absent from parent" and one it did not with "invalid scenario id". The grammar is the same at
//! every suite major, so the suite the ids were checked against does not decide anything here.
use aep_domain::ess_conformance_v2::ScenarioId;

/// Each id, and whether ESS 0.55.0 parsed it.
const MEASURED: [(&str, bool); 109] = [
    ("a/grant/denied", true),
    ("a.b/grant/denied", true),
    ("A-b.C_d/grant/denied", true),
    ("a.b/grant/admitted/c", true),
    ("a.b/grant/admitted/c-d_e.F", true),
    ("a.b/grant/admitted/1c", false),
    ("a.b/grant/admitted/c.", false),
    ("a.b/grant/admitted/.c", false),
    ("a.b/grant/read/denied", true),
    ("a.b/grant/read/admitted/c", true),
    ("a.b/grant/read/admitted/read", true),
    ("a.b/grant/admitted/read", true),
    ("a.b/grant/read/admitted/admitted", true),
    ("read/grant/read/denied", true),
    ("a.b/grant/denied/", false),
    ("a.b//grant/denied", false),
    ("a.b/grant//denied", false),
    ("a.b/grant/Denied", false),
    ("a.b/Grant/denied", false),
    ("a.b/grant/read/Denied", false),
    ("a.b/grant/write/denied", false),
    ("a.b/grant/denied/c", false),
    ("a.b/grant/read/admitted", false),
    (" a.b/grant/denied", false),
    ("a.é/grant/denied", false),
    ("a.b/grant/admitted/c d", false),
    ("a/binding/refusal/b", true),
    ("a-b/binding/refusal/c-d1", true),
    ("a1/binding/refusal/x", true),
    ("1a/binding/refusal/x", false),
    ("a/binding/refusal/x-", false),
    ("a/binding/refusal/x--y", false),
    ("a/binding/refusal/X", false),
    ("a/binding/refusal/x_y", false),
    ("a/binding/refusal/x.y", false),
    ("a.b/binding/refusal/x", false),
    ("a/binding/refusal/final-failure", true),
    ("a/binding/refusal/refusal", true),
    ("a/binding/refusal", false),
    ("a/binding/refusal/x/y", false),
    ("a/binding/condition-false", true),
    ("a/binding/condition-absent", true),
    ("a/binding/final-failure", true),
    ("a/binding/condition", false),
    ("a/binding/Condition-false", false),
    ("a.b/binding/condition-false", false),
    ("a/binding/refusal/x/as/anonymous", false),
    ("a.B/disclosure/o/f/origin/as/anonymous", true),
    ("a.B/disclosure/o/f/retry/as/anonymous", true),
    ("a.B/disclosure/o/f/rotation/as/actor/c.D", true),
    ("a.B/disclosure/o/f/read/v.W/as/anonymous", true),
    ("a.B/disclosure/o/f/command/c.D/e/as/anonymous", true),
    ("a.B/disclosure/o/f/denied/c.D/as/actor/e.F", true),
    ("a.B/disclosure/o/_f/origin/as/anonymous", true),
    ("a.B/disclosure/o/__f1_/origin/as/anonymous", true),
    ("a.B/disclosure/o/_/origin/as/anonymous", false),
    ("a.B/disclosure/o/__/origin/as/anonymous", false),
    ("a.B/disclosure/o/_1/origin/as/anonymous", false),
    ("a.B/disclosure/o/F/origin/as/anonymous", true),
    ("a.B/disclosure/o/f-g/origin/as/anonymous", false),
    ("a.B/disclosure/o/f.g/origin/as/anonymous", false),
    ("a.B/disclosure/O/f/origin/as/anonymous", false),
    ("a.B/disclosure/o-1/f/origin/as/anonymous", true),
    ("a.B/disclosure/o/f/origin/as/actor/anonymous", true),
    ("a.B/disclosure/o/f/origin/as/actor/as", true),
    ("a.B/disclosure/o/f/origin/as/actor/as/anonymous", false),
    ("a.B/disclosure/o/f/as/anonymous", false),
    ("a.B/disclosure/o/f/origin/origin/as/anonymous", false),
    ("a.B/disclosure/o/f/command/c.D/as/anonymous", false),
    ("a.B/disclosure/o/f/command/c.D/E/as/anonymous", false),
    ("a.B/disclosure/o/f/read/v.W/x/as/anonymous", false),
    ("a.B/disclosure/o/f/denied/c.D/as/anonymous", true),
    ("a.B/disclosure/o/f/denied/1c/as/anonymous", false),
    ("a.B/disclosure/o/f/origin/as/actor/c.D/x", false),
    ("a.B/disclosure/o/f/origin/as/Anonymous", false),
    ("a.B/disclosure/o/f/origin/AS/anonymous", false),
    ("1a/disclosure/o/f/origin/as/anonymous", false),
    ("a..B/disclosure/o/f/origin/as/anonymous", false),
    ("/disclosure/o/f/origin/as/anonymous", false),
    ("a.B/disclosure/o/f/read/disclosure/as/anonymous", true),
    ("a.B/disclosure/o/f/command/c.D/o/as/actor/as", true),
    ("a.B/disclosure", false),
    ("a.B/disclosure/o/f/origin/as/actor/c.D/as/anonymous", false),
    ("a.B/disclosure/o/f/read/v.W/as/actor/as/actor/c.D", false),
    ("a.B/disclosure/o/f/origin/as/actor/c-d.E_f", true),
    ("a.B/disclosure/o/f/origin/as/actor/c..D", false),
    ("disclosure/disclosure/o/f/origin/as/anonymous", true),
    ("a.B/disclosure/o/f/denied/c.D/as/actor/anonymous", true),
    ("a/aggregate", true),
    ("a.B/aggregate/", false),
    ("a.B/aggregate/x", false),
    ("1a/aggregate", false),
    ("a.B/outcome/x", true),
    ("a/binding/flow", true),
    ("a.b/binding/flow", false),
    ("a.B/state/S1/accepts/c.D", true),
    ("a.B/invariant/at/v.W/x.y", true),
    ("a.B/authored/x-1", true),
    ("a/authored/X", false),
    ("a.B/transition/t/by/c.D/o", true),
    ("a.B/transition/t.u/by/c.D/o", false),
    ("a.B/state/s/refuses/c.D", false),
    ("a.B/state/S_1/refuses/c.D", false),
    ("a.B/invariant/at/v.W/", false),
    ("a.B/invariant/at/v.W/x y", true),
    ("a.B/transition/t-u/by/c.D/o", true),
    ("a.B/transition/T/by/c.D/o", true),
    ("a.b/grant/denied ", false),
    ("a.b/grant/admitted/c\t", false),
];

#[test]
fn scenario_ids_are_admitted_exactly_where_ess_0_55_parses_them() {
    let disagreements: Vec<String> = MEASURED
        .iter()
        .filter(|(id, parsed)| ScenarioId::new(*id).is_ok() != *parsed)
        .map(|(id, parsed)| {
            let ess = if *parsed { "parses" } else { "refuses" };
            format!("{id:?}: ESS 0.55.0 {ess} it, AEP does not agree")
        })
        .collect();
    assert!(disagreements.is_empty(), "{disagreements:#?}");
    for (id, _) in MEASURED.iter().filter(|(_, parsed)| !parsed) {
        let error = ScenarioId::new(*id).expect_err(id);
        assert_eq!(error.issues[0].reason, "MalformedScenarioId", "{id:?}");
    }
}
