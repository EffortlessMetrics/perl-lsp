my $name = "world";
my $text = "hello $name";
$text =~ /(hello)\s+(\w+)/;
my $capture = $1;
