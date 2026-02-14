//! Platform-Specific Edge Cases for Perl Parser
//!
//! This test suite validates parser behavior with platform-specific scenarios,
//! including different path separators, line endings, file system behaviors,
//! and environment-specific features.

use perl_parser::Parser;
use std::time::Duration;

/// Maximum parsing time for platform-specific tests
const MAX_PLATFORM_PARSE_TIME: Duration = Duration::from_secs(5);

/// Test different line ending styles
#[test]
fn test_line_ending_styles() {
    println!("Testing different line ending styles...");
    
    let test_cases = vec![
        // Unix/Linux style (LF)
        ("Unix LF", "my $x = 1;\nmy $y = 2;\nprint \"$x\\n\";\n"),
        
        // Windows style (CRLF)
        ("Windows CRLF", "my $x = 1;\r\nmy $y = 2;\r\nprint \"$x\\n\";\r\n"),
        
        // Classic Mac style (CR)
        ("Classic Mac CR", "my $x = 1;\rmy $y = 2;\rprint \"$x\\n\";\r"),
        
        // Mixed line endings
        ("Mixed LF/CRLF", "my $x = 1;\nmy $y = 2;\r\nprint \"$x\\n\";\n"),
        ("Mixed CRLF/CR", "my $x = 1;\r\nmy $y = 2;\rprint \"$x\\n\";\r\n"),
        ("Mixed all three", "my $x = 1;\nmy $y = 2;\r\nprint \"$x\\n\";\rmy $z = 3;\n"),
        
        // Multiple consecutive line endings
        ("Multiple LF", "my $x = 1;\n\nmy $y = 2;\n\n\nprint \"$x\\n\";\n"),
        ("Multiple CRLF", "my $x = 1;\r\n\r\nmy $y = 2;\r\n\r\n\r\nprint \"$x\\n\";\r\n"),
        ("Multiple CR", "my $x = 1;\r\rmy $y = 2;\r\r\rprint \"$x\\n\";\r"),
        
        // No final line ending
        ("No final LF", "my $x = 1;\nmy $y = 2;\nprint \"$x\\n\""),
        ("No final CRLF", "my $x = 1;\r\nmy $y = 2;\r\nprint \"$x\\n\""),
        ("No final CR", "my $x = 1;\rmy $y = 2;\rprint \"$x\\n\""),
        
        // Only line endings
        ("Only LF", "\n\n\n"),
        ("Only CRLF", "\r\n\r\n\r\n"),
        ("Only CR", "\r\r\r"),
        
        // Empty lines with different endings
        ("Empty lines LF", "my $x = 1;\n\nmy $y = 2;\n\nprint \"$x\\n\";\n"),
        ("Empty lines CRLF", "my $x = 1;\r\n\r\nmy $y = 2;\r\n\r\nprint \"$x\\n\";\r\n"),
        ("Empty lines CR", "my $x = 1;\r\rmy $y = 2;\r\rprint \"$x\\n\";\r"),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the AST is reasonable
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Line ending issues should be handled gracefully
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test different path separator styles
#[test]
fn test_path_separator_styles() {
    println!("Testing different path separator styles...");
    
    let test_cases = vec![
        // Unix/Linux paths
        ("Unix absolute path", r#"my $path = '/usr/bin/perl';"#),
        ("Unix relative path", r#"my $path = 'lib/Module.pm';"#),
        ("Unix deep path", r#"my $path = '/very/deep/nested/path/to/module.pm';"#),
        ("Unix home path", r#"my $path = '~/Documents/script.pl';"#),
        ("Unix current path", r#"my $path = './script.pl';"#),
        ("Unix parent path", r#"my $path = '../parent/script.pl';"#),
        
        // Windows paths
        ("Windows absolute path", r#"my $path = 'C:\\Perl\\bin\\perl.exe';"#),
        ("Windows UNC path", r#"my $path = '\\\\server\\share\\file.pl';"#),
        ("Windows relative path", r#"my $path = 'lib\\Module.pm';"#),
        ("Windows deep path", r#"my $path = 'C:\\very\\deep\\nested\\path\\to\\module.pm';"#),
        ("Windows current path", r#"my $path = '.\\script.pl';"#),
        ("Windows parent path", r#"my $path = '..\\parent\\script.pl';"#),
        
        // Mixed separators (common in cross-platform code)
        ("Mixed separators", r#"my $path = 'lib/Module\\Sub.pm';"#),
        ("Mixed deep path", r#"my $path = '/mixed\\path/with\\various/separators';"#),
        
        // Edge cases
        ("Double slash Unix", r#"my $path = '//double//slash//path';"#),
        ("Double backslash Windows", r#"my $path = 'C:\\\\double\\\\backslash\\\\path';"#),
        ("Trailing slash Unix", r#"my $path = '/path/with/trailing/slash/';"#),
        ("Trailing backslash Windows", r#"my $path = 'C:\\path\\with\\trailing\\backslash\\';"#),
        
        // Special path components
        ("Dot components Unix", r#"my $path = './current/./directory/./path';"#),
        ("Dot components Windows", r#"my $path = '.\\current\\.\\directory\\.\\path';"#),
        ("Dot-dot Unix", r#"my $path = '../../parent/relative/path';"#),
        ("Dot-dot Windows", r#"my $path = '..\\..\\parent\\relative\\path';"#),
        
        // Paths in file operations
        ("Unix open path", r#"open my $fh, '<', '/usr/local/config.txt' or die $!;"#),
        ("Windows open path", r#"open my $fh, '<', 'C:\\config.txt' or die $!;"#),
        ("Unix require path", r#"require '/full/path/to/module.pm';"#),
        ("Windows require path", r#"require 'C:\\full\\path\\to\\module.pm';"#),
        ("Unix do path", r#"do './script.pl' or die $!;"#),
        ("Windows do path", r#"do '.\\script.pl' or die $!;"#),
        
        // Paths in system calls
        ("Unix system path", r#"system '/usr/bin/perl', 'script.pl';"#),
        ("Windows system path", r#"system 'C:\\Perl\\bin\\perl.exe', 'script.pl';"#),
        ("Unix exec path", r#"exec '/usr/bin/perl', 'script.pl' if 0;"#),
        ("Windows exec path", r#"exec 'C:\\Perl\\bin\\perl.exe', 'script.pl' if 0;"#),
        
        // Paths in file tests
        ("Unix file test", r#"if (-f '/usr/bin/perl') { print 'Exists\n'; }"#),
        ("Windows file test", r#"if (-f 'C:\\Perl\\bin\\perl.exe') { print 'Exists\n'; }"#),
        ("Unix directory test", r#"if (-d '/usr/local/bin') { print 'Directory\n'; }"#),
        ("Windows directory test", r#"if (-d 'C:\\Perl\\bin') { print 'Directory\n'; }"#),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the path is preserved in the AST
                let sexp = ast.to_sexp();
                assert!(sexp.contains("string") || sexp.contains("file") || sexp.contains("open") || 
                       sexp.contains("require") || sexp.contains("system") || sexp.contains("exec"), 
                       "Path operation not found in AST for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Path separator issues should be handled gracefully
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test environment-specific behaviors
#[test]
fn test_environment_specific_behaviors() {
    println!("Testing environment-specific behaviors...");
    
    let test_cases = vec![
        // Environment variable access
        ("Unix env var", r#"my $path = $ENV{'PATH'};"#),
        ("Windows env var", r#"my $path = $ENV{'PATH'};"#),
        ("Unix specific env", r#"my $home = $ENV{'HOME'};"#),
        ("Windows specific env", r#"my $home = $ENV{'USERPROFILE'};"#),
        ("Unix shell", r#"my $shell = $ENV{'SHELL'};"#),
        ("Windows shell", r#"my $shell = $ENV{'COMSPEC'};"#),
        ("Unix temp", r#"my $temp = $ENV{'TMPDIR'} || '/tmp';"#),
        ("Windows temp", r#"my $temp = $ENV{'TEMP'} || 'C:\\temp';"#),
        
        // Environment variable modification
        ("Set env var", r#"$ENV{'PATH'} = '/new/path';"#),
        ("Append env var", r#"$ENV{'PATH'} .= ':/additional/path';"#),
        ("Windows env append", r#"$ENV{'PATH'} .= ';C:\\additional';"#),
        
        // Platform-specific code blocks
        ("Unix check", r#"if ($^O eq 'linux') { print 'Linux\n'; }"#),
        ("Windows check", r#"if ($^O eq 'MSWin32') { print 'Windows\n'; }"#),
        ("Mac check", r#"if ($^O eq 'darwin') { print 'macOS\n'; }"#),
        
        // File handle operations
        ("Unix STDIN", r#"while (<STDIN>) { print $_; }"#),
        ("Windows STDIN", r#"while (<STDIN>) { print $_; }"#),
        ("Unix STDOUT", r#"print STDOUT "Hello\n";"#),
        ("Windows STDOUT", r#"print STDOUT "Hello\n";"#),
        ("Unix STDERR", r#"print STDERR "Error\n";"#),
        ("Windows STDERR", r#"print STDERR "Error\n";"#),
        
        // Process operations
        ("Unix fork", r#"my $pid = fork(); if ($pid == 0) { exec 'ls'; }"#),
        ("Windows system", r#"system 'dir';"#),
        ("Unix pipe", r#"open my $pipe, '-|', 'ls', '-l' or die $!;"#),
        ("Windows pipe", r#"open my $pipe, '-|', 'dir' or die $!;"#),
        
        // Signal handling (Unix-specific)
        ("Unix signal", r#"$SIG{'INT'} = sub { print "Caught SIGINT\n"; exit; };"#),
        ("Unix alarm", r#"eval { alarm 5; sleep 10; }; if ($@) { print "Timed out\n"; }"#),
        
        // File permissions (Unix-specific)
        ("Unix chmod", r#"chmod 0755, 'script.pl';"#),
        ("Unix chown", r#"chown $uid, $gid, 'file.txt';"#),
        
        // Windows-specific operations
        ("Windows Win32", r#"use Win32; my $result = Win32::GetCwd();"#),
        ("Windows OLE", r#"use Win32::OLE; my $excel = Win32::OLE->new('Excel.Application');"#),
        
        // Cross-platform compatibility
        ("File::Spec", r#"use File::Spec; my $path = File::Spec->catfile('dir', 'file.txt');"#),
        ("File::Path", r#"use File::Path; mkpath('/some/dir');"#),
        ("Cwd", r#"use Cwd; my $cwd = getcwd();"#),
        
        // Temp file handling
        ("File::Temp", r#"use File::Temp; my $temp = File::Temp->new();"#),
        
        // Platform-specific constants
        ("Unix constants", r#"use Fcntl ':mode'; my $mode = S_IRUSR | S_IWUSR;"#),
        ("Windows constants", r#"use Win32::File ':DEFAULT'; my $attrs = FILE_ATTRIBUTE_READONLY;"#),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the operation is present in the AST
                let sexp = ast.to_sexp();
                assert!(!sexp.is_empty(), "AST should not be empty for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Environment-specific features might not be supported, but should fail gracefully
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test character encoding and BOM handling
#[test]
fn test_encoding_and_bom_handling() {
    println!("Testing encoding and BOM handling...");
    
    let test_cases = vec![
        // UTF-8 BOM
        ("UTF-8 BOM", "\u{FEFF}my $x = 1; print \"$x\\n\";\n"),
        ("UTF-8 BOM CRLF", "\u{FEFF}my $x = 1;\r\nprint \"$x\\n\";\r\n"),
        
        // UTF-16 BE BOM (simulated through escape sequences)
        ("UTF-16 BE BOM", r#"my $text = "\xFE\xFF";"#),
        ("UTF-16 LE BOM", r#"my $text = "\xFF\xFE";"#),
        
        // Different encodings in strings
        ("Latin-1 string", r#"my $text = "Café"; print "$text\n";"#),
        ("CP1252 string", r#"my $text = "Windows text"; print "$text\n";"#),
        
        // Encoding pragmas
        ("use utf8", r#"use utf8; my $text = "Unicode"; print "$text\n";"#),
        ("use encoding", r#"use encoding 'utf8'; my $text = "Unicode"; print "$text\n";"#),
        ("use bytes", r#"use bytes; my $text = "Bytes"; print "$text\n";"#),
        
        // Encoding in file operations
        ("UTF-8 file handle", r#"open my $fh, '<:encoding(UTF-8)', 'file.txt' or die $!;"#),
        ("Latin-1 file handle", r#"open my $fh, '<:encoding(Latin-1)', 'file.txt' or die $!;"#),
        ("UTF-16 file handle", r#"open my $fh, '<:encoding(UTF-16)', 'file.txt' or die $!;"#),
        
        // Encoding in binmode
        ("UTF-8 binmode", r#"binmode STDOUT, ':encoding(UTF-8)';"#),
        ("Latin-1 binmode", r#"binmode STDOUT, ':encoding(Latin-1)';"#),
        
        // Encoding in pack/unpack
        ("UTF-8 pack", r#"my $packed = pack('U*', 0x1F600, 0x1F601); print "$packed\n";"#),
        ("Unicode unpack", r#"my @chars = unpack('U*', '😀😁'); print "@chars\n";"#),
        
        // Platform-specific newline handling
        ("Unix newline", r#"my $text = "Unix\nline\nendings"; print "$text\n";"#),
        ("Windows newline", r#"my $text = "Windows\r\nline\r\nendings"; print "$text\n";"#),
        ("Mac newline", r#"my $text = "Mac\rline\rendings"; print "$text\n";"#),
        
        // Mixed newlines in strings
        ("Mixed newlines", r#"my $text = "Mixed\nline\r\nendings\rhere"; print "$text\n";"#),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the encoding operation is present in the AST
                let sexp = ast.to_sexp();
                assert!(sexp.contains("string") || sexp.contains("use") || sexp.contains("open") || 
                       sexp.contains("binmode") || sexp.contains("pack") || sexp.contains("unpack"), 
                       "Encoding operation not found in AST for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Encoding issues should be handled gracefully
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test platform-specific file system edge cases
#[test]
fn test_file_system_edge_cases() {
    println!("Testing file system edge cases...");
    
    let test_cases = vec![
        // Reserved filenames (Windows)
        ("Windows CON device", r#"open my $fh, '>', 'CON' or die $!;"#),
        ("Windows PRN device", r#"open my $fh, '>', 'PRN' or die $!;"#),
        ("Windows AUX device", r#"open my $fh, '>', 'AUX' or die $!;"#),
        ("Windows COM1 device", r#"open my $fh, '>', 'COM1' or die $!;"#),
        ("Windows LPT1 device", r#"open my $fh, '>', 'LPT1' or die $!;"#),
        
        // Long filenames
        ("Unix long filename", r#"open my $fh, '>', 'very_long_filename_that_exceeds_normal_limits_and_tests_parser_handling.txt' or die $!;"#),
        ("Windows long filename", r#"open my $fh, '>', 'very_long_filename_that_exceeds_normal_limits_and_tests_parser_handling.txt' or die $!;"#),
        
        // Special characters in filenames
        ("Unix special chars", r#"open my $fh, '>', 'file-with_special.chars@#$%^&*().txt' or die $!;"#),
        ("Windows special chars", r#"open my $fh, '>', 'file-with_special.chars().txt' or die $!;"#),
        
        // Unicode filenames
        ("Unicode filename", r#"open my $fh, '>', 'العربية.txt' or die $!;"#),
        ("Mixed Unicode filename", r#"open my $fh, '>', 'test_العربية_file.txt' or die $!;"#),
        
        // File system limits
        ("Deep directory", r#"mkdir 'very/deep/nested/directory/structure/that/tests/parser/handling/of/long/paths', 0755;"#),
        ("Many files", r#"for my $i (1..1000) { open my $fh, '>', \"file_$i.txt\" or die $!; }"#),
        
        // File locking
        ("Unix flock", r#"open my $fh, '>', 'test.txt'; flock $fh, LOCK_EX;"#),
        ("Windows locking", r#"open my $fh, '>', 'test.txt'; flock $fh, LOCK_EX;"#),
        
        // Symbolic links (Unix)
        ("Unix symlink", r#"symlink 'target.txt', 'link.txt' or die $!;"#),
        ("Unix readlink", r#"my $target = readlink 'link.txt'; print "$target\n";"#),
        
        // Hard links
        ("Unix hardlink", r#"link 'original.txt', 'hardlink.txt' or die $!;"#),
        
        // File permissions edge cases
        ("Unix permissions", r#"chmod 0777, 'file.txt'; chmod 0000, 'file.txt';"#),
        ("Unix sticky bit", r#"chmod 01777, 'directory';"#),
        ("Unix setuid/setgid", r#"chmod 04755, 'executable'; chmod 02775, 'directory';"#),
        
        // File timestamps
        ("Unix utime", r#"utime $atime, $mtime, 'file.txt';"#),
        ("Windows file time", r#"use Win32::File; Win32::File::SetFileTime('file.txt', $atime, $mtime, $ctime);"#),
        
        // Case sensitivity
        ("Unix case sensitive", r#"open my $fh, '>', 'File.txt'; open my $fh2, '>', 'file.txt';"#),
        ("Windows case insensitive", r#"open my $fh, '>', 'File.txt'; open my $fh2, '>', 'file.txt';"#),
        
        // Hidden files
        ("Unix hidden file", r#"open my $fh, '>', '.hidden_file' or die $!;"#),
        ("Windows hidden file", r#"open my $fh, '>', 'hidden_file.txt'; system 'attrib +H hidden_file.txt';"#),
        
        // Temporary files
        ("Unix temp", r#"open my $fh, '>', '/tmp/tempfile_$$' or die $!;"#),
        ("Windows temp", r#"open my $fh, '>', \"$ENV{TEMP}\\tempfile_$$\" or die $!;"#),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the file system operation is present in the AST
                let sexp = ast.to_sexp();
                assert!(sexp.contains("file") || sexp.contains("open") || sexp.contains("mkdir") || 
                       sexp.contains("symlink") || sexp.contains("link") || sexp.contains("chmod") || 
                       sexp.contains("utime") || sexp.contains("system"), 
                       "File system operation not found in AST for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // File system edge cases should be handled gracefully
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}

/// Test platform-specific networking edge cases
#[test]
fn test_networking_edge_cases() {
    println!("Testing networking edge cases...");
    
    let test_cases = vec![
        // Unix socket operations
        ("Unix socket", r#"use Socket; socket my $sock, PF_INET, SOCK_STREAM, getprotobyname('tcp');"#),
        ("Unix bind", r#"bind $sock, pack_sockaddr_in(8080, INADDR_ANY);"#),
        ("Unix listen", r#"listen $sock, SOMAXCONN;"#),
        ("Unix accept", r#"accept my $client, $sock;"#),
        
        // Windows networking
        ("Windows Winsock", r#"use Win32::API; my $WSAStartup = Win32::API->new('ws2_32', 'WSAStartup', 'IP', 'I');"#),
        
        // Cross-platform networking
        ("IO::Socket", r#"use IO::Socket::INET; my $sock = IO::Socket::INET->new(LocalPort => 8080, Listen => 1);"#),
        
        // Network addresses
        ("IPv4 address", r#"my $ip = '192.168.1.1';"#),
        ("IPv6 address", r#"my $ip = '2001:db8::1';"#),
        ("Unix socket path", r#"my $path = '/tmp/socket.sock';"#),
        
        // Hostnames
        ("Localhost", r#"my $host = 'localhost';"#),
        ("Fully qualified", r#"my $host = 'example.com';"#),
        ("International domain", r#"my $host = '例子.测试';"#),
        
        // Port numbers
        ("Well-known port", r#"my $port = 80;"#),
        ("High port", r#"my $port = 8080;"#),
        ("System port", r#"my $port = 22;"#),
        
        // Protocol-specific
        ("TCP socket", r#"use IO::Socket::INET; my $sock = IO::Socket::INET->new(Proto => 'tcp');"#),
        ("UDP socket", r#"use IO::Socket::INET; my $sock = IO::Socket::INET->new(Proto => 'udp');"#),
        
        // Network file operations
        ("Network file path", r#"open my $fh, '<', '//server/share/file.txt' or die $!;"#),
        ("UNC path", r#"my $path = '\\\\server\\share\\file.txt';"#),
        
        // URL handling
        ("HTTP URL", r#"my $url = 'http://example.com/path';"#),
        ("HTTPS URL", r#"my $url = 'https://example.com/path';"#),
        ("FTP URL", r#"my $url = 'ftp://user:pass@example.com/path';"#),
        ("File URL", r#"my $url = 'file:///path/to/file.txt';"#),
        
        // Network timeouts
        ("Socket timeout", r#"use IO::Socket; $IO::Socket::timeout = 30;"#),
        ("Alarm timeout", r#"eval { alarm 30; my $result = network_operation(); alarm 0; };"#),
    ];
    
    for (name, code) in test_cases {
        println!("Testing: {}", name);
        
        let start_time = std::time::Instant::now();
        let mut parser = Parser::new(code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Parse time exceeded limit for {}", name);
                
                // Verify the networking operation is present in the AST
                let sexp = ast.to_sexp();
                assert!(sexp.contains("use") || sexp.contains("string") || sexp.contains("variable"), 
                       "Networking operation not found in AST for {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to parse: {}", e);
                // Networking edge cases should be handled gracefully
                assert!(parse_time < MAX_PLATFORM_PARSE_TIME, "Error detection took too long for {}", name);
            }
        }
    }
}