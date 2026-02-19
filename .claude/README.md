# perl-lsp PR Agent Workflow

This directory contains specialized agents for managing the complete PR lifecycle in the perl-lsp repository. The agents work together to provide automated review, testing, cleanup, and merge processes while maintaining the high quality standards required for the Perl parser ecosystem.

## Workflow Overview

The new PR workflow follows a structured pipeline designed to catch issues early and provide comprehensive validation:

```
1. pr-initial-reviewer     → Quick T1 review, catch obvious issues
2. [Review Loop]*:
   - test-runner-analyzer  → Run tests, diagnose failures
   - context-scout        → Code reconnaissance, find patterns
   - pr-cleanup-agent     → Fix issues systematically
3. pr-finalize-agent      → Final validation before merge
4. pr-merger              → Execute merge with final checks
5. pr-doc-finalize        → Update documentation post-merge
6. pr-finalizer          → Verify merge completion, prepare for next PR
```

## Agent Descriptions

### 1. pr-initial-reviewer
**Purpose**: Fast initial review to catch obvious issues early  
**Model**: Haiku (cost-effective)  
**Color**: Blue  

- Scans for syntax errors, basic code quality issues
- Verifies test coverage for new functionality
- Checks adherence to tree-sitter-perl standards
- Posts structured GitHub review comments
- Guides orchestrator to next appropriate agent

**When to use**: First step for any new PR or pushed commits

### 2. test-runner-analyzer
**Purpose**: Comprehensive test execution and failure analysis  
**Model**: Haiku  
**Color**: Yellow  

- Executes tree-sitter-perl test commands (cargo nextest, xtask)
- Analyzes test failures systematically
- Provides actionable diagnostic reports
- Handles performance regression testing
- **Note**: GitHub CI is disabled - this agent provides authoritative test validation

**Key commands**:
- `cargo nextest run` - Fast parallel testing (preferred)
- `cargo xtask corpus` - Comprehensive Perl 5 parsing validation
- `cargo xtask compare` - Performance regression testing

### 3. context-scout
**Purpose**: Code reconnaissance without modification  
**Model**: Haiku  
**Color**: Green  

- Rapidly locates implementations and patterns
- Maps architectural relationships
- Finds related code without expensive searches
- Provides focused code snippets and context
- Guides implementation decisions with existing patterns

**Specializations**:
- LSP server architecture patterns
- Parser implementation patterns
- Perl language edge cases
- Rust ecosystem patterns

### 4. pr-cleanup-agent
**Purpose**: Systematic PR issue resolution  
**Model**: Sonnet  
**Color**: Cyan  

- Addresses reviewer feedback comprehensively
- Fixes failing tests and code quality issues
- Updates documentation in sync with changes
- Provides clear explanations of fixes made
- Manages GitHub PR communication

**Validation performed**:
- Local test execution (cargo nextest, xtask)
- Performance regression checks
- Code quality (clippy, formatting)
- LSP and DAP functionality verification

### 5. pr-finalize-agent
**Purpose**: Final pre-merge validation  
**Model**: Sonnet  
**Color**: Cyan  

- Performs comprehensive final quality checks
- Verifies all review feedback addressed
- Ensures merge requirements satisfied
- Prepares merge metadata and commit messages
- Acts as quality gate before pr-merger

**Quality gates**:
- All tests passing locally
- No compilation warnings
- Performance benchmarks stable
- API contracts documented
- Breaking changes properly documented

### 6. pr-merger
**Purpose**: Execute final merge with integration checks  
**Model**: Sonnet  
**Color**: Red  

- Performs final validation post-finalization
- Resolves merge conflicts if needed
- Executes merge using appropriate strategy
- Updates PR status and labels
- Triggers post-merge documentation updates

**Merge strategies**:
- Squash merge for feature branches
- Merge commit for complex integrations
- Rebase for linear history when appropriate

### 7. pr-doc-finalize
**Purpose**: Post-merge documentation updates  
**Model**: Sonnet  
**Color**: Cyan  

- Updates all documentation affected by merged changes
- Applies Diataxis framework for structured docs
- Makes opportunistic improvements to related docs
- Verifies code examples and links
- Ensures published crate documentation currency

**Documentation types**:
- API documentation
- LSP capability matrix updates
- Performance benchmark updates
- Architecture documentation
- User guides and tutorials

### 8. pr-finalizer
**Purpose**: Post-merge verification and cleanup  
**Model**: Sonnet  
**Color**: Red  

- Verifies pr-merger completed successfully
- Ensures repository synchronized with origin
- Validates development environment ready
- Cleans up local branches and references
- Confirms readiness for next PR cycle

## Usage Guidelines

### Starting a PR Review
```bash
# User initiates with any PR
"Review PR #123 for the LSP hover improvements"

# Claude Code will invoke pr-initial-reviewer first
# The agent will post GitHub review comments and recommend next steps
```

### Automatic Flow Progression
The agents include orchestration guidance that directs the workflow:

- **pr-initial-reviewer** → routes to test-runner-analyzer or pr-cleanup-agent
- **test-runner-analyzer** → routes to context-scout or pr-cleanup-agent  
- **context-scout** → routes to pr-cleanup-agent or escalates
- **pr-cleanup-agent** → routes to pr-finalize-agent or returns to testing
- **pr-finalize-agent** → routes to pr-merger or back to review loop
- **pr-merger** → triggers pr-doc-finalize automatically
- **pr-doc-finalize** → triggers pr-finalizer for completion
- **pr-finalizer** → confirms workflow complete

### Manual Agent Invocation
You can also invoke agents directly when needed:

```bash
# Run tests on current PR
"Use test-runner-analyzer to run the full test suite"

# Find implementation patterns  
"Use context-scout to find how LSP completion is implemented"

# Clean up PR issues
"Use pr-cleanup-agent to address all reviewer feedback"

# Final merge preparation
"Use pr-finalize-agent to prepare for merge"
```

## tree-sitter-perl Context

All agents are configured with deep knowledge of:

### Published Crates (v0.8.7+ GA)
- **perl-parser**: Main parser + perl-lsp binary (~100% Perl 5 coverage)
- **perl-lexer**: Context-aware tokenizer with slash disambiguation  
- **perl-corpus**: Comprehensive test corpus with edge case collection
- **perl-parser-pest**: Legacy Pest-based parser (deprecated)

### Development Standards
- **Rust 2024** edition with MSRV 1.92+ compatibility
- **Performance targets**: 1-150 µs parsing speeds, 4-19x improvement over legacy
- **Test coverage**: ~100% Perl 5 syntax coverage including ALL edge cases
- **LSP compliance**: LSP 3.18+ protocol, ~75% feature coverage
- **Quality tools**: cargo-nextest, xtask automation, comprehensive corpus validation

### Key Testing Commands
- `cargo nextest run` - Fast parallel testing (preferred)
- `cargo xtask test` - Comprehensive test automation
- `cargo xtask corpus` - Full Perl parsing validation  
- `cargo xtask compare` - Performance regression testing
- `cargo test -p perl-parser --test lsp_comprehensive_e2e_test` - LSP validation

## Error Handling

Each agent includes comprehensive error handling:

### Common Error Scenarios
1. **Test failures**: Route to test-runner-analyzer for diagnosis
2. **Merge conflicts**: pr-merger handles resolution automatically  
3. **Performance regressions**: Caught by performance testing
4. **Missing documentation**: pr-doc-finalize ensures currency
5. **Repository sync issues**: pr-finalizer provides recovery guidance

### Escalation Paths
- **Agent limitations**: Clear handoff to manual review with preserved state
- **Complex conflicts**: Detailed analysis with recommended resolution steps
- **Architecture decisions**: Escalation to maintainers with full context

## Benefits

This workflow provides:

1. **Early Issue Detection**: pr-initial-reviewer catches problems before expensive validation
2. **Comprehensive Testing**: Local test execution with authoritative results
3. **Systematic Cleanup**: pr-cleanup-agent addresses all issues methodically  
4. **Quality Gates**: Multiple validation points ensure high standards
5. **Documentation Currency**: Automatic post-merge documentation updates
6. **Repository Health**: pr-finalizer ensures clean state for next development cycle

The agents work together to maintain the high quality standards required for tree-sitter-perl's published crate ecosystem while enabling efficient development velocity.