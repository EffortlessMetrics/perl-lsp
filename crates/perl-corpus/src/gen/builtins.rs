use proptest::prelude::*;

use super::qw::identifier;

/// Generate a small positive integer
fn small_int() -> impl Strategy<Value = i32> {
    1i32..100
}

/// Generate a byte value
fn byte_val() -> impl Strategy<Value = u8> {
    32u8..127 // printable ASCII
}

/// Generate a simple string literal
fn string_literal() -> impl Strategy<Value = String> {
    prop_oneof![
        identifier().prop_map(|s| format!("\"{}\"", s)),
        Just("\"hello\"".to_string()),
        Just("\"world\"".to_string()),
        Just("\"test\"".to_string()),
    ]
}

/// Generate a pack template
fn pack_template() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("C*"), Just("A*"), Just("Z*"), Just("n"), Just("N"), Just("v"), Just("V"),]
}

fn pack_unpack() -> impl Strategy<Value = String> {
    (identifier(), identifier(), pack_template(), prop::collection::vec(byte_val(), 1..5)).prop_map(
        |(packed, bytes, template, vals)| {
            let vals_str = vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
            format!(
                "my ${} = pack(\"{}\", {});\nmy @{} = unpack(\"{}\", ${});\n",
                packed, template, vals_str, bytes, template, packed
            )
        },
    )
}

fn split_join() -> impl Strategy<Value = String> {
    (
        identifier(),
        identifier(),
        identifier(),
        prop_oneof![Just(","), Just(":"), Just(";"), Just("\\s+")],
        prop_oneof![Just(":"), Just("-"), Just("_"), Just("/")],
    )
        .prop_map(|(line, parts, joined, split_on, join_with)| {
            format!(
                "my ${} = \"a{}b{}c\";\nmy @{} = split /{split_on}/, ${};\nmy ${} = join \"{}\", @{};\n",
                line, split_on.replace("\\s+", " "), split_on.replace("\\s+", " "),
                parts, line, joined, join_with, parts
            )
        })
}

fn printf_sprintf() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier(), string_literal(), small_int()).prop_map(
        |(name, count, msg, name_val, count_val)| {
            format!(
                "my ${} = {};\nmy ${} = {};\nmy ${} = sprintf(\"%s:%d\", ${}, ${});\nprintf \"%s\\n\", ${};\n",
                name, name_val, count, count_val, msg, name, count, msg
            )
        },
    )
}

fn print_say() -> impl Strategy<Value = String> {
    (string_literal(), string_literal())
        .prop_map(|(msg1, msg2)| format!("use v5.10;\nprint {};\nsay {};\n", msg1, msg2))
}

fn system_call() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("system \"echo\", \"ok\";\n".to_string()),
        Just("system \"true\";\n".to_string()),
        identifier().prop_map(|cmd| format!("system \"echo\", \"{}\";\n", cmd)),
    ]
}

fn time_localtime() -> impl Strategy<Value = String> {
    identifier().prop_map(|var| format!("my ${} = localtime(time);\n", var))
}

fn time_gmtime() -> impl Strategy<Value = String> {
    identifier().prop_map(|var| format!("my @{} = gmtime(time);\n", var))
}

fn chomp_line() -> impl Strategy<Value = String> {
    (identifier(), string_literal())
        .prop_map(|(var, val)| format!("my ${} = {};\nchomp ${};\n", var, val, var))
}

fn keys_values() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier(), small_int(), small_int()).prop_map(
        |(map, keys, vals, v1, v2)| {
            format!(
                "my %{} = (a => {}, b => {});\nmy @{} = keys %{};\nmy @{} = values %{};\n",
                map, v1, v2, keys, map, vals, map
            )
        },
    )
}

fn each_delete() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier(), small_int(), small_int()).prop_map(
        |(map, k, v, v1, v2)| {
            format!(
                "my %{} = (a => {}, b => {});\nmy (${}, ${}) = each %{};\ndelete ${}{{${}}};\n",
                map, v1, v2, k, v, map, map, k
            )
        },
    )
}

fn substr_ops() -> impl Strategy<Value = String> {
    (identifier(), identifier(), string_literal(), 0i32..5, 1i32..4).prop_map(
        |(text, chunk, val, start, len)| {
            format!(
                "my ${} = {};\nmy ${} = substr(${}, {}, {});\nsubstr(${}, 0, 1) = \"X\";\n",
                text, val, chunk, text, start, len, text
            )
        },
    )
}

fn index_ops() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier(), string_literal()).prop_map(
        |(text, pos, last, val)| {
            format!(
                "my ${} = {};\nmy ${} = index(${}, \"o\");\nmy ${} = rindex(${}, \"o\");\n",
                text, val, pos, text, last, text
            )
        },
    )
}

fn pos_study() -> impl Strategy<Value = String> {
    (identifier(), identifier(), string_literal()).prop_map(|(text, where_var, val)| {
        format!(
            "my ${} = {};\n${} =~ /o/g;\nmy ${} = pos(${});\nstudy ${};\n",
            text, val, text, where_var, text, text
        )
    })
}

fn length_chop() -> impl Strategy<Value = String> {
    (identifier(), identifier(), string_literal()).prop_map(|(text, len, val)| {
        format!("my ${} = {};\nmy ${} = length ${};\nchop ${};\n", text, val, len, text, text)
    })
}

fn quotemeta_op() -> impl Strategy<Value = String> {
    (identifier(), identifier()).prop_map(|(text, quoted)| {
        format!("my ${} = \"a.b*c\";\nmy ${} = quotemeta ${};\n", text, quoted, text)
    })
}

fn bless_ref() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier(), small_int()).prop_map(|(obj, kind, class, val)| {
        // Use to_ascii_uppercase() which returns a char, not an iterator
        let class_name = class.chars().next().unwrap_or('C').to_ascii_uppercase();
        format!(
            "my ${} = bless {{ count => {} }}, \"{}\";\nmy ${} = ref ${};\n",
            obj, val, class_name, kind, obj
        )
    })
}

fn caller_wantarray() -> impl Strategy<Value = String> {
    (identifier(), identifier()).prop_map(|(caller_var, context)| {
        format!("my @{} = caller;\nmy ${} = wantarray();\n", caller_var, context)
    })
}

fn warn_die() -> impl Strategy<Value = String> {
    (string_literal(), string_literal(), identifier())
        .prop_map(|(note, fatal, cond)| format!("warn {};\ndie {} if ${};\n", note, fatal, cond))
}

fn push_pop() -> impl Strategy<Value = String> {
    (identifier(), identifier(), prop::collection::vec(small_int(), 1..5), small_int()).prop_map(
        |(stack, last, init, push_val)| {
            let init_str = init.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
            format!(
                "my @{} = ({});\npush @{}, {};\nmy ${} = pop @{};\n",
                stack, init_str, stack, push_val, last, stack
            )
        },
    )
}

fn shift_unshift() -> impl Strategy<Value = String> {
    (identifier(), identifier(), prop::collection::vec(small_int(), 1..5), small_int()).prop_map(
        |(queue, first, init, unshift_val)| {
            let init_str = init.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
            format!(
                "my @{} = ({});\nunshift @{}, {};\nmy ${} = shift @{};\n",
                queue, init_str, queue, unshift_val, first, queue
            )
        },
    )
}

fn splice_replace() -> impl Strategy<Value = String> {
    (
        identifier(),
        identifier(),
        prop::collection::vec(small_int(), 3..7),
        0i32..2,
        1i32..3,
        small_int(),
        small_int(),
    )
        .prop_map(|(items, removed, init, offset, length, r1, r2)| {
            let init_str = init.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
            format!(
                "my @{} = ({});\nmy @{} = splice @{}, {}, {}, ({}, {});\n",
                items, init_str, removed, items, offset, length, r1, r2
            )
        })
}

fn reverse_list() -> impl Strategy<Value = String> {
    (identifier(), identifier(), prop::collection::vec(string_literal(), 2..5)).prop_map(
        |(items, rev, init)| {
            let init_str = init.join(", ");
            format!("my @{} = ({});\nmy @{} = reverse @{};\n", items, init_str, rev, items)
        },
    )
}

fn uc_lc() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier(), identifier(), identifier(), string_literal())
        .prop_map(|(name, upper, lower, upper_first, lower_first, val)| {
            format!(
                "my ${} = {};\nmy ${} = uc ${};\nmy ${} = lc ${};\nmy ${} = ucfirst ${};\nmy ${} = lcfirst ${};\n",
                name, val, upper, name, lower, name, upper_first, name, lower_first, name
            )
        })
}

fn chr_ord() -> impl Strategy<Value = String> {
    (identifier(), identifier(), byte_val()).prop_map(|(letter, code, val)| {
        format!("my ${} = chr {};\nmy ${} = ord ${};\n", letter, val, code, letter)
    })
}

fn rand_int() -> impl Strategy<Value = String> {
    (identifier(), small_int(), small_int()).prop_map(|(roll, seed, max)| {
        format!("srand {};\nmy ${} = int(rand {}) + 1;\n", seed, roll, max)
    })
}

fn math_ops() -> impl Strategy<Value = String> {
    (
        identifier(),
        identifier(),
        identifier(),
        identifier(),
        identifier(),
        -100i32..100i32,
        1i32..100,
    )
        .prop_map(|(value, abs_var, whole, root, angle, val, sqrt_val)| {
            format!(
                "my ${} = {};\nmy ${} = abs(${});\nmy ${} = int(${});\nmy ${} = sqrt({});\nmy ${} = atan2(1, 1);\n",
                value, val, abs_var, value, whole, value, root, sqrt_val, angle
            )
        })
}

fn hex_oct() -> impl Strategy<Value = String> {
    (identifier(), identifier(), 0u8..255, 0u8..255).prop_map(
        |(hex_var, oct_var, hex_val, oct_val)| {
            format!(
                "my ${} = hex(\"{:x}\");\nmy ${} = oct(\"{:o}\");\n",
                hex_var, hex_val, oct_var, oct_val
            )
        },
    )
}

fn vec_ops() -> impl Strategy<Value = String> {
    (identifier(), identifier(), 0i32..32, 0i32..1).prop_map(|(bits, flag, bit_pos, bit_val)| {
        format!(
            "my ${} = \"\\0\" x 4;\nvec(${}, {}, 1) = {};\nmy ${} = vec(${}, {}, 1);\n",
            bits, bits, bit_pos, bit_val, flag, bits, bit_pos
        )
    })
}

fn sleep_alarm() -> impl Strategy<Value = String> {
    (1i32..5, 1i32..3).prop_map(|(alarm_secs, sleep_secs)| {
        format!("alarm {};\nsleep {};\n", alarm_secs, sleep_secs)
    })
}

fn chdir_mkdir() -> impl Strategy<Value = String> {
    (identifier(), identifier()).prop_map(|(ok, dir)| {
        format!("my ${} = chdir \"/tmp\";\nmkdir \"{}\";\nrmdir \"{}\";\n", ok, dir, dir)
    })
}

fn rename_unlink() -> impl Strategy<Value = String> {
    (identifier(), identifier()).prop_map(|(old, new)| {
        format!("rename \"{}.log\", \"{}.log\";\nunlink \"{}.log\";\n", old, new, old)
    })
}

fn chmod_chown() -> impl Strategy<Value = String> {
    (identifier(), 0o644i32..0o755i32, 1000i32..2000).prop_map(|(file, mode, uid)| {
        format!("chmod 0{:o}, \"{}.txt\";\nchown {}, {}, \"{}.txt\";\n", mode, file, uid, uid, file)
    })
}

fn link_ops() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier(), identifier()).prop_map(|(a, b, c, target)| {
        format!(
            "link \"{}.txt\", \"{}.txt\";\nsymlink \"{}.txt\", \"{}.txt\";\nmy ${} = readlink \"{}.txt\";\n",
            a, b, a, c, target, c
        )
    })
}

fn truncate_umask() -> impl Strategy<Value = String> {
    (identifier(), identifier(), 0o022i32..0o077i32).prop_map(|(old, file, mask)| {
        format!("my ${} = umask 0{:o};\ntruncate \"{}.txt\", 0;\n", old, mask, file)
    })
}

fn stat_lstat() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier(), identifier()).prop_map(
        |(stat_var, lstat_var, file, link)| {
            format!(
                "my @{} = stat \"{}.txt\";\nmy @{} = lstat \"{}.txt\";\n",
                stat_var, file, lstat_var, link
            )
        },
    )
}

fn defined_exists() -> impl Strategy<Value = String> {
    (
        identifier(),
        identifier(),
        prop_oneof![Just("HOME"), Just("PATH"), Just("SHELL"), Just("USER")],
    )
        .prop_map(|(value, has, env_key)| {
            format!(
                "my ${} = defined $ENV{{{}}} ? $ENV{{{}}} : \"\";\nmy ${} = exists $ENV{{{}}};\n",
                value, env_key, env_key, has, env_key
            )
        })
}

fn fileno_close() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier()).prop_map(|(fh, fd, file)| {
        format!(
            "open my ${}, \"<\", \"{}.txt\" or die $!;\nmy ${} = fileno ${};\nclose ${};\n",
            fh, file, fd, fh, fh
        )
    })
}

fn readline_eof() -> impl Strategy<Value = String> {
    (identifier(), prop_oneof![Just("STDIN"), Just("ARGV"), Just("DATA")]).prop_map(
        |(line, handle)| {
            format!("my ${} = <{}>;\nif (eof {}) {{ warn \"eof\"; }}\n", line, handle, handle)
        },
    )
}

fn formline_statement() -> impl Strategy<Value = String> {
    (identifier(), string_literal()).prop_map(|(picture, val)| {
        format!(
            "my ${} = \"@<<\";\nformline ${}, {};\nmy $out = $^A;\n$^A = \"\";\n",
            picture, picture, val
        )
    })
}

/// Generate built-in function call statements.
pub fn builtin_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        pack_unpack(),
        split_join(),
        printf_sprintf(),
        print_say(),
        system_call(),
        time_localtime(),
        time_gmtime(),
        chomp_line(),
        keys_values(),
        each_delete(),
        substr_ops(),
        index_ops(),
        pos_study(),
        length_chop(),
        quotemeta_op(),
        bless_ref(),
        caller_wantarray(),
        warn_die(),
        push_pop(),
        shift_unshift(),
        splice_replace(),
        reverse_list(),
        uc_lc(),
        chr_ord(),
        rand_int(),
        math_ops(),
        hex_oct(),
        vec_ops(),
        sleep_alarm(),
        chdir_mkdir(),
        rename_unlink(),
        chmod_chown(),
        link_ops(),
        truncate_umask(),
        stat_lstat(),
        defined_exists(),
        fileno_close(),
        readline_eof(),
        formline_statement(),
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
                    || code.contains("gmtime")
                    || code.contains("print")
                    || code.contains("say")
                    || code.contains("chomp")
                    || code.contains("keys")
                    || code.contains("values")
                    || code.contains("substr")
                    || code.contains("index")
                    || code.contains("rindex")
                    || code.contains("pos")
                    || code.contains("study")
                    || code.contains("length")
                    || code.contains("chop")
                    || code.contains("quotemeta")
                    || code.contains("bless")
                    || code.contains("ref")
                    || code.contains("caller")
                    || code.contains("wantarray")
                    || code.contains("warn")
                    || code.contains("die")
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
                    || code.contains("vec")
                    || code.contains("alarm")
                    || code.contains("sleep")
                    || code.contains("each")
                    || code.contains("delete")
                    || code.contains("chdir")
                    || code.contains("mkdir")
                    || code.contains("rmdir")
                    || code.contains("rename")
                    || code.contains("unlink")
                    || code.contains("chmod")
                    || code.contains("chown")
                    || code.contains("link")
                    || code.contains("symlink")
                    || code.contains("readlink")
                    || code.contains("umask")
                    || code.contains("truncate")
                    || code.contains("stat")
                    || code.contains("lstat")
                    || code.contains("defined")
                    || code.contains("exists")
                    || code.contains("fileno")
                    || code.contains("open")
                    || code.contains("close")
                    || code.contains("eof")
                    || code.contains("formline"),
                "Expected builtin keyword in: {}",
                code
            );
        }
    }
}
