# ADR-001: Agent Architecture Specialization

## Status
Accepted - Implemented in PR #153

## Context
The tree-sitter-perl project initially used generic Claude Code agents stored in `.claude/agents/` directory. As the project matured with 5 published crates, enterprise-grade LSP features, and specialized Perl parsing requirements, it became clear that generic agents could not adequately address the domain-specific needs of the Perl parser ecosystem.

The project required specialized agents that understand:
- Perl 5 syntax coverage requirements (~100%)
- Dual indexing architecture patterns
- Performance benchmarks (sub-microsecond parsing, 5000x LSP improvements) 
- Incremental parsing with statistical validation
- Enterprise security patterns
- Mutation testing methodology for parser validation

## Decision
We will implement a specialized agent architecture with domain-specific agents organized in `.claude/agents2/` directory structure:

### Agent Categories
1. **Review Agents** (`review/`): 26 agents for PR review workflow
2. **Integration Agents** (`integration/`): 21 agents for CI/CD and testing
3. **Generative Agents** (`generative/`): 24 agents for content creation
4. **Mantle Agents** (`mantle/`): 17 agents for maintenance tasks
5. **Other Agents** (`other/`): 6 agents for specialized tasks

Total: **94 specialized agents** (vs. 53 generic agents)

### Key Architectural Decisions
- **Domain Specialization**: Each agent includes Perl parsing ecosystem context
- **Workflow Integration**: Agents route to specialized successors in the ecosystem
- **Quality Standards**: Agents understand mutation testing, performance benchmarks, clippy compliance
- **Security Awareness**: Agents implement enterprise security patterns (path traversal prevention, Unicode safety)

## Consequences

### Positive
- **Improved Accuracy**: Agents understand Perl parsing domain requirements
- **Better Routing**: Specialized agent chains for parser-specific workflows
- **Quality Enforcement**: Built-in understanding of mutation testing and performance standards
- **Maintenance Efficiency**: Self-documenting agents with inline expertise

### Negative
- **Complexity**: More agents to maintain (94 vs. 53)
- **Learning Curve**: New contributors need to understand agent specialization
- **Duplication Risk**: Potential overlap between agent responsibilities

### Neutral
- **Migration Path**: Original `.claude/agents/` maintained for compatibility
- **Incremental Adoption**: Can gradually migrate to specialized agents

## Implementation Details
- **PR #153**: Agent refactoring and customization features
- **Test Coverage**: 62 mutation hardening tests added (531-line test suite)
- **Quality Improvement**: Mutation score improved from 75% toward >85%
- **Documentation**: Self-documenting agent configuration files

## Related Documents
- [CLAUDE.md](../CLAUDE.md) - Agent ecosystem overview
- [AGENT_ORCHESTRATION.md](AGENT_ORCHESTRATION.md) - Agent workflow patterns
- [CLAUDE_AGENT_FLOW.md](CLAUDE_AGENT_FLOW.md) - PR review flow design