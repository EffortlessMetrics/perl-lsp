#[cfg(test)]
mod modernize_perl_tests {
    // Test both implementations
    use perl_parser::modernize::PerlModernizer;

    #[test]
    fn test_modernize_bareword_filehandles() {
        let modernizer = PerlModernizer::new();
        let old_code = "open FH, '<', 'file.txt';";

        let suggestions = modernizer.analyze(old_code);
        assert_eq!(suggestions.len(), 1);

        let suggestion = &suggestions[0];
        assert_eq!(suggestion.old_pattern, "open FH");
        assert_eq!(suggestion.new_pattern, "open my $fh");
        assert_eq!(suggestion.description, "Use lexical filehandles instead of barewords");

        let modernized = modernizer.apply(old_code);
        assert_eq!(modernized, "open my $fh, '<', 'file.txt';");
    }

    #[test]
    fn test_modernize_two_arg_open() {
        let modernizer = PerlModernizer::new();
        let old_code = "open(FH, 'file.txt');";

        let suggestions = modernizer.analyze(old_code);
        assert_eq!(suggestions.len(), 1);

        let suggestion = &suggestions[0];
        assert_eq!(suggestion.description, "Use three-argument open for safety");

        let modernized = modernizer.apply(old_code);
        assert_eq!(modernized, "open(my $fh, '<', 'file.txt');");
    }

    #[test]
    fn test_modernize_defined_array() {
        let modernizer = PerlModernizer::new();
        let old_code = "if (defined @array) { }";

        let suggestions = modernizer.analyze(old_code);
        assert_eq!(suggestions.len(), 1);

        let suggestion = &suggestions[0];
        assert_eq!(
            suggestion.description,
            "defined(@array) is deprecated, use @array in boolean context"
        );

        let modernized = modernizer.apply(old_code);
        assert_eq!(modernized, "if (@array) { }");
    }

    #[test]
    fn test_modernize_indirect_object_notation() {
        let modernizer = PerlModernizer::new();
        let old_code = "my $obj = new Class($arg);";

        let suggestions = modernizer.analyze(old_code);
        assert_eq!(suggestions.len(), 1);

        let suggestion = &suggestions[0];
        assert_eq!(
            suggestion.description,
            "Use direct method call instead of indirect object notation"
        );

        let modernized = modernizer.apply(old_code);
        assert_eq!(modernized, "my $obj = Class->new($arg);");
    }

    #[test]
    fn test_modernize_each_on_array() {
        let modernizer = PerlModernizer::new();
        let old_code = "while (my ($i, $val) = each @array) { }";

        let suggestions = modernizer.analyze(old_code);
        assert_eq!(suggestions.len(), 1);

        let suggestion = &suggestions[0];
        assert_eq!(
            suggestion.description,
            "each(@array) can cause unexpected behavior, use foreach with index"
        );

        let modernized = modernizer.apply(old_code);
        assert_eq!(modernized, "foreach my $i (0..$#array) { my $val = $array[$i]; }");
    }

    #[test]
    fn test_modernize_string_eval() {
        let modernizer = PerlModernizer::new();
        let old_code = "eval \"use $module\";";

        let suggestions = modernizer.analyze(old_code);
        assert_eq!(suggestions.len(), 1);

        let suggestion = &suggestions[0];
        assert_eq!(suggestion.description, "String eval is risky, consider block eval or require");

        // For this case, we suggest but don't auto-fix due to complexity
        let modernized = modernizer.apply(old_code);
        assert_eq!(modernized, old_code); // No automatic fix for risky patterns
        assert!(suggestion.manual_review_required);
    }

    #[test]
    fn test_modernize_use_strict_warnings() {
        let modernizer = PerlModernizer::new();
        let old_code = "#!/usr/bin/perl\n\nmy $x = 1;";

        let suggestions = modernizer.analyze(old_code);
        assert_eq!(suggestions.len(), 1);

        let suggestion = &suggestions[0];
        assert_eq!(
            suggestion.description,
            "Add 'use strict' and 'use warnings' for better code quality"
        );

        let modernized = modernizer.apply(old_code);
        assert_eq!(modernized, "#!/usr/bin/perl\nuse strict;\nuse warnings;\n\nmy $x = 1;");
    }

    #[test]
    fn test_modernize_say_instead_of_print() {
        let modernizer = PerlModernizer::new();
        let old_code = "print \"Hello\\n\";";

        let suggestions = modernizer.analyze(old_code);
        assert_eq!(suggestions.len(), 1);

        let suggestion = &suggestions[0];
        assert_eq!(
            suggestion.description,
            "Use 'say' instead of print with \\n (requires use feature 'say')"
        );

        let modernized = modernizer.apply(old_code);
        assert_eq!(modernized, "say \"Hello\";");
    }

    #[test]
    fn test_multiple_modernizations() {
        let modernizer = PerlModernizer::new();
        let old_code = r#"#!/usr/bin/perl

open FH, "file.txt";
print FH "Hello\n";
close FH;

my $obj = new MyClass();
"#;

        let suggestions = modernizer.analyze(old_code);
        assert!(suggestions.len() >= 3); // strict/warnings, filehandle, indirect notation

        let modernized = modernizer.apply(old_code);
        assert!(modernized.contains("use strict"));
        assert!(modernized.contains("use warnings"));
        assert!(modernized.contains("open my $fh"));
        assert!(modernized.contains("MyClass->new()"));
    }

    #[test]
    fn test_preserve_modern_code() {
        let modernizer = PerlModernizer::new();
        let modern_code = r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'say';

open my $fh, '<', 'file.txt';
say "Hello";
my $obj = MyClass->new();
"#;

        let suggestions = modernizer.analyze(modern_code);
        assert_eq!(suggestions.len(), 0, "Modern code should have no suggestions");

        let modernized = modernizer.apply(modern_code);
        assert_eq!(modernized, modern_code, "Modern code should not be changed");
    }
}
