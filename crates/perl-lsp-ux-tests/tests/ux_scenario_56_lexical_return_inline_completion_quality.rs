//! Scenario 56 - lexical return inline-completion quality proof.
//!
//! This test exercises deterministic visible-lexical inline completion through
//! a real stdio LSP process. It verifies that blank-line ghost text inside a sub
//! uses the nearby lexical the user already wrote.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness, binary_available};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const LEXICAL_RETURN_PATH: &str = "lib/Inline/LexicalReturn.pm";
const AFTER_COMMENT_PATH: &str = "lib/Inline/LexicalReturnAfterComment.pm";
const BLANK_LINE_MARKER: &str = "    \n}";
const EXPECTED_INSERT: &str = "return $result;";

const LEXICAL_RETURN_SOURCE: &str = r#"use strict;

sub helper {
    my $result = compute();
    
}
"#;

const AFTER_COMMENT_SOURCE: &str = r#"use strict;

sub helper {
    my $result = compute();
    # explain next step
    
}
"#;

const FORBIDDEN_INSERTS: &[&str] =
    &["use strict;\nuse warnings;\n", "done_testing();", "new()", "return $value;"];

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(LEXICAL_RETURN_PATH, LEXICAL_RETURN_SOURCE)
        .with_file(AFTER_COMMENT_PATH, AFTER_COMMENT_SOURCE);
    config.client_capability_overrides = json!({
        "textDocument": {
            "inlineCompletion": {
                "dynamicRegistration": true
            }
        }
    });

    UxHarness::new(config)
}

fn cursor_on_blank_line(source: &str) -> Result<(u32, u32)> {
    let byte_offset = source
        .find(BLANK_LINE_MARKER)
        .with_context(|| format!("missing blank-line marker `{BLANK_LINE_MARKER}`"))?
        + "    ".len();
    position_from_byte_offset(source, byte_offset)
}

fn position_from_byte_offset(source: &str, byte_offset: usize) -> Result<(u32, u32)> {
    let prefix = source
        .get(..byte_offset)
        .with_context(|| format!("byte offset {byte_offset} is not a UTF-8 boundary"))?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().map(str::chars).map(Iterator::count).unwrap_or(0);
    Ok((u32::try_from(line)?, u32::try_from(character)?))
}

fn item_has_inline_shape(item: &Value) -> bool {
    item.get("insertText").and_then(Value::as_str).is_some()
}

fn wait_for_lexical_return_inline_completion(
    harness: &UxHarness,
    file: &'static str,
    source: &str,
) -> Result<Vec<String>> {
    let (line, character) = cursor_on_blank_line(source)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let items = harness.inline_completion_with_trigger_kind(file, line, character, 1)?;
        for item in &items {
            anyhow::ensure!(
                item_has_inline_shape(item),
                "inline item must include insertText: {item:?}"
            );
        }
        let insert_texts = insert_texts_for(&items);
        if insert_texts.iter().any(|insert_text| insert_text == EXPECTED_INSERT)
            || Instant::now() >= deadline
        {
            return Ok(insert_texts);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.get("insertText").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn present_forbidden<'a>(insert_texts: &[String], forbidden: &'a [&str]) -> Vec<&'a str> {
    forbidden
        .iter()
        .copied()
        .filter(|forbidden| insert_texts.iter().any(|actual| actual == forbidden))
        .collect()
}

fn expected_present(insert_texts: &[String]) -> bool {
    insert_texts.iter().any(|actual| actual == EXPECTED_INSERT)
}

#[test]
fn scenario_56_lexical_return_inline_completion_quality_stdio() -> Result<()> {
    if !binary_available() {
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file(LEXICAL_RETURN_PATH, LEXICAL_RETURN_SOURCE)?;
    harness.open_file(AFTER_COMMENT_PATH, AFTER_COMMENT_SOURCE)?;
    std::thread::sleep(Duration::from_millis(250));

    let blank_line_insert_texts = wait_for_lexical_return_inline_completion(
        &harness,
        LEXICAL_RETURN_PATH,
        LEXICAL_RETURN_SOURCE,
    )?;
    let blank_line_forbidden = present_forbidden(&blank_line_insert_texts, FORBIDDEN_INSERTS);

    let after_comment_insert_texts = wait_for_lexical_return_inline_completion(
        &harness,
        AFTER_COMMENT_PATH,
        AFTER_COMMENT_SOURCE,
    )?;
    let after_comment_forbidden = present_forbidden(&after_comment_insert_texts, FORBIDDEN_INSERTS);

    assert!(
        !blank_line_insert_texts.is_empty(),
        "blank-line lexical return inline completion returned no candidates"
    );
    assert!(
        expected_present(&blank_line_insert_texts),
        "blank-line lexical return inline completion did not use nearby $result; actual: {blank_line_insert_texts:?}"
    );
    assert!(
        blank_line_forbidden.is_empty(),
        "blank-line lexical return inline completion returned forbidden snippets: {blank_line_forbidden:?}"
    );
    assert!(
        expected_present(&after_comment_insert_texts),
        "after-comment lexical return inline completion did not use nearby $result; actual: {after_comment_insert_texts:?}"
    );
    assert!(
        after_comment_forbidden.is_empty(),
        "after-comment lexical return inline completion returned forbidden snippets: {after_comment_forbidden:?}"
    );

    harness.assert_no_crash();
    Ok(())
}
