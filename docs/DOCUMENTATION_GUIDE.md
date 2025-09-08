# Documentation Guide - **Diataxis Framework Applied**

This guide helps you find the right documentation for your needs, organized using the **Diataxis framework** for optimal user experience.

## **Diataxis Framework Overview**

- **Tutorials** (Learning-oriented): Hands-on guidance for beginners
- **How-to Guides** (Problem-oriented): Step-by-step solutions to specific tasks
- **Explanations** (Understanding-oriented): Design concepts and architectural decisions  
- **Reference** (Information-oriented): Comprehensive specifications and lookups

## Primary Documentation

### **Tutorials** (Learning-oriented) 📚

*Hands-on guidance for beginners to learn tree-sitter-perl step-by-step*

1. **[README.md](../README.md)** - Project overview with guided quick start (*Diataxis: Tutorial* sections)
2. **Getting Started Section** - First-time user orientation with perl-lsp installation
3. **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)** - Step-by-step LSP server setup
4. **Editor Integration Tutorials** - VSCode, Neovim, Emacs setup guides
5. **Dual-Scanner Corpus Comparison Tutorial** (xtask corpus) - Learning to validate parser accuracy

### **How-to Guides** (Problem-oriented) 🔧

*Step-by-step solutions to specific development tasks*

5. **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Complete build/test command solutions
6. **Editor Configuration Guides** - Specific editor setup instructions (VSCode, Neovim, Emacs)
7. **[FILE_COMPLETION_GUIDE.md](FILE_COMPLETION_GUIDE.md)** - Enterprise-secure path completion setup
8. **[IMPORT_OPTIMIZER_GUIDE.md](IMPORT_OPTIMIZER_GUIDE.md)** - Advanced code actions configuration
9. **Performance Optimization Workflow** - How to run benchmarks and optimize test speed
10. **Development Server Setup** - How to use file watching and LSP hot-reload

### **Reference** (Information-oriented) 📊

*Comprehensive specifications and lookup tables for developers*

11. **[LSP_ACTUAL_STATUS.md](../LSP_ACTUAL_STATUS.md)** - Complete LSP feature status with validation results
12. **[CLAUDE.md](../CLAUDE.md)** - Development commands, crate architecture, and performance targets
13. **[BENCHMARK_RESULTS.md](../BENCHMARK_RESULTS.md)** - Detailed performance benchmark results and analysis
14. **[STABILITY.md](STABILITY.md)** - API stability guarantees and versioning policy
15. **[WORKSPACE_TEST_REPORT.md](../WORKSPACE_TEST_REPORT.md)** - Current workspace configuration and build reliability
16. **[CHANGELOG.md](../CHANGELOG.md)** - Complete version history and feature tracking
17. **Published Crate Documentation** - perl-parser, perl-lsp, perl-lexer API references
18. **Performance Benchmarks Matrix** - Cross-language comparison tables

### **Explanations** (Understanding-oriented) 🧠

*Design concepts, architectural decisions, and deep understanding*

19. **[ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md)** - System design and component relationships
20. **[INCREMENTAL_PARSING_GUIDE.md](INCREMENTAL_PARSING_GUIDE.md)** - Performance architecture and design decisions
21. **[ROPE_INTEGRATION_GUIDE.md](ROPE_INTEGRATION_GUIDE.md)** - Document management system concepts
22. **[EDGE_CASES.md](EDGE_CASES.md)** - Why Perl parsing edge cases are challenging
23. **Parser Design Philosophy** - Why recursive descent vs. Pest-based approaches
24. **LSP Architecture Decisions** - Protocol compliance, fallback mechanisms, error recovery
25. **Performance Trade-offs** - Memory usage vs. parsing speed, benchmark methodology

### For Contributors

*Development-focused documentation organized by Diataxis categories*

26. **[CONTRIBUTING.md](../CONTRIBUTING.md)** - Contribution guidelines (*How-to*)
27. **[DEBUGGING.md](DEBUGGING.md)** - Development troubleshooting (*How-to*)  
28. **[ARCHITECTURE.md](../ARCHITECTURE.md)** - Deep technical architecture (*Explanation*)
29. **Security Development Practices** - Enterprise-grade security standards (*How-to*)
30. **Code Review Guidelines** - Quality assurance processes (*Reference*)

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

## Quick Reference by Diataxis Category

| Need | Category | Where to Look |
|------|----------|---------------|
| **Getting started** | *Tutorial* | README.md, LSP_IMPLEMENTATION_GUIDE.md |
| **Solve specific problem** | *How-to* | COMMANDS_REFERENCE.md, Editor guides |
| **Look up information** | *Reference* | LSP_ACTUAL_STATUS.md, CLAUDE.md, BENCHMARK_RESULTS.md |
| **Understand design** | *Explanation* | ARCHITECTURE.md, EDGE_CASES.md, performance trade-offs |
| **Edge case handling** | *Explanation* | EDGE_CASES.md |
| **Development commands** | *Reference* | CLAUDE.md |
| **Contributing** | *How-to* | CONTRIBUTING.md |

## Updating Documentation

When updating docs:
1. Update the primary file (e.g., EDGE_CASES.md)
2. Ensure README.md links are current
3. Update this guide if adding new docs
4. Mark old docs as historical/deprecated