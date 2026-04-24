# Article Index: perl-lsp Launch Content

Master index of all article source material produced during session 3 (2026-03-19/20) and prior sessions. For current parser and LSP metrics, see [../project/CURRENT_STATUS.md](../project/CURRENT_STATUS.md). For roadmap, see [../project/ROADMAP.md](../project/ROADMAP.md).

---

## 1. Published Articles

These documents are polished prose ready for editorial review or direct publication. They are listed in recommended publication order.

| File | Title | Audience | Word Est. |
|------|-------|----------|-----------|
| [FIVE_ERAS.md](FIVE_ERAS.md) | "The Five Eras of AI-Assisted Development" | General tech, journalists | 4,000-5,500 |
| [SWARM_METHODOLOGY.md](SWARM_METHODOLOGY.md) | "Agentic Swarm Development: A Methodology for Trusted Change at Scale" | Engineering managers, AI practitioners | 3,000-4,000 |
| [PARSING_PERL.md](PARSING_PERL.md) | "Parsing Perl: Why It's Hard, How We Do It Anyway" | Language tooling developers, compiler engineers | 4,000-5,000 |
| [ZERO_PANIC.md](ZERO_PANIC.md) | "Zero-Panic: Reliability and Security in a Language Server" | Systems programmers, Rust developers | 3,500-4,500 |
| [WHEN_RECEIPTS_LIE.md](WHEN_RECEIPTS_LIE.md) | "When Receipts Lie" — six real cases where structured evidence misled | AI/ML practitioners, engineering leaders | 2,500-3,500 |
| [CURIOSITIES.md](CURIOSITIES.md) | "perl-lsp: Curiosities, Records, and Surprising Facts" | Developer audience, Rust community | 2,000-3,000 |
| [REFERENCE_IMPLEMENTATION.md](REFERENCE_IMPLEMENTATION.md) | "perl-lsp as a Reference Implementation of Agentic Software Development" | AI researchers, engineering leaders | 3,500-4,500 |
| [METHODOLOGY_REPLICATION_GUIDE.md](METHODOLOGY_REPLICATION_GUIDE.md) | "Methodology Replication Guide" — practical steps for other teams | Teams adopting AI agents | 3,000-4,000 |

### Additional Polished Articles

| File | Title | Notes |
|------|-------|-------|
| [ARTICLE_OUTLINES.md](ARTICLE_OUTLINES.md) | "perl-lsp Launch Articles: Structured Outlines" | 8 publication-ready article outlines with section outlines, pull quotes, interview cross-refs |
| [SESSION_6_ECONOMICS.md](SESSION_6_ECONOMICS.md) | "Era 7 Session 6: 59 PRs, 200+ Agents, Research-First Pipeline" | Session economics, CI cascade fix, research-first ROI |
| [SESSION_7_ECONOMICS.md](SESSION_7_ECONOMICS.md) | "Era 7 Session 7: Multi-Pass Review as Infrastructure, Not Overhead" | Per-stage catch documentation, deleted test files, vacuous test pattern, haiku vs sonnet analysis |
| [AI_NATIVE_OPERATIONS.md](AI_NATIVE_OPERATIONS.md) | "AI-Native Operations: When the System Improves Itself" | Three modes: assisted, swarm, native |
| [ANATOMY_OF_A_SESSION.md](ANATOMY_OF_A_SESSION.md) | "Anatomy of a Session: What Happens When 60 AI Agents Build a Perl LSP for Seven Hours" | Session 3 case study, March 19-20, 2026 |
| [COMPETITIVE_ANALYSIS.md](COMPETITIVE_ANALYSIS.md) | "Competitive Analysis: Perl Language Servers in 2026" | Honest comparison vs PerlNavigator, Perl::LanguageServer, PLS |
| [COST_ROI.md](COST_ROI.md) | "Code Is Cheap; Trusted Change Is Not" | Session economics, DevLT model |
| [FEATURE_CATALOG.md](FEATURE_CATALOG.md) | "perl-lsp Feature Catalog" | 98 LSP/DAP features, structured tour of implementation |
| [FUTURE_OF_AGENT_TEAMS.md](FUTURE_OF_AGENT_TEAMS.md) | "Are Agent Teams the Future of Software Development?" | Evidence-backed forward-looking analysis |
| [KNOWLEDGE_COMPOUNDING.md](KNOWLEDGE_COMPOUNDING.md) | "Knowledge Compounding: How Institutional Memory Becomes a Flywheel" | The self-improving swarm thesis |
| [PARSER_WINS.md](PARSER_WINS.md) | "Perl Parsing Hall of Fame" | Hardest constructs handled: heredocs, slash ambiguity, fat arrows |
| [THREE_LAYER_PRODUCT.md](THREE_LAYER_PRODUCT.md) | "The Three-Layer Product" — LSP + swarm OS + memory/evidence | Why the repo is three products in one |

### Session 2026-04-24: Economic Maturity + Deep Review Catalog + Architecture Audit

Articles from the 2026-04-24 session (75 merged, 156 closed, 231 total resolved). Economic
analysis from ChatGPT synthesis; deep-review catalog with 17 verified findings; architecture
audit after Wave 4-Completion (31 published crates).

| File | Title | Notes |
|------|-------|-------|
| [ECONOMIC_MATURITY_THROUGHPUT_VS_TRUSTWORTHY.md](ECONOMIC_MATURITY_THROUGHPUT_VS_TRUSTWORTHY.md) | "Economic Maturity: From Throughput to Trustworthy Throughput" | 3-metric evolution, 4 forward metrics, verified cost numbers, 4 biggest cost sinks. |
| [DEEP_REVIEW_FIX_FORWARD_CATALOG_2026_04_24.md](DEEP_REVIEW_FIX_FORWARD_CATALOG_2026_04_24.md) | "Deep Review Fix-Forward Catalog: Session 2026-04-24" | 17 findings across 14 PRs; double-parse regression, coordinate-space mixing, p95 formula, vacuous assertions, schema mismatch. |
| [ARCHITECTURE_POST_COLLAPSE_AUDIT.md](ARCHITECTURE_POST_COLLAPSE_AUDIT.md) | "Architecture Post-Collapse Audit: State and Next Steps" | 135→31 collapse done; 3 seams needing surface tightening; parser-family tracker-vs-manifest; post-alpha roadmap. |
| [SESSION_2026_04_24_ECONOMICS.md](SESSION_2026_04_24_ECONOMICS.md) | "Session Economics: 2026-04-24" | Verified numbers (75 merged, 156 closed); master bit-rot cascade pattern; Windows short-name canonicalize fix. |

### Wave G1 Collapse Session (2026-04-19)

Articles from the Wave G1 collapse session — 5 PRs merged, 74 → 49 published crates. Each is self-contained; [SCOPE_PIVOT_ON_DEFER.md](SCOPE_PIVOT_ON_DEFER.md), [LLM_READS_SPEC_NOT_CODE.md](LLM_READS_SPEC_NOT_CODE.md), and [VERIFICATION_LADDER_PER_LAYER_ROI.md](VERIFICATION_LADDER_PER_LAYER_ROI.md) are the strongest standalone pieces.

| File | Title | Notes |
|------|-------|-------|
| [SCOPE_PIVOT_ON_DEFER.md](SCOPE_PIVOT_ON_DEFER.md) | "Scope-Pivot on DEFER" | Agent verdicts are hypotheses bound to scope; shrink scope and defer-rationale often evaporates. Two reversals = 30-40% productivity. |
| [LLM_READS_SPEC_NOT_CODE.md](LLM_READS_SPEC_NOT_CODE.md) | "Your LLM Reads the Spec, Not the Code" | Red-TDD failure mode with growth data (G1a=3 fixes → G1b=6 fixes). Fix: explicit API-read step in agent prompt. |
| [VERIFICATION_LADDER_PER_LAYER_ROI.md](VERIFICATION_LADDER_PER_LAYER_ROI.md) | "Verification Ladder ROI by Layer" | Concrete per-layer catch data: which agent caught what, cost per catch. 9 agents, ~16 unique bugs, 5 PRs. |
| [VERIFY_BY_READING.md](VERIFY_BY_READING.md) | "Verify By Reading" | Prior comments are hypotheses; tool success reports ≠ state change. Verify-by-reading as a hardening principle. |
| [AGGREGATOR_ABSORPTION_PATTERN.md](AGGREGATOR_ABSORPTION_PATTERN.md) | "The Aggregator Absorption Pattern" | Collapsing a 1,600-LOC aggregator crate into a module with deprecated alias preservation. |
| [WINDOWS_HARNESS_GAPS.md](WINDOWS_HARNESS_GAPS.md) | "Five Windows Harness Gaps in One Session" | Five distinct platform-specific bugs; systemic fix via migration to `xtask`. |
| [MEMORY_COMPOUNDS_WITHIN_SESSION.md](MEMORY_COMPOUNDS_WITHIN_SESSION.md) | "Memory Compounds Within a Session" | Memory writes as context-window extensions within a single long session. |
| [CODERABBIT_INVERSE_SAFETY.md](CODERABBIT_INVERSE_SAFETY.md) | "CodeRabbit Skips Big PRs" | Inverse safety pattern — automated review thins out when human review should thicken. |
| [PIPELINE_STATE_MACHINE.md](PIPELINE_STATE_MACHINE.md) | "Pipeline State Machine" | Label-driven state machine reference: sign-off labels, state labels, routing labels, invariants, transition diagrams for issue / build / PR pipelines. |

---

## 2. Research Documents

Supporting research and source material. All files are in [research/](research/).

### Era and Workflow Archaeology

| File | Description |
|------|-------------|
| [research/ERA_TIMELINE.md](research/ERA_TIMELINE.md) | Era-by-era timeline and velocity notes (~8,000 words, 5 eras) |
| [research/DEVELOPMENT_ARCHAEOLOGY.md](research/DEVELOPMENT_ARCHAEOLOGY.md) | Development-history archaeology: 15 sections, git history, microcrate explosion, CPAN corpus (~4,200 words) |
| [research/ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md](research/ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md) | The intentional late-2025 to early-2026 slowdown that built parser, quality gates, and architecture foundations |
| [research/ALPHA_READINESS_ARCHAEOLOGY.md](research/ALPHA_READINESS_ARCHAEOLOGY.md) | How March 2026 kept shipped release truth separate from v0.12.0 hardening plans and defined explicit alpha blockers |
| [research/COPILOT_FLEET_ARCHAEOLOGY.md](research/COPILOT_FLEET_ARCHAEOLOGY.md) | The February 27 to March 5, 2026 Copilot CLI firehose and its attribution boundary |
| [research/DIRECT_DELIVERY_ARCHAEOLOGY.md](research/DIRECT_DELIVERY_ARCHAEOLOGY.md) | How early history reads as direct delivery before mid-September 2025 turns PR-based development into the delivery model |
| [research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md](research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md) | March 11-19, 2026 as a mixed-tool period of Claude swarm bursts plus Codex waves |
| [research/INSTALL_SURFACE_ARCHAEOLOGY.md](research/INSTALL_SURFACE_ARCHAEOLOGY.md) | How install scripts, health/info flags, editor discovery order, and managed downloads became part of launch trust surface |
| [research/Q3_CONTROL_PLANE_ARCHAEOLOGY.md](research/Q3_CONTROL_PLANE_ARCHAEOLOGY.md) | How agents4 turns the canonical Q3 swarm into a phase-aware operating surface |
| [research/Q4_Q1_HANDS_ON_ARCHAEOLOGY.md](research/Q4_Q1_HANDS_ON_ARCHAEOLOGY.md) | The late-2025 to early-2026 stable, release-focused, but still maintainer-heavy bridge era |
| [research/Q3_SWARM_PR_ARCHAEOLOGY.md](research/Q3_SWARM_PR_ARCHAEOLOGY.md) | How late Q3 2025 becomes a PR-heavy Claude swarm rather than a mostly direct coding stream |
| [research/Q3_SWARM_TALK_ARCHAEOLOGY.md](research/Q3_SWARM_TALK_ARCHAEOLOGY.md) | How the Q3 2025 swarm talk articulated trusted change, flows, receipts, and adversarial verification |
| [research/five_eras_swarm_methodology.md](research/five_eras_swarm_methodology.md) | Source draft behind the five-eras analysis |
| [research/swarm_development_methodology.md](research/swarm_development_methodology.md) | Source draft behind the swarm methodology article |
| [research/perl_parsing_challenges_report.md](research/perl_parsing_challenges_report.md) | Source report behind the Parsing Perl article |

### Control Plane and Process Archaeology

| File | Description |
|------|-------------|
| [research/CONTROL_PLANE_ARCHAEOLOGY.md](research/CONTROL_PLANE_ARCHAEOLOGY.md) | Tracked .claude and .jules lineage from Q3 swarm packs to the current control plane |
| [research/CONTROL_PLANE_REPAIR_CHAIN_ARCHAEOLOGY.md](research/CONTROL_PLANE_REPAIR_CHAIN_ARCHAEOLOGY.md) | How swarm self-audit issues turn into direct repair PRs, maintainer-superseded follow-ups, or banked control-plane debt |
| [research/AGENTS4_CANONICAL_Q3_ARCHAEOLOGY.md](research/AGENTS4_CANONICAL_Q3_ARCHAEOLOGY.md) | Why agents4 is the clearest perl-lsp-native preserved form of the canonical Q3 three-phase swarm |
| [research/CONTROL_PLANE_SELF_REPAIR_ARCHAEOLOGY.md](research/CONTROL_PLANE_SELF_REPAIR_ARCHAEOLOGY.md) | How March 16-19, 2026 turns the swarm operating system itself into a maintained target |
| [research/HYBRID_CONTROL_PLANE_ARCHAEOLOGY.md](research/HYBRID_CONTROL_PLANE_ARCHAEOLOGY.md) | How March 2026 split durable swarm-state from .ops-perl-lsp runtime without removing older paths |
| [research/HOOK_CONTROL_ARCHAEOLOGY.md](research/HOOK_CONTROL_ARCHAEOLOGY.md) | How hooks evolved from early interception into the deterministic control boundary for the current swarm |
| [research/HOOK_RELIABILITY_ARCHAEOLOGY.md](research/HOOK_RELIABILITY_ARCHAEOLOGY.md) | How hook payload handling, executable bits, ADR drift, and incomplete enforcement made hooks a reliability surface requiring repair |
| [research/INSTRUCTION_SURFACE_ARCHAEOLOGY.md](research/INSTRUCTION_SURFACE_ARCHAEOLOGY.md) | How orchestration guides, project doctrine, .claude, and AGENTS.md turned methodology into versioned operating instructions |
| [research/ISSUE_LABEL_ARCHAEOLOGY.md](research/ISSUE_LABEL_ARCHAEOLOGY.md) | How label families and title prefixes gave the issue tracker a typed routing vocabulary |
| [research/ISSUE_ROUTING_ARCHAEOLOGY.md](research/ISSUE_ROUTING_ARCHAEOLOGY.md) | How GitHub issues became swarm overflow memory and a typed routing surface instead of just backlog storage |
| [research/PUBLIC_VS_SWARM_INTAKE_ARCHAEOLOGY.md](research/PUBLIC_VS_SWARM_INTAKE_ARCHAEOLOGY.md) | How the public GitHub intake stays thin while the swarm-native control plane splits work into queue state, dedup, pitfalls, and durable findings |
| [research/SIGNAL_INTAKE_ARCHAEOLOGY.md](research/SIGNAL_INTAKE_ARCHAEOLOGY.md) | How PR templates, typed issue forms, and swarm_discovered.yml turned GitHub entry points into a handoff-ready signal stage |
| [research/ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md](research/ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md) | How issue bodies, PR bodies, learning issues, and article issues together made the GitHub ledger recoverable swarm memory |
| [research/LEARNING_LOOP_ARCHAEOLOGY.md](research/LEARNING_LOOP_ARCHAEOLOGY.md) | How lessons, forensics, casebook exhibits, swarm-state, and GitHub crosslinks form one durable learning loop |
| [research/JULES_LANE_ARCHAEOLOGY.md](research/JULES_LANE_ARCHAEOLOGY.md) | January 2026 Bolt/Sentinel/Palette lanes as proto-specialists, before the current swarm model |
| [research/MAINTAINER_BRIDGE_ARCHAEOLOGY.md](research/MAINTAINER_BRIDGE_ARCHAEOLOGY.md) | How autumn 2025 large PRs acted as maintained bridge bundles before the maint/pr-* naming made the pattern explicit |
| [research/MERGECODE_ARCHAEOLOGY.md](research/MERGECODE_ARCHAEOLOGY.md) | How agents2 and agents3 turned GitHub-native receipts, single ledgers, and three explicit flows into doctrine before the modern control plane |
| [research/MERGECODE_ROOTS_ARCHAEOLOGY.md](research/MERGECODE_ROOTS_ARCHAEOLOGY.md) | How agents3 preserves a MergeCode-derived donor control plane later specialized into the canonical perl-lsp Q3 swarm in agents4 |
| [research/MERGE_DISCIPLINE_ARCHAEOLOGY.md](research/MERGE_DISCIPLINE_ARCHAEOLOGY.md) | PR governance from Q3 flow packs to green-merge, review-pr, and triage-prs |
| [research/MAINTAINER_VISION_ARCHAEOLOGY.md](research/MAINTAINER_VISION_ARCHAEOLOGY.md) | Repeated waves of encoding maintainer judgment into prompts, lanes, commands, skills, hooks, and state |
| [research/MAINTAINER_PR_THREAD_ARCHAEOLOGY.md](research/MAINTAINER_PR_THREAD_ARCHAEOLOGY.md) | How maintainer judgment appears in GitHub PR threads as lane comments, supersede notes, and memory-backed verification |
| [research/OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md](research/OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md) | Why the repo could have review, quality, and specialization discipline before those behaviors were sufficiently externalized |
| [research/WORKTREE_PARALLELISM_ARCHAEOLOGY.md](research/WORKTREE_PARALLELISM_ARCHAEOLOGY.md) | How the repo moved from Q3 lane ideas and maint/pr-* bridges into deterministic worktree-agent-* execution |
| [research/KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md](research/KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md) | How the current swarm compounds knowledge through layered swarm-state, operator commands, skills, and preserved scout logs |
| [research/KNOWLEDGE_PROMOTION_ARCHAEOLOGY.md](research/KNOWLEDGE_PROMOTION_ARCHAEOLOGY.md) | How session output is promoted from volatile execution into tracked ledgers, scout logs, operator summaries, and source-linked article claims |
| [research/SWARM_STATE_ARCHAEOLOGY.md](research/SWARM_STATE_ARCHAEOLOGY.md) | How .claude/swarm-state/ became the committed memory ledger for the current swarm |
| [research/SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md](research/SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md) | How committed swarm-state files and issue-title prefixes split memory into queue state, pitfalls, findings, learning, and article artifacts |
| [research/SCOUT_LOG_ARCHAEOLOGY.md](research/SCOUT_LOG_ARCHAEOLOGY.md) | How tracked scout logs preserve dated session research as a memory tier between live swarm-state and polished archaeology |
| [research/SWARM_SURFACE_EVOLUTION.md](research/SWARM_SURFACE_EVOLUTION.md) | Jan to Mar 2026 transition from commands to the current skills/hooks/swarm-state control plane |
| [research/SWARM_IMPROVEMENTS.md](research/SWARM_IMPROVEMENTS.md) | Concrete swarm system improvements identified during session 3 |

### Trust, Provenance, and AI-Native Operations

| File | Description |
|------|-------------|
| [research/AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md](research/AI_NATIVE_OPERATING_MODEL_ARCHAEOLOGY.md) | How the repo moved from assisted coding toward an AI-native, receipt-driven operating model |
| [research/MODE_SHIFT_ARCHAEOLOGY.md](research/MODE_SHIFT_ARCHAEOLOGY.md) | How the repo moved from assisted to native to industrialized work, including the Q4/Q1 nuance |
| [research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md](research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md) | How issue #210 turned proof governance into gate harnesses, receipt schemas, status checks, and later audit prompts |
| [research/POST_210_GOVERNANCE_ARCHAEOLOGY.md](research/POST_210_GOVERNANCE_ARCHAEOLOGY.md) | How issue #210 propagated into .ci gate policy, receipt schemas, xtask runtime, and audit culture |
| [research/CASEBOOK_FORENSICS_ARCHAEOLOGY.md](research/CASEBOOK_FORENSICS_ARCHAEOLOGY.md) | How casebook exhibits, PR dossiers, lessons, and specialist auditors became a reusable scar-story memory system |
| [research/PROVENANCE_RECEIPTS_ARCHAEOLOGY.md](research/PROVENANCE_RECEIPTS_ARCHAEOLOGY.md) | How receipts, provenance schemas, and forensics turned proof into structured artifacts |
| [research/RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md](research/RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md) | How PR-body receipt bundles, PR templates, issue #210, and typed gate receipts formed a layered proof surface |
| [research/RECEIPTS_LIE_ARCHAEOLOGY.md](research/RECEIPTS_LIE_ARCHAEOLOGY.md) | How PR #209 and later validator repairs taught the repo that proof artifacts need governance too |
| [research/TRUTH_SURFACE_ARCHAEOLOGY.md](research/TRUTH_SURFACE_ARCHAEOLOGY.md) | How the repo externalized anti-drift into source catalogs, computed evidence docs, typed receipts, lessons, and fail-closed checks |
| [research/TRUSTED_CHANGE_ARCHAEOLOGY.md](research/TRUSTED_CHANGE_ARCHAEOLOGY.md) | How the repo industrialized trust through gates, receipts, drift checks, and durable lessons |
| [research/VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md](research/VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md) | How the repo kept repairing helpers, gates, baselines, and assertions when the measurement surface itself proved incomplete |

### CI, Queue, and Throughput Archaeology

| File | Description |
|------|-------------|
| [research/CI_BUDGET_DISCIPLINE_ARCHAEOLOGY.md](research/CI_BUDGET_DISCIPLINE_ARCHAEOLOGY.md) | How CI spend, lane design, and local-first validation became an explicit engineering constraint |
| [research/MAINTAINER_GATEKEEPER_ARCHAEOLOGY.md](research/MAINTAINER_GATEKEEPER_ARCHAEOLOGY.md) | How the human role shifted toward architectural direction, selection, merge pacing, and trusted-change oversight |
| [research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md](research/QUEUE_BOTTLENECK_ARCHAEOLOGY.md) | How the three-wide merge queue and CI throughput shaped swarm behavior and issue overflow |

### GitHub PR Ledger Archaeology

| File | Description |
|------|-------------|
| [research/PR_BRANCH_NAMING_ARCHAEOLOGY.md](research/PR_BRANCH_NAMING_ARCHAEOLOGY.md) | How head branches and PR titles reflect changing workflow eras |
| [research/ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md](research/ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md) | How recurring issue families preserve discovery, bridge fixes, implementation PRs, and later learning/article artifacts as recoverable lineages |
| [research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md](research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md) | How issues and PRs evolved into a shared delivery ledger for fixes, closures, learning reports, and article evidence |
| [research/PR_LIFECYCLE_ARCHAEOLOGY.md](research/PR_LIFECYCLE_ARCHAEOLOGY.md) | How drafts, merges, closures, and disposal became part of the operating model |
| [research/REVIEW_LABEL_ARCHAEOLOGY.md](research/REVIEW_LABEL_ARCHAEOLOGY.md) | How the canonical Q3 swarm encoded review stages, gates, lanes, and merge readiness directly in GitHub labels |
| [research/REVIEWER_ECOLOGY_ARCHAEOLOGY.md](research/REVIEWER_ECOLOGY_ARCHAEOLOGY.md) | How the repo layered human review, bot review, AI-reviewing-AI, and later gate/receipt enforcement |
| [research/BOT_REVIEW_NOISE_ARCHAEOLOGY.md](research/BOT_REVIEW_NOISE_ARCHAEOLOGY.md) | How the PR archive accumulates autogenerated review chatter while actual decision signal lives in maintainer comments and gates |
| [research/REVIEWER_NETWORK_ARCHAEOLOGY.md](research/REVIEWER_NETWORK_ARCHAEOLOGY.md) | How reviewer identities act as workflow-era signals across the PR archive |
| [research/PR_REVIEW_RECEIPT_ARCHAEOLOGY.md](research/PR_REVIEW_RECEIPT_ARCHAEOLOGY.md) | How labels, receipts, check runs, comments, and cleanup follow-ups turned PRs into governance artifacts |
| [research/PR_REVIEW_LOOP_ARCHAEOLOGY.md](research/PR_REVIEW_LOOP_ARCHAEOLOGY.md) | How cleanup passes, follow-up PRs, and review repair became explicit and normal |
| [research/PR_SLICE_SIZE_ARCHAEOLOGY.md](research/PR_SLICE_SIZE_ARCHAEOLOGY.md) | How the PR archive balances many small bounded slices with a smaller number of deliberate umbrella changes |
| [research/PR_WAVE_ARCHAEOLOGY.md](research/PR_WAVE_ARCHAEOLOGY.md) | How the repository moves in bursty PR waves rather than a smooth stream |

### Session 3 Research (2026-03-19/20)

| File | Description |
|------|-------------|
| [research/COMPETITIVE_LANDSCAPE.md](research/COMPETITIVE_LANDSCAPE.md) | Perl tooling market analysis: 78% of developers use no language server; 3 incumbents characterized |
| [research/COST_ROI_ANALYSIS.md](research/COST_ROI_ANALYSIS.md) | Session economics: DevLT 3-5 min/PR, $40-79K vs $500K-1.2M traditional estimate |
| [research/COST_ROI_EXECUTIVE_BRIEF.md](research/COST_ROI_EXECUTIVE_BRIEF.md) | Executive summary of cost/ROI findings for non-technical audiences |
| [research/FAILURE_STORIES.md](research/FAILURE_STORIES.md) | 10 documented development failures with cross-cutting patterns and lessons |
| [research/VERIFIED_METRICS.md](research/VERIFIED_METRICS.md) | All key metrics verified against source, with discrepancies from common claims explained |
| [research/CORPUS_ROADMAP.md](research/CORPUS_ROADMAP.md) | Bucket-by-bucket plan from 86.8% to 100% CPAN corpus coverage |
| [research/COUNTER_INTUITIVE_INSIGHTS.md](research/COUNTER_INTUITIVE_INSIGHTS.md) | Surprising findings that invert common assumptions about AI-assisted development |
| [research/HINDSIGHT_FINDINGS.md](research/HINDSIGHT_FINDINGS.md) | Things that are obvious in hindsight but were invisible at build time |
| [research/CPAN_CORPUS_AUDIT.md](research/CPAN_CORPUS_AUDIT.md) | Detailed CPAN corpus analysis: 4,355 files, top error buckets, coverage by module category |
| [research/MICROCRATE_EVOLUTION.md](research/MICROCRATE_EVOLUTION.md) | How the codebase grew from 2 to 134 crates: emergent architecture from swarm development |
| [research/TREE_SITTER_BREAKAGE.md](research/TREE_SITTER_BREAKAGE.md) | 7 tree-sitter breakage patterns and the mode-based lexer insight that drove v3 |
| [research/INTERVIEW_QUESTIONS.md](research/INTERVIEW_QUESTIONS.md) | 57 interview questions (35 original + 22 generated from session discoveries) |
| [research/BUILDER_SPECS_PHASE_A.md](research/BUILDER_SPECS_PHASE_A.md) | Builder-ready specifications from session 3 scout findings |
| [research/SCOUT_CORPUS_TEST_STRATEGY.md](research/SCOUT_CORPUS_TEST_STRATEGY.md) | Corpus testing strategy from scout analysis |
| [research/ROADMAP_100_PERCENT_CPAN_COVERAGE.md](research/ROADMAP_100_PERCENT_CPAN_COVERAGE.md) | Roadmap to 100% CPAN corpus coverage with prioritized error buckets |
| [research/REFERENCE_IMPLEMENTATION_FULL.md](research/REFERENCE_IMPLEMENTATION_FULL.md) | Full reference implementation analysis (extended version of REFERENCE_IMPLEMENTATION.md) |
| [research/REPLICATION_GUIDES.md](research/REPLICATION_GUIDES.md) | Methodology replication guide for other projects (source material for METHODOLOGY_REPLICATION_GUIDE.md) |
| [research/ASYNC_LSP_AUDIT.md](research/ASYNC_LSP_AUDIT.md) | Async/concurrency audit: parser cancellation, diagnostic debounce, subprocess timeouts |
| [research/EDGE_CASE_AUDIT.md](research/EDGE_CASE_AUDIT.md) | Parser edge case audit findings |
| [research/TESTING_INFRASTRUCTURE_GAPS_SCOUT.md](research/TESTING_INFRASTRUCTURE_GAPS_SCOUT.md) | Testing-gap research that fed related follow-up work |
| [research/DEEP_CONTEXT_LEARNINGS.md](research/DEEP_CONTEXT_LEARNINGS.md) | Learnings from deep codebase archaeology that inform future agent sessions |
| [research/CUSTOM_LSP_RUNTIME_ANALYSIS.md](research/CUSTOM_LSP_RUNTIME_ANALYSIS.md) | Deep analysis of the custom LSP runtime architecture, trade-offs, and comparison with alternatives (ADR-0034 context) |

### Research Maps and Source Drafts

| File | Description |
|------|-------------|
| [research/ARTICLE_EVIDENCE_LINEAGE_ARCHAEOLOGY.md](research/ARTICLE_EVIDENCE_LINEAGE_ARCHAEOLOGY.md) | Source map linking future article claims to exact issue/PR/doc evidence chains; verified 2026-03-19 |
| [research/BLOG_MATERIAL_INDEX.md](research/BLOG_MATERIAL_INDEX.md) | Scout-generated map of article angles, evidence, and recommended article outlines (5 articles) |
| [research/DOCUMENTATION_SUMMARY.md](research/DOCUMENTATION_SUMMARY.md) | Packaging summary for the full article set |
| [research/SCOUT_SUMMARY.md](research/SCOUT_SUMMARY.md) | Summary of scout output delivered during session 3 |

---

## 3. Interview Material

| File | Description |
|------|-------------|
| [INTERVIEW_QA.md](INTERVIEW_QA.md) | Full Q&A in Steven's voice — lightly edited, covers origin, Perl choice, 130 crates, accounting mindset, best LSP vision, and what to change |
| [NEW_INTERVIEW_QUESTIONS.md](NEW_INTERVIEW_QUESTIONS.md) | 22 questions generated from session 3 discoveries, with evidence from codebase and follow-up prompts |
| [research/INTERVIEW_QUESTIONS.md](research/INTERVIEW_QUESTIONS.md) | 57 interview questions total (35 original + 22 new); each includes the question, why it unlocks an interesting story, and codebase evidence |

### Key Interview Question Groups

**Origin and motivation** — INTERVIEW_QA.md Q1-3 (why Perl, why LSP, how it started)

**Technical decisions** — INTERVIEW_QA.md + INTERVIEW_QUESTIONS.md Q2 (aha moment), Q9 (130 crates), Q16 (Larry Wall quote), Q17 (weirdest syntax)

**Methodology** — INTERVIEW_QUESTIONS.md Q7 (DevLT), Q8 (90% vs 50%), Q19 (scout-constrain-build), Q38 (agent ratios)

**Failures and lessons** — NEW_INTERVIEW_QUESTIONS.md Q36 (assert_clean_parse bug), Q37 (52 orphaned worktrees), INTERVIEW_QUESTIONS.md Q14 (when receipts lie), Q15 (hardest lesson)

**Personal story** — INTERVIEW_QA.md Q21 (CPA background), Q22 (accounting influence on controls), Q28 (swarm as product)

---

## 4. Article Hooks

Eight publication-ready article titles with outline and source cross-references. Full outlines in [ARTICLE_OUTLINES.md](ARTICLE_OUTLINES.md).

| Title | Target Audience | Primary Sources | Recommended Position |
|-------|----------------|----------------|---------------------|
| "100 Agents, 56 PRs, 5 Days" | Engineering leaders, CTOs | FIVE_ERAS.md (Eras 4-5), SWARM_METHODOLOGY.md, CURIOSITIES.md | 2nd — scale story |
| "Only Rust Can Parse Perl" | Language tooling devs, PL enthusiasts | PARSING_PERL.md, FIVE_ERAS.md, CURIOSITIES.md | 4th — technical depth |
| "Code Is Cheap; Trusted Change Is Not" | AI practitioners, engineering managers | SWARM_METHODOLOGY.md, FIVE_ERAS.md, ZERO_PANIC.md | 3rd — methodology |
| "Five Eras of AI Development" | General tech, journalists | FIVE_ERAS.md (primary), SWARM_METHODOLOGY.md, CURIOSITIES.md | 1st — narrative arc |
| "No Panics Allowed" | Systems programmers, Rust devs | ZERO_PANIC.md (primary), CURIOSITIES.md, SWARM_METHODOLOGY.md | 6th — Rust community |
| "The Self-Improving Swarm" | AI researchers, agent framework devs | SWARM_METHODOLOGY.md, FIVE_ERAS.md, CURIOSITIES.md | 8th — forward-looking |
| "130 Crates, Zero Conflicts" | Rust community, architects | FIVE_ERAS.md (Era 3-4), CURIOSITIES.md, SWARM_METHODOLOGY.md | 7th — architecture |
| "From CPA to LSP" | Non-traditional devs, general tech | FIVE_ERAS.md, SWARM_METHODOLOGY.md, ZERO_PANIC.md, CURIOSITIES.md | 5th — human story |

### Pairing recommendations

- Articles 1 + 3 (scale + methodology) for engineering leadership publications
- Articles 2 + 7 (parsing + architecture) for Rust/PL community
- Articles 4 + 5 (eras + personal story) for general tech publications
- Articles 6 + 8 (self-improvement + reliability) for AI research venues

### Additional article angles (from research)

These angles have supporting research material but no finalized outline yet:

- "Branch Naming as Signal" — codex/* vs worktree-agent-HASH as workflow fingerprints (source: PR_BRANCH_NAMING_ARCHAEOLOGY.md, ERA_TIMELINE.md)
- "Why Slower Is Faster" — Era 3 rationale for intentional deceleration (source: ARCHITECTURAL_SIDECHAIN_ARCHAEOLOGY.md, ERA_TIMELINE.md)
- "Copilot vs Claude Agents: Bot vs Agent Trade-offs" (source: ERA5_MIXED_TOOL_ARCHAEOLOGY.md, COPILOT_FLEET_ARCHAEOLOGY.md)
- "From 4,355 CPAN Modules to Parse Errors: Corpus-Driven Development" (source: CPAN_CORPUS_AUDIT.md, CORPUS_ROADMAP.md, BLOG_MATERIAL_INDEX.md)
- "When Tests Lie: The assert_clean_parse Incident" (source: NEW_INTERVIEW_QUESTIONS.md Q36, WHEN_RECEIPTS_LIE.md Case 1)

---

## 5. Verification Status

**Automated check**: Run `just verify-publication-facts` to verify all computable metrics against the ledger. See [../project/PUBLICATION_FACTS_LEDGER.md](../project/PUBLICATION_FACTS_LEDGER.md) for the tier system (A=computed, B=measured, C=estimated, D=external).

### Verified claims (current as of 2026-03-22)

Verified by `just verify-publication-facts` against live repo data. Historical values from 2026-03-19 in parentheses where they differ.

| Claim | Current Value | 2026-03-19 Value | Tier | Command |
|-------|--------------|-----------------|------|---------|
| Lines of Rust | ~597,863 | 563,883 | A | `find crates/ -name "*.rs" -print0 \| xargs -0 cat \| wc -l` |
| Workspace crates | 134 | 133 | A | `cargo metadata --no-deps \| jq '.packages \| length'` |
| Total commits | 3,307 | 2,768 | A | `git log --oneline \| wc -l` |
| LSP features | 98 | 97 | A | `grep -c '^\[\[feature\]\]' features.toml` |
| CPAN corpus files | 4,355 | 4,355 | A | `jq .total_files .ci/cpan-corpus-baseline.json` |
| CPAN baseline clean rate | 85.4% (3,717/4,355) | 80.0% (3,484/4,355) | A | `jq .clean_files .ci/cpan-corpus-baseline.json` |
| Corpus manifest coverage | 47.1% (2,052/4,355) | — | A | `wc -l .ci/cpan-corpus-manifest.txt` |
| Constrained task success rate | ~90% | same | N | Memory files |
| Unconstrained task success rate | ~50% | same | N | Memory files |
| Peak commit day | 308 (2026-03-20) | 152 (Era 4) | A | `git log --format="%ad" --date=format:"%Y-%m-%d" \| sort \| uniq -c \| sort -rn \| head -1` |
| CI gate time | 3-5 min (B tier) | same | B | `just ci-gate` |
| 3-wide merge queue | 3 concurrent merges max | same | N | CI cancellation cascade |

**LOC method note**: Use `find crates/ -name "*.rs" -print0 | xargs -0 cat | wc -l` — the `xargs wc -l | tail -1` form produces incorrect results when xargs splits into multiple batches.

### Known remaining discrepancies in research/draft documents

These values appear in research documents (not primary articles) and reflect snapshots from their writing date. They do not need to be updated retroactively — they document the state at that time.

| Document | Value | Context |
|----------|-------|---------|
| BLOG_MATERIAL_INDEX.md | 425 commits in 24 hours | All-ref artifacts on 2026-03-18; merged-to-master peak was 308 on 2026-03-20 |
| BLOG_MATERIAL_INDEX.md | 546,283 lines | Snapshot from early March |
| COST_ROI_ANALYSIS.md | 480,934 LOC | Snapshot from session 3 (earlier) |

### Claims pending verification (Tier C/D — non-automatable)

| Claim | Source | Tier | Status |
|-------|--------|------|--------|
| DevLT 3-5 minutes per PR | COST_ROI_ANALYSIS.md Section 5 | C | Model estimate; not measured from CI receipts. Methodology documented. |
| $40-79K vs $500K-1.2M cost comparison | COST_ROI_ANALYSIS.md Section 9 | C | Model estimate; confidence intervals in Section 9. |
| 78% of Perl developers use no language server | COMPETITIVE_ANALYSIS.md | D | Attributed to 2025 Perl IDE Survey (602 respondents); primary source not linked. |
| PerlNavigator ~53,000 VSCode installs | COMPETITIVE_ANALYSIS.md | D | Point-in-time (early 2026); marketplace count changes. Article now date-stamped. |
| Perl::LanguageServer ~293,000 VSCode installs | COMPETITIVE_ANALYSIS.md | D | Point-in-time (early 2026); marketplace count changes. Article now date-stamped. |
| 90% success rate on constrained tasks | Multiple articles | N | Well-documented in memory files; specific session data not cited. |
| 4:1:2 scout:builder:reviewer ratio | NEW_INTERVIEW_QUESTIONS.md Q38 | N | Described as converged-on; not derived from a single measurement. |

### Verification guidance

- **Automated**: `just verify-publication-facts` — checks all Tier A metrics in under 60 seconds
- **Strict mode (CI)**: `just ci-publication-facts` — exits 1 if any metric drifts >10%
- For architecture claims: `cargo metadata --no-deps` and `git log`
- For corpus claims: `just cpan-corpus-check` and `.ci/cpan-corpus-baseline.json`
- For LSP feature claims: `features.toml` and `scripts/update-current-status.py`
- For session/agent claims: cross-reference `.claude/swarm-state/` committed memory
- For CI timing: `nix develop -c just ci-gate`

---

## Related Project Docs

- [../project/CURRENT_STATUS.md](../project/CURRENT_STATUS.md) — authoritative current metrics (auto-generated, never hand-edited)
- [../project/ROADMAP.md](../project/ROADMAP.md) — current development roadmap
- [../project/CODEBASE_HISTORY.md](../project/CODEBASE_HISTORY.md) — longer-form repository history
- [../project/AGENTIC_DEVELOPMENT.md](../project/AGENTIC_DEVELOPMENT.md) — earlier case-study framing for AI-assisted development
- [../project/AGENTIC_SWARM_ERA.md](../project/AGENTIC_SWARM_ERA.md) — earlier write-up focused on the swarm era
- [../project/CODEBASE_CURIOSITIES.md](../project/CODEBASE_CURIOSITIES.md) — current-tree curiosity tour
- [../project/JULES_BOT_ANALYSIS.md](../project/JULES_BOT_ANALYSIS.md) — earlier analysis of the January 2026 draft-PR bridge
- [../project/PARSING_PERL.md](../project/PARSING_PERL.md) — existing parser deep dive in the project-docs track
- [../project/QUALITY_INFRASTRUCTURE.md](../project/QUALITY_INFRASTRUCTURE.md) — broader quality and security infrastructure documentation

---

*This index was generated on 2026-03-21 by reading all files in `docs/articles/` and `docs/articles/research/`. It covers 21 files in `docs/articles/` (excluding this index and README.md) and 97 research files. For additions, update this file and cross-reference the new material in the appropriate section.*
