use proptest::prelude::*;

fn pack_unpack() -> impl Strategy<Value = String> {
    Just(
        "my $packed = pack(\"C*\", 65, 66, 67);\nmy @bytes = unpack(\"C*\", $packed);\n"
            .to_string(),
    )
}

fn split_join() -> impl Strategy<Value = String> {
    Just(
        "my $line = \"a,b,c\";\nmy @parts = split /,/, $line;\nmy $joined = join \":\", @parts;\n"
            .to_string(),
    )
}

fn printf_sprintf() -> impl Strategy<Value = String> {
    Just(
        "my $name = \"Ada\";\nmy $count = 3;\nmy $msg = sprintf(\"%s:%d\", $name, $count);\nprintf \"%s\\n\", $msg;\n"
            .to_string(),
    )
}

fn system_call() -> impl Strategy<Value = String> {
    Just("system \"echo\", \"ok\";\n".to_string())
}

fn time_localtime() -> impl Strategy<Value = String> {
    Just("my $when = localtime(time);\n".to_string())
}

fn chomp_line() -> impl Strategy<Value = String> {
    Just("my $line = \"value\\n\";\nchomp $line;\n".to_string())
}

fn keys_values() -> impl Strategy<Value = String> {
    Just(
        "my %map = (a => 1, b => 2);\nmy @keys = keys %map;\nmy @vals = values %map;\n".to_string(),
    )
}

fn each_delete() -> impl Strategy<Value = String> {
    Just(
        "my %map = (a => 1, b => 2);\nmy ($k, $v) = each %map;\ndelete $map{$k};\n".to_string(),
    )
}

fn substr_ops() -> impl Strategy<Value = String> {
    Just(
        "my $text = \"foobar\";\nmy $chunk = substr($text, 1, 3);\nsubstr($text, 0, 1) = \"F\";\n"
            .to_string(),
    )
}

fn index_ops() -> impl Strategy<Value = String> {
    Just(
        "my $text = \"foobar\";\nmy $pos = index($text, \"bar\");\nmy $last = rindex($text, \"o\");\n"
            .to_string(),
    )
}

fn length_chop() -> impl Strategy<Value = String> {
    Just("my $text = \"line\\n\";\nmy $len = length $text;\nchop $text;\n".to_string())
}

fn bless_ref() -> impl Strategy<Value = String> {
    Just("my $obj = bless { count => 1 }, \"Counter\";\nmy $kind = ref $obj;\n".to_string())
}

fn caller_wantarray() -> impl Strategy<Value = String> {
    Just("my @caller = caller;\nmy $context = wantarray();\n".to_string())
}

fn push_pop() -> impl Strategy<Value = String> {
    Just("my @stack = (1, 2, 3);\npush @stack, 4;\nmy $last = pop @stack;\n".to_string())
}

fn shift_unshift() -> impl Strategy<Value = String> {
    Just(
        "my @queue = (1, 2, 3);\nunshift @queue, 0;\nmy $first = shift @queue;\n".to_string(),
    )
}

fn splice_replace() -> impl Strategy<Value = String> {
    Just(
        "my @items = (1, 2, 3, 4, 5);\nmy @removed = splice @items, 1, 2, (9, 10);\n"
            .to_string(),
    )
}

fn reverse_list() -> impl Strategy<Value = String> {
    Just("my @items = (\"a\", \"b\", \"c\");\nmy @rev = reverse @items;\n".to_string())
}

fn uc_lc() -> impl Strategy<Value = String> {
    Just(
        "my $name = \"Ada\";\nmy $upper = uc $name;\nmy $lower = lc $name;\nmy $upper_first = ucfirst $name;\nmy $lower_first = lcfirst $name;\n"
            .to_string(),
    )
}

fn chr_ord() -> impl Strategy<Value = String> {
    Just("my $letter = chr 65;\nmy $code = ord $letter;\n".to_string())
}

fn rand_int() -> impl Strategy<Value = String> {
    Just("srand 42;\nmy $roll = int(rand 6) + 1;\n".to_string())
}

fn math_ops() -> impl Strategy<Value = String> {
    Just(
        "my $value = -4.2;\nmy $abs = abs($value);\nmy $whole = int($value);\nmy $root = sqrt(9);\nmy $angle = atan2(1, 1);\n"
            .to_string(),
    )
}

fn hex_oct() -> impl Strategy<Value = String> {
    Just("my $hex = hex(\"ff\");\nmy $oct = oct(\"377\");\n".to_string())
}

fn chdir_mkdir() -> impl Strategy<Value = String> {
    Just("my $ok = chdir \"/tmp\";\nmkdir \"data\";\nrmdir \"data\";\n".to_string())
}

fn rename_unlink() -> impl Strategy<Value = String> {
    Just("rename \"old.log\", \"new.log\";\nunlink \"old.log\";\n".to_string())
}

fn stat_lstat() -> impl Strategy<Value = String> {
    Just("my @stat = stat \"file.txt\";\nmy @lstat = lstat \"link.txt\";\n".to_string())
}

fn defined_exists() -> impl Strategy<Value = String> {
    Just(
        "my $value = defined $ENV{HOME} ? $ENV{HOME} : \"\";\nmy $has = exists $ENV{PATH};\n"
            .to_string(),
    )
}

/// Generate built-in function call statements.
pub fn builtin_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        pack_unpack(),
        split_join(),
        printf_sprintf(),
        system_call(),
        time_localtime(),
        chomp_line(),
        keys_values(),
        each_delete(),
        substr_ops(),
        index_ops(),
        length_chop(),
        bless_ref(),
        caller_wantarray(),
        push_pop(),
        shift_unshift(),
        splice_replace(),
        reverse_list(),
        uc_lc(),
        chr_ord(),
        rand_int(),
        math_ops(),
        hex_oct(),
        chdir_mkdir(),
        rename_unlink(),
        stat_lstat(),
        defined_exists(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn builtins_include_keyword(code in builtin_in_context()) {
            assert!(
                code.contains("pack")
                    || code.contains("split")
                    || code.contains("sprintf")
                    || code.contains("system")
                    || code.contains("localtime")
                    || code.contains("chomp")
                    || code.contains("keys")
                    || code.contains("values")
                    || code.contains("substr")
                    || code.contains("index")
                    || code.contains("rindex")
                    || code.contains("length")
                    || code.contains("chop")
                    || code.contains("bless")
                    || code.contains("ref")
                    || code.contains("caller")
                    || code.contains("wantarray")
                    || code.contains("push")
                    || code.contains("pop")
                    || code.contains("shift")
                    || code.contains("unshift")
                    || code.contains("splice")
                    || code.contains("reverse")
                    || code.contains("uc")
                    || code.contains("lc")
                    || code.contains("ucfirst")
                    || code.contains("lcfirst")
                    || code.contains("chr")
                    || code.contains("ord")
                    || code.contains("srand")
                    || code.contains("rand")
                    || code.contains("abs")
                    || code.contains("sqrt")
                    || code.contains("atan2")
                    || code.contains("hex")
                    || code.contains("oct")
                    || code.contains("each")
                    || code.contains("delete")
                    || code.contains("chdir")
                    || code.contains("mkdir")
                    || code.contains("rmdir")
                    || code.contains("rename")
                    || code.contains("unlink")
                    || code.contains("stat")
                    || code.contains("lstat")
                    || code.contains("defined")
                    || code.contains("exists"),
                "Expected builtin keyword in: {}",
                code
            );
        }
    }
}
