package MyModule;
use strict;
use warnings;
use feature 'say';

our $VERSION = '1.0.0';

# Unicode support
my $café = "coffee shop";
my $π = 3.14159265359;
sub 日本語 { "Japanese text" }

# Complex data structures
my %config = (
    database => {
        host => 'localhost',
        port => 5432,
        name => 'myapp',
        credentials => {
            username => 'admin',
            password => 'secret',
        },
    },
    cache => {
        type => 'redis',
        ttl => 3600,
        servers => ['127.0.0.1:6379', '127.0.0.2:6379'],
    },
);

# Reference operator tests
my $config_ref = \%config;
my $db_ref = \$config{database};
my $servers_ref = \@{$config{cache}{servers}};

# Modern Perl features
sub process_data {
    my ($self, $data) = @_;
    
    given (ref $data) {
        when ('ARRAY') {
            return $self->process_array($data);
        }
        when ('HASH') {
            return $self->process_hash($data);
        }
        default {
            return $self->process_scalar($data);
        }
    }
}

# Method with ellipsis
sub not_implemented {
    ...
}

# Operator overloading
use overload
    '""' => sub { shift->stringify },
    '0+' => sub { shift->numify },
    fallback => 1;

# Complex regex with substitutions
sub sanitize_input {
    my ($self, $input) = @_;
    
    # Remove HTML tags
    $input =~ s/<[^>]+>//g;
    
    # Normalize whitespace
    $input =~ s/\s+/ /g;
    $input =~ s/^\s+|\s+$//g;
    
    # Escape special characters
    $input =~ s/(['"\\])/\\$1/g;
    
    return $input;
}

# Heredoc usage
my $usage = <<'USAGE';
Usage: $0 [OPTIONS] FILE

Options:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output
    -d, --debug     Enable debug mode

Example:
    $0 -v input.txt
USAGE

# Anonymous subroutines and closures
my $counter = do {
    my $count = 0;
    sub { ++$count }
};

# File operations
sub read_config {
    my ($self, $filename) = @_;
    
    open my $fh, '<', $filename or die "Cannot open $filename: $!";
    
    my %data;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^\s*#/;  # Skip comments
        next if $line =~ /^\s*$/;  # Skip empty lines
        
        if ($line =~ /^(\w+)\s*=\s*(.+)$/) {
            $data{$1} = $2;
        }
    }
    
    close $fh;
    return \%data;
}

# Package with inheritance
package MyModule::Child;
use parent 'MyModule';

sub new {
    my $class = shift;
    my $self = $class->SUPER::new(@_);
    $self->{child_attribute} = 1;
    return $self;
}

# Back to main package
package MyModule;

# Export functions
use Exporter 'import';
our @EXPORT_OK = qw(process_data sanitize_input);
our %EXPORT_TAGS = (all => \@EXPORT_OK);

1;

__END__

=head1 NAME

MyModule - A sample Perl module for benchmarking

=head1 SYNOPSIS

    use MyModule;
    my $obj = MyModule->new();
    $obj->process_data($data);

=head1 DESCRIPTION

This module demonstrates various Perl features for parser benchmarking.

=cut
package MyModule;
use strict;
use warnings;
use feature 'say';

our $VERSION = '1.0.0';

# Unicode support
my $café = "coffee shop";
my $π = 3.14159265359;
sub 日本語 { "Japanese text" }

# Complex data structures
my %config = (
    database => {
        host => 'localhost',
        port => 5432,
        name => 'myapp',
        credentials => {
            username => 'admin',
            password => 'secret',
        },
    },
    cache => {
        type => 'redis',
        ttl => 3600,
        servers => ['127.0.0.1:6379', '127.0.0.2:6379'],
    },
);

# Reference operator tests
my $config_ref = \%config;
my $db_ref = \$config{database};
my $servers_ref = \@{$config{cache}{servers}};

# Modern Perl features
sub process_data {
    my ($self, $data) = @_;
    
    given (ref $data) {
        when ('ARRAY') {
            return $self->process_array($data);
        }
        when ('HASH') {
            return $self->process_hash($data);
        }
        default {
            return $self->process_scalar($data);
        }
    }
}

# Method with ellipsis
sub not_implemented {
    ...
}

# Operator overloading
use overload
    '""' => sub { shift->stringify },
    '0+' => sub { shift->numify },
    fallback => 1;

# Complex regex with substitutions
sub sanitize_input {
    my ($self, $input) = @_;
    
    # Remove HTML tags
    $input =~ s/<[^>]+>//g;
    
    # Normalize whitespace
    $input =~ s/\s+/ /g;
    $input =~ s/^\s+|\s+$//g;
    
    # Escape special characters
    $input =~ s/(['"\\])/\\$1/g;
    
    return $input;
}

# Heredoc usage
my $usage = <<'USAGE';
Usage: $0 [OPTIONS] FILE

Options:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output
    -d, --debug     Enable debug mode

Example:
    $0 -v input.txt
USAGE

# Anonymous subroutines and closures
my $counter = do {
    my $count = 0;
    sub { ++$count }
};

# File operations
sub read_config {
    my ($self, $filename) = @_;
    
    open my $fh, '<', $filename or die "Cannot open $filename: $!";
    
    my %data;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^\s*#/;  # Skip comments
        next if $line =~ /^\s*$/;  # Skip empty lines
        
        if ($line =~ /^(\w+)\s*=\s*(.+)$/) {
            $data{$1} = $2;
        }
    }
    
    close $fh;
    return \%data;
}

# Package with inheritance
package MyModule::Child;
use parent 'MyModule';

sub new {
    my $class = shift;
    my $self = $class->SUPER::new(@_);
    $self->{child_attribute} = 1;
    return $self;
}

# Back to main package
package MyModule;

# Export functions
use Exporter 'import';
our @EXPORT_OK = qw(process_data sanitize_input);
our %EXPORT_TAGS = (all => \@EXPORT_OK);

1;

__END__

=head1 NAME

MyModule - A sample Perl module for benchmarking

=head1 SYNOPSIS

    use MyModule;
    my $obj = MyModule->new();
    $obj->process_data($data);

=head1 DESCRIPTION

This module demonstrates various Perl features for parser benchmarking.

=cut
package MyModule;
use strict;
use warnings;
use feature 'say';

our $VERSION = '1.0.0';

# Unicode support
my $café = "coffee shop";
my $π = 3.14159265359;
sub 日本語 { "Japanese text" }

# Complex data structures
my %config = (
    database => {
        host => 'localhost',
        port => 5432,
        name => 'myapp',
        credentials => {
            username => 'admin',
            password => 'secret',
        },
    },
    cache => {
        type => 'redis',
        ttl => 3600,
        servers => ['127.0.0.1:6379', '127.0.0.2:6379'],
    },
);

# Reference operator tests
my $config_ref = \%config;
my $db_ref = \$config{database};
my $servers_ref = \@{$config{cache}{servers}};

# Modern Perl features
sub process_data {
    my ($self, $data) = @_;
    
    given (ref $data) {
        when ('ARRAY') {
            return $self->process_array($data);
        }
        when ('HASH') {
            return $self->process_hash($data);
        }
        default {
            return $self->process_scalar($data);
        }
    }
}

# Method with ellipsis
sub not_implemented {
    ...
}

# Operator overloading
use overload
    '""' => sub { shift->stringify },
    '0+' => sub { shift->numify },
    fallback => 1;

# Complex regex with substitutions
sub sanitize_input {
    my ($self, $input) = @_;
    
    # Remove HTML tags
    $input =~ s/<[^>]+>//g;
    
    # Normalize whitespace
    $input =~ s/\s+/ /g;
    $input =~ s/^\s+|\s+$//g;
    
    # Escape special characters
    $input =~ s/(['"\\])/\\$1/g;
    
    return $input;
}

# Heredoc usage
my $usage = <<'USAGE';
Usage: $0 [OPTIONS] FILE

Options:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output
    -d, --debug     Enable debug mode

Example:
    $0 -v input.txt
USAGE

# Anonymous subroutines and closures
my $counter = do {
    my $count = 0;
    sub { ++$count }
};

# File operations
sub read_config {
    my ($self, $filename) = @_;
    
    open my $fh, '<', $filename or die "Cannot open $filename: $!";
    
    my %data;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^\s*#/;  # Skip comments
        next if $line =~ /^\s*$/;  # Skip empty lines
        
        if ($line =~ /^(\w+)\s*=\s*(.+)$/) {
            $data{$1} = $2;
        }
    }
    
    close $fh;
    return \%data;
}

# Package with inheritance
package MyModule::Child;
use parent 'MyModule';

sub new {
    my $class = shift;
    my $self = $class->SUPER::new(@_);
    $self->{child_attribute} = 1;
    return $self;
}

# Back to main package
package MyModule;

# Export functions
use Exporter 'import';
our @EXPORT_OK = qw(process_data sanitize_input);
our %EXPORT_TAGS = (all => \@EXPORT_OK);

1;

__END__

=head1 NAME

MyModule - A sample Perl module for benchmarking

=head1 SYNOPSIS

    use MyModule;
    my $obj = MyModule->new();
    $obj->process_data($data);

=head1 DESCRIPTION

This module demonstrates various Perl features for parser benchmarking.

=cut
package MyModule;
use strict;
use warnings;
use feature 'say';

our $VERSION = '1.0.0';

# Unicode support
my $café = "coffee shop";
my $π = 3.14159265359;
sub 日本語 { "Japanese text" }

# Complex data structures
my %config = (
    database => {
        host => 'localhost',
        port => 5432,
        name => 'myapp',
        credentials => {
            username => 'admin',
            password => 'secret',
        },
    },
    cache => {
        type => 'redis',
        ttl => 3600,
        servers => ['127.0.0.1:6379', '127.0.0.2:6379'],
    },
);

# Reference operator tests
my $config_ref = \%config;
my $db_ref = \$config{database};
my $servers_ref = \@{$config{cache}{servers}};

# Modern Perl features
sub process_data {
    my ($self, $data) = @_;
    
    given (ref $data) {
        when ('ARRAY') {
            return $self->process_array($data);
        }
        when ('HASH') {
            return $self->process_hash($data);
        }
        default {
            return $self->process_scalar($data);
        }
    }
}

# Method with ellipsis
sub not_implemented {
    ...
}

# Operator overloading
use overload
    '""' => sub { shift->stringify },
    '0+' => sub { shift->numify },
    fallback => 1;

# Complex regex with substitutions
sub sanitize_input {
    my ($self, $input) = @_;
    
    # Remove HTML tags
    $input =~ s/<[^>]+>//g;
    
    # Normalize whitespace
    $input =~ s/\s+/ /g;
    $input =~ s/^\s+|\s+$//g;
    
    # Escape special characters
    $input =~ s/(['"\\])/\\$1/g;
    
    return $input;
}

# Heredoc usage
my $usage = <<'USAGE';
Usage: $0 [OPTIONS] FILE

Options:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output
    -d, --debug     Enable debug mode

Example:
    $0 -v input.txt
USAGE

# Anonymous subroutines and closures
my $counter = do {
    my $count = 0;
    sub { ++$count }
};

# File operations
sub read_config {
    my ($self, $filename) = @_;
    
    open my $fh, '<', $filename or die "Cannot open $filename: $!";
    
    my %data;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^\s*#/;  # Skip comments
        next if $line =~ /^\s*$/;  # Skip empty lines
        
        if ($line =~ /^(\w+)\s*=\s*(.+)$/) {
            $data{$1} = $2;
        }
    }
    
    close $fh;
    return \%data;
}

# Package with inheritance
package MyModule::Child;
use parent 'MyModule';

sub new {
    my $class = shift;
    my $self = $class->SUPER::new(@_);
    $self->{child_attribute} = 1;
    return $self;
}

# Back to main package
package MyModule;

# Export functions
use Exporter 'import';
our @EXPORT_OK = qw(process_data sanitize_input);
our %EXPORT_TAGS = (all => \@EXPORT_OK);

1;

__END__

=head1 NAME

MyModule - A sample Perl module for benchmarking

=head1 SYNOPSIS

    use MyModule;
    my $obj = MyModule->new();
    $obj->process_data($data);

=head1 DESCRIPTION

This module demonstrates various Perl features for parser benchmarking.

=cut
