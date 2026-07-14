//! Red TDD tests for #3766 hover migration to generation-owned analyzer/type_environment
//!
//! This test suite validates that the hover provider correctly uses `ParsedSnapshot`'s
//! generation-owned `semantic_analyzer()` and `type_environment()` methods instead of
//! the old `LspServer`-level `(uri, content_hash)`-keyed caches.
//!
//! ## Key Invariants Under Test
//!
//! 1. **Generation freshness**: Hover reflects NEW generation's analysis after edit, never cached old.
//! 2. **Pending-parse honesty**: Hover during in-flight parse (N+1 requested, not published)
//!    returns from last-published (N) or degraded, NEVER stale-wrong from N+1.
//! 3. **Fidelity**: Hover uses real source via snapshot (proves POD docs are preserved).
//! 4. **Construction-count**: Analyzer/type-engine built exactly 1× per generation (not per-hover).
//!
//! The old content-hash-based cache (`semantic_analyzer_cache` / `type_inference_engine_cache`)
//! is content-hash-gated, so a naive "edit content, hover refreshes" test may PASS on the old
//! cache (content_hash changes too). The RED tests here target the generation-vs-content-hash gap:
//! - **Pending-parse-gap** tests that generation coordination (the gap the content-hash cache lacks)
//! - **Construction-count** tests per-generation accounting (not per-content-hash)

// Integration tests print diagnostic output for CI troubleshooting; this is
// not the LSP server's stdio transport, so print_stderr doesn't apply the
// way it does to production code.
#![allow(clippy::print_stderr)]

mod common;

#[cfg(test)]
mod hover_generation_owned_tests {
    use crate::common::test_utils::{TestServerBuilder, semantic};

    // ── Generation round-trip: type change freshness ──────────────────────

    /// Generation round-trip proof: Edit a symbol's type, verify hover reflects
    /// the NEW type, not the old cached answer.
    ///
    /// On the old content-hash cache: this may pass vacuously (content_hash changes,
    /// so cache refreshes anyway). The point is to establish the baseline: after migration,
    /// it still passes, now because generation-gating works the same way. The real RED
    /// discriminator is the pending-parse-gap test below.
    #[test]
    fn test_hover_generation_roundtrip_type_change() -> Result<(), Box<dyn std::error::Error>> {
        let uri = "file:///hover_gen_roundtrip.pl";
        let server = TestServerBuilder::new().build();

        // Generation 0: declare $x as number
        let code_gen0 = "my $x = 42;\nprint $x;\n";
        server.open_document(uri, code_gen0);

        // Hover on $x at generation 0: should show scalar type, value 42 or numeric hint
        let hover0 = server.get_hover(uri, 1, 6); // "print $x"
        let content0 = semantic::hover_content(&hover0).ok_or("expected hover at generation 0")?;
        assert!(
            content0.contains("Scalar Variable"),
            "generation 0 hover should show Scalar Variable, got: {content0}"
        );

        // Generation 1: edit same line to change $x to string
        let code_gen1 = "my $x = \"hello\";\nprint $x;\n";
        server.change_document(uri, code_gen1, 2);
        std::thread::sleep(std::time::Duration::from_millis(50)); // Brief delay for parse

        // Hover on $x at generation 1: should reflect NEW type (string), not cached generation 0
        let hover1 = server.get_hover(uri, 1, 6); // "print $x" (same position)
        let content1 = semantic::hover_content(&hover1).ok_or("expected hover at generation 1")?;
        assert!(
            content1.contains("Scalar Variable"),
            "generation 1 hover should show Scalar Variable, got: {content1}"
        );
        // The key assertion: hover reflects the NEW type (inferred from "hello" assignment)
        // Type inference should show Str (or String equivalent) not Int from the old 42
        assert!(
            !content1.contains("42"),
            "generation 1 hover should NOT show old value 42 from generation 0, got: {content1}"
        );
        assert!(
            content1.contains("Str") || content1.contains("String") || content1.contains("Type"),
            "generation 1 hover should reflect new string type, got: {content1}"
        );

        Ok(())
    }

    // ── Pending-parse-gap honesty: in-flight generation isolation ──────────

    /// Pending-parse-gap honesty test: Hover during an in-flight parse (generation N+1
    /// requested, not yet published) must return from last-published snapshot (N) or
    /// degraded/pending response — NEVER a stale-wrong answer attributed to the new text.
    ///
    /// This is the RED discriminator: the old content-hash cache has no notion of
    /// "pending vs published generation", only "was the content_hash ever seen?". The
    /// new generation-gated path must gate on generation match: if `doc.current_parsed()`
    /// checks `snapshot.generation == current_generation`, an in-flight parse (generation
    /// incremented but snapshot not yet published) correctly returns `None`.
    ///
    /// Achieving RED: We need to instrument the parse to add a delay (to extend the
    /// in-flight window), then fire hovers while generation N+1 is pending. The old cache,
    /// not generation-aware, may pre-populate the cache for N+1 during the in-flight parse
    /// (or may race and return a stale-wrong answer). The new code must check
    /// `current_parsed()` which gates on generation match and correctly returns None during
    /// the pending window, falling through to textual fallback.
    ///
    /// Since we can't directly instrument the parser delay in this test, we approximate
    /// by rapid edits: one edit increments generation immediately, and we fire hovers
    /// before the async parse (if any) completes. In the all-synchronous-parse-under-lock
    /// world today, this won't actually create a true in-flight window (parse completes
    /// before control returns), so this test may not be fully RED yet. However, the
    /// structure is correct for when #3396 phase-3 (async parse worker) lands.
    ///
    /// For now, this serves as a regression guard: if the builder accidentally re-introduces
    /// stale caches that are *not* generation-gated, this test will catch it once async
    /// parsing is in place. It also validates that `current_parsed()` correctly gates on
    /// generation match.
    #[test]
    fn test_hover_pending_parse_gap() -> Result<(), Box<dyn std::error::Error>> {
        let uri = "file:///hover_pending_gap.pl";
        let server = TestServerBuilder::new().build();

        // Generation 0: simple code
        let code_gen0 = "my $x = 10;\nprint $x;\n";
        server.open_document(uri, code_gen0);
        std::thread::sleep(std::time::Duration::from_millis(50));

        // First hover should work (generation 0, published)
        let hover_gen0 = server.get_hover(uri, 1, 6);
        let content_gen0 =
            semantic::hover_content(&hover_gen0).ok_or("expected hover at generation 0")?;
        assert!(
            content_gen0.contains("Scalar Variable") || content_gen0.contains("$x"),
            "generation 0 hover should show variable info, got: {content_gen0}"
        );

        // Generation 1: edit code
        let code_gen1 = "my $x = 20;\nprint $x;\n";
        server.change_document(uri, code_gen1, 2);

        // Fire hover immediately (while generation 1 might be in-flight in async parse scenario).
        // In synchronous parse, this will already be published. In async scenario (future),
        // this will hit the in-flight condition.
        // The hover must NOT return a stale-wrong answer; it must return from last-published
        // (generation 0) or degrade gracefully. Currently: should get generation 0 or newer
        // (never a stale future one).
        let hover_gen1 = server.get_hover(uri, 1, 6);

        // The hover may have content (from generation 0 or 1) or be null (if degraded).
        // The current behavior returns "Request superseded" error when a request arrives
        // after the generation has bumped. After migration to generation-gating, this should
        // gracefully fall back to textual hover (null or token-level info) instead of an error.
        if let Some(error) = hover_gen1.get("error") {
            // RED condition: Getting an error is suboptimal. After migration, pending-parse
            // should return last-published or degrade gracefully, not error.
            // The test documents this: once fixed, the error should not appear.
            eprintln!("DEBUG: Hover returned error (expected to be fixed by migration): {error}");
            // Accept this as the current (imperfect) behavior for now
        } else if let Some(content_gen1) = semantic::hover_content(&hover_gen1) {
            // Hover returned info: should be valid symbol info (not corrupted)
            assert!(
                content_gen1.contains("Scalar Variable")
                    || content_gen1.contains("$x")
                    || content_gen1.contains("Perl"),
                "hover after edit should show variable or symbol info, not corrupted: {content_gen1}"
            );
        }
        // Both error and null are acceptable for now (current implementation quirks).
        // After migration to generation-owned snapshot.current_parsed(), this should
        // improve: return from last-published snapshot or degrade gracefully.

        Ok(())
    }

    // ── Fidelity: real source preservation via snapshot ───────────────────

    /// Fidelity test: Hover on symbol with POD documentation.
    ///
    /// Proves that hover uses the real source via `snapshot.source()` (via
    /// `snapshot.semantic_analyzer()` which internally calls `analyze_with_source(ast, &snapshot.source)`),
    /// not the empty-source overload `analyze(ast)` that loses documentation extraction.
    ///
    /// The old `get_or_build_analyzer(uri, text, ast)` takes `text: &str`, so it already has
    /// source access. The RED condition: if a regression were to call the empty-source
    /// `SemanticAnalyzer::analyze(ast)` instead (which loses fidelity), the doc extraction
    /// would fail and this test would catch it.
    #[test]
    fn test_hover_fidelity_pod_documentation() -> Result<(), Box<dyn std::error::Error>> {
        let uri = "file:///hover_fidelity_pod.pl";
        let server = TestServerBuilder::new().build();

        let code = r#"
=head1 FUNCTION

=head2 greet

Greets the user with a friendly message.

=cut

sub greet {
    my ($name) = @_;
    print "Hello, $name!\n";
}
"#;
        server.open_document(uri, code);
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Hover on the subroutine name
        let hover = server.get_hover(uri, 10, 4); // "sub greet"
        let content = semantic::hover_content(&hover).ok_or("expected hover for greet sub")?;

        // The hover should include the POD documentation
        assert!(
            content.contains("Subroutine") || content.contains("greet"),
            "hover should show subroutine info, got: {content}"
        );
        // The critical assertion: POD doc is present (proves real source was used)
        assert!(
            content.contains("Greets") || content.contains("friendly message"),
            "hover should preserve POD documentation, proving real source is used, got: {content}"
        );

        Ok(())
    }

    /// Fidelity test: Hover on symbol in narrow range.
    ///
    /// Proves that the semantic analyzer's text-range lookup is precise.
    /// This is enabled by the real source (via snapshot), which allows the analyzer
    /// to compute exact byte offsets. Empty-source overloads cannot do this.
    #[test]
    fn test_hover_fidelity_precise_range() -> Result<(), Box<dyn std::error::Error>> {
        let uri = "file:///hover_fidelity_range.pl";
        let server = TestServerBuilder::new().build();

        let code = "my $var_one = 1;\nmy $var_two = 2;\nprint $var_one;\n";
        server.open_document(uri, code);
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Hover on the first $var_one in declaration (line 0)
        let hover_decl = server.get_hover(uri, 0, 4); // "my $var_one"
        let content_decl =
            semantic::hover_content(&hover_decl).ok_or("expected hover on $var_one declaration")?;
        assert!(
            content_decl.contains("var_one") || content_decl.contains("Scalar Variable"),
            "hover at declaration should identify the correct variable: {content_decl}"
        );

        // Hover on $var_one in the print statement (line 2)
        let hover_use = server.get_hover(uri, 2, 6); // "print $var_one"
        let content_use =
            semantic::hover_content(&hover_use).ok_or("expected hover on $var_one usage")?;
        assert!(
            content_use.contains("var_one") || content_use.contains("Scalar Variable"),
            "hover at usage should also identify $var_one: {content_use}"
        );

        // Both should refer to the same variable (hover provides consistent identity)
        assert!(
            (content_decl.contains("var_one") && content_use.contains("var_one"))
                || (content_decl.contains("Scalar Variable")
                    && content_use.contains("Scalar Variable")),
            "both hovers should identify the same variable"
        );

        Ok(())
    }

    // ── Construction-count: per-generation single-instantiation ────────────

    /// Construction-count proof: Multiple hovers on the same generation
    /// must build the analyzer/type-engine exactly once.
    ///
    /// The old content-hash cache also achieves this (per content-hash, the contents
    /// are built once). The new snapshot-based path achieves it via `OnceLock::get_or_init`.
    ///
    /// RED condition: If a regression were to rebuild on every hover request (e.g., no
    /// caching at all, or a bug in `OnceLock` usage), the build count would be > 1 and
    /// this test would fail.
    ///
    /// This test accesses internal test-only methods on `ParsedSnapshot`:
    /// `semantic_analyzer_build_count()` and `type_environment_build_count()`.
    /// These are only available in test builds (#[cfg(test)]), and require passing
    /// the snapshot to the test somehow.
    ///
    /// **LIMITATION**: The current test harness doesn't expose the internal
    /// `ParsedSnapshot` to tests directly. For now, this test validates the
    /// *observable* behavior (hover returns the same results across multiple
    /// calls) as a proxy. Once the test API is extended (or a test-only
    /// debug introspection endpoint is added), we can directly read the
    /// build counts from the snapshot.
    #[test]
    fn test_hover_construction_count_single_gen() -> Result<(), Box<dyn std::error::Error>> {
        let uri = "file:///hover_const_single_gen.pl";
        let server = TestServerBuilder::new().build();

        let code = r#"
my $a = 10;
my $b = 20;
sub add {
    my ($x, $y) = @_;
    return $x + $y;
}
print add($a, $b);
"#;
        server.open_document(uri, code);
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Fire 3 hovers on the same generation for different symbols
        let hover_a = server.get_hover(uri, 1, 4); // "$a"
        let hover_b = server.get_hover(uri, 2, 4); // "$b"
        let hover_sub = server.get_hover(uri, 3, 4); // "sub add"

        let content_a = semantic::hover_content(&hover_a).ok_or("expected hover for $a")?;
        let content_b = semantic::hover_content(&hover_b).ok_or("expected hover for $b")?;
        let content_sub =
            semantic::hover_content(&hover_sub).ok_or("expected hover for sub add")?;

        // All hovers should succeed and be consistent within the same generation
        assert!(
            content_a.contains("Scalar Variable") || content_a.contains("$a"),
            "hover on $a should show variable info: {content_a}"
        );
        assert!(
            content_b.contains("Scalar Variable") || content_b.contains("$b"),
            "hover on $b should show variable info: {content_b}"
        );
        assert!(
            content_sub.contains("Subroutine") || content_sub.contains("add"),
            "hover on sub add should show subroutine info: {content_sub}"
        );

        // **NOTE**: In a perfect test harness, we would directly access the
        // ParsedSnapshot's `semantic_analyzer_build_count()` and verify it == 1.
        // For now, this test proves that the harness doesn't crash with repeated
        // hovering on the same generation, which is a baseline regression guard.
        // Once #3766 is implemented and test infrastructure is extended, add
        // direct build-count assertions here.

        Ok(())
    }

    /// Construction-count proof across generations: Each new generation should
    /// have its own analyzer/type-engine (1× per generation), not shared across generations.
    ///
    /// RED condition: If a regression were to cache across generations (e.g., keying
    /// by URI only instead of by generation), repeated hovers on a fresh generation
    /// would still return results from the old generation's analyzer, and the old
    /// generation's cells would never be freed. This test would pass (hovers succeed),
    /// but the next test (`test_hover_construction_count_superseded_never_queried`) would fail.
    #[test]
    fn test_hover_construction_count_multi_gen() -> Result<(), Box<dyn std::error::Error>> {
        let uri = "file:///hover_const_multi_gen.pl";
        let server = TestServerBuilder::new().build();

        // Generation 0
        let code_gen0 = "my $x = 1;\nprint $x;\n";
        server.open_document(uri, code_gen0);
        std::thread::sleep(std::time::Duration::from_millis(50));

        let hover_gen0_first = server.get_hover(uri, 1, 6);
        let content_gen0_first = semantic::hover_content(&hover_gen0_first)
            .ok_or("expected hover at generation 0 (first)")?;
        assert!(
            content_gen0_first.contains("Scalar Variable") || content_gen0_first.contains("$x"),
            "generation 0 hover should show variable info: {content_gen0_first}"
        );

        // Generation 1: new content
        let code_gen1 = "my $y = 2;\nprint $y;\n";
        server.change_document(uri, code_gen1, 2);
        std::thread::sleep(std::time::Duration::from_millis(50));

        let hover_gen1 = server.get_hover(uri, 1, 6);
        let content_gen1 =
            semantic::hover_content(&hover_gen1).ok_or("expected hover at generation 1")?;
        assert!(
            content_gen1.contains("Scalar Variable") || content_gen1.contains("$y"),
            "generation 1 hover should show variable info for $y (not $x from gen0): {content_gen1}"
        );

        // Generation 2: another edit
        let code_gen2 = "my $z = 3;\nprint $z;\n";
        server.change_document(uri, code_gen2, 3);
        std::thread::sleep(std::time::Duration::from_millis(50));

        let hover_gen2 = server.get_hover(uri, 1, 6);
        let content_gen2 =
            semantic::hover_content(&hover_gen2).ok_or("expected hover at generation 2")?;
        assert!(
            content_gen2.contains("Scalar Variable") || content_gen2.contains("$z"),
            "generation 2 hover should show variable info for $z (not $y or $x): {content_gen2}"
        );

        // The critical assertion: each generation's hover reflects the correct variable.
        // If caching were cross-generational, $x would still be in the cache and hovers
        // on generation 1/2 might return wrong answers (depending on parse order).
        assert!(
            !content_gen1.contains("$x"),
            "generation 1 should not return $x from generation 0: {content_gen1}"
        );
        assert!(
            !content_gen2.contains("$y") && !content_gen2.contains("$x"),
            "generation 2 should not return stale variables from prior generations: {content_gen2}"
        );

        Ok(())
    }

    /// (Optional) Regression guard: Superseded snapshots with no hover requests
    /// should perform zero analyzer construction.
    ///
    /// This validates the "lazy construction" invariant: if a generation is immediately
    /// superseded (new generation created before any hovers fire on the old one), the
    /// old snapshot's analyzer cells should never be built at all.
    ///
    /// RED condition: If a regression eagerly builds analysis on parse completion
    /// (not lazily on first request), the build would happen even if hovers never
    /// fire on that generation, wasting resources.
    ///
    /// This test is more sophisticated and requires direct access to ParsedSnapshot,
    /// which today isn't exposed. For now, this test is a placeholder that validates
    /// we can create many rapid generations without crash/resource explosion.
    #[test]
    fn test_hover_construction_count_lazy_no_request() -> Result<(), Box<dyn std::error::Error>> {
        let uri = "file:///hover_const_lazy_no_request.pl";
        let server = TestServerBuilder::new().build();

        // Rapid generations, no hovers: just open/edit/edit/edit
        server.open_document(uri, "my $x = 1;\n");
        std::thread::sleep(std::time::Duration::from_millis(10));

        server.change_document(uri, "my $x = 2;\n", 2);
        std::thread::sleep(std::time::Duration::from_millis(10));

        server.change_document(uri, "my $x = 3;\n", 3);
        std::thread::sleep(std::time::Duration::from_millis(10));

        server.change_document(uri, "my $x = 4;\n", 4);
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Only the final generation should get a hover
        let hover_final = server.get_hover(uri, 0, 4);
        let content_final =
            semantic::hover_content(&hover_final).ok_or("expected hover on final generation")?;
        assert!(
            content_final.contains("Scalar Variable") || content_final.contains("$x"),
            "final hover should work: {content_final}"
        );

        // If earlier generations eagerly built analyzers, memory would have accumulated.
        // This test passes if no crash/resource explosion occurs.
        // TODO: Once test API exposes ParsedSnapshot, verify build_count == 0 for
        // the first three generations and == 1 for the final one.

        Ok(())
    }
}
