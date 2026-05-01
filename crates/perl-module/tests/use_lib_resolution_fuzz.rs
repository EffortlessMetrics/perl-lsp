use std::path::Path;

use perl_module::resolution::use_lib::{
    extract_use_lib_operations, extract_use_lib_paths, resolve_use_lib_paths,
};

fn fuzz_string(state: &mut u64, max_len: usize) -> String {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;

    let len = (*state as usize) % (max_len.saturating_add(1));
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        let byte = match (*state % 16) as u8 {
            0 => b'\n',
            1 => b'\r',
            2 => b'\t',
            3 => b'"',
            4 => b'\'',
            5 => b';',
            6 => b'(',
            7 => b')',
            8 => b'/',
            9 => b'\\',
            10 => b'.',
            11 => b',',
            _ => b'a' + ((*state % 26) as u8),
        };
        out.push(char::from(byte));
    }
    out
}

#[test]
fn fuzz_use_lib_parser_and_resolver_preserve_core_invariants() {
    let mut seed = 0x00C0_FFEE_F00D_BAAD_u64;

    for _ in 0..5_000 {
        let source = fuzz_string(&mut seed, 256);
        let paths = extract_use_lib_paths(&source);
        let ops = extract_use_lib_operations(&source);

        for path in &paths {
            assert!(
                !path.path.is_empty(),
                "extracted use lib path should never be empty; source={source:?}"
            );
        }

        let resolved = resolve_use_lib_paths(
            &paths,
            Path::new("/workspace"),
            Some(Path::new("/workspace/project/lib")),
        );

        for rel in resolved {
            assert!(
                !Path::new(&rel).is_absolute(),
                "resolved include path should be relative to workspace: {rel:?}; source={source:?}"
            );
            assert!(
                !Path::new(&rel)
                    .components()
                    .any(|component| component == std::path::Component::ParentDir),
                "resolved include path should not contain parent traversals: {rel:?}; source={source:?}"
            );
        }

        for op in &ops {
            match op {
                perl_module::resolution::use_lib::UseLibAction::Add(op_paths)
                | perl_module::resolution::use_lib::UseLibAction::Remove(op_paths) => {
                    for op_path in op_paths {
                        assert!(
                            !op_path.path.is_empty(),
                            "operation paths should never be empty; source={source:?}"
                        );
                    }
                }
            }
        }
    }
}
