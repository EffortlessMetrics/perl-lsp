//! Edge case tests for format strings and special blocks

pub fn get_tests() -> Vec<(&'static str, &'static str)> {
    vec![
        // Format declarations
        (
            r#"format STDOUT =
@<<<<< @||||| @>>>>>
$name, $age, $score
.
"#,
            "basic format declaration",
        ),
        (
            r#"format REPORT_TOP =
Name              Age Score
----              --- -----
.
"#,
            "format with header",
        ),
        (
            r#"format COMPLEX =
@<<<<<<<<<<<<<<<< @### @#.##
$item, $count, $price
~~                @### @#.##
$item2, $count2, $price2
.
"#,
            "format with multiple lines and ~~ continuation",
        ),
        ("format =\n.\n", "anonymous format"),
        (
            r#"format MINE =
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$text
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<~~
$text
.
"#,
            "format with multiline field",
        ),
        // Special blocks
        ("AUTOLOAD { print $AUTOLOAD }", "AUTOLOAD block"),
        ("DESTROY { cleanup() }", "DESTROY block"),
        ("BEGIN { require Module }", "BEGIN block"),
        ("END { close_files() }", "END block"),
        ("INIT { setup() }", "INIT block"),
        ("CHECK { validate() }", "CHECK block"),
        ("UNITCHECK { unit_setup() }", "UNITCHECK block"),
        // Nested special blocks
        ("BEGIN { BEGIN { $x = 1 } }", "nested BEGIN blocks"),
        ("END { BEGIN { warn 'odd' } }", "BEGIN inside END"),
        // Special blocks with labels
        ("FOO: BEGIN { last FOO if $done }", "labeled special block"),
        // Special subs
        (
            "sub AUTOLOAD { our $AUTOLOAD; print $AUTOLOAD }",
            "AUTOLOAD subroutine",
        ),
        ("sub DESTROY { shift->{handle}->close }", "DESTROY method"),
        ("sub CLONE { ... }", "CLONE method"),
        ("sub CLONE_SKIP { 1 }", "CLONE_SKIP method"),
        // Import/unimport
        ("sub import { shift; print @_ }", "import method"),
        (
            "sub unimport { shift; delete $^H{feature} }",
            "unimport method",
        ),
        // Tying methods
        ("sub TIESCALAR { bless {}, shift }", "TIESCALAR method"),
        ("sub TIEARRAY { bless [], shift }", "TIEARRAY method"),
        ("sub TIEHASH { bless {}, shift }", "TIEHASH method"),
        (
            "sub TIEHANDLE { bless \\*HANDLE, shift }",
            "TIEHANDLE method",
        ),
        // Tie interface methods
        ("sub FETCH { shift->{value} }", "FETCH method"),
        ("sub STORE { shift->{value} = shift }", "STORE method"),
        ("sub DELETE { delete shift->{shift()} }", "DELETE method"),
        ("sub EXISTS { exists shift->{shift()} }", "EXISTS method"),
        ("sub FIRSTKEY { ... }", "FIRSTKEY method"),
        ("sub NEXTKEY { ... }", "NEXTKEY method"),
        ("sub SCALAR { shift->length }", "SCALAR method"),
        // Overloading related
        ("sub OVERLOAD { ... }", "OVERLOAD method"),
        ("sub FALLBACK { 1 }", "FALLBACK for overloading"),
        // Write formats
        ("write", "write with default format"),
        ("write STDOUT", "write with filehandle"),
        ("write HANDLE", "write with custom handle"),
        (
            "select((select(FH), $~ = 'MYFORMAT')[0])",
            "changing format",
        ),
        // Format variables
        ("$~ = 'MYFORMAT'", "set format name"),
        ("$^ = 'MYFORMAT_TOP'", "set format top name"),
        ("$% = 1", "set page number"),
        ("$= = 60", "set page length"),
        ("$- = 10", "set lines remaining"),
        // Complex format usage
        (
            r#"local $~ = "DYNAMIC";
format DYNAMIC =
@<<<<<<<<<<<<
$_
.
write"#,
            "dynamic format selection",
        ),
        // Format with expressions
        (
            r#"format EXPR =
@###.## @<<<<<<< @||||||||
$price * 1.1, $item, $category
.
"#,
            "format with expressions",
        ),
        // Picture lines
        (
            r#"format PICTURE =
@<<<@>>>@|||
$a, $b, $c
@###.##
$total
.
"#,
            "format with picture lines",
        ),
    ]
}
