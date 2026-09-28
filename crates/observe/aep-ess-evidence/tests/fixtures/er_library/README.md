# Actual Entity Runtime execution evidence

These are the exact suite and report from ER's standalone Rust checker, executed on 2026-09-28
for version 0.25.1. They were copied unchanged after all 414 selected scenarios passed and AEP's
CLI successfully imported the pair. They are actual implementation evidence, separate from the
independently authored reader reports in `direct_returns.rs`.

The checker pins ESS `ac6fc6fe2f39b43f016e4d3a9edecb3573f7d6a1`; its source closure and executable
SHA256 identities are recorded in the report. The source closure is
`567200cbb0faefbaa6efaa7ad849393b7088e60e151c21405e114dbbac07f92c`. The model digest is
`a13e6c07be82c913363ea52b8042c6e379343acfda7fcc03a212c0942569da5d` and the exact suite digest is
`sha256:a6f8ceb64b95f76d33f75c741fb3b67770ad6130243504b77761921be6832548`.

ER retains the model, coverage fingerprints, observations, implementation identity, mutation
proof and complete gate evidence in `docs/ess/evidence/final/` and its sibling evidence directories.
The integration is tracked by beyond10x/entity-runtime#47 and ESS's producer by beyond10x/ess#185.
No timestamp, result, identity or version in this pair was relabeled for admission.

This run precedes ER's final alignment of internal dependency version constraints to 0.25.1.
ER retains a separate final release run under `docs/ess/evidence/final/release/`; this fixture
remains the unchanged reader-integration run identified above, not a release-completion claim.
