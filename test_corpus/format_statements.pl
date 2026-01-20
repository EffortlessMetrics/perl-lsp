use strict;
use warnings;

my ($name, $age, $salary) = ("Ada", 37, 1234.50);
my $title = "Report";

format STDOUT =
@<<<<<< @>>>>  @####.##
$name, $age, $salary
.

format REPORT =
Name: @<<<<<<<<<<<<<<<<<<<<<<<  Age: @##  Salary: @#######.##
$name, $age, $salary
.

format STDOUT_TOP =
Page @<<
$%
.

format =
@|||||||||||||||||||||||||||
$title
.

write;
