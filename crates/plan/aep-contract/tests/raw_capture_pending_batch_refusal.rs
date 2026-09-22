//! Regression coverage for the dedicated pending-batch capture refusal.

use aep_contract::migration::*;

fn unix(value: &str) -> HostPathV1 {
    HostPathV1::Unix(HexBytesV1::new(value.as_bytes().to_vec()))
}

#[test]
fn retained_pending_batch_accepts_its_dedicated_refusal() {
    let relative = unix(".aep-batch.pending.json");
    let evidence = PhaseEvidenceV1::Markdown(MarkdownEvidenceV1 {
        root: PresenceV1::Present(unix("planning")),
        nodes: ObservedListV1 {
            items: vec![MarkdownNodeEvidenceV1::Captured(MarkdownNodeV1 {
                relative: relative.clone(),
                node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                    bytes: HexBytesV1::new(b"retained pending batch".to_vec()),
                }),
            })],
            terminal: EnumerationTerminalV1::Complete,
        },
    });
    let evidence_digest = evidence.evidence_digest();
    let observation = RawCaptureObservationV1 {
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
                            code: CaptureRefusalCodeV1::PendingBatchPresent,
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
    };

    observation
        .validate()
        .expect("a retained pending-batch marker has its own exact refusal code");
}
