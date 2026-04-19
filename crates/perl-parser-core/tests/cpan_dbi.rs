//! CPAN Pattern Tests: DBI

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn connect() {
    let code =
        r#"my $dbh = DBI->connect("dbi:Pg:dbname=test", "user", "pass", { RaiseError => 1 });"#;
    assert_clean_parse(code);
}

#[test]
fn prepare_execute() {
    let code = r#"
my $sth = $dbh->prepare("SELECT * FROM foo WHERE id = ?");
$sth->execute($id);
"#;
    assert_clean_parse(code);
}

#[test]
fn fetchrow_hashref_loop() {
    let code = r#"while (my $row = $sth->fetchrow_hashref) { process($row) }"#;
    assert_clean_parse(code);
    let ast = parse(code);
    let kinds = top_level_kinds(&ast);
    assert!(
        kinds.contains(&"While"),
        "expected While loop, got: {:?}",
        kinds
    );
}

#[test]
fn fetchrow_array_loop() {
    let code = "while (my @row = $sth->fetchrow_array) { print $row[0] }";
    assert_clean_parse(code);
}

#[test]
fn selectall_arrayref() {
    let code = r#"my $rows = $dbh->selectall_arrayref("SELECT * FROM users", { Slice => {} });"#;
    assert_clean_parse(code);
}

#[test]
fn do_statement() {
    let code = r#"$dbh->do("DELETE FROM sessions WHERE expired < ?", undef, time());"#;
    assert_clean_parse(code);
}

#[test]
fn transaction_pattern() {
    let code = r#"
eval {
    $dbh->begin_work;
    $dbh->do("INSERT INTO log (msg) VALUES (?)", undef, $message);
    $dbh->do("UPDATE counters SET count = count + 1 WHERE name = ?", undef, 'inserts');
    $dbh->commit;
};
if ($@) {
    $dbh->rollback;
    die "Transaction failed: $@";
}
"#;
    assert_clean_parse(code);
}

#[test]
fn full_dbi_workflow() {
    let code = r#"
my $dbh = DBI->connect("dbi:SQLite:dbname=test.db", "", "", { RaiseError => 1 });
my $sth = $dbh->prepare("SELECT id, name FROM users WHERE active = ?");
$sth->execute(1);
while (my $row = $sth->fetchrow_hashref) {
    printf "%d: %s\n", $row->{id}, $row->{name};
}
$sth->finish;
$dbh->disconnect;
"#;
    assert_clean_parse(code);
}

#[test]
fn placeholder_bind_values() {
    let code = r#"
my $sth = $dbh->prepare("INSERT INTO users (name, email, age) VALUES (?, ?, ?)");
$sth->execute($name, $email, $age);
"#;
    assert_clean_parse(code);
}
