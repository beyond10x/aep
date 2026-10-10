//! Which recorded ESS conformance records count toward a specification's move.
//!
//! A move is decided on counts of records by kind, and for most kinds a record is a record. An
//! ESS conformance record is not: it says a run happened *and how it went*, against *which
//! revision* of the model. Counting it by kind alone let a failed report pay for `conforming` as
//! well as a passed one did, and a run against yesterday's model pay for today's.
//!
//! So on an artifact that records a `model_digest` — only an `executable-system-specification`
//! does — a record of the three ESS kinds counts only when it passed and was run against that
//! digest. The judgement is the `ess-conformance` principle's (`passed`, no failed scenario, and a
//! `spec_digest` equal to the specification's `model_digest`, failing closed when there is none),
//! read out of the source text `aep plan artifact evidence --from` wrote for the record:
//!
//! | kind | passed means |
//! |---|---|
//! | `ess_conformance` (report/1) | `0 of N scenario(s) failed`, N > 0 |
//! | `ess_conformance_v2` (report/2, count-stage or ordinary suite) | `execution_status: passed`, no failed count, total > 0 |
//! | `ess_conformance_coverage_v1` (report/2, coverage suite) | `conformance_status: passed`, no failed count |
//!
//! A count-stage record's `conformance_status` is `inconclusive` by construction — a suite/1–4 or
//! an ordinary suite from /6 on has no coverage inventory, so the adapter refuses any other value
//! — which is why its execution status is what is read. That puts it level with a report/1, which carries no coverage either.
//!
//! A record that does not count is still shown, with its reason, so a refusal says why the record
//! somebody just made did not pay for the rung.

use aep_backend_markdown::journal::{Change, Entry};
use aep_backend_markdown::kernel::EvidenceOnHand;
use aep_domain::artifact::ArtifactKind;
use aep_domain::evidence::{EvidenceKind, SpecDigest};

/// A recorded ESS conformance record that did not count, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Discounted {
    /// Its kind.
    pub(crate) kind: EvidenceKind,
    /// When it says it was observed.
    pub(crate) at: String,
    /// Why it does not count, as a reader reads it.
    pub(crate) reason: String,
}

/// Counts `entries`' evidence by kind, judging the ESS conformance records of an artifact whose
/// kind records a model digest. `None` — no document to judge against — counts every record.
///
/// A kind whose every record was discounted is present with a count of zero: records of it were
/// presented and none was enough, which is a different fact from none being presented.
pub(crate) fn count(
    entries: &[Entry],
    artifact: Option<&ArtifactKind>,
    model_digest: Option<&SpecDigest>,
) -> (EvidenceOnHand, Vec<Discounted>) {
    let mut counted = EvidenceOnHand::new();
    let mut discounted = Vec::new();
    for entry in entries {
        let Change::Evidence { kind, source, .. } = &entry.change else {
            continue;
        };
        let held = counted.entry(*kind).or_default();
        if !artifact.is_some_and(ArtifactKind::carries_model_digest) || !is_conformance(*kind) {
            *held += 1;
            continue;
        }
        match judge(*kind, source, model_digest) {
            Ok(()) => *held += 1,
            Err(reason) => discounted.push(Discounted {
                kind: *kind,
                at: entry.at.clone(),
                reason,
            }),
        }
    }
    (counted, discounted)
}

/// `true` for the three kinds an ESS conformance report is recorded as.
fn is_conformance(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::EssConformance
            | EvidenceKind::EssConformanceV2
            | EvidenceKind::EssConformanceCoverageV1
    )
}

/// What a record says about its run: its digest, and why it did not pass when it did not.
struct Reading {
    digest: String,
    not_passed: Option<String>,
}

/// `Ok` when the record passed against `model_digest`; otherwise every reason it does not count.
fn judge(
    kind: EvidenceKind,
    source: &str,
    model_digest: Option<&SpecDigest>,
) -> Result<(), String> {
    let reading = match kind {
        EvidenceKind::EssConformance => read_report_1(source),
        _ => read_report_2(kind, source),
    }?;
    let mut reasons: Vec<String> = reading.not_passed.into_iter().collect();
    match model_digest {
        None => reasons.push(
            "this specification records no `model_digest`, so no run can be shown to be of this \
             revision — `aep plan artifact set <id> --model-digest <hex>` records it"
                .to_owned(),
        ),
        Some(expected) => match SpecDigest::new(reading.digest.clone()) {
            Ok(digest) if &digest == expected => {}
            Ok(_) => reasons.push(format!(
                "it was run against {}, and this specification is at {}",
                reading.digest,
                expected.as_str()
            )),
            Err(_) => reasons.push(format!(
                "its `spec_digest` `{}` is not a digest, so it cannot be shown to be of this \
                 revision",
                reading.digest
            )),
        },
    }
    if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons.join("; "))
    }
}

/// The line `evidence --from` writes for a report/1:
/// `ess conform run: <implementation> against <specification> at <digest>, <failed> of <total>
/// scenario(s) failed`. Read from the right, because the two names are free text.
fn read_report_1(source: &str) -> Result<Reading, String> {
    let unreadable = || {
        "its source is not the line `aep plan artifact evidence --from` writes for an \
         ess-conformance-report/1, so neither its result nor its digest can be read"
            .to_owned()
    };
    let rest = source
        .strip_prefix("ess conform run: ")
        .and_then(|rest| rest.strip_suffix(" scenario(s) failed"))
        .ok_or_else(unreadable)?;
    let (head, tally) = rest.rsplit_once(", ").ok_or_else(unreadable)?;
    let (_, digest) = head.rsplit_once(" at ").ok_or_else(unreadable)?;
    let (failed, total) = tally.split_once(" of ").ok_or_else(unreadable)?;
    let failed: u64 = failed.parse().map_err(|_| unreadable())?;
    let total: u64 = total.parse().map_err(|_| unreadable())?;
    let not_passed = if failed > 0 {
        Some(format!("it reports {failed} of {total} scenario(s) failed"))
    } else if total == 0 {
        Some("it ran no scenario".to_owned())
    } else {
        None
    };
    Ok(Reading {
        digest: digest.to_owned(),
        not_passed,
    })
}

/// The JSON `evidence --from --suite` writes for a report/2.
fn read_report_2(kind: EvidenceKind, source: &str) -> Result<Reading, String> {
    let unreadable = || {
        format!(
            "its source is not the document `aep plan artifact evidence --from --suite` writes \
             for {}, so neither its result nor its digest can be read",
            kind.as_str()
        )
    };
    let value: serde_json::Value = serde_json::from_str(source).map_err(|_| unreadable())?;
    let text = |key: &str| value.get(key).and_then(serde_json::Value::as_str);
    let digest = text("spec_digest").ok_or_else(unreadable)?.to_owned();
    let count = |key: &str| {
        value
            .get("counts")
            .and_then(|counts| counts.get(key))
            .and_then(serde_json::Value::as_u64)
    };
    // A count-stage record's conformance is inconclusive by construction; its execution is what
    // says whether the selected scenarios passed. A coverage record's conformance says it.
    let field = if kind == EvidenceKind::EssConformanceV2 {
        "execution_status"
    } else {
        "conformance_status"
    };
    let status = text(field).ok_or_else(unreadable)?;
    let failed = count("failed").ok_or_else(unreadable)?;
    let total = count("total").ok_or_else(unreadable)?;
    let not_passed = if status != "passed" {
        Some(format!("its {field} is {status}"))
    } else if failed > 0 {
        Some(format!("it reports {failed} of {total} scenario(s) failed"))
    } else if total == 0 {
        Some("it ran no scenario".to_owned())
    } else {
        None
    };
    Ok(Reading { digest, not_passed })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";
    const OTHER: &str = "afb43b637e6c8e3eb7743c0a7549e622c7bb317ce856891f30aa5e88c69881d7";

    fn digest(value: &str) -> SpecDigest {
        SpecDigest::new(value.to_owned()).unwrap()
    }

    fn v1(digest: &str, failed: u64) -> String {
        format!(
            "ess conform run: impl 1.0 at home against spec/v1 at {digest}, {failed} of 29 \
             scenario(s) failed"
        )
    }

    fn v2(field: &str, status: &str, digest: &str) -> String {
        serde_json::json!({
            "spec_digest": digest, field: status,
            "counts": {"total": 3, "passed": 3, "failed": 0}
        })
        .to_string()
    }

    #[test]
    fn a_report_1_counts_only_when_nothing_failed_against_this_digest() {
        let kind = EvidenceKind::EssConformance;
        assert_eq!(judge(kind, &v1(DIGEST, 0), Some(&digest(DIGEST))), Ok(()));
        let failed = judge(kind, &v1(DIGEST, 2), Some(&digest(DIGEST))).unwrap_err();
        assert!(failed.contains("2 of 29"), "{failed}");
        let other = judge(kind, &v1(OTHER, 0), Some(&digest(DIGEST))).unwrap_err();
        assert!(other.contains(OTHER) && other.contains(DIGEST), "{other}");
        let typed = judge(kind, "task check", Some(&digest(DIGEST))).unwrap_err();
        assert!(typed.contains("nor its digest can be read"), "{typed}");
    }

    #[test]
    fn a_report_2_counts_on_the_status_its_kind_can_pass_on() {
        let v2_kind = EvidenceKind::EssConformanceV2;
        let coverage = EvidenceKind::EssConformanceCoverageV1;
        let at = Some(digest(DIGEST));
        assert_eq!(
            judge(v2_kind, &v2("execution_status", "passed", DIGEST), at.as_ref()),
            Ok(())
        );
        assert_eq!(
            judge(coverage, &v2("conformance_status", "passed", DIGEST), at.as_ref()),
            Ok(())
        );
        for status in ["failed", "inconclusive"] {
            let refused =
                judge(coverage, &v2("conformance_status", status, DIGEST), at.as_ref())
                    .unwrap_err();
            assert!(refused.contains(status), "{refused}");
        }
        let other =
            judge(coverage, &v2("conformance_status", "passed", OTHER), at.as_ref()).unwrap_err();
        assert!(other.contains(OTHER), "{other}");
    }

    #[test]
    fn no_record_counts_on_a_specification_that_records_no_digest() {
        let refused = judge(EvidenceKind::EssConformance, &v1(DIGEST, 0), None).unwrap_err();
        assert!(refused.contains("model_digest"), "{refused}");
    }
}
