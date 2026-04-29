#[cfg(test)]
mod tests {
    use crate::refactor::refactoring::RefactoringEngine;
    use crate::refactor::refactoring::RefactoringScope;
    use crate::refactor::refactoring::RefactoringType;
    use perl_tdd_support::must;
    use std::io::Write;

    #[test]
    fn test_file_scoped_rename() {
        // AC1: support File scope
        let mut file = must(tempfile::NamedTempFile::new());
        let code = "my $foo = 1; print $foo;";
        must(write!(file, "{}", code));
        let path = file.path().to_path_buf();

        let config = crate::refactor::refactoring::RefactoringConfig {
            safe_mode: false,
            ..Default::default()
        };
        let mut engine = RefactoringEngine::with_config(config);
        must(engine.index_file(&path, code));

        let result = must(engine.refactor(
            RefactoringType::SymbolRename {
                old_name: "$foo".to_string(),
                new_name: "$bar".to_string(),
                scope: RefactoringScope::File(path.clone()),
            },
            vec![path.clone()],
        ));

        assert!(result.success);
        assert_eq!(result.files_modified, 1);
        let new_code = must(std::fs::read_to_string(&path));
        assert!(new_code.contains("$bar"));
        assert!(!new_code.contains("$foo"));
    }

    #[test]
    fn test_package_scoped_rename_limits_changes_to_target_package() {
        // AC1: support Package scope by limiting changes to requested package only
        let mut file = must(tempfile::NamedTempFile::new());
        let code = "package P; my $foo = 1;
package Q; my $foo = 2;
";
        must(write!(file, "{}", code));
        let path = file.path().to_path_buf();

        let config = crate::refactor::refactoring::RefactoringConfig {
            safe_mode: false,
            ..Default::default()
        };
        let mut engine = RefactoringEngine::with_config(config);
        must(engine.index_file(&path, code));

        let result = must(engine.refactor(
            RefactoringType::SymbolRename {
                old_name: "$foo".to_string(),
                new_name: "$bar".to_string(),
                scope: RefactoringScope::Package { file: path.clone(), name: "P".to_string() },
            },
            vec![path.clone()],
        ));

        assert!(result.success);
        let new_code = must(std::fs::read_to_string(&path));
        assert!(new_code.contains("package P; my $bar = 1;"));
        assert!(new_code.contains("package Q; my $foo = 2;"));
        assert!(!new_code.contains("package Q; my $bar = 2;"));
    }

    #[test]
    fn test_workspace_wide_rename() {
        // AC1: support Workspace scope
        let mut file1 = must(tempfile::NamedTempFile::new());
        let mut file2 = must(tempfile::NamedTempFile::new());
        let code1 = "my $foo = 1;";
        let code2 = "print $foo;";
        must(write!(file1, "{}", code1));
        must(write!(file2, "{}", code2));
        let path1 = file1.path().to_path_buf();
        let path2 = file2.path().to_path_buf();

        let config = crate::refactor::refactoring::RefactoringConfig {
            safe_mode: false,
            ..Default::default()
        };
        let mut engine = RefactoringEngine::with_config(config);
        must(engine.index_file(&path1, code1));
        must(engine.index_file(&path2, code2));

        let result = must(engine.refactor(
            RefactoringType::SymbolRename {
                old_name: "$foo".to_string(),
                new_name: "$bar".to_string(),
                scope: RefactoringScope::Workspace,
            },
            vec![path1.clone(), path2.clone()],
        ));

        assert!(result.success);
        assert_eq!(result.files_modified, 2);
        let new_code1 = must(std::fs::read_to_string(&path1));
        let new_code2 = must(std::fs::read_to_string(&path2));
        assert!(new_code1.contains("$bar"));
        assert!(new_code2.contains("$bar"));
    }
}
