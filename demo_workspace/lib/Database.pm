package Database;
use strict;
use warnings;
use DBI;

sub connect {
    # Database connection logic
    return 1;
}

sub save {
    my ($data) = @_;
    # Save data to database
    print "Saving data...\n";
    return 1;
}

sub unused_query {
    # This is dead code
    return "SELECT * FROM table";
}

1;
