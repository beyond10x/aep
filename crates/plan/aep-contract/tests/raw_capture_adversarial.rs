//! Adversarial regressions for the public raw-capture validation boundary.

use aep_contract::migration::*;

fn unix(value: &str) -> HostPathV1 {
    HostPathV1::Unix(HexBytesV1::new(value.as_bytes().to_vec()))
}

fn root_refusal(code: CaptureRefusalCodeV1) -> CaptureRefusalV1 {
    CaptureRefusalV1 {
        code,
        at: PhysicalCoordinateV1::Root(RootCoordinateV1::Observation),
    }
}

fn markdown_source() -> PresenceV1<SourceCoordinateV1> {
    PresenceV1::Present(SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
        root: unix("planning"),
    }))
}

fn empty_markdown_evidence() -> PhaseEvidenceV1 {
    PhaseEvidenceV1::Markdown(MarkdownEvidenceV1 {
        root: PresenceV1::Present(unix("planning")),
        nodes: ObservedListV1 {
            items: Vec::new(),
            terminal: EnumerationTerminalV1::Complete,
        },
    })
}

fn complete_phase(phase: CapturePhaseV1, evidence: PhaseEvidenceV1) -> PhaseObservationV1 {
    let evidence_digest = evidence.evidence_digest();
    PhaseObservationV1 {
        phase,
        result: PhaseResultV1::Complete(CompletePhaseResultV1 {
            evidence,
            evidence_digest,
        }),
    }
}

#[test]
fn known_method_preflight_still_checks_present_source_compatibility() {
    let observation = RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source: markdown_source(),
        selector: PresenceV1::Missing,
        config_digest: PresenceV1::Missing,
        observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
            method: PresenceV1::Present(ObservationMethodV1::SqliteReadTransaction),
            phases: vec![PhaseObservationV1 {
                phase: CapturePhaseV1::SqlSnapshot,
                result: PhaseResultV1::NotAttempted,
            }],
            preflight_refusals: vec![root_refusal(CaptureRefusalCodeV1::SourceUnreachable)],
        }),
    };

    let errors = observation
        .validate()
        .expect_err("a known SQLite method cannot describe a Markdown source");
    assert!(
        errors.contains(CaptureValidationCodeV1::IncompatibleVariant),
        "the known preflight method must remain bound to its present source coordinate"
    );
}

#[test]
fn unresolved_source_refusals_require_root_coordinates() {
    let observation = RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source: PresenceV1::Missing,
        selector: PresenceV1::Missing,
        config_digest: PresenceV1::Missing,
        observation: ObservationOutcomeV1::Refused(RefusedObservationV1 {
            method: PresenceV1::Missing,
            phases: Vec::new(),
            preflight_refusals: vec![CaptureRefusalV1 {
                code: CaptureRefusalCodeV1::UnresolvedSourceCoordinate,
                at: PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                    relative: unix("story/item.md"),
                }),
            }],
        }),
    };

    let errors = observation
        .validate()
        .expect_err("source resolution cannot invent a captured Markdown path");
    assert!(
        errors.contains(CaptureValidationCodeV1::IncompatibleVariant),
        "resolution and source-unreachable failures are root-coordinate facts"
    );
}

#[test]
fn retained_foreign_markdown_nodes_require_their_exact_refusal() {
    let relative = unix("foreign.bin");
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
    let observation = RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source: markdown_source(),
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
                            code: CaptureRefusalCodeV1::ReadFailure,
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

    let errors = observation
        .validate()
        .expect_err("a foreign node must be named as foreign_markdown_node");
    assert!(
        errors.contains(CaptureValidationCodeV1::IncompatibleVariant),
        "retaining the node does not let an unrelated refusal code admit it"
    );
}

#[test]
fn complete_validation_accumulates_capture_defects_when_reconstruction_fails() {
    let invalid_capture = LegacyRawCaptureV1::Markdown(MarkdownRawV1 {
        nodes: vec![MarkdownNodeV1 {
            relative: unix("foreign.bin"),
            node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(Vec::new()),
            }),
        }],
    });
    let observation = RawCaptureObservationV1 {
        format: RawCaptureFormatV1,
        source: markdown_source(),
        selector: PresenceV1::Present(SelectorCoordinateV1 {
            project_root: unix("repo"),
            project_file: unix(".engineering/project.yaml"),
            presence: SelectorPresenceV1::MissingV1Default,
            store_field: StoreFieldV1::MissingDefault,
        }),
        config_digest: PresenceV1::Present(config_digest_v1(&[])),
        observation: ObservationOutcomeV1::Complete(CompleteObservationV1 {
            method: ObservationMethodV1::MarkdownDoubleScan,
            phases: vec![complete_phase(
                CapturePhaseV1::MarkdownFirst,
                empty_markdown_evidence(),
            )],
            capture: invalid_capture,
            transcript_digest: DigestV1::from_bytes([0; 32]),
            raw_snapshot_id: DigestV1::from_bytes([0; 32]),
        }),
    };

    let errors = observation
        .validate()
        .expect_err("phase and independent capture defects must accumulate");
    assert!(errors.contains(CaptureValidationCodeV1::PhaseRoster));
    assert!(
        errors.contains(CaptureValidationCodeV1::UnsupportedSchema),
        "failed reconstruction must not suppress validation of the supplied capture"
    );
}
