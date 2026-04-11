# Documentation Index

This page is the documentation front door for the `perl-lsp` workspace.

## Start Here

Choose the path that matches what you are trying to do:

| I want to... | Read this first |
|---|---|
| Install the language server | [Installation Guide](how-to/INSTALLATION.md) |
| Integrate perl-lsp into GitHub Actions | [GitHub Actions Integration](how-to/GITHUB_ACTIONS.md) |
| Upgrade an existing installation | [Upgrading](how-to/UPGRADING.md) |
| Get a working editor setup quickly | [Getting Started](tutorials/GETTING_STARTED.md) |
| Set up continuous testing and watch loops | [Continuous Testing](how-to/CONTINUOUS_TESTING.md) |
| Configure editor or workspace settings | [Configuration Reference](reference/CONFIG.md) |
| Share project settings with my team | [Project Configuration File (.perl-lsp.toml)](reference/CONFIG.md#project-configuration-file-perl-lsptoml) |
| Troubleshoot startup, indexing, or editor issues | [Troubleshooting](how-to/TROUBLESHOOTING.md) |
| Understand the server architecture | [Architecture Overview](reference/ARCHITECTURE_OVERVIEW.md) |
| Work on LSP features as a contributor | [LSP Development Guide](tutorials/LSP_DEVELOPMENT_GUIDE.md) |
| Run builds, tests, and CI commands | [Commands Reference](reference/COMMANDS_REFERENCE.md) |
| Add or audit public API documentation | [Missing Documentation Guide](reference/MISSING_DOCUMENTATION_GUIDE.md) |
| Understand stability and compatibility | [Stability Policy](reference/STABILITY.md) |
| Read the historical analyses and launch material | [Articles and Research Notes](articles/README.md) |

## Documentation Map

### Tutorials
Hands-on guides for learning the system by doing.

- [Getting Started](tutorials/GETTING_STARTED.md)
- [LSP Development Guide](tutorials/LSP_DEVELOPMENT_GUIDE.md)
- [DAP User Guide](tutorials/DAP_USER_GUIDE.md)
- [Comprehensive Testing Guide](tutorials/COMPREHENSIVE_TESTING_GUIDE.md)
- [AI Build Guide](tutorials/AI_BUILD_GUIDE.md)

### How-to Guides
Task-focused instructions for common operational and development workflows.

- [Installation Guide](how-to/INSTALLATION.md)
- [GitHub Actions Integration](how-to/GITHUB_ACTIONS.md)
- [Upgrading](how-to/UPGRADING.md)
- [Editor Setup](how-to/EDITOR_SETUP.md)
- [Troubleshooting](how-to/TROUBLESHOOTING.md)
- [Continuous Testing](how-to/CONTINUOUS_TESTING.md)
- [Contributing LSP Features](how-to/CONTRIBUTING_LSP.md)
- [Threading Configuration Guide](how-to/THREADING_CONFIGURATION_GUIDE.md)
- [Performance Tuning](how-to/PERFORMANCE_TUNING.md)
- [Security Development Guide](how-to/SECURITY_DEVELOPMENT_GUIDE.md)

### Reference
Authoritative descriptions of configuration, architecture, commands, and feature contracts.

- [Commands Reference](reference/COMMANDS_REFERENCE.md)
- [Configuration Reference](reference/CONFIG.md)
- [Architecture Overview](reference/ARCHITECTURE_OVERVIEW.md)
- [LSP Features](reference/LSP_FEATURES.md)
- [Missing Documentation Guide](reference/MISSING_DOCUMENTATION_GUIDE.md)
- [API Documentation Standards](reference/API_DOCUMENTATION_STANDARDS.md)
- [FAQ](reference/FAQ.md)
- [Parser Feature Matrix](reference/PARSER_FEATURE_MATRIX.md)
- [Known Limitations](reference/KNOWN_LIMITATIONS.md)

### Explanation
Background material that explains why the system is designed the way it is.

- [LSP Documentation](explanation/LSP_DOCUMENTATION.md)
- [Cancellation Architecture Guide](explanation/CANCELLATION_ARCHITECTURE_GUIDE.md)
- [Pure Rust Parser](explanation/PURE_RUST_PARSER.md)
- [Slash Disambiguation](explanation/SLASH_DISAMBIGUATION.md)

### Project / ADR / Specs
Decision records, project status, and planning documents.

- [ADR Index](adr/README.md) — chronological index plus a topic guide for parser, runtime, DAP, security, and swarm decisions
- [Project Milestones](project/MILESTONES.md)
- [Feature Governance](project/FEATURE_GOVERNANCE.md)
- [Metric Stack](project/metrics/README.md) — contributor-facing summary of the layered scorecard model and the ratchet
- [Latency Caps SLO Spec](specs/LATENCY_CAPS_SLO_SPEC.md)
- [Release Candidate Baseline](specs/RELEASE_CANDIDATE_BASELINE.md)

### Historical Analyses
Long-form historical writing plus the supporting research notes that fed it.

- [Articles and Research Notes](articles/README.md)
- [Five Eras of AI-Assisted Development](articles/FIVE_ERAS.md)
- [Agentic Swarm Development](articles/SWARM_METHODOLOGY.md)
- [Parsing Perl](articles/PARSING_PERL.md)
- [Zero-Panic Reliability and Security](articles/ZERO_PANIC.md)
- [Curiosities, Records, and Surprising Facts](articles/CURIOSITIES.md)

## Suggested Reading Order for New Contributors

1. [Getting Started](tutorials/GETTING_STARTED.md)
2. [Installation Guide](how-to/INSTALLATION.md)
3. [Commands Reference](reference/COMMANDS_REFERENCE.md)
4. [Architecture Overview](reference/ARCHITECTURE_OVERVIEW.md)
5. [LSP Development Guide](tutorials/LSP_DEVELOPMENT_GUIDE.md)

## CLI Quick Reference

These commands are especially useful when validating an installation or triaging an environment issue:

```bash
perllsp --version
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
perllsp --completion bash
```

For the complete option list and behavior, see the [Commands Reference](reference/COMMANDS_REFERENCE.md).
