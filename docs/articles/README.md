# Historical Analyses and Research Notes

This folder collects long-form historical analyses, launch-article drafts, and supporting research notes for the `perl-lsp` codebase.

These documents intentionally preserve dated observations and period-specific metrics. For current release posture, current capability coverage, and evidence-backed receipts, use [../project/CURRENT_STATUS.md](../project/CURRENT_STATUS.md) and [../project/ROADMAP.md](../project/ROADMAP.md).

## Polished Historical Analyses

- [FIVE_ERAS.md](FIVE_ERAS.md) — five distinct eras of AI-assisted development across the project
- [SWARM_METHODOLOGY.md](SWARM_METHODOLOGY.md) — the agentic swarm methodology and operating model
- [ZERO_PANIC.md](ZERO_PANIC.md) — reliability, failure handling, and security posture for the language server
- [PARSING_PERL.md](PARSING_PERL.md) — why Perl is hard to parse and how the parser tackles it
- [CURIOSITIES.md](CURIOSITIES.md) — unusual records, architectural oddities, and codebase curiosities

## Research and Source Material

### Era and Workflow Archaeology

- [research/ERA_TIMELINE.md](research/ERA_TIMELINE.md) — era-by-era timeline and velocity notes
- [research/ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md](research/ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md) — the intentional late-2025 to early-2026 slowdown that built parser, architecture, and quality foundations
- [research/COPILOT_FLEET_ARCHAEOLOGY.md](research/COPILOT_FLEET_ARCHAEOLOGY.md) — the February 27 to March 5, 2026 Copilot CLI firehose and its attribution boundary
- [research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md](research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md) — March 11 to 19, 2026 as a mixed-tool period of short Claude swarm bursts plus Codex waves
- [research/Q4_Q1_HANDS_ON_ARCHAEOLOGY.md](research/Q4_Q1_HANDS_ON_ARCHAEOLOGY.md) — the late-2025 to early-2026 stable, release-focused, but still maintainer-heavy bridge era
- [research/Q3_SWARM_PR_ARCHAEOLOGY.md](research/Q3_SWARM_PR_ARCHAEOLOGY.md) — how late Q3 2025 becomes a PR-heavy Claude swarm rather than a mostly direct coding stream
- [research/Q3_SWARM_TALK_ARCHAEOLOGY.md](research/Q3_SWARM_TALK_ARCHAEOLOGY.md) — how the Q3 2025 swarm talk articulated trusted change, flows, receipts, and adversarial verification before the control plane fully hardened

### Control Plane and Process Archaeology

- [research/CONTROL_PLANE_ARCHAEOLOGY.md](research/CONTROL_PLANE_ARCHAEOLOGY.md) — tracked `.claude` and `.jules` lineage from Q3 swarm packs to the current control plane
- [research/ISSUE_LABEL_ARCHAEOLOGY.md](research/ISSUE_LABEL_ARCHAEOLOGY.md) — how label families and title prefixes gave the issue tracker a typed routing vocabulary for swarm discovery, self-improvement, and learning artifacts
- [research/ISSUE_ROUTING_ARCHAEOLOGY.md](research/ISSUE_ROUTING_ARCHAEOLOGY.md) — how GitHub issues became swarm overflow memory and a typed routing surface instead of just backlog storage
- [research/ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md](research/ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md) — how issue bodies, PR bodies, learning issues, and article issues together made the GitHub ledger recoverable swarm memory
- [research/JULES_LANE_ARCHAEOLOGY.md](research/JULES_LANE_ARCHAEOLOGY.md) — January 2026 Bolt/Sentinel/Palette lanes as proto-specialists
- [research/MAINTAINER_BRIDGE_ARCHAEOLOGY.md](research/MAINTAINER_BRIDGE_ARCHAEOLOGY.md) — how autumn 2025 large PRs acted as maintained bridge bundles before the January `maint/pr-*` naming made the pattern explicit
- [research/MERGE_DISCIPLINE_ARCHAEOLOGY.md](research/MERGE_DISCIPLINE_ARCHAEOLOGY.md) — PR governance from Q3 flow packs to `green-merge`, `review-pr`, and `triage-prs`
- [research/MAINTAINER_VISION_ARCHAEOLOGY.md](research/MAINTAINER_VISION_ARCHAEOLOGY.md) — repeated waves of encoding maintainer judgment into prompts, lanes, commands, skills, hooks, and state
- [research/SWARM_STATE_ARCHAEOLOGY.md](research/SWARM_STATE_ARCHAEOLOGY.md) — how `.claude/swarm-state/` became the committed memory ledger for the current swarm
- [research/SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md](research/SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md) — how committed swarm-state files and issue-title prefixes split memory into queue state, pitfalls, findings, learning, and article artifacts
- [research/SWARM_SURFACE_EVOLUTION.md](research/SWARM_SURFACE_EVOLUTION.md) — Jan→Mar 2026 transition from commands to the current skills/hooks/swarm-state control plane

### Trust, Provenance, and AI-Native Operations

- [research/AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md](research/AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md) — how the repo moved from assisted coding toward an AI-native, receipt-driven operating model
- [research/MODE_SHIFT_ARCHAEOLOGY.md](research/MODE_SHIFT_ARCHAEOLOGY.md) — how the repo moved from assisted to native to industrialized work, including the nuance that Q4/Q1 was already AI-native but still hands-on
- [research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md](research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md) — how issue `#210` turned proof governance into gate harnesses, receipt schemas, status checks, and later audit prompts
- [research/PROVENANCE_RECEIPTS_ARCHAEOLOGY.md](research/PROVENANCE_RECEIPTS_ARCHAEOLOGY.md) — how receipts, provenance schemas, and forensics turned proof into structured artifacts
- [research/RECEIPTS_LIE_ARCHAEOLOGY.md](research/RECEIPTS_LIE_ARCHAEOLOGY.md) — how PR `#209` and later validator repairs taught the repo that proof artifacts need governance too
- [research/TRUSTED_CHANGE_ARCHAEOLOGY.md](research/TRUSTED_CHANGE_ARCHAEOLOGY.md) — how the repo industrialized trust through gates, receipts, drift checks, and durable lessons
- [research/VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md](research/VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md) — how the repo kept repairing helpers, gates, baselines, and assertions when the measurement surface itself proved incomplete

### CI, Queue, and Throughput Archaeology

- [research/CI_BUDGET_DISCIPLINE_ARCHAEOLOGY.md](research/CI_BUDGET_DISCIPLINE_ARCHAEOLOGY.md) — how CI spend, lane design, and local-first validation became an explicit engineering constraint
- [research/MAINTAINER_GATEKEEPER_ARCHAEOLOGY.md](research/MAINTAINER_GATEKEEPER_ARCHAEOLOGY.md) — how the human role shifted toward architectural direction, selection, merge pacing, and trusted-change oversight
- [research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md](research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md) — how the three-wide merge queue and CI throughput shaped swarm behavior and issue overflow

### GitHub PR Ledger Archaeology

- [research/PR_BRANCH_NAMING_ARCHAEOLOGY.md](research/PR_BRANCH_NAMING_ARCHAEOLOGY.md) — how head branches and PR titles reflect changing workflow eras
- [research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md](research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md) — how issues and PRs evolved into a shared delivery ledger for fixes, closures, learning reports, and article evidence
- [research/PR_LIFECYCLE_ARCHAEOLOGY.md](research/PR_LIFECYCLE_ARCHAEOLOGY.md) — how drafts, merges, closures, and disposal became part of the operating model
- [research/REVIEW_LABEL_ARCHAEOLOGY.md](research/REVIEW_LABEL_ARCHAEOLOGY.md) — how the canonical Q3 swarm encoded review stages, gates, lanes, and merge readiness directly in GitHub labels alongside the three-phase `issue-to-draft` / `draft-to-pr` / `pr-to-merge` flow
- [research/PR_REVIEW_LOOP_ARCHAEOLOGY.md](research/PR_REVIEW_LOOP_ARCHAEOLOGY.md) — how cleanup passes, follow-up PRs, and review repair became explicit and normal
- [research/PR_SLICE_SIZE_ARCHAEOLOGY.md](research/PR_SLICE_SIZE_ARCHAEOLOGY.md) — how the PR archive balances many small bounded slices with a smaller number of deliberate umbrella changes
- [research/PR_WAVE_ARCHAEOLOGY.md](research/PR_WAVE_ARCHAEOLOGY.md) — how the repository moves in bursty PR waves rather than a smooth stream

### Research Maps and Source Drafts

- [research/BLOG_MATERIAL_INDEX.md](research/BLOG_MATERIAL_INDEX.md) — scout-generated map of article angles and evidence
- [research/DEVELOPMENT_ARCHAEOLOGY.md](research/DEVELOPMENT_ARCHAEOLOGY.md) — development-history archaeology and launch-story findings
- [research/DOCUMENTATION_SUMMARY.md](research/DOCUMENTATION_SUMMARY.md) — packaging summary for the article set
- [research/SCOUT_SUMMARY.md](research/SCOUT_SUMMARY.md) — summary of the scout output delivered during the session
- [research/TESTING_INFRASTRUCTURE_GAPS_SCOUT.md](research/TESTING_INFRASTRUCTURE_GAPS_SCOUT.md) — testing-gap research that fed related follow-up work
- [research/five_eras_swarm_methodology.md](research/five_eras_swarm_methodology.md) — source draft behind the five-eras analysis
- [research/swarm_development_methodology.md](research/swarm_development_methodology.md) — source draft behind the swarm methodology article
- [research/perl_parsing_challenges_report.md](research/perl_parsing_challenges_report.md) — source report behind the Parsing Perl article

## Related Project Docs

- [../project/CODEBASE_HISTORY.md](../project/CODEBASE_HISTORY.md) — longer-form repository history across the full project arc
- [../project/AGENTIC_DEVELOPMENT.md](../project/AGENTIC_DEVELOPMENT.md) — earlier case-study framing for AI-assisted development
- [../project/AGENTIC_SWARM_ERA.md](../project/AGENTIC_SWARM_ERA.md) — earlier write-up focused on the swarm era
- [../project/CODEBASE_CURIOSITIES.md](../project/CODEBASE_CURIOSITIES.md) — current-tree curiosity tour
- [../project/JULES_BOT_ANALYSIS.md](../project/JULES_BOT_ANALYSIS.md) — earlier analysis of the January 2026 draft-PR bridge
- [../project/PARSING_PERL.md](../project/PARSING_PERL.md) — existing parser deep dive in the project-docs track
- [../project/QUALITY_INFRASTRUCTURE.md](../project/QUALITY_INFRASTRUCTURE.md) — broader quality and security infrastructure documentation
