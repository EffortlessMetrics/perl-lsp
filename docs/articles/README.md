# Historical Analyses and Research Notes

This folder collects long-form historical analyses, launch-article drafts, and supporting research notes for the `perl-lsp` codebase.

These documents intentionally preserve dated observations and period-specific metrics. For current release posture, current capability coverage, and evidence-backed receipts, use [../project/CURRENT_STATUS.md](../project/CURRENT_STATUS.md) and [../project/ROADMAP.md](../project/ROADMAP.md).

## Polished Historical Analyses

- [FIVE_ERAS.md](FIVE_ERAS.md) — five distinct eras of AI-assisted development across the project
- [SWARM_METHODOLOGY.md](SWARM_METHODOLOGY.md) — the agentic swarm methodology and operating model
- [ZERO_PANIC.md](ZERO_PANIC.md) — reliability, failure handling, and security posture for the language server
- [PARSING_PERL.md](PARSING_PERL.md) — why Perl is hard to parse and how the parser tackles it
- [WHEN_RECEIPTS_LIE.md](WHEN_RECEIPTS_LIE.md) — six real cases where structured evidence was technically correct but operationally misleading
- [CURIOSITIES.md](CURIOSITIES.md) — unusual records, architectural oddities, and codebase curiosities
- [REFERENCE_IMPLEMENTATION.md](REFERENCE_IMPLEMENTATION.md) — perl-lsp as a reference implementation of agentic software development
- [METHODOLOGY_REPLICATION_GUIDE.md](METHODOLOGY_REPLICATION_GUIDE.md) — practical guide for other teams to replicate the swarm methodology

## Research and Source Material

### Era and Workflow Archaeology

- [research/ERA_TIMELINE.md](research/ERA_TIMELINE.md) — era-by-era timeline and velocity notes
- [research/ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md](research/ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md) — the intentional late-2025 to early-2026 slowdown that built parser, architecture, and quality foundations
- [research/ALPHA_READINESS_ARCHAEOLOGY.md](research/ALPHA_READINESS_ARCHAEOLOGY.md) — how March 2026 kept shipped release truth separate from `v0.12.0` hardening plans while defining explicit alpha blockers and non-blockers
- [research/COPILOT_FLEET_ARCHAEOLOGY.md](research/COPILOT_FLEET_ARCHAEOLOGY.md) — the February 27 to March 5, 2026 Copilot CLI firehose and its attribution boundary
- [research/DIRECT_DELIVERY_ARCHAEOLOGY.md](research/DIRECT_DELIVERY_ARCHAEOLOGY.md) — how the early history still reads as direct delivery before mid-to-late September 2025 turns review, staging, and integration into the delivery model
- [research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md](research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md) — March 11 to 19, 2026 as a mixed-tool period of short Claude swarm bursts plus Codex waves
- [research/INSTALL_SURFACE_ARCHAEOLOGY.md](research/INSTALL_SURFACE_ARCHAEOLOGY.md) — how install scripts, health/info flags, editor discovery order, and managed downloads became part of the March 2026 launch trust surface
- [research/Q3_CONTROL_PLANE_ARCHAEOLOGY.md](research/Q3_CONTROL_PLANE_ARCHAEOLOGY.md) — how `agents4` turns the canonical Q3 swarm into a phase-aware operating surface with evolving gates, Perl-LSP-specific evidence, and worktree-serial discipline
- [research/Q4_Q1_HANDS_ON_ARCHAEOLOGY.md](research/Q4_Q1_HANDS_ON_ARCHAEOLOGY.md) — the late-2025 to early-2026 stable, release-focused, but still maintainer-heavy bridge era
- [research/Q3_SWARM_PR_ARCHAEOLOGY.md](research/Q3_SWARM_PR_ARCHAEOLOGY.md) — how late Q3 2025 becomes a PR-heavy Claude swarm rather than a mostly direct coding stream
- [research/Q3_SWARM_TALK_ARCHAEOLOGY.md](research/Q3_SWARM_TALK_ARCHAEOLOGY.md) — how the Q3 2025 swarm talk articulated trusted change, flows, receipts, and adversarial verification before the control plane fully hardened

### Control Plane and Process Archaeology

- [research/CONTROL_PLANE_ARCHAEOLOGY.md](research/CONTROL_PLANE_ARCHAEOLOGY.md) — tracked `.claude` and `.jules` lineage from Q3 swarm packs to the current control plane
- [research/CONTROL_PLANE_REPAIR_CHAIN_ARCHAEOLOGY.md](research/CONTROL_PLANE_REPAIR_CHAIN_ARCHAEOLOGY.md) — how swarm self-audit issues turn into direct repair PRs, maintainer-superseded follow-ups, or explicitly banked control-plane debt
- [research/AGENTS4_CANONICAL_Q3_ARCHAEOLOGY.md](research/AGENTS4_CANONICAL_Q3_ARCHAEOLOGY.md) — why `agents4` is the clearest perl-lsp-native preserved form of the canonical Q3 three-phase swarm
- [research/CONTROL_PLANE_SELF_REPAIR_ARCHAEOLOGY.md](research/CONTROL_PLANE_SELF_REPAIR_ARCHAEOLOGY.md) — how March 16-19, 2026 turns the swarm operating system itself into a maintained target through audits, friction logs, swarm-infra issues, and follow-up control-plane PRs
- [research/HYBRID_CONTROL_PLANE_ARCHAEOLOGY.md](research/HYBRID_CONTROL_PLANE_ARCHAEOLOGY.md) — how the March 2026 swarm split durable `swarm-state` from `.ops-perl-lsp` runtime without fully removing the older paths from live skills, hooks, commands, and donor agent packs
- [research/HOOK_CONTROL_ARCHAEOLOGY.md](research/HOOK_CONTROL_ARCHAEOLOGY.md) — how hooks evolved from early interception into the deterministic control boundary for the current swarm, while still exposing live gaps and reserved lifecycle ownership
- [research/HOOK_RELIABILITY_ARCHAEOLOGY.md](research/HOOK_RELIABILITY_ARCHAEOLOGY.md) — how hook payload handling, executable bits, ADR drift, and incomplete enforcement made hooks a reliability surface that had to be repaired, not just a breakthrough feature
- [research/INSTRUCTION_SURFACE_ARCHAEOLOGY.md](research/INSTRUCTION_SURFACE_ARCHAEOLOGY.md) — how orchestration guides, project doctrine, `.claude`, and `AGENTS.md` turned methodology into versioned operating instructions
- [research/ISSUE_LABEL_ARCHAEOLOGY.md](research/ISSUE_LABEL_ARCHAEOLOGY.md) — how label families and title prefixes gave the issue tracker a typed routing vocabulary for swarm discovery, self-improvement, and learning artifacts
- [research/ISSUE_ROUTING_ARCHAEOLOGY.md](research/ISSUE_ROUTING_ARCHAEOLOGY.md) — how GitHub issues became swarm overflow memory and a typed routing surface instead of just backlog storage
- [research/PUBLIC_VS_SWARM_INTAKE_ARCHAEOLOGY.md](research/PUBLIC_VS_SWARM_INTAKE_ARCHAEOLOGY.md) — how the public GitHub intake stays intentionally thin while the swarm-native control plane splits the same work into queue state, dedup, pitfalls, leads, and durable findings
- [research/SIGNAL_INTAKE_ARCHAEOLOGY.md](research/SIGNAL_INTAKE_ARCHAEOLOGY.md) — how PR templates, typed issue forms, and `swarm_discovered.yml` turned generic GitHub entry points into a handoff-ready signal stage for the swarm
- [research/ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md](research/ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md) — how issue bodies, PR bodies, learning issues, and article issues together made the GitHub ledger recoverable swarm memory
- [research/LEARNING_LOOP_ARCHAEOLOGY.md](research/LEARNING_LOOP_ARCHAEOLOGY.md) — how lessons, forensics, casebook exhibits, swarm-state, and GitHub crosslinks form one durable learning loop
- [research/JULES_LANE_ARCHAEOLOGY.md](research/JULES_LANE_ARCHAEOLOGY.md) — January 2026 Bolt/Sentinel/Palette lanes as proto-specialists
- [research/MAINTAINER_BRIDGE_ARCHAEOLOGY.md](research/MAINTAINER_BRIDGE_ARCHAEOLOGY.md) — how autumn 2025 large PRs acted as maintained bridge bundles before the January `maint/pr-*` naming made the pattern explicit
- [research/MERGECODE_ARCHAEOLOGY.md](research/MERGECODE_ARCHAEOLOGY.md) — how `agents2` and `agents3` turned GitHub-native receipts, single ledgers, and three explicit flows into a doctrine layer before the modern swarm control plane
- [research/MERGECODE_ROOTS_ARCHAEOLOGY.md](research/MERGECODE_ROOTS_ARCHAEOLOGY.md) — how `agents3` preserves a MergeCode-derived donor control plane later specialized into the canonical perl-lsp Q3 swarm in `agents4`
- [research/MERGE_DISCIPLINE_ARCHAEOLOGY.md](research/MERGE_DISCIPLINE_ARCHAEOLOGY.md) — PR governance from Q3 flow packs to `green-merge`, `review-pr`, and `triage-prs`
- [research/MAINTAINER_VISION_ARCHAEOLOGY.md](research/MAINTAINER_VISION_ARCHAEOLOGY.md) — repeated waves of encoding maintainer judgment into prompts, lanes, commands, skills, hooks, and state
- [research/MAINTAINER_PR_THREAD_ARCHAEOLOGY.md](research/MAINTAINER_PR_THREAD_ARCHAEOLOGY.md) — how maintainer judgment appears in GitHub PR threads as lane comments, supersede notes, contract review, and memory-backed verification rather than only as formal review approvals
- [research/OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md](research/OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md) — why the repo could already have review discipline, quality discipline, and specialization before those behaviors were sufficiently externalized into a lower-attention control plane
- [research/WORKTREE_PARALLELISM_ARCHAEOLOGY.md](research/WORKTREE_PARALLELISM_ARCHAEOLOGY.md) — how the repo moved from Q3 lane ideas and `maint/pr-*` bridges into deterministic `worktree-agent-*` execution
- [research/KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md](research/KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md) — how the current swarm compounds knowledge through layered `swarm-state`, operator commands, skills, and preserved scout logs rather than one generic memory file
- [research/KNOWLEDGE_PROMOTION_ARCHAEOLOGY.md](research/KNOWLEDGE_PROMOTION_ARCHAEOLOGY.md) — how session output is promoted from volatile execution into tracked ledgers, scout logs, operator summaries, archaeology notes, and source-linked article claims
- [research/SWARM_STATE_ARCHAEOLOGY.md](research/SWARM_STATE_ARCHAEOLOGY.md) — how `.claude/swarm-state/` became the committed memory ledger for the current swarm
- [research/SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md](research/SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md) — how committed swarm-state files and issue-title prefixes split memory into queue state, pitfalls, findings, learning, and article artifacts
- [research/SCOUT_LOG_ARCHAEOLOGY.md](research/SCOUT_LOG_ARCHAEOLOGY.md) — how tracked scout logs preserve dated session research as a memory tier between live swarm-state and polished archaeology
- [research/SWARM_SURFACE_EVOLUTION.md](research/SWARM_SURFACE_EVOLUTION.md) — Jan→Mar 2026 transition from commands to the current skills/hooks/swarm-state control plane

### Trust, Provenance, and AI-Native Operations

- [research/AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md](research/AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md) — how the repo moved from assisted coding toward an AI-native, receipt-driven operating model
- [research/MODE_SHIFT_ARCHAEOLOGY.md](research/MODE_SHIFT_ARCHAEOLOGY.md) — how the repo moved from assisted to native to industrialized work, including the nuance that Q4/Q1 was already AI-native but still hands-on
- [research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md](research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md) — how issue `#210` turned proof governance into gate harnesses, receipt schemas, status checks, and later audit prompts
- [research/POST_210_GOVERNANCE_ARCHAEOLOGY.md](research/POST_210_GOVERNANCE_ARCHAEOLOGY.md) — how issue `#210` propagated into `.ci` gate policy, receipt schemas, `xtask` runtime, status/update commands, and later audit culture while leaving visible recurring debt
- [research/CASEBOOK_FORENSICS_ARCHAEOLOGY.md](research/CASEBOOK_FORENSICS_ARCHAEOLOGY.md) — how casebook exhibits, PR dossiers, lessons, and specialist auditors became a reusable scar-story memory system
- [research/PROVENANCE_RECEIPTS_ARCHAEOLOGY.md](research/PROVENANCE_RECEIPTS_ARCHAEOLOGY.md) — how receipts, provenance schemas, and forensics turned proof into structured artifacts
- [research/RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md](research/RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md) — how PR-body receipt bundles, PR templates, issue `#210`, and typed gate receipts formed a layered proof surface instead of one flat “receipt” concept
- [research/RECEIPTS_LIE_ARCHAEOLOGY.md](research/RECEIPTS_LIE_ARCHAEOLOGY.md) — how PR `#209` and later validator repairs taught the repo that proof artifacts need governance too
- [research/TRUTH_SURFACE_ARCHAEOLOGY.md](research/TRUTH_SURFACE_ARCHAEOLOGY.md) — how the repo externalized anti-drift into source catalogs, computed evidence docs, typed receipts, lessons, and fail-closed checks
- [research/TRUSTED_CHANGE_ARCHAEOLOGY.md](research/TRUSTED_CHANGE_ARCHAEOLOGY.md) — how the repo industrialized trust through gates, receipts, drift checks, and durable lessons
- [research/VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md](research/VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md) — how the repo kept repairing helpers, gates, baselines, and assertions when the measurement surface itself proved incomplete

### CI, Queue, and Throughput Archaeology

- [research/CI_BUDGET_DISCIPLINE_ARCHAEOLOGY.md](research/CI_BUDGET_DISCIPLINE_ARCHAEOLOGY.md) — how CI spend, lane design, and local-first validation became an explicit engineering constraint
- [research/MAINTAINER_GATEKEEPER_ARCHAEOLOGY.md](research/MAINTAINER_GATEKEEPER_ARCHAEOLOGY.md) — how the human role shifted toward architectural direction, selection, merge pacing, and trusted-change oversight
- [research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md](research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md) — how the three-wide merge queue and CI throughput shaped swarm behavior and issue overflow

### GitHub PR Ledger Archaeology

- [research/PR_BRANCH_NAMING_ARCHAEOLOGY.md](research/PR_BRANCH_NAMING_ARCHAEOLOGY.md) — how head branches and PR titles reflect changing workflow eras
- [research/ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md](research/ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md) — how recurring issue families preserve discovery, bridge fixes, implementation PRs, and later learning/article artifacts as recoverable lineages
- [research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md](research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md) — how issues and PRs evolved into a shared delivery ledger for fixes, closures, learning reports, and article evidence
- [research/PR_LIFECYCLE_ARCHAEOLOGY.md](research/PR_LIFECYCLE_ARCHAEOLOGY.md) — how drafts, merges, closures, and disposal became part of the operating model
- [research/REVIEW_LABEL_ARCHAEOLOGY.md](research/REVIEW_LABEL_ARCHAEOLOGY.md) — how the canonical Q3 swarm encoded review stages, gates, lanes, and merge readiness directly in GitHub labels alongside the three-phase `issue-to-draft` / `draft-to-pr` / `pr-to-merge` flow
- [research/REVIEWER_ECOLOGY_ARCHAEOLOGY.md](research/REVIEWER_ECOLOGY_ARCHAEOLOGY.md) — how the repo layered human review, bot review, AI-reviewing-AI, and later gate/receipt enforcement instead of keeping review in one place
- [research/BOT_REVIEW_NOISE_ARCHAEOLOGY.md](research/BOT_REVIEW_NOISE_ARCHAEOLOGY.md) — how the PR archive accumulates large amounts of autogenerated review chatter while the actual decision signal usually lives in maintainer comments, labels, gates, and verification notes
- [research/REVIEWER_NETWORK_ARCHAEOLOGY.md](research/REVIEWER_NETWORK_ARCHAEOLOGY.md) — how reviewer identities themselves act as workflow-era signals, from human-led mixed review to machine-dense review lanes and later thinner gate-era threads
- [research/PR_REVIEW_RECEIPT_ARCHAEOLOGY.md](research/PR_REVIEW_RECEIPT_ARCHAEOLOGY.md) — how labels, receipts, check runs, comments, and cleanup follow-ups turned PRs into governance artifacts
- [research/PR_REVIEW_LOOP_ARCHAEOLOGY.md](research/PR_REVIEW_LOOP_ARCHAEOLOGY.md) — how cleanup passes, follow-up PRs, and review repair became explicit and normal
- [research/PR_SLICE_SIZE_ARCHAEOLOGY.md](research/PR_SLICE_SIZE_ARCHAEOLOGY.md) — how the PR archive balances many small bounded slices with a smaller number of deliberate umbrella changes
- [research/PR_WAVE_ARCHAEOLOGY.md](research/PR_WAVE_ARCHAEOLOGY.md) — how the repository moves in bursty PR waves rather than a smooth stream

### Session 3 Research (2026-03-20)

- [research/COMPETITIVE_LANDSCAPE.md](research/COMPETITIVE_LANDSCAPE.md) — Perl tooling market analysis: 78% greenfield, 3 incumbents
- [research/COST_ROI_ANALYSIS.md](research/COST_ROI_ANALYSIS.md) — session economics: DevLT 3-5 min/PR, $40-79K vs $500K-1.2M traditional
- [research/COST_ROI_EXECUTIVE_BRIEF.md](research/COST_ROI_EXECUTIVE_BRIEF.md) — executive summary of cost/ROI findings
- [research/FAILURE_STORIES.md](research/FAILURE_STORIES.md) — 10 documented development failures with cross-cutting patterns
- [research/VERIFIED_METRICS.md](research/VERIFIED_METRICS.md) — verified metrics with 4 corrections from audit
- [research/CORPUS_ROADMAP.md](research/CORPUS_ROADMAP.md) — bucket-by-bucket plan from 86.8% to 100% CPAN corpus coverage
- [research/COUNTER_INTUITIVE_INSIGHTS.md](research/COUNTER_INTUITIVE_INSIGHTS.md) — surprising findings that invert common assumptions
- [research/HINDSIGHT_FINDINGS.md](research/HINDSIGHT_FINDINGS.md) — things that are obvious in hindsight but were invisible at the time
- [research/CPAN_CORPUS_AUDIT.md](research/CPAN_CORPUS_AUDIT.md) — detailed CPAN corpus analysis and coverage audit
- [research/MICROCRATE_EVOLUTION.md](research/MICROCRATE_EVOLUTION.md) — 2 to 133 crates: emergent architecture from swarm development
- [research/TREE_SITTER_BREAKAGE.md](research/TREE_SITTER_BREAKAGE.md) — 7 tree-sitter breakage patterns and mode-based lexer insight
- [research/INTERVIEW_QUESTIONS.md](research/INTERVIEW_QUESTIONS.md) — 57 interview questions (35 original + 22 generated from session discoveries)
- [research/BUILDER_SPECS_PHASE_A.md](research/BUILDER_SPECS_PHASE_A.md) — builder-ready specifications from scout findings
- [research/SCOUT_CORPUS_TEST_STRATEGY.md](research/SCOUT_CORPUS_TEST_STRATEGY.md) — corpus testing strategy from scout analysis
- [research/ROADMAP_100_PERCENT_CPAN_COVERAGE.md](research/ROADMAP_100_PERCENT_CPAN_COVERAGE.md) — roadmap to 100% CPAN corpus coverage
- [research/REFERENCE_IMPLEMENTATION_FULL.md](research/REFERENCE_IMPLEMENTATION_FULL.md) — full reference implementation analysis
- [research/REPLICATION_GUIDES.md](research/REPLICATION_GUIDES.md) — methodology replication guide for other projects
- [research/SWARM_IMPROVEMENTS.md](research/SWARM_IMPROVEMENTS.md) — concrete swarm system improvements identified during session

### Research Maps and Source Drafts

- [research/ARTICLE_EVIDENCE_LINEAGE_ARCHAEOLOGY.md](research/ARTICLE_EVIDENCE_LINEAGE_ARCHAEOLOGY.md) — source map linking future launch-article claims to exact issue/PR/doc evidence chains
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
