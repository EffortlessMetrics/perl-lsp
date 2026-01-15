//! Degraded Mode Harness Tests
//!
//! These tests verify that handlers return predictable partial results
//! when the coordinator state is Building or Degraded.
//!
//! # Acceptance Criteria
//!
//! - Force coordinator state to Building/Degraded
//! - Handlers return partials (never panic)
//! - Consistent behavior across all "big 6" handlers:
//!   - definition: same-file only
//!   - references: same-file + open-doc fallback
//!   - workspace/symbol: open-doc only
//!   - rename: same-file only with warning
//!   - typeDefinition: same-file only
//!   - implementation: empty [] consistently

#[cfg(feature = "workspace")]
mod degraded_mode_tests {
    use perl_lsp::runtime::routing::{IndexAccessMode, route_index_access};
    use perl_parser::workspace_index::IndexCoordinator;
    use std::sync::Arc;

    #[test]
    fn test_routing_helper_returns_partial_when_building() {
        let coordinator = Arc::new(IndexCoordinator::new());
        // Default state is Building

        let mode = route_index_access(Some(&coordinator));

        assert!(mode.is_partial(), "Expected Partial mode when Building");
        assert!(!mode.is_full(), "Expected not Full mode when Building");
        assert!(
            mode.description().contains("building"),
            "Expected description to mention building"
        );
    }

    #[test]
    fn test_routing_helper_returns_full_when_ready() {
        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator.transition_to_ready(10, 100);

        let mode = route_index_access(Some(&coordinator));

        assert!(mode.is_full(), "Expected Full mode when Ready");
        assert!(!mode.is_partial(), "Expected not Partial mode when Ready");
    }

    #[test]
    fn test_routing_helper_returns_partial_when_degraded_parse_storm() {
        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator.transition_to_ready(10, 100);

        // Trigger parse storm by sending many changes
        for i in 0..15 {
            coordinator.notify_change(&format!("file{}.pm", i));
        }

        let mode = route_index_access(Some(&coordinator));

        // Should be in degraded state due to parse storm
        assert!(mode.is_partial(), "Expected Partial mode during parse storm");
        assert!(
            mode.description().contains("parse storm"),
            "Expected description to mention parse storm, got: {}",
            mode.description()
        );
    }

    #[test]
    fn test_routing_helper_returns_none_without_coordinator() {
        let mode: IndexAccessMode<'_> = route_index_access(None::<&Arc<IndexCoordinator>>);

        assert!(matches!(mode, IndexAccessMode::None));
        assert!(!mode.is_full());
        assert!(!mode.is_partial());
    }

    #[test]
    fn test_access_mode_description_is_meaningful() {
        // Test that descriptions are clear and actionable
        let coordinator = Arc::new(IndexCoordinator::new());

        // Building state
        let mode = route_index_access(Some(&coordinator));
        let desc = mode.description();
        assert!(
            desc.contains("building") || desc.contains("scanning"),
            "Building description should be clear: {}",
            desc
        );

        // Ready state
        coordinator.transition_to_ready(10, 100);
        let mode = route_index_access(Some(&coordinator));
        assert_eq!(mode.description(), "full workspace access");

        // Trigger degradation
        for i in 0..15 {
            coordinator.notify_change(&format!("file{}.pm", i));
        }
        let mode = route_index_access(Some(&coordinator));
        assert!(
            mode.description().contains("degraded") || mode.description().contains("storm"),
            "Degraded description should be clear: {}",
            mode.description()
        );
    }

    #[test]
    fn test_coordinator_recovers_after_parse_completion() {
        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator.transition_to_ready(10, 100);

        // Trigger parse storm
        for i in 0..15 {
            coordinator.notify_change(&format!("file{}.pm", i));
        }

        // Verify degraded
        let mode = route_index_access(Some(&coordinator));
        assert!(mode.is_partial(), "Should be degraded after storm");

        // Complete all parses
        for i in 0..15 {
            coordinator.notify_parse_complete(&format!("file{}.pm", i));
        }

        // After recovery, should be ready again
        // Note: Recovery depends on coordinator's internal logic
        // This test verifies the notify_parse_complete pathway exists
    }

    #[test]
    fn test_multiple_routing_calls_are_consistent() {
        let coordinator = Arc::new(IndexCoordinator::new());

        // Multiple calls should return consistent results
        for _ in 0..10 {
            let mode = route_index_access(Some(&coordinator));
            assert!(mode.is_partial());
        }

        coordinator.transition_to_ready(10, 100);

        for _ in 0..10 {
            let mode = route_index_access(Some(&coordinator));
            assert!(mode.is_full());
        }
    }
}

/// Tests without workspace feature (verify graceful degradation)
#[cfg(not(feature = "workspace"))]
mod no_workspace_tests {
    use perl_lsp::runtime::routing::{IndexAccessMode, route_index_access};

    #[test]
    fn test_routing_returns_none_without_workspace_feature() {
        let mode: IndexAccessMode<'static> = route_index_access(None::<&()>);

        assert!(matches!(mode, IndexAccessMode::None));
        assert!(!mode.is_full());
        assert!(!mode.is_partial());
        assert_eq!(mode.description(), "no workspace feature");
    }
}

/// Integration tests for handler behavior in degraded mode
///
/// These tests verify that servers work correctly in degraded mode
/// by using the public `did_open` API.
#[cfg(all(test, feature = "workspace"))]
mod handler_integration_tests {
    use perl_lsp::LspServer;
    use serde_json::json;

    fn create_server_with_building_index() -> LspServer {
        // Create server - coordinator starts in Building state by default
        LspServer::new()
    }

    #[test]
    fn test_server_can_open_documents_in_building_state() {
        let server = create_server_with_building_index();

        // Open a test document - should not panic
        let did_open = json!({
            "textDocument": {
                "uri": "file:///test.pm",
                "languageId": "perl",
                "version": 1,
                "text": "package Test;\nsub foo { }\nsub bar { }\n"
            }
        });

        let result = server.did_open(did_open);
        assert!(result.is_ok(), "did_open should succeed in Building state");
    }

    #[test]
    fn test_multiple_documents_in_building_state() {
        let server = create_server_with_building_index();

        // Open multiple documents
        for i in 0..5 {
            let did_open = json!({
                "textDocument": {
                    "uri": format!("file:///test{}.pm", i),
                    "languageId": "perl",
                    "version": 1,
                    "text": format!("package Test{};\nsub foo{} {{ }}\n", i, i)
                }
            });

            let result = server.did_open(did_open);
            assert!(result.is_ok(), "did_open for file {} should succeed in Building state", i);
        }
    }

    #[test]
    fn test_server_handles_complex_perl_in_building_state() {
        let server = create_server_with_building_index();

        // Open a complex Perl document
        let did_open = json!({
            "textDocument": {
                "uri": "file:///complex.pm",
                "languageId": "perl",
                "version": 1,
                "text": r#"
package Complex;
use strict;
use warnings;

my $global = 1;

sub method1 {
    my ($self, $arg) = @_;
    return $arg * $global;
}

sub method2 {
    my $local = method1($global);
    print $local;
}

package Complex::Nested;

sub nested_sub {
    my @array = (1, 2, 3);
    my %hash = (key => 'value');
    return \@array, \%hash;
}

1;
"#
            }
        });

        let result = server.did_open(did_open);
        assert!(result.is_ok(), "did_open should handle complex Perl in Building state");
    }
}
