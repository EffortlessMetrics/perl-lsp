package Fuzz::Exporter;
use Exporter 'import';
our @EXPORT = qw(default_func);
our @EXPORT_OK = qw(optional_func);
sub default_func { return 'default' }
sub optional_func { return 'optional' }
