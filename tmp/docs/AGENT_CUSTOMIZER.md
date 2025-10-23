# Agent Customization Framework

This document provides guidance for customizing Claude Code agents to work effectively with the tree-sitter-perl parsing ecosystem.

## Overview

The agent customization framework enables adaptation of generic Claude Code agents to understand and work with the specialized requirements of Perl parsing development. This system transforms generic agents into domain-specific specialists that understand:

- **Multi-crate workspace architecture** with 5 published crates
- **Revolutionary performance requirements** (5000x LSP improvements, sub-microsecond parsing)
- **Enterprise security standards** (UTF-16 safety, path traversal prevention)
- **Comprehensive quality metrics** (87% mutation score, zero clippy warnings)
- **Dual indexing patterns** for cross-file navigation

## Agent Architecture

### Specialized Agent Categories

The framework organizes 97 specialized agents across 5 functional domains:

1. **Review Agents** (`review/`): **29 agents**
   - Code quality validation, security scanning, performance validation
   - Mutation testing coordination, clippy compliance verification
   - UTF-16 security validation, architectural alignment checking

2. **Integration Agents** (`integration/`): **18 agents**
   - Automated testing workflows, continuous integration management
   - Cross-crate dependency validation, workspace orchestration
   - Performance regression detection, adaptive threading coordination

3. **Generative Agents** (`generative/`): **22 agents**
   - Documentation generation, test case creation, code scaffolding
   - Parser feature development, LSP provider implementation
   - Benchmark suite generation, security test creation

4. **Mantle Agents** (`mantle/`): **22 agents**
   - Dependency management, version coordination, release preparation
   - Codebase cleanup, refactoring coordination, deprecation management
   - Security audit scheduling, performance monitoring

5. **Other Agents** (`other/`): **6 agents**
   - Agent customization, workflow orchestration, emergency response
   - Cross-domain coordination, ecosystem monitoring

## Customization Process

### 1. Domain Context Integration

Each agent receives comprehensive Perl parsing ecosystem context:

```yaml
# Example agent customization
parser_ecosystem_context:
  crates:
    - perl-parser (main parsing library)
    - perl-lsp (LSP server binary)
    - perl-lexer (tokenization)
    - perl-corpus (test infrastructure)
    - perl-parser-pest (legacy)

  performance_requirements:
    - sub_microsecond_parsing: true
    - lsp_improvements: "5000x"
    - incremental_parsing: "<1ms"
    - adaptive_threading: true

  security_standards:
    - utf16_safety: true
    - path_traversal_prevention: true
    - unicode_safe_handling: true
    - enterprise_grade: true

  quality_metrics:
    - mutation_score: "87%"
    - clippy_warnings: 0
    - test_coverage: "295+ tests"
    - ci_reliability: "100%"
```

### 2. Workflow Integration

Agents understand sophisticated routing patterns:

- **Review Chain**: security-scanner → mutation-tester → performance-validator → governance-gate
- **Integration Chain**: test-coordinator → workspace-validator → performance-monitor → release-gate
- **Development Chain**: code-enhancer → test-creator → doc-generator → review-prep

### 3. Quality Enforcement

Built-in understanding of enterprise standards:

- **Mutation Testing**: 87% quality score with comprehensive edge case coverage
- **Performance Benchmarks**: Revolutionary LSP performance maintenance
- **Security Standards**: UTF-16 position conversion security, Unicode safety validation
- **Code Quality**: Zero clippy warnings, consistent formatting, comprehensive test coverage

## Usage Patterns

### Basic Agent Customization

```bash
# Navigate to agent customizer
cd .claude/agents2

# Use agent-customizer to adapt generic agents
# The agent-customizer.md file contains the specialized customization logic
```

### Agent Selection Guidelines

1. **For Code Review**: Use `review/` category agents
   - `review-security-scanner` for security validation
   - `review-mutation-tester` for comprehensive testing
   - `review-performance-validator` for performance requirements

2. **For CI/CD Integration**: Use `integration/` category agents
   - `integration-test-coordinator` for test orchestration
   - `integration-workspace-validator` for multi-crate validation
   - `integration-performance-monitor` for regression detection

3. **For Development Tasks**: Use `generative/` category agents
   - `generative-doc-writer` for documentation
   - `generative-test-creator` for test generation
   - `generative-parser-enhancer` for feature development

4. **For Maintenance**: Use `mantle/` category agents
   - `mantle-dependency-manager` for dependency coordination
   - `mantle-release-coordinator` for version management
   - `mantle-security-auditor` for security monitoring

## Key Features

### Self-Adapting Architecture

- **Contextual Adaptation**: Agents understand project-specific patterns
- **Intelligent Routing**: Context-aware successor selection
- **Quality Integration**: Built-in understanding of parser ecosystem standards
- **Performance Awareness**: Revolutionary performance requirement integration

### Security-First Design

- **UTF-16 Position Security**: Symmetric conversion validation
- **Path Traversal Prevention**: Workspace boundary validation
- **Unicode Safety**: Multi-byte character handling
- **Vulnerability Detection**: Mutation testing for security bug discovery

### Performance Integration

- **Revolutionary Standards**: 5000x LSP improvements maintained
- **Adaptive Threading**: Thread-aware timeout scaling
- **Incremental Parsing**: <1ms update validation
- **Benchmark Integration**: Performance regression prevention

## Implementation Details

### Agent File Structure

```markdown
---
name: agent-name
description: Agent purpose and specialization
model: sonnet
color: domain-specific-color
---

You are a [Domain] Specialist specialized in Rust-based parsing ecosystems...
[Comprehensive parser ecosystem context]
[Domain-specific customization logic]
[Integration patterns and routing]
```

### Customization Framework Location

The primary customization logic resides in:
- **Agent Customizer**: `.claude/agents2/agent-customizer.md`
- **Specialized Agents**: `.claude/agents2/{category}/agent-name.md`
- **Orchestration Logic**: `docs/AGENT_ORCHESTRATION.md`

## Best Practices

1. **Domain Specialization**: Ensure agents understand comprehensive Perl parsing context
2. **Workflow Integration**: Implement sophisticated routing with quality gates
3. **Security-First**: Integrate enterprise security validation throughout
4. **Performance Awareness**: Maintain revolutionary performance standards
5. **Quality Enforcement**: Built-in understanding of mutation testing and benchmarks

## Related Documentation

- **[ADR-001: Agent Architecture](ADR_001_AGENT_ARCHITECTURE.md)** - Design decisions and rationale
- **[Agent Orchestration](AGENT_ORCHESTRATION.md)** - Workflow patterns and routing
- **[CLAUDE.md](../CLAUDE.md)** - Project overview and agent ecosystem context

## Validation

The agent customization framework is validated through:

- **Comprehensive Test Coverage**: 147+ mutation hardening tests
- **Quality Achievement**: 87% mutation score (exceeded enterprise target)
- **Security Validation**: Real vulnerability discovery through testing
- **Performance Preservation**: All revolutionary achievements maintained

This framework ensures that agents are not just customized but truly specialized for the demanding requirements of enterprise-grade Perl parsing development.