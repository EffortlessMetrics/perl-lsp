# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

**Latest Release**: v0.8.9 GA - Comprehensive PR Workflow Integration with Production-Stable AST Generation  
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md)

## Project Overview

This repository contains **five published crates** forming a complete Perl parsing ecosystem:

### Published Crates (v0.8.9 GA)

1. **perl-parser** (`/crates/perl-parser/`) ⭐ **MAIN CRATE**
   - Native recursive descent parser with ~100% Perl 5 syntax coverage
   - 4-19x faster than legacy implementations (1-150 µs parsing)
   - True incremental parsing with <1ms LSP updates
   - Production-ready Rope integration for UTF-16/UTF-8 position conversion
   - Enhanced workspace navigation and PR workflow integration

2. **perl-lsp** (`/crates/perl-lsp/`) ⭐ **LSP BINARY**
   - Standalone Language Server binary with production-grade CLI
   - Clean separation from parser logic for improved maintainability
   - Works with VSCode, Neovim, Emacs, and all LSP-compatible editors

3. **perl-lexer** (`/crates/perl-lexer/`)
   - Context-aware tokenizer with mode-based lexing
   - Handles slash disambiguation and Unicode identifiers

4. **perl-corpus** (`/crates/perl-corpus/`)
   - Comprehensive test corpus with property-based testing infrastructure

5. **perl-parser-pest** (`/crates/perl-parser-pest/`) ⚠️ **LEGACY**
   - Pest-based parser (v2 implementation), marked as legacy

## Quick Start

### Installation
```bash
# Install LSP server
cargo install perl-lsp

# Or quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash

# Or Homebrew (macOS)
brew tap tree-sitter-perl/tap && brew install perl-lsp
```

### Usage
```bash
# Run LSP server (for editors)
perl-lsp --stdio

# Build parser from source
cargo build -p perl-parser --release

# Run tests
cargo test
```

## Key Features

- **~100% Perl Syntax Coverage**: Handles all modern Perl constructs including edge cases
- **Production-Ready LSP Server**: ~85% of LSP features functional with comprehensive workspace support
- **Enhanced Incremental Parsing**: <1ms updates with 70-99% node reuse efficiency
- **Comprehensive Testing**: 100% test pass rate (195 library tests, 33 LSP E2E tests, 19 DAP tests)
- **Unicode-Safe**: Full Unicode identifier and emoji support with proper UTF-8/UTF-16 handling
- **Enterprise Security**: Path traversal prevention, file completion safeguards

## Architecture

### Crate Structure
- **Core Parser**: `/crates/perl-parser/` - parsing logic, LSP providers, Rope implementation
- **LSP Binary**: `/crates/perl-lsp/` - standalone server, CLI interface, protocol handling
- **Lexer**: `/crates/perl-lexer/` - tokenization, Unicode support
- **Test Corpus**: `/crates/perl-corpus/` - comprehensive test suite

### Parser Versions
- **v3 (Native)** ⭐ **RECOMMENDED**: ~100% coverage, 4-19x faster, production incremental parsing
- **v2 (Pest)**: ~99.996% coverage, legacy but stable
- **v1 (C-based)**: ~95% coverage, benchmarking only

## Essential Commands

**AI tools can run bare `cargo build` and `cargo test`** - the `.cargo/config.toml` ensures correct behavior.

### Build & Install
```bash
# Build main components
cargo build -p perl-lsp --release        # LSP server
cargo build -p perl-parser --release     # Parser library

# Install globally
cargo install perl-lsp                   # From crates.io
cargo install --path crates/perl-lsp     # From source
```

### Testing
```bash
cargo test                               # All tests
cargo test -p perl-parser               # Parser tests
cargo test -p perl-lsp                  # LSP integration tests
cargo xtask corpus                       # Comprehensive integration
```

### Development
```bash
cargo xtask check --all                 # Format + clippy
cargo bench                            # Performance benchmarks
perl-lsp --stdio --log                 # Debug LSP server
```

## Documentation

- **[Commands Reference](docs/COMMANDS_REFERENCE.md)** - Comprehensive build/test commands
- **[LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture
- **[Incremental Parsing Guide](docs/INCREMENTAL_PARSING_GUIDE.md)** - Performance and implementation
- **[Architecture Overview](docs/ARCHITECTURE_OVERVIEW.md)** - System design and components
- **[Development Guidelines](docs/DEBUGGING.md)** - Contributing and troubleshooting

### Specialized Guides
- **[LSP Crate Separation](docs/LSP_CRATE_SEPARATION_GUIDE.md)** - v0.8.9 architectural improvements
- **[Workspace Navigation](docs/WORKSPACE_NAVIGATION_GUIDE.md)** - Enhanced cross-file features
- **[Rope Integration](docs/ROPE_INTEGRATION_GUIDE.md)** - Document management system
- **[Source Threading](docs/SOURCE_THREADING_GUIDE.md)** - Comment documentation extraction
- **[Position Tracking](docs/POSITION_TRACKING_GUIDE.md)** - UTF-16/UTF-8 position mapping
- **[Variable Resolution](docs/VARIABLE_RESOLUTION_GUIDE.md)** - Scope analysis system
- **[File Completion](docs/FILE_COMPLETION_GUIDE.md)** - Enterprise-secure path completion
- **[Import Optimizer](docs/IMPORT_OPTIMIZER_GUIDE.md)** - Advanced code actions

## Performance Targets ✅ **EXCEEDED**

| Operation | Target | Achieved | Status |
|-----------|--------|----------|--------|
| Simple Edits | <100µs | 65µs avg | ✅ Excellent |
| Moderate Edits | <500µs | 205µs avg | ✅ Very Good |
| Large Documents (100 stmt) | <1ms | 538µs avg | ✅ Good |
| Node Reuse Efficiency | ≥70% | 99.7% peak | ✅ Exceptional |
| Statistical Consistency | <1.0 CV | 0.6 CV | ✅ Excellent |
| Incremental Success Rate | ≥95% | 100% | ✅ Perfect |

## Current Status (v0.8.9)

✅ **Production Ready**:
- 100% test pass rate across all components
- Zero clippy warnings, consistent formatting
- Enterprise-grade LSP server with comprehensive features
- Production-stable incremental parsing with statistical validation
- Enhanced workspace navigation and PR workflow integration

**LSP Features (~85% functional)**:
- ✅ Syntax checking, diagnostics, completion, hover
- ✅ Workspace symbols, rename, code actions, semantic tokens
- ✅ Enhanced call hierarchy, go-to-definition, find references
- ✅ File path completion with enterprise security
- ✅ Debug Adapter Protocol (DAP) support

**Recent Enhancements (v0.8.9)**:
- ✅ Comprehensive S-expression generation with 50+ operators
- ✅ Enhanced AST traversal including ExpressionStatement support
- ✅ Production-ready workspace indexing and cross-file analysis
- ✅ Advanced code actions with parameter threshold validation
- ✅ Statistical performance testing infrastructure

## Security Development Guidelines (PR #44)

This project demonstrates **enterprise-grade security practices** in its test infrastructure. All contributors should follow these security development standards:

### Secure Authentication Implementation

When implementing authentication systems (including test scenarios), use production-grade security:

```perl
use Crypt::PBKDF2;

# OWASP 2021 compliant PBKDF2 configuration
sub get_pbkdf2_instance {
    return Crypt::PBKDF2->new(
        hash_class => 'HMACSHA2',      # SHA-2 family for cryptographic strength
        hash_args => { sha_size => 256 }, # SHA-256 for collision resistance
        iterations => 100_000,          # 100k iterations (OWASP 2021 minimum)
        salt_len => 16,                 # 128-bit cryptographically random salt
    );
}

sub authenticate_user {
    my ($username, $password) = @_;
    my $users = load_users();
    my $pbkdf2 = get_pbkdf2_instance();
    
    foreach my $user (@$users) {
        if ($user->{name} eq $username) {
            # Constant-time validation prevents timing attacks
            if ($pbkdf2->validate($user->{password_hash}, $password)) {
                return $user;
            }
        }
    }
    return undef;  # Authentication failed
}
```

### Security Requirements

✅ **Cryptographic Standards**: Use OWASP 2021 compliant algorithms and parameters  
✅ **Timing Attack Prevention**: Implement constant-time comparisons for authentication  
✅ **No Plaintext Storage**: Hash all passwords immediately, never store in clear text  
✅ **Secure Salt Generation**: Use cryptographically secure random salts (≥16 bytes)  
✅ **Input Validation**: Sanitize and validate all user inputs  
✅ **Path Security**: Use canonical paths with workspace boundary validation  

### Security Testing Requirements

All security-related code must include comprehensive tests:

- **Authentication Security**: Test password hashing, validation, and timing consistency
- **Input Validation**: Verify proper sanitization and boundary checking
- **File Access Security**: Test path traversal prevention and workspace boundaries
- **Error Message Security**: Ensure no sensitive information disclosure

### Security Review Process

- All authentication/security code changes require security review
- Test implementations serve as security best practice examples  
- Document security assumptions and threat models in code comments
- Use the security implementation in PR #44 as the reference standard

## Contributing

1. **Parser improvements** → `/crates/perl-parser/src/`
2. **LSP features** → `/crates/perl-parser/src/` (provider logic)
3. **CLI enhancements** → `/crates/perl-lsp/src/` (binary interface)
4. **Testing** → Use existing comprehensive test infrastructure
5. **Security features** → Follow PR #44 PBKDF2 implementation standards

Run `cargo xtask check --all` before committing. All tests must pass with zero warnings.