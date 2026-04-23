#!/usr/bin/env perl
# Test: Enhanced Package Production Scenarios
# Impact: Comprehensive testing of package declarations and namespace management
# NodeKinds: Package, Version, PackageBlock
# 
# This file tests the parser's ability to handle:
# 1. Complex package declarations with versions
# 2. Package blocks and lexical scoping
# 3. Nested packages and inheritance patterns
# 4. Package variables and symbol table manipulation
# 5. Cross-package interactions and dependencies
# 6. Modern vs legacy package syntax
# 7. Real-world package organization patterns
# 8. Performance and memory considerations

use strict;
use warnings;

print "=== Enhanced Package Production Tests ===\n\n";

# Test 1: Basic package declarations with versions
print "=== Basic Package Declarations ===\n";

# Simple package declaration
package SimplePackage;
our $VERSION = '1.00';
our $package_var = 'simple_value';

sub simple_sub {
    return "SimplePackage::simple_sub called";
}

package main;
print "SimplePackage version: $SimplePackage::VERSION\n";
print "SimplePackage variable: $SimplePackage::package_var\n";
print "SimplePackage sub: " . SimplePackage::simple_sub() . "\n";

# Versioned package declaration
package VersionedPackage 2.34;
our $versioned_var = 'versioned_value';

sub versioned_sub {
    return "VersionedPackage::versioned_sub called";
}

package main;
print "VersionedPackage version: $VersionedPackage::VERSION\n";
print "VersionedPackage variable: $VersionedPackage::versioned_var\n";
print "VersionedPackage sub: " . VersionedPackage::versioned_sub() . "\n";

# V-string version
package VStringPackage v1.2.3;
our $vstring_var = 'vstring_value';

sub vstring_sub {
    return "VStringPackage::vstring_sub called";
}

package main;
print "VStringPackage version: $VStringPackage::VERSION\n";
print "VStringPackage variable: $VStringPackage::vstring_var\n";
print "VStringPackage sub: " . VStringPackage::vstring_sub() . "\n\n";

# Test 2: Package blocks and lexical scoping
print "=== Package Blocks and Lexical Scoping ===\n";

# Package block with version
package BlockPackage 1.50 {
    our $block_var = 'block_value';
    my $lexical_var = 'lexical_value';
    
    sub block_sub {
        return "BlockPackage::block_sub called";
    }
    
    sub get_lexical {
        return $lexical_var;
    }
}

package main;
print "BlockPackage version: $BlockPackage::VERSION\n";
print "BlockPackage variable: $BlockPackage::block_var\n";
print "BlockPackage sub: " . BlockPackage::block_sub() . "\n";
print "BlockPackage lexical: " . BlockPackage::get_lexical() . "\n";

# Multiple package blocks in sequence
package FirstBlock 0.01 {
    our $first_var = 'first_value';
    sub first_sub { return "first"; }
}

package SecondBlock 0.02 {
    our $second_var = 'second_value';
    sub second_sub { return "second"; }
}

package main;
print "FirstBlock: " . FirstBlock::first_sub() . "\n";
print "SecondBlock: " . SecondBlock::second_sub() . "\n\n";

# Test 3: Nested packages and inheritance
print "=== Nested Packages and Inheritance ===\n";

# Nested package structure
package Outer::Package {
    our $VERSION = '1.00';
    our $outer_var = 'outer_value';
    
    sub outer_method {
        return "Outer::Package::outer_method";
    }
    
    # Nested package
    package Outer::Package::Inner {
        our $inner_var = 'inner_value';
        
        sub inner_method {
            return "Outer::Package::Inner::inner_method";
        }
        
        sub access_outer {
            return $Outer::Package::outer_var;
        }
    }
    
    # Deeply nested package
    package Outer::Package::Inner::Deeper {
        sub deeper_method {
            return "Outer::Package::Inner::Deeper::deeper_method";
        }
        
        sub access_all_levels {
            return join(' | ', 
                $Outer::Package::outer_var,
                $Outer::Package::Inner::inner_var,
                'deeper'
            );
        }
    }
}

package main;
print "Outer method: " . Outer::Package::outer_method() . "\n";
print "Inner method: " . Outer::Package::Inner::inner_method() . "\n";
print "Inner accessing outer: " . Outer::Package::Inner::access_outer() . "\n";
print "Deeper method: " . Outer::Package::Inner::Deeper::deeper_method() . "\n";
print "Deeper accessing all: " . Outer::Package::Inner::Deeper::access_all_levels() . "\n\n";

# Test 4: Package variables and symbol table manipulation
print "=== Symbol Table Manipulation ===\n";

package SymbolTableTest {
    our $public_var = 'public';
    my $private_var = 'private';
    
    sub get_private {
        return $private_var;
    }
    
    sub set_private {
        my ($value) = @_;
        $private_var = $value;
    }
}

# Access symbol table
package main;
print "Public var: $SymbolTableTest::public_var\n";
print "Private var via accessor: " . SymbolTableTest::get_private() . "\n";

# Manipulate symbol table
no strict 'refs';
*SymbolTableTest::new_sub = sub {
    return "Dynamically created subroutine";
};

*SymbolTableTest::aliased_var = \$SymbolTableTest::public_var;
use strict 'refs';

print "Dynamic sub: " . SymbolTableTest::new_sub() . "\n";
print "Aliased var: $SymbolTableTest::aliased_var\n";

# Modify aliased variable
$SymbolTableTest::aliased_var = 'modified';
print "Original var after alias modification: $SymbolTableTest::public_var\n\n";

# Test 5: Cross-package interactions and dependencies
print "=== Cross-Package Interactions ===\n";

# Base package
package BasePackage {
    our $VERSION = '1.00';
    our @ISA = ();  # No inheritance initially
    
    sub new {
        my ($class, %args) = @_;
        return bless \%args, $class;
    }
    
    sub base_method {
        my ($self) = @_;
        return "BasePackage::base_method";
    }
    
    sub common_method {
        my ($self) = @_;
        return "BasePackage::common_method";
    }
}

# Derived package
package DerivedPackage {
    our $VERSION = '1.00';
    our @ISA = ('BasePackage');
    
    sub derived_method {
        my ($self) = @_;
        return "DerivedPackage::derived_method";
    }
    
    sub common_method {
        my ($self) = @_;
        return "DerivedPackage::common_method (overrides base)";
    }
}

# Utility package
package UtilityPackage {
    sub utility_function {
        my ($package_name) = @_;
        return "Utility for $package_name";
    }
    
    sub create_object {
        my ($class, $type, %args) = @_;
        if ($type eq 'base') {
            return BasePackage->new(%args);
        } elsif ($type eq 'derived') {
            return DerivedPackage->new(%args);
        }
        return undef;
    }
}

package main;

# Test inheritance
my $base_obj = BasePackage->new(name => 'base_obj');
my $derived_obj = DerivedPackage->new(name => 'derived_obj');

print "Base object method: " . $base_obj->base_method() . "\n";
print "Base object common: " . $base_obj->common_method() . "\n";
print "Derived object method: " . $derived_obj->derived_method() . "\n";
print "Derived object common: " . $derived_obj->common_method() . "\n";
print "Derived inherited method: " . $derived_obj->base_method() . "\n";

# Test utility package
print "Utility for BasePackage: " . UtilityPackage::utility_function('BasePackage') . "\n";
my $created_base = UtilityPackage::create_object('base', name => 'created_base');
my $created_derived = UtilityPackage::create_object('derived', name => 'created_derived');

print "Created base name: " . $created_base->{name} . "\n";
print "Created derived name: " . $created_derived->{name} . "\n\n";

# Test 6: Modern vs legacy package syntax
print "=== Modern vs Legacy Package Syntax ===\n";

# Modern syntax with block
package Modern::Syntax 1.00 {
    use strict;
    use warnings;
    
    our $modern_var = 'modern_value';
    
    sub modern_sub {
        return "Modern syntax subroutine";
    }
}

# Legacy syntax
package Legacy::Syntax;
our $VERSION = '2.00';
our $legacy_var = 'legacy_value';

sub legacy_sub {
    return "Legacy syntax subroutine";
}

# Old-style package separator (apostrophe)
package Old'Style'Separator;
our $VERSION = '0.50';
our $old_style_var = 'old_style_value';

sub old_style_sub {
    return "Old style separator subroutine";
}

package main;

print "Modern syntax: " . Modern::Syntax::modern_sub() . "\n";
print "Legacy syntax: " . Legacy::Syntax::legacy_sub() . "\n";
print "Old style separator: " . Old::Style::Separator::old_style_sub() . "\n";

# Verify apostrophe separator works
print "Old style via apostrophe: " . Old'Style'Separator::old_style_sub() . "\n\n";

# Test 7: Real-world package organization patterns
print "=== Real-World Package Organization ===\n";

# Pattern 1: Module with exports
package Exportable::Module {
    our $VERSION = '1.00';
    our @EXPORT = qw(exported_function exported_variable);
    our @EXPORT_OK = qw(optional_function);
    our %EXPORT_TAGS = (
        all => [qw(exported_function exported_variable optional_function)],
        basic => [qw(exported_function)]
    );
    
    our $exported_variable = 'exported_value';
    
    sub exported_function {
        return "Exported function";
    }
    
    sub optional_function {
        return "Optional function";
    }
    
    sub internal_function {
        return "Internal function";
    }
}

# Pattern 2: Configuration package
package Config::Manager {
    our $VERSION = '1.00';
    my %config = (
        debug => 0,
        verbose => 0,
        log_level => 'INFO',
        timeout => 30
    );
    
    sub get {
        my ($key) = @_;
        return $config{$key};
    }
    
    sub set {
        my ($key, $value) = @_;
        $config{$key} = $value;
    }
    
    sub get_all {
        return %config;
    }
    
    sub load_from_file {
        my ($filename) = @_;
        # Simulate loading config from file
        $config{debug} = 1;
        $config{log_level} = 'DEBUG';
    }
}

# Pattern 3: Factory package
package Object::Factory {
    our $VERSION = '1.00';
    my %registry = ();
    
    sub register {
        my ($type, $class) = @_;
        $registry{$type} = $class;
    }
    
    sub create {
        my ($type, %args) = @_;
        my $class = $registry{$type};
        return undef unless $class;
        return $class->new(%args);
    }
    
    sub list_types {
        return keys %registry;
    }
}

# Pattern 4: Singleton package
package Singleton::Manager {
    our $VERSION = '1.00';
    my $instance = undef;
    
    sub new {
        my ($class) = @_;
        return $instance if $instance;
        $instance = bless {
            created_time => time(),
            counter => 0
        }, $class;
        return $instance;
    }
    
    sub get_instance {
        return new('Singleton::Manager');
    }
    
    def increment_counter {
        my ($self) = @_;
        $self->{counter}++;
        return $self->{counter};
    }
    
    sub get_counter {
        my ($self) = @_;
        return $self->{counter};
    }
}

package main;

# Test exportable module
print "Exportable function: " . Exportable::Module::exported_function() . "\n";
print "Exportable variable: " . $Exportable::Module::exported_variable . "\n";

# Test config manager
print "Initial debug setting: " . Config::Manager::get('debug') . "\n";
Config::Manager::set('debug', 1);
print "Updated debug setting: " . Config::Manager::get('debug') . "\n";
Config::Manager::load_from_file('config.txt');
print "Debug after file load: " . Config::Manager::get('debug') . "\n";

# Test object factory
Object::Factory::register('user', 'BasePackage');
Object::Factory::register('admin', 'DerivedPackage');

my $factory_user = Object::Factory::create('user', name => 'factory_user');
my $factory_admin = Object::Factory::create('admin', name => 'factory_admin');

print "Factory created types: " . join(', ', Object::Factory::list_types()) . "\n";
print "Factory user name: " . $factory_user->{name} . "\n";
print "Factory admin name: " . $factory_admin->{name} . "\n";

# Test singleton
my $singleton1 = Singleton::Manager::get_instance();
my $singleton2 = Singleton::Manager::get_instance();

print "Same instance: " . ($singleton1 == $singleton2 ? "yes" : "no") . "\n";
print "Initial counter: " . $singleton1->get_counter() . "\n";
$singleton1->increment_counter();
$singleton2->increment_counter();
print "Counter after increments: " . $singleton1->get_counter() . "\n\n";

# Test 8: Performance and memory considerations
print "=== Performance and Memory Considerations ===\n";

# Benchmark package variable access
sub benchmark_package_access {
    my ($iterations) = @_;
    $iterations ||= 100000;
    
    # Create test package with many variables
    package Benchmark::Package {
        our $var1 = 'value1';
        our $var2 = 'value2';
        our $var3 = 'value3';
        our $var4 = 'value4';
        our $var5 = 'value5';
        
        sub get_var1 { return $var1; }
        sub get_var2 { return $var2; }
        sub get_var3 { return $var3; }
        sub get_var4 { return $var4; }
        sub get_var5 { return $var5; }
    }
    
    package main;
    
    # Benchmark direct variable access
    my $start = time();
    for (1..$iterations) {
        my $v1 = $Benchmark::Package::var1;
        my $v2 = $Benchmark::Package::var2;
        my $v3 = $Benchmark::Package::var3;
        my $v4 = $Benchmark::Package::var4;
        my $v5 = $Benchmark::Package::var5;
    }
    my $direct_time = time() - $start;
    
    # Benchmark subroutine access
    $start = time();
    for (1..$iterations) {
        my $v1 = Benchmark::Package::get_var1();
        my $v2 = Benchmark::Package::get_var2();
        my $v3 = Benchmark::Package::get_var3();
        my $v4 = Benchmark::Package::get_var4();
        my $v5 = Benchmark::Package::get_var5();
    }
    my $sub_time = time() - $start;
    
    print "Package access benchmark ($iterations iterations):\n";
    print "  Direct variable access: $direct_time seconds\n";
    print "  Subroutine access: $sub_time seconds\n";
    print "  Performance ratio: " . sprintf('%.2f', $direct_time / $sub_time) . "x\n";
}

benchmark_package_access(100000);

# Test package symbol table cleanup
sub test_symbol_cleanup {
    # Create temporary package
    package Temp::Cleanup {
        our $temp_var = 'temp_value';
        sub temp_sub { return 'temp'; }
    }
    
    package main;
    
    # Access temporary package
    print "Temp var before cleanup: $Temp::Cleanup::temp_var\n";
    
    # Clean up symbol table (in real scenario, this would be more complex)
    no strict 'refs';
    delete $Temp::Cleanup::{temp_var};
    delete $Temp::Cleanup::{temp_sub};
    use strict 'refs';
    
    print "Symbol table cleanup completed\n";
}

test_symbol_cleanup();

print "\n=== Enhanced Package Production Tests Completed ===\n";
print "This file demonstrates comprehensive package management patterns\n";
print "for production Perl applications with various organizational strategies.\n";