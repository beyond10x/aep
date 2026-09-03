//! Review finding **F3**, as a test: the tool set is the *decision*, not one of its three inputs.
//!
//! The mutation this file exists to kill is one line long. The first draft of D3(a) derived an
//! `llm` step's tools from `CapabilityPolicy::allow` and called that invariant 6's ordering. It is
//! not: the ordering lives in `CapabilityPolicy::decide`, the three sets are independent — `grant`
//! extends all three — and membership is by `covers` rather than equality.

use aep_domain::capability::{
    Audience, Capability, CapabilityDecision, CapabilityPolicy, Environment,
};
use aep_driver::tool::{tool_config, TOOL_CANDIDATES};

/// The pairing the shipped profiles avoid **in a comment**, which is why it is a fixture here.
///
/// `allow` holds unscoped `deployment.create`, which parses to `Deploy(Environment::Any)`;
/// `approval_required` holds the scoped `deployment.create:production` that
/// `principles/governance/approval-gates.yaml` gates and the protocol floor re-checks.
fn policy_with_a_wide_grant_and_a_narrow_gate() -> CapabilityPolicy {
    CapabilityPolicy {
        allow: [
            Capability::Deploy(Environment::Any),
            Capability::RepositoryWrite,
        ]
        .into_iter()
        .collect(),
        approval_required: [Capability::Deploy(Environment::Production)]
            .into_iter()
            .collect(),
        deny: [Capability::ProductionWrite].into_iter().collect(),
    }
}

#[test]
fn a_wide_allow_entry_does_not_hand_out_a_narrowly_gated_deploy() {
    let policy = policy_with_a_wide_grant_and_a_narrow_gate();

    // The shape of the implementation this kills: `policy.allow.iter().cloned().collect()`. That
    // reads `deployment.create` out of `allow`, hands over the tool, and the model can then deploy
    // to production — the exact grant a principle put behind a human approval. It passes every
    // other test in this file.
    let tools = tool_config(&policy);
    let admitted: Vec<&Capability> = tools
        .capabilities()
        .iter()
        .filter(|capability| matches!(capability, Capability::Deploy(_) | Capability::Rollback(_)))
        .collect();
    assert!(
        admitted.is_empty(),
        "no deployment capability may be offered when one of them is gated: {admitted:?}"
    );

    // And the decision it rests on, asserted separately so a failure says which half moved.
    assert_eq!(
        policy.decide(&Capability::Deploy(Environment::Production)),
        CapabilityDecision::RequiresApproval,
        "`decide` is the one function that owns invariant 6's ordering"
    );
    assert!(
        policy.allow.contains(&Capability::Deploy(Environment::Any)),
        "the fixture's whole point is that `allow` does hold a deploy grant"
    );
}

#[test]
fn a_capability_that_was_never_granted_is_no_tool_and_a_denied_one_is_not_either() {
    let policy = policy_with_a_wide_grant_and_a_narrow_gate();
    let tools = tool_config(&policy);

    assert!(
        !tools.shell_offered(),
        "this fixture never granted `command.execute`, and `NotGranted` maps to no tool. The \
         reason stated here used to be `no development profile grants command.execute`, which is \
         false: `profiles/development-driven.yaml:78` grants it deliberately so a driven step can \
         reach the `protocol` CLI. The property is the mechanism — a shell is rendered exactly \
         when `command.execute` is admitted — and `tests/shell_echo.rs` asserts it both ways"
    );
    assert!(
        !tools.admits(&Capability::CommandExecution),
        "`NotGranted` maps to no tool"
    );
    assert!(
        !tools.admits(&Capability::ProductionWrite),
        "`Denied` maps to no tool"
    );
    assert!(
        tools.admits(&Capability::RepositoryWrite),
        "`Allowed` is the only decision that maps to a tool"
    );
}

#[test]
fn every_simple_capability_is_a_candidate_and_the_scoped_ones_ask_about_production() {
    for capability in Capability::SIMPLE {
        assert!(
            TOOL_CANDIDATES.contains(capability),
            "`{capability}` is a simple capability the tool table would never ask about; the two \
             lists have drifted, and a capability nobody asks about is one nobody can be offered"
        );
    }
    assert_eq!(
        TOOL_CANDIDATES.len(),
        Capability::SIMPLE.len() + Capability::SCOPED.len(),
        "the candidates are the simple capabilities plus one for each that takes a scope"
    );
    for scoped in [
        Capability::Deploy(Environment::Production),
        Capability::Rollback(Environment::Production),
        Capability::NetworkRead(Audience::Private),
    ] {
        assert!(
            TOOL_CANDIDATES.contains(&scoped),
            "`{scoped}` must be asked about at its strictest scope: `covers` widens from the \
             wildcard outwards, so asking about `Any` would step around a narrow denial or gate"
        );
    }
    for wildcard in [
        Capability::Deploy(Environment::Any),
        Capability::NetworkRead(Audience::Any),
    ] {
        assert!(
            !TOOL_CANDIDATES.contains(&wildcard),
            "asking about `{wildcard}` is the question a narrow denial cannot answer"
        );
    }
}

#[test]
fn a_denied_private_read_takes_the_web_tools_away_from_a_profile_that_grants_the_broad_read() {
    // The driver cannot tell which audience a URL will reach, so it asks the private question. A
    // profile that grants the broad read gets the tools; the same profile with the denial the
    // protocol's floor asks for does not, because the tool it would be offered is one that could
    // reach the thing it just denied.
    fn offers_a_network_read(policy: &CapabilityPolicy) -> bool {
        tool_config(policy)
            .capabilities()
            .iter()
            .any(|capability| matches!(capability, Capability::NetworkRead(_)))
    }

    let broad = CapabilityPolicy::allowing([Capability::NetworkRead(Audience::Any)]);
    assert!(
        offers_a_network_read(&broad),
        "the fixture has to start from a policy that does get a network read, or the denial below \
         is taking nothing away"
    );

    let mut denied = broad.clone();
    denied.restrict(&CapabilityPolicy::denying([Capability::NetworkRead(
        Audience::Private,
    )]));
    assert!(
        !offers_a_network_read(&denied),
        "a profile that may never read a direct message is offered no tool that could reach one: \
         this table cannot classify a URL, so it has to ask the strictest audience question"
    );
    assert_eq!(
        denied.decide(&Capability::NetworkRead(Audience::Public)),
        CapabilityDecision::Allowed,
        "and the public read it was granted is still granted — the tool table is conservative, the \
         policy is not wrong"
    );
}

#[test]
fn an_empty_policy_offers_nothing_and_the_two_constant_answers_stand() {
    let tools = tool_config(&CapabilityPolicy::default());
    assert!(
        tools.is_empty(),
        "a policy that grants nothing offers nothing"
    );
    assert!(
        tools.skills_offered(),
        "a skill loader takes no action; everything it causes is a subsequent governed tool call"
    );
    assert!(
        !tools.subagents_offered(),
        "a subagent's tool set is derived by nothing here, so it would be a route around the \
         per-state allowlist"
    );
}

/// One capability in `allow`, `approval_required` **and** `deny` at once.
///
/// Not a contrived shape. `CapabilityPolicy::grant` extends all three sets, and nothing removes an
/// entry from `allow` when a narrowing document adds the same one to the other two — so a profile
/// assembled from a wide grant, an approval gate and a floor denial genuinely holds the *same*
/// capability three times. The capability is spelled at the exact scope `TOOL_CANDIDATES` asks
/// about, so `covers` is equality here and this fixture is about the three sets alone; the
/// widening question is `policy_with_a_wide_grant_and_a_narrow_gate`'s.
fn policy_holding_one_capability_in_all_three_sets() -> CapabilityPolicy {
    CapabilityPolicy {
        allow: [Capability::Deploy(Environment::Production)]
            .into_iter()
            .collect(),
        approval_required: [Capability::Deploy(Environment::Production)]
            .into_iter()
            .collect(),
        deny: [Capability::Deploy(Environment::Production)]
            .into_iter()
            .collect(),
    }
}

#[test]
fn a_capability_held_in_all_three_sets_at_once_is_handed_over_as_no_tool() {
    let gated = Capability::Deploy(Environment::Production);

    // Two facts that make the assertions below load-bearing rather than vacuous: the table does ask
    // about this capability, and a policy that only allows it does get the tool. Without both, the
    // test would pass on a tool nobody was ever going to be offered.
    assert!(
        TOOL_CANDIDATES.contains(&gated),
        "the fixture has to name a capability the table asks about"
    );
    assert!(
        tool_config(&CapabilityPolicy::allowing([gated.clone()])).admits(&gated),
        "`allow` on its own does hand `{gated}` over, so what follows takes a tool away rather \
         than asserting about one that was never on offer"
    );

    let policy = policy_holding_one_capability_in_all_three_sets();
    for (set, name) in [
        (&policy.allow, "allow"),
        (&policy.approval_required, "approval_required"),
        (&policy.deny, "deny"),
    ] {
        assert!(
            set.contains(&gated),
            "the fixture's whole point is that `{gated}` is in `{name}` at the same time as the \
             other two sets"
        );
    }

    let tools = tool_config(&policy);
    assert!(
        !tools.admits(&gated),
        "a capability a principle gated and a floor denied is offered as no tool, however wide the \
         grant sitting beside them in `allow`: reading the tool set out of `allow` is F3, and this \
         is the fixture where all three sets answer differently about one capability"
    );
    assert!(
        !tools.capabilities().contains(&gated),
        "and it is absent from the set itself, not merely unreported by `admits`: {:?}",
        tools.capabilities()
    );

    // And it is the approval gate that keeps the tool away, not only the denial. With `deny`
    // emptied the capability is still in `allow`, still gated, and still no tool — which is the
    // property in its own right, since an approval is granted by a human at a step boundary and
    // never by a model holding the tool.
    let mut gated_but_not_denied = policy.clone();
    gated_but_not_denied.deny.clear();
    assert_eq!(
        gated_but_not_denied.decide(&gated),
        CapabilityDecision::RequiresApproval,
        "with the denial lifted the gate is what remains, or the assertion below is re-testing the \
         denial under another name"
    );
    assert!(
        !tool_config(&gated_but_not_denied).admits(&gated),
        "`RequiresApproval` maps to no tool: an approval-gated capability never appears in a \
         step's tool set"
    );
}
