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
        "no development profile grants `command.execute`, so a development `llm` step holds no \
         shell: `cargo test` runs as a `command` step the driver executes, not as a tool the model \
         holds"
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
