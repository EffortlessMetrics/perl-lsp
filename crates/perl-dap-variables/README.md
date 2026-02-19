# perl-dap-variables

Variable parsing and rendering for the Perl Debug Adapter Protocol (DAP).

## Overview

This crate provides types and utilities for converting Perl debugger output into
structured values and rendering them in DAP-compatible format for display in
VSCode and other DAP-compatible editors.

## Public API

- **`PerlValue`** -- enum representing Perl values: `Undef`, `Scalar`, `Number`, `Integer`, `Array`, `Hash`, `Reference`, `Object`, `Code`, `Glob`, `Regex`, `Tied`, `Truncated`, `Error`
- **`VariableParser`** -- parses Perl debugger text output (e.g., `$x = 42`) into `PerlValue` instances
- **`VariableParseError`** -- error type for parsing failures
- **`VariableRenderer`** -- trait for rendering `PerlValue` into DAP variables
- **`PerlVariableRenderer`** -- default renderer with configurable truncation and preview limits
- **`RenderedVariable`** -- DAP-compatible variable with name, value, type, and expansion metadata

## Workspace Role

Internal support crate consumed by `perl-dap` for variable inspection and
evaluate responses. Part of the
[tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs)
workspace.

## License

MIT OR Apache-2.0
