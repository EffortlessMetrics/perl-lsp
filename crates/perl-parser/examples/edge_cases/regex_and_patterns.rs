//! Edge case tests for regex and pattern matching edge cases

pub fn get_tests() -> Vec<(&'static str, &'static str)> {
    vec![
        // Basic regex
        ("/foo/", "basic regex"),
        ("m/foo/", "match operator"),
        ("m{foo}", "match with braces"),
        ("m<foo>", "match with angles"),
        ("m[foo]", "match with brackets"),
        ("m(foo)", "match with parens"),
        ("m!foo!", "match with exclamation"),
        ("m#foo#", "match with hash"),
        ("m'foo'", "match with single quotes"),
        ("m\"foo\"", "match with double quotes"),
        ("m|foo|", "match with pipes"),
        ("m,foo,", "match with commas"),
        
        // Substitution
        ("s/foo/bar/", "basic substitution"),
        ("s{foo}{bar}", "substitution with braces"),
        ("s<foo><bar>", "substitution with angles"),
        ("s[foo][bar]", "substitution with brackets"),
        ("s(foo)(bar)", "substitution with parens"),
        ("s!foo!bar!", "substitution with exclamation"),
        ("s#foo#bar#", "substitution with hash"),
        ("s'foo'bar'", "substitution with single quotes"),
        ("s|foo|bar|", "substitution with pipes"),
        
        // Mixed delimiters
        ("s{foo}[bar]", "substitution mixed delimiters"),
        ("s<foo>(bar)", "substitution mixed angles parens"),
        ("s[foo]/bar/", "substitution mixed brackets slashes"),
        
        // Transliteration
        ("tr/a-z/A-Z/", "basic transliteration"),
        ("tr{a-z}{A-Z}", "transliteration with braces"),
        ("tr<a-z><A-Z>", "transliteration with angles"),
        ("tr[a-z][A-Z]", "transliteration with brackets"),
        ("tr(a-z)(A-Z)", "transliteration with parens"),
        ("y/a-z/A-Z/", "y/// transliteration"),
        
        // Regex modifiers
        ("/foo/i", "case insensitive"),
        ("/foo/m", "multiline"),
        ("/foo/s", "single line"),
        ("/foo/x", "extended"),
        ("/foo/g", "global"),
        ("/foo/o", "compile once"),
        ("/foo/e", "eval in substitution"),
        ("/foo/ee", "double eval"),
        ("/foo/eee", "triple eval"),
        ("/foo/r", "non-destructive"),
        ("/foo/a", "ASCII"),
        ("/foo/u", "Unicode"),
        ("/foo/d", "Default"),
        ("/foo/l", "Locale"),
        ("/foo/c", "continue"),
        ("/foo/p", "preserve"),
        ("/foo/n", "non-capturing"),
        
        // Combined modifiers
        ("/foo/igmsx", "multiple modifiers"),
        ("s/foo/bar/igmsx", "substitution with modifiers"),
        ("tr/a-z/A-Z/cds", "transliteration modifiers"),
        
        // Regex with interpolation
        ("/$var/", "interpolated variable"),
        ("/foo$var/", "partial interpolation"),
        ("/${var}/", "braced interpolation"),
        ("/@{[expr]}/", "expression interpolation"),
        ("/\\Q$var\\E/", "quoted interpolation"),
        
        // Named captures
        ("/(?<name>\\w+)/", "named capture"),
        ("/(?'name'\\w+)/", "named capture single quote"),
        ("/(?P<name>\\w+)/", "Python-style named capture"),
        ("$+{name}", "named capture hash"),
        ("$-{name}[0]", "named capture array"),
        
        // Backreferences
        ("/(\\w+) \\1/", "numbered backreference"),
        ("/(?<word>\\w+) \\k<word>/", "named backreference"),
        ("/(?'word'\\w+) \\k'word'/", "named backreference single"),
        ("/(?<word>\\w+) \\g{word}/", "g-style backreference"),
        ("/(\\w+) \\g1/", "g-style numbered"),
        ("/(\\w+) \\g{1}/", "g-style numbered braced"),
        ("/(\\w+) \\g{-1}/", "relative backreference"),
        
        // Non-capturing groups
        ("/(?:foo|bar)/", "non-capturing group"),
        ("/(?i:foo)/", "inline modifier group"),
        ("/(?-i:foo)/", "negative inline modifier"),
        ("/(?i-m:foo)/", "mixed inline modifiers"),
        
        // Lookahead/lookbehind
        ("/foo(?=bar)/", "positive lookahead"),
        ("/foo(?!bar)/", "negative lookahead"),
        ("/(?<=foo)bar/", "positive lookbehind"),
        ("/(?<!foo)bar/", "negative lookbehind"),
        
        // Atomic groups
        ("/(?>foo|foobar)/", "atomic group"),
        
        // Conditional patterns
        ("/(?(1)yes|no)/", "conditional pattern"),
        ("/(?(<name>)yes|no)/", "conditional with named"),
        ("/(?(condition)yes|no)/", "conditional with condition"),
        ("/(?(?=foo)yes|no)/", "conditional with lookahead"),
        
        // Recursive patterns
        ("/(?R)/", "recursive pattern"),
        ("/(?1)/", "subroutine call"),
        ("/(?&name)/", "named subroutine call"),
        ("/(?P>name)/", "Python-style subroutine"),
        
        // Comments in regex
        ("/foo(?#comment)bar/", "inline comment"),
        ("/foo  # comment\n  bar/x", "extended comment"),
        
        // Special escape sequences
        ("/\\A/", "beginning of string"),
        ("/\\Z/", "end of string before newline"),
        ("/\\z/", "absolute end of string"),
        ("/\\b/", "word boundary"),
        ("/\\B/", "non-word boundary"),
        ("/\\G/", "end of previous match"),
        ("/\\K/", "keep left"),
        
        // Character classes
        ("/[abc]/", "character class"),
        ("/[^abc]/", "negated character class"),
        ("/[a-z]/", "character range"),
        ("/[a-zA-Z0-9]/", "multiple ranges"),
        ("/[\\w\\s]/", "escape in class"),
        ("/[\\]\\[]/", "escaped brackets in class"),
        ("/[^]]/", "negated ] in class"),
        ("/[\\-]/", "escaped dash in class"),
        ("/[a-z-]/", "dash at end of class"),
        ("/[-a-z]/", "dash at start of class"),
        
        // POSIX character classes
        ("/[[:alpha:]]/", "POSIX alpha"),
        ("/[[:digit:]]/", "POSIX digit"),
        ("/[[:alnum:]]/", "POSIX alnum"),
        ("/[[:space:]]/", "POSIX space"),
        ("/[[:punct:]]/", "POSIX punct"),
        ("/[[:xdigit:]]/", "POSIX xdigit"),
        ("/[[:print:]]/", "POSIX print"),
        ("/[[:graph:]]/", "POSIX graph"),
        ("/[[:upper:]]/", "POSIX upper"),
        ("/[[:lower:]]/", "POSIX lower"),
        ("/[[:cntrl:]]/", "POSIX control"),
        ("/[[:blank:]]/", "POSIX blank"),
        
        // Unicode properties
        ("/\\p{L}/", "Unicode letter"),
        ("/\\p{Letter}/", "Unicode letter long"),
        ("/\\P{L}/", "Unicode non-letter"),
        ("/\\p{Ll}/", "Unicode lowercase"),
        ("/\\p{Lu}/", "Unicode uppercase"),
        ("/\\p{N}/", "Unicode number"),
        ("/\\p{Script=Latin}/", "Unicode script"),
        ("/\\p{Block=BasicLatin}/", "Unicode block"),
        ("/\\p{InGreek}/", "Unicode in block"),
        
        // Quantifiers
        ("/a*/", "zero or more"),
        ("/a+/", "one or more"),
        ("/a?/", "zero or one"),
        ("/a{3}/", "exactly n"),
        ("/a{3,}/", "n or more"),
        ("/a{3,5}/", "between n and m"),
        ("/a{,5}/", "up to m"),
        
        // Non-greedy quantifiers
        ("/a*?/", "non-greedy zero or more"),
        ("/a+?/", "non-greedy one or more"),
        ("/a??/", "non-greedy zero or one"),
        ("/a{3,5}?/", "non-greedy range"),
        
        // Possessive quantifiers
        ("/a*+/", "possessive zero or more"),
        ("/a++/", "possessive one or more"),
        ("/a?+/", "possessive zero or one"),
        ("/a{3,5}+/", "possessive range"),
        
        // Special regex variables
        ("$&", "entire match"),
        ("$`", "prematch"),
        ("$'", "postmatch"),
        ("$+", "last capture"),
        ("$^N", "most recent capture"),
        ("$1", "numbered capture"),
        ("@-", "start positions"),
        ("@+", "end positions"),
        ("%+", "named captures hash"),
        ("%-", "named captures array hash"),
        ("$^R", "regex eval result"),
        ("${^MATCH}", "entire match variable"),
        ("${^PREMATCH}", "prematch variable"),
        ("${^POSTMATCH}", "postmatch variable"),
        
        // Regex in different contexts
        ("if (/foo/) { }", "regex in if"),
        ("while (/foo/g) { }", "regex in while"),
        ("split /,/, $string", "regex in split"),
        ("grep /foo/, @list", "regex in grep"),
        ("map s/foo/bar/, @list", "substitution in map"),
        
        // qr// operator
        ("qr/foo/", "qr basic"),
        ("qr{foo}", "qr with braces"),
        ("qr/foo/i", "qr with modifiers"),
        ("$re = qr/foo/", "qr assignment"),
        ("/$re/", "qr interpolation"),
        
        // Code in regex
        ("/(?{ code })/", "code in regex"),
        ("/(??{ code })/", "postponed code"),
        ("/(*{ code })/", "optimized code"),
        
        // Verbs
        ("/(*PRUNE)/", "PRUNE verb"),
        ("/(*SKIP)/", "SKIP verb"),
        ("/(*MARK:name)/", "MARK verb"),
        ("/(*THEN)/", "THEN verb"),
        ("/(*COMMIT)/", "COMMIT verb"),
        ("/(*FAIL)/", "FAIL verb"),
        ("/(*ACCEPT)/", "ACCEPT verb"),
        
        // Branch reset
        ("/(?|(foo)|(bar))/", "branch reset"),
        
        // Captures in list context
        ("($a, $b) = /(.)(.)./", "regex captures to list"),
        ("@captures = /(.)(.)/g", "global captures to array"),
        
        // Special cases
        ("//", "empty regex"),
        ("s///", "empty substitution"),
        ("m??", "match once"),
        ("s???", "substitute once"),
        ("/\\c[/", "control character"),
        ("/\\x{263A}/", "hex character"),
        ("/\\o{123}/", "octal character"),
        ("/\\N{SNOWMAN}/", "named character"),
        
        // Regex with variables
        ("/$foo/", "variable interpolation"),
        ("/${foo}/", "braced variable"),
        ("/$foo{bar}/", "hash in regex"),
        ("/$foo[0]/", "array in regex"),
        
        // Multiline patterns
        ("/^foo$/m", "multiline anchors"),
        ("/foo.bar/s", "dot matches newline"),
        
        // Extended regex
        ("/foo   # this is foo\n   bar  # this is bar/x", "extended with comments"),
        ("/foo\\ bar/x", "extended with escaped space"),
        
        // Case folding
        ("/\\F/", "case fold"),
        ("/\\l/", "lowercase next"),
        ("/\\u/", "uppercase next"),
        ("/\\L/", "lowercase till \\E"),
        ("/\\U/", "uppercase till \\E"),
        ("/\\Q/", "quote till \\E"),
        ("/\\E/", "end modification"),
    ]
}