# perl-lsp 0.12.0 Launch Plan

> Public alpha launch plan for the first broad-audience release of perl-lsp.

---

## 1. Release Mechanics

### Tag and Publish

- [ ] Tag `v0.12.0` on master after all exit criteria pass
- [ ] `cargo publish` the published crate set in topological order:
  `perl-token`, `perl-ast`, `perl-lexer`, `perl-parser`, `perl-corpus`, `perl-dap`, `perl-lsp-rs`, `perllsp`
- [ ] GitHub Release with release notes, CHANGELOG excerpt, and binary artifacts

### Binary Artifacts

- [ ] Linux x86_64 (musl static)
- [ ] macOS aarch64 (Apple Silicon)
- [ ] macOS x86_64
- [ ] Windows x86_64
- [ ] cargo-binstall metadata (PR #2209 merged)
- [ ] SBOM generation (SPDX and CycloneDX) and SLSA Level 2 provenance attestations

### Editor Distribution

- [ ] VSCode extension publish to marketplace (`effortlessmetrics.perl-lsp-rs`)
- [ ] VSCode extension auto-download verified for all three platforms
- [ ] Neovim lspconfig entry verified (`perl_ls` or `perl_lsp`)
- [ ] Emacs eglot configuration documented and tested

### Future Distribution (post-launch)

- [ ] Homebrew formula
- [ ] Nix package
- [ ] AUR package
- [ ] OpenVSX marketplace (open-source VSCode forks)

---

## 2. Launch Content (Week 1)

### Blog Post #1: "Introducing perl-lsp: A Rust-Native Perl Language Server"

The anchor announcement. README-level positioning aimed at Perl developers who want better tooling.

**Key messages:**
- No Perl runtime required -- a single native binary
- Sub-millisecond incremental parsing, under 50ms LSP responses
- Completion, diagnostics, hover, go-to-definition, references, rename, formatting, semantic highlighting, code actions, debugging
- Parses Perl 5.8 through 5.40 including heredocs, regex, quoting constructs, formats, and OO frameworks
- Built-in DAP debugger -- breakpoints, stepping, variables, watch expressions
- 120+ focused Rust crates, each with a single responsibility
- Zero `unsafe`, zero `unwrap`/`expect`, zero `panic!` in production code
- Competitive comparison: vs PerlNavigator, Perl::LanguageServer, PLS

**Source material:** README, `docs/articles/research/COMPETITIVE_LANDSCAPE.md`, `features.toml`

### Blog Post #2: "100 Agents, 56 PRs, 5 Days: Building perl-lsp with an AI Swarm"

The methodology story. Aimed at the AI/dev-tools audience.

**Key messages:**
- One human maintainer directing 100 AI agents in parallel
- 56 reviewed, tested, CI-gated PRs merged in a single 5-day cycle
- The attention bottleneck: code is cheap, trusted change is not
- Seven-flow SDLC: Signal, Plan, Build, Review, Gate, Deploy, Wisdom
- Worktree isolation enables safe parallelism across 120+ microcrates
- DevLT (Developer Lead Time) as the metric that matters
- Receipts, ratchets, and gates: trust by construction, not by reading every line

**Source material:** `docs/articles/SWARM_METHODOLOGY.md`, `docs/articles/FIVE_ERAS.md`

### README Update

- [ ] Add screenshot or GIF showing completion, hover, and diagnostics in action
- [ ] Verify all install paths work end-to-end on a fresh machine
- [ ] Update version references from 0.11.0 to 0.12.0

### CHANGELOG.md

- [ ] Finalize CHANGELOG.md for 0.12.0
- [ ] Include parser improvements, new LSP features, corpus coverage gains, security hardening

### Launch Day Announcements

- [ ] Hacker News: "Show HN: perl-lsp -- A Rust-native Perl language server"
- [ ] Reddit r/perl: announcement post
- [ ] Reddit r/rust: announcement post (Rust implementation angle)
- [ ] Lobsters: submission

---

## 3. Launch Content (Week 2--4)

### Blog Post #3: "Only Rust Can Parse Perl"

Technical deep dive into why Perl is one of the hardest mainstream languages to parse statically and how the parser handles each ambiguity.

**Key angles:**
- The 10 ambiguities: `/` (division vs regex), `{}` (hash vs block), `<<` (heredoc vs shift), `->` method calls, prototypes vs signatures, and more
- Why tree-sitter and PEG grammars hit walls on Perl
- The mode-state-machine approach in the lexer
- CPAN corpus as the ground truth: 80%+ of top-1000 distributions parsing clean
- "Only perl can parse Perl" -- Larry Wall's quote, and why a Rust parser gets close enough

**Source material:** `docs/articles/PARSING_PERL.md`, `docs/articles/research/perl_parsing_challenges_report.md`

### Blog Post #4: "Five Eras of AI Development"

The evolution narrative. How one project went through five distinct AI development methodologies in nine months.

**Key eras:**
1. Opus Direct (Jul--Aug 2025): single developer + single AI, 22.5 commits/day
2. Early Swarms (Aug--Oct 2025): first parallelism experiments, `codex/*` branches
3. Structured Swarms (Oct--Dec 2025): lanes, review gates, merge discipline
4. Hands-On Hardening (Dec 2025--Mar 2026): stability, quality, release preparation
5. Industrial Swarm (Mar 2026): 100 agents, 56 PRs/session, skills/hooks/worktrees

**Source material:** `docs/articles/FIVE_ERAS.md`, `docs/articles/research/ERA_TIMELINE.md`

### Blog Post #5: "No Panics Allowed: Reliability in a Language Server"

The reliability story. How perl-lsp enforces zero panics in production.

**Key points:**
- Seven banned constructs: `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `abort()`, `dbg!()`
- Enforced at compile time via workspace-level `deny` Clippy lints
- Why panics are worse than errors in a language server (silent failure)
- Defense-in-depth: supply chain security, SBOM, SLSA provenance
- Graceful degradation: partial parse results over crashes

**Source material:** `docs/articles/ZERO_PANIC.md`, `docs/reference/SUPPLY_CHAIN_SECURITY.md`

### Conference Talk (if applicable)

- [ ] Submit to The Perl and Raku Conference 2026
- [ ] Submit to RustConf 2026 (if timeline aligns)
- [ ] Prepare 30-minute talk: "Parsing the Unparseable: Building a Perl Language Server in Rust"

### Perl Community Outreach

- [ ] PerlMonks article/announcement
- [ ] Perl Weekly newsletter submission
- [ ] perl.org news submission
- [ ] r/perl ongoing engagement with user questions

---

## 4. Community Engagement

### GitHub Infrastructure

- [ ] Enable GitHub Discussions (issue #2169)
- [ ] CONTRIBUTING.md finalized (PR #2010 merged)
- [ ] Label 5--10 issues as "good first issue" for new contributors
- [ ] Create issue templates for bug reports and feature requests (if not already done)
- [ ] SUPPORT.md verified and linked from README

### Response Readiness

- [ ] Prepare response templates for common questions:
  - "How does this compare to PerlNavigator?"
  - "Does this work with Moose/Moo/Mouse?"
  - "Can I use this with Neovim/Emacs/Sublime?"
  - "How do I report parsing errors?"
  - "Why not just use `perl -c`?"
- [ ] Identify which questions route to existing docs vs need new content

### Community Signals

- [ ] CODE_OF_CONDUCT.md in place
- [ ] LICENSE files (MIT + Apache-2.0) clearly linked
- [ ] Security policy documented (`docs/reference/SUPPLY_CHAIN_SECURITY.md`)

---

## 5. Distribution Channels

### Perl Community (Primary Audience)

| Channel | Format | Timing |
|---------|--------|--------|
| **PerlMonks** | Article + discussion | Week 1 |
| **Perl Weekly** | Newsletter submission | Week 1 |
| **perl.org** | News item | Week 1 |
| **r/perl** | Reddit post | Day 1 |
| **Perl IRC / Discord** | Announcement | Day 1 |
| **CPAN** | `perl-corpus` crate (already published) | Pre-launch |

### Rust Community

| Channel | Format | Timing |
|---------|--------|--------|
| **r/rust** | Reddit post | Day 1 |
| **This Week in Rust** | Submission | Week 1 |
| **Rust Users Forum** | Announcement thread | Week 1 |
| **crates.io** | Published crate | Day 1 |

### General Dev / AI Community

| Channel | Format | Timing |
|---------|--------|--------|
| **Hacker News** | Show HN | Day 1 |
| **Lobsters** | Submission | Day 1 |
| **Dev.to** | Cross-post blog #1 | Week 1 |
| **X / Twitter** | Thread | Day 1 |

### IDE Communities

| Channel | Format | Timing |
|---------|--------|--------|
| **VSCode Marketplace** | Extension listing | Day 1 |
| **Neovim lspconfig** | PR to add/update entry | Pre-launch |
| **Emacs wiki / eglot docs** | Configuration example | Week 1 |

---

## 6. Success Metrics (Week 1)

### Quantitative

| Metric | Target (Week 1) | Stretch |
|--------|-----------------|---------|
| **cargo install downloads** | 100+ | 500+ |
| **VSCode extension installs** | 200+ | 1,000+ |
| **GitHub stars** | 50+ | 200+ |
| **GitHub issues opened** | 10+ (bug reports = users!) | 30+ |
| **Hacker News points** | Front page | Top 10 |

### Qualitative

- At least one "I switched from PerlNavigator" report
- At least one Neovim/Emacs user successfully configures and uses perl-lsp
- Community feedback identifies the top 3 real-world pain points to fix in 0.12.1
- No critical bugs (crashes, hangs, data loss) reported in the first week

### Tracking

- [ ] Set up GitHub star tracking
- [ ] Monitor crates.io download counts
- [ ] Watch VSCode marketplace install metrics
- [ ] Track GitHub issues by label (bug, enhancement, question)
- [ ] Save notable community mentions and quotes

---

## 7. Post-Launch Roadmap

### v0.12.1 -- Quick Response (1--2 weeks post-launch)

- Fix top user-reported parsing issues
- Address any crash or hang reports
- Improve error messages for common misparses
- Update CPAN corpus coverage based on user-reported real-world code

### v0.13.0 -- Coverage and Intelligence (1--2 months post-launch)

- CPAN corpus clean parse rate to 95%+
- Auto-import completions
- Snippet completions for common Perl patterns
- perlcritic bridge (if community requests it)
- Moo/Moose/Class::Accessor semantic coverage hardening
- Cross-file `use parent` / `use base` inheritance resolution

### v0.14.0 -- Performance and Scale

- Large workspace performance optimization
- Incremental indexing improvements
- Memory usage reduction for large codebases
- Workspace-level diagnostics

### v1.0.0 -- Production Ready

- Stability contract for APIs and advertised wire behavior
- Platform certification (Linux, macOS, Windows)
- Comprehensive semantic analysis for core Perl patterns
- Performance benchmarks published and maintained
- **When?** When the community tells us the tool is reliable enough for daily use on real projects. The gate is user trust, not a feature checklist.

---

## Pre-Launch Checklist

### Release Blockers

- [ ] All v0.12.0 exit criteria pass (see ROADMAP.md)
- [ ] `nix develop -c just ci-gate` green on tagged commit
- [ ] Version bumped to 0.12.0 in workspace Cargo.toml
- [ ] CHANGELOG.md finalized
- [ ] README.md updated with 0.12.0 content
- [ ] `perllsp --health` works on all three platforms
- [ ] VSCode extension tested on fresh install

### Content Blockers

- [ ] Blog post #1 drafted and reviewed
- [ ] Blog post #2 drafted and reviewed
- [ ] Screenshot/GIF created for README
- [ ] Announcement text prepared for each channel

### Infrastructure

- [ ] GitHub Release automation tested (binary artifact upload)
- [ ] `cargo publish --dry-run` succeeds for all published crates
- [ ] VSCode marketplace publish tested
- [ ] SBOM and provenance attestation pipeline verified
