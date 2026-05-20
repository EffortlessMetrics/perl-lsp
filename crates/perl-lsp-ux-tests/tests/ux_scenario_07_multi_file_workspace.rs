// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 07 — Multi-file workspace / cross-file navigation.
//!
//! Sets up a small Perl project (cpanfile, library modules, script).
//! Verifies multi-file open and go-to-definition work or degrade gracefully.
//!
//! Acceptance criteria:
//! - All files open without crashing.
//! - `textDocument/definition` MUST NOT crash (empty result is acceptable).
//! - Server remains responsive after workspace indexing.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

#[test]
fn scenario_07_multi_file_workspace_opens_without_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_07: perl-lsp binary not found");
        return;
    }

    let module_a = "package MyProject::Utils;\nuse strict;\nuse warnings;\n\n\
                    sub greet { my ($self, $name) = @_; return \"Hello, $name!\"; }\n1;\n";
    let module_b = "package MyProject::Config;\nuse strict;\nuse warnings;\n\n\
                    our $VERSION = '1.0';\nsub get_setting { return 'default'; }\n1;\n";
    let script = "#!/usr/bin/env perl\nuse strict;\nuse warnings;\n\n\
                  use MyProject::Utils;\nuse MyProject::Config;\n\n\
                  my $utils = MyProject::Utils->new();\nprint $utils->greet('World');\n";

    let harness = UxHarness::new(
        ScenarioConfig::default()
            .with_file("lib/MyProject/Utils.pm", module_a)
            .with_file("lib/MyProject/Config.pm", module_b)
            .with_file("script.pl", script)
            .with_file("cpanfile", "requires 'Moo', '2.0';\n"),
    )
    .expect("Failed to create multi-file harness");

    harness.open_file("lib/MyProject/Utils.pm", module_a).expect("Utils.pm should open");
    harness.open_file("lib/MyProject/Config.pm", module_b).expect("Config.pm should open");
    harness.open_file("script.pl", script).expect("script.pl should open");

    harness.assert_no_crash();
}

#[test]
fn scenario_07_definition_request_does_not_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_07: perl-lsp binary not found");
        return;
    }

    let module = "package Counter;\nuse strict;\nuse warnings;\n\n\
                  sub new { bless {count => 0}, shift }\n\
                  sub increment { $_[0]->{count}++ }\n\
                  sub value { $_[0]->{count} }\n1;\n";
    let script = "use strict;\nuse warnings;\n\nuse Counter;\n\n\
                  my $c = Counter->new();\n$c->increment();\nprint $c->value();\n";

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("lib/Counter.pm", module)
            .with_file("main.pl", script),
    )
    .expect("Failed to create harness");

    harness.open_file("lib/Counter.pm", module).expect("Counter.pm should open");
    harness.open_file("main.pl", script).expect("main.pl should open");

    // Allow workspace index to build.
    std::thread::sleep(Duration::from_secs(2));

    let defs = harness.definition("main.pl", 3, 4);
    assert!(defs.is_ok(), "definition request crashed server — UX regression: {:?}", defs);

    harness.assert_no_crash();
}
