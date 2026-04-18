# Vision Alignment — work-b4f6edfe

## Vision Alignment: ALIGNED

## Rationale

The work of decomposing the `unexpected_token_in_expr` bucket and filing targeted follow-up issues is **aligned** with the project's stated goals. Here's why:

### 1. Real-World Perl Support
The issue title explicitly references "CPAN corpus files" — real Perl modules from CPAN. The project's parser exists to provide IDE services for real Perl code, not just toy examples. Fixing parse errors in English.pm, File/Copy.pm, and IPC/Cmd.pm directly serves the LSP's core purpose.

### 2. Systematic Error Reduction
The approach of decomposing a catch-all bucket into specific sub-patterns with concrete test cases is the RIGHT methodology. This is how a mature parser project should handle heterogeneous error categories — one fix at a time with regression tests, rather than attempting a large undifferentiated change.

### 3. Test-Driven Fixes
The plan's emphasis on "minimal Perl code snippet (≤10 lines)" and filing targeted issues per sub-pattern ensures that future fixes are verifiable. This matches the existing test culture in `perl-parser-core` where `#[test]` functions with `assert_clean_parse` are the standard regression guard.

### 4. Consistency with Prior Art
The existing `fix_unexpected_token_in_expr_2731.rs` file shows this methodology has already been applied successfully to 4 sub-patterns. The current work extends that success pattern to remaining patterns.

## Misalignment Concerns (Minor)

The research agent's incorrect pattern attribution is a concern but doesn't invalidate the approach — it just means the builder needs to do their own pattern identification rather than relying on the research agent's claims. The approach (sample → decompose → file targeted issues → fix) remains sound.

## Conclusion

**ALIGNED** — The goal of eliminating `unexpected_token_in_expr` errors from real Perl modules is precisely what the LSP should be doing. The methodology is correct even if the specific pattern identifications were wrong.
