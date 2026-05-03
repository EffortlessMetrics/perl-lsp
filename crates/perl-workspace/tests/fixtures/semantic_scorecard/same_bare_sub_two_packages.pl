package FirstPackage;

sub duplicated {
    return 1;
}

package SecondPackage;

sub duplicated {
    return 2;
}

duplicated();
