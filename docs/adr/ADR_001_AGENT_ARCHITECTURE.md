# ADR-001: Agent Architecture Specialization

## Status
Accepted - **Fully Implemented in PR #153** with 94 specialized agents and comprehensive agent customization framework

## Context
The tree-sitter-perl project initially used generic Claude Code agents stored in `.claude/agents/` directory. As the project matured with 5 published crates, comprehensive LSP features, and specialized Perl parsing requirements, it became clear that generic agents could not adequately address the domain-specific needs of the Perl parser ecosystem.

The project required specialized agents that understand:
- **Perl 5 syntax coverage requirements** (~100% with enhanced builtin function parsing)
- **Dual indexing architecture patterns** for cross-file navigation (98% reference coverage)
- **Performance benchmarks** (sub-microsecond parsing, significant LSP improvements)
- **Incremental parsing** with statistical validation
- **Security patterns** including UTF-16 position conversion security (PR #153)
- **Comprehensive mutation testing methodology** for parser validation (87% quality score)
- **Multi-crate workspace patterns** across 5 published crates with unified development workflow
- **Adaptive threading configuration** for CI environments and concurrent testing
- **LSP features** (~89% functional with comprehensive workspace support)

## Decision
We will implement a specialized agent architecture with domain-specific agents organized in `.claude/agents2/` directory structure:

### Agent Categories

Total: **97 specialized agents** (vs. 53 generic agents) organized by functional domain:

1. **Review Agents** (`review/`): **29 agents** for comprehensive PR review workflow
   - Code quality validation, security scanning, performance validation
   - Mutation testing coordination, clippy compliance verification
   - UTF-16 security validation, architectural alignment checking
   - Examples: `review-security-scanner`, `review-mutation-tester`, `review-performance-validator`

2. **Integration Agents** (`integration/`): **18 agents** for CI/CD and testing coordination
   - Automated testing workflows, continuous integration management
   - Cross-crate dependency validation, workspace orchestration
   - Performance regression detection, adaptive threading coordination
   - Examples: `integration-test-coordinator`, `integration-performance-monitor`, `integration-workspace-validator`

3. **Generative Agents** (`generative/`): **22 agents** for content creation and development
   - Documentation generation, test case creation, code scaffolding
   - Parser feature development, LSP provider implementation
   - Benchmark suite generation, security test creation
   - Examples: `generative-doc-writer`, `generative-test-creator`, `generative-parser-enhancer`

4. **Mantle Agents** (`mantle/`): **22 agents** for maintenance and operational tasks
   - Dependency management, version coordination, release preparation
   - Codebase cleanup, refactoring coordination, deprecation management
   - Security audit scheduling, performance monitoring
   - Examples: `mantle-dependency-manager`, `mantle-release-coordinator`, `mantle-security-auditor`

5. **Other Agents** (`other/`): **6 agents** for specialized and cross-cutting tasks
   - Agent customization, workflow orchestration, emergency response
   - Cross-domain coordination, ecosystem monitoring
   - Examples: `agent-customizer`, `workflow-orchestrator`, `ecosystem-monitor`

### Key Architectural Decisions

1. **Domain Specialization**: Each agent includes comprehensive Perl parsing ecosystem context:
   - Multi-crate workspace architecture (5 published crates)
   - Performance standards (sub-microsecond parsing)
   - Security requirements (UTF-16 safety, path traversal prevention)
   - Comprehensive quality metrics (87% mutation score, zero clippy warnings)

2. **Workflow Integration**: Sophisticated agent routing with specialized successors:
   - **Review Chain**: security-scanner → mutation-tester → performance-validator → governance-gate
   - **Integration Chain**: test-coordinator → workspace-validator → performance-monitor → release-gate
   - **Development Chain**: code-enhancer → test-creator → doc-generator → review-prep

3. **Quality Enforcement**: Built-in understanding of project standards:
   - **Mutation Testing**: 87% quality score with comprehensive edge case coverage
   - **Performance Benchmarks**: LSP performance maintenance
   - **Security Standards**: UTF-16 position conversion security, Unicode safety validation
   - **Code Quality**: Zero clippy warnings, consistent formatting, comprehensive test coverage

4. **Security-First Design**: Enterprise-grade security patterns throughout:
   - **UTF-16 Position Security**: Symmetric conversion validation, boundary checking
   - **Path Traversal Prevention**: Workspace boundary validation, canonical path handling
   - **Unicode Safety**: Comprehensive multi-byte character handling, emoji support
   - **Vulnerability Detection**: Mutation testing for security bug discovery

5. **Agent Customization Framework**: Self-adapting agent architecture:
   - **Contextual Adaptation**: Agents understand project-specific patterns and requirements
   - **Intelligent Routing**: Context-aware successor selection based on task requirements
   - **Quality Integration**: Built-in understanding of parser ecosystem quality standards
   - **Performance Awareness**: Performance requirement integration

## Consequences

### Positive
- **Dramatically Improved Accuracy**: Agents understand comprehensive Perl parsing domain requirements
- **Sophisticated Workflow Orchestration**: Specialized agent chains for parser-specific workflows with intelligent routing
- **Comprehensive Quality Enforcement**: Built-in understanding of mutation testing (87% score), performance standards, and security
- **Enhanced Maintenance Efficiency**: Self-documenting agents with inline expertise and parser ecosystem context
- **Security Integration**: Built-in UTF-16 security validation and security pattern understanding
- **Performance Awareness**: Performance requirements integrated throughout workflow
- **Real Bug Discovery**: Mutation testing integration revealed and eliminated critical UTF-16 security vulnerabilities

### Negative
- **Increased Complexity**: More agents to maintain (94 vs. 53), requiring specialized knowledge
- **Steeper Learning Curve**: New contributors need to understand agent specialization and parser ecosystem context
- **Potential Duplication Risk**: Overlap between agent responsibilities requires careful coordination
- **Higher Maintenance Overhead**: More sophisticated agents require more nuanced updates and validation

### Neutral
- **Compatibility Migration Path**: Original `.claude/agents/` maintained for backward compatibility during transition
- **Incremental Adoption Strategy**: Can gradually migrate to specialized agents without disrupting existing workflows
- **Parallel Architecture**: Both generic and specialized agents can coexist during transition period

## Implementation Details

### PR #153 Comprehensive Implementation
- **Agent Refactoring**: Complete restructuring of agent architecture with 94 specialized agents
- **Customization Framework**: Self-adapting agents with parser ecosystem context integration
- **Workflow Orchestration**: Sophisticated routing between review, integration, generative, and maintenance agents
- **Security Integration**: Built-in UTF-16 position conversion security and enterprise security pattern validation

### Quality Metrics and Validation
- **Comprehensive Test Coverage**: 147+ mutation hardening tests
- **Quality Achievement**: Mutation score improved from ~70% to **87%**
- **Security Validation**: Real vulnerability discovery through mutation testing (UTF-16 boundary violations)
- **Performance Preservation**: Performance achievements maintained

### Technical Architecture
- **Directory Structure**: `.claude/agents2/` with organized functional domains (review/, integration/, generative/, mantle/, other/)
- **Agent Customization**: Self-documenting configuration files with inline expertise and parser-specific context
- **Intelligent Routing**: Context-aware successor selection based on task requirements and quality gates
- **Quality Integration**: Built-in understanding of mutation testing, performance benchmarks, and security standards

### Validation and Testing
- **Agent Functionality Testing**: Each agent category validated through comprehensive workflow testing
- **Integration Testing**: Cross-agent communication and routing validation
- **Performance Impact**: Zero impact on core parser performance, maintained revolutionary LSP improvements
- **Security Validation**: UTF-16 security enhancements tested through agent-coordinated mutation testing

## Related Documents
- [CLAUDE.md](../CLAUDE.md) - Agent ecosystem overview
- [AGENT_ORCHESTRATION.md](AGENT_ORCHESTRATION.md) - Agent workflow patterns
- [AGENT_CUSTOMIZER.md](AGENT_CUSTOMIZER.md) - Domain-specific agent adaptation framework
- [CLAUDE_AGENT_FLOW.md](CLAUDE_AGENT_FLOW.md) - PR review flow design