#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_lsp_rs_core::config::{ProjectConfig, ServerConfig, WorkspaceConfig};

const MAX_INPUT_BYTES: usize = 4096;
const MAX_FRAGMENT_CHARS: usize = 160;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    if data.is_empty() {
        return std::borrow::Cow::Borrowed("");
    }

    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };

    String::from_utf8_lossy(capped)
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn toml_basic_string(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars().take(MAX_FRAGMENT_CHARS) {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push('_'),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn json_string(input: &str) -> serde_json::Value {
    serde_json::Value::String(truncate_chars(input, MAX_FRAGMENT_CHARS))
}

fn exercise_project_config_toml(toml_source: &str) {
    if let Ok(project) = toml::from_str::<ProjectConfig>(toml_source) {
        let mut config = ServerConfig::default();
        project.apply_to_server_config(&mut config);

        // Applying the same project config repeatedly should stay total and
        // keep severity within the public clamp range.
        project.apply_to_server_config(&mut config);
        let _ = config.perlcritic_severity.clamp(1, 5);
    }
}

fn exercise_lsp_settings(settings: serde_json::Value) {
    let mut config = ServerConfig::default();
    config.update_from_value(&settings);
    config.update_from_value(&settings);
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);
    let input = input.as_ref();
    let escaped = toml_basic_string(input);

    // Raw fuzz bytes as TOML exercise syntax-error and unknown-key recovery.
    exercise_project_config_toml(input);

    // Structured TOML variants cover every section currently modeled by
    // ProjectConfig while still deriving values from arbitrary input.
    let toml_variants = [
        format!(
            r#"
[perl]
include_paths = ["{escaped}", "lib", "."]
version = "{escaped}"
use_perl5lib = true
perl5lib_precedence = "prepend"

[diagnostics]
perlcritic = true
perlcritic_severity = 99

[features]
inlay_hints = false
"#
        ),
        format!(
            r#"
[formatting]
enabled = true
engine = "{escaped}"
perltidy_profile = "{escaped}"
perltidy_maximum_line_length = 4294967295
perltidy_indent_columns = 4294967295
perltidy_tabs = false
perltidy_extra_args = ["{escaped}", "--standard-output"]

[critic]
engine = "{escaped}"
profile = "{escaped}"
include = ["{escaped}", "Variables::ProhibitUnusedVariables"]
exclude = ["", "{escaped}"]
"#
        ),
        format!(
            r#"
[ai_completion]
enabled = true
provider = "{escaped}"
endpoint = "{escaped}"
model = "{escaped}"
api_key_env = "{escaped}"
"#
        ),
    ];

    for source in &toml_variants {
        exercise_project_config_toml(source);
    }

    // LSP settings arrive as JSON and are intentionally permissive. Exercise
    // nested valid, invalid, oversized, and type-mismatched values.
    let settings = serde_json::json!({
        "inlayHints": {
            "enabled": true,
            "parameterHints": false,
            "typeHints": true,
            "chainedHints": false,
            "maxLength": u64::MAX,
        },
        "testRunner": {
            "enabled": true,
            "command": input,
            "args": [input, 7, null, true],
            "timeout": u64::MAX,
        },
        "telemetry": { "enabled": true },
        "perlcritic": {
            "enabled": false,
            "severity": u64::MAX,
            "profile": input,
            "theme": input,
        },
        "critic": {
            "engine": input,
            "profile": input,
            "include": [input, "", "Native::Rule"],
            "exclude": [7, input, null],
        },
        "formatting": {
            "enabled": true,
            "engine": input,
            "profile": input,
            "maximumLineLength": u64::MAX,
            "indentColumns": u64::MAX,
            "tabs": true,
            "openingBraceOnNewLine": false,
            "cuddledElse": true,
            "spaceAfterKeyword": false,
            "addTrailingCommas": true,
            "verticalAlignment": false,
            "blockCommentIndentation": u64::MAX,
            "extraArgs": [input, 1, false],
            "timeoutSecs": u64::MAX,
        },
        "aiCompletion": {
            "enabled": true,
            "provider": input,
            "endpoint": input,
            "model": input,
            "apiKeyEnv": input,
            "timeoutMs": u64::MAX,
            "maxOutputTokens": u64::MAX,
            "rateLimitRps": f64::MAX,
            "maxInflight": u64::MAX,
            "fallback": false,
            "streaming": {
                "enabled": true,
                "updateDebounceMs": u64::MAX,
            },
        },
    });
    exercise_lsp_settings(settings);

    exercise_lsp_settings(serde_json::json!({
        "inlayHints": json_string(input),
        "testRunner": [json_string(input)],
        "telemetry": null,
        "perlcritic": { "severity": json_string(input) },
        "formatting": { "extraArgs": [json_string(input)] },
    }));

    // PERL5LIB parsing is a public configuration helper used to ingest host
    // environment data. Include both Unix and Windows separators.
    let _ = WorkspaceConfig::parse_perl5lib(input);
    let _ = WorkspaceConfig::parse_perl5lib(&format!("{input}:{input};{input}"));
});
