# AEP coverage prerequisite — complete replan outputs

These are unmodified command outputs observed on 2026-09-06, before implementation.
The global draft computation includes unrelated AEP backlog work outside the ESS task.
The complete proposed set is the bounded reader prerequisite selected by the companion wave page.
All commands exited zero; the initial unsupported global --root argument was refused before execution and its stderr is retained in preparation scratch.

## aep plan artifact waves --kind story --status draft --format json

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:a-stale-binary-refuses-itself",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "AGENTS.md"
            },
            {
              "confidence": "cited",
              "path": "Taskfile.yml"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/planning.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/plan/aep-backend-markdown/src/journal.rs"
            }
          ]
        },
        {
          "id": "story:acquisition-phase-set",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "docs/guide/adopting.md"
            },
            {
              "confidence": "cited",
              "path": "examples"
            },
            {
              "confidence": "cited",
              "path": "principles"
            },
            {
              "confidence": "cited",
              "path": "protocols"
            }
          ]
        },
        {
          "id": "story:admit-ess-conformance-coverage",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-schema"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-engine"
            },
            {
              "confidence": "cited",
              "path": "crates/observe/aep-ess-evidence"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/ess-conformance-coverage-evidence.md"
            },
            {
              "confidence": "inferred",
              "path": "principles/verification/ess-conformance-coverage.yaml"
            },
            {
              "confidence": "inferred",
              "path": "profiles/development-ess-conformance-coverage.yaml"
            },
            {
              "confidence": "inferred",
              "path": "protocols/adp-ess-conformance-coverage/1.yaml"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/artifact-lifecycle.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/artifact-manifest.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/driver-steps.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/event.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/evidence.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/principle.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/profile.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/protocol.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/task.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/workflow.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "website/docs/reference/cli.md"
            }
          ]
        },
        {
          "id": "story:advisory-enforcement-tier",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": ".engineering/checks/run.sh"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain/src/requirement.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-engine/src/evaluate.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/plan/gap-register.md"
            }
          ]
        },
        {
          "id": "story:agent-eval-cases",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "conformance/eval/decomposer-charter"
            },
            {
              "confidence": "cited",
              "path": "conformance/eval/plan-reviewer-charter"
            },
            {
              "confidence": "cited",
              "path": "conformance/trace/expectations.trace.yaml"
            },
            {
              "confidence": "inferred",
              "path": "crates/observe/trace-spec"
            }
          ]
        },
        {
          "id": "story:claim-retirement",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-domain/src/evidence.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/plan/aep-backend-markdown/src/claim.rs"
            }
          ]
        },
        {
          "id": "story:codex-adapter",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/drive/aep-driver/tests/shell_echo.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/drive.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/observe/trace-spec/src/adapter.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/observe/trace-spec/src/codex.rs"
            }
          ]
        },
        {
          "id": "story:fanout-promote",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-domain/src/workflow.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-engine/src/execution.rs"
            },
            {
              "confidence": "cited",
              "path": "workflows/releases/progressive.yaml"
            }
          ]
        },
        {
          "id": "story:machine-readable-service-contract",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/serve"
            },
            {
              "confidence": "cited",
              "path": "crates/plan/aep-client"
            },
            {
              "confidence": "inferred",
              "path": "crates/plan/aep-conformance"
            }
          ]
        },
        {
          "id": "story:partial-edits-cost-more-replay-than-they-save",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/drive/aep-driver"
            },
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver-spec"
            },
            {
              "confidence": "inferred",
              "path": "docs/reviews/2026-08-24-scope-cache-and-the-native-arm.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 2,
      "artifacts": [
        {
          "id": "story:a-story-records-where-it-lands",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "artifacts/kinds"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/planning.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-domain/src/artifact.rs"
            }
          ]
        },
        {
          "id": "story:adopt-documents-into-the-event-log",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/reverse.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/plan/aep-backend-markdown/src/drift.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/plan/aep-backend-markdown/src/journal.rs"
            }
          ]
        },
        {
          "id": "story:cold-start-outlives-the-deadline",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/drive/aep-driver/src/run.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/drive.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/observe/trace-spec/src/report.rs"
            }
          ]
        },
        {
          "id": "story:communication-publish-capability",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain/src/capability.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain/src/protocol.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/guide/open-vocabulary.md"
            },
            {
              "confidence": "cited",
              "path": "protocols/aep/1.yaml"
            },
            {
              "confidence": "cited",
              "path": "website/docs/reference/vocabulary.md"
            }
          ]
        },
        {
          "id": "story:completion-needs-evidence",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-engine"
            },
            {
              "confidence": "cited",
              "path": "docs/design/story-completion-evidence-design-v0.1.md"
            },
            {
              "confidence": "cited",
              "path": "docs/plan/harness-wave-4-governed-dogfood.md"
            },
            {
              "confidence": "inferred",
              "path": "principles/development"
            },
            {
              "confidence": "cited",
              "path": "profiles/development-standard.yaml"
            }
          ]
        },
        {
          "id": "story:corpus-observables",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "docs/guide/adopting.md"
            },
            {
              "confidence": "cited",
              "path": "examples"
            },
            {
              "confidence": "cited",
              "path": "protocols"
            }
          ]
        },
        {
          "id": "story:evidence-subject-binding",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain/src/requirement.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-engine/src/execution.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/plan/gap-register.md"
            },
            {
              "confidence": "cited",
              "path": "website/docs/concepts/design-principles.md"
            },
            {
              "confidence": "cited",
              "path": "website/docs/concepts/evidence.md"
            },
            {
              "confidence": "cited",
              "path": "website/docs/status/limitations.md"
            }
          ]
        },
        {
          "id": "story:gate-lanes-count-what-ran",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": ".engineering/checks"
            },
            {
              "confidence": "cited",
              "path": "Taskfile.yml"
            },
            {
              "confidence": "inferred",
              "path": "conformance"
            },
            {
              "confidence": "cited",
              "path": "docs/reviews/2026-08-20-guard-efficacy-review.md"
            },
            {
              "confidence": "cited",
              "path": "docs/status.md"
            }
          ]
        },
        {
          "id": "story:harness-denies-direct-store-writes",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": ".claude"
            },
            {
              "confidence": "cited",
              "path": "AGENTS.md"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/tests"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/tests/store_selection.rs"
            }
          ]
        },
        {
          "id": "story:journal-carries-digests-not-bodies",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "cited",
              "path": "crates/plan/aep-backend-markdown"
            }
          ]
        },
        {
          "id": "story:skill-text-in-context",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/observe/trace-domain"
            },
            {
              "confidence": "cited",
              "path": "crates/observe/trace-spec"
            }
          ]
        }
      ]
    },
    {
      "wave": 3,
      "artifacts": [
        {
          "id": "story:a-tag-can-be-filtered-on",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/src/planning.rs"
            }
          ]
        },
        {
          "id": "story:confined-driven-workspace",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/drive/aep-driver/src/run.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/drive.rs"
            },
            {
              "confidence": "cited",
              "path": "scripts/drive-score"
            }
          ]
        },
        {
          "id": "story:native-arm-needs-a-window-that-fits",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "inferred",
              "path": "docs/plan/eval-program-three-arms.md"
            },
            {
              "confidence": "cited",
              "path": "drivers/development/default.yaml"
            }
          ]
        },
        {
          "id": "story:published-pattern-residue",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver-spec"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "cited",
              "path": "schemas/generated/protocol.schema.json"
            }
          ]
        },
        {
          "id": "story:streaming-checker",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/observe/trace-domain"
            },
            {
              "confidence": "inferred",
              "path": "crates/observe/trace-spec"
            }
          ]
        }
      ]
    },
    {
      "wave": 4,
      "artifacts": [
        {
          "id": "story:attested-approver",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver/src/attest.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver/src/run.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver/tests/attested.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/drive.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/src/planning.rs"
            }
          ]
        },
        {
          "id": "story:native-plugin-eval",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli"
            }
          ]
        },
        {
          "id": "story:source-record-evidence",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "cited",
              "path": "docs/guide/open-vocabulary.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
      "artifacts": [
        {
          "id": "story:bulk-create-from-a-manifest",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/src/planning.rs"
            }
          ]
        },
        {
          "id": "story:drive-watch-is-a-verb",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "AGENTS.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/drive.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/plan/harness-wave-4-governed-dogfood.md"
            },
            {
              "confidence": "cited",
              "path": "scripts/drive-score"
            },
            {
              "confidence": "cited",
              "path": "scripts/drive-watch"
            }
          ]
        },
        {
          "id": "story:outbound-claims-and-status-vocabulary",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "artifacts/lifecycles"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-engine"
            },
            {
              "confidence": "inferred",
              "path": "crates/plan/aep-backend-markdown"
            }
          ]
        }
      ]
    },
    {
      "wave": 6,
      "artifacts": [
        {
          "id": "story:dry-run-on-the-write-verbs",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/src/planning.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/src/reverse.rs"
            }
          ]
        },
        {
          "id": "story:frame-subjects-from-the-step-map",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/drive/aep-driver-spec/src/map.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/src/drive.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/observe/trace-domain/src/ir.rs"
            }
          ]
        },
        {
          "id": "story:per-record-horizons",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "AGENTS.md"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-engine"
            },
            {
              "confidence": "cited",
              "path": "examples/evidence-horizons-corpus/distribution.json"
            }
          ]
        }
      ]
    },
    {
      "wave": 7,
      "artifacts": [
        {
          "id": "story:external-clock-obligations",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "artifacts/kinds/obligation.yaml"
            },
            {
              "confidence": "cited",
              "path": "artifacts/lifecycles/obligation.yaml"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/src/planning.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/plan/aep-backend-markdown/tests/evidence.rs"
            }
          ]
        },
        {
          "id": "story:governed-dogfood-run",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": ".engineering/checks/run.sh"
            },
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver/src/run.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/src/drive.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/plan/harness-wave-4-governed-dogfood.md"
            },
            {
              "confidence": "cited",
              "path": "drivers/development/checks.yaml"
            },
            {
              "confidence": "cited",
              "path": "drivers/development/default.yaml"
            }
          ]
        },
        {
          "id": "story:recurrence-key",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "inferred",
              "path": "crates/profile/aep-profile-operations"
            },
            {
              "confidence": "inferred",
              "path": "protocols/aop/1.yaml"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/workflow.schema.json"
            },
            {
              "confidence": "cited",
              "path": "workflows/incidents/standard.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 8,
      "artifacts": [
        {
          "id": "story:legacy-ess-report-calendar-refusal",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli/src/planning.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli/tests"
            }
          ]
        },
        {
          "id": "story:reusable-workflow-nodes",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver"
            },
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver-spec"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "cited",
              "path": "drivers/development/default.yaml"
            }
          ]
        },
        {
          "id": "story:time-based-transitions",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "artifacts/lifecycles"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-engine"
            }
          ]
        }
      ]
    },
    {
      "wave": 9,
      "artifacts": [
        {
          "id": "story:decision-with-default",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "artifacts/kinds"
            },
            {
              "confidence": "inferred",
              "path": "artifacts/lifecycles"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-domain/src/artifact.rs"
            }
          ]
        },
        {
          "id": "story:review-lens-value",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "inferred",
              "path": "crates/govern/aep-domain"
            }
          ]
        }
      ]
    },
    {
      "wave": 10,
      "artifacts": [
        {
          "id": "story:task-scoped-artifact-requirements",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver-spec"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-engine"
            },
            {
              "confidence": "cited",
              "path": "drivers/development/default.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 11,
      "artifacts": [
        {
          "id": "story:provenance-scale",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "cited",
              "path": "docs/guide/open-vocabulary.md"
            },
            {
              "confidence": "inferred",
              "path": "protocols/aep/1.yaml"
            }
          ]
        },
        {
          "id": "story:the-store-knows-who-wrote-it",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/drive/aep-driver"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "inferred",
              "path": "crates/plan/aep-backend-markdown"
            }
          ]
        }
      ]
    },
    {
      "wave": 12,
      "artifacts": [
        {
          "id": "story:three-arm-pilot-2",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "conformance/eval"
            },
            {
              "confidence": "cited",
              "path": "conformance/eval/development-honest/expectations.trace.yaml"
            },
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "cited",
              "path": "docs/plan/eval-program-three-arms.md"
            },
            {
              "confidence": "cited",
              "path": "drivers/development/default.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 13,
      "artifacts": [
        {
          "id": "story:transcript-diff",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "inferred",
              "path": "crates/observe/trace-spec"
            }
          ]
        }
      ]
    },
    {
      "wave": 14,
      "artifacts": [
        {
          "id": "story:walk-an-artifact-up-its-ladder",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "inferred",
              "path": "website/docs/reference/cli.md"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:a-story-records-where-it-lands",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:a-tag-can-be-filtered-on",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:adopt-documents-into-the-event-log",
      "path": "crates/plan/aep-backend-markdown/src/journal.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:attested-approver",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:bulk-create-from-a-manifest",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:drive-watch-is-a-verb",
      "path": "AGENTS.md",
      "confidence": "cited"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:dry-run-on-the-write-verbs",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:external-clock-obligations",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:gate-lanes-count-what-ran",
      "path": "Taskfile.yml",
      "confidence": "cited"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:harness-denies-direct-store-writes",
      "path": "AGENTS.md",
      "confidence": "cited"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:legacy-ess-report-calendar-refusal",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-stale-binary-refuses-itself",
      "b": "story:per-record-horizons",
      "path": "AGENTS.md",
      "confidence": "cited"
    },
    {
      "a": "story:a-story-records-where-it-lands",
      "b": "story:a-tag-can-be-filtered-on",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-story-records-where-it-lands",
      "b": "story:attested-approver",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-story-records-where-it-lands",
      "b": "story:bulk-create-from-a-manifest",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-story-records-where-it-lands",
      "b": "story:decision-with-default",
      "path": "artifacts/kinds",
      "confidence": "inferred"
    },
    {
      "a": "story:a-story-records-where-it-lands",
      "b": "story:decision-with-default",
      "path": "crates/govern/aep-domain/src/artifact.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-story-records-where-it-lands",
      "b": "story:dry-run-on-the-write-verbs",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-story-records-where-it-lands",
      "b": "story:external-clock-obligations",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-story-records-where-it-lands",
      "b": "story:legacy-ess-report-calendar-refusal",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-tag-can-be-filtered-on",
      "b": "story:attested-approver",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:a-tag-can-be-filtered-on",
      "b": "story:bulk-create-from-a-manifest",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:a-tag-can-be-filtered-on",
      "b": "story:dry-run-on-the-write-verbs",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:a-tag-can-be-filtered-on",
      "b": "story:external-clock-obligations",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:a-tag-can-be-filtered-on",
      "b": "story:legacy-ess-report-calendar-refusal",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:acquisition-phase-set",
      "b": "story:corpus-observables",
      "path": "docs/guide/adopting.md",
      "confidence": "cited"
    },
    {
      "a": "story:acquisition-phase-set",
      "b": "story:corpus-observables",
      "path": "examples",
      "confidence": "cited"
    },
    {
      "a": "story:acquisition-phase-set",
      "b": "story:corpus-observables",
      "path": "protocols",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:communication-publish-capability",
      "path": "CHANGELOG.md",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:completion-needs-evidence",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:journal-carries-digests-not-bodies",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:journal-carries-digests-not-bodies",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:native-arm-needs-a-window-that-fits",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:native-plugin-eval",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:per-record-horizons",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:per-record-horizons",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:per-record-horizons",
      "path": "crates/govern/aep-engine",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:provenance-scale",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:published-pattern-residue",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:published-pattern-residue",
      "path": "schemas/generated/protocol.schema.json",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:recurrence-key",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:recurrence-key",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:recurrence-key",
      "path": "schemas/generated/workflow.schema.json",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:review-lens-value",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:review-lens-value",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:source-record-evidence",
      "path": "CHANGELOG.md",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:source-record-evidence",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-engine",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:admit-ess-conformance-coverage",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "website/docs/reference/cli.md",
      "confidence": "inferred"
    },
    {
      "a": "story:adopt-documents-into-the-event-log",
      "b": "story:dry-run-on-the-write-verbs",
      "path": "crates/edge/aep-cli/src/reverse.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:advisory-enforcement-tier",
      "b": "story:evidence-subject-binding",
      "path": "crates/govern/aep-domain/src/requirement.rs",
      "confidence": "cited"
    },
    {
      "a": "story:advisory-enforcement-tier",
      "b": "story:evidence-subject-binding",
      "path": "docs/plan/gap-register.md",
      "confidence": "cited"
    },
    {
      "a": "story:advisory-enforcement-tier",
      "b": "story:governed-dogfood-run",
      "path": ".engineering/checks/run.sh",
      "confidence": "inferred"
    },
    {
      "a": "story:agent-eval-cases",
      "b": "story:skill-text-in-context",
      "path": "crates/observe/trace-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:agent-eval-cases",
      "b": "story:streaming-checker",
      "path": "crates/observe/trace-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:agent-eval-cases",
      "b": "story:transcript-diff",
      "path": "crates/observe/trace-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:bulk-create-from-a-manifest",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:attested-approver",
      "b": "story:codex-adapter",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:cold-start-outlives-the-deadline",
      "path": "crates/drive/aep-driver/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:cold-start-outlives-the-deadline",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:confined-driven-workspace",
      "path": "crates/drive/aep-driver/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:confined-driven-workspace",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:drive-watch-is-a-verb",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:dry-run-on-the-write-verbs",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:attested-approver",
      "b": "story:external-clock-obligations",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:frame-subjects-from-the-step-map",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:governed-dogfood-run",
      "path": "crates/drive/aep-driver/src/run.rs",
      "confidence": "cited"
    },
    {
      "a": "story:attested-approver",
      "b": "story:governed-dogfood-run",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:attested-approver",
      "b": "story:legacy-ess-report-calendar-refusal",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:bulk-create-from-a-manifest",
      "b": "story:dry-run-on-the-write-verbs",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:bulk-create-from-a-manifest",
      "b": "story:external-clock-obligations",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:bulk-create-from-a-manifest",
      "b": "story:legacy-ess-report-calendar-refusal",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:codex-adapter",
      "b": "story:cold-start-outlives-the-deadline",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:codex-adapter",
      "b": "story:confined-driven-workspace",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:codex-adapter",
      "b": "story:drive-watch-is-a-verb",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:codex-adapter",
      "b": "story:frame-subjects-from-the-step-map",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:codex-adapter",
      "b": "story:governed-dogfood-run",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:cold-start-outlives-the-deadline",
      "b": "story:confined-driven-workspace",
      "path": "crates/drive/aep-driver/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:cold-start-outlives-the-deadline",
      "b": "story:confined-driven-workspace",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:cold-start-outlives-the-deadline",
      "b": "story:drive-watch-is-a-verb",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:cold-start-outlives-the-deadline",
      "b": "story:frame-subjects-from-the-step-map",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:cold-start-outlives-the-deadline",
      "b": "story:governed-dogfood-run",
      "path": "crates/drive/aep-driver/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:cold-start-outlives-the-deadline",
      "b": "story:governed-dogfood-run",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:communication-publish-capability",
      "b": "story:provenance-scale",
      "path": "docs/guide/open-vocabulary.md",
      "confidence": "cited"
    },
    {
      "a": "story:communication-publish-capability",
      "b": "story:provenance-scale",
      "path": "protocols/aep/1.yaml",
      "confidence": "inferred"
    },
    {
      "a": "story:communication-publish-capability",
      "b": "story:source-record-evidence",
      "path": "CHANGELOG.md",
      "confidence": "cited"
    },
    {
      "a": "story:communication-publish-capability",
      "b": "story:source-record-evidence",
      "path": "docs/guide/open-vocabulary.md",
      "confidence": "cited"
    },
    {
      "a": "story:completion-needs-evidence",
      "b": "story:drive-watch-is-a-verb",
      "path": "docs/plan/harness-wave-4-governed-dogfood.md",
      "confidence": "cited"
    },
    {
      "a": "story:completion-needs-evidence",
      "b": "story:governed-dogfood-run",
      "path": "docs/plan/harness-wave-4-governed-dogfood.md",
      "confidence": "cited"
    },
    {
      "a": "story:completion-needs-evidence",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:completion-needs-evidence",
      "b": "story:per-record-horizons",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:completion-needs-evidence",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:completion-needs-evidence",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:confined-driven-workspace",
      "b": "story:drive-watch-is-a-verb",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confined-driven-workspace",
      "b": "story:drive-watch-is-a-verb",
      "path": "scripts/drive-score",
      "confidence": "cited"
    },
    {
      "a": "story:confined-driven-workspace",
      "b": "story:frame-subjects-from-the-step-map",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confined-driven-workspace",
      "b": "story:governed-dogfood-run",
      "path": "crates/drive/aep-driver/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confined-driven-workspace",
      "b": "story:governed-dogfood-run",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:decision-with-default",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "artifacts/lifecycles",
      "confidence": "inferred"
    },
    {
      "a": "story:decision-with-default",
      "b": "story:time-based-transitions",
      "path": "artifacts/lifecycles",
      "confidence": "inferred"
    },
    {
      "a": "story:drive-watch-is-a-verb",
      "b": "story:frame-subjects-from-the-step-map",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:drive-watch-is-a-verb",
      "b": "story:governed-dogfood-run",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:drive-watch-is-a-verb",
      "b": "story:governed-dogfood-run",
      "path": "docs/plan/harness-wave-4-governed-dogfood.md",
      "confidence": "cited"
    },
    {
      "a": "story:drive-watch-is-a-verb",
      "b": "story:harness-denies-direct-store-writes",
      "path": "AGENTS.md",
      "confidence": "cited"
    },
    {
      "a": "story:drive-watch-is-a-verb",
      "b": "story:per-record-horizons",
      "path": "AGENTS.md",
      "confidence": "cited"
    },
    {
      "a": "story:dry-run-on-the-write-verbs",
      "b": "story:external-clock-obligations",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:dry-run-on-the-write-verbs",
      "b": "story:legacy-ess-report-calendar-refusal",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "cited"
    },
    {
      "a": "story:evidence-subject-binding",
      "b": "story:fanout-promote",
      "path": "crates/govern/aep-engine/src/execution.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:external-clock-obligations",
      "b": "story:legacy-ess-report-calendar-refusal",
      "path": "crates/edge/aep-cli/src/planning.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:frame-subjects-from-the-step-map",
      "b": "story:governed-dogfood-run",
      "path": "crates/edge/aep-cli/src/drive.rs",
      "confidence": "cited"
    },
    {
      "a": "story:governed-dogfood-run",
      "b": "story:native-arm-needs-a-window-that-fits",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:governed-dogfood-run",
      "b": "story:reusable-workflow-nodes",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:governed-dogfood-run",
      "b": "story:task-scoped-artifact-requirements",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:governed-dogfood-run",
      "b": "story:three-arm-pilot-2",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:harness-denies-direct-store-writes",
      "b": "story:legacy-ess-report-calendar-refusal",
      "path": "crates/edge/aep-cli/tests",
      "confidence": "inferred"
    },
    {
      "a": "story:harness-denies-direct-store-writes",
      "b": "story:per-record-horizons",
      "path": "AGENTS.md",
      "confidence": "cited"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:native-arm-needs-a-window-that-fits",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:native-plugin-eval",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/plan/aep-backend-markdown",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:per-record-horizons",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:per-record-horizons",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:provenance-scale",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:published-pattern-residue",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:recurrence-key",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:recurrence-key",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:review-lens-value",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:review-lens-value",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:source-record-evidence",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/plan/aep-backend-markdown",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:journal-carries-digests-not-bodies",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:native-plugin-eval",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:per-record-horizons",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:recurrence-key",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:reusable-workflow-nodes",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:review-lens-value",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:task-scoped-artifact-requirements",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:three-arm-pilot-2",
      "path": "docs/plan/eval-program-three-arms.md",
      "confidence": "inferred"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:three-arm-pilot-2",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-arm-needs-a-window-that-fits",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:outbound-claims-and-status-vocabulary",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:per-record-horizons",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:recurrence-key",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:review-lens-value",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:native-plugin-eval",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:per-record-horizons",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:per-record-horizons",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:per-record-horizons",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:provenance-scale",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:published-pattern-residue",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:recurrence-key",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:recurrence-key",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:review-lens-value",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:review-lens-value",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:source-record-evidence",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/plan/aep-backend-markdown",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:time-based-transitions",
      "path": "artifacts/lifecycles",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:outbound-claims-and-status-vocabulary",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:partial-edits-cost-more-replay-than-they-save",
      "b": "story:published-pattern-residue",
      "path": "crates/drive/aep-driver-spec",
      "confidence": "cited"
    },
    {
      "a": "story:partial-edits-cost-more-replay-than-they-save",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/drive/aep-driver",
      "confidence": "inferred"
    },
    {
      "a": "story:partial-edits-cost-more-replay-than-they-save",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/drive/aep-driver-spec",
      "confidence": "cited"
    },
    {
      "a": "story:partial-edits-cost-more-replay-than-they-save",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/drive/aep-driver-spec",
      "confidence": "cited"
    },
    {
      "a": "story:partial-edits-cost-more-replay-than-they-save",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/drive/aep-driver",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:provenance-scale",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:published-pattern-residue",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:recurrence-key",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:recurrence-key",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:review-lens-value",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:review-lens-value",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:source-record-evidence",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-engine",
      "confidence": "cited"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:per-record-horizons",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:provenance-scale",
      "b": "story:published-pattern-residue",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:provenance-scale",
      "b": "story:recurrence-key",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:provenance-scale",
      "b": "story:review-lens-value",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:provenance-scale",
      "b": "story:source-record-evidence",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:provenance-scale",
      "b": "story:source-record-evidence",
      "path": "docs/guide/open-vocabulary.md",
      "confidence": "cited"
    },
    {
      "a": "story:provenance-scale",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:provenance-scale",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:published-pattern-residue",
      "b": "story:recurrence-key",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:published-pattern-residue",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/drive/aep-driver-spec",
      "confidence": "cited"
    },
    {
      "a": "story:published-pattern-residue",
      "b": "story:review-lens-value",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:published-pattern-residue",
      "b": "story:source-record-evidence",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:published-pattern-residue",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/drive/aep-driver-spec",
      "confidence": "cited"
    },
    {
      "a": "story:published-pattern-residue",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:published-pattern-residue",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:reusable-workflow-nodes",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:review-lens-value",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:review-lens-value",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:source-record-evidence",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:recurrence-key",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:review-lens-value",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/drive/aep-driver-spec",
      "confidence": "cited"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:task-scoped-artifact-requirements",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/drive/aep-driver",
      "confidence": "cited"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:three-arm-pilot-2",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:reusable-workflow-nodes",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:review-lens-value",
      "b": "story:source-record-evidence",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:review-lens-value",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:review-lens-value",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:review-lens-value",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:review-lens-value",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:review-lens-value",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:review-lens-value",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:review-lens-value",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:skill-text-in-context",
      "b": "story:streaming-checker",
      "path": "crates/observe/trace-domain",
      "confidence": "inferred"
    },
    {
      "a": "story:skill-text-in-context",
      "b": "story:streaming-checker",
      "path": "crates/observe/trace-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:skill-text-in-context",
      "b": "story:transcript-diff",
      "path": "crates/observe/trace-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:source-record-evidence",
      "b": "story:task-scoped-artifact-requirements",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:source-record-evidence",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:streaming-checker",
      "b": "story:transcript-diff",
      "path": "crates/observe/trace-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:task-scoped-artifact-requirements",
      "b": "story:the-store-knows-who-wrote-it",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:task-scoped-artifact-requirements",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:task-scoped-artifact-requirements",
      "b": "story:three-arm-pilot-2",
      "path": "drivers/development/default.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:task-scoped-artifact-requirements",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-domain",
      "confidence": "cited"
    },
    {
      "a": "story:task-scoped-artifact-requirements",
      "b": "story:time-based-transitions",
      "path": "crates/govern/aep-engine",
      "confidence": "inferred"
    },
    {
      "a": "story:task-scoped-artifact-requirements",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:task-scoped-artifact-requirements",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:the-store-knows-who-wrote-it",
      "b": "story:three-arm-pilot-2",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:the-store-knows-who-wrote-it",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:the-store-knows-who-wrote-it",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "cited"
    },
    {
      "a": "story:three-arm-pilot-2",
      "b": "story:transcript-diff",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:three-arm-pilot-2",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    },
    {
      "a": "story:transcript-diff",
      "b": "story:walk-an-artifact-up-its-ladder",
      "path": "crates/edge/aep-cli",
      "confidence": "inferred"
    }
  ],
  "unassessed": [
    "story:a-directory-and-a-file-inside-it-read-as-disjoint",
    "story:a-dropped-stream-does-not-end-a-run",
    "story:a-guarded-rung-counts-records-not-verdicts",
    "story:an-llm-step-cannot-declare-its-tools",
    "story:compile-scope-into-a-run",
    "story:history-shows-a-removed-edge",
    "story:implementor-and-adversary-agents",
    "story:journal-actor-from-configuration",
    "story:relations-order-is-the-same-in-every-store",
    "story:two-adapters-two-paths",
    "story:wave-as-a-surface",
    "story:wave-skill-defects-found-by-running-it"
  ],
  "cycles": []
}
```

## aep plan artifact waves --kind story --status proposed --format json

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:admit-ess-conformance-coverage",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-cli"
            },
            {
              "confidence": "cited",
              "path": "crates/edge/aep-schema"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-domain"
            },
            {
              "confidence": "cited",
              "path": "crates/govern/aep-engine"
            },
            {
              "confidence": "cited",
              "path": "crates/observe/aep-ess-evidence"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/ess-conformance-coverage-evidence.md"
            },
            {
              "confidence": "inferred",
              "path": "principles/verification/ess-conformance-coverage.yaml"
            },
            {
              "confidence": "inferred",
              "path": "profiles/development-ess-conformance-coverage.yaml"
            },
            {
              "confidence": "inferred",
              "path": "protocols/adp-ess-conformance-coverage/1.yaml"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/artifact-lifecycle.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/artifact-manifest.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/driver-steps.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/event.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/evidence.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/principle.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/profile.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/protocol.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/task.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "schemas/generated/workflow.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "website/docs/reference/cli.md"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [],
  "unassessed": [],
  "cycles": []
}
```

## Lifecycle and validation

```text
story starts at draft
  active -> implemented, archived
  archived -> nothing
  draft -> proposed, archived
  implemented -> archived
  proposed -> draft, rejected, active
  rejected -> archived
273 file(s) in /home/timo/.local/state/worktree/trees/b10x/aep/ess-conformance-v2-reader/.engineering/planning: 273 artifact(s)
53 document(s) predate the event log
4 closed on an assertion:
  - story:guard-tests reached implemented on an assertion rather than a record — the evidence was claimed, not held
  - story:journal-reconciliation reached implemented on an assertion rather than a record — the evidence was claimed, not held
  - story:changelog-claims-are-checked reached implemented on an assertion rather than a record — the evidence was claimed, not held
  - story:sqlite-backend-adapter reached implemented on an assertion rather than a record — the evidence was claimed, not held
1 review(s) recorded no findings block:
  - review-result:ess-conformance-v2-counts-adversary-pass-1 states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere
valid
```
