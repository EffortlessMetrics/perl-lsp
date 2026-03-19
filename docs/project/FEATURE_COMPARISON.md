# Feature Comparison: Perl Language Servers

A factual comparison of the three actively maintained Perl language servers.

| Server | Language | Parser | Latest Release |
|--------|----------|--------|----------------|
| **perl-lsp** | Rust | Recursive descent (v3) | 0.12.0 |
| **PLS** | Perl | PPI | 0.906 (Aug 2025) |
| **Perl::LanguageServer** | Perl | Compiler::Lexer | 2.6.2 (Dec 2023) |

## LSP Feature Matrix

Legend: **Yes** = implemented, **No** = not implemented, **Partial** = limited support

### Core Editing Features

| Feature | perl-lsp | PLS | Perl::LanguageServer |
|---------|----------|-----|----------------------|
| Completion | Yes | Yes | No |
| Signature help | Yes | Yes | Yes (call signatures) |
| Hover (documentation) | Yes | Yes | No |
| Go to definition | Yes | Yes | Yes |
| Go to declaration | Yes | No | No |
| Go to type definition | Yes | No | No |
| Go to implementation | Yes | No | No |
| Find references | Yes | No | Yes |
| Document symbols | Yes | Yes | Yes |
| Workspace symbols | Yes | No | Yes |
| Rename | Yes | No | No |
| Prepare rename | Yes | No | No |
| Document highlight | Yes | No | No |
| Linked editing range | Yes | No | No |

### Navigation and Hierarchy

| Feature | perl-lsp | PLS | Perl::LanguageServer |
|---------|----------|-----|----------------------|
| Call hierarchy | Yes | No | No |
| Type hierarchy | Yes | No | No |
| Document links | Yes | No | No |
| Folding ranges | Yes | No | No |
| Selection range | Yes | No | No |
| Breadcrumbs (via symbols) | Yes | Yes | Yes |

### Code Intelligence

| Feature | perl-lsp | PLS | Perl::LanguageServer |
|---------|----------|-----|----------------------|
| Diagnostics (syntax) | Yes | Yes (perl -c) | Yes (perl -c) |
| Diagnostics (perlcritic) | Yes | Yes | No |
| Pull diagnostics (LSP 3.17) | Yes | No | No |
| Code actions / quick fixes | Yes | No | No |
| Code lens | Yes | No | No |
| Inlay hints | Yes | No | No |
| Inline completion | Yes | No | No |
| Semantic tokens | Yes | No | No |
| Inline values | Yes | No | No |

### Formatting

| Feature | perl-lsp | PLS | Perl::LanguageServer |
|---------|----------|-----|----------------------|
| Document formatting | Yes (perltidy) | Yes (perltidy) | Yes (perltidy) |
| Range formatting | Yes | Yes | Yes |
| Multi-range formatting (LSP 3.18) | Yes | No | No |
| On-type formatting | Yes | No | No |
| Sort imports | No | Yes | No |

### Workspace Features

| Feature | perl-lsp | PLS | Perl::LanguageServer |
|---------|----------|-----|----------------------|
| Multi-root workspaces | Yes | Yes | Yes |
| File operations (rename/create/delete) | Yes | No | No |
| Configuration | Yes | Yes | Yes |
| Workspace edits | Yes | No | No |

### Debug Adapter Protocol (DAP)

| Feature | perl-lsp | PLS | Perl::LanguageServer |
|---------|----------|-----|----------------------|
| Debug adapter | Yes (native) | No | Yes |
| Breakpoints | Yes | No | Yes |
| Conditional breakpoints | Yes | No | Yes |
| Hit-count breakpoints | Yes | No | No |
| Logpoints | Yes | No | No |
| Exception breakpoints | Yes (die + warn) | No | No |
| Watchpoints (data breakpoints) | Yes | No | No |
| Variable inspection | Yes | No | Yes |
| Set variable | Yes | No | Yes |
| Evaluate expressions | Yes | No | Yes |
| Debug completions | Yes | No | No |
| Loaded modules view | Yes | No | No |
| Inline values (debug) | Yes | No | No |
| Coro thread support | No | No | Yes |
| Auto-reload modules | No | No | Yes |
| Remote debugging (SSH) | No | No | Yes |
| Container debugging | No | No | Yes |

### Protocol and Lifecycle

| Feature | perl-lsp | PLS | Perl::LanguageServer |
|---------|----------|-----|----------------------|
| LSP spec version | 3.18 | ~3.16 | ~3.14 |
| Dynamic registration | Yes | No | No |
| Progress reporting | Yes | No | No |
| Notebook support | Yes | No | No |
| Telemetry | Yes | No | No |
| Refresh requests | Yes (6 types) | No | No |

## Architecture Comparison

### Parser Approach

| | perl-lsp | PLS | Perl::LanguageServer |
|--|----------|-----|----------------------|
| **Technology** | Custom recursive descent parser written in Rust | PPI (pure Perl parser) | Compiler::Lexer (XS tokenizer) |
| **Parse model** | Full AST with error recovery | PPI document tree | Token stream |
| **Error recovery** | Yes -- continues parsing after errors | PPI handles partial parses | Limited |
| **Incremental** | Yes -- re-parses on change | Full re-parse | Full re-parse |

### Language and Performance

| | perl-lsp | PLS | Perl::LanguageServer |
|--|----------|-----|----------------------|
| **Implementation** | Rust | Perl 5 | Perl 5 (with XS deps) |
| **Concurrency** | Native async (tokio) | Single-threaded | AnyEvent + Coro |
| **Startup** | Single binary, instant | Perl process startup | Perl + heavy deps |
| **Memory** | Low (native binary) | Moderate (Perl runtime) | Higher (Coro, AIO) |
| **Dependencies** | None at runtime | Perl + optional XS JSON | Perl + AnyEvent + Coro + AIO |
| **Installation** | Download binary | `cpanm PLS` | `cpanm Perl::LanguageServer` (complex deps) |

## What Makes Each Server Unique

### perl-lsp

- **Broadest LSP coverage**: Implements 96 of 97 LSP 3.18 features including modern capabilities like semantic tokens, inlay hints, call hierarchy, and type hierarchy.
- **Native performance**: Written in Rust with zero runtime dependencies. Sub-millisecond response times for most operations.
- **Integrated DAP**: Debug adapter is built-in as a native Rust implementation, not a separate process.
- **Custom parser**: Purpose-built recursive descent parser with error recovery, avoiding Perl's "only Perl can parse Perl" limitation.
- **CPAN corpus tested**: Parser validated against a large subset of CPAN modules.

### PLS

- **PPI foundation**: Uses the mature, well-tested PPI module that the Perl community trusts for static analysis.
- **Perlcritic integration**: First-class linting via Perl::Critic with configurable policies.
- **Import sorting**: Unique feature for organizing `use` statements.
- **Simple installation**: Pure CPAN install with minimal dependencies.
- **Wide editor support**: Tested with VS Code, Neovim, BBEdit, and Emacs.

### Perl::LanguageServer

- **Remote development**: Mature SSH and container support for running on remote systems, Docker, and Kubernetes.
- **Coro debugging**: Unique support for debugging Coro-based concurrent Perl applications.
- **Module hot-reload**: Automatically reloads changed modules during debug sessions.
- **Established**: Longest-running Perl language server, first released in 2018.

## Feature Count Summary

| Category | perl-lsp | PLS | Perl::LanguageServer |
|----------|----------|-----|----------------------|
| Core editing | 14/14 | 4/14 | 4/14 |
| Navigation | 6/6 | 1/6 | 1/6 |
| Code intelligence | 9/9 | 2/9 | 1/9 |
| Formatting | 4/5 | 2/5 | 2/5 |
| Workspace | 4/4 | 2/4 | 2/4 |
| Debug (DAP) | 12/16 | 0/16 | 9/16 |
| **Total** | **49/54** | **11/54** | **19/54** |

*Counts based on user-facing features in the matrix above. Protocol/lifecycle features excluded from totals.*

---

*Last updated: 2026-03-19. Feature data sourced from each project's documentation and source code.*
