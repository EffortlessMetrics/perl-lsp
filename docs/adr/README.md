# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) for significant design decisions in the Perl LSP project.

## ADR Index

| ADR | Status | Date | Title | Description |
|-----|--------|------|-------|-------------|
| [ADR-0001](0001-substitution-operator-parsing-architecture.md) | Accepted | 2025-01-20 | Substitution Operator Parsing Architecture | Comprehensive parsing support for Perl substitution operators (`s///`) with all modifiers and delimiter styles |
| [ADR-0002](0002-api-documentation-infrastructure.md) | Accepted | 2025-09-20 | API Documentation Infrastructure Strategy | Enterprise-grade documentation enforcement with `#![warn(missing_docs)]` and systematic validation framework |
| [ADR-001](ADR_001_AGENT_ARCHITECTURE.md) | Accepted | PR #153 | Agent Architecture Specialization | 97 specialized agents with domain-specific expertise for Perl parser ecosystem workflow |
| [ADR-002](ADR_002_API_DOCUMENTATION_INFRASTRUCTURE.md) | Accepted | PR #160 | API Documentation Infrastructure (SPEC-149) | Comprehensive documentation enforcement strategy with acceptance criteria and quality gates |
| [ADR-003a](ADR_003_EXECUTE_COMMAND_CODE_ACTIONS_ARCHITECTURE.md) | Draft | 2025-01-15 | Execute Command and Code Actions | LSP executeCommand and code actions integration with perlcritic and refactoring support |
| [ADR-003b](ADR_003_HEREDOC_MANUAL_PARSING.md) | Proposed | 2025-11-05 | Manual Heredoc Declaration Parsing | Character-by-character state machine parser for heredoc declarations |
| [ADR-003c](ADR_003_MISSING_DOCUMENTATION_INFRASTRUCTURE.md) | Accepted | PR #159 | Missing Documentation Infrastructure (SPEC-149) | Documentation enforcement infrastructure with validation framework |

> **Note**: ADR-003a/b/c are disambiguated variants pending proper renumbering. File names unchanged for compatibility.

## About ADRs

Architecture Decision Records (ADRs) capture important architectural decisions along with their context and consequences. Each ADR includes:

- **Context**: The situation that led to the decision
- **Decision**: The architectural choice made
- **Consequences**: The results of the decision, both positive and negative

## ADR Process

1. **Identify Decision**: When facing a significant architectural choice
2. **Document Options**: Record all considered alternatives with pros/cons
3. **Make Decision**: Choose the best option based on decision drivers
4. **Record ADR**: Document the decision with full context
5. **Update Index**: Add the new ADR to this index
6. **Link Documentation**: Cross-reference with relevant implementation docs

## Status Definitions

- **Proposed**: Under consideration
- **Accepted**: Decision made and implemented
- **Deprecated**: No longer current but kept for historical context
- **Superseded**: Replaced by a newer decision

## Cross-References

- [CLAUDE.md](../CLAUDE.md) - Project overview and capabilities
- [CRATE_ARCHITECTURE_GUIDE.md](../CRATE_ARCHITECTURE_GUIDE.md) - System architecture
- [PARSER_COMPARISON.md](../PARSER_COMPARISON.md) - Parser implementation details