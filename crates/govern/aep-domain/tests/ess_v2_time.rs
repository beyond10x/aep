//! Exact integer observations with measured legacy scalar/date controls.
use aep_domain::time::ObservedAt;

#[test]
fn legacy_observed_at_scalar_and_calendar_controls() {
    for document in ["0", "1", "1.0", "1e0", "-0.0", "-0e0"] {
        let actual: ObservedAt = serde_json::from_str(document).unwrap();
        let expected = u64::from(document.starts_with('1'));
        assert_eq!(actual.timestamp().epoch_millis(), expected, "{document}");
    }
    for document in ["-1", "-1.0", "1.5", "18446744073709551616", "true", "null"] {
        assert!(
            serde_json::from_str::<ObservedAt>(document).is_err(),
            "{document}"
        );
    }
    let date: ObservedAt = serde_json::from_str("\"2026-09-06\"").unwrap();
    assert_eq!(date.written_as(), aep_domain::time::Granularity::Day);
    assert_eq!(serde_json::to_string(&date).unwrap(), "1788652800000");
}

#[test]
fn actual_integer_observation_tokens_retain_full_u64() {
    for expected in [
        0,
        9_007_199_254_740_993,
        i64::MAX as u64,
        i64::MAX as u64 + 1,
        u64::MAX,
    ] {
        let actual: ObservedAt = serde_json::from_str(&expected.to_string()).unwrap();
        assert_eq!(actual.timestamp().epoch_millis(), expected);
        assert_eq!(
            serde_json::to_string(&actual).unwrap(),
            expected.to_string()
        );
    }
}
