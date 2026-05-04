package Accuracy::ImportsExports;

use strict;
use warnings;

our @EXPORT_OK = qw(answer);

sub answer {
    return 42;
}

package Accuracy::ImportsConsumer;

use strict;
use warnings;
use Accuracy::ImportsExports qw(answer);

sub call_imported {
    return answer();
}

1;
