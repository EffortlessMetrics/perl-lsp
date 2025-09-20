# Documentation Guide - **Diataxis Framework Applied**

This guide helps you find the right documentation for your needs, organized using the **Diataxis framework** for optimal user experience.

## **Diataxis Framework Overview**

- **Tutorials** (Learning-oriented): Hands-on guidance for beginners
- **How-to Guides** (Problem-oriented): Step-by-step solutions to specific tasks
- **Explanations** (Understanding-oriented): Design concepts and architectural decisions  
- **Reference** (Information-oriented): Comprehensive specifications and lookups

## Primary Documentation

### **Tutorials** (Learning-oriented) 📚

1. **[README.md](../README.md)** - Project overview with guided quick start
2. **Getting Started Section** - First-time user orientation with perl-lsp installation
3. **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)** - Step-by-step LSP server setup
4. **Editor Integration Tutorials** - VSCode, Neovim, Emacs setup guides
5. **[WORKSPACE_REFACTORING_TUTORIAL.md](WORKSPACE_REFACTORING_TUTORIAL.md)** - Cross-file refactoring walkthrough

### **How-to Guides** (Problem-oriented) 🔧

1. **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Complete build/test command solutions ⭐ **Updated with PR #140 Revolutionary Performance**
2. **[THREADING_CONFIGURATION_GUIDE.md](THREADING_CONFIGURATION_GUIDE.md)** - Adaptive timeout methodology and CI configuration ⭐ **Revolutionary 5000x improvements**
3. **Editor Configuration Guides** - Specific editor setup instructions
4. **[FILE_COMPLETION_GUIDE.md](FILE_COMPLETION_GUIDE.md)** - Enterprise-secure path completion
5. **[IMPORT_OPTIMIZER_GUIDE.md](IMPORT_OPTIMIZER_GUIDE.md)** - Advanced code actions
6. **[WORKSPACE_REFACTORING_GUIDE.md](WORKSPACE_REFACTORING_GUIDE.md)** - Practical refactoring operations

### **Reference** (Information-oriented) 📊

1. **[LSP_ACTUAL_STATUS.md](../LSP_ACTUAL_STATUS.md)** - Comprehensive LSP feature status with validation results
2. **[CLAUDE.md](../CLAUDE.md)** - Development commands, crate architecture, and performance targets ⭐ **Updated with PR #140 metrics**
3. **[BENCHMARK_FRAMEWORK.md](BENCHMARK_FRAMEWORK.md)** - Performance benchmarking system ⭐ **Revolutionary 5000x performance achievements**
4. **[STABILITY.md](STABILITY.md)** - API stability guarantees and versioning policy
5. **[WORKSPACE_TEST_REPORT.md](../WORKSPACE_TEST_REPORT.md)** - Current workspace configuration status and build reliability
6. **[CHANGELOG.md](../CHANGELOG.md)** - Complete version history and feature tracking
7. **[WORKSPACE_REFACTOR_API_REFERENCE.md](WORKSPACE_REFACTOR_API_REFERENCE.md)** - API-level refactoring details

### **Explanations** (Understanding-oriented) 🧠

1. **[ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md)** - System design and component relationships ⭐ **Updated with Revolutionary Adaptive Timeout System**
2. **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture ⭐ **Enhanced with 5000x performance improvements**
3. **[LSP_DEVELOPMENT_GUIDE.md](LSP_DEVELOPMENT_GUIDE.md)** - Advanced LSP development ⭐ **Revolutionary test harness features**
4. **[INCREMENTAL_PARSING_GUIDE.md](INCREMENTAL_PARSING_GUIDE.md)** - Performance architecture and design decisions
5. **[ROPE_INTEGRATION_GUIDE.md](ROPE_INTEGRATION_GUIDE.md)** - Document management system explained
6. **[EDGE_CASES.md](EDGE_CASES.md)** - Why Perl parsing edge cases are challenging
7. **[MODERN_ARCHITECTURE.md](MODERN_ARCHITECTURE.md)** - Contemporary design evolution
8. **[ADR_001_AGENT_ARCHITECTURE.md](ADR_001_AGENT_ARCHITECTURE.md)** - Agent specialization architectural decisions ⭐ **NEW: PR #153 Agent Ecosystem**

### For Contributors

1. **[CONTRIBUTING.md](../CONTRIBUTING.md)** - Contribution guidelines (How-to)
2. **[DEBUGGING.md](DEBUGGING.md)** - Development troubleshooting (How-to)
3. **[ARCHITECTURE.md](../ARCHITECTURE.md)** - Deep technical architecture (Explanation)

## Specialized Documentation

### Revolutionary LSP Implementation (PR #140 Enhanced)
- **[LSP_DOCUMENTATION.md](LSP_DOCUMENTATION.md)** - Complete LSP guide
- **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)** - Revolutionary performance improvements ⭐ **5000x faster behavioral tests**
- **[LSP_DEVELOPMENT_GUIDE.md](LSP_DEVELOPMENT_GUIDE.md)** - Enhanced test harness features ⭐ **4700x faster user stories**
- **[../crates/perl-parser/README_LSP.md](../crates/perl-parser/README_LSP.md)** - Quick LSP reference
- **[../crates/perl-parser/examples/](../crates/perl-parser/examples/)** - LSP demos and examples

### Test-Driven Development (*Diataxis: How-to & Reference*)
- **TestGenerator API**: Available in `perl-parser` crate with auto-detection capabilities
- **Auto-detect Test Expectations**: Analyzes subroutine ASTs to detect expected return values
- **Cross-Framework Support**: TestMore, Test2V0, TestSimple, TestClass framework generation
- **Performance Testing**: AST complexity estimation for automated performance test creation
- **Tutorial**: See README.md "Test Generation" section for getting started
- **Reference**: See CLAUDE.md "TestGenerator Commands" section for comprehensive usage

### Heredoc Implementation
- **[HEREDOC_IMPLEMENTATION.md](HEREDOC_IMPLEMENTATION.md)** - Core heredoc parsing
- **[SLASH_DISAMBIGUATION.md](SLASH_DISAMBIGUATION.md)** - Slash operator handling
- **[MULTI_PHASE_PARSING.md](MULTI_PHASE_PARSING.md)** - Multi-phase parsing approach

### Edge Case Handling
- **[EDGE_CASES.md](EDGE_CASES.md)** ⭐ - **Primary reference** (consolidated)
- Legacy files (for historical context only):
  - EDGE_CASE_IMPLEMENTATION_PLAN.md
  - EDGE_CASE_SOLUTION_COMPLETE.md
  - EDGE_CASE_TEST_COVERAGE.md
  - Other edge case files

### Technical Deep Dives
- **[COMPLETE_PARSER_ARCHITECTURE.md](COMPLETE_PARSER_ARCHITECTURE.md)** - Full parser design
- **[ITERATIVE_PARSER.md](ITERATIVE_PARSER.md)** - Iterative parsing approach
- **[UNICODE_SUPPORT.md](UNICODE_SUPPORT.md)** - Unicode handling

## Documentation Status

### Active (Current)
- README.md (Updated for v0.8.9, including crate separation, incremental parsing, and latest features)
- ROADMAP.md (NEW - comprehensive future vision)
- FEATURE_ROADMAP.md (NEW - detailed feature plans)
- ROADMAP_2025.md (NEW - this year's focus)
- CLAUDE.md (Updated with latest commands and project status)
- LSP_DOCUMENTATION.md (Updated with all implemented LSP features)
- EDGE_CASES.md (consolidated)
- HEREDOC_IMPLEMENTATION.md
- SLASH_DISAMBIGUATION.md
- ARCHITECTURE.md (Updated with LSP crate separation and latest architectural patterns)
- CONTRIBUTING.md
- QUICK_REFERENCE.md (NEW - one-page guide)
- BENCHMARK_RESULTS.md (Populated with latest performance data)

### Revolutionary Performance Improvements (PR #140) ⭐ **TRANSFORMATIONAL**

PR #140 delivers transformative performance optimizations achieving unprecedented improvements:

**Performance Achievement Summary**:
- **LSP behavioral tests**: 1560s+ → 0.31s (**5000x faster**, Transformational)
- **User story tests**: 1500s+ → 0.32s (**4700x faster**, Revolutionary)
- **Individual workspace tests**: 60s+ → 0.26s (**230x faster**, Game-changing)
- **Overall test suite**: 60s+ → <10s (**6x faster**, Production-ready)
- **CI reliability**: 100% pass rate (was ~55% due to timeouts)

**Updated Documentation Cross-References**:
- **[THREADING_CONFIGURATION_GUIDE.md](THREADING_CONFIGURATION_GUIDE.md)** - Adaptive timeout methodology
- **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Revolutionary CI testing guidance  
- **[BENCHMARK_FRAMEWORK.md](BENCHMARK_FRAMEWORK.md)** - 5000x performance validation
- **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)** - Enhanced test reliability
- **[LSP_DEVELOPMENT_GUIDE.md](LSP_DEVELOPMENT_GUIDE.md)** - Revolutionary test harness
- **[ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md)** - Adaptive timeout system design

### Historical (Reference Only)
- Individual edge case files (superseded by EDGE_CASES.md)
- Migration plans and interim solutions
- Implementation drafts

## Quick Reference

| What You Need | Where to Look |
|---------------|---------------|
| Getting started | README.md |
| Edge case handling | EDGE_CASES.md |
| Development commands | CLAUDE.md |
| Parser architecture | ARCHITECTURE.md |
| Heredoc details | HEREDOC_IMPLEMENTATION.md |
| Contributing | CONTRIBUTING.md |

## Updating Documentation

When updating docs:
1. Update the primary file (e.g., EDGE_CASES.md)
2. Ensure README.md links are current
3. Update this guide if adding new docs
4. Mark old docs as historical/deprecated