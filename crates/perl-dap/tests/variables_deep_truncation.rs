use perl_dap::variables::{PerlValue, PerlVariableRenderer, VariableRenderer};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn load_fixture() -> Result<Value, Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/mocks/dap_deferred_gap_sessions.json");
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn fixture_variables() -> Result<Value, Box<dyn std::error::Error>> {
    let fixture = load_fixture()?;
    fixture
        .get("variables")
        .cloned()
        .ok_or_else(|| "missing variables section in deferred gap fixture".into())
}

fn build_nested_hash(depth: usize, leaf_key: &str, leaf_value: i64) -> PerlValue {
    let mut value = PerlValue::Hash(vec![(leaf_key.to_string(), PerlValue::Integer(leaf_value))]);
    for idx in (1..depth).rev() {
        value = PerlValue::Hash(vec![(format!("level_{idx}"), value)]);
    }
    value
}

#[test]
fn fixture_large_array_200_preview_and_pagination() -> TestResult {
    let renderer = PerlVariableRenderer::new();
    let vars = fixture_variables()?;
    let cases = vars.get("arrays").and_then(Value::as_array).ok_or("missing arrays in fixture")?;
    let case = cases
        .iter()
        .find(|entry| entry.get("id").and_then(Value::as_str) == Some("array_200"))
        .ok_or("missing array_200 case")?;

    let size_u64 = case.get("size").and_then(Value::as_u64).ok_or("missing array_200 size")?;
    let size = usize::try_from(size_u64)?;
    let values = PerlValue::Array((0..size).map(|idx| PerlValue::Integer(idx as i64)).collect());
    let name = case.get("variable").and_then(Value::as_str).ok_or("missing array_200 variable")?;
    let rendered = renderer.render(name, &values);

    assert!(rendered.value.contains("200 total"));
    assert_eq!(rendered.indexed_variables, Some(200));

    let page = case.get("page").ok_or("missing array_200 page")?;
    let start = usize::try_from(page.get("start").and_then(Value::as_u64).ok_or("missing start")?)?;
    let count = usize::try_from(page.get("count").and_then(Value::as_u64).ok_or("missing count")?)?;
    let children = renderer.render_children(&values, start, count);
    let first_name = page.get("first_name").and_then(Value::as_str).ok_or("missing first_name")?;
    let last_name = page.get("last_name").and_then(Value::as_str).ok_or("missing last_name")?;

    assert_eq!(children.first().ok_or("empty children")?.name, first_name);
    assert_eq!(children.last().ok_or("empty children")?.name, last_name);
    Ok(())
}

#[test]
fn fixture_large_array_500_preview_and_tail_page() -> TestResult {
    let renderer = PerlVariableRenderer::new();
    let vars = fixture_variables()?;
    let cases = vars.get("arrays").and_then(Value::as_array).ok_or("missing arrays in fixture")?;
    let case = cases
        .iter()
        .find(|entry| entry.get("id").and_then(Value::as_str) == Some("array_500"))
        .ok_or("missing array_500 case")?;

    let size_u64 = case.get("size").and_then(Value::as_u64).ok_or("missing array_500 size")?;
    let size = usize::try_from(size_u64)?;
    let values = PerlValue::Array((0..size).map(|idx| PerlValue::Integer(idx as i64)).collect());
    let rendered = renderer.render("@events", &values);
    assert!(rendered.value.contains("500 total"));
    assert_eq!(rendered.indexed_variables, Some(500));

    let page = case.get("page").ok_or("missing array_500 page")?;
    let start = usize::try_from(page.get("start").and_then(Value::as_u64).ok_or("missing start")?)?;
    let count = usize::try_from(page.get("count").and_then(Value::as_u64).ok_or("missing count")?)?;
    let children = renderer.render_children(&values, start, count);

    assert_eq!(children.len(), 50);
    assert_eq!(children.first().ok_or("empty children")?.name, "[450]");
    assert_eq!(children.last().ok_or("empty children")?.name, "[499]");
    Ok(())
}

#[test]
fn fixture_deep_nested_hash_preview_is_bounded() -> TestResult {
    let renderer = PerlVariableRenderer::new();
    let vars = fixture_variables()?;
    let deep_hash = vars.get("deep_hash").ok_or("missing deep_hash fixture")?;
    let depth =
        usize::try_from(deep_hash.get("depth").and_then(Value::as_u64).ok_or("missing depth")?)?;
    let leaf_key = deep_hash.get("leaf_key").and_then(Value::as_str).ok_or("missing leaf_key")?;
    let leaf_value =
        deep_hash.get("leaf_value").and_then(Value::as_i64).ok_or("missing leaf_value")?;
    let value = build_nested_hash(depth, leaf_key, leaf_value);

    let rendered = renderer.render("%config", &value);
    assert_eq!(rendered.type_name.as_deref(), Some("HASH"));
    assert!(rendered.value.len() < 1000);
    assert_eq!(rendered.named_variables, Some(1));
    Ok(())
}

#[test]
fn fixture_scope_visibility_keeps_lexical_package_and_global_distinct() -> TestResult {
    let renderer = PerlVariableRenderer::new();
    let vars = fixture_variables()?;
    let scopes = vars.get("scopes").ok_or("missing scopes fixture")?;

    let locals = scopes.get("locals").and_then(Value::as_array).ok_or("missing locals")?;
    let package = scopes.get("package").and_then(Value::as_array).ok_or("missing package")?;
    let globals = scopes.get("globals").and_then(Value::as_array).ok_or("missing globals")?;

    let local_name = locals.first().and_then(Value::as_str).ok_or("missing lexical variable")?;
    let package_name = package.first().and_then(Value::as_str).ok_or("missing package variable")?;
    let global_name = globals.first().and_then(Value::as_str).ok_or("missing global variable")?;

    let local = renderer.render(local_name, &PerlValue::scalar("lex"));
    let package = renderer.render(package_name, &PerlValue::Integer(10));
    let global = renderer.render(global_name, &PerlValue::scalar("prod"));

    assert!(local.name.starts_with("$lex_"));
    assert!(package.name.contains("::"));
    assert!(global.name.starts_with("$::"));
    Ok(())
}

#[test]
fn fixture_coderef_preview_displays_subroutine_identity() -> TestResult {
    let renderer = PerlVariableRenderer::new();
    let vars = fixture_variables()?;
    let coderef = vars.get("coderef_preview").ok_or("missing coderef fixture")?;
    let variable =
        coderef.get("variable").and_then(Value::as_str).ok_or("missing coderef variable")?;
    let name = coderef.get("name").and_then(Value::as_str).ok_or("missing coderef name")?;

    let rendered = renderer.render(variable, &PerlValue::Code { name: Some(name.to_string()) });
    assert_eq!(rendered.type_name.as_deref(), Some("CODE"));
    assert!(rendered.value.contains(name));
    Ok(())
}

#[test]
fn fixture_blessed_object_preview_includes_class_and_fields() -> TestResult {
    let renderer = PerlVariableRenderer::new();
    let vars = fixture_variables()?;
    let object = vars.get("blessed_object_preview").ok_or("missing blessed object fixture")?;
    let variable =
        object.get("variable").and_then(Value::as_str).ok_or("missing blessed variable")?;
    let class = object.get("class").and_then(Value::as_str).ok_or("missing class")?;

    let value = PerlValue::Object {
        class: class.to_string(),
        value: Box::new(PerlValue::Hash(vec![
            ("id".to_string(), PerlValue::Integer(42)),
            ("name".to_string(), PerlValue::scalar("Mónica")),
        ])),
    };
    let rendered = renderer.render(variable, &value);
    assert_eq!(rendered.type_name.as_deref(), Some(class));
    assert!(rendered.value.contains(class));
    assert_eq!(rendered.named_variables, Some(2));
    Ok(())
}

#[test]
fn fixture_unicode_keys_and_values_round_trip_in_preview() -> TestResult {
    let renderer = PerlVariableRenderer::new();
    let vars = fixture_variables()?;
    let unicode = vars.get("unicode_hash").ok_or("missing unicode_hash fixture")?;
    let variable =
        unicode.get("variable").and_then(Value::as_str).ok_or("missing unicode variable")?;

    let value = PerlValue::Hash(vec![
        ("日本語".to_string(), PerlValue::scalar("こんにちは世界")),
        ("emoji_🧪".to_string(), PerlValue::scalar("テスト✅")),
    ]);
    let rendered = renderer.render(variable, &value);
    assert!(rendered.value.contains("日本語"));
    assert!(rendered.value.contains("テスト✅"));
    assert_eq!(rendered.named_variables, Some(2));
    Ok(())
}

#[test]
fn fixture_truncated_deep_structure_reports_summary_and_count() -> TestResult {
    let renderer = PerlVariableRenderer::new();
    let vars = fixture_variables()?;
    let trunc = vars.get("deep_truncation").ok_or("missing deep_truncation fixture")?;
    let variable = trunc.get("variable").and_then(Value::as_str).ok_or("missing trunc variable")?;
    let summary = trunc.get("summary").and_then(Value::as_str).ok_or("missing summary")?;
    let total = usize::try_from(
        trunc.get("total_count").and_then(Value::as_u64).ok_or("missing total_count")?,
    )?;
    let value = PerlValue::Truncated { summary: summary.to_string(), total_count: Some(total) };

    let rendered = renderer.render(variable, &value);
    assert!(rendered.value.contains(summary));
    assert!(rendered.value.contains("2048 total"));
    Ok(())
}
