//! Mason navigation MVP tests.
//!
//! Covers the intentionally narrow Mason surface:
//! - `.mason` / `.mas` file recognition at the editor layer
//! - same-file `<%method>` / `<%sub>` goto-definition
//! - `<& component &>` component-file goto-definition
//!
//! This suite intentionally does not cover `.m`, `<%args>`, syntax highlighting,
//! or embedded Perl diagnostics.

mod support;

use serde_json::{Value, json};
use support::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn find_pos(code: &str, needle: &str, target_line: usize) -> Result<(u32, u32), String> {
    let line =
        code.lines().nth(target_line).ok_or_else(|| format!("missing line {target_line}"))?;
    let character =
        line.find(needle).ok_or_else(|| format!("missing `{needle}` on line {target_line}"))?;
    Ok((target_line as u32, character as u32))
}

fn first_location(resp: &Value) -> Option<(String, u32, u32)> {
    let result = resp.get("result").unwrap_or(resp);

    let location = if let Some(arr) = result.as_array() {
        arr.first()?
    } else if result.is_object() {
        result
    } else {
        return None;
    };

    let uri = location.get("uri").or_else(|| location.get("targetUri"))?.as_str()?.to_string();
    let range = location.get("range").or_else(|| location.get("targetRange"))?;
    let start = range.get("start")?;
    let line = start.get("line")?.as_u64()? as u32;
    let character = start.get("character")?.as_u64()? as u32;
    Some((uri, line, character))
}

#[test]
fn mason_language_extensions_and_navigation_smoke() -> TestResult {
    const MAIN: &str = r#"<%method greet>
  Hello from greet
</%method>

<%sub helper>
  Hello from helper
</%sub>

<& greet &>
<& helper &>

<& components/menu &>
<& components/header &>
"#;

    const MENU_COMPONENT: &str = r#"<p>Menu component</p>
"#;

    const HEADER_COMPONENT: &str = r#"<p>Header component</p>
"#;

    let (mut harness, workspace) = LspHarness::with_workspace(&[
        ("main.mason", MAIN),
        ("components/menu.mason", MENU_COMPONENT),
        ("components/header.mas", HEADER_COMPONENT),
    ])?;

    let main_uri = workspace.uri("main.mason");
    let menu_uri = workspace.uri("components/menu.mason");
    let header_uri = workspace.uri("components/header.mas");

    harness.open_document(&main_uri, MAIN)?;

    let (greet_line, greet_character) = find_pos(MAIN, "greet", 8)?;
    let greet = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": &main_uri },
            "position": { "line": greet_line, "character": greet_character }
        }),
    )?;
    let (greet_uri, greet_line, _) =
        first_location(&greet).ok_or("expected Mason greet definition")?;
    assert_eq!(greet_uri, main_uri);
    assert_eq!(greet_line, 0);

    let (helper_line, helper_character) = find_pos(MAIN, "helper", 9)?;
    let helper = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": &main_uri },
            "position": { "line": helper_line, "character": helper_character }
        }),
    )?;
    let (helper_uri, helper_line, _) =
        first_location(&helper).ok_or("expected Mason helper definition")?;
    assert_eq!(helper_uri, main_uri);
    assert_eq!(helper_line, 4);

    let (menu_line, menu_character) = find_pos(MAIN, "components/menu", 11)?;
    let menu = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": &main_uri },
            "position": { "line": menu_line, "character": menu_character }
        }),
    )?;
    let (menu_def_uri, menu_def_line, _) =
        first_location(&menu).ok_or("expected Mason menu component definition")?;
    assert_eq!(menu_def_uri, menu_uri);
    assert_eq!(menu_def_line, 0);

    let (header_line, header_character) = find_pos(MAIN, "components/header", 12)?;
    let header = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": &main_uri },
            "position": { "line": header_line, "character": header_character }
        }),
    )?;
    let (header_def_uri, header_def_line, _) =
        first_location(&header).ok_or("expected Mason header component definition")?;
    assert_eq!(header_def_uri, header_uri);
    assert_eq!(header_def_line, 0);

    Ok(())
}
