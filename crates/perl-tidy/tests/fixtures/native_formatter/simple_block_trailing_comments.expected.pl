sub demo {
    return 1;
} # sub tail
if ($ok) {
    return 1;
} # if tail
while ($ok) {
    next;
} # while tail
unless ($ok) {
    return 0;
} # unless tail
until ($done) {
    return 1;
} # until tail
foreach my $item (@items) {
    return $item;
} # foreach tail
for (my $i = 0; $i < 3; $i++) {
    next;
} # for tail
if ($maybe) {
    return 2;
} else {
    return 3;
} # if else tail
while ($again) {
    next;
} continue {
    last;
} # continue tail
