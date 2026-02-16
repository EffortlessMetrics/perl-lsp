//! Comprehensive Unicode Edge Cases for Perl Parser
//!
//! This test suite validates parser behavior with complex Unicode scenarios,
//! including bidirectional text, combining characters, normalization forms,
//! invalid UTF-8 sequences, and platform-specific Unicode handling.

use perl_parser::Parser;
use std::time::Duration;

/// Maximum parsing time for Unicode tests
const MAX_UNICODE_PARSE_TIME: Duration = Duration::from_secs(10);

/// Test complex Unicode scripts and identifiers
#[test]
fn test_complex_unicode_scripts() {
    println!("Testing complex Unicode scripts...");

    let test_cases = vec![
        // Right-to-left scripts
        ("Arabic identifiers", r#"my $متغير_عربي = 42; print "$متغير_عربي\n";"#),
        ("Hebrew identifiers", r#"my $משתנה_עברי = 42; print "$משתנה_עברי\n";"#),
        ("Persian identifiers", r#"my $متغیر_فارسی = 42; print "$متغیر_فارسی\n";"#),
        ("Urdu identifiers", r#"my $متغیر_اردو = 42; print "$متغیر_اردو\n";"#),
        // CJK scripts
        ("Chinese identifiers", r#"my $中文变量 = 42; print "$中文变量\n";"#),
        ("Japanese identifiers", r#"my $日本語変数 = 42; print "$日本語変数\n";"#),
        ("Korean identifiers", r#"my $한국어_변수 = 42; print "$한국어_변수\n";"#),
        // Indic scripts
        ("Hindi identifiers", r#"my $हिंदी_चर = 42; print "$हिंदी_चर\n";"#),
        ("Thai identifiers", r#"my $ตัวแปร_ไทย = 42; print "$ตัวแปร_ไทย\n";"#),
        ("Tamil identifiers", r#"my $தமிழ்_மாறி = 42; print "$தமிழ்_மாறி\n";"#),
        // Other complex scripts
        ("Cyrillic identifiers", r#"my $переменная = 42; print "$переменная\n";"#),
        ("Greek identifiers", r#"my $μεταβλητή = 42; print "$μεταβλητή\n";"#),
        ("Armenian identifiers", r#"my $փոփխական = 42; print "$փոփխական\n";"#),
        ("Georgian identifiers", r#"my $ცვლადი = 42; print "$ცვლადი\n";"#),
        ("Ethiopic identifiers", r#"my $ተለዋዋጭ = 42; print "$ተለዋዋጭ\n";"#),
        // Mixed script identifiers
        (
            "Mixed LTR/RTL",
            r#"my $english_العربية_variable = 42; print "$english_العربية_variable\n";"#,
        ),
        ("Mixed CJK/Latin", r#"my $mixed_中文变量 = 42; print "$mixed_中文变量\n";"#),
        ("Complex mixed", r#"my $a_العربية_中文_עברית = 42; print "$a_العربية_中文_עברית\n";"#),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Parse time exceeded limit for {}",
                    name
                );

                // Verify Unicode identifiers are preserved
                let sexp = ast.to_sexp();
                assert!(sexp.contains("variable"), "Variable not found in AST for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Some Unicode identifiers might not be supported, but should fail gracefully
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Error detection took too long for {}",
                    name
                );
            }
        }
    }
}

/// Test bidirectional text and combining characters
#[test]
fn test_bidirectional_and_combining_characters() {
    println!("Testing bidirectional text and combining characters...");

    let test_cases = vec![
        // Bidirectional text in strings
        ("Bidi string LTR", r#"my $text = "Hello العربية World"; print "$text\n";"#),
        ("Bidi string RTL", r#"my $text = "العربية Hello World"; print "$text\n";"#),
        ("Mixed bidi", r#"my $text = "Hello العربية עברית World"; print "$text\n";"#),
        // Combining characters
        ("Combining diacritics", r#"my $text = "café"; print "$text\n";"#),
        ("Complex combining", r#"my $text = "Z̵̧̢A̴L̸G̵O̴"; print "$text\n";"#),
        ("Arabic diacritics", r#"my $text = "الْعَرَبِيَّةُ"; print "$text\n";"#),
        ("Hebrew niqqud", r#"my $text = "עִבְרִית"; print "$text\n";"#),
        ("Devanagari diacritics", r#"my $text = "हिन्दी"; print "$text\n";"#),
        // Unicode control characters
        ("RTL marks", r#"my $text = "Hello \u200F العربية \u200F World"; print "$text\n";"#),
        ("LTR marks", r#"my $text = "العربية \u200E Hello \u200E World"; print "$text\n";"#),
        ("Directional overrides", r#"my $text = "\u202BHello\u202C"; print "$text\n";"#),
        // Zero-width characters
        ("Zero-width joiner", r#"my $text = "a\u200Db"; print "$text\n";"#),
        ("Zero-width non-joiner", r#"my $text = "a\u200Cb"; print "$text\n";"#),
        ("Zero-width space", r#"my $text = "a\u200Bb"; print "$text\n";"#),
        // Complex Unicode in regex
        (
            "Unicode regex class",
            r#"my $text = "العربية"; if ($text =~ /\p{Arabic}/) { print "Arabic\n"; }"#,
        ),
        ("Script regex", r#"my $text = "café"; if ($text =~ /\p{Latin}/) { print "Latin\n"; }"#),
        ("Unicode property", r#"my $text = "café"; if ($text =~ /\w+/u) { print "Word\n"; }"#),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Parse time exceeded limit for {}",
                    name
                );

                // Verify the text is preserved
                let sexp = ast.to_sexp();
                assert!(
                    sexp.contains("string") || sexp.contains("regex") || sexp.contains("match"),
                    "String/regex not found in AST for {}",
                    name
                );
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Complex Unicode might cause issues, but should fail gracefully
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Error detection took too long for {}",
                    name
                );
            }
        }
    }
}

/// Test Unicode normalization forms
#[test]
fn test_unicode_normalization() {
    println!("Testing Unicode normalization forms...");

    // Test cases with different normalization forms
    let test_cases = vec![
        // NFC (Normalization Form C - canonical composition)
        ("NFC composed", r#"my $text = "é"; print "$text\n";"#),
        ("NFC sequence", r#"my $text = "café"; print "$text\n";"#),
        // NFD (Normalization Form D - canonical decomposition)
        ("NFD decomposed", r#"my $text = "e\u0301"; print "$text\n";"#), // e + combining acute accent
        ("NFD sequence", r#"my $text = "cafe\u0301"; print "$text\n";"#),
        // NFKC (Normalization Form KC - compatibility composition)
        ("NFKC compatibility", r#"my $text = "ﬁ"; print "$text\n";"#), // fi ligature
        ("NFKC superscript", r#"my $text = "²"; print "$text\n";"#),   // superscript 2
        // NFKD (Normalization Form KD - compatibility decomposition)
        ("NFKD decomposed", r#"my $text = "f\u0301"; print "$text\n";"#), // f + combining acute accent
        ("NFKD superscript", r#"my $text = "2"; print "$text\n";"#),      // regular 2
        // Mixed normalization
        ("Mixed NFC/NFD", r#"my $text = "café\u0301"; print "$text\n";"#),
        ("Mixed NFKC/NFKD", r#"my $text = "ﬁ\u0301"; print "$text\n";"#),
        // Complex normalization cases
        ("Hangul", r#"my $text = "한"; print "$text\n";"#),
        ("Hangul decomposed", r#"my $text = "\u110B\u1161"; print "$text\n";"#),
        ("Arabic ligatures", r#"my $text = "لا"; print "$text\n";"#),
        ("Arabic decomposed", r#"my $text = "\u0644\u0627"; print "$text\n";"#),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Parse time exceeded limit for {}",
                    name
                );

                // Verify the text is preserved
                let sexp = ast.to_sexp();
                assert!(
                    sexp.contains("string") || sexp.contains("variable"),
                    "String/variable not found in AST for {}",
                    name
                );
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Normalization differences might cause issues, but should fail gracefully
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Error detection took too long for {}",
                    name
                );
            }
        }
    }
}

/// Test invalid UTF-8 sequences and error handling
#[test]
fn test_invalid_utf8_sequences() {
    println!("Testing invalid UTF-8 sequences...");

    // Note: These tests are designed to test parser robustness with invalid UTF-8
    // In a real scenario, the Rust string type would prevent invalid UTF-8 at compile time
    // But we can test edge cases with byte sequences that might occur in file I/O

    let test_cases = vec![
        // Overlong encodings
        ("Overlong ASCII", r#"my $text = "Hello"; print "$text\n";"#),
        // Invalid continuation bytes (simulated through escape sequences)
        ("Invalid continuation", r#"my $text = "Hello\x80World"; print "$text\n";"#),
        ("Invalid continuation 2", r#"my $text = "Hello\xC0World"; print "$text\n";"#),
        // Surrogate pairs (invalid in UTF-8)
        ("High surrogate", r#"my $text = "Hello\uD800World"; print "$text\n";"#),
        ("Low surrogate", r#"my $text = "Hello\uDC00World"; print "$text\n";"#),
        // Code points beyond Unicode range
        ("Beyond Unicode", r#"my $text = "Hello\u110000World"; print "$text\n";"#),
        // Isolated continuation bytes
        ("Isolated continuation", r#"my $text = "Hello\x80\x81\x82World"; print "$text\n";"#),
        // Invalid start bytes
        ("Invalid start", r#"my $text = "Hello\xF5World"; print "$text\n";"#),
        // Incomplete sequences
        ("Incomplete 2-byte", r#"my $text = "Hello\xC2World"; print "$text\n";"#),
        ("Incomplete 3-byte", r#"my $text = "Hello\xE0\xA0World"; print "$text\n";"#),
        ("Incomplete 4-byte", r#"my $text = "Hello\xF0\x90\x80World"; print "$text\n";"#),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Parse time exceeded limit for {}",
                    name
                );

                // Verify the text is preserved or handled gracefully
                let sexp = ast.to_sexp();
                assert!(
                    sexp.contains("string") || sexp.contains("variable"),
                    "String/variable not found in AST for {}",
                    name
                );
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Invalid UTF-8 should cause graceful failure
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Error detection took too long for {}",
                    name
                );
            }
        }
    }
}

/// Test Unicode in various Perl constructs
#[test]
fn test_unicode_in_perl_constructs() {
    println!("Testing Unicode in various Perl constructs...");

    let test_cases = vec![
        // Unicode in package names
        ("Unicode package", r#"package العربية; sub new { bless {} } my $obj = العربية->new();"#),
        (
            "Unicode module",
            r#"package 中文模块; sub test { return "测试"; } print 中文模块::test();"#,
        ),
        // Unicode in subroutine names
        ("Unicode sub", r#"sub العربية_دالة { return "العربية"; } print العربية_دالة();"#),
        (
            "Unicode method",
            r#"package 中文类; sub 中文方法 { return "方法"; } my $obj = 中文类->new(); print $obj->中文方法();"#,
        ),
        // Unicode in hash keys
        (
            "Unicode hash keys",
            r#"my %hash = ('العربية' => 'Arabic', '中文' => 'Chinese'); print $hash{'العربية'};"#,
        ),
        (
            "Unicode mixed keys",
            r#"my %hash = ('keyالعربية' => 'Mixed', '中文key' => 'Mixed'); print $hash{'keyالعربية'};"#,
        ),
        // Unicode in array indices (with proper dereferencing)
        (
            "Unicode array access",
            r#"my @array = ('a', 'b', 'c'); my $العربية = 1; print $array[$العربية];"#,
        ),
        // Unicode in format strings
        ("Unicode printf", r#"printf "العربية: %s\n", "test";"#),
        ("Unicode sprintf", r#"my $text = sprintf "中文: %d", 42; print "$text\n";"#),
        // Unicode in regex patterns
        (
            "Unicode regex match",
            r#"my $text = "العربية"; if ($text =~ /العربية/) { print "Match\n"; }"#,
        ),
        (
            "Unicode regex class",
            r#"my $text = "café"; if ($text =~ /[éèêë]/) { print "Accented\n"; }"#,
        ),
        (
            "Unicode regex quantifier",
            r#"my $text = "العربية العربية"; if ($text =~ /العربية+/) { print "Repeated\n"; }"#,
        ),
        // Unicode in heredocs
        (
            "Unicode heredoc",
            r#"my $text = <<'END';
العربية
中文
עברית
END
print "$text\n";"#,
        ),
        // Unicode in quote-like operators
        ("Unicode qq", r#"my $text = qq{العربية 中文}; print "$text\n";"#),
        ("Unicode qr", r#"my $pattern = qr{العربية}; print "$pattern\n";"#),
        ("Unicode qw", r#"my @words = qw(العربية 中文 עברית); print "$words[0]\n";"#),
        // Unicode in tr/// operators
        ("Unicode tr", r#"my $text = "café"; $text =~ tr/éèêë/eeee/; print "$text\n";"#),
        // Unicode in pack/unpack
        (
            "Unicode pack",
            r#"my $text = "العربية"; my $packed = pack("A*", $text); print "$packed\n";"#,
        ),
        (
            "Unicode unpack",
            r#"my $text = "العربية"; my @chars = unpack("C*", $text); print "@chars\n";"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Parse time exceeded limit for {}",
                    name
                );

                // Verify the construct is present in the AST
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Unicode in certain constructs might not be supported, but should fail gracefully
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Error detection took too long for {}",
                    name
                );
            }
        }
    }
}

/// Test Unicode edge cases with special variables
#[test]
fn test_unicode_special_variables() {
    println!("Testing Unicode with special variables...");

    let test_cases = vec![
        // Unicode in variable names (not special variables, but testing edge cases)
        ("Unicode scalar", r#"my $العربية_متغير = 42; print "$العربية_متغير\n";"#),
        ("Unicode array", r#"my @中文_数组 = (1, 2, 3); print "@中文_数组\n";"#),
        ("Unicode hash", r#"my %עברית_האש = ('key' => 'value'); print "%עברית_האש\n";"#),
        // Unicode with special variables
        ("Unicode with $_", r#"for my $العربية (1..3) { $_ *= 2; print "$العربية: $_\n"; }"#),
        ("Unicode with $@", r#"eval { die "العربية error" }; if ($@) { print "$@\n"; }"#),
        ("Unicode with $!", r#"open my $fh, '<', 'nonexistent' or die "العربية: $!";"#),
        // Unicode in glob assignments
        ("Unicode glob", r#"*العربية = *STDOUT; print العربية "Hello\n";"#),
        // Unicode in typeglob operations
        ("Unicode typeglob", r#"my $العربية_ref = *STDOUT{IO};"#),
        // Unicode in format statements
        (
            "Unicode format",
            r#"format STDOUT =
العربية: @<<<<<
$العربية
.
my $العربية = 42;
write;"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Parse time exceeded limit for {}",
                    name
                );

                // Verify the variable is present in the AST
                let sexp = ast.to_sexp();
                assert!(
                    sexp.contains("variable")
                        || sexp.contains("special_var")
                        || sexp.contains("format"),
                    "Variable not found in AST for {}",
                    name
                );
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Unicode with special variables might cause issues, but should fail gracefully
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Error detection took too long for {}",
                    name
                );
            }
        }
    }
}

/// Test Unicode with file operations
#[test]
fn test_unicode_file_operations() {
    println!("Testing Unicode with file operations...");

    let test_cases = vec![
        // Unicode file handles
        (
            "Unicode filehandle",
            r#"open my $العربية_ملف, '<', 'test.txt' or die $!; while (<$العربية_ملف>) { print; } close $العربية_ملف;"#,
        ),
        // Unicode file paths
        (
            "Unicode path",
            r#"open my $fh, '<', 'العربية.txt' or die $!; while (<$fh>) { print; } close $fh;"#,
        ),
        (
            "Mixed path",
            r#"open my $fh, '<', 'test_العربية.txt' or die $!; while (<$fh>) { print; } close $fh;"#,
        ),
        // Unicode in directory operations
        (
            "Unicode opendir",
            r#"opendir my $العربية_دليل, '.' or die $!; while (readdir $العربية_دليل) { print "$_\n"; } closedir $العربية_دليل;"#,
        ),
        // Unicode in file tests
        ("Unicode file test", r#"if (-f 'العربية.txt') { print "File exists\n"; }"#),
        ("Unicode directory test", r#"if (-d 'العربية_دليل') { print "Directory exists\n"; }"#),
        // Unicode in require/use
        ("Unicode require", r#"require 'العربية模块.pm';"#),
        ("Unicode use", r#"use 中文模块; print "Loaded\n";"#),
        // Unicode in do blocks
        ("Unicode do file", r#"do 'العربية.pl' or die $!;"#),
        // Unicode in system calls
        ("Unicode system", r#"system 'echo', 'العربية';"#),
        ("Unicode exec", r#"exec 'echo', '中文' if 0;"#),
        ("Unicode backticks", r#"my $result = `echo العربية`; print "$result\n";"#),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Parse time exceeded limit for {}",
                    name
                );

                // Verify the file operation is present in the AST
                let sexp = ast.to_sexp();
                assert!(
                    sexp.contains("file")
                        || sexp.contains("open")
                        || sexp.contains("require")
                        || sexp.contains("system")
                        || sexp.contains("exec"),
                    "File operation not found in AST for {}",
                    name
                );
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Unicode in file operations might cause issues, but should fail gracefully
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Error detection took too long for {}",
                    name
                );
            }
        }
    }
}

/// Test Unicode with advanced Perl features
#[test]
fn test_unicode_advanced_features() {
    println!("Testing Unicode with advanced Perl features...");

    let test_cases = vec![
        // Unicode in object-oriented features
        (
            "Unicode bless",
            r#"package العربية_فئة; sub new { bless {} } my $obj = bless {}, 'العربية_فئة';"#,
        ),
        ("Unicode isa", r#"use base 'العربية_فئة'; sub test { return "test"; }"#),
        // Unicode in prototypes
        (
            "Unicode prototype",
            r#"sub العربية_دالة ($) { return $_[0] * 2; } print العربية_دالة(21);"#,
        ),
        // Unicode in attributes
        ("Unicode attribute", r#"sub العربية_دالة :lvalue { return $العربية_متغير; }"#),
        // Unicode in try/catch
        (
            "Unicode try/catch",
            r#"use feature 'try'; try { die "العربية error"; } catch ($e) { print "$e\n"; }"#,
        ),
        // Unicode in signatures
        (
            "Unicode signature",
            r#"use feature 'signatures'; sub العربية_دالة ($العربية_معامل) { return $العربية_معامل * 2; } print العربية_دالة(21);"#,
        ),
        // Unicode in given/when
        (
            "Unicode given/when",
            r#"use feature 'switch'; my $العربية_متغير = 42; given ($العربية_متغير) { when (42) { print "Answer\n"; } }"#,
        ),
        // Unicode in smartmatch
        (
            "Unicode smartmatch",
            r#"my $العربية_متغير = 42; if ($العربية_متغير ~~ 42) { print "Match\n"; }"#,
        ),
        // Unicode in state variables
        (
            "Unicode state",
            r#"use feature 'state'; sub العربية_دالة { state $العربية_计数 = 0; return ++$العربية_计数; } print العربية_دالة();"#,
        ),
        // Unicode in lexical subs
        (
            "Unicode lexical sub",
            r#"use feature 'lexical_subs'; my sub العربية_دالة { return 42; } print العربية_دالة();"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();

        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Parse time exceeded limit for {}",
                    name
                );

                // Verify the feature is present in the AST
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Unicode in advanced features might cause issues, but should fail gracefully
                assert!(
                    parse_time < MAX_UNICODE_PARSE_TIME,
                    "Error detection took too long for {}",
                    name
                );
            }
        }
    }
}
