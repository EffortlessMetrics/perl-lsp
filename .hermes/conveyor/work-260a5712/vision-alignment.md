# Vision Alignment — work-260a5712

## Status: **aligned**

The issue (#3430) requests VS Code commands and settings UI for perltidy profile configuration. My verification confirms:

1. **Gap is real**: The VS Code extension does not currently wire perltidy settings to the LSP server — only perlcritic settings are wired.

2. **Rust side is complete**: `PerlTidyConfig` has all options, `FormattingProvider` accepts them, LSP server exposes `perl.formatting.*` settings.

3. **Implementation approach is sound**: Adding VS Code settings + `workspace/didChangeConfiguration` wiring is the correct pattern (matching how perlcritic is wired).

## Minor Concerns (not blockers)

- The plan's proposed `profile` enum `["default", "pbp", "gnu"]` is incorrect — `profile` is a file path, not a profile selector. Built-in profiles use `extra_args`. Implementation must use a different approach (e.g., a `builtinProfile` enum that maps to `extra_args`).
- Precedence between `perltidyConfig` path and individual settings must be clearly documented — when a profile path is set, individual settings are ignored by perltidy.

## Verdict

**Proceed with corrected implementation**. The design agent should address the `profile` vs `extra_args` issue before building.
