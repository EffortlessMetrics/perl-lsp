//! Edge case tests for operator overloading and pragmas

pub fn get_tests() -> Vec<(&'static str, &'static str)> {
    vec![
        // Basic operator overloading
        (r#"use overload '+' => sub { $_[0]{val} + $_[1] }"#, "overload addition"),
        (r#"use overload '-' => sub { $_[0]{val} - $_[1] }"#, "overload subtraction"),
        (r#"use overload '*' => sub { $_[0]{val} * $_[1] }"#, "overload multiplication"),
        (r#"use overload '/' => sub { $_[0]{val} / $_[1] }"#, "overload division"),
        (r#"use overload '%' => sub { $_[0]{val} % $_[1] }"#, "overload modulo"),
        (r#"use overload '**' => sub { $_[0]{val} ** $_[1] }"#, "overload exponentiation"),
        
        // Comparison overloading
        (r#"use overload '<=>' => sub { $_[0]{val} <=> $_[1] }"#, "overload spaceship"),
        (r#"use overload 'cmp' => sub { $_[0]{str} cmp $_[1] }"#, "overload string compare"),
        (r#"use overload '<' => sub { $_[0]{val} < $_[1] }"#, "overload less than"),
        (r#"use overload '>' => sub { $_[0]{val} > $_[1] }"#, "overload greater than"),
        (r#"use overload '<=' => sub { $_[0]{val} <= $_[1] }"#, "overload less equal"),
        (r#"use overload '>=' => sub { $_[0]{val} >= $_[1] }"#, "overload greater equal"),
        (r#"use overload '==' => sub { $_[0]{val} == $_[1] }"#, "overload numeric equality"),
        (r#"use overload '!=' => sub { $_[0]{val} != $_[1] }"#, "overload numeric inequality"),
        (r#"use overload 'eq' => sub { $_[0]{str} eq $_[1] }"#, "overload string equality"),
        (r#"use overload 'ne' => sub { $_[0]{str} ne $_[1] }"#, "overload string inequality"),
        
        // Bitwise overloading
        (r#"use overload '&' => sub { $_[0]{val} & $_[1] }"#, "overload bitwise and"),
        (r#"use overload '|' => sub { $_[0]{val} | $_[1] }"#, "overload bitwise or"),
        (r#"use overload '^' => sub { $_[0]{val} ^ $_[1] }"#, "overload bitwise xor"),
        (r#"use overload '~' => sub { ~$_[0]{val} }"#, "overload bitwise not"),
        (r#"use overload '<<' => sub { $_[0]{val} << $_[1] }"#, "overload left shift"),
        (r#"use overload '>>' => sub { $_[0]{val} >> $_[1] }"#, "overload right shift"),
        
        // String overloading
        (r#"use overload '""' => sub { $_[0]->to_string }"#, "overload stringification"),
        (r#"use overload '0+' => sub { $_[0]->to_number }"#, "overload numification"),
        (r#"use overload 'bool' => sub { $_[0]->is_true }"#, "overload boolification"),
        (r#"use overload '!' => sub { !$_[0]->is_true }"#, "overload negation"),
        (r#"use overload 'qr' => sub { $_[0]->to_regex }"#, "overload regex conversion"),
        
        // Special overloading
        (r#"use overload '=' => sub { $_[0]->clone }"#, "overload assignment"),
        (r#"use overload 'x' => sub { $_[0]->repeat($_[1]) }"#, "overload repetition"),
        (r#"use overload '.' => sub { $_[0]->concat($_[1]) }"#, "overload concatenation"),
        (r#"use overload 'abs' => sub { abs($_[0]{val}) }"#, "overload abs"),
        (r#"use overload 'neg' => sub { bless { val => -$_[0]{val} } }"#, "overload negation"),
        (r#"use overload '++' => sub { $_[0]{val}++ }"#, "overload increment"),
        (r#"use overload '--' => sub { $_[0]{val}-- }"#, "overload decrement"),
        (r#"use overload 'atan2' => sub { atan2($_[0]{y}, $_[0]{x}) }"#, "overload atan2"),
        (r#"use overload 'cos' => sub { cos($_[0]{angle}) }"#, "overload cos"),
        (r#"use overload 'sin' => sub { sin($_[0]{angle}) }"#, "overload sin"),
        (r#"use overload 'exp' => sub { exp($_[0]{val}) }"#, "overload exp"),
        (r#"use overload 'log' => sub { log($_[0]{val}) }"#, "overload log"),
        (r#"use overload 'sqrt' => sub { sqrt($_[0]{val}) }"#, "overload sqrt"),
        (r#"use overload 'int' => sub { int($_[0]{val}) }"#, "overload int"),
        
        // Dereferencing overloading
        (r#"use overload '@{}' => sub { $_[0]{array} }"#, "overload array deref"),
        (r#"use overload '%{}' => sub { $_[0]{hash} }"#, "overload hash deref"),
        (r#"use overload '&{}' => sub { $_[0]{code} }"#, "overload code deref"),
        (r#"use overload '*{}' => sub { $_[0]{glob} }"#, "overload glob deref"),
        (r#"use overload '${}' => sub { \$_[0]{scalar} }"#, "overload scalar deref"),
        
        // Multiple overloads
        (r#"use overload
    '+' => sub { $_[0]{val} + $_[1] },
    '-' => sub { $_[0]{val} - $_[1] },
    '""' => sub { $_[0]{str} }"#, "multiple overloads"),
        
        // Fallback
        (r#"use overload '+' => sub { ... }, fallback => 1"#, "overload with fallback"),
        (r#"use overload '+' => sub { ... }, fallback => 0"#, "overload no fallback"),
        (r#"use overload '+' => sub { ... }, fallback => 'undef'"#, "overload undef fallback"),
        
        // Method names instead of subs
        (r#"use overload '+' => 'add'"#, "overload with method name"),
        (r#"use overload '-' => 'subtract', '*' => 'multiply'"#, "multiple method overloads"),
        
        // No overloading
        ("no overload", "disable all overloading"),
        (r#"no overload '+', '-', '*'"#, "disable specific overloads"),
        
        // Pragmas
        ("use strict", "strict pragma"),
        ("use warnings", "warnings pragma"),
        ("use strict 'refs'", "strict refs only"),
        ("use strict 'vars'", "strict vars only"),
        ("use strict 'subs'", "strict subs only"),
        ("use warnings 'all'", "all warnings"),
        ("use warnings FATAL => 'all'", "fatal warnings"),
        ("no warnings 'uninitialized'", "disable specific warning"),
        
        // Feature pragmas
        ("use feature 'say'", "enable say feature"),
        ("use feature ':5.10'", "enable 5.10 features"),
        ("use feature qw(say state switch)", "multiple features"),
        ("no feature 'switch'", "disable feature"),
        
        // Encoding pragmas
        ("use encoding 'utf8'", "utf8 encoding pragma"),
        ("use encoding 'latin1'", "latin1 encoding pragma"),
        ("no encoding", "disable encoding pragma"),
        
        // Other pragmas
        ("use constant PI => 3.14159", "constant pragma"),
        ("use constant { FOO => 1, BAR => 2 }", "multiple constants"),
        ("use lib '/path/to/lib'", "lib pragma"),
        ("use lib qw(/path1 /path2)", "multiple lib paths"),
        ("use vars qw($foo @bar %baz)", "vars pragma"),
        ("use subs qw(foo bar baz)", "subs pragma"),
        ("use attributes", "attributes pragma"),
        ("use autodie", "autodie pragma"),
        ("use autodie ':all'", "autodie all"),
        ("use bigint", "bigint pragma"),
        ("use bignum", "bignum pragma"),
        ("use bigrat", "bigrat pragma"),
        ("use bytes", "bytes pragma"),
        ("use charnames ':full'", "charnames pragma"),
        ("use diagnostics", "diagnostics pragma"),
        ("use integer", "integer pragma"),
        ("use locale", "locale pragma"),
        ("use open ':utf8'", "open pragma"),
        ("use sort '_quicksort'", "sort pragma"),
        ("use threads", "threads pragma"),
        ("use utf8", "utf8 pragma"),
        
        // Version pragmas
        ("use 5.010", "version pragma"),
        ("use v5.10.0", "v-string version pragma"),
        ("use 5.010_001", "underscored version"),
        ("require 5.010", "require version"),
        
        // Import lists
        ("use Module ()", "empty import list"),
        ("use Module qw()", "empty qw import"),
        ("use Module qw(foo bar baz)", "qw import list"),
        ("use Module 'foo', 'bar'", "list import"),
        ("use Module foo => 'bar'", "import with arrow"),
        
        // Complex pragma usage
        (r#"use overload
    '+' => sub { $_[0]->add($_[1]) },
    '-' => sub { $_[0]->subtract($_[1]) },
    '*' => sub { $_[0]->multiply($_[1]) },
    '/' => sub { $_[0]->divide($_[1]) },
    '""' => sub { $_[0]->stringify },
    '0+' => sub { $_[0]->numify },
    fallback => 1"#, "complex overload declaration"),
    ]
}