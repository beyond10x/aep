//! Adversary cases for `aep_backend_markdown::roadmap` (`story:roadmap-kind`).

use aep_backend_markdown::roadmap::recorded;

/// `RoadmapError::line` is documented as "the body line". For an entry-level defect it is always
/// the block's first line, whichever entry is wrong.
#[test]
fn a_bad_horizon_in_the_second_entry_is_located_at_the_second_entry() {
    let body = "# Roadmap\n\n```roadmap\n- epic: epic:a\n  horizon: now\n\
                - epic: epic:b\n  horizon: soon\n```\n";
    // Line 6 is `- epic: epic:b`, line 7 its `horizon: soon`.
    let errors = recorded(body).expect_err("refused");
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert!(
        (6..=7).contains(&error.line),
        "the defect is in entry 2 (body lines 6-7), and the error says line {}: {error}",
        error.line
    );
}

/// The same for an entry whose `epic` is not an id.
#[test]
fn a_bad_epic_id_in_the_third_entry_is_located_at_the_third_entry() {
    let body = "# Roadmap\n\n```roadmap\n- {epic: 'epic:a', horizon: now}\n\
                - {epic: 'epic:b', horizon: now}\n- {epic: 'not an id', horizon: now}\n```\n";
    let errors = recorded(body).expect_err("refused");
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(
        error.line, 6,
        "the defect is entry 3 on body line 6: {error}"
    );
}
