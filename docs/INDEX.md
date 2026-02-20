> For the documentation hub, see [README.md](README.md). This page is a legacy index.

# Documentation Index

This is the front door. If you want X, read Y.

## Documentation Site

**📖 [Browse the documentation site](https://effortlessmetrics.github.io/tree-sitter-perl/)** - Searchable, organized documentation with navigation

For local preview: `just docs-serve`

See [DOCUMENTATION_SITE.md](DOCUMENTATION_SITE.md) for setup and deployment details.

## Canonical Truth Sources

| What               | Where                                      | Verified By        |
| ------------------ | ------------------------------------------ | ------------------ |
| Metrics            | [`CURRENT_STATUS.md`](CURRENT_STATUS.md)   | `just status-check` |
| Plans              | [`ROADMAP.md`](ROADMAP.md)                 | Human review       |
| Milestones         | [`MILESTONES.md`](MILESTONES.md)           | GitHub Milestones  |
| Capability catalog | [`features.toml`](../features.toml)        | `just ci-gate`     |
| CI lanes           | [`CI_TEST_LANES.md`](CI_TEST_LANES.md)     | `just ci-gate`     |
| Local validation   | [`CI_LOCAL_VALIDATION.md`](CI_LOCAL_VALIDATION.md) | `just ci-gate` |
| CI cost tracking   | [`CI_COST_TRACKING.md`](CI_COST_TRACKING.md) | Manual review      |
| What went wrong    | [`LESSONS.md`](LESSONS.md)                 | Human review       |
| What went right    | [`CASEBOOK.md`](CASEBOOK.md)               | Human review       |
| PR archaeology     | [`forensics/INDEX.md`](forensics/INDEX.md) | Human review       |

**Rule**: All metrics are computed and live in `CURRENT_STATUS.md`. If you see a number elsewhere, treat it as stale.

## Verification Commands

```bash
# Canonical local gate (required before push)
nix develop -c just ci-gate

# Verify computed metrics haven't drifted
just status-check

# Show ignored test breakdown
bash scripts/ignored-test-count.sh
```

## By Task

### Use the LSP server

- [Getting Started](GETTING_STARTED.md) - Beginner-friendly setup guide
- [Quick Start](../README.md#quick-start)
- [Editor Setup](EDITOR_SETUP.md)
- [FAQ](FAQ.md) - Frequently asked questions

### Understand the architecture

- [Architecture Overview](ARCHITECTURE_OVERVIEW.md)
- [Crate Architecture Guide](CRATE_ARCHITECTURE_GUIDE.md)
- [LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md)

### Contribute code

- [Contributing Guidelines](../CONTRIBUTING.md)
- [Commands Reference](COMMANDS_REFERENCE.md)
- [Test Infrastructure Guide](TEST_INFRASTRUCTURE_GUIDE.md)

### Debug issues

- [FAQ](FAQ.md) - Common questions answered
- [Troubleshooting](TROUBLESHOOTING.md) - Common issues and solutions
- [Known Limitations](KNOWN_LIMITATIONS.md)
- [Known Flaky Tests](KNOWN_FLAKY_TESTS.md)
- [Debugging](DEBUGGING.md)

### Check project status

- [Current Status](CURRENT_STATUS.md) (computed metrics)
- [Roadmap](ROADMAP.md) (plans and milestones)

## Document Categories

### v0.9.x (Production-Ready) Core Documentation

- **[docs/README.md](README.md)** - **v0.9.x (Production-Ready) Documentation Index** - Complete documentation hub
- `GETTING_STARTED.md` - **New users start here** - Installation and first steps
- `FAQ.md` - Frequently asked questions
- `CURRENT_STATUS.md` - Computed project metrics for v0.9.x (Production-Ready)
- `ROADMAP.md` - Release plans and exit criteria
- `RELEASE_NOTES.md` - v0.9.x (Production-Ready) release details and changelog
- `../SECURITY.md` - v0.9.x (Production-Ready) security policy and procedures

### Operator Docs (start here)

- `GETTING_STARTED.md` - **New users start here** - Installation and first steps
- `FAQ.md` - Frequently asked questions
- `CURRENT_STATUS.md` - Computed project metrics
- `ROADMAP.md` - Release plans and exit criteria
- `MILESTONES.md` - GitHub milestone tracking and blockers
- `COMMANDS_REFERENCE.md` - Build, test, and lint commands
- `CI_README.md` - **CI documentation index** (start here for CI docs)
- `CI_LOCAL_VALIDATION.md` - Local-first CI validation workflow
- `CI_TEST_LANES.md` - CI lane definitions
- `CI_COST_TRACKING.md` - CI budget management and cost optimization
- `CI.md` - GitHub Actions architecture

### Architecture and Design

- `ARCHITECTURE_OVERVIEW.md` - High-level system design
- `LSP_IMPLEMENTATION_GUIDE.md` - LSP server internals
- `CRATE_ARCHITECTURE_GUIDE.md` - Workspace structure
- `adr/` - Architecture Decision Records

### Feature Docs

- `LSP_FEATURES.md` - LSP capability details
- `DAP_USER_GUIDE.md` - Debug adapter usage
- `WORKSPACE_NAVIGATION_GUIDE.md` - Cross-file features

### Process Docs (v0.9.x (Production-Ready))

- `AGENTIC_DEV.md` - Development model and budget definitions
- `LESSONS.md` - What went wrong and what changed
- `CASEBOOK.md` - Exhibit PRs demonstrating the model
- `forensics/INDEX.md` - PR archaeology inventory
- `FORENSICS_SCHEMA.md` - PR archaeology dossier template
- `STABILITY.md` - API stability policy
- `STABILITY_STATEMENT_v0.9.x (Production-Ready).md` - v0.9.x (Production-Ready) stability guarantees
- `CONTRIBUTING_LSP.md` - LSP contribution guidelines
- `../CONTRIBUTING.md` - Development workflow and release process

### Historical (archived)

- `archive/` - Old roadmaps, superseded docs
- `reports/` - Point-in-time analysis reports

## Truth Contract (v0.9.x (Production-Ready))

1. Metrics come from computation, not hand-editing
2. `just status-check` fails if docs drift from computed values
3. Claims require receipts (test output, gate output, or targeted tests)
4. No adjectives without evidence (no "revolutionary", "enterprise-grade" without proof)
5. **v0.9.x (Production-Ready) Production Claims**: All performance and security claims are validated with comprehensive testing

See [`LESSONS.md`](LESSONS.md) for what happens when we violate these rules.

---

**Note**: This is the legacy documentation index. For the most current v0.9.x (Production-Ready) documentation, please see **[docs/README.md](README.md)**.
