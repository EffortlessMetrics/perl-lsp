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

my ($dept, $id, $hours, $rate) = ("Engineering", 42, 160, 75.50);
my $description = "Weekly status report for team lead review";

format EMPLOYEE =
ID: @####  Name: @<<<<<<<<<<  Dept: @<<<<<<<<<<<<
$id, $name, $dept
.

format CENTERED =
@|||||||||||||||||||||||||||||||||||||||||
$title
@|||||||||||||||||||||||||||||||||||||||||
$description
.

format NUMERIC =
Hours: @###  Rate: @####.##  Total: @#####.##
$hours, $rate, $hours * $rate
.

format MULTILINE =
Name: @<<<<<<<<<<<<<<
$name
Title: @<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$title
.

format WRAPPED =
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$description
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$description
.

format EMPTY_BODY =
.

write;
