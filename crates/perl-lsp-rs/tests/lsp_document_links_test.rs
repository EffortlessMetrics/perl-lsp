//! UX-focused behavioral coverage for document links.
//!
//! Exercises the real JSON-RPC workflow used by editors:
//! 1) open document
//! 2) request deferred links
//! 3) resolve a chosen link

mod support;

use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Scenario {
    name: &'static str,
}

impl Scenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, step: &str) {
        eprintln!("[{}] Given {step}", self.name);
    }

    fn when(&self, step: &str) {
        eprintln!("[{}] When {step}", self.name);
    }

    fn then(&self, step: &str) {
        eprintln!("[{}] Then {step}", self.name);
    }
}

fn first_link_of_type<'a>(links: &'a [Value], kind: &str) -> Option<&'a Value> {
    links.iter().find(|link| link.pointer("/data/type").and_then(Value::as_str) == Some(kind))
}

#[test]
fn textDocument_documentLink_responds_with_deferred_links() -> TestResult {
    let mut harness = LspHarness::new();
    let scenario = Scenario::new("use qw + use Data::Dumper resolve workflow");

    scenario.given("a document with use qw imports and other use statements");
    let doc = r#"#!/usr/bin/perl
use strict;
use warnings;
use Data::Dumper;
use Getopt::Long;
my @modules = qw(Data::Dumper Getopt::Long);
"#;

    harness.open_untitled(doc)?;
    scenario.when("requesting documentLink at position of first use statement");
    let result = harness.request(
        "textDocument/documentLink",
        json!({
            "textDocument": {"uri": harness.doc_uri()},
        }),
    )?;

    scenario.then("receive array of deferred DocumentLink objects");
    let links = result.as_array().ok_or("expected DocumentLink[]")?;
    assert!(!links.is_empty(), "expected at least one link in response");

    // Verify each link has the deferred pattern
    for link in links {
        assert!(link.get("target").is_none(), "resolved links should omit target (deferred)");
        assert!(link.pointer("/data/type").is_some(), "expected data.type field for deferred link");
    }

    Ok(())
}

#[test]
fn textDocument_documentLink_resolvesTo_module_paths() -> TestResult {
    let mut harness = LspHarness::new();
    let scenario = Scenario::new("documentLink/resolve workflow");

    scenario.given("deferred DocumentLink from documentLink response");
    let doc = r#"#!/usr/bin/perl
use Data::Dumper;
use JSON::PP;
"#;

    harness.open_untitled(doc)?;
    let links_response = harness.request(
        "textDocument/documentLink",
        json!({
            "textDocument": {"uri": harness.doc_uri()},
        }),
    )?;

    let links = links_response.as_array().ok_or("expected DocumentLink[]")?;
    assert!(!links.is_empty(), "expected deferred links");

    scenario.when("resolving the first link");
    let first_link = &links[0];
    let resolved = harness.request("documentLink/resolve", first_link)?;

    scenario.then("received resolved link contains target module path");
    let target = resolved
        .get("target")
        .and_then(Value::as_str)
        .ok_or("expected resolved link to have target")?;
    assert!(!target.is_empty(), "resolved link target must not be empty");

    Ok(())
}

#[test]
fn textDocument_documentLink_handles_missing_modules() -> TestResult {
    let mut harness = LspHarness::new();
    let scenario = Scenario::new("missing module graceful handling");

    scenario.given("document with use statement for non-existent module");
    let doc = r#"#!/usr/bin/perl
use NonExistentModule::Does::Not::Exist;
"#;

    harness.open_untitled(doc)?;
    scenario.when("requesting documentLink");
    let result = harness.request(
        "textDocument/documentLink",
        json!({
            "textDocument": {"uri": harness.doc_uri()},
        }),
    )?;

    scenario.then("respond with empty array or links without targets");
    let links = result.as_array().ok_or("expected DocumentLink[]")?;
    for link in links {
        if let Some(link_obj) = link.as_object() {
            // Deferred links should have data.type set, but not resolved targets
            assert!(
                link_obj.contains_key("data") && link_obj.get("target").is_none(),
                "non-resolved links must not have target field"
            );
        }
    }

    Ok(())
}
