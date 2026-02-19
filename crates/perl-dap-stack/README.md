# perl-dap-stack

Stack trace parsing and frame classification for the Perl Debug Adapter Protocol.

## Overview

This crate provides types and utilities for parsing Perl debugger stack trace
output into DAP-compatible structures and classifying frames as user code,
library code, or core Perl internals.

## Public API

- **`StackFrame`** / **`Source`** -- DAP-compatible model types with builder methods
- **`StackFramePresentationHint`** / **`SourcePresentationHint`** -- UI rendering hints
- **`StackTraceProvider`** -- trait for stack trace retrieval implementations
- **`PerlStackParser`** -- parses Perl debugger output (`T` command, context lines, eval frames)
- **`StackParseError`** -- error type for parse failures
- **`FrameClassifier`** / **`PerlFrameClassifier`** -- classifies frames by origin (user, library, core, eval)
- **`FrameCategory`** -- classification result enum

## Workspace Role

Internal support crate consumed by `perl-dap` request handlers. Provides the
shared stack frame model and debugger output parsing used during debug sessions.

## License

MIT OR Apache-2.0
