//! Mutation-proof boundary tests for the qw-whitespace seam in the semantic
//! dependency index (`crates/perl-semantic-analyzer/src/analysis/index.rs`, #1203).
//!
//! Each test pins ONE decision boundary so that a single-line mutation of the
//! guarding sub-condition causes exactly that test to fail.
//!
//! ## Seam — qw-whitespace delimiter fix (`parse_qw_content`, #1203)
//!
//! **Issue**: `use parent qw [Foo::Base]` failed to extract `Foo::Base` as a
//! dependency because Perl allows optional whitespace between the `qw` keyword
//! and its opening delimiter.  Before the fix, the space character was consumed
//! as the delimiter itself, so `parse_qw_content` treated `[Foo::Base]` as the
//! content (with the `[` being the first character), and `rfind(']')` found no
//! matching close, returning `None` — or worse, recording the raw `qw [...]`
//! token as a bogus dependency.
//!
//! **Fix**: `.trim_start()` applied to the tail after stripping the `"qw"` prefix,
//! before reading the opening delimiter character.
//!
//! **Discriminating signal**: `file_dependencies()` — the public API that returns
//! the set of module names extracted from a Perl source file.  A mutation that
//! removes `.trim_start()` causes the spaced-delimiter tests to return an empty
//! set (or a bogus entry) instead of the correct module name, failing each
//! boundary test below.
//!
//! ## Decision boundaries covered
//!
//! | Boundary | Guarding condition | Fails when mutated by |
//! |----------|-------------------|----------------------|
//! | A | `trim_start()` before delimiter read | Removing `.trim_start()` |
//! | B | `[` → `]` bracket-pair mapping | Changing close to wrong char |
//! | C | `(` → `)` paren-pair mapping | Changing close to wrong char |
//! | D | `rfind(close)` with symmetric delimiter `/` | Removing rfind logic |
//! | E | Bareword `qwfoo` → `None` (no-delimiter guard) | Removing the `end < start` guard |
//! | F | Multi-module extraction via `split_whitespace` | Removing split step |

use perl_semantic_analyzer::index::WorkspaceIndex;

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY A — trim_start() strips leading space before the delimiter
// ═══════════════════════════════════════════════════════════════════════════
//
// Without `.trim_start()`, the space after `qw` is consumed as the opening
// delimiter, so `rfind(' ')` would search for a space inside the content,
// returning garbage.  The dependency set would be empty or wrong.
//
// Mutation: remove `.trim_start()` → `file_dependencies` returns {} or a
// bogus string, not {"Foo::Base"}.

/// `use parent qw [Foo]` with a space before `[` must extract `Foo::Base`.
/// Boundary A: `.trim_start()` allows leading space before the `[` delimiter.
/// Removing `.trim_start()` makes the space become the delimiter — `rfind(' ')`
/// finds a space inside the content only by accident, so the module is lost.
#[test]
fn seam_a_space_before_bracket_delimiter_extracts_module() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///a.pl", "use parent qw [Foo::Base];\n1;\n")?;
    let deps = index.file_dependencies("file:///a.pl");
    assert!(
        deps.contains("Foo::Base"),
        "trim_start boundary A: space before '[' delimiter must yield Foo::Base; got: {deps:?}"
    );
    assert!(
        !deps.iter().any(|d| d.starts_with("qw")),
        "trim_start boundary A: no bogus qw-prefixed dep must appear; got: {deps:?}"
    );
    Ok(())
}

/// `use base qw [Foo::Base]` — same boundary through `base` pragma.
/// Confirms the seam fires identically for `base` as for `parent`.
#[test]
fn seam_a_space_before_bracket_delimiter_base_pragma() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///b.pl", "use base qw [Bar::Base];\n1;\n")?;
    let deps = index.file_dependencies("file:///b.pl");
    assert!(
        deps.contains("Bar::Base"),
        "trim_start boundary A (base): space before '[' must yield Bar::Base; got: {deps:?}"
    );
    Ok(())
}

/// `use parent qw\t[Foo::Base]` — tab before delimiter also trimmed.
/// Boundary A: `trim_start()` handles all Unicode whitespace, not only space.
#[test]
fn seam_a_tab_before_bracket_delimiter_extracts_module() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///tab.pl", "use parent qw\t[Foo::Tab];\n1;\n")?;
    let deps = index.file_dependencies("file:///tab.pl");
    assert!(
        deps.contains("Foo::Tab"),
        "trim_start boundary A (tab): tab before '[' must yield Foo::Tab; got: {deps:?}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY B — bracket pair: '[' opens, ']' closes
// ═══════════════════════════════════════════════════════════════════════════
//
// The match arm `'[' => ']'` selects the correct closing delimiter.
// A mutation that changes `']'` to a wrong char (e.g. `')'`) makes
// `rfind(wrong_close)` return `None` and the module is lost.

/// `qw [Foo]` — bracket pair must produce `Foo`.
/// Boundary B: `'[' => ']'` is the only match arm that satisfies this test.
#[test]
fn seam_b_bracket_pair_produces_correct_extraction() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///bracket.pl", "use parent qw [Bkt::Mod];\n1;\n")?;
    let deps = index.file_dependencies("file:///bracket.pl");
    assert!(
        deps.contains("Bkt::Mod"),
        "bracket boundary B: qw [Bkt::Mod] must extract Bkt::Mod; got: {deps:?}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY C — paren pair: '(' opens, ')' closes
// ═══════════════════════════════════════════════════════════════════════════
//
// The match arm `'(' => ')'` selects the correct closing delimiter.
// A mutation changing `')'` to another char causes rfind to fail → no module.

/// `qw(Foo)` — paren pair must produce `Foo`.
/// Boundary C: `'(' => ')'` arm; mutation to wrong close drops the module.
#[test]
fn seam_c_paren_pair_produces_correct_extraction() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///paren.pl", "use parent qw(Par::Mod);\n1;\n")?;
    let deps = index.file_dependencies("file:///paren.pl");
    assert!(
        deps.contains("Par::Mod"),
        "paren boundary C: qw(Par::Mod) must extract Par::Mod; got: {deps:?}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY D — symmetric delimiter: '/' closes with '/'
// ═══════════════════════════════════════════════════════════════════════════
//
// The default arm `delimiter => delimiter` handles symmetric delimiters like `/`.
// A mutation that accidentally changes the default arm to a fixed char breaks
// rfind for all symmetric-delimiter forms.

/// `qw/Foo/` — slash delimiter must produce `Foo`.
/// Boundary D: `delimiter => delimiter` wildcard arm; any off-by-one breaks this.
#[test]
fn seam_d_slash_symmetric_delimiter_extracts_module() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///slash.pl", "use parent qw/Sl::Mod/;\n1;\n")?;
    let deps = index.file_dependencies("file:///slash.pl");
    assert!(
        deps.contains("Sl::Mod"),
        "symmetric-delimiter boundary D: qw/Sl::Mod/ must extract Sl::Mod; got: {deps:?}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY E — bareword guard: 'qwfoo' is not valid qw syntax → None
// ═══════════════════════════════════════════════════════════════════════════
//
// After trim_start(), if `open` is an alphanumeric char (not a valid delimiter),
// `rfind(open)` finds the char within the identifier itself.  The bounds guard
// `if end < start { return None }` prevents a slice panic and signals rejection.
//
// A mutation that removes `if end < start { return None }` would cause a
// slice panic at `&rest[start..end]` when `end < start`.

/// `qwfoo` directly after `parent` — not valid qw syntax; the function must
/// return `None` without panicking.
/// Boundary E: the `end < start` guard prevents `&rest[start..end]` from
/// being evaluated when start > end, which would panic.
/// A mutation removing the guard causes a runtime panic on this input.
///
/// The observable signal: `index_file_str` completes without returning an error.
/// Because `qwfoo` is treated as a bareword string by `expand_parent_arg`, it
/// may be recorded as a dep named `"qwfoo"` — that is acceptable.
/// What must NOT happen is a panic or error.
#[test]
fn seam_e_bareword_after_qw_does_not_panic() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    // "qwfoo" is a bareword argument to `parent`, not a valid qw list.
    // The bounds guard `if end < start { return None }` ensures parse_qw_content
    // returns None cleanly, and expand_parent_arg falls back to treating it as a
    // plain string.  A mutation removing that guard would produce a panic here.
    index.index_file_str("file:///bareword.pl", "use parent qwfoo;\n1;\n")?;
    // The function completed without panic — boundary E is satisfied.
    // (Content of deps for a bareword is an implementation detail, not the seam.)
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY F — multi-module extraction: split_whitespace produces N entries
// ═══════════════════════════════════════════════════════════════════════════
//
// After `parse_qw_content` returns the interior string, `expand_parent_arg`
// calls `content.split_whitespace().map(str::to_string).collect()` to yield
// each token as a separate dependency.
//
// A mutation that removes the split (e.g. returning the whole string as one
// entry) would mean `file_dependencies` contains one entry like
// "Foo::Base Bar::Base" instead of two separate entries.

/// `qw [Foo::Base Bar::Base]` — two modules, both appear in dependencies.
/// Boundary F: split_whitespace separates them; a missing split yields one
/// concatenated string instead of two distinct entries.
#[test]
fn seam_f_multi_module_qw_extracts_each_module_separately() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str(
        "file:///multi.pl",
        "use parent qw [Alpha::Base Beta::Base Gamma::Base];\n1;\n",
    )?;
    let deps = index.file_dependencies("file:///multi.pl");
    assert!(
        deps.contains("Alpha::Base"),
        "split boundary F: Alpha::Base must be a separate dep; got: {deps:?}"
    );
    assert!(
        deps.contains("Beta::Base"),
        "split boundary F: Beta::Base must be a separate dep; got: {deps:?}"
    );
    assert!(
        deps.contains("Gamma::Base"),
        "split boundary F: Gamma::Base must be a separate dep; got: {deps:?}"
    );
    // None of the joined strings should appear as a single entry.
    assert!(
        !deps.iter().any(|d| d.contains(' ')),
        "split boundary F: no dep must contain a space (unsplit); got: {deps:?}"
    );
    Ok(())
}

/// Compact form `qw(Foo::X Bar::X)` — confirms multi-module works with paren delimiter.
/// Boundary F + C combined: both seams must fire together.
#[test]
fn seam_f_multi_module_paren_qw_extracts_each_module_separately() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///multi_paren.pl", "use parent qw(Foo::X Bar::X);\n1;\n")?;
    let deps = index.file_dependencies("file:///multi_paren.pl");
    assert!(
        deps.contains("Foo::X"),
        "split+paren boundary F+C: Foo::X must be extracted; got: {deps:?}"
    );
    assert!(
        deps.contains("Bar::X"),
        "split+paren boundary F+C: Bar::X must be extracted; got: {deps:?}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTROL — compact qw (no whitespace before delimiter) still works
// ═══════════════════════════════════════════════════════════════════════════
//
// `trim_start()` is a no-op when there is no leading whitespace.
// These tests verify the fix does not regress the original compact form.

/// `qw(Foo)` compact form (no space) — must still extract `Foo`.
/// Control: ensures trim_start does not break the zero-whitespace path.
#[test]
fn seam_control_compact_qw_paren_still_works() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///compact_paren.pl", "use parent qw(Compact::Paren);\n1;\n")?;
    let deps = index.file_dependencies("file:///compact_paren.pl");
    assert!(
        deps.contains("Compact::Paren"),
        "control: compact qw(Compact::Paren) must extract Compact::Paren; got: {deps:?}"
    );
    Ok(())
}

/// `qw[Foo]` compact bracket form — must still extract `Foo`.
#[test]
fn seam_control_compact_qw_bracket_still_works() -> Result<(), String> {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///compact_bracket.pl", "use parent qw[Compact::Bracket];\n1;\n")?;
    let deps = index.file_dependencies("file:///compact_bracket.pl");
    assert!(
        deps.contains("Compact::Bracket"),
        "control: compact qw[Compact::Bracket] must extract Compact::Bracket; got: {deps:?}"
    );
    Ok(())
}
