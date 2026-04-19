//! Edge case tests for indirect object syntax and method calls

pub fn get_tests() -> Vec<(&'static str, &'static str)> {
    vec![
        // Indirect object syntax
        ("new Class", "indirect object basic"),
        ("new Class()", "indirect object with parens"),
        ("new Class @args", "indirect object with args"),
        ("new Class 'arg1', 'arg2'", "indirect object with list"),
        ("new Class { foo => 'bar' }", "indirect object with hashref"),
        ("new Class->new", "indirect on method call"),
        ("print STDOUT 'hello'", "print with indirect filehandle"),
        ("print $fh 'hello'", "print with scalar filehandle"),
        ("print {$fh} 'hello'", "print with braced filehandle"),
        ("printf STDERR '%s', $msg", "printf indirect"),
        ("say STDOUT 'hello'", "say with indirect"),
        // Method calls with special cases
        ("$obj->method", "basic method call"),
        ("$obj->method()", "method call with parens"),
        ("$obj->method(@args)", "method call with args"),
        ("$obj->$method", "dynamic method call"),
        ("$obj->$method()", "dynamic method with parens"),
        ("$obj->${method}", "braced dynamic method"),
        ("$obj->${\\method}", "reference dynamic method"),
        ("$obj->{'key'}", "hash element via method syntax"),
        ("$obj->[0]", "array element via method syntax"),
        ("$obj->@*", "postfix array deref"),
        ("$obj->%*", "postfix hash deref"),
        ("$obj->$*", "postfix scalar deref"),
        ("$obj->&*", "postfix code deref"),
        ("$obj->**", "postfix glob deref"),
        // Class method calls
        ("Class->method", "class method call"),
        ("Class->method()", "class method with parens"),
        ("Class->new", "class constructor"),
        ("Class->new()", "class constructor with parens"),
        ("Class->new->method", "chained constructor"),
        ("Foo::Bar->new", "qualified class method"),
        ("::Class->method", "absolute class method"),
        // SUPER and NEXT
        ("$obj->SUPER::method", "SUPER method call"),
        ("$obj->SUPER::method()", "SUPER with parens"),
        ("SUPER->method", "SUPER class method"),
        ("$obj->NEXT::method", "NEXT method call"),
        ("NEXT->method", "NEXT class method"),
        // Can method
        ("$obj->can('method')", "can method"),
        ("Class->can('new')", "class can"),
        ("UNIVERSAL::can($obj, 'method')", "UNIVERSAL can"),
        // Isa method
        ("$obj->isa('Class')", "isa method"),
        ("$obj->isa($class)", "isa with variable"),
        ("UNIVERSAL::isa($obj, 'Class')", "UNIVERSAL isa"),
        // Does method (roles)
        ("$obj->does('Role')", "does method"),
        ("$obj->DOES('Role')", "DOES method"),
        // Version method
        ("$obj->VERSION", "VERSION method"),
        ("$obj->VERSION(1.5)", "VERSION with requirement"),
        ("Class->VERSION", "class VERSION"),
        // Import/unimport
        ("Module->import", "import method call"),
        ("Module->import('foo')", "import with args"),
        ("Module->import(qw(foo bar))", "import with qw"),
        ("Module->unimport", "unimport method"),
        // Autoload
        ("$obj->any_undefined_method", "autoloaded method"),
        ("Class->undefined_class_method", "autoloaded class method"),
        // Method calls on special variables
        ("$_->method", "method on $_"),
        ("@_->method", "method on @_ (weird but valid)"),
        // Method calls on references
        ("$ref->()->method", "method on code deref"),
        ("$ref->{key}->method", "method on hash element"),
        ("$ref->[0]->method", "method on array element"),
        // Complex method chains
        ("$obj->foo->bar->baz", "method chain"),
        ("$obj->foo()->{bar}->[0]->baz", "complex chain"),
        ("$obj->foo->$bar->baz", "chain with dynamic"),
        // Method calls with blocks
        ("$obj->map { $_ * 2 }", "method with block"),
        ("$obj->grep { $_ > 10 }", "grep method with block"),
        ("$obj->sort { $a <=> $b }", "sort method with block"),
        // Indirect object with blocks
        (
            "print { select_fh() } 'hello'",
            "print with block filehandle",
        ),
        ("printf { $fh } '%s', $text", "printf with block filehandle"),
        // Special indirect syntax
        ("do File 'test.pl'", "do with indirect file"),
        ("require Module", "require indirect"),
        ("use Module", "use indirect"),
        // Method on literal
        ("'string'->length", "method on string literal"),
        ("123->abs", "method on number literal"),
        ("[1,2,3]->$method", "method on arrayref literal"),
        ("{a=>1}->keys", "method on hashref literal"),
        ("(sub { 42 })->()", "call on anonymous sub"),
        // Method with prototypes
        ("$obj->method($)", "method with prototype hint"),
        ("Class->new(\\@)", "constructor with prototype hint"),
        // Destructuring in method calls
        (
            "my ($x, $y) = $obj->get_coords",
            "destructure method return",
        ),
        ("my @results = $obj->method", "array context method"),
        ("my %hash = $obj->to_hash", "hash context method"),
        // Goto method
        ("goto &{$obj->can('method')}", "goto method"),
        ("goto $obj->can('method')", "goto can result"),
        // Blessed references
        ("bless {}, 'Class'", "bless with class"),
        ("bless {}", "bless into current package"),
        ("bless [], $class", "bless arrayref"),
        ("bless \\$scalar, 'Class'", "bless scalarref"),
        ("bless sub {}, 'Class'", "bless coderef"),
        // Ref and reftype
        ("ref $obj", "ref function"),
        ("ref $obj eq 'Class'", "ref comparison"),
        ("Scalar::Util::reftype($obj)", "reftype"),
        // Tied methods
        ("tied $var", "tied function"),
        ("tied @array", "tied array"),
        ("tied %hash", "tied hash"),
        ("tied *HANDLE", "tied handle"),
        // Method modifiers (if using Moose-like)
        ("before method => sub { }", "before modifier syntax"),
        ("after method => sub { }", "after modifier syntax"),
        ("around method => sub { }", "around modifier syntax"),
        // BUILD/DEMOLISH (Moose-like)
        ("sub BUILD { }", "BUILD method"),
        ("sub DEMOLISH { }", "DEMOLISH method"),
        ("sub BUILDARGS { }", "BUILDARGS method"),
    ]
}
