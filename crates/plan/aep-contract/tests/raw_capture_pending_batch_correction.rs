//! Boundary coverage for pending-batch and foreign Markdown refusal classification.

use aep_contract::migration::*;

fn unix(value: &str) -> HostPathV1 {
    HostPathV1::Unix(HexBytesV1::new(value.as_bytes().to_vec()))
}

fn windows(value: &str) -> HostPathV1 {
    HostPathV1::Windows(value.encode_utf16().collect())
}

fn refused_with(relative: HostPathV1, code: CaptureRefusalCodeV1) -> RawCaptureObservationV1 {
    let evidence = PhaseEvidenceV1::Markdown(MarkdownEvidenceV1 {
        root: PresenceV1::Present(unix("planning")),
        nodes: ObservedListV1 {
            items: vec![MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 {
                relative: relative.clone(),
                node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                    bytes: HexBytesV1::new(b"retained".to_vec()),
                }),
            })],
            terminal: EnumerationTerminalV1::Complete,
        },
    });
    let evidence_digest = evidence.evidence_digest();
    RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source: PresenceV1::Present(SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
            root: unix("planning"),
        })),
        selector: PresenceV1::Missing,
        config_digest: PresenceV1::Missing,
        observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
            method: PresenceV1::Present(ObservationMethodV1::MarkdownDoubleScan),
            phases: vec![
                PhaseObservationV1 {
                    phase: CapturePhaseV1::MarkdownFirst,
                    result: PhaseResultV1::Refused(RefusedPhaseResultV1 {
                        evidence,
                        evidence_digest,
                        refusals: vec![CaptureRefusalV1 {
                            code,
                            at: PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                                relative,
                            }),
                        }],
                    }),
                },
                PhaseObservationV1 {
                    phase: CapturePhaseV1::MarkdownSecond,
                    result: PhaseResultV1::NotAttempted,
                },
            ],
            preflight_refusals: Vec::new(),
        }),
    }
}

#[test]
fn exact_pending_batch_marker_uses_its_code_for_both_path_variants() {
    for relative in [
        unix(".aep-batch.pending.json"),
        windows(".aep-batch.pending.json"),
    ] {
        refused_with(relative, CaptureRefusalCodeV1::PendingBatchPresent)
            .validate()
            .expect("the exact root pending-batch marker has a dedicated refusal");
    }
}

#[test]
fn exact_pending_batch_marker_rejects_the_generic_foreign_code() {
    for relative in [
        unix(".aep-batch.pending.json"),
        windows(".aep-batch.pending.json"),
    ] {
        let errors = refused_with(relative, CaptureRefusalCodeV1::ForeignMarkdownNode)
            .validate()
            .expect_err("the exact pending-batch marker is not a generic foreign node");
        assert!(errors.contains(CaptureValidationCodeV1::IncompatibleVariant));
    }
}

#[test]
fn nested_pending_batch_lookalikes_remain_generic_foreign_nodes() {
    for relative in [
        unix("story/.aep-batch.pending.json"),
        windows("story\\.aep-batch.pending.json"),
    ] {
        refused_with(relative.clone(), CaptureRefusalCodeV1::ForeignMarkdownNode)
            .validate()
            .expect("a nested lookalike is a generic foreign node");

        let errors = refused_with(relative, CaptureRefusalCodeV1::PendingBatchPresent)
            .validate()
            .expect_err("the pending-batch code is reserved for the exact root marker");
        assert!(errors.contains(CaptureValidationCodeV1::IncompatibleVariant));
    }
}
