package ExportTagExpansion;

use Exporter 'import';

our @EXPORT = qw(defaulted);
our @EXPORT_OK = qw(optional);
our %EXPORT_TAGS = (all => [qw(defaulted optional)]);

sub defaulted {
    return 1;
}

sub optional {
    return 2;
}
