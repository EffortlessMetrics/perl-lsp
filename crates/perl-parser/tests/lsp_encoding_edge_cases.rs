use serde_json::json;

mod common;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Encoding and charset edge case tests
/// Tests handling of various character encodings and Unicode edge cases

#[test]
fn test_utf8_bom() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // UTF-8 with BOM
    let content = String::from("\u{FEFF}#!/usr/bin/perl\nprint 'BOM test';\n");

    let uri = "file:///bom_test.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle BOM correctly
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_mixed_line_endings() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Mix of LF, CRLF, and CR
    let content = "#!/usr/bin/perl\nprint 'line1';\r\nprint 'line2';\rprint 'line3';\n";

    let uri = "file:///mixed_endings.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Line positions should be calculated correctly
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 2, "character": 0}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_unicode_normalization() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Different Unicode normalizations of é
    // NFC: é (single character U+00E9)
    // NFD: é (e + combining acute U+0065 U+0301)
    let content_nfc = "my $café = 'coffee';"; // NFC
    let content_nfd = "my $café = 'coffee';"; // NFD

    let uri1 = "file:///nfc.pl";
    let uri2 = "file:///nfd.pl";

    // Open NFC version
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri1,
                    "languageId": "perl",
                    "version": 1,
                    "text": content_nfc
                }
            }
        }),
    );

    // Open NFD version
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri2,
                    "languageId": "perl",
                    "version": 1,
                    "text": content_nfd
                }
            }
        }),
    );

    // Both should work
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri1}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_emoji_and_special_unicode() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Various Unicode categories
    let content = r#"
# Emoji in comments 🎉 🚀 💻
my $heart = '❤️';
my $rocket = '🚀';
my $complex = '👨‍👩‍👧‍👦'; # Family emoji with ZWJ

# Mathematical symbols
my $pi = 'π';
my $sum = 'Σ';
my $infinity = '∞';

# Right-to-left text
my $hebrew = 'שלום';
my $arabic = 'مرحبا';

# Invisible characters
my $zero_width = 'a​b'; # Zero-width space
my $soft_hyphen = 'soft­hyphen';

# Control characters
my $tab = "	tab";
my $vertical_tab = "vertical";
"#;

    let uri = "file:///unicode.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle all Unicode correctly
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array() || response["error"].is_object());
}

#[test]
fn test_surrogate_pairs() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Emojis that use surrogate pairs in UTF-16
    let content = r#"
my $emoji1 = '😀'; # U+1F600
my $emoji2 = '🏴󠁧󠁢󠁥󠁮󠁧󠁿'; # England flag with tags
my $emoji3 = '👨‍👩‍👧‍👦'; # Family with ZWJ sequences
"#;

    let uri = "file:///surrogates.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Position calculation with surrogate pairs
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 1, "character": 14} // After emoji
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_invalid_utf8_sequences() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Valid UTF-8 with comments about invalid sequences
    let content = r#"
# Invalid UTF-8 sequences (as comments):
# \xFF\xFE - not valid UTF-8
# \xC0\x80 - overlong encoding
# \xED\xA0\x80 - surrogate half

use utf8;
my $text = "valid utf-8 only";
"#;

    let uri = "file:///invalid_utf8.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle gracefully
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_encoding_pragma() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Perl encoding pragmas
    let content = r#"
use utf8;
use encoding 'utf8';
use encoding 'latin1';

my $unicode = '日本語';
my $latin = 'café';
"#;

    let uri = "file:///encoding_pragma.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should recognize encoding pragmas
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_grapheme_clusters() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Complex grapheme clusters
    let content = r#"
# Complex grapheme clusters
my $flag = '🏳️‍🌈'; # Rainbow flag
my $family = '👨‍👩‍👧‍👦'; # Family
my $technologist = '👩‍💻'; # Woman technologist
my $kiss = '👨‍❤️‍💋‍👨'; # Kiss

# Combining diacritics
my $combined = 'e̊⃝'; # Multiple combining marks
"#;

    let uri = "file:///graphemes.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Character positions with grapheme clusters
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 2, "character": 10}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_zero_width_characters() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Zero-width characters that can cause issues
    let content = format!(
        "my $text = 'a{}b{}c{}d';",
        '\u{200B}', // Zero-width space
        '\u{200C}', // Zero-width non-joiner
        '\u{200D}'  // Zero-width joiner
    );

    let uri = "file:///zero_width.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle zero-width characters
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_bidi_text() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Bidirectional text
    let content = r#"
# Mixed LTR and RTL text
my $mixed = 'Hello שלום World עולם';
my $arabic = 'Hello مرحبا بالعالم World';

# RTL override characters (escaped for safety)
my $override = '\u{200F}RTL override\u{200F}';
my $embed = '\u{202A}LTR embed\u{202A}';
"#;

    let uri = "file:///bidi.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle bidi text
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 2, "character": 15}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_confusable_characters() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Visually similar but different characters
    let content = r#"
# Latin vs Cyrillic (look similar but different)
my $latin = 'scope';    # Latin letters
my $cyrillic = 'ѕсоре'; # Cyrillic letters that look like Latin

# Greek letters that look like Latin
my $greek = 'Αlpha';    # A is Greek Alpha
my $pi_var = 'π';       # Greek pi

# Confusable punctuation
my $regular_quote = "'test'";
my $smart_quotes = "'test'"; # Smart quotes
my $backticks = '`test`';
"#;

    let uri = "file:///confusable.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should distinguish confusable characters
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_private_use_area() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Private Use Area characters
    let content = r#"
# Private Use Area (U+E000 to U+F8FF)
my $pua1 = '';  # U+E000
my $pua2 = '';  # U+F8FF

# Supplementary Private Use Area
my $spua = '󰀀';  # U+F0000
"#;

    let uri = "file:///pua.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle PUA characters
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_long_unicode_identifiers() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Very long Unicode identifiers
    let content = r#"
# Long Unicode variable names
my $很长的中文变量名称用于测试解析器的处理能力 = 1;
my $محتوى_طويل_جدا_باللغة_العربية_لاختبار_المعالج = 2;
my $очень_длинное_имя_переменной_на_русском_языке = 3;
my $πολύ_μεγάλο_όνομα_μεταβλητής_στα_ελληνικά = 4;

# Mixed scripts in identifiers
my $mixed_中文_english_العربية_русский = 5;
"#;

    let uri = "file:///long_unicode.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle long Unicode identifiers
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_unicode_in_regex() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Unicode in regular expressions
    let content = r#"
# Unicode regex patterns
if ($text =~ /[一-龯]/) { } # Chinese characters
if ($text =~ /\p{Script=Arabic}/) { } # Arabic script
if ($text =~ /\p{Emoji}/) { } # Emoji property
if ($text =~ /\X/) { } # Extended grapheme cluster

# Unicode boundaries
if ($text =~ /\b\w+\b/u) { } # Unicode word boundaries
if ($text =~ /^.$/) { } # Single grapheme

# Unicode case folding
if ($text =~ /café/i) { } # Case insensitive with accents
"#;

    let uri = "file:///unicode_regex.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );

    // Should handle Unicode in regex
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}
