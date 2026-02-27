# PR Review Flow Orchestration Guide

This document provides guidelines for the tree-sitter-perl PR review agent orchestration flow, designed for Rust 2024 parser development with MSRV 1.92+ compatibility, cargo-nextest parallel testing, and xtask automation.

## Flow Overview

**Standard Flow**: `pr-initial-reviewer` → **[iterative loop]** → `pr-finalize-agent` → `pr-merger` → `pr-doc-finalize`

**Iterative Loop**: `test-runner-analyzer` → `context-scout` → `pr-cleanup-agent`

## Agent Responsibilities & Decision Points

### 1. pr-initial-reviewer (Entry Point)
**Role**: Fast T1 analysis, catch obvious issues early
**Domain**: tree-sitter-perl ecosystem awareness, Rust 2024, LSP 3.18+, parser coverage

**Decision Matrix**:
- ✅ **No significant issues** → Route to `test-runner-analyzer`
- ⚠️ **Tests failing/parser edge cases** → Direct to `test-runner-analyzer` 
- 🔍 **Complex architecture concerns** → Route to `context-scout`
- 🛠️ **Obvious code issues but sound foundation** → Direct to `pr-cleanup-agent`
- ❌ **Fundamentally flawed** → Document issues, recommend manual review

**GitHub Integration**: 
- Post structured review using `gh pr comment`
- Address reviewer feedback with `gh pr review`
- Update labels based on findings

### 2. test-runner-analyzer (Validation Authority)
**Role**: Execute comprehensive testing with cargo-nextest and xtask automation
**Authority**: Since GitHub CI is disabled, this agent is the authoritative test validator

**Test Arsenal**:
- `cargo nextest run --workspace` (parallel testing)
- `cargo xtask corpus` (~100% Perl syntax coverage)
- `cargo xtask compare` (performance regression 1-150 µs)
- `cargo test -p perl-parser --test lsp_comprehensive_e2e_test` (LSP validation)

**Decision Matrix**:
- ✅ **All tests pass cleanly** → Route to `pr-finalize-agent`
- 🔍 **Parser/lexer architectural issues** → Route to `context-scout`
- 📊 **Performance regressions** → Continue analysis with detailed benchmarks
- 🛠️ **Systematic code quality issues** → Direct to `pr-cleanup-agent`
- 🧪 **Edge case test failures** → Route to `context-scout` for coverage analysis
- 🔄 **Fundamental parser failures** → Return to `pr-initial-reviewer`

### 3. context-scout (Architecture Analysis)
**Role**: Rapid code reconnaissance for implementation patterns
**Domain Expertise**: Perl/LSP/parser patterns, no code modification

**Pattern Recognition**:
- **Perl Language**: Edge cases, modern syntax, pragma system
- **LSP Architecture**: Protocol methods, feature providers, capabilities
- **Parser Patterns**: Recursive descent, lexing, error recovery
- **Rust Ecosystem**: Testing, automation, performance patterns

**Decision Matrix**:
- ✅ **Implementation patterns clear** → Route to `pr-cleanup-agent`
- 🧪 **Test coverage gaps identified** → Return to `test-runner-analyzer`
- ⚠️ **Architectural concerns persist** → Escalate to manual review
- 📊 **Performance implications** → Continue analysis with benchmarks

### 4. pr-cleanup-agent (Systematic Remediation)
**Role**: Comprehensive issue resolution with GitHub integration
**Authority**: Execute fixes, address reviewer feedback, local validation

**Capabilities**:
- Fix failing tests with xtask automation
- Implement reviewer suggestions maintaining parser coverage
- Apply Rust 2024 standards with MSRV 1.92+ compatibility
- GitHub status management and reviewer communication

**Decision Matrix**:
- ✅ **All issues resolved, tests pass** → Route to `pr-finalize-agent`
- 🧪 **New issues discovered during fixes** → Return to `test-runner-analyzer`
- 🔍 **Architectural patterns unclear** → Route to `context-scout`
- ⚠️ **Fundamental design problems** → Escalate with detailed findings
- 🔧 **Complexity exceeds scope** → Push progress, recommend manual intervention

### 5. pr-finalize-agent (Quality Gate)
**Role**: Final validation and merge preparation
**Authority**: Authoritative quality gate, comprehensive local verification

**Validation Requirements**:
- Complete test suite validation (nextest + xtask)
- Performance regression checks via `cargo xtask compare`
- Rust 2024 compliance with MSRV 1.92+
- Reviewer feedback resolution verification

**Decision Matrix**:
- ✅ **Full validation successful** → Route to `pr-merger`
- 🛠️ **Critical issues discovered** → Return to `pr-cleanup-agent`
- 🧪 **Test failures during final validation** → Return to `test-runner-analyzer`
- 🔧 **Merge conflicts/external blockers** → Document, push progress, manual review

### 6. pr-merger (Integration Execution)
**Role**: Execute merge after finalization validation
**Authority**: Final integration decision and execution

**Merge Process**:
- Verify pr-finalize-agent validation completed
- Execute final smoke tests
- Resolve any last-minute conflicts
- Execute merge with appropriate strategy

**Post-Merge Action**:
- ✅ **Successful merge** → Immediately trigger `pr-doc-finalize`
- 🔄 **Issues discovered during merge** → Route back to appropriate agent
- 📋 **Complex conflicts** → Manual escalation with detailed analysis

### 7. pr-doc-finalize (Documentation Completion)
**Role**: Post-merge documentation updates using Diataxis framework
**Domain**: tree-sitter-perl documentation ecosystem, published crates

**Diataxis Application**:
- **Tutorials**: Getting started guides for perl-lsp, parser setup
- **How-to Guides**: Configuration, contribution, feature implementation
- **Reference**: LSP capabilities, API docs, xtask commands
- **Explanation**: Architecture decisions, parsing complexity

**Completion**: Final step in PR review flow - workflow complete

## GitHub Integration Best Practices

### Communication Standards
- Use structured markdown with clear sections
- Include file references with line numbers: `file.rs:123`
- Tag relevant stakeholders when escalating
- Provide actionable next steps with specific commands

### Status Management Commands
```bash
# Post comprehensive updates
gh pr comment --body "🔍 Analysis Complete: $(findings)"

# Update PR labels and status
gh pr edit --add-label "tests-passing" --remove-label "needs-work"

# Address reviewer feedback
gh pr review --comment --body "✅ Fixed: [explanation]"

# Request re-review after fixes
gh pr ready
```

### Error Recovery Protocol
When agents encounter blockers:
1. **Document findings** in PR comment
2. **Push current progress**: `git push origin HEAD`
3. **Create handoff instructions** for resuming work
4. **Tag appropriate stakeholders** for decisions beyond agent scope
5. **Provide specific next steps** for manual resolution

## Flow Control Principles

### Flexibility Guidelines
- Agents recommend next steps but orchestrator makes final routing decisions
- Multiple valid paths exist - adapt based on PR complexity and findings
- Early escalation is preferred over extended failing loops
- Preserve work state when handing off to manual review

### Loop Management
- **Maximum 3 iterations** through test-runner-analyzer → context-scout → pr-cleanup-agent
- **Progress tracking**: Each iteration should show measurable improvement
- **Escalation triggers**: Fundamental design issues, performance regressions, architectural conflicts
- **Success criteria**: Clear path to pr-finalize-agent with comprehensive validation

### Local Verification Priority
Since GitHub CI is disabled:
- cargo-nextest is the authoritative test runner
- xtask automation provides comprehensive validation
- Performance benchmarks via `cargo xtask compare` are required
- Local validation results are final - no external CI dependency

## Quality Gates Summary

Each agent enforces specific quality requirements:

1. **pr-initial-reviewer**: Obvious issues, basic standards compliance
2. **test-runner-analyzer**: Comprehensive test coverage, performance validation
3. **context-scout**: Implementation pattern consistency, architectural alignment
4. **pr-cleanup-agent**: Systematic issue resolution, reviewer satisfaction
5. **pr-finalize-agent**: Merge readiness, comprehensive final validation
6. **pr-merger**: Integration execution, conflict resolution
7. **pr-doc-finalize**: Documentation completeness using Diataxis framework

The orchestration flow ensures every PR meets tree-sitter-perl's standards for Rust 2024 parser development, LSP 3.18+ compliance, and comprehensive Perl syntax coverage while maintaining development velocity and code quality.