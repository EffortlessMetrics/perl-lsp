package TypeglobAlias;

sub original {
    return 1;
}

*alias = \&original;
alias();
