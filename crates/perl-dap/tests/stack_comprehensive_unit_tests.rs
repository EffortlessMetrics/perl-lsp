//! Comprehensive unit tests for perl-dap-stack crate.
//!
//! Tests cover: StackFrame, Source, presentation hints, serde serialization,
//! PerlStackParser, FrameClassifier, and visibility filtering.

use perl_dap::stack::{
    FrameCategory, FrameClassifier, PerlFrameClassifier, PerlStackParser, Source,
    SourcePresentationHint, StackFrame, StackFramePresentationHint, StackParseError,
    filter_user_visible_frames, is_internal_frame, is_internal_frame_name_and_path,
};

// ─── StackFrame construction ────────────────────────────────────────────────

#[test]
fn stack_frame_new_sets_defaults() {
    let frame = StackFrame::new(5, "main::run", Some(Source::new("/app.pl")), 10);

    assert_eq!(frame.id, 5);
    assert_eq!(frame.name, "main::run");
    assert_eq!(frame.line, 10);
    assert_eq!(frame.column, 1);
    assert!(frame.end_line.is_none());
    assert!(frame.end_column.is_none());
    assert!(frame.can_restart.is_none());
    assert!(frame.presentation_hint.is_none());
    assert!(frame.module_id.is_none());
}

#[test]
fn stack_frame_default_is_unknown() {
    let frame = StackFrame::default();

    assert_eq!(frame.id, 0);
    assert_eq!(frame.name, "<unknown>");
    assert!(frame.source.is_none());
    assert_eq!(frame.line, 0);
    assert_eq!(frame.column, 1);
}

#[test]
fn stack_frame_for_subroutine_qualifies_name() {
    let frame = StackFrame::for_subroutine(1, "My::Package", "process", "/lib/My/Package.pm", 42);

    assert_eq!(frame.name, "My::Package::process");
    assert_eq!(frame.line, 42);
    assert_eq!(frame.file_path(), Some("/lib/My/Package.pm"));
}

#[test]
fn stack_frame_for_subroutine_main_uses_bare_name() {
    let frame = StackFrame::for_subroutine(1, "main", "run", "/script.pl", 1);
    assert_eq!(frame.name, "run");
}

#[test]
fn stack_frame_for_subroutine_empty_package_uses_bare_name() {
    let frame = StackFrame::for_subroutine(1, "", "init", "/startup.pl", 5);
    assert_eq!(frame.name, "init");
}

// ─── StackFrame builder methods ─────────────────────────────────────────────

#[test]
fn stack_frame_with_column() {
    let frame = StackFrame::new(1, "f", None, 1).with_column(15);
    assert_eq!(frame.column, 15);
}

#[test]
fn stack_frame_with_end() {
    let frame = StackFrame::new(1, "f", None, 10).with_end(20, 5);
    assert_eq!(frame.end_line, Some(20));
    assert_eq!(frame.end_column, Some(5));
}

#[test]
fn stack_frame_with_presentation_hint() {
    let frame =
        StackFrame::new(1, "f", None, 1).with_presentation_hint(StackFramePresentationHint::Label);
    assert_eq!(frame.presentation_hint, Some(StackFramePresentationHint::Label));
}

#[test]
fn stack_frame_with_module() {
    let frame = StackFrame::new(1, "f", None, 1).with_module("My::Module");
    assert_eq!(frame.module_id, Some("My::Module".to_string()));
}

#[test]
fn stack_frame_builder_chaining() {
    let frame = StackFrame::new(1, "test", Some(Source::new("/test.pl")), 10)
        .with_column(5)
        .with_end(15, 1)
        .with_presentation_hint(StackFramePresentationHint::Normal)
        .with_module("Test");

    assert_eq!(frame.column, 5);
    assert_eq!(frame.end_line, Some(15));
    assert_eq!(frame.end_column, Some(1));
    assert_eq!(frame.presentation_hint, Some(StackFramePresentationHint::Normal));
    assert_eq!(frame.module_id, Some("Test".to_string()));
}

// ─── StackFrame accessors ───────────────────────────────────────────────────

#[test]
fn stack_frame_qualified_name_returns_full_name() {
    let frame = StackFrame::new(1, "Foo::Bar::baz", None, 1);
    assert_eq!(frame.qualified_name(), "Foo::Bar::baz");
}

#[test]
fn stack_frame_file_path_with_source() {
    let frame = StackFrame::new(1, "f", Some(Source::new("/path/to/file.pm")), 1);
    assert_eq!(frame.file_path(), Some("/path/to/file.pm"));
}

#[test]
fn stack_frame_file_path_without_source() {
    let frame = StackFrame::new(1, "f", None, 1);
    assert_eq!(frame.file_path(), None);
}

#[test]
fn stack_frame_file_path_from_reference_source() {
    let source = Source::from_reference(42, "eval code");
    let frame = StackFrame::new(1, "f", Some(source), 1);
    // Reference sources have no path
    assert_eq!(frame.file_path(), None);
}

#[test]
fn stack_frame_is_user_code_without_hint() {
    let frame = StackFrame::new(1, "f", None, 1);
    assert!(frame.is_user_code());
}

#[test]
fn stack_frame_is_user_code_normal_hint() {
    let frame =
        StackFrame::new(1, "f", None, 1).with_presentation_hint(StackFramePresentationHint::Normal);
    assert!(frame.is_user_code());
}

#[test]
fn stack_frame_is_not_user_code_subtle_hint() {
    let frame =
        StackFrame::new(1, "f", None, 1).with_presentation_hint(StackFramePresentationHint::Subtle);
    assert!(!frame.is_user_code());
}

#[test]
fn stack_frame_is_user_code_label_hint() {
    let frame =
        StackFrame::new(1, "f", None, 1).with_presentation_hint(StackFramePresentationHint::Label);
    assert!(frame.is_user_code());
}

// ─── Source construction ────────────────────────────────────────────────────

#[test]
fn source_new_extracts_filename() {
    let source = Source::new("/path/to/module.pm");
    assert_eq!(source.path, Some("/path/to/module.pm".to_string()));
    assert_eq!(source.name, Some("module.pm".to_string()));
}

#[test]
fn source_new_bare_filename() {
    let source = Source::new("script.pl");
    assert_eq!(source.path, Some("script.pl".to_string()));
    assert_eq!(source.name, Some("script.pl".to_string()));
}

#[test]
fn source_new_deeply_nested_path() {
    let source = Source::new("/a/b/c/d/e/f.pm");
    assert_eq!(source.name, Some("f.pm".to_string()));
}

#[test]
fn source_from_reference_has_no_path() {
    let source = Source::from_reference(99, "dynamic source");
    assert!(source.path.is_none());
    assert_eq!(source.source_reference, Some(99));
    assert_eq!(source.name, Some("dynamic source".to_string()));
}

#[test]
fn source_default_is_empty() {
    let source = Source::default();
    assert!(source.name.is_none());
    assert!(source.path.is_none());
    assert!(source.source_reference.is_none());
    assert!(source.origin.is_none());
    assert!(source.presentation_hint.is_none());
}

// ─── Source builder methods ─────────────────────────────────────────────────

#[test]
fn source_with_origin() {
    let source = Source::new("/file.pl").with_origin("eval");
    assert_eq!(source.origin, Some("eval".to_string()));
}

#[test]
fn source_with_presentation_hint() {
    let source = Source::new("/file.pl").with_presentation_hint(SourcePresentationHint::Emphasize);
    assert_eq!(source.presentation_hint, Some(SourcePresentationHint::Emphasize));
}

// ─── Source::is_eval ────────────────────────────────────────────────────────

#[test]
fn source_is_eval_by_path() {
    assert!(Source::new("(eval 42)").is_eval());
}

#[test]
fn source_is_eval_by_path_with_bracket() {
    assert!(Source::new("(eval 10)[/path/file.pm:42]").is_eval());
}

#[test]
fn source_is_eval_by_origin() {
    assert!(Source::new("/normal/file.pl").with_origin("eval").is_eval());
}

#[test]
fn source_is_not_eval_for_regular_file() {
    assert!(!Source::new("/path/to/file.pl").is_eval());
}

#[test]
fn source_is_not_eval_for_reference_source() {
    assert!(!Source::from_reference(1, "code").is_eval());
}

// ─── Source::has_file ───────────────────────────────────────────────────────

#[test]
fn source_has_file_for_regular_path() {
    assert!(Source::new("/path/to/file.pl").has_file());
}

#[test]
fn source_has_no_file_for_eval() {
    assert!(!Source::new("(eval 42)").has_file());
}

#[test]
fn source_has_no_file_for_reference() {
    assert!(!Source::from_reference(1, "ref").has_file());
}

#[test]
fn source_has_no_file_for_eval_origin() {
    assert!(!Source::new("/file.pl").with_origin("eval").has_file());
}

// ─── Serde serialization ───────────────────────────────────────────────────

#[test]
fn stack_frame_serde_round_trip() -> Result<(), serde_json::Error> {
    let frame = StackFrame::new(1, "main::foo", Some(Source::new("/app.pl")), 42)
        .with_column(5)
        .with_end(50, 1)
        .with_presentation_hint(StackFramePresentationHint::Normal)
        .with_module("App");

    let json = serde_json::to_string(&frame)?;
    let deserialized: StackFrame = serde_json::from_str(&json)?;

    assert_eq!(deserialized.id, frame.id);
    assert_eq!(deserialized.name, frame.name);
    assert_eq!(deserialized.line, frame.line);
    assert_eq!(deserialized.column, frame.column);
    assert_eq!(deserialized.end_line, frame.end_line);
    assert_eq!(deserialized.end_column, frame.end_column);
    assert_eq!(deserialized.presentation_hint, frame.presentation_hint);
    assert_eq!(deserialized.module_id, frame.module_id);
    Ok(())
}

#[test]
fn stack_frame_serde_skips_none_fields() -> Result<(), serde_json::Error> {
    let frame = StackFrame::new(1, "test", None, 1);
    let json = serde_json::to_string(&frame)?;

    // Optional None fields should not appear in JSON
    assert!(!json.contains("endLine"));
    assert!(!json.contains("endColumn"));
    assert!(!json.contains("canRestart"));
    assert!(!json.contains("presentationHint"));
    assert!(!json.contains("moduleId"));
    assert!(!json.contains("source"));
    Ok(())
}

#[test]
fn stack_frame_serde_uses_camel_case() -> Result<(), serde_json::Error> {
    let frame = StackFrame::new(1, "f", None, 1).with_end(10, 5).with_module("mod");

    let json = serde_json::to_string(&frame)?;

    assert!(json.contains("endLine"));
    assert!(json.contains("endColumn"));
    assert!(json.contains("moduleId"));
    // Should NOT have snake_case
    assert!(!json.contains("end_line"));
    assert!(!json.contains("end_column"));
    assert!(!json.contains("module_id"));
    Ok(())
}

#[test]
fn source_serde_round_trip() -> Result<(), serde_json::Error> {
    let source = Source::new("/path/file.pm")
        .with_origin("require")
        .with_presentation_hint(SourcePresentationHint::Deemphasize);

    let json = serde_json::to_string(&source)?;
    let deserialized: Source = serde_json::from_str(&json)?;

    assert_eq!(deserialized.path, source.path);
    assert_eq!(deserialized.name, source.name);
    assert_eq!(deserialized.origin, source.origin);
    assert_eq!(deserialized.presentation_hint, source.presentation_hint);
    Ok(())
}

#[test]
fn source_serde_skips_none_fields() -> Result<(), serde_json::Error> {
    let source = Source::default();
    let json = serde_json::to_string(&source)?;

    assert!(!json.contains("name"));
    assert!(!json.contains("path"));
    assert!(!json.contains("sourceReference"));
    assert!(!json.contains("origin"));
    assert!(!json.contains("presentationHint"));
    Ok(())
}

#[test]
fn presentation_hint_serde_values() -> Result<(), serde_json::Error> {
    let normal = serde_json::to_string(&StackFramePresentationHint::Normal)?;
    let label = serde_json::to_string(&StackFramePresentationHint::Label)?;
    let subtle = serde_json::to_string(&StackFramePresentationHint::Subtle)?;

    assert_eq!(normal, "\"normal\"");
    assert_eq!(label, "\"label\"");
    assert_eq!(subtle, "\"subtle\"");
    Ok(())
}

#[test]
fn source_presentation_hint_serde_values() -> Result<(), serde_json::Error> {
    let normal = serde_json::to_string(&SourcePresentationHint::Normal)?;
    let emphasize = serde_json::to_string(&SourcePresentationHint::Emphasize)?;
    let deemphasize = serde_json::to_string(&SourcePresentationHint::Deemphasize)?;

    assert_eq!(normal, "\"normal\"");
    assert_eq!(emphasize, "\"emphasize\"");
    assert_eq!(deemphasize, "\"deemphasize\"");
    Ok(())
}

// ─── StackFrame equality ────────────────────────────────────────────────────

#[test]
fn stack_frame_equality() {
    let a = StackFrame::new(1, "foo", Some(Source::new("/a.pl")), 10);
    let b = StackFrame::new(1, "foo", Some(Source::new("/a.pl")), 10);
    assert_eq!(a, b);
}

#[test]
fn stack_frame_inequality_different_id() {
    let a = StackFrame::new(1, "foo", None, 10);
    let b = StackFrame::new(2, "foo", None, 10);
    assert_ne!(a, b);
}

#[test]
fn stack_frame_inequality_different_line() {
    let a = StackFrame::new(1, "foo", None, 10);
    let b = StackFrame::new(1, "foo", None, 20);
    assert_ne!(a, b);
}

#[test]
fn stack_frame_clone() {
    let frame = StackFrame::new(1, "foo", Some(Source::new("/a.pl")), 10)
        .with_module("Mod")
        .with_presentation_hint(StackFramePresentationHint::Normal);
    let cloned = frame.clone();
    assert_eq!(frame, cloned);
}

// ─── PerlStackParser ────────────────────────────────────────────────────────

#[test]
fn parser_standard_frame_format() {
    let mut parser = PerlStackParser::new();
    let frame = parser.parse_frame("  #0  main::foo at /script.pl line 42", 0);

    assert!(frame.is_some());
    let f = frame.as_ref();
    assert_eq!(f.map(|f| f.name.as_str()), Some("main::foo"));
    assert_eq!(f.map(|f| f.line), Some(42));
    assert_eq!(f.and_then(|f| f.file_path()), Some("/script.pl"));
}

#[test]
fn parser_verbose_frame_with_args() {
    let mut parser = PerlStackParser::new();
    let line = "$ = My::Mod::func('arg1', 42) called from file `/lib/My/Mod.pm' line 100";
    let frame = parser.parse_frame(line, 0);

    assert!(frame.is_some());
    let f = frame.as_ref();
    assert_eq!(f.map(|f| f.name.as_str()), Some("My::Mod::func"));
    assert_eq!(f.map(|f| f.line), Some(100));
}

#[test]
fn parser_simple_frame_no_args() {
    let mut parser = PerlStackParser::new();
    let line = ". = main::run() called from '-e' line 1";
    let frame = parser.parse_frame(line, 0);

    assert!(frame.is_some());
    assert_eq!(frame.as_ref().map(|f| f.name.as_str()), Some("main::run"));
}

#[test]
fn parser_context_format_main() {
    let mut parser = PerlStackParser::new();
    let frame = parser.parse_frame("main::(script.pl):42:", 0);

    assert!(frame.is_some());
    let f = frame.as_ref();
    assert_eq!(f.map(|f| f.name.as_str()), Some("main"));
    assert_eq!(f.map(|f| f.line), Some(42));
}

#[test]
fn parser_eval_context() {
    let mut parser = PerlStackParser::new();
    let frame = parser.parse_frame("(eval 10)[/path/file.pm:42]", 0);

    assert!(frame.is_some());
    let f = frame.as_ref();
    assert!(f.is_some_and(|f| f.name.contains("eval 10")));
    assert_eq!(f.map(|f| f.line), Some(42));
    assert!(f.is_some_and(|f| f.source.as_ref().is_some_and(|s| s.is_eval())));
    assert_eq!(
        f.and_then(|f| f.presentation_hint.clone()),
        Some(StackFramePresentationHint::Label)
    );
}

#[test]
fn parser_returns_none_for_unrecognized() {
    let mut parser = PerlStackParser::new();
    assert!(parser.parse_frame("random garbage text", 0).is_none());
    assert!(parser.parse_frame("", 0).is_none());
    assert!(parser.parse_frame("   ", 0).is_none());
}

#[test]
fn parser_multi_line_stack_trace() {
    let mut parser = PerlStackParser::new();
    let output = "\
$ = A::B::first() called from file `/lib/A/B.pm' line 10
$ = C::D::second() called from file `/lib/C/D.pm' line 20
$ = main::entry() called from file `app.pl' line 5
";

    let frames = parser.parse_stack_trace(output);

    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].name, "A::B::first");
    assert_eq!(frames[1].name, "C::D::second");
    assert_eq!(frames[2].name, "main::entry");
    // Auto-assigned sequential IDs
    assert_eq!(frames[0].id, 1);
    assert_eq!(frames[1].id, 2);
    assert_eq!(frames[2].id, 3);
}

#[test]
fn parser_multi_line_skips_blank_lines() {
    let mut parser = PerlStackParser::new();
    let output = "\n\n$ = main::foo() called from file `a.pl' line 1\n\n$ = main::bar() called from file `b.pl' line 2\n";

    let frames = parser.parse_stack_trace(output);
    assert_eq!(frames.len(), 2);
}

#[test]
fn parser_multi_line_resets_auto_ids() {
    let mut parser = PerlStackParser::new().with_starting_id(100);

    let _ = parser.parse_stack_trace("$ = a::b() called from file `x.pl' line 1");

    // Second call should reset IDs back to 1
    let frames = parser.parse_stack_trace("$ = c::d() called from file `y.pl' line 2");
    assert_eq!(frames[0].id, 1);
}

#[test]
fn parser_custom_starting_id() {
    let mut parser = PerlStackParser::new().with_starting_id(50);
    let frame = parser.parse_frame("  #0  main::foo at a.pl line 1", 0);
    assert_eq!(frame.map(|f| f.id), Some(50));
}

#[test]
fn parser_manual_id_from_frame_number() {
    let mut parser = PerlStackParser::new().with_auto_ids(false);
    let frame = parser.parse_frame("  #7  main::foo at a.pl line 1", 0);
    assert_eq!(frame.map(|f| f.id), Some(7));
}

#[test]
fn parser_manual_id_falls_back_to_provided() {
    let mut parser = PerlStackParser::new().with_auto_ids(false);
    // Verbose format has no frame number
    let frame = parser.parse_frame("$ = main::foo() called from file `a.pl' line 1", 99);
    assert_eq!(frame.map(|f| f.id), Some(99));
}

#[test]
fn parser_parse_context_method() {
    let parser = PerlStackParser::new();
    let result = parser.parse_context("main::(file.pm):100:");

    assert!(result.is_some());
    let r = result.as_ref();
    assert_eq!(r.map(|t| t.0.as_str()), Some("main"));
    assert_eq!(r.map(|t| t.1.as_str()), Some("file.pm"));
    assert_eq!(r.map(|t| t.2), Some(100));
}

#[test]
fn parser_parse_context_returns_none_for_garbage() {
    let parser = PerlStackParser::new();
    assert!(parser.parse_context("not a context line").is_none());
}

#[test]
fn parser_looks_like_frame_positive() {
    assert!(PerlStackParser::looks_like_frame("  #0  main::foo at script.pl line 10"));
    assert!(PerlStackParser::looks_like_frame("$ = foo() called from file 'x' line 1"));
    assert!(PerlStackParser::looks_like_frame("@ = bar() called from file 'y' line 2"));
    assert!(PerlStackParser::looks_like_frame(". = baz() called from file 'z' line 3"));
}

#[test]
fn parser_looks_like_frame_negative() {
    assert!(!PerlStackParser::looks_like_frame("some random text"));
    assert!(!PerlStackParser::looks_like_frame(""));
    assert!(!PerlStackParser::looks_like_frame("my $x = 1;"));
}

// ─── FrameCategory ──────────────────────────────────────────────────────────

#[test]
fn frame_category_presentation_hints() {
    assert_eq!(FrameCategory::User.presentation_hint(), StackFramePresentationHint::Normal);
    assert_eq!(FrameCategory::Library.presentation_hint(), StackFramePresentationHint::Subtle);
    assert_eq!(FrameCategory::Core.presentation_hint(), StackFramePresentationHint::Subtle);
    assert_eq!(FrameCategory::Eval.presentation_hint(), StackFramePresentationHint::Label);
    assert_eq!(FrameCategory::Unknown.presentation_hint(), StackFramePresentationHint::Subtle);
}

#[test]
fn frame_category_is_user_code() {
    assert!(FrameCategory::User.is_user_code());
    assert!(!FrameCategory::Library.is_user_code());
    assert!(!FrameCategory::Core.is_user_code());
    assert!(!FrameCategory::Eval.is_user_code());
    assert!(!FrameCategory::Unknown.is_user_code());
}

#[test]
fn frame_category_is_external() {
    assert!(!FrameCategory::User.is_external());
    assert!(FrameCategory::Library.is_external());
    assert!(FrameCategory::Core.is_external());
    assert!(!FrameCategory::Eval.is_external());
    assert!(!FrameCategory::Unknown.is_external());
}

// ─── PerlFrameClassifier ───────────────────────────────────────────────────

fn frame_with_path(path: &str) -> StackFrame {
    StackFrame::new(1, "test", Some(Source::new(path)), 1)
}

#[test]
fn classifier_user_code_default() {
    let classifier = PerlFrameClassifier::new();
    assert_eq!(
        classifier.classify(&frame_with_path("/home/user/project/lib/App.pm")),
        FrameCategory::User
    );
    assert_eq!(classifier.classify(&frame_with_path("./script.pl")), FrameCategory::User);
    assert_eq!(classifier.classify(&frame_with_path("bin/myapp")), FrameCategory::User);
}

#[test]
fn classifier_core_modules() {
    let classifier = PerlFrameClassifier::new();

    // Path-based detection
    assert_eq!(
        classifier.classify(&frame_with_path("/usr/lib/perl5/strict.pm")),
        FrameCategory::Core
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/usr/share/perl/5.30/Exporter.pm")),
        FrameCategory::Core
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/usr/lib/perl/warnings.pm")),
        FrameCategory::Core
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/usr/lib/perl5/Carp.pm")),
        FrameCategory::Core
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/usr/share/perl5/base.pm")),
        FrameCategory::Core
    );
    assert_eq!(classifier.classify(&frame_with_path("/opt/perl5/parent.pm")), FrameCategory::Core);

    // site_perl and vendor_perl are core paths
    assert_eq!(
        classifier.classify(&frame_with_path("/usr/lib/perl5/site_perl/Foo.pm")),
        FrameCategory::Core
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/usr/lib/perl5/vendor_perl/Bar.pm")),
        FrameCategory::Core
    );
}

#[test]
fn classifier_library_code() {
    let classifier = PerlFrameClassifier::new();

    assert_eq!(
        classifier.classify(&frame_with_path("/home/user/project/extlib/Moose.pm")),
        FrameCategory::Library
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/home/user/.cpanm/work/Foo.pm")),
        FrameCategory::Library
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/home/user/project/local/lib/perl5/Moo.pm")),
        FrameCategory::Core // local/lib/perl5 matches core pattern
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/home/user/project/fatlib/Module.pm")),
        FrameCategory::Library
    );
    assert_eq!(
        classifier.classify(&frame_with_path("/vendor/perl/Module.pm")),
        FrameCategory::Core // vendor_perl not matched, but /perl/ matched
    );
}

#[test]
fn classifier_eval_code() {
    let classifier = PerlFrameClassifier::new();

    assert_eq!(classifier.classify(&frame_with_path("(eval 42)")), FrameCategory::Eval);
    assert_eq!(
        classifier.classify(&frame_with_path("(eval 10)[script.pl:5]")),
        FrameCategory::Eval
    );

    // Eval via origin
    let mut frame = frame_with_path("/file.pm");
    frame.source = Some(Source::new("/file.pm").with_origin("eval"));
    assert_eq!(classifier.classify(&frame), FrameCategory::Eval);
}

#[test]
fn classifier_no_source_is_unknown() {
    let classifier = PerlFrameClassifier::new();
    let frame = StackFrame::new(1, "test", None, 1);
    assert_eq!(classifier.classify(&frame), FrameCategory::Unknown);
}

#[test]
fn classifier_explicit_user_path_overrides() {
    let classifier = PerlFrameClassifier::new().with_user_path("/my/project/");

    // A path that would normally be library due to local/lib/ pattern
    let frame = frame_with_path("/my/project/local/lib/perl5/MyModule.pm");
    assert_eq!(classifier.classify(&frame), FrameCategory::User);
}

#[test]
fn classifier_explicit_library_path() {
    let classifier = PerlFrameClassifier::new().with_library_path("/opt/custom/libs/");

    let frame = frame_with_path("/opt/custom/libs/MyLib.pm");
    assert_eq!(classifier.classify(&frame), FrameCategory::Library);
}

#[test]
fn classifier_multiple_custom_paths() {
    let classifier = PerlFrameClassifier::new()
        .with_user_path("/project/src/")
        .with_user_path("/project/lib/")
        .with_library_path("/project/vendor/");

    assert_eq!(classifier.classify(&frame_with_path("/project/src/App.pm")), FrameCategory::User);
    assert_eq!(classifier.classify(&frame_with_path("/project/lib/Util.pm")), FrameCategory::User);
    assert_eq!(
        classifier.classify(&frame_with_path("/project/vendor/External.pm")),
        FrameCategory::Library
    );
}

#[test]
fn classifier_apply_classification_sets_hint() {
    let classifier = PerlFrameClassifier::new();

    let core_frame = frame_with_path("/usr/lib/perl5/strict.pm");
    let classified = classifier.apply_classification(core_frame);
    assert_eq!(classified.presentation_hint, Some(StackFramePresentationHint::Subtle));

    let user_frame = frame_with_path("/project/app.pl");
    let classified = classifier.apply_classification(user_frame);
    assert_eq!(classified.presentation_hint, Some(StackFramePresentationHint::Normal));
}

#[test]
fn classifier_classify_all_includes_all() {
    let classifier = PerlFrameClassifier::new();
    let frames = vec![
        frame_with_path("/project/app.pl"),
        frame_with_path("/usr/lib/perl5/strict.pm"),
        frame_with_path("/project/lib/Module.pm"),
    ];

    let all = classifier.classify_all(frames, true);
    assert_eq!(all.len(), 3);
}

#[test]
fn classifier_classify_all_excludes_external() {
    let classifier = PerlFrameClassifier::new();
    let frames = vec![
        frame_with_path("/project/app.pl"),
        frame_with_path("/usr/lib/perl5/strict.pm"),
        frame_with_path("/project/lib/Module.pm"),
    ];

    let user_only = classifier.classify_all(frames, false);
    assert_eq!(user_only.len(), 2);
    assert!(user_only.iter().all(|f| f.is_user_code()));
}

// ─── Visibility filtering ───────────────────────────────────────────────────

#[test]
fn is_internal_frame_db_prefix() {
    let frame = StackFrame::new(1, "DB::sub", Some(Source::new("/app.pl")), 1);
    assert!(is_internal_frame(&frame));
}

#[test]
fn is_internal_frame_devel_tsperlap() {
    let frame = StackFrame::new(1, "Devel::TSPerlDAP::handler", Some(Source::new("/shim.pm")), 1);
    assert!(is_internal_frame(&frame));
}

#[test]
fn is_internal_frame_perl5db_path() {
    let frame =
        StackFrame::new(1, "helper_func", Some(Source::new("/usr/lib/perl5/perl5db.pl")), 1);
    assert!(is_internal_frame(&frame));
}

#[test]
fn is_not_internal_regular_frame() {
    let frame = StackFrame::new(1, "main::run", Some(Source::new("/app.pl")), 1);
    assert!(!is_internal_frame(&frame));
}

#[test]
fn is_not_internal_frame_no_source() {
    let frame = StackFrame::new(1, "main::run", None, 1);
    assert!(!is_internal_frame(&frame));
}

#[test]
fn is_internal_frame_name_and_path_combinations() {
    assert!(is_internal_frame_name_and_path("DB::sub", Some("/app.pl")));
    assert!(is_internal_frame_name_and_path("DB::anything", None));
    assert!(is_internal_frame_name_and_path("Devel::TSPerlDAP::x", Some("/a.pl")));
    assert!(is_internal_frame_name_and_path("some_func", Some("/usr/perl5db.pl")));
    assert!(!is_internal_frame_name_and_path("main::run", Some("/app.pl")));
    assert!(!is_internal_frame_name_and_path("main::run", None));
}

#[test]
fn filter_user_visible_frames_removes_internal() {
    let frames = vec![
        StackFrame::new(1, "main::start", Some(Source::new("/app.pl")), 1),
        StackFrame::new(2, "DB::sub", Some(Source::new("/perl5db.pl")), 50),
        StackFrame::new(3, "App::Util::helper", Some(Source::new("/lib/Util.pm")), 20),
        StackFrame::new(4, "Devel::TSPerlDAP::shim", Some(Source::new("/shim.pm")), 1),
    ];

    let visible = filter_user_visible_frames(frames);
    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].name, "main::start");
    assert_eq!(visible[1].name, "App::Util::helper");
}

#[test]
fn filter_user_visible_frames_preserves_order() {
    let frames = vec![
        StackFrame::new(1, "a::b", Some(Source::new("/a.pl")), 1),
        StackFrame::new(2, "c::d", Some(Source::new("/c.pl")), 2),
        StackFrame::new(3, "e::f", Some(Source::new("/e.pl")), 3),
    ];

    let visible = filter_user_visible_frames(frames);
    assert_eq!(visible.len(), 3);
    assert_eq!(visible[0].id, 1);
    assert_eq!(visible[1].id, 2);
    assert_eq!(visible[2].id, 3);
}

#[test]
fn filter_user_visible_frames_empty_input() {
    let visible = filter_user_visible_frames(vec![]);
    assert!(visible.is_empty());
}

#[test]
fn filter_user_visible_frames_all_internal() {
    let frames = vec![
        StackFrame::new(1, "DB::sub", Some(Source::new("/db.pl")), 1),
        StackFrame::new(2, "DB::eval", Some(Source::new("/db.pl")), 2),
    ];

    let visible = filter_user_visible_frames(frames);
    assert!(visible.is_empty());
}

// ─── StackParseError ────────────────────────────────────────────────────────

#[test]
fn stack_parse_error_display() {
    let err = StackParseError::UnrecognizedFormat("bad input".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("unrecognized"));
    assert!(msg.contains("bad input"));
}

// ─── Edge cases ─────────────────────────────────────────────────────────────

#[test]
fn stack_frame_negative_line() {
    let frame = StackFrame::new(1, "f", None, -1);
    assert_eq!(frame.line, -1);
}

#[test]
fn stack_frame_zero_id() {
    let frame = StackFrame::new(0, "f", None, 0);
    assert_eq!(frame.id, 0);
}

#[test]
fn stack_frame_unicode_name() {
    let frame = StackFrame::new(1, "Módule::αβγ", None, 1);
    assert_eq!(frame.qualified_name(), "Módule::αβγ");
}

#[test]
fn source_unicode_path() {
    let source = Source::new("/home/ユーザー/プロジェクト/script.pl");
    assert_eq!(source.name, Some("script.pl".to_string()));
    assert!(source.has_file());
}

#[test]
fn stack_frame_debug_impl() {
    let frame = StackFrame::new(1, "main::test", None, 42);
    let debug = format!("{:?}", frame);
    assert!(debug.contains("main::test"));
    assert!(debug.contains("42"));
}

#[test]
fn source_debug_impl() {
    let source = Source::new("/test.pl");
    let debug = format!("{:?}", source);
    assert!(debug.contains("test.pl"));
}

#[test]
fn frame_category_debug_impl() {
    let debug = format!("{:?}", FrameCategory::User);
    assert_eq!(debug, "User");
}

#[test]
fn parser_with_unknown_frames_builder() {
    let parser = PerlStackParser::new().with_unknown_frames(true);
    // Just verify it doesn't panic; the flag is used internally
    let debug = format!("{:?}", parser);
    assert!(debug.contains("true"));
}

// ─── Serde deserialization from raw JSON ────────────────────────────────────

#[test]
fn stack_frame_deserialize_from_json_object() -> Result<(), serde_json::Error> {
    let json = r#"{
        "id": 1,
        "name": "main::handler",
        "line": 42,
        "column": 1,
        "source": {
            "name": "app.pl",
            "path": "/home/user/app.pl"
        }
    }"#;

    let frame: StackFrame = serde_json::from_str(json)?;
    assert_eq!(frame.id, 1);
    assert_eq!(frame.name, "main::handler");
    assert_eq!(frame.line, 42);
    assert_eq!(frame.file_path(), Some("/home/user/app.pl"));
    assert!(frame.end_line.is_none());
    Ok(())
}

#[test]
fn source_deserialize_minimal() -> Result<(), serde_json::Error> {
    let json = r#"{}"#;
    let source: Source = serde_json::from_str(json)?;
    assert!(source.name.is_none());
    assert!(source.path.is_none());
    Ok(())
}

#[test]
fn stack_frame_deserialize_with_all_fields() -> Result<(), serde_json::Error> {
    let json = r#"{
        "id": 5,
        "name": "My::Module::process",
        "line": 100,
        "column": 10,
        "endLine": 110,
        "endColumn": 1,
        "canRestart": true,
        "presentationHint": "subtle",
        "moduleId": "My::Module",
        "source": {
            "name": "Module.pm",
            "path": "/lib/My/Module.pm",
            "sourceReference": 42,
            "origin": "require",
            "presentationHint": "deemphasize"
        }
    }"#;

    let frame: StackFrame = serde_json::from_str(json)?;
    assert_eq!(frame.id, 5);
    assert_eq!(frame.name, "My::Module::process");
    assert_eq!(frame.line, 100);
    assert_eq!(frame.column, 10);
    assert_eq!(frame.end_line, Some(110));
    assert_eq!(frame.end_column, Some(1));
    assert_eq!(frame.can_restart, Some(true));
    assert_eq!(frame.presentation_hint, Some(StackFramePresentationHint::Subtle));
    assert_eq!(frame.module_id, Some("My::Module".to_string()));
    assert!(!frame.is_user_code());

    let source = frame.source.as_ref();
    assert!(source.is_some());
    let s = source;
    assert_eq!(s.and_then(|s| s.source_reference), Some(42));
    assert_eq!(s.and_then(|s| s.origin.as_deref()), Some("require"));
    assert_eq!(
        s.and_then(|s| s.presentation_hint.clone()),
        Some(SourcePresentationHint::Deemphasize)
    );
    Ok(())
}
