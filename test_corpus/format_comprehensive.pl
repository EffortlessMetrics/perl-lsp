#!/usr/bin/perl
use strict;
use warnings;

# Comprehensive format statement test corpus for issue #432
# This file tests all acceptance criteria for format statement coverage

# Test variables
my ($name, $age, $salary) = ("Ada Lovelace", 37, 1234.50);
my $title = "Employee Report";
my $payee = "John Doe";
my $amount = 5000.00;
my ($dept, $employee_id) = ("Engineering", "EMP-12345");
my $description = "This is a long description that needs to wrap across multiple lines in the output";

# TC1: Basic format with left-justified field (@<<<)
# AC1: Parser recognizes format keyword
# AC2: Picture lines parsed correctly
# AC3: Field specifiers recognized
format STDOUT =
@<<<<<< @>>>>  @####.##
$name, $age, $salary
.

# TC2: Format with multiple field types (left, right, numeric)
# AC3: Multiple field specifier types in one format
format REPORT =
Name: @<<<<<<<<<<<<<<<<<<<<<<<  Age: @##  Salary: @#######.##
$name, $age, $salary
.

# TC3: Format with _TOP variant (page header)
# AC4: Format variable binding with _TOP suffix
format STDOUT_TOP =
Page @<<
$%
.

# TC4: Anonymous format (no name)
# AC4: Format variable binding supported (anonymous)
format =
@|||||||||||||||||||||||||||
$title
.

# TC5: Format with center-justified fields (@|||)
# AC3: Center-justified field specifiers
format CENTERED =
@|||||||||||||||||||||||||||||||||||||||||||||||||||
$title
@|||||||||||||||||||  @|||||||||||||||||||
$name,                $dept
.

# TC6: Format with numeric field specifiers (@###.##)
# AC3: Numeric field specifiers with decimal points
format PAYCHECK =
*******************************************************
*  PAY TO THE ORDER OF: @<<<<<<<<<<<<<<<<<<<<<<<<  *
$payee
*  AMOUNT:             @#######.##                 *
                       $amount
*******************************************************
.

# TC7: Format with right-justified fields (@>>>)
# AC3: Right-justified field specifiers
format RIGHT_ALIGNED =
Employee ID: @>>>>>>>>>>>>
             $employee_id
Department:  @>>>>>>>>>>>>
             $dept
.

# TC8: Multi-line format with multiple picture lines
# AC2: Multiline picture lines parsed correctly
# AC7: Picture lines captured correctly in AST
format COMPLEX_REPORT =
================================================================================
                            EMPLOYEE REPORT
================================================================================
Name:          @<<<<<<<<<<<<<<<<<<<<<<<<<<<<
               $name
Employee ID:   @<<<<<<<<<<<
               $employee_id
Department:    @<<<<<<<<<<<<<<<<
               $dept
Age:           @##
               $age
Salary:        $@########.##
                $salary
================================================================================
.

# TC9: Format with text field specifiers and continuation (@*)
# AC3: Continuation field specifiers (^<<<)
format WRAPPED_TEXT =
Description:
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$description
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$description
.

# TC10: Format with special page number variable ($%)
# AC2: Special variables in value lines
format PAGE_HEADER =
================================================================================
                     REPORT - Page @##
                                   $%
================================================================================
.

# TC11: Format with multiple variables on same line
# AC7: Multiple variables in value lines captured correctly
format MULTI_VAR =
@<<<<<<<<<<<<<<  @>>  @#######.##  @<<<<<<<<<<<
$name,           $age, $salary,    $dept
.

# TC12: Empty format body (edge case)
# AC2: Empty format body handled correctly
format EMPTY =
.

# TC13: Format with literal text and special characters
# AC2: Literal text and special characters in picture lines
format SPECIAL_CHARS =
*** Special Characters Test ***
Name: @<<<<<<<<<<<  Dept: @<<<<<<<<<<
      $name,              $dept
=================================
.

# TC14: Format with suppress blank lines (~~)
# AC3: Special format modifiers recognized
format SUPPRESS_BLANKS =
~~^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$description
.

# TC15: Format with all field type combinations
# AC3: All field specifier types in one format
# AC6: Format statements produce correct AST with NodeKind::Format
format ALL_TYPES =
Left:        @<<<<<<<<<<<
             $name
Right:       @>>>>>>>>>>>
             $dept
Center:      @|||||||||||
             $title
Numeric:     @#######.##
             $salary
NumericInt:  @####
             $age
Continue:    ^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
             $description
.

# Use write to output format
write;

print "Format test corpus loaded successfully\n";
