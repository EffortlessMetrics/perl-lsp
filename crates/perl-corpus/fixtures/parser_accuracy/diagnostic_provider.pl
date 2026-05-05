# provider-index-support:start
package Accuracy::Diagnostics::Exporter;
use Exporter 'import';
our @EXPORT_OK = qw(imported_known);
sub imported_known { 1 }
# provider-index-support:end

package Accuracy::Diagnostics::UseCases;
use strict;
use warnings;
use Accuracy::Diagnostics::Exporter qw(imported_known);

sub ordinary_undefined_variable {
    print $ordinary_missing;
}

sub ordinary_undefined_bareword {
    print truly_missing_symbol;
}

sub eval_string_boundary {
    eval "sub eval_generated_symbol { 1 }";
    print eval_generated_symbol;
}

sub dynamic_require_boundary {
    my $module = "Accuracy::Diagnostics::Dynamic";
    require $module;
    $module->import(qw(dynamic_imported_symbol generated_accessor));
    print dynamic_imported_symbol;
    print generated_accessor;
}

our $AUTOLOAD;
sub AUTOLOAD {
    our $AUTOLOAD;
    return $AUTOLOAD;
}

sub autoload_boundary {
    print $AUTOLOAD;
}

sub known_imported_symbol {
    print imported_known;
}

our $package_local_symbol;

sub package_local_symbol_case {
    print $package_local_symbol;
}

1;
