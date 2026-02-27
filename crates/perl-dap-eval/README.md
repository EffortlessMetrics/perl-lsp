# perl-dap-eval

Safe expression evaluation validation for the Perl Debug Adapter Protocol (DAP).

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Overview

This crate validates Perl expressions before evaluation during debugging sessions, blocking operations that could mutate state, execute arbitrary code, or perform I/O. It provides the `SafeEvaluator` type which checks expressions against a comprehensive list of dangerous Perl operations.

## Public API

- **`SafeEvaluator`** -- validates expressions via `validate(&self, expression: &str) -> ValidationResult`
- **`ValidationError`** -- enum of rejection reasons (dangerous operation, assignment, backticks, regex mutation, newlines)
- **`ValidationResult`** -- type alias for `Result<(), ValidationError>`
- **`DANGEROUS_OPERATIONS`** -- the list of blocked Perl built-in names

## Blocked Categories

Code execution (`eval`, `system`, `exec`), process control (`fork`, `exit`, `kill`), I/O (`print`, `open`, `write`), filesystem (`mkdir`, `unlink`, `chmod`), network (`socket`, `connect`), tie mechanism, IPC, assignment operators, increment/decrement, regex mutation (`s///`, `tr///`, `y///`), backticks, and newlines.

## Context-Aware Filtering

The validator avoids false positives for sigil-prefixed identifiers (`$print`, `@say`), braced variables (`${print}`), package-qualified names (`Foo::print`), single-quoted strings, and escape sequences (`\s`).

## License

MIT OR Apache-2.0
