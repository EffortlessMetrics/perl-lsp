// release-readiness.js — Adversarial release-readiness workflow for perl-lsp-swarm.
//
// IMPORTANT: This workflow produces a dispatch recommendation only.
// Dispatch itself is NEVER performed by this workflow — human approval is required
// before any tag, publish, or release action is taken. The workflow always returns
// explicitly_requires_human_approval: true and never initiates a release.
//
// Usage:
//   Invoke via the Claude Code Workflow tool with name="release-readiness" and args:
//     { swarmSha: "<40-char SHA>", sourceRepo: "<org/repo>", version: "<semver>" }
//
// Cross-reference:
//   docs/workflows/release-readiness.md — what it checks, human-approval boundary
//   docs/reference/QUEUE_CONVERGENCE_DOCTRINE.md — 0.16.0-cycle lessons
//   docs/project/plans/2026-06-convergence-to-release.md — the four-layer plan

export const meta = {
  name: "release-readiness",
  description:
    "Adversarial release-readiness check for perl-lsp-swarm. Runs six sequential phases " +
    "(ancestry, consistency, receipts, smoke, claims, verdict) and returns a structured " +
    "go/no-go recommendation. Human approval is always required before any release action.",
  whenToUse:
    "Before cutting a release tag or publishing a new version. Run after the merge queue " +
    "has drained, CI is green on the release commit, and the release captain has completed " +
    "the pre-release checklist. Do not run on a branch that has open needs-* routing labels.",
  phases: [
    {
      id: "phase-1-ancestry",
      name: "Ancestry",
      model: "haiku",
      description:
        "Prove that the swarm SHA is reachable from the intended release branch " +
        "(merge-base --is-ancestor check). Explain the tree diff between the swarm " +
        "SHA and the release branch HEAD. Only documented exclusions are accepted.",
      inputs: ["swarmSha", "sourceRepo"],
      outputSchema: {
        type: "object",
        required: ["swarm_sha", "release_branch_head", "is_ancestor", "ancestry_command", "ancestry_output", "tree_diff_summary", "unexplained_exclusions"],
        properties: {
          swarm_sha: { type: "string", minLength: 40, maxLength: 40 },
          release_branch_head: { type: "string", minLength: 40, maxLength: 40 },
          is_ancestor: { type: "boolean" },
          ancestry_command: { type: "string" },
          ancestry_output: { type: "string", enum: ["ANCESTOR", "NOT ANCESTOR"] },
          tree_diff_summary: { type: "string" },
          unexplained_exclusions: {
            type: "array",
            items: { type: "string" },
            description: "Files or paths present in the diff that are not covered by a documented exclusion. Must be empty for ancestry to PASS."
          },
          verdict: { type: "string", enum: ["PASS", "FAIL"] }
        }
      },
      checks: [
        "git merge-base --is-ancestor <swarmSha> <releaseBranchHead> — output must be ANCESTOR",
        "git diff <swarmSha> <releaseBranchHead> --stat — explain every changed path",
        "Verify all diff paths are covered by documented exclusions (docs/reference/RELEASE_PROOF_PROTOCOL.md)"
      ]
    },
    {
      id: "phase-2-consistency",
      name: "Consistency",
      model: "haiku",
      description:
        "Verify version sites are internally consistent: Cargo.toml workspace version, " +
        "CHANGELOG.md entry, RELEASE_HISTORY.md, and any version constants in source. " +
        "Verify the release tag does not yet exist. Verify release-history script passes.",
      inputs: ["version"],
      outputSchema: {
        type: "object",
        required: ["version_sites", "tag_absent", "changelog_entry_present", "release_history_passes", "inconsistencies", "verdict"],
        properties: {
          version_sites: {
            type: "array",
            items: {
              type: "object",
              required: ["file", "found_version", "matches_target"],
              properties: {
                file: { type: "string" },
                found_version: { type: "string" },
                matches_target: { type: "boolean" }
              }
            }
          },
          tag_absent: { type: "boolean", description: "True if the release tag does not yet exist in the remote." },
          changelog_entry_present: { type: "boolean" },
          release_history_passes: { type: "boolean" },
          inconsistencies: { type: "array", items: { type: "string" } },
          verdict: { type: "string", enum: ["PASS", "FAIL"] }
        }
      },
      checks: [
        "cargo xtask check-version-sync — must exit 0",
        "grep '^## \\[<version>\\]' CHANGELOG.md — entry must exist",
        "grep '<version>' RELEASE_HISTORY.md — entry must exist",
        "git tag --list 'v<version>' — must return empty (tag absent)",
        "cargo xtask release-history-check — must exit 0"
      ]
    },
    {
      id: "phase-3-receipts",
      name: "Receipts",
      model: "haiku",
      description:
        "Validate that quality-gate, coverage, and ripr receipts are fresh and pass in " +
        "--check mode. Enumerate all active proof exceptions with their expiry dates. " +
        "A receipt older than 24 hours or missing is a blocker.",
      inputs: ["swarmSha"],
      outputSchema: {
        type: "object",
        required: ["quality_gate_receipt", "coverage_receipt", "ripr_receipt", "active_exceptions", "stale_receipts", "verdict"],
        properties: {
          quality_gate_receipt: {
            type: "object",
            required: ["present", "sha", "age_hours", "result"],
            properties: {
              present: { type: "boolean" },
              sha: { type: "string" },
              age_hours: { type: "number" },
              result: { type: "string", enum: ["PASS", "FAIL", "MISSING"] }
            }
          },
          coverage_receipt: {
            type: "object",
            required: ["present", "sha", "age_hours", "result"],
            properties: {
              present: { type: "boolean" },
              sha: { type: "string" },
              age_hours: { type: "number" },
              result: { type: "string", enum: ["PASS", "FAIL", "MISSING"] }
            }
          },
          ripr_receipt: {
            type: "object",
            required: ["present", "sha", "age_hours", "result"],
            properties: {
              present: { type: "boolean" },
              sha: { type: "string" },
              age_hours: { type: "number" },
              result: { type: "string", enum: ["PASS", "FAIL", "MISSING"] }
            }
          },
          active_exceptions: {
            type: "array",
            items: {
              type: "object",
              required: ["exception_id", "description", "expiry_date", "approved_by"],
              properties: {
                exception_id: { type: "string" },
                description: { type: "string" },
                expiry_date: { type: "string", pattern: "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" },
                approved_by: { type: "string" }
              }
            }
          },
          stale_receipts: { type: "array", items: { type: "string" } },
          verdict: { type: "string", enum: ["PASS", "FAIL"] }
        }
      },
      checks: [
        "just pr-fast --check — re-run gate in check mode, do not write",
        "just coverage-lcov --check — verify coverage receipt is fresh",
        "just ripr --check — verify ripr receipt is fresh",
        "Read .receipts/ directory — enumerate all active proof exceptions with expiry dates",
        "Any receipt older than 24 hours from the release commit timestamp is STALE"
      ]
    },
    {
      id: "phase-4-smoke",
      name: "Smoke",
      model: "sonnet",
      description:
        "Run the release build, inline/LSP-UX smoke tests, and package verification. " +
        "Report build output, binary sizes, and smoke test results. Smoke failures are blockers.",
      inputs: ["version"],
      outputSchema: {
        type: "object",
        required: ["release_build", "smoke_tests", "package_verification", "binary_sizes", "verdict"],
        properties: {
          release_build: {
            type: "object",
            required: ["command", "exit_code", "warnings_count", "errors_count"],
            properties: {
              command: { type: "string" },
              exit_code: { type: "integer" },
              warnings_count: { type: "integer" },
              errors_count: { type: "integer" }
            }
          },
          smoke_tests: {
            type: "array",
            items: {
              type: "object",
              required: ["name", "result", "output_summary"],
              properties: {
                name: { type: "string" },
                result: { type: "string", enum: ["PASS", "FAIL", "SKIP"] },
                output_summary: { type: "string" }
              }
            }
          },
          package_verification: {
            type: "object",
            required: ["archive_present", "checksums_valid", "install_script_passes"],
            properties: {
              archive_present: { type: "boolean" },
              checksums_valid: { type: "boolean" },
              install_script_passes: { type: "boolean" }
            }
          },
          binary_sizes: {
            type: "object",
            description: "Key=binary name, value=size in bytes.",
            additionalProperties: { type: "integer", minimum: 0 }
          },
          verdict: { type: "string", enum: ["PASS", "FAIL"] }
        }
      },
      checks: [
        "cargo build -p perl-lsp-rs --release — must exit 0 with zero errors",
        "cargo build -p perl-dap --release — must exit 0 with zero errors",
        "cargo test --workspace --lib -- --test-threads=2 — must exit 0",
        "just ci-lsp-def — LSP semantic definition smoke",
        "dist-workspace pack — verify archive structure and checksums",
        "install.sh --dry-run — verify install script does not error"
      ]
    },
    {
      id: "phase-5-claims",
      name: "Claims",
      model: "haiku",
      description:
        "Audit release notes and public channel claims (CHANGELOG.md, README.md, " +
        "any draft announcement) against evidence. Flag any claim that uses 'live', " +
        "'production', 'all', or absolute language without a public verification artifact. " +
        "Unverified claims are blockers.",
      inputs: ["version"],
      outputSchema: {
        type: "object",
        required: ["claims_audited", "unverified_claims", "hallucinated_claims", "verdict"],
        properties: {
          claims_audited: { type: "integer", minimum: 0 },
          unverified_claims: {
            type: "array",
            items: {
              type: "object",
              required: ["file", "line", "claim_text", "required_evidence"],
              properties: {
                file: { type: "string" },
                line: { type: "integer" },
                claim_text: { type: "string" },
                required_evidence: { type: "string" }
              }
            }
          },
          hallucinated_claims: {
            type: "array",
            items: {
              type: "object",
              required: ["file", "line", "claim_text", "contradiction_evidence"],
              properties: {
                file: { type: "string" },
                line: { type: "integer" },
                claim_text: { type: "string" },
                contradiction_evidence: { type: "string" }
              }
            }
          },
          verdict: { type: "string", enum: ["PASS", "FAIL"] }
        }
      },
      checks: [
        "Read CHANGELOG.md entry for <version> — audit every claim",
        "Read README.md — flag any 'live', 'production', or unqualified absolute claims",
        "Cross-reference every feature claim against features.toml",
        "Cross-reference every test count / coverage number against docs/project/status/index.md",
        "Any claim using 'live' without a public URL or CI run receipt is UNVERIFIED"
      ]
    },
    {
      id: "phase-6-verdict",
      name: "Verdict",
      model: "sonnet",
      description:
        "Adversarial synthesizer: actively try to REFUTE readiness from the phase 1–5 " +
        "outputs. Assume the worst about each ambiguity. Return a structured go/no-go " +
        "recommendation. Human approval is always required — this workflow never dispatches.",
      inputs: ["phase1Result", "phase2Result", "phase3Result", "phase4Result", "phase5Result"],
      outputSchema: {
        type: "object",
        required: ["dispatch_recommendation", "blockers", "evidence", "phase_verdicts", "explicitly_requires_human_approval"],
        properties: {
          dispatch_recommendation: {
            type: "string",
            enum: ["go", "no-go"],
            description: "go = all phases PASS, no blockers. no-go = one or more blockers found."
          },
          blockers: {
            type: "array",
            description: "All blocking issues found across phases. Must be empty for go recommendation.",
            items: {
              type: "object",
              required: ["phase", "description", "evidence"],
              properties: {
                phase: { type: "string" },
                description: { type: "string" },
                evidence: { type: "string" }
              }
            }
          },
          evidence: {
            type: "array",
            description: "Supporting evidence for the go/no-go recommendation.",
            items: { type: "string" }
          },
          phase_verdicts: {
            type: "object",
            required: ["ancestry", "consistency", "receipts", "smoke", "claims"],
            properties: {
              ancestry: { type: "string", enum: ["PASS", "FAIL"] },
              consistency: { type: "string", enum: ["PASS", "FAIL"] },
              receipts: { type: "string", enum: ["PASS", "FAIL"] },
              smoke: { type: "string", enum: ["PASS", "FAIL"] },
              claims: { type: "string", enum: ["PASS", "FAIL"] }
            }
          },
          explicitly_requires_human_approval: {
            type: "boolean",
            const: true,
            description: "Always true. This workflow produces a recommendation only. A human must approve before any tag, publish, or release action is taken."
          }
        },
        additionalProperties: false
      },
      adversarialInstructions: [
        "Start from the assumption that the release is NOT ready. Work backwards.",
        "For every PASS verdict in phases 1–5, find the weakest assumption and state it.",
        "If any phase returned FAIL, that is a blocker regardless of other phases.",
        "Do not soften language: 'may be a concern' is not a blocker; 'is a blocker' is.",
        "The dispatch_recommendation is 'go' only when blockers is empty after adversarial scrutiny.",
        "explicitly_requires_human_approval must always be true — never suggest the workflow itself should dispatch."
      ]
    }
  ]
};
