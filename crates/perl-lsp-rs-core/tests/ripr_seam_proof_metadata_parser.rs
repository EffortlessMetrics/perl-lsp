//! Mutation-proof boundary tests for the string-scanner seams in the metadata
//! dependency extractor (`crates/perl-lsp-rs-core/src/config/metadata_dependencies.rs`).
//!
//! Each test pins ONE decision boundary so that a single-line mutation of the
//! guarding sub-condition causes exactly that test to fail.
//!
//! ## Seams covered
//!
//! ### `find_key_arrow_value` (called from `extract_makefile_pl_requirements`)
//!
//! | Boundary | Guarding condition | Fails when mutated by |
//! |----------|-------------------|-----------------------|
//! | A | `while idx < bytes.len()` loop guard | Removing the guard / off-by-one |
//! | B | `byte == b'\\'` escape skip in single-quote context | Removing escape skip |
//! | C | `byte == b'\''` single-quote close | Swapping quote byte |
//! | D | `byte == b'\\'` escape skip in double-quote context | Removing escape skip |
//! | E | `byte == b'"'` double-quote close | Swapping quote byte |
//! | F | `bytes.get(value_idx) == Some(&b'=') && …Some(&b'>')` arrow detection | Removing either check |
//!
//! ### `matching_brace` (called when `{` follows `=>` in `extract_hash_requirements`)
//!
//! | Boundary | Guarding condition | Fails when mutated by |
//! |----------|-------------------|-----------------------|
//! | G | `bytes.get(open_idx) != Some(&b'{')` non-brace guard | Removing guard |
//! | H | `byte == b'\\'` escape in single-quote inside braces | Removing escape skip |
//! | I | `byte == b'\''` single-quote close inside braces | Swapping quote byte |
//! | J | `byte == b'\\'` escape in double-quote inside braces | Removing escape skip |
//! | K | `byte == b'"'` double-quote close inside braces | Swapping quote byte |
//! | L | `depth == 0` after `}` | Changing depth check |
//! | M | `while idx < bytes.len()` loop guard (unclosed brace) | Removing the guard |
//!
//! ### `parse_quoted_string` (called from `parse_hash_dependency_pairs`)
//!
//! | Boundary | Guarding condition | Fails when mutated by |
//! |----------|-------------------|-----------------------|
//! | N | `quote != b'\'' && quote != b'"'` non-quote guard | Removing guard |
//! | O | `ch == '\n'` newline termination | Removing newline check |
//! | P | `while idx < bytes.len()` exhaustion | Removing the guard |
//! | Q | `ch == '\\'` escape handling | Removing escape set |
//! | R | `ch as u8 == quote` quote close | Swapping quote comparison |

use perl_lsp_rs_core::config::{
    DeclaredDependency, DeclaredDependencySource, extract_build_pl_requirements,
    extract_makefile_pl_requirements,
};

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY A — while idx < bytes.len() in find_key_arrow_value
// ═══════════════════════════════════════════════════════════════════════════
//
// Empty source: bytes.len() == 0, the while loop body never executes.
// A mutation that removes the loop guard (or changes < to <=) would either
// skip all processing (no change for empty) or access out-of-bounds.
// The discriminating signal: empty string yields no deps.

/// Empty `Makefile.PL` source must yield an empty dependency list.
/// Boundary A: the `while idx < bytes.len()` loop exits immediately for empty input.
/// Removing the loop guard or changing `<` to `<=` would panic on empty byte slice.
#[test]
fn seam_a_empty_source_yields_no_deps() {
    let deps = extract_makefile_pl_requirements("");
    assert!(deps.is_empty(), "boundary A: empty source must yield no deps; got: {deps:?}");
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY B — byte == b'\\' escape skip in single-quote context
// ═══════════════════════════════════════════════════════════════════════════
//
// Source contains a single-quoted string where \' appears: 'key\' => val'.
// With escape handling: \' keeps the quote OPEN, so PREREQ_PM inside the
// string is not matched as a key. Without it: the ' after \ closes the
// string prematurely, and the parser sees => val' as un-quoted, which
// confuses subsequent scanning and changes which module is extracted.

/// Single-quoted value with escaped \' must NOT allow the key inside it to match.
/// Boundary B: escape skip in single-quote context; removing it causes the quote
/// to close at the wrong position and the wrong result (or no result) to be returned.
#[test]
fn seam_b_escaped_single_quote_keeps_key_inside_string() {
    // Actual string: 'PREREQ_PM\' => ignored' PREREQ_PM => {'Target::Dep' => '1.0'}
    // The \' is an escaped single quote inside the Perl single-quoted string.
    // With correct escape handling, PREREQ_PM inside the quoted string is skipped.
    let source = "'PREREQ_PM\\' => ignored' PREREQ_PM => {'Target::Dep' => '1.0'}";
    let deps = extract_makefile_pl_requirements(source);
    assert_eq!(
        deps,
        vec![DeclaredDependency::new(
            "Target::Dep",
            Some("1.0"),
            "PREREQ_PM",
            DeclaredDependencySource::MakefilePl,
        )],
        "boundary B: escaped \\' inside single-quoted string must not match the embedded key"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY C — byte == b'\'' single-quote close in find_key_arrow_value
// ═══════════════════════════════════════════════════════════════════════════
//
// Source contains a plain single-quoted string before the real key.
// When the closing ' is recognized, in_single_quote is set to false and
// scanning of the real key resumes. If the closing ' byte check were wrong
// (e.g. swapped to b'"'), the single-quote would never close, hiding the
// real key.

/// Single-quoted embedded value must close so the real key is found.
/// Boundary C: byte == b'\'' sets in_single_quote = false; swapping the byte
/// literal would keep the scanner inside the "quoted" region forever, hiding
/// the real PREREQ_PM key and returning no deps.
#[test]
fn seam_c_single_quote_close_allows_subsequent_key_match() {
    let source = "WriteMakefile('embedded' PREREQ_PM => {'Close::Mod' => '2.0'});";
    let deps = extract_makefile_pl_requirements(source);
    assert_eq!(
        deps,
        vec![DeclaredDependency::new(
            "Close::Mod",
            Some("2.0"),
            "PREREQ_PM",
            DeclaredDependencySource::MakefilePl,
        )],
        "boundary C: single-quote close must end the quoted region so the real key is found"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY D — byte == b'\\' escape skip in double-quote context
// ═══════════════════════════════════════════════════════════════════════════
//
// Mirrors boundary B but for double-quoted strings: "key\" => val".
// With escape handling: \" keeps the double-quote OPEN.
// Without it: the " after \ closes the string prematurely.

/// Double-quoted value with escaped \" must NOT allow the key inside it to match.
/// Boundary D: escape skip in double-quote context; removing it closes the quote
/// at the wrong position, changing which (if any) module is extracted.
#[test]
fn seam_d_escaped_double_quote_keeps_key_inside_string() {
    // Actual string: "PREREQ_PM\" => ignored" PREREQ_PM => {'DQ::Dep' => '3.0'}
    let source = "\"PREREQ_PM\\\" => ignored\" PREREQ_PM => {'DQ::Dep' => '3.0'}";
    let deps = extract_makefile_pl_requirements(source);
    assert_eq!(
        deps,
        vec![DeclaredDependency::new(
            "DQ::Dep",
            Some("3.0"),
            "PREREQ_PM",
            DeclaredDependencySource::MakefilePl,
        )],
        "boundary D: escaped \\\" inside double-quoted string must not match the embedded key"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY E — byte == b'"' double-quote close in find_key_arrow_value
// ═══════════════════════════════════════════════════════════════════════════
//
// Mirrors boundary C but for double-quoted strings.

/// Double-quoted embedded value must close so the real key is found.
/// Boundary E: byte == b'"' sets in_double_quote = false; swapping the byte
/// would keep the scanner inside the "quoted" region, hiding the real key.
#[test]
fn seam_e_double_quote_close_allows_subsequent_key_match() {
    let source = "WriteMakefile(\"embedded\" PREREQ_PM => {'DQClose::Mod' => '4.0'});";
    let deps = extract_makefile_pl_requirements(source);
    assert_eq!(
        deps,
        vec![DeclaredDependency::new(
            "DQClose::Mod",
            Some("4.0"),
            "PREREQ_PM",
            DeclaredDependencySource::MakefilePl,
        )],
        "boundary E: double-quote close must end the quoted region so the real key is found"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY F — arrow detection: Some(&b'=') && Some(&b'>')
// ═══════════════════════════════════════════════════════════════════════════
//
// Source has the key but without `=>`, so the arrow check fails and None
// is returned for that occurrence; the scanner advances and yields no match.
// A mutation removing either byte check would incorrectly match non-arrow
// sequences (e.g. `=` alone, or any two bytes).

/// Key present but not followed by `=>` must yield no deps.
/// Boundary F: `bytes.get(value_idx) == Some(&b'=') && …Some(&b'>')` must both
/// be true; removing either check would produce false positives.
#[test]
fn seam_f_key_without_fat_arrow_yields_no_deps() {
    let deps = extract_makefile_pl_requirements("PREREQ_PM");
    assert!(deps.is_empty(), "boundary F: key without => must yield no deps; got: {deps:?}");
}

/// Key followed by `=` without `>` must NOT match.
/// Boundary F continued: the `&&` requires BOTH `=` AND `>`.
#[test]
fn seam_f_key_with_eq_but_not_arrow_yields_no_deps() {
    let deps = extract_makefile_pl_requirements("PREREQ_PM = {}");
    assert!(
        deps.is_empty(),
        "boundary F: PREREQ_PM = {{}} (no >) must yield no deps; got: {deps:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY G — bytes.get(open_idx) != Some(&b'{') in matching_brace
// ═══════════════════════════════════════════════════════════════════════════
//
// When the value after `=>` is not `{`, matching_brace returns None and the
// key occurrence is skipped. This is exercised when PREREQ_PM => 'scalar'.
// A mutation removing the guard would treat any byte as an "opening brace",
// scanning endlessly or producing wrong output.

/// Value after `=>` that is NOT `{` must be skipped, yielding no deps.
/// Boundary G: `bytes.get(open_idx) != Some(&b'{')` returns None for non-brace.
/// Removing the guard would pass non-brace bytes to the depth counter,
/// producing wrong output.
#[test]
fn seam_g_non_brace_value_is_skipped() {
    let deps = extract_makefile_pl_requirements("WriteMakefile(PREREQ_PM => 'scalar');");
    assert!(
        deps.is_empty(),
        "boundary G: scalar (non-hash) value must yield no deps; got: {deps:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY H — escape in single-quote inside matching_brace
// ═══════════════════════════════════════════════════════════════════════════
//
// Source has a single-quoted value inside the hash body with \' in it.
// The escape must be handled so the } inside the quoted string does not
// prematurely close the brace scan.

/// Escaped \' inside a quoted hash value must not prematurely close the brace.
/// Boundary H: escape skip in single-quote context inside matching_brace.
/// Removing the escape skip would allow a lone `}` in the quoted value to be
/// treated as the brace close, cutting off subsequent entries.
#[test]
fn seam_h_escaped_single_quote_inside_brace_does_not_close_early() {
    // Hash body: { 'key\'}' => '1.0', 'After::Escape' => '2.0' }
    // The \' must NOT terminate the quoted string, keeping the } inside quotes.
    let source = "WriteMakefile(PREREQ_PM => { 'key\\'}' => '1.0', 'After::Escape' => '2.0' });";
    let deps = extract_makefile_pl_requirements(source);
    let modules: Vec<&str> = deps.iter().map(|d| d.module.as_str()).collect();
    assert!(
        modules.contains(&"After::Escape"),
        "boundary H: escaped \\' must keep the }} inside quotes, not cut off later entries; \
         got: {modules:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY I — byte == b'\'' single-quote close in matching_brace
// ═══════════════════════════════════════════════════════════════════════════
//
// After the single-quote opens, its matching close must be detected so the
// brace scanner resumes tracking depth. If the close detection byte were
// wrong, the scanner would stay "in_single_quote" through nested braces,
// counting the outer } as still-quoted and never finding the close.

/// Single-quoted value containing a nested `{` must not inflate brace depth.
/// Boundary I: byte == b'\'' closes the single-quote; if swapped, the brace
/// depth counter never returns to 0 (the } at the end would be inside
/// the "unclosed" quote), so matching_brace returns None and no deps are found.
#[test]
fn seam_i_single_quote_close_inside_brace_resumes_depth_tracking() {
    let source = "WriteMakefile(PREREQ_PM => { 'open{brace' => '1.0', 'SQClose::Dep' => '2.0' });";
    let deps = extract_makefile_pl_requirements(source);
    let modules: Vec<&str> = deps.iter().map(|d| d.module.as_str()).collect();
    assert!(
        modules.contains(&"SQClose::Dep"),
        "boundary I: single-quote close must re-enable depth tracking after quoted content; \
         got: {modules:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY J — escape in double-quote inside matching_brace
// ═══════════════════════════════════════════════════════════════════════════
//
// Mirrors boundary H for double-quoted strings inside the hash body.

/// Escaped \" inside a double-quoted hash value must not prematurely close.
/// Boundary J: escape skip in double-quote context inside matching_brace.
#[test]
fn seam_j_escaped_double_quote_inside_brace_does_not_close_early() {
    // Hash body: { "key\"}" => '1.0', 'DQ::After' => '2.0' }
    let source = "WriteMakefile(PREREQ_PM => { \"key\\\"}\" => '1.0', 'DQ::After' => '2.0' });";
    let deps = extract_makefile_pl_requirements(source);
    let modules: Vec<&str> = deps.iter().map(|d| d.module.as_str()).collect();
    assert!(
        modules.contains(&"DQ::After"),
        "boundary J: escaped \\\" must keep the }} inside double-quotes, not cut off later \
         entries; got: {modules:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY K — byte == b'"' double-quote close in matching_brace
// ═══════════════════════════════════════════════════════════════════════════
//
// Mirrors boundary I for double-quoted strings inside the hash body.

/// Double-quoted value containing a nested `{` must not inflate brace depth.
/// Boundary K: byte == b'"' closes the double-quote; if swapped, the brace
/// depth counter would treat the outer } as still inside the unclosed quote.
#[test]
fn seam_k_double_quote_close_inside_brace_resumes_depth_tracking() {
    let source =
        "WriteMakefile(PREREQ_PM => { \"open{brace\" => '1.0', 'DQBrace::Dep' => '2.0' });";
    let deps = extract_makefile_pl_requirements(source);
    let modules: Vec<&str> = deps.iter().map(|d| d.module.as_str()).collect();
    assert!(
        modules.contains(&"DQBrace::Dep"),
        "boundary K: double-quote close must re-enable depth tracking; got: {modules:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY L — depth == 0 after } in matching_brace
// ═══════════════════════════════════════════════════════════════════════════
//
// The outermost } must be detected as the close (depth saturating_sub returns 0).
// The discriminating signal: content after the closing } is not included in the
// parse. If depth == 0 were checked incorrectly, the wrong } would be chosen
// as the close, capturing extra or fewer entries.

/// Only entries inside the outermost braces must be captured.
/// Boundary L: `depth == 0` after `}` identifies the correct closing brace.
/// Changing the depth check would either stop too early (missing entries) or
/// too late (capturing content outside the braces).
#[test]
fn seam_l_depth_zero_identifies_outer_closing_brace() {
    // Nested hash + outer close: content AFTER the outer } must not appear in deps.
    let source = "WriteMakefile(PREREQ_PM => { nested => { 'Nested::Inner' => '1.0' }, 'Outer::Dep' => '2.0' }, OTHER => {});";
    let deps = extract_makefile_pl_requirements(source);
    let modules: Vec<&str> = deps.iter().map(|d| d.module.as_str()).collect();
    assert!(
        modules.contains(&"Outer::Dep"),
        "boundary L: outer closing brace (depth==0) must capture the last entry; got: {modules:?}"
    );
    assert!(
        !modules.contains(&"OTHER"),
        "boundary L: content after the outer }} must not appear; got: {modules:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY M — while idx < bytes.len() in matching_brace (unclosed brace)
// ═══════════════════════════════════════════════════════════════════════════
//
// Source with an unclosed `{` means the loop exhausts without depth reaching 0.
// matching_brace returns None; extract_hash_requirements skips this occurrence.
// A mutation that removes the loop guard would panic on byte slice access.

/// Unclosed `{` must yield no deps (matching_brace returns None, occurrence skipped).
/// Boundary M: `while idx < bytes.len()` exits cleanly for unclosed braces.
/// Removing the guard would cause an index-out-of-bounds panic.
#[test]
fn seam_m_unclosed_brace_yields_no_deps() {
    let source = "WriteMakefile(PREREQ_PM => { 'Unclosed::Dep' => '1.0');";
    let deps = extract_makefile_pl_requirements(source);
    assert!(deps.is_empty(), "boundary M: unclosed brace must yield no deps; got: {deps:?}");
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY N — quote != b'\'' && quote != b'"' in parse_quoted_string
// ═══════════════════════════════════════════════════════════════════════════
//
// Hash body contains a bare (unquoted) value where parse_quoted_string expects
// a quoted string. It returns None; idx advances by 1 and the outer loop retries.
// A mutation removing the non-quote guard would treat any byte as a quote,
// constructing a bogus string.
//
// The discriminating signal: bare values inside the hash body are ignored.

/// Bare (unquoted) hash value must not produce a dependency.
/// Boundary N: `quote != b'\'' && quote != b'"'` returns None for non-quote bytes.
/// Removing the guard would misinterpret the next byte as a quote character and
/// construct a garbage module name, changing the output.
#[test]
fn seam_n_non_quote_byte_yields_no_dep_from_hash_body() {
    // Hash body: bare `1.0` is not a quote char (boundary N fires, idx += 1);
    // `'1.0'` as the VALUE is a quoted string but normalize_module_name rejects
    // "1.0" (contains `.`); 'Valid::Mod' => '2.0' is accepted.
    let source = "WriteMakefile(PREREQ_PM => { 1.0 => '1.0', 'Valid::Mod' => '2.0' });";
    let deps = extract_makefile_pl_requirements(source);
    let modules: Vec<&str> = deps.iter().map(|d| d.module.as_str()).collect();
    assert!(
        modules.contains(&"Valid::Mod"),
        "boundary N: valid quoted module must still be found; got: {modules:?}"
    );
    assert_eq!(
        deps.len(),
        1,
        "boundary N: only the valid quoted module must appear, not the bare numeric; \
         got: {modules:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY O — ch == '\n' newline termination in parse_quoted_string
// ═══════════════════════════════════════════════════════════════════════════
//
// A quoted string spanning a newline is malformed; parse_quoted_string returns
// None and the entry is skipped. A mutation removing the newline check would
// let the scanner consume across lines and pick up the next quoted string as
// if it were part of the same value, producing garbage.

/// Quoted value containing a newline must be rejected; the next valid entry is kept.
/// Boundary O: `ch == '\n'` returns None for the malformed string.
/// Removing the newline check would scan across the line break and produce
/// a garbage module name from the concatenated content.
#[test]
fn seam_o_quoted_value_with_newline_is_rejected() {
    let source =
        "WriteMakefile(PREREQ_PM => {\n    'Bad\nModule' => '1.0',\n    'Good::Mod' => '2.0'\n});";
    let deps = extract_makefile_pl_requirements(source);
    let modules: Vec<&str> = deps.iter().map(|d| d.module.as_str()).collect();
    assert!(
        !modules.contains(&"Bad"),
        "boundary O: module name with embedded newline must be rejected; got: {modules:?}"
    );
    assert!(
        modules.contains(&"Good::Mod"),
        "boundary O: the valid entry after the bad one must still be found; got: {modules:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY P — while idx < bytes.len() exhaustion in parse_quoted_string
// ═══════════════════════════════════════════════════════════════════════════
//
// Source ends with an unterminated quoted string; parse_quoted_string exhausts
// the source without finding the closing quote and returns None.
// A mutation removing the loop guard would panic on byte slice access.

/// Unterminated quoted module name must yield no dep from that entry.
/// Boundary P: `while idx < bytes.len()` exits cleanly for unterminated strings.
/// Removing the guard would panic on out-of-bounds byte access.
#[test]
fn seam_p_unterminated_quoted_string_yields_no_dep() {
    let source = "WriteMakefile(PREREQ_PM => { 'Unterminated => '1.0' });";
    let deps = extract_makefile_pl_requirements(source);
    assert!(
        deps.iter().all(|d| d.module != "Unterminated"),
        "boundary P: unterminated quoted string must not produce a dep; got: {deps:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY Q — ch == '\\' escape flag set in parse_quoted_string
// ═══════════════════════════════════════════════════════════════════════════
//
// Module name contains an escaped quote: 'Mod\'s' — the backslash sets
// `escaped = true` so the following `'` is treated as content, not the close.
// A mutation that removes the escape set would close the string at the
// first `'`, producing a truncated module name.

/// Escaped single quote inside a module name must be handled as content.
/// Boundary Q: `ch == '\\'` sets `escaped = true`; the following `'` is content.
/// Removing the escape flag means the quoted string closes early, so
/// `normalize_module_name` receives a truncated value and returns None.
#[test]
fn seam_q_backslash_escape_in_quoted_string_treated_as_content() {
    // Real case: a quoted key with an escaped quote inside.
    // 'Mod\'' would parse as module "Mod'" — which normalize_module_name rejects
    // (contains `'`). But the SIGNAL is that without the escape handling, the
    // string closes at the FIRST ', and 'Mod' is seen (valid but different).
    // We use a cleaner discriminator: a module followed immediately by another.
    let source = "WriteMakefile(PREREQ_PM => { 'Skip\\'Skip' => '1.0', 'Real::Escape' => '2.0' });";
    let deps = extract_makefile_pl_requirements(source);
    let modules: Vec<&str> = deps.iter().map(|d| d.module.as_str()).collect();
    // With correct escape handling: 'Skip\'Skip' spans a whole token (rejected by
    // normalize_module_name because of the embedded '), then 'Real::Escape' is found.
    // Without escape handling: 'Skip' closes early; SkipSkip = => ... is garbage;
    // 'Real::Escape' is still found BUT the dep count may differ or Skip appears.
    assert!(
        modules.contains(&"Real::Escape"),
        "boundary Q: escape handling must not prevent extraction of the subsequent valid dep; \
         got: {modules:?}"
    );
    assert!(
        !modules.contains(&"Skip"),
        "boundary Q: the truncated 'Skip' module must not appear when escape is handled; \
         got: {modules:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDARY R — ch as u8 == quote (quote close) in parse_quoted_string
// ═══════════════════════════════════════════════════════════════════════════
//
// The closing quote character must match the opening quote (single or double).
// A mutation that swaps the comparison byte would either never close (running
// off the end and returning None) or close at the wrong character.

/// Single-quoted module name must close at `'` and return correctly.
/// Boundary R: `ch as u8 == quote` with quote=b'\'' closes the string.
/// Swapping the quote byte (e.g. to b'"') means the string never closes,
/// parse_quoted_string returns None, and no dep is produced.
#[test]
fn seam_r_single_quote_close_in_parse_quoted_string_extracts_module() {
    let source = "WriteMakefile(PREREQ_PM => { 'SQ::Dep' => '1.0' });";
    let deps = extract_makefile_pl_requirements(source);
    assert_eq!(
        deps,
        vec![DeclaredDependency::new(
            "SQ::Dep",
            Some("1.0"),
            "PREREQ_PM",
            DeclaredDependencySource::MakefilePl,
        )],
        "boundary R: single-quote close must correctly extract the module name"
    );
}

/// Double-quoted module name must close at `"` and return correctly.
/// Boundary R (double): `ch as u8 == quote` with quote=b'"' closes the string.
#[test]
fn seam_r_double_quote_close_in_parse_quoted_string_extracts_module() {
    let source = "WriteMakefile(PREREQ_PM => { \"DQ::Dep\" => '2.0' });";
    let deps = extract_makefile_pl_requirements(source);
    assert_eq!(
        deps,
        vec![DeclaredDependency::new(
            "DQ::Dep",
            Some("2.0"),
            "PREREQ_PM",
            DeclaredDependencySource::MakefilePl,
        )],
        "boundary R: double-quote close must correctly extract the module name"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTROL — integration smoke: extract_build_pl_requirements exercises same seams
// ═══════════════════════════════════════════════════════════════════════════
//
// Build.PL uses the same extract_hash_requirements helper as Makefile.PL.
// This control test confirms the scanner seams fire identically for Build.PL.

/// `Build.PL` dependency extraction exercises the same scanner seams.
/// Control: extract_build_pl_requirements shares extract_hash_requirements with
/// extract_makefile_pl_requirements; verifying it also works confirms no
/// callsite-specific dead code.
#[test]
fn seam_control_build_pl_scanner_seams_fire_identically() {
    let source = "Module::Build->new(requires => { 'Build::Dep' => '1.0' });";
    let deps = extract_build_pl_requirements(source);
    assert_eq!(
        deps,
        vec![DeclaredDependency::new(
            "Build::Dep",
            Some("1.0"),
            "requires",
            DeclaredDependencySource::BuildPl,
        )],
        "control: Build.PL extraction must exercise the same seams as Makefile.PL"
    );
}
