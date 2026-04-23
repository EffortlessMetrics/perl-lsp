//! CPAN Pattern Tests: File::Find

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn find_with_anonymous_sub() {
    let code = "find(sub { push @files, $File::Find::name if -f }, @dirs);";
    assert_clean_parse(code);
}

#[test]
fn find_with_options_hash() {
    let code = "find({ wanted => sub { 1 }, follow => 1 }, $dir);";
    assert_clean_parse(code);
}

#[test]
fn find_filtering_by_extension() {
    let code = r#"
use File::Find;
my @pm_files;
find(
    sub {
        return unless -f;
        return unless /\.pm$/;
        push @pm_files, $File::Find::name;
    },
    @INC
);
"#;
    assert_clean_parse(code);
}

#[test]
fn find_with_no_chdir() {
    let code = r#"
find({
    wanted   => sub { push @found, $_ if -f && /\.t$/ },
    no_chdir => 1,
}, 't/');
"#;
    assert_clean_parse(code);
}

#[test]
fn find_with_preprocess() {
    let code = r#"
find({
    wanted     => sub { process($_) },
    preprocess => sub { sort @_ },
}, $start_dir);
"#;
    assert_clean_parse(code);
}
