//! Edge case tests for v-strings and version numbers

pub fn get_tests() -> Vec<(&'static str, &'static str)> {
    vec![
        // Basic v-strings
        ("v1.2.3", "basic v-string"),
        ("v5.10.0", "perl version v-string"),
        ("v65.66.67", "v-string with ASCII values"),
        ("v0", "single component v-string"),
        ("v999.999.999", "large v-string"),
        ("v1.2.3.4.5.6.7.8.9", "many component v-string"),
        
        // Unicode v-strings
        ("v9786.9787.9788", "unicode v-string (smileys)"),
        ("v2665.2666", "unicode v-string (chess pieces)"),
        ("v945.946.947", "unicode v-string (greek letters)"),
        
        // V-strings in different contexts
        ("$version = v1.2.3", "v-string assignment"),
        ("use v5.10.0", "v-string in use"),
        ("require v5.10.0", "v-string in require"),
        ("if ($] >= v5.10.0) { }", "v-string comparison"),
        
        // Old-style version numbers
        ("5.010", "decimal version"),
        ("5.010_001", "decimal with underscore"),
        ("5.10.0", "dotted decimal version"),
        ("1.234567890123456", "long decimal version"),
        
        // Version comparisons
        ("$version == 1.23", "numeric version compare"),
        ("$version eq 'v1.2.3'", "string version compare"),
        ("$version >= 5.010", "version greater equal"),
        ("$version < v5.10", "version less than"),
        ("$version cmp 'v1.2.3'", "version string compare"),
        
        // Package versions
        ("package Foo 1.23", "package with decimal version"),
        ("package Foo v1.2.3", "package with v-string"),
        ("package Foo 1.23_45", "package with underscore version"),
        ("package Foo 1.2.3", "package with dotted decimal"),
        ("package Foo::Bar 2.0", "qualified package with version"),
        
        // Module versions
        ("use Module 1.23", "use with version"),
        ("use Module v1.2.3", "use with v-string"),
        ("use Module 1.23 qw(foo)", "use with version and import"),
        ("use Module v1.2.3 ()", "use v-string empty import"),
        ("require Module 1.23", "require with version check"),
        
        // Version methods
        ("Module->VERSION", "VERSION method"),
        ("Module->VERSION(1.23)", "VERSION with required"),
        ("Module->VERSION(v1.2.3)", "VERSION with v-string required"),
        ("$obj->VERSION", "object VERSION"),
        
        // Special version variables
        ("$VERSION = '1.23'", "string VERSION assignment"),
        ("$VERSION = 1.23", "numeric VERSION assignment"),
        ("$VERSION = v1.2.3", "v-string VERSION assignment"),
        ("our $VERSION = '1.23'", "our VERSION"),
        ("use version; our $VERSION = version->new('1.23')", "version object"),
        
        // Version in different contexts
        ("$Foo::VERSION", "package VERSION variable"),
        ("$Foo::Bar::Baz::VERSION", "deep package VERSION"),
        ("${Foo::VERSION}", "braced VERSION"),
        ("${'Foo::VERSION'}", "symbolic VERSION"),
        
        // Eval version
        ("$VERSION = eval $VERSION", "eval VERSION pattern"),
        (r#"$VERSION = "1.23_45"; $VERSION = eval $VERSION"#, "eval underscore VERSION"),
        
        // Version declarations
        (r#"our $VERSION = '1.23';
our $AUTHORITY = 'cpan:AUTHOR';"#, "VERSION with AUTHORITY"),
        
        // Complex version patterns
        ("use Module 1.23 if $] >= 5.010", "conditional version use"),
        ("BEGIN { require Module; Module->VERSION(1.23) }", "BEGIN version check"),
        
        // Version extraction patterns
        (r#"our $VERSION = (qw$Revision: 1.23 $)[1]"#, "RCS revision extraction"),
        (r#"our $VERSION = sprintf "%d.%03d", q$Revision: 1.23 $ =~ /(\d+)\.(\d+)/g"#, "complex version extraction"),
        
        // CPAN style versions
        ("our $VERSION = '0.01_01'", "developer release version"),
        ("our $VERSION = '1.234_567_890'", "multi underscore version"),
        
        // Git versions
        (r#"our $VERSION = '1.23-TRIAL'"#, "trial version"),
        (r#"our $VERSION = '1.23-dev'"#, "dev version"),
        
        // Version with encoding
        ("use 5.010;", "perl version requirement"),
        ("use 5.10.0;", "dotted perl version"),
        ("use 5.010_000;", "underscore perl version"),
        ("use 5.010001;", "combined perl version"),
        
        // Version in interpolation
        (r#"print "Version $VERSION\n""#, "interpolated VERSION"),
        (r#"print "Perl $]\n""#, "interpolated perl version"),
        (r#"print "Perl $^V\n""#, "interpolated v-string perl version"),
        
        // Binary/octal in version context
        ("$VERSION = 0b1010", "binary in version"),
        ("$VERSION = 0755", "octal in version"),
        ("$VERSION = 0x1F", "hex in version"),
        
        // Version with math
        ("$VERSION = 1.23 + 0.01", "version arithmetic"),
        ("$VERSION = int(1.234 * 1000) / 1000", "version rounding"),
        
        // Tied/magic versions
        ("tie $VERSION, 'Version::Class'", "tied VERSION"),
        
        // Version checking idioms
        ("die 'Too old' if $] < 5.010", "perl version check"),
        ("die 'Too old' unless $^V ge v5.10.0", "v-string version check"),
        ("die 'Wrong version' unless $Module::VERSION eq '1.23'", "module version check"),
        
        // Version declaration styles
        ("our $VERSION; $VERSION = '1.23'", "separate VERSION declaration"),
        ("our ($VERSION) = '1.23'", "list VERSION assignment"),
        (r#"our ($VERSION) = (q$Revision: 1.23 $ =~ /(\d+(?:\.\d+)+)/)"#, "extracted VERSION"),
        
        // Multi-line version
        (r#"our $VERSION = 
    '1.23'"#, "multiline VERSION"),
        
        // Version in BEGIN
        ("BEGIN { our $VERSION = '1.23' }", "BEGIN VERSION"),
        ("BEGIN { $VERSION = '1.23' }", "BEGIN package VERSION"),
        
        // Readonly versions
        ("use Readonly; Readonly our $VERSION => '1.23'", "Readonly VERSION"),
        ("use constant VERSION => '1.23'", "constant VERSION"),
        
        // Version formats
        ("v49.46.51", "v-string as chars (1.2.3)"),
        ("100.200_300", "underscore in decimal"),
        ("v1.2_3", "underscore in v-string component"),
    ]
}