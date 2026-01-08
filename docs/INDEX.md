# Documentation Index

This is the front door. If you want X, read Y.

## Canonical Truth Sources

| What               | Where                                      | Verified By        |
| ------------------ | ------------------------------------------ | ------------------ |
| Metrics            | [`CURRENT_STATUS.md`](CURRENT_STATUS.md)   | `just status-check` |
| Plans              | [`ROADMAP.md`](ROADMAP.md)                 | Human review       |
| Milestones         | [`MILESTONES.md`](MILESTONES.md)           | GitHub Milestones  |
| Capability catalog | [`features.toml`](../features.toml)        | `just ci-gate`     |
| CI lanes           | [`CI_TEST_LANES.md`](CI_TEST_LANES.md)     | `just ci-gate`     |
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

- [Quick Start](../README.md#quick-start)
- [Editor Setup](EDITOR_SETUP.md)

### Understand the architecture

- [Architecture Overview](ARCHITECTURE_OVERVIEW.md)
- [Crate Architecture Guide](CRATE_ARCHITECTURE_GUIDE.md)
- [LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md)

### Contribute code

- [Contributing Guidelines](../CONTRIBUTING.md)
- [Commands Reference](COMMANDS_REFERENCE.md)
- [Test Infrastructure Guide](TEST_INFRASTRUCTURE_GUIDE.md)

### Debug issues

- [Known Limitations](KNOWN_LIMITATIONS.md)
- [Known Flaky Tests](KNOWN_FLAKY_TESTS.md)
- [Debugging](DEBUGGING.md)

### Check project status

- [Current Status](CURRENT_STATUS.md) (computed metrics)
- [Roadmap](ROADMAP.md) (plans and milestones)

## Document Categories

### Operator Docs (start here)

- `CURRENT_STATUS.md` - Computed project metrics
- `ROADMAP.md` - Release plans and exit criteria
- `MILESTONES.md` - GitHub milestone tracking and blockers
- `COMMANDS_REFERENCE.md` - Build, test, and lint commands
- `CI_TEST_LANES.md` - CI lane definitions

### Architecture and Design

- `ARCHITECTURE_OVERVIEW.md` - High-level system design
- `LSP_IMPLEMENTATION_GUIDE.md` - LSP server internals
- `CRATE_ARCHITECTURE_GUIDE.md` - Workspace structure
- `adr/` - Architecture Decision Records

### Feature Docs

- `LSP_FEATURES.md` - LSP capability details
- `DAP_USER_GUIDE.md` - Debug adapter usage
- `WORKSPACE_NAVIGATION_GUIDE.md` - Cross-file features

### Process Docs

- `AGENTIC_DEV.md` - Development model and budget definitions
- `LESSONS.md` - What went wrong and what changed
- `CASEBOOK.md` - Exhibit PRs demonstrating the model
- `forensics/INDEX.md` - PR archaeology inventory
- `FORENSICS_SCHEMA.md` - PR archaeology dossier template
- `STABILITY.md` - API stability policy
- `CONTRIBUTING_LSP.md` - LSP contribution guidelines

### Historical (archived)

- `archive/` - Old roadmaps, superseded docs
- `reports/` - Point-in-time analysis reports

## Truth Contract

1. Metrics come from computation, not hand-editing
2. `just status-check` fails if docs drift from computed values
3. Claims require receipts (test output, gate output, or targeted tests)
4. No adjectives without evidence (no "revolutionary", "enterprise-grade" without proof)

See [`LESSONS.md`](LESSONS.md) for what happens when we violate these rules.
