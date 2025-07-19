#!/usr/bin/env perl

# Test bareword qualified names
my $x = Foo::Bar->new();
my $y = Some::Long::Class::Name->method();
my $z = Package::Name->new(arg1 => 'value1', arg2 => 'value2');

# Test with chained method calls
my $result = Config::Data->instance()->get_value('key');

# Test as part of expressions
if (Test::Module->can_do_something()) {
    print "Can do it\n";
}

# Test in list context
my @items = Data::Provider->get_all_items();