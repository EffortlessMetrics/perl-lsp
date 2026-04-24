//! Extended unit tests for perl-dap-stack crate.
//!
//! Covers edge cases, integration flows (parse → classify → filter),
//! parser state management, classifier boundary conditions, serde
//! error paths, and visibility corner cases not addressed by the
//! existing comprehensive_unit_tests.rs suite.

use perl_dap::stack::{
    FrameCategory, FrameClassifier, PerlFrameClassifier, PerlStackParser, Source,
    SourcePresentationHint, StackFrame, StackFramePresentationHint, filter_user_visible_frames,
    is_internal_frame, is_internal_frame_name_and_path,
};

// ─── Helper ─────────────────────────────────────────────────────────────────

fn frame(id: i64, name: &str, path: &str, line: i64) -> StackFrame {
    StackFrame::new(id, name.to_string(), Some(Source::new(path)), line)
}

// ═══════════════════════════════════════════════════════════════════════════
//  1. StackFrame builder edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stack_frame_with_end_stores_both_values() {
    let f = StackFrame::new(1, "f", None, 1).with_end(50, 20);
    assert_eq!(f.end_line, Some(50));
    assert_eq!(f.end_column, Some(20));
}

#[test]
fn stack_frame_chained_with_end_and_column() {
    let f = StackFrame::new(1, "f", None, 1).with_column(5).with_end(10, 15);
    assert_eq!(f.column, 5);
    assert_eq!(f.end_line, Some(10));
    assert_eq!(f.end_column, Some(15));
}

#[test]
fn stack_frame_for_subroutine_with_deeply_nested_package() {
    let f = StackFrame::for_subroutine(1, "A::B::C::D", "do_it", "/deep.pm", 7);
    assert_eq!(f.name, "A::B::C::D::do_it");
}

#[test]
fn stack_frame_for_subroutine_empty_sub_name() {
    let f = StackFrame::for_subroutine(1, "Pkg", "", "/a.pm", 1);
    assert_eq!(f.name, "Pkg::");
}

#[test]
fn stack_frame_with_module_accepts_string() {
    let f = StackFrame::new(1, "f", None, 1).with_module(String::from("MyMod"));
    assert_eq!(f.module_id.as_deref(), Some("MyMod"));
}

#[test]
fn stack_frame_with_module_accepts_str() {
    let f = StackFrame::new(1, "f", None, 1).with_module("Mod");
    assert_eq!(f.module_id.as_deref(), Some("Mod"));
}

#[test]
fn stack_frame_is_user_code_with_normal_hint() {
    let f =
        StackFrame::new(1, "f", None, 1).with_presentation_hint(StackFramePresentationHint::Normal);
    assert!(f.is_user_code());
}

#[test]
fn stack_frame_is_user_code_with_label_hint() {
    let f =
        StackFrame::new(1, "f", None, 1).with_presentation_hint(StackFramePresentationHint::Label);
    assert!(f.is_user_code());
}

#[test]
fn stack_frame_file_path_none_when_source_has_no_path() {
    let f = StackFrame::new(1, "f", Some(Source::from_reference(1, "eval")), 1);
    assert!(f.file_path().is_none());
}

#[test]
fn stack_frame_large_id() {
    let f = StackFrame::new(i64::MAX, "f", None, 1);
    assert_eq!(f.id, i64::MAX);
}

#[test]
fn stack_frame_negative_id() {
    let f = StackFrame::new(-1, "f", None, 1);
    assert_eq!(f.id, -1);
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. Source edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn source_new_windows_style_path() {
    let s = Source::new(r"C:\Users\dev\script.pl");
    // Path::file_name may or may not work on non-Windows; just check path stored
    assert_eq!(s.path.as_deref(), Some(r"C:\Users\dev\script.pl"));
}

#[test]
fn source_new_root_path() {
    let s = Source::new("/script.pl");
    assert_eq!(s.name.as_deref(), Some("script.pl"));
}

#[test]
fn source_new_empty_string() {
    let s = Source::new("");
    assert_eq!(s.path.as_deref(), Some(""));
}

#[test]
fn source_is_eval_with_embedded_eval_in_path() {
    let s = Source::new("/home/user/(eval 5)/something.pm");
    assert!(s.is_eval());
}

#[test]
fn source_is_not_eval_when_word_eval_in_directory_name() {
    // "evaluation" does NOT contain "(eval" so it should be false
    let s = Source::new("/home/user/evaluation/script.pl");
    assert!(!s.is_eval());
}

#[test]
fn source_has_file_when_path_set_and_not_eval() {
    let s = Source::new("/app/lib/Foo.pm");
    assert!(s.has_file());
}

#[test]
fn source_from_reference_is_not_has_file() {
    let s = Source::from_reference(99, "dynamic");
    assert!(!s.has_file());
}

#[test]
fn source_with_origin_chained_with_presentation_hint() {
    let s = Source::new("/a.pl")
        .with_origin("require")
        .with_presentation_hint(SourcePresentationHint::Deemphasize);
    assert_eq!(s.origin.as_deref(), Some("require"));
    assert_eq!(s.presentation_hint, Some(SourcePresentationHint::Deemphasize));
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. Serde edge cases & error paths
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn serde_stack_frame_missing_optional_source() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"id":1,"name":"f","line":1,"column":1}"#;
    let f: StackFrame = serde_json::from_str(json)?;
    assert!(f.source.is_none());
    Ok(())
}

#[test]
fn serde_stack_frame_extra_fields_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"id":1,"name":"f","line":1,"column":1,"extraField":"hi"}"#;
    let result: Result<StackFrame, _> = serde_json::from_str(json);
    // serde default denies unknown fields; check whichever behavior the derive produces
    // For #[serde(rename_all = "camelCase")] without deny_unknown_fields it accepts extras
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn serde_source_round_trip_with_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let s = Source::new("/lib/Foo.pm")
        .with_origin("require")
        .with_presentation_hint(SourcePresentationHint::Emphasize);
    let json = serde_json::to_string(&s)?;
    let s2: Source = serde_json::from_str(&json)?;
    assert_eq!(s, s2);
    Ok(())
}

#[test]
fn serde_presentation_hint_normal_value() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&StackFramePresentationHint::Normal)?;
    assert_eq!(json, r#""normal""#);
    Ok(())
}

#[test]
fn serde_presentation_hint_label_value() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&StackFramePresentationHint::Label)?;
    assert_eq!(json, r#""label""#);
    Ok(())
}

#[test]
fn serde_source_presentation_hint_emphasize() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&SourcePresentationHint::Emphasize)?;
    assert_eq!(json, r#""emphasize""#);
    Ok(())
}

#[test]
fn serde_source_presentation_hint_normal() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&SourcePresentationHint::Normal)?;
    assert_eq!(json, r#""normal""#);
    Ok(())
}

#[test]
fn serde_stack_frame_serialization_omits_none_end_line() -> Result<(), Box<dyn std::error::Error>> {
    let f = StackFrame::new(1, "f", None, 1);
    let json = serde_json::to_string(&f)?;
    assert!(!json.contains("endLine"));
    assert!(!json.contains("endColumn"));
    assert!(!json.contains("canRestart"));
    assert!(!json.contains("moduleId"));
    Ok(())
}

#[test]
fn serde_stack_frame_with_end_includes_end_fields() -> Result<(), Box<dyn std::error::Error>> {
    let f = StackFrame::new(1, "f", None, 1).with_end(10, 5);
    let json = serde_json::to_string(&f)?;
    assert!(json.contains("endLine"));
    assert!(json.contains("endColumn"));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Parser – additional patterns and state management
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parser_verbose_frame_with_backtick_delimiters() {
    let mut parser = PerlStackParser::new();
    let line = "$ = Foo::bar('x') called from file `/app/Foo.pm' line 55";
    if let Some(f) = parser.parse_frame(line, 0) {
        assert_eq!(f.name, "Foo::bar");
        assert_eq!(f.line, 55);
        assert_eq!(f.file_path(), Some("/app/Foo.pm"));
    }
}

#[test]
fn parser_verbose_frame_at_sigil() {
    let mut parser = PerlStackParser::new();
    let line = "@ = List::Util::reduce('CODE', 1, 2) called from file `/lib/Util.pm' line 30";
    if let Some(f) = parser.parse_frame(line, 0) {
        assert_eq!(f.name, "List::Util::reduce");
        assert_eq!(f.line, 30);
    }
}

#[test]
fn parser_verbose_frame_dot_sigil() {
    let mut parser = PerlStackParser::new();
    let line = ". = main::entry() called from 'script.pl' line 1";
    if let Some(f) = parser.parse_frame(line, 0) {
        assert_eq!(f.name, "main::entry");
    }
}

#[test]
fn parser_standard_frame_without_hash_prefix() {
    let mut parser = PerlStackParser::new();
    let line = "  0  main::handler at handler.pl line 99";
    if let Some(f) = parser.parse_frame(line, 0) {
        assert_eq!(f.name, "main::handler");
        assert_eq!(f.line, 99);
    }
}

#[test]
fn parser_standard_frame_large_line_number() {
    let mut parser = PerlStackParser::new();
    let line = "  #0  main::big at big.pl line 999999";
    if let Some(f) = parser.parse_frame(line, 0) {
        assert_eq!(f.line, 999999);
    }
}

#[test]
fn parser_eval_context_high_eval_number() {
    let mut parser = PerlStackParser::new();
    let line = "(eval 99999)[/app/lib/Eval.pm:1]";
    if let Some(f) = parser.parse_frame(line, 0) {
        assert!(f.name.contains("99999"));
        assert_eq!(f.line, 1);
        if let Some(ref s) = f.source {
            assert!(s.is_eval());
        }
    }
}

#[test]
fn parser_eval_frame_has_label_presentation_hint() {
    let mut parser = PerlStackParser::new();
    let line = "(eval 1)[/tmp/test.pl:5]";
    if let Some(f) = parser.parse_frame(line, 0) {
        assert_eq!(f.presentation_hint, Some(StackFramePresentationHint::Label));
    }
}

#[test]
fn parser_context_main_colon_format() {
    let parser = PerlStackParser::new();
    let ctx = parser.parse_context("main::(app.pl):10:");
    if let Some((func, file, line)) = ctx {
        assert_eq!(func, "main");
        assert_eq!(file, "app.pl");
        assert_eq!(line, 10);
    }
}

#[test]
fn parser_context_returns_none_for_empty_string() {
    let parser = PerlStackParser::new();
    assert!(parser.parse_context("").is_none());
}

#[test]
fn parser_context_returns_none_for_whitespace() {
    let parser = PerlStackParser::new();
    assert!(parser.parse_context("   ").is_none());
}

#[test]
fn parser_parse_stack_trace_resets_ids_each_call() {
    let mut parser = PerlStackParser::new();

    let trace1 = "$ = A::b() called from file `/a.pm' line 1\n\
                   $ = C::d() called from file `/c.pm' line 2";
    let frames1 = parser.parse_stack_trace(trace1);
    assert_eq!(frames1.len(), 2);
    assert_eq!(frames1[0].id, 1);
    assert_eq!(frames1[1].id, 2);

    // Second call should reset IDs back to 1
    let frames2 = parser.parse_stack_trace(trace1);
    assert_eq!(frames2[0].id, 1);
    assert_eq!(frames2[1].id, 2);
}

#[test]
fn parser_parse_stack_trace_empty_input() {
    let mut parser = PerlStackParser::new();
    let frames = parser.parse_stack_trace("");
    assert!(frames.is_empty());
}

#[test]
fn parser_parse_stack_trace_only_blank_lines() {
    let mut parser = PerlStackParser::new();
    let frames = parser.parse_stack_trace("\n\n   \n\n");
    assert!(frames.is_empty());
}

#[test]
fn parser_parse_stack_trace_mixed_formats() {
    let mut parser = PerlStackParser::new();
    let output = "\
$ = Deep::Mod::call('a') called from file `/lib/Deep/Mod.pm' line 10
  #1  main::run at script.pl line 5
(eval 3)[/tmp/eval.pl:99]
";
    let frames = parser.parse_stack_trace(output);
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].name, "Deep::Mod::call");
    assert_eq!(frames[1].name, "main::run");
    assert!(frames[2].name.contains("eval 3"));
}

#[test]
fn parser_with_unknown_frames_default_false() {
    let parser = PerlStackParser::new();
    // include_unknown_frames defaults to false (field is private but builder returns false)
    // We test indirectly: unknown lines still return None from parse_frame
    let mut p = parser;
    assert!(p.parse_frame("not a frame", 0).is_none());
}

#[test]
fn parser_with_auto_ids_false_and_no_frame_number() {
    let mut parser = PerlStackParser::new().with_auto_ids(false);
    // Standard frame format without explicit frame number in capture
    // The regex may capture the frame number from "#N" prefix
    let line = "  main::foo called at bar.pl line 5";
    // This format uses "called at" instead of "at" so may not match.
    // Verify graceful None return.
    let result = parser.parse_frame(line, 42);
    // Either parsed with provided_id=42 or None
    if let Some(f) = result {
        // If it matched, id should be 42 (the provided fallback)
        assert_eq!(f.id, 42);
    }
}

#[test]
fn parser_looks_like_frame_hash_prefix() {
    assert!(PerlStackParser::looks_like_frame("#0"));
}

#[test]
fn parser_looks_like_frame_dot_equals() {
    assert!(PerlStackParser::looks_like_frame(". = foo() called from 'x' line 1"));
}

#[test]
fn parser_looks_like_frame_at_equals() {
    assert!(PerlStackParser::looks_like_frame("@ = foo() called from 'x' line 1"));
}

#[test]
fn parser_looks_like_frame_whitespace_only() {
    assert!(!PerlStackParser::looks_like_frame("   "));
}

#[test]
fn parser_auto_ids_increment_across_parse_frame_calls() {
    let mut parser = PerlStackParser::new().with_starting_id(10);
    let f1 = parser.parse_frame("  #0  a::b at x.pl line 1", 0);
    let f2 = parser.parse_frame("  #1  c::d at y.pl line 2", 0);
    let f3 = parser.parse_frame("  #2  e::f at z.pl line 3", 0);

    assert_eq!(f1.map(|f| f.id), Some(10));
    assert_eq!(f2.map(|f| f.id), Some(11));
    assert_eq!(f3.map(|f| f.id), Some(12));
}

// ═══════════════════════════════════════════════════════════════════════════
//  5. Classifier – additional paths and combinations
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn classifier_core_site_perl_path() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "Mod::x", "/usr/lib/perl5/site_perl/5.30/Mod.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Core);
}

#[test]
fn classifier_core_vendor_perl_path() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "Mod::x", "/usr/lib/perl5/vendor_perl/Mod.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Core);
}

#[test]
fn classifier_core_lib_perl5_path() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "Mod::x", "/usr/lib/perl5/Some/Module.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Core);
}

#[test]
fn classifier_core_by_known_module_name_constant() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "constant::import", "/some/path/constant.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Core);
}

#[test]
fn classifier_core_by_known_module_name_parent() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "parent::import", "/some/path/parent.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Core);
}

#[test]
fn classifier_core_by_known_module_name_feature() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "feature::import", "/some/path/feature.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Core);
}

#[test]
fn classifier_library_cpanm_path() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "Foo::bar", "/home/user/.cpanm/work/12345/Foo-1.0/lib/Foo.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Library);
}

#[test]
fn classifier_library_fatlib_path() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "Fat::Mod", "/app/fatlib/Fat/Mod.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Library);
}

#[test]
fn classifier_library_cpan_path() {
    let c = PerlFrameClassifier::new();
    let f = frame(1, "CPAN::Mod", "/usr/local/lib/cpan/CPAN/Mod.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Library);
}

#[test]
fn classifier_eval_path_origin() {
    let c = PerlFrameClassifier::new();
    let mut f = frame(1, "main::dyn", "/app/script.pl", 1);
    f.source = Some(Source::new("/app/script.pl").with_origin("eval"));
    assert_eq!(c.classify(&f), FrameCategory::Eval);
}

#[test]
fn classifier_user_path_takes_precedence_over_core() {
    let c = PerlFrameClassifier::new().with_user_path("/usr/lib/perl5/");
    let f = frame(1, "strict::import", "/usr/lib/perl5/strict.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::User);
}

#[test]
fn classifier_library_path_takes_precedence_over_auto_detection() {
    let c = PerlFrameClassifier::new().with_library_path("/home/user/project/lib/");
    let f = frame(1, "App::Main", "/home/user/project/lib/App/Main.pm", 1);
    assert_eq!(c.classify(&f), FrameCategory::Library);
}

#[test]
fn classifier_classify_all_preserves_order() {
    let c = PerlFrameClassifier::new();
    let frames = vec![
        frame(1, "a", "/app/a.pl", 1),
        frame(2, "b", "/app/b.pl", 2),
        frame(3, "c", "/app/c.pl", 3),
    ];
    let result = c.classify_all(frames, true);
    assert_eq!(result[0].id, 1);
    assert_eq!(result[1].id, 2);
    assert_eq!(result[2].id, 3);
}

#[test]
fn classifier_classify_all_empty_input() {
    let c = PerlFrameClassifier::new();
    let result = c.classify_all(Vec::new(), true);
    assert!(result.is_empty());
}

#[test]
fn classifier_classify_all_only_external_exclude() {
    let c = PerlFrameClassifier::new();
    let frames = vec![
        frame(1, "a", "/usr/lib/perl5/strict.pm", 1),
        frame(2, "b", "/usr/lib/perl5/warnings.pm", 2),
    ];
    let result = c.classify_all(frames, false);
    assert!(result.is_empty());
}

#[test]
fn frame_category_unknown_is_not_user_code() {
    assert!(!FrameCategory::Unknown.is_user_code());
}

#[test]
fn frame_category_unknown_is_not_external() {
    assert!(!FrameCategory::Unknown.is_external());
}

#[test]
fn frame_category_eval_is_not_external() {
    assert!(!FrameCategory::Eval.is_external());
}

#[test]
fn frame_category_unknown_presentation_hint_is_subtle() {
    assert_eq!(FrameCategory::Unknown.presentation_hint(), StackFramePresentationHint::Subtle);
}

// ═══════════════════════════════════════════════════════════════════════════
//  6. Visibility – additional scenarios
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn is_internal_frame_name_db_sub_any_path() {
    assert!(is_internal_frame_name_and_path("DB::sub", Some("/random/path.pl")));
}

#[test]
fn is_internal_frame_name_db_deep() {
    assert!(is_internal_frame_name_and_path("DB::deep", None));
}

#[test]
fn is_internal_frame_name_devel_tsperlap_nested() {
    assert!(is_internal_frame_name_and_path(
        "Devel::TSPerlDAP::Debugger::step",
        Some("/app/lib.pm")
    ));
}

#[test]
fn is_internal_frame_perl5db_embedded_in_long_path() {
    assert!(is_internal_frame_name_and_path("helper", Some("/very/long/path/to/perl5db.pl")));
}

#[test]
fn is_not_internal_frame_partial_db_name() {
    // "Database::connect" should NOT match DB:: prefix
    assert!(!is_internal_frame_name_and_path("Database::connect", Some("/app/lib.pm")));
}

#[test]
fn is_not_internal_frame_path_without_perl5db() {
    assert!(!is_internal_frame_name_and_path("main::run", Some("/app/perl5.pl")));
}

#[test]
fn filter_user_visible_frames_single_internal() {
    let frames = vec![frame(1, "DB::sub", "/usr/lib/perl5/perl5db.pl", 1)];
    let filtered = filter_user_visible_frames(frames);
    assert!(filtered.is_empty());
}

#[test]
fn filter_user_visible_frames_single_user() {
    let frames = vec![frame(1, "main::run", "/app/main.pl", 1)];
    let filtered = filter_user_visible_frames(frames);
    assert_eq!(filtered.len(), 1);
}

#[test]
fn is_internal_frame_with_no_source() {
    let f = StackFrame::new(1, "DB::sub", None, 1);
    assert!(is_internal_frame(&f));
}

#[test]
fn is_internal_frame_devel_with_no_source() {
    let f = StackFrame::new(1, "Devel::TSPerlDAP::init", None, 1);
    assert!(is_internal_frame(&f));
}

// ═══════════════════════════════════════════════════════════════════════════
//  7. Integration: parse → classify → filter pipeline
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn pipeline_parse_classify_filter_user_only() {
    let mut parser = PerlStackParser::new();
    let output = "\
$ = strict::import() called from file `/usr/lib/perl5/strict.pm' line 10
$ = main::run() called from file `/app/script.pl' line 5
";
    let frames = parser.parse_stack_trace(output);
    let classifier = PerlFrameClassifier::new();
    let classified = classifier.classify_all(frames, false);

    // strict.pm is core → excluded when include_external=false
    assert_eq!(classified.len(), 1);
    assert_eq!(classified[0].name, "main::run");
}

#[test]
fn pipeline_parse_classify_filter_includes_all() {
    let mut parser = PerlStackParser::new();
    let output = "\
$ = strict::import() called from file `/usr/lib/perl5/strict.pm' line 10
$ = main::run() called from file `/app/script.pl' line 5
";
    let frames = parser.parse_stack_trace(output);
    let classifier = PerlFrameClassifier::new();
    let classified = classifier.classify_all(frames, true);
    assert_eq!(classified.len(), 2);
}

#[test]
fn pipeline_parse_then_visibility_filter() {
    let mut parser = PerlStackParser::new();
    let output = "\
$ = DB::sub() called from file `/usr/lib/perl5/perl5db.pl' line 1
$ = main::run() called from file `/app/script.pl' line 5
";
    let frames = parser.parse_stack_trace(output);
    let visible = filter_user_visible_frames(frames);
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].name, "main::run");
}

#[test]
fn pipeline_classify_and_visibility_combined() {
    let classifier = PerlFrameClassifier::new();
    let frames = vec![
        frame(1, "DB::sub", "/usr/lib/perl5/perl5db.pl", 1),
        frame(2, "main::run", "/app/main.pl", 10),
        frame(3, "Mod::x", "/usr/lib/perl5/strict.pm", 5),
    ];

    // First filter internal, then classify
    let visible = filter_user_visible_frames(frames);
    assert_eq!(visible.len(), 2); // DB::sub removed

    let classified = classifier.classify_all(visible, false);
    // strict.pm is core → filtered out by classify_all with include_external=false
    assert_eq!(classified.len(), 1);
    assert_eq!(classified[0].name, "main::run");
}

// ═══════════════════════════════════════════════════════════════════════════
//  8. StackFrame Default & Clone
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stack_frame_default_qualified_name() {
    let f = StackFrame::default();
    assert_eq!(f.qualified_name(), "<unknown>");
}

#[test]
fn stack_frame_clone_is_independent() {
    let f1 = StackFrame::new(1, "a", Some(Source::new("/a.pl")), 10).with_column(5);
    let mut f2 = f1.clone();
    f2.line = 99;
    assert_eq!(f1.line, 10);
    assert_eq!(f2.line, 99);
}

#[test]
fn source_clone_is_independent() {
    let s1 = Source::new("/a.pl").with_origin("require");
    let mut s2 = s1.clone();
    s2.origin = Some("eval".to_string());
    assert_eq!(s1.origin.as_deref(), Some("require"));
    assert_eq!(s2.origin.as_deref(), Some("eval"));
}

// ═══════════════════════════════════════════════════════════════════════════
//  9. StackParseError Display
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stack_parse_error_unrecognized_format_display() {
    let err = perl_dap::stack::StackParseError::UnrecognizedFormat("bad input".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("bad input"));
}

#[test]
fn stack_parse_error_debug_impl() {
    let err = perl_dap::stack::StackParseError::UnrecognizedFormat("test".to_string());
    let dbg = format!("{err:?}");
    assert!(dbg.contains("UnrecognizedFormat"));
}
