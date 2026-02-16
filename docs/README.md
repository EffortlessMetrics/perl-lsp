# Perl LSP Documentation

Welcome to the comprehensive documentation for Perl LSP v1.0. This index will help you find the right documentation for your needs.

## 🚀 Quick Start

### For New Users
- **[Getting Started](GETTING_STARTED.md)** - Installation and first steps
- **[Installation Guide](QUICK_START.md)** - Detailed installation instructions
- **[FAQ](FAQ.md)** - Frequently asked questions
- **[Editor Setup](EDITOR_SETUP.md)** - Configure your favorite editor

### For Developers
- **[Contributing Guide](../CONTRIBUTING.md)** - Development workflow and contribution process
- **[Development Guide](DEVELOPMENT.md)** - Setting up development environment
- **[API Documentation](API_DOCUMENTATION_STANDARDS.md)** - Complete API reference

## 📚 Core Documentation

### Project Overview
- **[Architecture Overview](ARCHITECTURE_OVERVIEW.md)** - System design and components
- **[Current Status](CURRENT_STATUS.md)** - Real-time metrics and coverage
- **[Roadmap](ROADMAP.md)** - Future plans and development roadmap
- **[Release Notes](../RELEASE_NOTES.md)** - v1.0 release details and changelog

### User Documentation
- **[Commands Reference](COMMANDS_REFERENCE.md)** - Build, test, and development commands
- **[Troubleshooting](TROUBLESHOOTING.md)** - Common issues and solutions
- **[Migration Guide](UPGRADE_v0.8_to_v1.0.md)** - Upgrading from previous versions

### Feature Documentation
- **[LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md)** - Language Server Protocol details
- **[DAP User Guide](DAP_USER_GUIDE.md)** - Debug Adapter Protocol setup and usage
- **[Workspace Navigation Guide](WORKSPACE_NAVIGATION_GUIDE.md)** - Cross-file features and navigation
- **[Import Optimizer Guide](IMPORT_OPTIMIZER_GUIDE.md)** - Import analysis and optimization

## 🔧 Technical Documentation

### Parser & Language Features
- **[Builtin Function Parsing](BUILTIN_FUNCTION_PARSING.md)** - Enhanced parsing for builtin functions
- **[Heredoc Implementation](HEREDOC_IMPLEMENTATION.md)** - Heredoc parsing and special contexts
- **[Incremental Parsing Guide](INCREMENTAL_PARSING_GUIDE.md)** - Performance and implementation details
- **[Variable Resolution Guide](VARIABLE_RESOLUTION_GUIDE.md)** - Scope analysis system

### Performance & Optimization
- **[Performance SLO](PERFORMANCE_SLO.md)** - Performance characteristics and benchmarks
- **[Performance Monitoring](PERFORMANCE_MONITORING.md)** - Performance tracking and analysis
- **[Threading Configuration Guide](THREADING_CONFIGURATION_GUIDE.md)** - Concurrency management
- **[Rope Integration Guide](ROPE_INTEGRATION_GUIDE.md)** - Document management system

### Security & Reliability
- **[Security Development Guide](SECURITY_DEVELOPMENT_GUIDE.md)** - Enterprise security practices
- **[Error Handling Strategy](ERROR_HANDLING_STRATEGY.md)** - Defensive programming principles
- **[Supply Chain Security](SUPPLY_CHAIN_SECURITY.md)** - Dependency security and management

## 🧪 Testing & Quality

### Testing Documentation
- **[Test Infrastructure Guide](TEST_INFRASTRUCTURE_GUIDE.md)** - Testing framework and tools
- **[Mutation Testing Methodology](MUTATION_TESTING_METHODOLOGY.md)** - Quality assurance with mutation testing
- **[Coverage Documentation](COVERAGE.md)** - Test coverage analysis and reporting

### Quality Assurance
- **[API Documentation Standards](API_DOCUMENTATION_STANDARDS.md)** - Documentation requirements and standards
- **[Quality Surfaces](QUALITY_SURFACES.md)** - Quality metrics and validation
- **[Production Readiness Report](PRODUCTION_READINESS_REPORT.md)** - Production validation results

## 🏗️ Architecture & Implementation

### Core Components
- **[Crate Architecture Guide](CRATE_ARCHITECTURE_GUIDE.md)** - System design and components
- **[LSP Providers Reference](LSP_PROVIDERS_REFERENCE.md)** - LSP feature implementation
- **[Semantic Analyzer Reference](SCOPE_ANALYZER_REFERENCE.md)** - Semantic analysis implementation

### Advanced Features
- **[Workspace Refactoring Guide](WORKSPACE_REFACTORING_GUIDE.md)** - Cross-file refactoring capabilities
- **[Execute Command Configuration](EXECUTE_COMMAND_CONFIGURATION_GUIDE.md)** - LSP execute command setup
- **[Cancellation Architecture Guide](CANCELLATION_ARCHITECTURE_GUIDE.md)** - LSP cancellation system

## 📖 Reference Documentation

### Configuration & Setup
- **[Configuration Guide](CONFIG.md)** - Configuration options and setup
- **[File Completion Guide](FILE_COMPLETION_GUIDE.md)** - Path completion and security
- **[Source Threading Guide](SOURCE_THREADING_GUIDE.md)** - Comment documentation extraction

### Standards & Policies
- **[Stability Statement](STABILITY_STATEMENT_v1.0.md)** - API stability guarantees
- **[SemVer Workflow](SEMVER_WORKFLOW.md)** - Semantic versioning process
- **[Documentation Guide](DOCUMENTATION_GUIDE.md)** - Documentation standards and practices

## 🔍 Specialized Documentation

### Development Tools
- **[CI Documentation](CI.md)** - Continuous integration and automation
- **[Dependency Management](DEPENDENCY_MANAGEMENT.md)** - Dependency update policies and procedures
- **[Publishing Guide](PUBLISHING.md)** - Release and publishing process

### Debugging & Analysis
- **[Debugging Guide](DEBUGGING.md)** - Debugging techniques and tools
- **[Performance Regression Reports](PERFORMANCE_REGRESSION_REPORT_PR153.md)** - Performance analysis and improvements
- **[Edge Cases Documentation](EDGE_CASES.md)** - Edge case handling and solutions

## 📋 Documentation by Role

### For End Users
1. **[Getting Started](GETTING_STARTED.md)** - Start here
2. **[Installation Guide](QUICK_START.md)** - Install perl-lsp
3. **[Editor Setup](EDITOR_SETUP.md)** - Configure your editor
4. **[FAQ](FAQ.md)** - Common questions
5. **[Troubleshooting](TROUBLESHOOTING.md)** - Get help

### For Plugin/Extension Developers
1. **[LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md)** - LSP protocol details
2. **[Commands Reference](COMMANDS_REFERENCE.md)** - Available commands
3. **[API Documentation](API_DOCUMENTATION_STANDARDS.md)** - API reference
4. **[Configuration Guide](CONFIG.md)** - Configuration options

### For Contributors
1. **[Contributing Guide](../CONTRIBUTING.md)** - How to contribute
2. **[Development Guide](DEVELOPMENT.md)** - Development setup
3. **[Architecture Overview](ARCHITECTURE_OVERVIEW.md)** - System architecture
4. **[Test Infrastructure Guide](TEST_INFRASTRUCTURE_GUIDE.md)** - Testing framework
5. **[Security Development Guide](SECURITY_DEVELOPMENT_GUIDE.md)** - Security practices

### For Maintainers
1. **[Release Process](../CONTRIBUTING.md#release-process)** - Release procedures
2. **[SemVer Workflow](SEMVER_WORKFLOW.md)** - Version management
3. **[Production Readiness Report](PRODUCTION_READINESS_REPORT.md)** - Production validation
4. **[Security Policy](../SECURITY.md)** - Security procedures

## 🔄 Documentation Maintenance

### Documentation Standards
- All documentation follows the [Diataxis framework](DOCUMENTATION_GUIDE.md)
- API documentation enforced by `#![warn(missing_docs)]`
- Regular reviews and updates scheduled quarterly
- Community contributions welcome and encouraged

### Getting Help
- **Issues**: Report documentation issues on GitHub
- **Discussions**: Use GitHub Discussions for questions
- **Contributions**: See [Contributing Guide](../CONTRIBUTING.md) for how to help improve documentation

---

**Last Updated**: 2026-02-13  
**Version**: v1.0.0  
**Documentation Status**: Production Ready ✅

For the most up-to-date information, check the [Current Status](CURRENT_STATUS.md) page.