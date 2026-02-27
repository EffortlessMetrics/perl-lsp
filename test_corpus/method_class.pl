#!/usr/bin/env perl
# Test: Comprehensive Method and Class Declarations
# Impact: Ensures parser handles modern OOP syntax
# NodeKinds: Method, Class

use strict;
use warnings;
use feature 'class';
no warnings 'experimental::class';

# Basic class declaration
class Point {
    field $x :param = 0;
    field $y :param = 0;
    
    method new($class: $x = 0, $y = 0) {
        return bless { x => $x, y => $y }, $class;
    }
    
    method x() { return $x; }
    method y() { return $y; }
    
    method move($dx, $dy) {
        $x += $dx;
        $y += $dy;
    }
    
    method distance($other) {
        return sqrt(($other->x - $x)**2 + ($other->y - $y)**2);
    }
}

# Class with inheritance
class Point3D :isa(Point) {
    field $z :param = 0;
    
    method z() { return $z; }
    
    method move3d($dx, $dy, $dz) {
        $self->move($dx, $dy);
        $z += $dz;
    }
    
    method distance3d($other) {
        my $dx = $other->x - $x;
        my $dy = $other->y - $y;
        my $dz = $other->z - $z;
        return sqrt($dx**2 + $dy**2 + $dz**2);
    }
}

# Class with multiple roles/adapt (if supported)
class Drawable {
    method draw() {
        print "Drawing object\n";
    }
}

class Colored {
    field $color :param = 'black';
    
    method color() { return $color; }
    method set_color($new_color) { $color = $new_color; }
}

class ColoredPoint :isa(Point) :does(Drawable) :does(Colored) {
    method draw() {
        print "Drawing point at (${$self->x}, ${$self->y}) in color $color\n";
    }
}

# Class with private fields and methods
class Counter {
    field $count :reader = 0;
    field $max :reader = 100;
    
    method increment() {
        die "Counter overflow" if $count >= $max;
        $count++;
        return $count;
    }
    
    method reset() {
        $count = 0;
    }
    
    method _private_method() {
        return "private";
    }
}

# Class with field attributes
class AdvancedFields {
    field $id :param :reader;
    field $name :param :reader :writer;
    field $data :writer :accessor;
    field $cache :reader = {};
    
    method process() {
        $data = $data // {};
        $cache->{processed} = 1;
    }
}

# Class with method attributes
class MethodAttributes {
    method public_method() :public {
        return "public";
    }
    
    method private_method() :private {
        return "private";
    }
    
    method deprecated_method() :deprecated {
        return "deprecated";
    }
}

# Abstract class-like pattern
class Shape {
    method area() { die "Abstract method" }
    method perimeter() { die "Abstract method" }
}

class Circle :isa(Shape) {
    field $radius :param;
    
    method area() {
        return 3.14159 * $radius**2;
    }
    
    method perimeter() {
        return 2 * 3.14159 * $radius;
    }
}

class Rectangle :isa(Shape) {
    field $width :param;
    field $height :param;
    
    method area() {
        return $width * $height;
    }
    
    method perimeter() {
        return 2 * ($width + $height);
    }
}

# Class with complex initialization
class ComplexInit {
    field $config :param;
    field $computed;
    field $dependencies :reader = [];
    
    ADJUST {
        $computed = $config->{compute} // 'default';
        push @$dependencies, $config->{deps} if $config->{deps};
    }
    
    method computed() { return $computed; }
}

# Traditional Perl OO with method declarations
package TraditionalClass;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

# Traditional method (not using 'method' keyword)
sub traditional_method {
    my ($self, $arg) = @_;
    return "traditional: $arg";
}

sub regular_method {
    my ($self, $arg) = @_;
    return "regular: $arg";
}

package main;

# Method in package context
package UtilityPackage {
    sub helper {
        return "helper";
    }
    
    sub package_method {
        my ($class, $arg) = @_;
        return "package method: $arg";
    }
}

# Anonymous methods (if supported)
my $anon_method = sub ($self, $x) {
    return $x * 2;
};

# Method references and calls
my $point = Point->new(x => 1, y => 2);
my $method_ref = $point->can('move');
$point->$method_ref(3, 4);

print "All method and class tests completed\n";