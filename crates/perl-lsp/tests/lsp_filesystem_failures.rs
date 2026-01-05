use serde_json::json;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

mod common;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Filesystem failure scenario tests
/// Tests handling of permission errors, disk space, and I/O failures

#[test]
#[cfg(unix)] // set_mode API is Unix-only
fn test_read_only_file() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create a read-only file
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("readonly_{}.pl", std::process::id()));
    fs::write(&file_path, "print 'readonly';").unwrap();

    // Make file read-only
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&file_path, perms).unwrap();

    let uri = format!("file://{}", file_path.display());

    // Open read-only file (should work)
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'readonly';"
                }
            }
        }),
    );

    // Try to save changes (should fail gracefully)
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": {
                "textDocument": {"uri": uri},
                "text": "print 'modified';"
            }
        }),
    );

    // Verify file wasn't modified
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "print 'readonly';");

    // Cleanup
    let _ = fs::remove_file(&file_path);
}

#[test]
fn test_directory_as_file() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    let dir_path = temp_dir.join(format!("dir_{}", std::process::id()));
    fs::create_dir(&dir_path).unwrap();

    let uri = format!("file://{}", dir_path.display());

    // Try to open a directory as a file
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": ""
                }
            }
        }),
    );

    // Should handle gracefully
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 0}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());

    // Cleanup
    let _ = fs::remove_dir(&dir_path);
}

#[test]
fn test_non_existent_file() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///completely/non/existent/path/file.pl";

    // Try to open non-existent file
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'virtual';"
                }
            }
        }),
    );

    // Should work with in-memory content
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
#[cfg(unix)]
fn test_permission_denied_directory() {
    // Skip test if running as root (no permission denied for root)
    // Check if we're root by trying to read a protected file
    if std::env::var("USER").unwrap_or_default() == "root" {
        eprintln!("Skipping permission-denied test when running as root.");
        return;
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    // Use unique directory name to avoid conflicts
    let restricted_dir = &temp_dir.join(format!("restricted_{}", std::process::id()));

    // Clean up any existing directory first
    let _ = fs::remove_dir_all(restricted_dir);
    fs::create_dir(restricted_dir).unwrap();

    // Create file in directory
    let file_path = restricted_dir.join("file.pl");
    fs::write(&file_path, "print 'test';").unwrap();

    // Remove read permission from directory
    let mut perms = fs::metadata(restricted_dir).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(restricted_dir, perms.clone()).unwrap();

    let uri = format!("file://{}", file_path.display());

    // Try to access file in restricted directory
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 0}
            }
        }),
    );

    // Restore permissions for cleanup
    perms.set_mode(0o755);
    fs::set_permissions(restricted_dir, perms).unwrap();

    let response = read_response(&mut server);
    assert!(response.is_object());

    // Clean up directory
    let _ = fs::remove_dir_all(restricted_dir);
}

#[test]
#[cfg(windows)]
fn test_permission_denied_directory() {
    // Windows permission handling is different, skip for now
    eprintln!("Skipping Unix-specific permission test on Windows");
}

#[test]
#[cfg(unix)]
fn test_symlink_loop() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    // Use unique names to avoid conflicts
    let link1 = &temp_dir.join(format!("loop_a_{}.pl", std::process::id()));
    let link2 = &temp_dir.join(format!("loop_b_{}.pl", std::process::id()));

    // Remove any existing links first
    let _ = fs::remove_file(link1);
    let _ = fs::remove_file(link2);

    // Create symlink loop
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(link2, link1).unwrap();
        std::os::unix::fs::symlink(link1, link2).unwrap();
    }

    let uri = format!("file://{}", link1.display());

    // Try to open symlink loop
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'loop';"
                }
            }
        }),
    );

    // Should handle without infinite loop
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());

    // Clean up symlinks
    let _ = fs::remove_file(link1);
    let _ = fs::remove_file(link2);
}

#[test]
#[cfg(windows)]
fn test_symlink_loop() {
    // Windows symlink handling requires admin privileges, skip for now
    eprintln!("Skipping Unix-specific symlink test on Windows");
}

#[test]
fn test_broken_symlink() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    let target = &temp_dir.join("target.pl");
    let link = &temp_dir.join("link.pl");

    // Create file and symlink
    fs::write(target, "print 'target';").unwrap();
    // Remove any existing symlink first to make test idempotent
    let _ = fs::remove_file(link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link).unwrap();

    // Delete target, leaving broken symlink
    fs::remove_file(target).unwrap();

    let uri = format!("file://{}", link.display());

    // Try to open broken symlink
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'broken';"
                }
            }
        }),
    );

    // Should handle gracefully
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 0}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_very_long_path() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create extremely long path (may exceed PATH_MAX on some systems)
    let mut long_path = String::from("file:///");
    for _ in 0..500 {
        long_path.push_str("very_long_directory_name/");
    }
    long_path.push_str("file.pl");

    // Try to open file with very long path
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": long_path,
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'long path';"
                }
            }
        }),
    );

    // Should handle gracefully
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": long_path}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_special_filename_characters() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();

    // Test various special characters in filenames
    let special_names = vec![
        "file with spaces.pl",
        "file\twith\ttabs.pl",
        "file\nwith\nnewlines.pl",
        "file[brackets].pl",
        "file{braces}.pl",
        "file$dollar.pl",
        "file#hash.pl",
        "file%percent.pl",
        "file&ampersand.pl",
        "file*asterisk.pl",
        "file?question.pl",
        "file|pipe.pl",
        "file<less.pl",
        "file>greater.pl",
        "file\"quote.pl",
        "file'apostrophe.pl",
        "file\\backslash.pl",
        "file`backtick.pl",
        "file~tilde.pl",
        "file!exclamation.pl",
        "file@at.pl",
        "file^caret.pl",
        "file=equals.pl",
        "file+plus.pl",
        "file,comma.pl",
        "file.multiple.dots.pl",
        "émoji🎉file.pl",
        "中文文件.pl",
        "файл.pl",
        "αρχείο.pl",
    ];

    for name in special_names {
        // Skip names with characters that can't be in filenames
        if name.contains('\0') || name.contains('/') || name.contains('\n') {
            continue;
        }

        let file_path = &temp_dir.join(name);

        // Try to create file (may fail on some filesystems)
        if fs::write(file_path, "print 'special';").is_ok() {
            let uri = format!("file://{}", file_path.display());

            send_notification(
                &mut server,
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "perl",
                            "version": 1,
                            "text": "print 'special';"
                        }
                    }
                }),
            );
        }
    }
}

#[test]
fn test_case_sensitive_filesystem() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    let file_lower = &temp_dir.join("test.pl");
    let file_upper = &temp_dir.join("TEST.pl");

    fs::write(file_lower, "print 'lowercase';").unwrap();

    // Check if filesystem is case-sensitive
    let is_case_sensitive = !file_upper.exists();

    if is_case_sensitive {
        fs::write(file_upper, "print 'uppercase';").unwrap();
    }

    // Open with different case
    let uri_lower = format!("file://{}", file_lower.display());
    let uri_upper = format!("file://{}", file_upper.display());

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri_lower,
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'lowercase';"
                }
            }
        }),
    );

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri_upper,
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'uppercase';"
                }
            }
        }),
    );

    // Should handle both files correctly
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri_lower}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());
}

#[test]
fn test_file_deleted_while_open() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    let file_path = &temp_dir.join("delete_me.pl");
    fs::write(file_path, "print 'delete me';").unwrap();

    let uri = format!("file://{}", file_path.display());

    // Open file
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri.clone(),
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'delete me';"
                }
            }
        }),
    );

    // Delete file while it's open
    fs::remove_file(file_path).unwrap();

    // Try to perform operations on deleted file
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri.clone()},
                "position": {"line": 0, "character": 0}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());

    // Try to save
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": {
                "textDocument": {"uri": uri},
                "text": "print 'saved';"
            }
        }),
    );
}

#[test]
fn test_file_modified_externally() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    let file_path = &temp_dir.join("external.pl");
    fs::write(file_path, "print 'original';").unwrap();

    let uri = format!("file://{}", file_path.display());

    // Open file
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri.clone(),
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'original';"
                }
            }
        }),
    );

    // Modify file externally
    fs::write(file_path, "print 'modified externally';").unwrap();

    // Server state may be out of sync
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri.clone()}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response.is_object());

    // Notify of external change
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 2
                },
                "contentChanges": [{
                    "text": "print 'modified externally';"
                }]
            }
        }),
    );
}

#[test]
fn test_workspace_folder_deleted() {
    let mut server = start_lsp_server();

    let temp_dir = std::env::temp_dir();
    let workspace_path = &temp_dir.to_path_buf();

    // Initialize with workspace folder
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": format!("file://{}", workspace_path.display()),
                "capabilities": {},
                "workspaceFolders": [{
                    "uri": format!("file://{}", workspace_path.display()),
                    "name": "test"
                }]
            }
        }),
    );

    eprintln!("Initialize response: {:?}", response);
    assert!(response["result"].is_object());

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    // Delete workspace folder
    // Note: We can't actually delete temp_dir while we're using it
    // Instead, simulate by removing workspace folder via LSP
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeWorkspaceFolders",
            "params": {
                "event": {
                    "added": [],
                    "removed": [{
                        "uri": format!("file://{}", workspace_path.display()),
                        "name": "test"
                    }]
                }
            }
        }),
    );

    // Try to perform workspace operations
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "workspace/symbol",
            "params": {
                "query": "test"
            }
        }),
    );

    let response = read_response(&mut server);
    // Should return an array (possibly empty) or null
    assert!(response.is_object());
    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_hidden_files() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    let hidden_file = &temp_dir.join(".hidden.pl");
    fs::write(hidden_file, "print 'hidden';").unwrap();

    let uri = format!("file://{}", hidden_file.display());

    // Open hidden file
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'hidden';"
                }
            }
        }),
    );

    // Should work normally
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": uri}
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_device_files() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try to open device files (Linux specific)
    let device_files = vec!["/dev/null", "/dev/zero", "/dev/random", "/dev/urandom"];

    for device in device_files {
        if PathBuf::from(device).exists() {
            let uri = format!("file://{}", device);

            send_notification(
                &mut server,
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "perl",
                            "version": 1,
                            "text": ""
                        }
                    }
                }),
            );
        }
    }
}

#[test]
fn test_fifo_pipe() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let temp_dir = std::env::temp_dir();
    let fifo_path = &temp_dir.join("pipe.pl");

    // Create FIFO (named pipe)
    let _ = std::process::Command::new("mkfifo").arg(fifo_path).output();

    if fifo_path.exists() {
        let uri = format!("file://{}", fifo_path.display());

        // Try to open FIFO
        send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "perl",
                        "version": 1,
                        "text": "print 'fifo';"
                    }
                }
            }),
        );

        // Should handle special file type
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": 0, "character": 0}
                }
            }),
        );

        let response = read_response(&mut server);
        assert!(response.is_object());
    }
}
