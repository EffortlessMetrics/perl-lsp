use lsp_types::CompletionItemKind;
use perl_lsp_completion::CompletionProvider;
/// Edge case and regression tests for hash key completion (issue #4264)
///
/// These tests verify:
/// - Single-key hashes
/// - Quoted keys
/// - Multiline definitions
/// - Individual key assignments
/// - Mixed definition patterns
/// - Whitespace handling
/// - Nested hash access
/// - Hash slice contexts (should NOT trigger)
/// - Double-sigil derefs (should NOT trigger)
/// - Prefix filtering accuracy
/// - Case sensitivity
/// - Duplicate key deduplication
/// - Numeric and underscore keys
/// - Empty hashes
/// - Fat comma and string conversion
/// - String interpolation contexts
use perl_parser::Parser;
use perl_tdd_support::must;

#[test]
fn test_hash_key_completion_single_key() {
    let code = "my %map = (single => 'value');\n$map{sin";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "single"),
        "single key hash should complete; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_quoted_keys() {
    let code = "my %quoted = ('first_key' => 1, \"second_key\" => 2);\n$quoted{fir";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "first_key"),
        "quoted key 'first_key' should be extracted; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "second_key"),
        "quoted key 'second_key' should be extracted; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_multiline_definition() {
    let code = "my %config = (\n  host => 'localhost',\n  port => 5432,\n);\n$config{ho";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "host"),
        "multiline hash should extract keys correctly; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "port"),
        "multiline hash should extract all keys; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_individual_key_assignment() {
    let code = "$config{database} = 'mydb';\n$config{d";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "database"),
        "individual key assignment should be recognized; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_mixed_definitions() {
    let code = "my %data = (color => 'red');\n$data{shade} = 'dark';\n$data{c";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "color"),
        "literal hash keys should be found; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "shade"),
        "individual assignment keys should be found; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_with_whitespace() {
    let code = "my %config = (hostname => 'localhost');\n$config  {  hos";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "hostname"),
        "whitespace around brace should be handled; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_nested_access() {
    let code = "my %outer = (key1 => 1);\nmy %inner = (nested => 2);\n$outer{key1}{nest";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.label == "key1" && c.kind == CompletionItemKind::Property),
        "nested hash access should not suggest keys from outer hash; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_does_not_fire_for_hash_slice() {
    let code = "my %config = (host => 'localhost', port => 5432);\n@config{ho";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.label == "host" && c.kind == CompletionItemKind::Property),
        "hash slice @config{{...}} should not trigger hash key completion; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_double_sigil_dereference() {
    let code = "my %data = (key => 'value');\n$$ref{ke";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.label == "key" && c.kind == CompletionItemKind::Property),
        "double-sigil deref $$ref{{...}} should not trigger hash key completion; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_prefix_filtering() {
    let code = "my %errors = (invalid_input => 'bad', invalid_format => 'ugly', valid_format => 'good');\n$errors{invalid_";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "invalid_input"),
        "prefix 'invalid_' should match 'invalid_input'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "invalid_format"),
        "prefix 'invalid_' should match 'invalid_format'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        !completions.iter().any(|c| c.label == "valid_format"),
        "prefix 'invalid_' should NOT match 'valid_format'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_case_sensitive() {
    let code = "my %config = (Host => 'localhost', host => 'local');\n$config{H";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "Host"),
        "uppercase prefix 'H' should match 'Host'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        !completions.iter().any(|c| c.label == "host"),
        "uppercase prefix 'H' should not match lowercase 'host'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_duplicate_keys() {
    let code = "my %dup = (key => 1, key => 2);\n$dup{k";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    let key_completions: Vec<_> = completions
        .iter()
        .filter(|c| c.label == "key" && c.kind == CompletionItemKind::Property)
        .collect();
    assert_eq!(
        key_completions.len(),
        1,
        "duplicate key should appear only once in completions; got {} occurrences",
        key_completions.len()
    );
}

#[test]
fn test_hash_key_completion_numeric_and_underscore_keys() {
    let code = "my %data = (key_1 => 'a', key_2 => 'b', _private => 'c', __init => 'd');\n$data{_";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "_private"),
        "underscore-prefix key '_private' should be found; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "__init"),
        "double-underscore key '__init' should be found; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        !completions.iter().any(|c| c.label == "key_1"),
        "prefix '_' should not match 'key_1'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_empty_hash() {
    let code = "my %empty = ();\n$empty{x";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    let property_completions: Vec<_> =
        completions.iter().filter(|c| c.kind == CompletionItemKind::Property).collect();
    assert!(
        property_completions.is_empty(),
        "empty hash should not suggest any keys; got: {:?}",
        property_completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_fat_comma_and_string_conversion() {
    let code = "my %config = (bare_word => 1, 'quoted' => 2);\n$config{b";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "bare_word"),
        "bare word left of fat comma should be extracted; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_in_string_no_suggestions() {
    let code = "my %config = (host => 'localhost');\nmy $s = \"$config{ho";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.label == "host" && c.kind == CompletionItemKind::Property),
        "hash key completion must not fire inside a string literal; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
}
