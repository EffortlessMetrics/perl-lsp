package Demo::Symbols;

sub alpha {
    return 1;
}

sub beta {
    return alpha();
}

1;
