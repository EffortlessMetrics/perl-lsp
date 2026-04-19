// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 08 — Shebang detection / non-standard extensions.
//!
//! Files with `#!/usr/bin/env perl` shebang but no `.pl`/`.pm` extension.
//!
//! Acceptance criteria:
//! - Server MUST accept `didOpen` with any URI when languageId is "perl".
//! - Hover, completion MUST NOT crash.
//! - Null results are acceptable.

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

#[test]
fn scenario_08_shebang_file_without_pl_extension() {
    if !binary_available() {
        eprintln!("SKIP scenario_08: perl-lsp binary not found");
        return;
    }

    let source = "#!/usr/bin/env perl\nuse strict;\nuse warnings;\n\n\
                  my $answer = 42;\nprint \"Answer: $answer\\n\";\n";
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness
        .open_file("deploy_script", source)
        .expect("didOpen should succeed for shebang file");

    let hover = harness.hover("deploy_script", 4, 3);
    assert!(
        hover.is_ok(),
        "hover crashed on non-.pl file — UX regression: {:?}",
        hover
    );

    harness.assert_no_crash();
}

#[test]
fn scenario_08_no_extension_file_completion_does_not_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_08: perl-lsp binary not found");
        return;
    }

    let source = "#!/usr/bin/perl\nmy $va\n";
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness
        .open_file("run_tests", source)
        .expect("didOpen should succeed");

    let result = harness.completion("run_tests", 1, 7);
    assert!(
        result.is_ok(),
        "completion crashed on non-.pl file — UX regression: {:?}",
        result
    );
}

#[test]
fn scenario_08_test_file_t_extension() {
    if !binary_available() {
        eprintln!("SKIP scenario_08: perl-lsp binary not found");
        return;
    }

    let source = "use Test::More;\nuse strict;\n\nok(1, 'basic');\ndone_testing();\n";
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness
        .open_file("basic.t", source)
        .expect("didOpen should succeed for .t extension");

    let hover = harness.hover("basic.t", 3, 1);
    assert!(
        hover.is_ok(),
        "hover crashed on .t test file — UX regression: {:?}",
        hover
    );
}
