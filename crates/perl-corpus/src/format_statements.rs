//! Format statement test fixtures for Perl LSP corpus.
//!
//! Provides comprehensive test cases for Perl's format/formline feature,
//! covering picture lines, field holders, and write() operations.

/// A format statement test fixture.
#[derive(Debug, Clone, Copy)]
pub struct FormatStatementCase {
    /// Stable identifier for the fixture.
    pub id: &'static str,
    /// Short human-readable description.
    pub description: &'static str,
    /// Tags for filtering and grouping.
    pub tags: &'static [&'static str],
    /// Perl source for the format statement.
    pub source: &'static str,
}

static FORMAT_STATEMENT_CASES: &[FormatStatementCase] = &[
    FormatStatementCase {
        id: "format.basic.declaration",
        description: "Basic format declaration with name.",
        tags: &["format", "declaration", "basic"],
        source: r#"format MYFORMAT =
@<<<<<<<<<<<
$text
.
"#,
    },
    FormatStatementCase {
        id: "format.stdout",
        description: "Format declaration for STDOUT filehandle.",
        tags: &["format", "stdout", "filehandle"],
        source: r#"my ($name, $age) = ("Ada", 37);
format STDOUT =
Name: @<<<<<<  Age: @>>
$name,       $age
.
write;
"#,
    },
    FormatStatementCase {
        id: "format.picture.left",
        description: "Format with left-justified picture lines.",
        tags: &["format", "picture", "alignment"],
        source: r#"my $text = "hello";
format REPORT =
@<<<<<<<<
$text
.
"#,
    },
    FormatStatementCase {
        id: "format.picture.right",
        description: "Format with right-justified picture lines.",
        tags: &["format", "picture", "alignment"],
        source: r#"my $value = 12345;
format NUMBERS =
@>>>>>>>>>
$value
.
"#,
    },
    FormatStatementCase {
        id: "format.picture.center",
        description: "Format with centered picture lines.",
        tags: &["format", "picture", "alignment"],
        source: r#"my $title = "Report";
format HEADER =
@|||||||||||
$title
.
"#,
    },
    FormatStatementCase {
        id: "format.mixed.alignment",
        description: "Format with mixed alignment picture lines.",
        tags: &["format", "picture", "alignment", "complex"],
        source: r#"my ($left, $center, $right) = ("L", "C", "R");
format MIXED =
@<<<<<  @|||||  @>>>>>
$left,  $center, $right
.
"#,
    },
    FormatStatementCase {
        id: "format.numeric.field",
        description: "Format with numeric field picture.",
        tags: &["format", "picture", "numeric"],
        source: r#"my $amount = 1234.56;
format MONEY =
$@##.##
$amount
.
"#,
    },
    FormatStatementCase {
        id: "format.multiline",
        description: "Format with multiple line specifications.",
        tags: &["format", "multiline", "complex"],
        source: r#"my ($name, $address, $city) = ("Ada Lovelace", "123 Main St", "London");
format ADDRESS =
Name:    @<<<<<<<<<<<<<<
         $name
Address: @<<<<<<<<<<<<<<
         $address
City:    @<<<<<<<<<<<<<<
         $city
.
"#,
    },
    FormatStatementCase {
        id: "format.multivalue.line",
        description: "Format with multiple values on a single line.",
        tags: &["format", "multivalue"],
        source: r#"my ($a, $b, $c) = (1, 2, 3);
format DATA =
@### @### @###
$a,  $b,  $c
.
"#,
    },
    FormatStatementCase {
        id: "format.text.block",
        description: "Format with text block field holder.",
        tags: &["format", "text-block", "multiline"],
        source: r#"my $paragraph = "This is a long paragraph that should be formatted across multiple lines.";
format PARAGRAPH =
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$paragraph
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$paragraph
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$paragraph
.
"#,
    },
    FormatStatementCase {
        id: "format.suppressed.blank",
        description: "Format with suppressed blank lines.",
        tags: &["format", "text-block", "suppressed"],
        source: r#"my $text = "Short text";
format SUPPRESS =
~~ ^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
   $text
~~ ^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
   $text
.
"#,
    },
    FormatStatementCase {
        id: "format.write.call",
        description: "Format declaration with write() call.",
        tags: &["format", "write", "execution"],
        source: r#"my $data = "test data";
format OUTPUT =
Data: @<<<<<<<<
      $data
.
write OUTPUT;
"#,
    },
    FormatStatementCase {
        id: "format.filehandle.association",
        description: "Format associated with a filehandle.",
        tags: &["format", "filehandle", "association"],
        source: r#"open my $fh, ">", "output.txt";
my $value = 42;
format $fh =
Result: @>>>
        $value
.
write $fh;
close $fh;
"#,
    },
    FormatStatementCase {
        id: "format.computed.field",
        description: "Format with computed field values.",
        tags: &["format", "computed", "expression"],
        source: r#"my ($x, $y) = (10, 20);
format COMPUTED =
Sum: @>>>
     $x + $y
.
"#,
    },
    FormatStatementCase {
        id: "format.variable.interpolation",
        description: "Format with variable interpolation in fields.",
        tags: &["format", "interpolation", "variable"],
        source: r#"my ($first, $last) = ("Ada", "Lovelace");
format FULLNAME =
@<<<<<< @<<<<<<<<
$first, $last
.
"#,
    },
    FormatStatementCase {
        id: "format.literal.text",
        description: "Format with literal text and field holders.",
        tags: &["format", "literal", "mixed"],
        source: r#"my $count = 5;
format REPORT =
Total items: @>>>
             $count
.
"#,
    },
    FormatStatementCase {
        id: "format.formline.builtin",
        description: "Formline builtin with accumulator and picture.",
        tags: &["format", "formline", "builtin"],
        source: r#"my $picture = "@<<<<<<";
my $value = "test";
formline $picture, $value;
my $formatted = $^A;
$^A = "";
"#,
    },
    FormatStatementCase {
        id: "format.accumulator.variable",
        description: "Format accumulator variable manipulation.",
        tags: &["format", "accumulator", "special-var"],
        source: r#"$^A = "";
formline "@>>", 42;
my $result = $^A;
$^A = "";
"#,
    },
    FormatStatementCase {
        id: "format.top.of.page",
        description: "Format with top-of-page header format.",
        tags: &["format", "header", "top"],
        source: r#"format STDOUT_TOP =
Page Header
-----------
.
format STDOUT =
@<<<<<
$data
.
my $data = "content";
write;
"#,
    },
    FormatStatementCase {
        id: "format.page.length",
        description: "Format with page length control.",
        tags: &["format", "page", "length", "special-var"],
        source: r#"$= = 60;  # Set page length
format STDOUT =
@<<<<<
$line
.
my $line = "text";
write;
"#,
    },
    FormatStatementCase {
        id: "format.lines.left",
        description: "Format with lines-left-on-page tracking.",
        tags: &["format", "page", "special-var"],
        source: r#"format STDOUT =
@<<<<<
$item
.
my $item = "data";
write;
my $remaining = $-;
"#,
    },
    FormatStatementCase {
        id: "format.name.special.var",
        description: "Format name special variable usage.",
        tags: &["format", "special-var", "name"],
        source: r#"format MYFORMAT =
@<<<<<
$text
.
$~ = "MYFORMAT";
my $text = "test";
write;
"#,
    },
    FormatStatementCase {
        id: "format.top.special.var",
        description: "Format top-of-page special variable.",
        tags: &["format", "special-var", "header"],
        source: r#"format MYFORMAT_TOP =
Header Line
.
$^ = "MYFORMAT_TOP";
"#,
    },
    FormatStatementCase {
        id: "format.empty",
        description: "Empty format declaration.",
        tags: &["format", "empty", "edge-case"],
        source: r#"format EMPTY =
.
"#,
    },
    FormatStatementCase {
        id: "format.nested.blocks",
        description: "Format with nested code blocks for field values.",
        tags: &["format", "nested", "complex"],
        source: r#"my @values = (1, 2, 3);
format BLOCKS =
@>>>
do { my $sum = 0; $sum += $_ for @values; $sum }
.
"#,
    },
    FormatStatementCase {
        id: "format.special.characters",
        description: "Format with special characters in picture lines.",
        tags: &["format", "special", "edge-case"],
        source: r#"my $value = "test";
format SPECIAL =
[@<<<<<]
 $value
.
"#,
    },
    FormatStatementCase {
        id: "format.array.iteration",
        description: "Format using array iteration for repeated output.",
        tags: &["format", "array", "iteration"],
        source: r#"my @items = ("one", "two", "three");
format LIST =
@<<<<<
$item
.
for my $item (@items) {
    write;
}
"#,
    },
    FormatStatementCase {
        id: "format.continuation.field",
        description: "Format with field continuation across lines.",
        tags: &["format", "continuation", "multiline"],
        source: r#"my $long_text = "This is a very long text that needs to continue";
format CONTINUE =
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$long_text
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$long_text
.
"#,
    },
    FormatStatementCase {
        id: "format.hash.values",
        description: "Format with hash value fields.",
        tags: &["format", "hash", "variable"],
        source: r#"my %data = (name => "Ada", age => 37);
format HASH =
Name: @<<<<<  Age: @>>
      $data{name}, $data{age}
.
"#,
    },
    FormatStatementCase {
        id: "format.method.result",
        description: "Format with method call results.",
        tags: &["format", "method", "oop"],
        source: r#"my $obj = MyClass->new();
format METHOD =
@<<<<<<<<
$obj->get_name()
.
"#,
    },
    FormatStatementCase {
        id: "format.justified.numeric",
        description: "Format with justified numeric fields.",
        tags: &["format", "numeric", "alignment"],
        source: r#"my ($price, $qty, $total) = (19.99, 5, 99.95);
format INVOICE =
Price: $@##.## Qty: @>> Total: $@###.##
        $price,      $qty,       $total
.
"#,
    },
    FormatStatementCase {
        id: "format.repeated.picture",
        description: "Format with repeated picture line patterns.",
        tags: &["format", "repeated", "pattern"],
        source: r#"my @values = (1, 2, 3, 4, 5);
my $value = shift @values;
format REPEAT =
@###
$value
@###
$value
@###
$value
.
"#,
    },
    FormatStatementCase {
        id: "format.conditional.field",
        description: "Format with conditional field evaluation.",
        tags: &["format", "conditional", "expression"],
        source: r#"my ($flag, $yes, $no) = (1, "YES", "NO");
format CONDITIONAL =
@<<<<<
$flag ? $yes : $no
.
"#,
    },
    FormatStatementCase {
        id: "format.write.to.variable",
        description: "Format output captured to a variable.",
        tags: &["format", "capture", "variable"],
        source: r#"my $output;
open my $fh, ">", \$output;
format $fh =
@<<<<<
$data
.
my $data = "test";
write $fh;
close $fh;
"#,
    },
    FormatStatementCase {
        id: "format.legacy.compatibility",
        description: "Format with legacy Perl 4 style patterns.",
        tags: &["format", "legacy", "perl4"],
        source: r#"format LEGACY =
@<<<<<<<<<<<  @>>>>>>>>>>>>  @||||||||||||||
$left,        $right,        $center
.
my ($left, $right, $center) = ("L", "R", "C");
"#,
    },
];

/// Return all format statement test fixtures.
pub fn format_statement_cases() -> &'static [FormatStatementCase] {
    FORMAT_STATEMENT_CASES
}

/// Find a format statement fixture by ID.
pub fn find_format_case(id: &str) -> Option<&'static FormatStatementCase> {
    format_statement_cases().iter().find(|case| case.id == id)
}

/// Convenience helper for working with format statement fixtures.
pub struct FormatStatementGenerator;

impl FormatStatementGenerator {
    /// Return all available format statement cases.
    pub fn all_cases() -> &'static [FormatStatementCase] {
        format_statement_cases()
    }

    /// Return format statement cases with a matching tag.
    pub fn by_tag(tag: &str) -> Vec<&'static FormatStatementCase> {
        format_statement_cases().iter().filter(|case| case.tags.contains(&tag)).collect()
    }

    /// Return format statement cases that match any of the provided tags.
    pub fn by_tags_any(tags: &[&str]) -> Vec<&'static FormatStatementCase> {
        if tags.is_empty() {
            return format_statement_cases().iter().collect();
        }

        format_statement_cases()
            .iter()
            .filter(|case| case.tags.iter().any(|tag| tags.contains(tag)))
            .collect()
    }

    /// Return format statement cases that match all of the provided tags.
    pub fn by_tags_all(tags: &[&str]) -> Vec<&'static FormatStatementCase> {
        if tags.is_empty() {
            return format_statement_cases().iter().collect();
        }

        format_statement_cases()
            .iter()
            .filter(|case| tags.iter().all(|tag| case.tags.iter().any(|t| t == tag)))
            .collect()
    }

    /// Find a single format statement case by ID.
    pub fn find(id: &str) -> Option<&'static FormatStatementCase> {
        find_format_case(id)
    }

    /// Return sorted unique format statement tags.
    pub fn tags() -> Vec<&'static str> {
        let mut tags: Vec<&'static str> =
            format_statement_cases().iter().flat_map(|case| case.tags.iter().copied()).collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn format_cases_have_ids() {
        assert!(format_statement_cases().iter().all(|case| !case.id.is_empty()));
    }

    #[test]
    fn format_cases_have_descriptions() {
        assert!(format_statement_cases().iter().all(|case| !case.description.is_empty()));
    }

    #[test]
    fn format_cases_have_tags() {
        assert!(format_statement_cases().iter().all(|case| !case.tags.is_empty()));
    }

    #[test]
    fn format_cases_have_source() {
        assert!(format_statement_cases().iter().all(|case| !case.source.is_empty()));
    }

    #[test]
    fn format_case_ids_are_unique() {
        let mut seen = HashSet::new();
        for case in format_statement_cases() {
            assert!(seen.insert(case.id), "Duplicate format case id: {}", case.id);
        }
    }

    #[test]
    fn format_case_lookup_by_id() {
        let case = find_format_case("format.basic.declaration");
        assert!(case.is_some());
        assert_eq!(case.unwrap().id, "format.basic.declaration");
    }

    #[test]
    fn format_cases_can_filter_by_tag() {
        let basic = FormatStatementGenerator::by_tag("format");
        assert!(!basic.is_empty());
    }

    #[test]
    fn format_cases_can_filter_by_any_tag() {
        let matches = FormatStatementGenerator::by_tags_any(&["formline", "picture"]);
        assert!(!matches.is_empty());
    }

    #[test]
    fn format_cases_can_filter_by_all_tags() {
        let matches = FormatStatementGenerator::by_tags_all(&["format", "picture", "alignment"]);
        assert!(!matches.is_empty());
    }

    #[test]
    fn format_case_tags_are_unique() {
        let tags = FormatStatementGenerator::tags();
        let mut deduped = tags.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(tags, deduped);
    }

    #[test]
    fn format_basic_declaration_exists() {
        let case = FormatStatementGenerator::find("format.basic.declaration");
        assert!(case.is_some());
        assert!(case.unwrap().source.contains("format MYFORMAT"));
    }

    #[test]
    fn format_stdout_exists() {
        let case = FormatStatementGenerator::find("format.stdout");
        assert!(case.is_some());
        assert!(case.unwrap().source.contains("format STDOUT"));
    }

    #[test]
    fn format_picture_lines_exist() {
        let left = FormatStatementGenerator::find("format.picture.left");
        let right = FormatStatementGenerator::find("format.picture.right");
        let center = FormatStatementGenerator::find("format.picture.center");

        assert!(left.is_some());
        assert!(right.is_some());
        assert!(center.is_some());

        assert!(left.unwrap().source.contains("@<<<<<"));
        assert!(right.unwrap().source.contains("@>>>>>"));
        assert!(center.unwrap().source.contains("@|||||"));
    }

    #[test]
    fn format_formline_exists() {
        let case = FormatStatementGenerator::find("format.formline.builtin");
        assert!(case.is_some());
        assert!(case.unwrap().source.contains("formline"));
    }

    #[test]
    fn format_write_call_exists() {
        let case = FormatStatementGenerator::find("format.write.call");
        assert!(case.is_some());
        assert!(case.unwrap().source.contains("write"));
    }

    #[test]
    fn format_multiline_exists() {
        let case = FormatStatementGenerator::find("format.multiline");
        assert!(case.is_some());
    }

    #[test]
    fn format_computed_field_exists() {
        let case = FormatStatementGenerator::find("format.computed.field");
        assert!(case.is_some());
    }

    #[test]
    fn format_text_block_exists() {
        let case = FormatStatementGenerator::find("format.text.block");
        assert!(case.is_some());
        assert!(case.unwrap().source.contains("^<<<<<"));
    }
}
