//! Perl::Critic integration for code quality analysis
//!
//! This module provides integration with Perl::Critic for static code analysis
//! and policy enforcement in Perl code.

mod analyzer;
mod built_in;
mod quick_fix;
mod types;

pub use analyzer::CriticAnalyzer;
pub use built_in::{BuiltInAnalyzer, Policy};
pub use quick_fix::{QuickFix, TextEdit};
pub use types::{CriticConfig, Severity, Violation};

#[cfg(not(feature = "lsp-compat"))]
pub use types::ViolationSummary;

pub(crate) use quick_fix::built_in_quick_fix;
#[cfg(feature = "lsp-compat")]
pub(crate) use quick_fix::perlcritic_quick_fix;
pub(crate) use types::insertion_range;

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::{NodeKind, SourceLocation};
    use perl_tdd_support::{must, must_some};
    use std::path::Path;
    use std::sync::Arc;

    #[test]
    fn test_severity_levels() {
        assert_eq!(Severity::from_number(1), Severity::Brutal);
        assert_eq!(Severity::from_number(5), Severity::Gentle);
    }

    #[test]
    fn test_builtin_policies() {
        let analyzer = BuiltInAnalyzer::new();
        let ast = perl_parser_core::Node::new(
            NodeKind::Error {
                message: "test".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            SourceLocation { start: 0, end: 10 },
        );

        let violations = analyzer.analyze(&ast, "print 'hello';\n");
        assert_eq!(violations.len(), 2);

        let violations = analyzer.analyze(&ast, "use strict;\nuse warnings;\nprint 'hello';\n");
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_analyzer_with_mock_runtime() {
        use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        let mock_output =
            b"test.pl:5:1:3:TestingAndDebugging::RequireUseStrict:Code does not use strict\n";
        runtime.add_response(MockResponse::success(mock_output.to_vec()));

        let config = CriticConfig::default();
        let mut analyzer = CriticAnalyzer::new(config, runtime.clone());

        let result = analyzer.analyze_file(Path::new("test.pl"));
        let violations = must(result);
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].policy,
            "TestingAndDebugging::RequireUseStrict"
        );
        assert_eq!(violations[0].range.start.line, 4);

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].program, "perlcritic");
        assert!(invocations[0].args.contains(&"--severity=3".to_string()));
        assert!(invocations[0].args.contains(&"--".to_string()));
        let sep_pos = must_some(invocations[0].args.iter().position(|a| a == "--"));
        let file_pos = must_some(invocations[0].args.iter().position(|a| a == "test.pl"));
        assert!(
            sep_pos < file_pos,
            "-- separator must come before file path"
        );
    }

    #[test]
    fn test_analyzer_caching() {
        use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"".to_vec()));

        let config = CriticConfig::default();
        let mut analyzer = CriticAnalyzer::new(config, runtime.clone());

        let result1 = analyzer.analyze_file(Path::new("test.pl"));
        assert!(result1.is_ok());

        let result2 = analyzer.analyze_file(Path::new("test.pl"));
        assert!(result2.is_ok());

        assert_eq!(runtime.invocations().len(), 1);
    }

    #[test]
    fn test_analyzer_config_args() {
        use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"".to_vec()));

        let config = CriticConfig {
            severity: 1,
            profile: Some("/path/to/.perlcriticrc".to_string()),
            theme: Some("pbp".to_string()),
            include: vec!["RequireUseStrict".to_string()],
            exclude: vec!["ProhibitMagicNumbers".to_string()],
            ..Default::default()
        };
        let mut analyzer = CriticAnalyzer::new(config, runtime.clone());

        let _ = analyzer.analyze_file(Path::new("test.pl"));

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert!(invocations[0].args.contains(&"--severity=1".to_string()));
        assert!(
            invocations[0]
                .args
                .contains(&"--profile=/path/to/.perlcriticrc".to_string())
        );
        assert!(invocations[0].args.contains(&"--theme=pbp".to_string()));
        assert!(
            invocations[0]
                .args
                .contains(&"--include=RequireUseStrict".to_string())
        );
        assert!(
            invocations[0]
                .args
                .contains(&"--exclude=ProhibitMagicNumbers".to_string())
        );
    }
}
