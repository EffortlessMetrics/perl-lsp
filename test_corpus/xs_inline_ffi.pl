#!/usr/bin/env perl
# Test: XS, Inline::C, and FFI integration
# Impact: Many CPAN modules use XS; parser must handle bootstrap/loader patterns

# Standard XS module loading
package Math::FastCalc;
use strict;
use warnings;
our $VERSION = '1.42';

use XSLoader;
XSLoader::load(__PACKAGE__, $VERSION);

# Bootstrap alternative
package String::Util;
our $VERSION = '0.01';
require DynaLoader;
our @ISA = qw(DynaLoader);
bootstrap String::Util $VERSION;

# XS function stubs (parser should recognize these)
sub new {
    my $class = shift;
    return bless {}, $class;
}

# XS constants
use constant {
    XS_VERSION => $VERSION,
    HAS_XS     => 1,
};

# Inline::C example
package Inline::Example;
use Inline C => <<'END_C';
    #include <math.h>
    
    SV* calculate_distance(SV* x1, SV* y1, SV* x2, SV* y2) {
        double dx = SvNV(x2) - SvNV(x1);
        double dy = SvNV(y2) - SvNV(y1);
        return newSVnv(sqrt(dx*dx + dy*dy));
    }
    
    void process_array(AV* input) {
        int len = av_len(input) + 1;
        for (int i = 0; i < len; i++) {
            SV** elem = av_fetch(input, i, 0);
            if (elem) sv_setnv(*elem, SvNV(*elem) * 2);
        }
    }
END_C

# Inline with config
use Inline C => Config =>
    LIBS => '-lm',
    INC  => '-I/usr/local/include',
    CC   => 'gcc',
    OPTIMIZE => '-O3';

use Inline C => q{
    int fast_add(int a, int b) {
        return a + b;
    }
};

# FFI::Platypus example
package FFI::Example;
use FFI::Platypus;

my $ffi = FFI::Platypus->new(api => 1);
$ffi->lib(undef);  # Use current process

# Attach libc functions
$ffi->attach(puts => ['string'] => 'int');
$ffi->attach(strlen => ['string'] => 'size_t');
$ffi->attach(malloc => ['size_t'] => 'opaque');
$ffi->attach(free => ['opaque'] => 'void');

# Custom type
$ffi->type('int[10]' => 'int_array');

# Closure
my $closure = $ffi->closure(sub {
    my ($x, $y) = @_;
    return $x + $y;
});

$ffi->attach([qsort => 'my_sort'] => ['opaque', 'size_t', 'size_t', 'opaque'] => 'void');

# Alien::Build integration
package Alien::Example;
use base qw(Alien::Base);
our $VERSION = '0.01';

# XS typemap references
package Typemap::Example;

# These would be in a .xs file normally
my $typemap = <<'TYPEMAP';
TYPEMAP
MyClass *    T_PTROBJ
const char * T_PV

INPUT
T_PTROBJ
    if (sv_derived_from($arg, \"${ntype}\")) {
        IV tmp = SvIV((SV*)SvRV($arg));
        $var = INT2PTR($type, tmp);
    }
TYPEMAP

# XS AUTOLOAD
package XS::AutoLoad;
use AutoLoader;
our @ISA = qw(AutoLoader);
our $AUTOLOAD;

sub AUTOLOAD {
    my $constname = $AUTOLOAD;
    $constname =~ s/.*:://;
    my $val = constant($constname);
    *$AUTOLOAD = sub { $val };
    goto &$AUTOLOAD;
}

# Module::Build::XSUtil patterns
package Build::WithXS;
use Module::Build::XSUtil;

my $builder = Module::Build::XSUtil->new(
    module_name => 'My::XS::Module',
    xs_files    => {
        'src/module.xs' => 'lib/My/XS/Module.xs',
    },
    include_dirs => ['.'],
    c_source     => 'src',
);

__END__
Parser assertions:
1. XSLoader::load and bootstrap patterns recognized
2. Document symbols show Perl parts; C blocks don't crash parser
3. Module resolver handles XS loading without errors
4. Folding ranges work around Inline C blocks
5. No syntax errors from C code in heredocs/strings