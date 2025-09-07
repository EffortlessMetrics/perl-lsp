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

### **How-to Guides** (Problem-oriented) 🔧

5. **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Complete build/test command solutions
6. **Editor Configuration Guides** - Specific editor setup instructions
7. **[FILE_COMPLETION_GUIDE.md](FILE_COMPLETION_GUIDE.md)** - Enterprise-secure path completion
8. **[IMPORT_OPTIMIZER_GUIDE.md](IMPORT_OPTIMIZER_GUIDE.md)** - Advanced code actions

### **Reference** (Information-oriented) 📊

9. **[LSP_ACTUAL_STATUS.md](../LSP_ACTUAL_STATUS.md)** - Comprehensive LSP feature status with validation results
10. **[CLAUDE.md](../CLAUDE.md)** - Development commands, crate architecture, and performance targets
11. **[STABILITY.md](STABILITY.md)** - API stability guarantees and versioning policy
12. **[WORKSPACE_TEST_REPORT.md](../WORKSPACE_TEST_REPORT.md)** - Current workspace configuration status and build reliability
13. **[CHANGELOG.md](../CHANGELOG.md)** - Complete version history and feature tracking
14. **[BENCHMARK_RESULTS.md](../BENCHMARK_RESULTS.md)** - Performance benchmark results.

### **Explanations** (Understanding-oriented) 🧠

14. **[ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md)** - System design and component relationships
15. **[INCREMENTAL_PARSING_GUIDE.md](INCREMENTAL_PARSING_GUIDE.md)** - Performance architecture and design decisions
16. **[ROPE_INTEGRATION_GUIDE.md](ROPE_INTEGRATION_GUIDE.md)** - Document management system explained
17. **[EDGE_CASES.md](EDGE_CASES.md)** - Why Perl parsing edge cases are challenging

### For Contributors

18. **[CONTRIBUTING.md](../CONTRIBUTING.md)** - Contribution guidelines (How-to)
19. **[DEBUGGING.md](DEBUGGING.md)** - Development troubleshooting (How-to)  
20. **[ARCHITECTURE.md](../ARCHITECTURE.md)** - Deep technical architecture (Explanation)

## Specialized Documentation

### LSP Implementation
- **[LSP_DOCUMENTATION.md](LSP_DOCUMENTATION.md)** - Complete LSP guide
- **[../crates/perl-parser/README_LSP.md](../crates/perl-parser/README_LSP.md)** - Quick LSP reference
- **[../crates/perl-parser/examples/](../crates/perl-parser/examples/)** - LSP demos and examples

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