# Documentation Guide

This guide helps you find the right documentation for your needs.

## Primary Documentation

### For Users

1. **[README.md](../README.md)** - Project overview, quick start, features
2. **[LSP_DOCUMENTATION.md](LSP_DOCUMENTATION.md)** 🚀 - Language Server Protocol guide
3. **[EDGE_CASES.md](EDGE_CASES.md)** - Comprehensive edge case handling guide
4. **[CLAUDE.md](../CLAUDE.md)** - Development commands and architecture

### For Contributors

1. **[CONTRIBUTING.md](../CONTRIBUTING.md)** - Contribution guidelines
2. **[DEVELOPMENT.md](../DEVELOPMENT.md)** - Development setup and workflow
3. **[ARCHITECTURE.md](../ARCHITECTURE.md)** - System architecture details (updated)

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
- README.md (updated with LSP)
- CLAUDE.md (updated with LSP commands)
- LSP_DOCUMENTATION.md (NEW)
- EDGE_CASES.md (consolidated)
- HEREDOC_IMPLEMENTATION.md
- SLASH_DISAMBIGUATION.md
- ARCHITECTURE.md (updated with v3 parser and LSP)
- CONTRIBUTING.md
- ROADMAP.md (updated with current status)

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