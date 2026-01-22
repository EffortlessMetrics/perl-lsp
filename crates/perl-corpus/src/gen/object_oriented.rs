use proptest::prelude::*;

use super::declarations::package_name;
use super::qw::identifier;

fn bless_hash() -> impl Strategy<Value = String> {
    (package_name(), identifier()).prop_map(|(pkg, field)| {
        format!(
            "package {pkg};\nsub new {{ bless {{ {field} => 1 }}, shift }}\npackage main;\nmy $obj = {pkg}->new();\nmy $kind = ref $obj;\n"
        )
    })
}

fn can_does_checks() -> impl Strategy<Value = String> {
    Just(
        "package Widget;\nsub new { bless {}, shift }\nsub work { return 1; }\n\npackage main;\nmy $obj = Widget->new();\nif ($obj->can(\"work\")) { $obj->work(); }\nif ($obj->DOES(\"Role::Worker\")) { print \"role\"; }\n"
            .to_string(),
    )
}

fn inheritance_super() -> impl Strategy<Value = String> {
    Just(
        "package Base;\nsub new { bless {}, shift }\nsub greet { return \"hi\"; }\n\npackage Child;\nuse parent \"Base\";\nsub greet { my $self = shift; return $self->SUPER::greet(); }\n\npackage main;\nmy $obj = Child->new();\n"
            .to_string(),
    )
}

fn overload_package() -> impl Strategy<Value = String> {
    Just(
        "package Counter;\nuse overload '\"\"' => sub { $_[0]->{count} }, '0+' => sub { $_[0]->{count} }, fallback => 1;\nsub new { bless { count => 1 }, shift }\n\npackage main;\nmy $c = Counter->new();\n"
            .to_string(),
    )
}

fn mro_example() -> impl Strategy<Value = String> {
    Just(
        "package Base;\nsub ping { return \"base\"; }\n\npackage Mid;\nuse mro \"c3\";\nour @ISA = (\"Base\");\nsub ping { return \"mid\"; }\n\npackage main;\nmy $obj = bless {}, \"Mid\";\n"
            .to_string(),
    )
}

fn class_feature() -> impl Strategy<Value = String> {
    Just(
        "use v5.38;\nuse feature 'class';\nno warnings 'experimental::class';\nclass Point {\n    field $x :param = 0;\n    method get_x { return $x; }\n}\n"
            .to_string(),
    )
}

/// Generate object-oriented Perl snippets (bless, inheritance, overload, class).
pub fn object_oriented_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        bless_hash(),
        can_does_checks(),
        inheritance_super(),
        overload_package(),
        mro_example(),
        class_feature(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn object_snippets_include_oop_keywords(code in object_oriented_in_context()) {
            assert!(
                code.contains("bless")
                    || code.contains("use parent")
                    || code.contains("SUPER::")
                    || code.contains("use overload")
                    || code.contains("use mro")
                    || code.contains("class "),
                "Expected object-oriented keyword in: {}",
                code
            );
        }
    }
}
