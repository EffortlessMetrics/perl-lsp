//! Tests for comprehensive built-in function signatures

use perl_parser::{Parser, SignatureHelpProvider};

#[test]
fn test_comprehensive_builtin_coverage() {
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);
    
    // Test that we have signatures for all major built-in functions
    let functions = vec![
        // String functions
        ("chomp(", "chomp"),
        ("chop(", "chop"),
        ("chr(", "chr"),
        ("ord(", "ord"),
        ("hex(", "hex"),
        ("oct(", "oct"),
        ("length(", "length"),
        ("lc(", "lc"),
        ("lcfirst(", "lcfirst"),
        ("uc(", "uc"),
        ("ucfirst(", "ucfirst"),
        ("quotemeta(", "quotemeta"),
        ("reverse(", "reverse"),
        ("index(", "index"),
        ("rindex(", "rindex"),
        ("sprintf(", "sprintf"),
        
        // Array functions
        ("shift(", "shift"),
        ("unshift(", "unshift"),
        ("splice(", "splice"),
        
        // Hash functions
        ("each(", "each"),
        ("keys(", "keys"),
        ("values(", "values"),
        
        // I/O functions
        ("say(", "say"),
        ("read(", "read"),
        ("sysread(", "sysread"),
        ("write(", "write"),
        ("syswrite(", "syswrite"),
        ("seek(", "seek"),
        ("tell(", "tell"),
        ("eof(", "eof"),
        
        // File operations
        ("stat(", "stat"),
        ("lstat(", "lstat"),
        ("chmod(", "chmod"),
        ("chown(", "chown"),
        ("link(", "link"),
        ("symlink(", "symlink"),
        ("readlink(", "readlink"),
        ("rename(", "rename"),
        ("unlink(", "unlink"),
        ("mkdir(", "mkdir"),
        ("rmdir(", "rmdir"),
        
        // Directory functions
        ("opendir(", "opendir"),
        ("readdir(", "readdir"),
        ("closedir(", "closedir"),
        ("rewinddir(", "rewinddir"),
        ("telldir(", "telldir"),
        ("seekdir(", "seekdir"),
        
        // Process functions
        ("fork(", "fork"),
        ("wait(", "wait"),
        ("waitpid(", "waitpid"),
        ("kill(", "kill"),
        ("getpid(", "getpid"),
        ("getppid(", "getppid"),
        
        // Time functions
        ("time(", "time"),
        ("localtime(", "localtime"),
        ("gmtime(", "gmtime"),
        ("sleep(", "sleep"),
        ("alarm(", "alarm"),
        
        // Math functions
        ("abs(", "abs"),
        ("atan2(", "atan2"),
        ("cos(", "cos"),
        ("sin(", "sin"),
        ("exp(", "exp"),
        ("log(", "log"),
        ("sqrt(", "sqrt"),
        ("int(", "int"),
        ("rand(", "rand"),
        ("srand(", "srand"),
        
        // Type functions
        ("scalar(", "scalar"),
        ("wantarray(", "wantarray"),
        
        // Control flow
        ("die(", "die"),
        ("warn(", "warn"),
        ("exit(", "exit"),
        ("return(", "return"),
        ("next(", "next"),
        ("last(", "last"),
        ("redo(", "redo"),
        ("goto(", "goto"),
        
        // Module functions
        ("require(", "require"),
        ("use(", "use"),
        ("no(", "no"),
        ("import(", "import"),
        ("unimport(", "unimport"),
        
        // Package functions
        ("package(", "package"),
        ("caller(", "caller"),
        
        // Eval and do
        ("eval(", "eval"),
        ("do(", "do"),
        
        // Tied variables
        ("tie(", "tie"),
        ("tied(", "tied"),
        ("untie(", "untie"),
        
        // Socket functions
        ("socket(", "socket"),
        ("bind(", "bind"),
        ("listen(", "listen"),
        ("accept(", "accept"),
        ("connect(", "connect"),
        ("shutdown(", "shutdown"),
        ("send(", "send"),
        ("recv(", "recv"),
        
        // Pack/unpack
        ("pack(", "pack"),
        ("unpack(", "unpack"),
        
        // Regular expression
        ("study(", "study"),
        ("pos(", "pos"),
        ("reset(", "reset"),
        
        // Miscellaneous
        ("dump(", "dump"),
        ("vec(", "vec"),
        ("prototype(", "prototype"),
        ("lock(", "lock"),
    ];
    
    let mut missing = Vec::new();
    
    for (code, func_name) in functions {
        let help = provider.get_signature_help(code, code.len() - 1);
        if help.is_none() {
            missing.push(func_name);
        }
    }
    
    assert!(missing.is_empty(), "Missing signatures for: {:?}", missing);
}

#[test]
fn test_string_function_signatures() {
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);
    
    // Test chomp signature
    let code = "chomp(";
    let help = provider.get_signature_help(code, code.len() - 1).unwrap();
    assert!(!help.signatures.is_empty());
    let sig = &help.signatures[0];
    assert!(sig.label.contains("chomp"));
    assert!(sig.documentation.is_some());
    
    // Test substr with multiple parameters
    let code = "substr($str, 5, ";
    let help = provider.get_signature_help(code, code.len() - 1).unwrap();
    assert_eq!(help.active_parameter, Some(2)); // Third parameter
}

#[test]
fn test_io_function_signatures() {
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);
    
    // Test say (Perl 5.10+)
    let code = "say(";
    let help = provider.get_signature_help(code, code.len() - 1).unwrap();
    assert!(!help.signatures.is_empty());
    assert!(help.signatures[0].label.contains("say"));
    
    // Test sysread
    let code = "sysread($fh, $buf, ";
    let help = provider.get_signature_help(code, code.len() - 1).unwrap();
    assert_eq!(help.active_parameter, Some(2));
}

#[test]
fn test_math_function_signatures() {
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);
    
    // Test atan2 which takes two parameters
    let code = "atan2($y, ";
    let help = provider.get_signature_help(code, code.len() - 1).unwrap();
    assert!(!help.signatures.is_empty());
    assert!(help.signatures[0].label.contains("atan2"));
    assert_eq!(help.active_parameter, Some(1));
}

#[test]
fn test_socket_function_signatures() {
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);
    
    // Test socket function
    let code = "socket($sock, $domain, ";
    let help = provider.get_signature_help(code, code.len() - 1).unwrap();
    assert!(!help.signatures.is_empty());
    assert!(help.signatures[0].label.contains("socket"));
}

#[test]
fn test_tied_variable_signatures() {
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);
    
    // Test tie function
    let code = "tie(%hash, 'Tie::File', ";
    let help = provider.get_signature_help(code, code.len() - 1).unwrap();
    assert!(!help.signatures.is_empty());
    assert!(help.signatures[0].label.contains("tie"));
}