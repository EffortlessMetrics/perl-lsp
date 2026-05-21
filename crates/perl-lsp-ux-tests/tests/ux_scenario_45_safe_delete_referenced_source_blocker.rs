//! Scenario 45 - safe-delete referenced-source blocker receipt.
//!
//! This receipt exercises the live `perl.safeDeleteSymbol` command over a
//! CPAN-style workspace where the requested source-backed subroutine is still
//! referenced. The expected product behavior is no edit, a blocker reason, and
//! a copyable provider explanation.

use anyhow::{Context, Result, anyhow};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_45_safe_delete_referenced_source_blocker.rs";
const APP_PATH: &str = "lib/RealBaseline/App.pm";
const BASE_PATH: &str = "lib/RealBaseline/Base.pm";
const UTIL_PATH: &str = "lib/RealBaseline/Util.pm";
const SCRIPT_PATH: &str = "script/real-baseline.pl";

const APP_PM: &str = r#"package RealBaseline::App;
use strict;
use warnings;
use parent 'RealBaseline::Base';
use RealBaseline::Util qw(helper alias);

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub run {
    my ($self) = @_;
    helper($self->name);
    alias($self->shared);
    return $self->shared;
}

sub name {
    return $_[0]->{name};
}

1;
"#;

const BASE_PM: &str = r#"package RealBaseline::Base;
use strict;
use warnings;

sub shared {
    return 'shared';
}

sub reset {
    return 1;
}

1;
"#;

const UTIL_PM: &str = r#"package RealBaseline::Util;
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(helper alias);

sub helper {
    return shift;
}

*alias = \&helper;

sub bounce {
    goto &helper;
}

1;
"#;

const SCRIPT_PL: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealBaseline::App;

my $app = RealBaseline::App->new(name => 'demo');
$app->run;
"#;

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_file(APP_PATH, APP_PM)
            .with_file(BASE_PATH, BASE_PM)
            .with_file(UTIL_PATH, UTIL_PM)
            .with_file(SCRIPT_PATH, SCRIPT_PL),
    )
}

fn position_at(source: &str, needle: &str) -> Result<(u32, u32)> {
    let byte_offset = source.find(needle).with_context(|| format!("missing `{needle}`"))?;
    let prefix = source
        .get(..byte_offset)
        .with_context(|| format!("byte offset {byte_offset} is not a UTF-8 boundary"))?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().map(str::chars).map(Iterator::count).unwrap_or(0);
    Ok((u32::try_from(line)?, u32::try_from(character)?))
}

fn execute_command(harness: &UxHarness, command: &str, arguments: Value) -> Result<Value> {
    let response = harness.client.request(
        "workspace/executeCommand",
        json!({
            "command": command,
            "arguments": arguments,
        }),
        Duration::from_secs(20),
    )?;
    if response.get("error").is_some() {
        return Err(anyhow!("{command} returned error: {}", response["error"]));
    }
    Ok(response["result"].clone())
}

fn safe_delete_helper(harness: &UxHarness) -> Result<Value> {
    let (line, character) = position_at(UTIL_PM, "helper {")?;
    execute_command(
        harness,
        "perl.safeDeleteSymbol",
        json!([{
            "textDocument": {"uri": harness.workspace.uri(UTIL_PATH)},
            "position": {"line": line, "character": character}
        }]),
    )
}

fn explain_safe_delete(harness: &UxHarness) -> Result<Value> {
    execute_command(
        harness,
        "perl.explainProviderDecision",
        json!([{
            "provider": "safe_delete"
        }]),
    )
}

fn workspace_edit_change_count(result: &Value) -> Option<usize> {
    result.pointer("/workspace_edit/changes").and_then(Value::as_object).map(serde_json::Map::len)
}

#[test]
fn scenario_45_safe_delete_referenced_source_blocker_receipt() {
    run_ux_scenario(
        "safe_delete_referenced_source_blocker",
        SCENARIO_FILE,
        "scenario_45_safe_delete_referenced_source_blocker_receipt",
        UxCiTier::Pr,
        Some(UxComponent::SafeDelete),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(APP_PATH, APP_PM)?;
            harness.open_file(BASE_PATH, BASE_PM)?;
            harness.open_file(UTIL_PATH, UTIL_PM)?;
            harness.open_file(SCRIPT_PATH, SCRIPT_PL)?;
            std::thread::sleep(Duration::from_millis(500));

            recorder.mark_request_start("safe_delete_referenced_helper");
            let result = safe_delete_helper(&harness)?;
            recorder.mark_first_useful_result("safe_delete_referenced_helper");

            let decision = result.get("decision").and_then(Value::as_str);
            let reason = result.get("reason").and_then(Value::as_str);
            let provider_action = result.get("provider_action").and_then(Value::as_str);
            let live_enabled = result.get("live_symbol_delete_enabled").and_then(Value::as_bool);
            let edit_count = result.get("returned_workspace_edit_count").and_then(Value::as_u64);
            let edits_applied = result.get("edits_applied").and_then(Value::as_bool);
            let change_count = workspace_edit_change_count(&result);

            recorder.mark_request_start("explain_safe_delete_referenced_helper");
            let explanation = explain_safe_delete(&harness)?;
            recorder.mark_first_useful_result("explain_safe_delete_referenced_helper");

            let explanation_decision = explanation.get("decision").and_then(Value::as_str);
            let copyable_payload_present = explanation.get("copyable_payload").is_some();
            let request_receipt =
                explanation.get("request_receipt").cloned().unwrap_or(Value::Null);
            let request_receipt_reason = request_receipt.get("reason").and_then(Value::as_str);
            let request_receipt_edit_count =
                request_receipt.get("returned_workspace_edit_count").and_then(Value::as_u64);

            let receipt = json!({
                "schema_version": 1,
                "receipt": "safe_delete_referenced_source_blocker",
                "workspace_fixture": "RealBaseline four-file CPAN-style workspace",
                "claim_boundary": "safe-delete referenced-source blocker receipt only; no provider behavior broadening, support-tier promotion, or server-applied edits",
                "provider_action": provider_action,
                "decision": decision,
                "reason": reason,
                "live_symbol_delete_enabled": live_enabled,
                "returned_workspace_edit_count": edit_count,
                "workspace_edit_change_count": change_count,
                "edits_applied": edits_applied,
                "explanation_decision": explanation_decision,
                "explanation_copyable_payload": copyable_payload_present,
                "request_receipt_reason": request_receipt_reason,
                "request_receipt_returned_workspace_edit_count": request_receipt_edit_count,
            });
            eprintln!(
                "safe_delete_referenced_source_blocker_receipt={}",
                serde_json::to_string_pretty(&receipt)?,
            );

            recorder.check(
                "safe-delete live command used the symbol-delete provider action",
                provider_action == Some("perl.safeDeleteSymbol"),
            )?;
            recorder.check("referenced helper was blocked", decision == Some("blocked"))?;
            recorder.check(
                "referenced helper recorded references_exist",
                reason == Some("references_exist"),
            )?;
            recorder.check(
                "referenced helper did not enable live symbol delete",
                live_enabled == Some(false),
            )?;
            recorder.check("referenced helper returned zero edits", edit_count == Some(0))?;
            recorder
                .check("referenced helper did not apply edits", edits_applied == Some(false))?;
            recorder.check(
                "referenced helper workspace edit changes are empty",
                change_count == Some(0),
            )?;
            recorder.check(
                "explain-provider-decision replayed the safe-delete block",
                explanation_decision == Some("blocked")
                    && request_receipt_reason == Some("references_exist"),
            )?;
            recorder.check(
                "safe-delete explanation included a copyable payload",
                copyable_payload_present,
            )?;
            recorder.check(
                "safe-delete explanation preserved zero-edit receipt",
                request_receipt_edit_count == Some(0),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
