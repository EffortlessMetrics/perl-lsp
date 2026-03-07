> For the documentation hub, see [README.md](README.md). This page describes documentation standards and the Diataxis structure.

# Documentation Guide

> **Main entry point for all perl-lsp documentation** - Navigate this guide to find exactly what you need.

## Quick Start

New to perl-lsp? Start here:

1. **[CLAUDE.md](../CLAUDE.md)** - Project overview, installation, essential commands
2. **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Comprehensive build/test commands
3. **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture overview

**Need help now?** Jump to:
- [Installation & Setup](#tutorials-learning-oriented) for getting started
- [Common Tasks](#how-to-guides-problem-oriented) for specific problems
- [API Reference](#reference-documentation-information-oriented) for technical specs
- [Architecture](#explanation-understanding-oriented) for understanding design

---

## Documentation Structure (Diataxis Framework)

This documentation follows the [Diataxis framework](https://diataxis.fr/), organizing content into four distinct categories:

### 🎓 Tutorials (Learning-Oriented)

**Goal**: Learn by doing through step-by-step guided experiences.

**Core Learning Paths**:
- **[EXECUTE_COMMAND_TUTORIAL.md](../tutorials/EXECUTE_COMMAND_TUTORIAL.md)** - Build custom LSP commands from scratch
- **[WORKSPACE_REFACTORING_TUTORIAL.md](../tutorials/WORKSPACE_REFACTORING_TUTORIAL.md)** - Master workspace-wide refactoring operations
- **[COMPREHENSIVE_TESTING_GUIDE.md](../tutorials/COMPREHENSIVE_TESTING_GUIDE.md)** - Test-driven development workflows

**Feature-Specific Tutorials**:
- **[DAP_USER_GUIDE.md](../tutorials/DAP_USER_GUIDE.md)** - Set up debugging with Debug Adapter Protocol
- **[LSP_DEVELOPMENT_GUIDE.md](../tutorials/LSP_DEVELOPMENT_GUIDE.md)** - Develop LSP features with source threading
- **[AI_BUILD_GUIDE.md](../tutorials/AI_BUILD_GUIDE.md)** - AI-assisted development workflows

**Validation**: All tutorials include testable code examples validated with `cargo test --doc`.

---

### 🔧 How-to Guides (Problem-Oriented)

**Goal**: Solve specific problems with task-oriented instructions.

**Development Tasks**:
- **[IMPORT_OPTIMIZER_GUIDE.md](../how-to/IMPORT_OPTIMIZER_GUIDE.md)** - Remove unused imports, add missing ones, sort alphabetically
- **[FILE_COMPLETION_GUIDE.md](../how-to/FILE_COMPLETION_GUIDE.md)** - Implement enterprise-secure path completion
- **[INCREMENTAL_PARSING_GUIDE.md](../how-to/INCREMENTAL_PARSING_GUIDE.md)** - Optimize parsing performance with <1ms updates
- **[SOURCE_THREADING_GUIDE.md](../how-to/SOURCE_THREADING_GUIDE.md)** - Extract documentation from source comments

**Testing & Debugging**:
- **[THREADING_CONFIGURATION_GUIDE.md](../how-to/THREADING_CONFIGURATION_GUIDE.md)** - Adaptive threading for CI environments
- **[LSP_ERROR_HANDLING_MONITORING_GUIDE.md](../how-to/LSP_ERROR_HANDLING_MONITORING_GUIDE.md)** - Monitor and debug LSP errors
- **[DAP_BREAKPOINT_VALIDATION_GUIDE.md](../how-to/DAP_BREAKPOINT_VALIDATION_GUIDE.md)** - Validate breakpoint positioning

**Optimization**:
- **[PERFORMANCE_PRESERVATION_GUIDE.md](../how-to/PERFORMANCE_PRESERVATION_GUIDE.md)** - Maintain performance baselines
- **[benchmarks/BENCHMARK_FRAMEWORK.md](benchmarks/BENCHMARK_FRAMEWORK.md)** - Cross-language performance analysis

**Security**:
- **[SECURITY_DEVELOPMENT_GUIDE.md](../how-to/SECURITY_DEVELOPMENT_GUIDE.md)** - Security development practices

**Commands**: All guides include proper cargo command specifications (`cargo test -p perl-parser`, `RUST_TEST_THREADS=2 cargo test -p perl-lsp`).

---

### 📚 Reference Documentation (Information-Oriented)

**Goal**: Look up precise technical information and API contracts.

**API Documentation**:
- **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Complete command reference with threading patterns
- **[WORKSPACE_REFACTOR_API_REFERENCE.md](WORKSPACE_REFACTOR_API_REFERENCE.md)** - Refactoring API contracts
- **[ERROR_HANDLING_API_CONTRACTS.md](ERROR_HANDLING_API_CONTRACTS.md)** - Error handling specifications
- **[API_DOCUMENTATION_STANDARDS.md](API_DOCUMENTATION_STANDARDS.md)** - Documentation requirements (PR #160)

**Architecture References**:
- **[CRATE_ARCHITECTURE_GUIDE.md](CRATE_ARCHITECTURE_GUIDE.md)** - System design and component organization
- **[CRATE_ARCHITECTURE_DAP.md](CRATE_ARCHITECTURE_DAP.md)** - DAP-specific architecture
- **[ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md)** - High-level system architecture
- **[MODERN_ARCHITECTURE.md](MODERN_ARCHITECTURE.md)** - Current architectural patterns

**Protocol Specifications**:
- **[LSP_CANCELLATION_PROTOCOL_SPECIFICATION.md](LSP_CANCELLATION_PROTOCOL_SPECIFICATION.md)** - JSON-RPC 2.0 cancellation protocol
- **[LSP_CANCELLATION_PERFORMANCE_SPECIFICATION.md](LSP_CANCELLATION_PERFORMANCE_SPECIFICATION.md)** - Performance requirements
- **[ISSUE_207_DAP_SPECIFICATION_ANALYSIS.md](ISSUE_207_DAP_SPECIFICATION_ANALYSIS.md)** - DAP protocol analysis

**Technical Specifications**:
- **[ROPE_INTEGRATION_GUIDE.md](ROPE_INTEGRATION_GUIDE.md)** - Document management implementation
- **[POSITION_TRACKING_GUIDE.md](POSITION_TRACKING_GUIDE.md)** - UTF-16/UTF-8 position mapping
- **[VARIABLE_RESOLUTION_GUIDE.md](VARIABLE_RESOLUTION_GUIDE.md)** - Scope analysis algorithms
- **[WORKSPACE_NAVIGATION_GUIDE.md](WORKSPACE_NAVIGATION_GUIDE.md)** - Cross-file navigation patterns

**Standards & Validation**:
- **[VALIDATION-149-acceptance-criteria.md](VALIDATION-149-acceptance-criteria.md)** - API documentation quality gates
- **[MISSING_DOCUMENTATION_GUIDE.md](MISSING_DOCUMENTATION_GUIDE.md)** - Documentation enforcement strategy
- **[STABILITY.md](STABILITY.md)** - API stability guarantees

**Doctests**: All reference documentation validated with `cargo test --doc` and `cargo doc --no-deps --package perl-parser`.

---

### 💡 Explanation (Understanding-Oriented)

**Goal**: Understand concepts, design decisions, and architectural choices.

**Core Concepts**:
- **[CANCELLATION_ARCHITECTURE_GUIDE.md](../explanation/CANCELLATION_ARCHITECTURE_GUIDE.md)** - LSP cancellation system design (PR #165)
- **[ERROR_HANDLING_STRATEGY.md](../explanation/ERROR_HANDLING_STRATEGY.md)** - Defensive programming principles (Issue #178)
- **[CONDITIONAL_DOCS_COMPILATION_STRATEGY.md](../explanation/CONDITIONAL_DOCS_COMPILATION_STRATEGY.md)** - Performance-optimized documentation enforcement

**Parsing Theory**:
- **[BUILTIN_FUNCTION_PARSING.md](../explanation/BUILTIN_FUNCTION_PARSING.md)** - Enhanced empty block parsing for map/grep/sort
- **[SLASH_DISAMBIGUATION.md](../explanation/SLASH_DISAMBIGUATION.md)** - Division vs. regex disambiguation
- **[TREE_SITTER_COMPATIBILITY.md](../explanation/TREE_SITTER_COMPATIBILITY.md)** - Tree-sitter integration patterns

**LSP Architecture**:
- **[LSP_CRATE_SEPARATION_GUIDE.md](../explanation/LSP_CRATE_SEPARATION_GUIDE.md)** - Crate organization rationale
- **[EXECUTE_COMMAND_CONFIGURATION_GUIDE.md](../explanation/EXECUTE_COMMAND_CONFIGURATION_GUIDE.md)** - executeCommand integration design
- **[LSP_DOCUMENTATION.md](../explanation/LSP_DOCUMENTATION.md)** - LSP feature implementation philosophy

**Design Decisions** (Architecture Decision Records):
- **[adr/ADR_001_AGENT_ARCHITECTURE.md](adr/ADR_001_AGENT_ARCHITECTURE.md)** - 97 specialized agents (PR #153)
- **[adr/ADR_002_API_DOCUMENTATION_INFRASTRUCTURE.md](adr/ADR_002_API_DOCUMENTATION_INFRASTRUCTURE.md)** - Documentation enforcement (PR #160)
- **[AGENT_ORCHESTRATION.md](AGENT_ORCHESTRATION.md)** - Agent ecosystem patterns
- **[AGENT_CUSTOMIZER.md](AGENT_CUSTOMIZER.md)** - Domain-specific agent adaptation

**Implementation Context**:
- **[PARSER_ROBUSTNESS_IMPROVEMENTS.md](../explanation/PARSER_ROBUSTNESS_IMPROVEMENTS.md)** - Fuzz testing and mutation hardening
- **[CI_HARDENING.md](CI_HARDENING.md)** - CI/CD reliability improvements
- **[SPEC_149_GOVERNANCE.md](SPEC_149_GOVERNANCE.md)** - Documentation quality governance

---

## Feature Index

Map features to their documentation:

### Parser Features
| Feature | Tutorial | How-to | Reference | Explanation |
|---------|----------|--------|-----------|-------------|
| **Incremental Parsing** | [COMPREHENSIVE_TESTING_GUIDE](../tutorials/COMPREHENSIVE_TESTING_GUIDE.md) | [INCREMENTAL_PARSING_GUIDE](../how-to/INCREMENTAL_PARSING_GUIDE.md) | [PERFORMANCE_PRESERVATION_GUIDE](../how-to/PERFORMANCE_PRESERVATION_GUIDE.md) | [ARCHITECTURE_OVERVIEW](ARCHITECTURE_OVERVIEW.md) |
| **Builtin Functions** | - | [COMMANDS_REFERENCE](COMMANDS_REFERENCE.md#testing) | [BUILTIN_FUNCTION_PARSING](../explanation/BUILTIN_FUNCTION_PARSING.md) | [BUILTIN_FUNCTION_PARSING](../explanation/BUILTIN_FUNCTION_PARSING.md) |
| **UTF-16/UTF-8** | - | [POSITION_TRACKING_GUIDE](POSITION_TRACKING_GUIDE.md) | [POSITION_TRACKING_GUIDE](POSITION_TRACKING_GUIDE.md) | [ROPE_INTEGRATION_GUIDE](ROPE_INTEGRATION_GUIDE.md) |

### LSP Features
| Feature | Tutorial | How-to | Reference | Explanation |
|---------|----------|--------|-----------|-------------|
| **Cross-File Navigation** | [WORKSPACE_REFACTORING_TUTORIAL](../tutorials/WORKSPACE_REFACTORING_TUTORIAL.md) | [WORKSPACE_NAVIGATION_GUIDE](WORKSPACE_NAVIGATION_GUIDE.md) | [WORKSPACE_REFACTOR_API_REFERENCE](WORKSPACE_REFACTOR_API_REFERENCE.md) | [LSP_CRATE_SEPARATION_GUIDE](../explanation/LSP_CRATE_SEPARATION_GUIDE.md) |
| **Import Optimization** | - | [IMPORT_OPTIMIZER_GUIDE](../how-to/IMPORT_OPTIMIZER_GUIDE.md) | [IMPORT_OPTIMIZER_GUIDE](../how-to/IMPORT_OPTIMIZER_GUIDE.md) | - |
| **File Completion** | - | [FILE_COMPLETION_GUIDE](../how-to/FILE_COMPLETION_GUIDE.md) | [FILE_COMPLETION_GUIDE](../how-to/FILE_COMPLETION_GUIDE.md) | [SECURITY_DEVELOPMENT_GUIDE](../how-to/SECURITY_DEVELOPMENT_GUIDE.md) |
| **executeCommand** | [EXECUTE_COMMAND_TUTORIAL](../tutorials/EXECUTE_COMMAND_TUTORIAL.md) | [EXECUTE_COMMAND_CONFIGURATION_GUIDE](../explanation/EXECUTE_COMMAND_CONFIGURATION_GUIDE.md) | [WORKSPACE_REFACTOR_API_REFERENCE](WORKSPACE_REFACTOR_API_REFERENCE.md) | [LSP_IMPLEMENTATION_GUIDE](LSP_IMPLEMENTATION_GUIDE.md) |
| **Cancellation** | - | [LSP_ERROR_HANDLING_MONITORING_GUIDE](../how-to/LSP_ERROR_HANDLING_MONITORING_GUIDE.md) | [LSP_CANCELLATION_PROTOCOL_SPECIFICATION](LSP_CANCELLATION_PROTOCOL_SPECIFICATION.md) | [CANCELLATION_ARCHITECTURE_GUIDE](../explanation/CANCELLATION_ARCHITECTURE_GUIDE.md) |

### DAP Features
| Feature | Tutorial | How-to | Reference | Explanation |
|---------|----------|--------|-----------|-------------|
| **Debug Setup** | [DAP_USER_GUIDE](../tutorials/DAP_USER_GUIDE.md) | [DAP_USER_GUIDE](../tutorials/DAP_USER_GUIDE.md) | [CRATE_ARCHITECTURE_DAP](CRATE_ARCHITECTURE_DAP.md) | [ISSUE_207_DAP_SPECIFICATION_ANALYSIS](ISSUE_207_DAP_SPECIFICATION_ANALYSIS.md) |
| **Breakpoints** | [DAP_USER_GUIDE](../tutorials/DAP_USER_GUIDE.md) | [DAP_BREAKPOINT_VALIDATION_GUIDE](../how-to/DAP_BREAKPOINT_VALIDATION_GUIDE.md) | [CRATE_ARCHITECTURE_DAP](CRATE_ARCHITECTURE_DAP.md) | - |

### Testing Features
| Feature | Tutorial | How-to | Reference | Explanation |
|---------|----------|--------|-----------|-------------|
| **Adaptive Threading** | [COMPREHENSIVE_TESTING_GUIDE](../tutorials/COMPREHENSIVE_TESTING_GUIDE.md) | [THREADING_CONFIGURATION_GUIDE](../how-to/THREADING_CONFIGURATION_GUIDE.md) | [COMMANDS_REFERENCE](COMMANDS_REFERENCE.md) | [CI_HARDENING](CI_HARDENING.md) |
| **Mutation Testing** | - | [COMPREHENSIVE_TESTING_GUIDE](../tutorials/COMPREHENSIVE_TESTING_GUIDE.md) | [PARSER_ROBUSTNESS_IMPROVEMENTS](../explanation/PARSER_ROBUSTNESS_IMPROVEMENTS.md) | [PARSER_ROBUSTNESS_IMPROVEMENTS](../explanation/PARSER_ROBUSTNESS_IMPROVEMENTS.md) |
| **Benchmarking** | - | [benchmarks/BENCHMARK_FRAMEWORK](benchmarks/BENCHMARK_FRAMEWORK.md) | [benchmarks/BENCHMARK_FRAMEWORK](benchmarks/BENCHMARK_FRAMEWORK.md) | [benchmarks/BENCHMARK_DESIGN](benchmarks/BENCHMARK_DESIGN.md) |

---

## Progressive Learning Paths

### Path 1: LSP Developer (New to Project)
1. Start: [CLAUDE.md](../CLAUDE.md) - Project overview and quick start
2. Setup: [AI_BUILD_GUIDE.md](../tutorials/AI_BUILD_GUIDE.md) - Development environment
3. Architecture: [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - System design
4. First Feature: [EXECUTE_COMMAND_TUTORIAL.md](../tutorials/EXECUTE_COMMAND_TUTORIAL.md) - Build custom command
5. Advanced: [WORKSPACE_REFACTORING_TUTORIAL.md](../tutorials/WORKSPACE_REFACTORING_TUTORIAL.md) - Cross-file features

### Path 2: Parser Contributor
1. Start: [CLAUDE.md](../CLAUDE.md) - Project overview
2. Architecture: [CRATE_ARCHITECTURE_GUIDE.md](CRATE_ARCHITECTURE_GUIDE.md) - Component design
3. Testing: [COMPREHENSIVE_TESTING_GUIDE.md](../tutorials/COMPREHENSIVE_TESTING_GUIDE.md) - TDD workflow
4. Performance: [INCREMENTAL_PARSING_GUIDE.md](../how-to/INCREMENTAL_PARSING_GUIDE.md) - Optimization
5. Quality: [PARSER_ROBUSTNESS_IMPROVEMENTS.md](../explanation/PARSER_ROBUSTNESS_IMPROVEMENTS.md) - Hardening

### Path 3: API Documentation Maintainer
1. Standards: [API_DOCUMENTATION_STANDARDS.md](API_DOCUMENTATION_STANDARDS.md) - Requirements
2. Validation: [VALIDATION-149-acceptance-criteria.md](VALIDATION-149-acceptance-criteria.md) - Quality gates
3. Strategy: [MISSING_DOCUMENTATION_GUIDE.md](MISSING_DOCUMENTATION_GUIDE.md) - Enforcement
4. Governance: [SPEC_149_GOVERNANCE.md](SPEC_149_GOVERNANCE.md) - Process
5. Implementation: [adr/ADR_002_API_DOCUMENTATION_INFRASTRUCTURE.md](adr/ADR_002_API_DOCUMENTATION_INFRASTRUCTURE.md) - Design

### Path 4: Debugging Integration
1. Setup: [DAP_USER_GUIDE.md](../tutorials/DAP_USER_GUIDE.md) - Installation and configuration
2. Architecture: [CRATE_ARCHITECTURE_DAP.md](CRATE_ARCHITECTURE_DAP.md) - System design
3. Validation: [DAP_BREAKPOINT_VALIDATION_GUIDE.md](../how-to/DAP_BREAKPOINT_VALIDATION_GUIDE.md) - Testing
4. Analysis: [ISSUE_207_DAP_SPECIFICATION_ANALYSIS.md](ISSUE_207_DAP_SPECIFICATION_ANALYSIS.md) - Protocol details

---

## Navigation Hints

### Finding Information by Type

**"How do I...?"** → [How-to Guides](#how-to-guides-problem-oriented)
- Specific task-oriented instructions with cargo commands
- Example: "How do I optimize imports?" → [IMPORT_OPTIMIZER_GUIDE.md](../how-to/IMPORT_OPTIMIZER_GUIDE.md)

**"What is...?"** → [Reference Documentation](#reference-documentation-information-oriented)
- Precise technical specifications and API contracts
- Example: "What is the cancellation protocol?" → [LSP_CANCELLATION_PROTOCOL_SPECIFICATION.md](LSP_CANCELLATION_PROTOCOL_SPECIFICATION.md)

**"Why does...?"** → [Explanation](#explanation-understanding-oriented)
- Conceptual understanding and design rationale
- Example: "Why use dual indexing?" → [WORKSPACE_NAVIGATION_GUIDE.md](WORKSPACE_NAVIGATION_GUIDE.md)

**"Teach me..."** → [Tutorials](#tutorials-learning-oriented)
- Step-by-step learning experiences
- Example: "Teach me workspace refactoring" → [WORKSPACE_REFACTORING_TUTORIAL.md](../tutorials/WORKSPACE_REFACTORING_TUTORIAL.md)

### Finding Information by Component

**Parser** (`/crates/perl-parser/`):
- Reference: [CRATE_ARCHITECTURE_GUIDE.md](CRATE_ARCHITECTURE_GUIDE.md)
- Performance: [INCREMENTAL_PARSING_GUIDE.md](../how-to/INCREMENTAL_PARSING_GUIDE.md)
- Robustness: [PARSER_ROBUSTNESS_IMPROVEMENTS.md](../explanation/PARSER_ROBUSTNESS_IMPROVEMENTS.md)

**LSP Server** (`/crates/perl-lsp/`):
- Tutorial: [EXECUTE_COMMAND_TUTORIAL.md](../tutorials/EXECUTE_COMMAND_TUTORIAL.md)
- Reference: [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)
- Explanation: [LSP_CRATE_SEPARATION_GUIDE.md](../explanation/LSP_CRATE_SEPARATION_GUIDE.md)

**DAP Server** (`/crates/perl-dap/`):
- Tutorial: [DAP_USER_GUIDE.md](../tutorials/DAP_USER_GUIDE.md)
- Reference: [CRATE_ARCHITECTURE_DAP.md](CRATE_ARCHITECTURE_DAP.md)
- How-to: [DAP_BREAKPOINT_VALIDATION_GUIDE.md](../how-to/DAP_BREAKPOINT_VALIDATION_GUIDE.md)

**Testing** (`/tests/`):
- Tutorial: [COMPREHENSIVE_TESTING_GUIDE.md](../tutorials/COMPREHENSIVE_TESTING_GUIDE.md)
- How-to: [THREADING_CONFIGURATION_GUIDE.md](../how-to/THREADING_CONFIGURATION_GUIDE.md)
- Reference: [COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)

---

## Documentation Quality Standards

All documentation in this project follows comprehensive quality standards:

### Validation Requirements
```bash
# All code examples must pass doctests
cargo test --doc

# API documentation must build without warnings
cargo doc --no-deps --package perl-parser

# Documentation quality gates
cargo test -p perl-parser --test missing_docs_ac_tests
```

### Content Standards
- **Executable Examples**: All code blocks tested via doctests or integration tests
- **Cargo Commands**: Proper package specifications (`cargo test -p perl-parser`)
- **Threading Patterns**: Adaptive threading examples (`RUST_TEST_THREADS=2 cargo test -p perl-lsp`)
- **LSP Workflow Integration**: Parse → Index → Navigate → Complete → Analyze pipeline references
- **Cross-References**: Proper Rust documentation linking (`[`function_name`]`)

### Diataxis Compliance
- **Clear Separation**: No mixing of tutorial/how-to/reference/explanation concerns
- **Consistent Terminology**: Perl LSP, workspace structure, parser/lsp/lexer references
- **Progressive Disclosure**: Learning paths guide users from basics to advanced topics

---

## Contributing to Documentation

When adding new features, update documentation systematically:

1. **Analyze Impact**: Identify affected Diataxis categories
2. **Update Tutorials**: Add step-by-step learning experiences
3. **Enhance How-tos**: Create task-oriented instructions
4. **Revise Reference**: Update API docs and specifications
5. **Expand Explanations**: Add conceptual context

See [API_DOCUMENTATION_STANDARDS.md](API_DOCUMENTATION_STANDARDS.md) for detailed requirements.

---

## Additional Resources

### Project Management
- **[ISSUE_STATUS_2025-11-12.md](ISSUE_STATUS_2025-11-12.md)** - Complete issue analysis and priorities
- **[CURRENT_STATUS.md](../project/CURRENT_STATUS.md)** - Real-time project health dashboard
- **[STABILITY.md](STABILITY.md)** - API stability guarantees

### Archived Documentation
- **[archive/](archive/)** - Historical documentation for reference

### Benchmark Reports
- **[benchmarks/](benchmarks/)** - Performance analysis and cross-language comparisons

---

## Quick Command Reference

```bash
# Build & Install
cargo build -p perl-lsp --release        # LSP server
cargo install perl-lsp                   # Install globally

# Testing (Adaptive Threading)
cargo test                               # All tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp  # LSP with thread constraints

# Documentation Validation
cargo test --doc                         # Validate doctests
cargo doc --no-deps --package perl-parser   # Generate docs
cargo test -p perl-parser --test missing_docs_ac_tests  # Quality gates

# Development Server (Hot-Reload)
cd xtask && cargo run --no-default-features -- dev --watch

# Performance Testing
cd xtask && cargo run --no-default-features -- optimize-tests

# Highlight Testing (Tree-Sitter)
cd xtask && cargo run --no-default-features -- highlight
```

---

**Need Help?** If you can't find what you need:
1. Check the [Feature Index](#feature-index) for your specific feature
2. Follow a [Progressive Learning Path](#progressive-learning-paths) for your role
3. Use [Navigation Hints](#navigation-hints) to find information by type
4. Review [CLAUDE.md](../CLAUDE.md) for project-wide context

**Found Missing Documentation?** See [MISSING_DOCUMENTATION_GUIDE.md](MISSING_DOCUMENTATION_GUIDE.md) for contribution guidelines.
