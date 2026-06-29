//! Call-observation regression test for the mid-surrogate UTF-16 column clamp
//! in `LineStartsCache::position_to_offset` (fix #2478).
//!
//! This test drives `detect_dead_code` — a real production caller of
//! `LineStartsCache::position_to_offset` — with a symbol whose UTF-16 column
//! lands on the trailing surrogate of an emoji in the source text.  It asserts
//! that the returned byte offset clamps to the *start* of the codepoint, not
//! past it.  The assertion would fail if the `uc + ch.len_utf16() > character`
//! clamp at line_index.rs:87 were reverted to the old `uc >= character` guard.

#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use perl_lsp_rs_core::providers::diagnostics::detect_dead_code;
    use perl_parser_core::position::LineStartsCache;
    use perl_workspace::workspace_index::WorkspaceIndex;

    /// Index a Perl file, call `detect_dead_code` with emoji source text whose
    /// UTF-16 column 2 is the trailing surrogate of "😀", and assert the byte
    /// offset clamps to the emoji start.
    ///
    /// Setup:
    ///   Perl source  = "1;sub foo{}"
    ///     • `1;` is a valid Perl no-op statement (2 ASCII chars = 2 UTF-16 units).
    ///     • `sub foo{}` follows immediately; the parser places the Subroutine
    ///       node at byte offset 2 on line 0.
    ///     • `LineIndex::offset_to_position(2)` yields (line=0, col=2) because
    ///       the 2 bytes before it are each 1 UTF-16 unit.
    ///   Position source for detect_dead_code = "x😀y"
    ///     • UTF-16 layout: x=0, 😀=1(first surrogate), 😀=2(trailing), y=3.
    ///     • Column 2 lands on the trailing surrogate of "😀" — a mid-surrogate.
    ///     • Correct clamped byte offset: 1 (start of the emoji, just past 'x').
    ///     • Without the fix the guard would be `uc >= character`, which would
    ///       advance past the emoji and return byte offset 5 ('y' position).
    #[test]
    fn detect_dead_code_position_to_offset_clamps_mid_surrogate_column() {
        // ── 1. Build a WorkspaceIndex with one unused subroutine. ─────────────
        //
        // "1;sub foo{}" is valid Perl:
        //   byte 0-1: "1;" (two ASCII chars, 2 UTF-16 units)
        //   byte 2+:  "sub foo{}"
        //
        // The Subroutine node starts at byte 2.  LineIndex::offset_to_position(2)
        // on this single-line text yields (line=0, column=2).

        let perl_source = "1;sub foo{}";
        let index = WorkspaceIndex::new();
        let raw_uri = "file:///test_mid_surrogate_2478.pl";
        index.index_file_str(raw_uri, perl_source).expect("Perl source must parse without error");

        // ── 2. Confirm the symbol is present and unused ───────────────────────
        let unused = index.find_unused_symbols();
        let foo_sym = unused
            .iter()
            .find(|s| s.name == "foo")
            .expect("sub foo should be unused (no callers indexed)");

        // The symbol must start at line=0, column=2 (UTF-16).
        // This is what drives the mid-surrogate branch in position_to_offset.
        assert_eq!(foo_sym.range.start.line, 0, "sub foo should be on line 0 (0-based)");
        assert_eq!(
            foo_sym.range.start.column, 2,
            "sub foo should start at UTF-16 column 2 (after the two-unit '1;' prefix)"
        );

        // Use the actual stored URI so the detect_dead_code filter matches.
        let symbol_uri = foo_sym.uri.clone();

        // ── 3. Build LineStartsCache for emoji-containing source text ─────────
        //
        // "x😀y": UTF-16 layout x=0, 😀=1+2 (surrogate pair), y=3.
        // Column 2 is the trailing surrogate — a mid-surrogate position.
        // Byte layout: x=byte0, 😀=bytes1-4, y=byte5.

        let position_text = "x\u{1F600}y"; // "x😀y"
        let line_cache = LineStartsCache::new(position_text);

        // ── 4. Drive detect_dead_code ─────────────────────────────────────────
        //
        // detect_dead_code calls:
        //   line_cache.position_to_offset(position_text, sym.range.start.line, sym.range.start.column)
        //  = position_to_offset("x😀y", 0, 2)
        //
        // With the fix  (uc + ch.len_utf16() > character): clamps to byte 1 (emoji start).
        // Without the fix (uc >= character):                returns byte 5 ('y' position).

        let diags = detect_dead_code(&index, &symbol_uri, position_text, &line_cache);

        assert!(
            !diags.is_empty(),
            "detect_dead_code must return at least one diagnostic for the unused sub foo; \
             symbol_uri={symbol_uri:?}"
        );

        // The start byte for "foo" must be 1 — the clamped emoji start, not 5 ('y').
        let start_byte = diags
            .iter()
            .find(|d| d.message.contains("foo"))
            .map(|d| d.range.0)
            .expect("a diagnostic mentioning 'foo' must be present");

        assert_eq!(
            start_byte, 1,
            "position_to_offset(\"x😀y\", 0, 2) must clamp to byte 1 (emoji start); \
             got {start_byte} — if this is 5 the mid-surrogate clamp at line_index.rs:87 \
             was not applied"
        );
    }
}
