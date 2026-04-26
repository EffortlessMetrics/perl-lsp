my $value = "abc";
if ($value =~ /(a)(b)c/) {
    my $joined = "$1-$2";
    print $joined;
}
