//! BDD-style UX coverage for cross-file navigation workflows.

mod support;

use support::lsp_ux_harness::{
    LspUxHarness, assert_has_location_in_uri, assert_symbol_results_include_uri, find_position,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const APP_MAIN: &str = r#"#!/usr/bin/perl
use strict;
use warnings;
use lib './lib';
use MyApp::Utils qw(format_date);

my $created_at = time();
my $display = format_date($created_at);
print "$display\n";
"#;

const UTILS_MODULE: &str = r#"package MyApp::Utils;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(format_date);

sub format_date {
    my ($epoch) = @_;
    return scalar localtime($epoch);
}

1;
"#;

#[test]
fn given_exported_helper_when_workspace_symbol_search_then_result_points_to_module_file()
-> TestResult {
    let mut ux = LspUxHarness::given_workspace(&[
        ("app/main.pl", APP_MAIN),
        ("lib/MyApp/Utils.pm", UTILS_MODULE),
    ])?;

    let response = ux.when_workspace_symbol("format_date")?;

    assert_symbol_results_include_uri(&response, &ux.uri_for("lib/MyApp/Utils.pm"))?;
    Ok(())
}

#[test]
fn given_imported_helper_call_when_go_to_definition_then_location_resolves_to_exporter()
-> TestResult {
    let mut ux = LspUxHarness::given_workspace(&[
        ("app/main.pl", APP_MAIN),
        ("lib/MyApp/Utils.pm", UTILS_MODULE),
    ])?;

    let (line, character) = find_position(APP_MAIN, "format_date($created_at)")?;
    let response = ux.when_go_to_definition("app/main.pl", line, character)?;

    assert_has_location_in_uri(&response, &ux.uri_for("lib/MyApp/Utils.pm"))?;
    Ok(())
}
