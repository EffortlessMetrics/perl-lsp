# perl-dap-stack

Stack trace parsing and frame classification utilities for Perl DAP.

## Scope

- Parses Perl debugger stack output into DAP-compatible `StackFrame` values.
- Classifies frames (user code vs library/runtime code).
- Provides shared frame/source model types for debug UI integration.

## Public Surface

- `PerlStackParser`, `StackParseError`.
- `FrameClassifier`, `PerlFrameClassifier`, `FrameCategory`.
- DAP model types: `StackFrame`, `Source`, and presentation hints.

## Workspace Role

Internal support crate used by `perl-dap` request handlers.

## License

MIT OR Apache-2.0.
