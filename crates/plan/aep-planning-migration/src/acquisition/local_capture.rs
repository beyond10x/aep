//! Physical filesystem observations retain completed nodes and exact refused path units.

use super::{host_path, other_kind};
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;
use std::{fs, path::Path};

fn path_coordinate(root: &Path, path: &Path, source: &HostPathV1) -> PhysicalCoordinateV1 {
    if root == path {
        PhysicalCoordinateV1::Root(RootCoordinateV1::MarkdownRoot(MarkdownRootCoordinateV1 {
            root: source.clone(),
        }))
    } else {
        PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
            relative: host_path(
                path.strip_prefix(root)
                    .expect("visited paths are descendants of the supplied root"),
            ),
        })
    }
}

fn relative(node: &MarkdownNodeEvidenceV1) -> &HostPathV1 {
    match node {
        MarkdownNodeEvidenceV1::Captured(node) => &node.relative,
        MarkdownNodeEvidenceV1::Unreadable(node) => &node.relative,
    }
}

fn refused(evidence: PhaseEvidenceV1, refusal: CaptureRefusalV1) -> Box<RefusedPhaseResultV1> {
    let evidence_digest = evidence.evidence_digest();
    Box::new(RefusedPhaseResultV1 {
        evidence,
        evidence_digest,
        refusals: vec![refusal],
    })
}

pub(super) fn scan(
    root: &Path,
    source: &HostPathV1,
) -> Result<MarkdownRawV1, Box<RefusedPhaseResultV1>> {
    let mut nodes = Vec::new();
    let result = visit(root, root, source, &mut nodes);
    nodes.sort_by(|left, right| relative(left).cmp(relative(right)));
    match result {
        Ok(()) => Ok(MarkdownRawV1 {
            nodes: nodes
                .into_iter()
                .map(|node| match node {
                    MarkdownNodeEvidenceV1::Captured(node) => node,
                    MarkdownNodeEvidenceV1::Unreadable(_) => {
                        unreachable!("unreadable nodes end acquisition immediately")
                    }
                })
                .collect(),
        }),
        Err(failure) => Err(refused(
            PhaseEvidenceV1::Markdown(MarkdownEvidenceV1 {
                root: PresenceV1::Present(source.clone()),
                nodes: ObservedListV1 {
                    items: nodes,
                    terminal: EnumerationTerminalV1::Refused(EnumerationRefusalV1 {
                        at: failure.at.clone(),
                        code: failure.code.clone(),
                    }),
                },
            }),
            failure,
        )),
    }
}

fn visit(
    root: &Path,
    directory: &Path,
    source: &HostPathV1,
    nodes: &mut Vec<MarkdownNodeEvidenceV1>,
) -> Result<(), CaptureRefusalV1> {
    let directory_failure = || CaptureRefusalV1 {
        code: CaptureRefusalCodeV1::ReadFailure,
        at: path_coordinate(root, directory, source),
    };
    // Process entries as they arrive. Collecting the whole iterator first would discard the
    // obtained prefix if read_dir failed partway through. Canonical ordering happens afterward.
    for entry in fs::read_dir(directory).map_err(|_| directory_failure())? {
        let path = entry.map_err(|_| directory_failure())?.path();
        let relative = host_path(
            path.strip_prefix(root)
                .expect("entry belongs to visited directory"),
        );
        let at = path_coordinate(root, &path, source);
        if relative.validate_relative().is_err() {
            nodes.push(MarkdownNodeEvidenceV1::Unreadable(
                UnreadableMarkdownNodeV1 {
                    relative,
                    metadata: PresenceV1::Missing,
                },
            ));
            return Err(CaptureRefusalV1 {
                code: CaptureRefusalCodeV1::InvalidRelativePath,
                at,
            });
        }
        let metadata = fs::symlink_metadata(&path).map_err(|_| {
            nodes.push(MarkdownNodeEvidenceV1::Unreadable(
                UnreadableMarkdownNodeV1 {
                    relative: relative.clone(),
                    metadata: PresenceV1::Missing,
                },
            ));
            CaptureRefusalV1 {
                code: CaptureRefusalCodeV1::ReadFailure,
                at: at.clone(),
            }
        })?;
        let kind = if metadata.file_type().is_symlink() {
            MarkdownNodeKindTagV1::Symlink
        } else if metadata.is_dir() {
            MarkdownNodeKindTagV1::Directory
        } else if metadata.is_file() {
            MarkdownNodeKindTagV1::Regular
        } else {
            MarkdownNodeKindTagV1::Other
        };
        let mut unreadable = || {
            nodes.push(MarkdownNodeEvidenceV1::Unreadable(
                UnreadableMarkdownNodeV1 {
                    relative: relative.clone(),
                    metadata: PresenceV1::Present(NodeMetadataV1 {
                        kind: kind.clone(),
                        length: PresenceV1::Present(metadata.len()),
                    }),
                },
            ));
            CaptureRefusalV1 {
                code: CaptureRefusalCodeV1::ReadFailure,
                at: at.clone(),
            }
        };
        let node = if metadata.file_type().is_symlink() {
            MarkdownNodeKindV1::Symlink(SymlinkMarkdownNodeV1 {
                target: host_path(&fs::read_link(&path).map_err(|_| unreadable())?),
            })
        } else if metadata.is_dir() {
            MarkdownNodeKindV1::Directory
        } else if metadata.is_file() {
            MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(fs::read(&path).map_err(|_| unreadable())?),
            })
        } else {
            MarkdownNodeKindV1::Other(OtherMarkdownNodeV1 {
                kind: other_kind(&metadata),
            })
        };
        let node = MarkdownNodeV1 { relative, node };
        let refusal = node.capture_refusal();
        nodes.push(MarkdownNodeEvidenceV1::Captured(node));
        if let Some(code) = refusal {
            return Err(CaptureRefusalV1 { code, at });
        }
        if metadata.is_dir() {
            visit(root, &path, source, nodes)?;
        }
    }
    Ok(())
}

pub(super) fn divergence(path: &Path) -> Result<FileImageV1, Box<RefusedPhaseResultV1>> {
    match fs::read(path) {
        Ok(bytes) => Ok(FileImageV1::Present(PresentFileImageV1 {
            bytes: HexBytesV1::new(bytes),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileImageV1::Absent),
        Err(_) => {
            let at = PhysicalCoordinateV1::Root(RootCoordinateV1::HybridSide(
                HybridSideRootCoordinateV1 {
                    side: HybridSideV1::Divergences,
                },
            ));
            let code = CaptureRefusalCodeV1::ReadFailure;
            Err(refused(
                PhaseEvidenceV1::Divergences(ObservedValueV1::Refused(ObservedValueRefusalV1 {
                    at: at.clone(),
                    code: code.clone(),
                })),
                CaptureRefusalV1 { code, at },
            ))
        }
    }
}
