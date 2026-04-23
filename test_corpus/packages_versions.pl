#!/usr/bin/env perl
# Test: Versioned packages and multi-package files
# Impact: Common pattern in CPAN distributions; symbols and navigation

# Simple package
package Simple::Package;

our $VERSION = '1.00';

sub new {
    my $class = shift;
    return bless {}, $class;
}

# Versioned package (inline version)
package Versioned::Package 2.34;

sub method {
    return "version 2.34";
}

# Package with v-string version
package VString::Package v1.2.3;

sub get_version {
    return $VERSION;  # Automatically set
}

# Multiple packages in one file
package First::Module;
our $VERSION = '0.01';

sub first_sub {
    return "first";
}

package Second::Module;
our $VERSION = '0.02';

sub second_sub {
    return "second";
}

package Third::Module;
our $VERSION = '0.03';

sub third_sub {
    return "third";
}

# Package block syntax
package Block::Syntax 1.50 {
    # Lexically scoped package
    our $VERSION = '1.50';  # Redundant but common
    
    sub in_block {
        return __PACKAGE__;
    }
}

# Multiple package blocks
package First::Block 0.01 {
    sub method1 { }
}

package Second::Block 0.02 {
    sub method2 { }
}

# Nested packages (unusual but valid)
package Outer {
    sub outer_method { }
    
    package Outer::Inner {
        sub inner_method { }
        
        package Outer::Inner::Deepest {
            sub deepest_method { }
        }
    }
    
    # Back to Outer
    sub another_outer { }
}

# Package with imports
package With::Imports 3.00;
use strict;
use warnings;
use parent qw(Some::Base::Class);
use Exporter qw(import);

our @EXPORT = qw(exported_sub);
our @EXPORT_OK = qw(optional_export);
our %EXPORT_TAGS = (
    all => [@EXPORT, @EXPORT_OK],
);

sub exported_sub { }
sub optional_export { }
sub private_sub { }

# Role/Mixin packages
package Role::Example;
use Role::Tiny;

requires 'required_method';

sub provided_method {
    my $self = shift;
    return $self->required_method() * 2;
}

around 'existing_method' => sub {
    my $orig = shift;
    my $self = shift;
    return $self->$orig(@_) + 1;
};

# Package with different separators (legacy)
package Old::Style;
package Old'Style'Sub;  # Old separator syntax (')

sub old_style_method {
    return 1;
}

# Declaring version via VERSION method
package Version::Via::Method;

sub VERSION {
    return '4.56';
}

# Package inheriting with @ISA
package ISA::Based;
our @ISA = qw(Parent::Class Another::Parent);

sub child_method { }

# Package with unusual but valid names
package _Private::Package;
package Package::With::Numbers123;
package Ǜňɨȼōđℇ;  # Unicode package name

# Switch back to main
package main;

# Code in main after multiple packages
sub main_function {
    my $obj1 = First::Module->new();
    my $obj2 = Second::Module->new();
    return ($obj1, $obj2);
}

# Version checks
use Simple::Package 0.50;     # Minimum version
use Versioned::Package 2.00;  # Minimum version

# Package at end of file (no code after)
package Final::Package 99.99;

__END__
Parser assertions:
1. All package declarations appear in document symbols
2. workspace/symbol finds all packages and their methods
3. Each package's version is correctly parsed
4. Package blocks have proper scoping
5. Navigation jumps to correct package
6. Old-style ' separator recognized as ::
7. Unicode package names handled correctly