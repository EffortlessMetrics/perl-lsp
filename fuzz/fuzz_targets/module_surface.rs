#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_module::{
    apply_module_rename_edits, contains_module_token, contains_standalone_module_token,
    extract_module_reference, extract_module_reference_extended, extract_require_import_symbols,
    file_path_to_module_name, find_module_reference, find_module_reference_extended,
    find_standalone_module_token_ranges, legacy_package_separator, line_references_isa_assignment,
    line_references_module_import, line_references_package_declaration,
    line_references_qualified_call, module_name_to_path, module_path_to_name, module_variant_pairs,
    normalize_package_separator, parse_module_import_head, parse_module_token,
    plan_module_rename_edits, replace_module_name_prefix, replace_module_token,
    resolve_known_export_tag,
};

const MAX_INPUT_BYTES: usize = 1536;
const MAX_SNIPPET_CHARS: usize = 160;
const MAX_MODULE_CHARS: usize = 64;
const MAX_CURSOR_CHECKS: usize = 96;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };
    String::from_utf8_lossy(capped)
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn module_name_from(input: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut previous_was_sep = false;

    for ch in input.chars() {
        let candidate = if ch.is_ascii_alphanumeric() || ch == '_' {
            Some(ch)
        } else if matches!(ch, ':' | '/' | '\\' | '-' | '.') {
            Some(':')
        } else {
            None
        };

        let Some(candidate) = candidate else { continue };

        if candidate == ':' {
            if out.is_empty() || previous_was_sep {
                continue;
            }
            out.push_str("::");
            previous_was_sep = true;
        } else {
            out.push(candidate);
            previous_was_sep = false;
        }

        if out.len() >= MAX_MODULE_CHARS {
            break;
        }
    }

    while out.ends_with(':') {
        out.pop();
    }

    if out.is_empty() || out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        fallback.to_string()
    } else {
        out
    }
}

fn perl_single_quote(input: &str) -> String {
    input.replace('\\', "\\\\").replace('\'', "\\'")
}

fn char_boundary_offsets(input: &str) -> impl Iterator<Item = usize> + '_ {
    input.char_indices().map(|(offset, _)| offset).chain(std::iter::once(input.len()))
}

fn exercise_name_and_path(input: &str, module: &str, replacement: &str) {
    let path = module_name_to_path(module);
    let roundtrip_name = module_path_to_name(&path);
    let file_name = file_path_to_module_name(&path);

    let _ = normalize_package_separator(module);
    let _ = legacy_package_separator(&roundtrip_name);
    let _ = module_variant_pairs(module, replacement);
    let _ = module_variant_pairs(&file_name, module);
    let _ = module_path_to_name(input);
    let _ = file_path_to_module_name(input);
}

fn exercise_token_surfaces(input: &str, module: &str, replacement: &str) {
    let snippet = truncate_chars(input, MAX_SNIPPET_CHARS);
    let line = format!("use {module}; package {module}; {module}->method(); # {snippet}");

    let _ = contains_module_token(&line, module);
    let _ = contains_standalone_module_token(&line, module);
    let _ = line_references_module_import(&line, module);
    let _ = line_references_package_declaration(&line, module);
    let _ = line_references_qualified_call(&line, module);
    let _ = line_references_isa_assignment(&format!("our @ISA = ('{module}');"), module);

    let (replaced, changed) = replace_module_token(&line, module, replacement);
    if changed {
        let _ = contains_module_token(&replaced, replacement);
    }

    let _ = replace_module_name_prefix(&line, module, replacement);

    for range in find_standalone_module_token_ranges(&line, module).take(16) {
        let _ = &line[range.start..range.end];
    }

    for offset in char_boundary_offsets(&line).take(MAX_CURSOR_CHECKS) {
        let _ = parse_module_token(&line, offset);
    }
}

fn exercise_import_and_reference(input: &str, module: &str, replacement: &str) {
    let snippet = truncate_chars(input, MAX_SNIPPET_CHARS);
    let quoted = perl_single_quote(&snippet);
    let source = format!(
        "use {module};\nuse parent '{replacement}';\nuse base qw({module} {replacement});\nrequire {module};\nrequire '{quoted}.pm';\n{snippet}\n"
    );

    for line in source.lines() {
        if let Some(head) = parse_module_import_head(line) {
            let _ = head.kind.dispatch_semantics();
            let _ = head.require_form();
            let _ = head.import_list;
        }
    }

    let _ = extract_require_import_symbols(&source);
    let _ = resolve_known_export_tag(module, input);
    let _ = resolve_known_export_tag("POSIX", input.trim_start_matches(':'));

    for offset in char_boundary_offsets(&source).take(MAX_CURSOR_CHECKS) {
        if let Some(reference) = find_module_reference(&source, offset) {
            let _ = reference.canonical_module_name();
        }
        if let Some(reference) = find_module_reference_extended(&source, offset) {
            let _ = reference.canonical_module_name();
        }
        let _ = extract_module_reference(&source, offset);
        let _ = extract_module_reference_extended(&source, offset);
    }
}

fn exercise_rename(input: &str, module: &str, replacement: &str) {
    let snippet = truncate_chars(input, MAX_SNIPPET_CHARS);
    let source = format!(
        "package {module};\nuse {module};\nour @ISA = ('{module}');\n{module}->new();\n{snippet}\n"
    );

    let edits = plan_module_rename_edits(&source, module, replacement);
    let renamed = apply_module_rename_edits(&source, &edits);
    let _ = plan_module_rename_edits(&renamed, replacement, module);
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);
    let source = input.as_ref();
    let module = module_name_from(source, "Fuzz::Module");
    let replacement =
        module_name_from(&source.chars().rev().collect::<String>(), "Fuzz::Replacement");

    exercise_name_and_path(source, &module, &replacement);
    exercise_token_surfaces(source, &module, &replacement);
    exercise_import_and_reference(source, &module, &replacement);
    exercise_rename(source, &module, &replacement);
});
