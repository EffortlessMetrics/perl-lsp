//! Comprehensive tests for I/O and file operations
//! 
//! These tests validate complex interactions between I/O operations
//! including readline, diamond operator, filehandle operations, and error handling.

use perl_parser::{Parser, ast::{Node, NodeKind}};

/// Test readline with complex file processing and error handling
#[test]
fn test_readline_complex_file_processing() {
    let code = r#"
# Simple readline operations
my $line = <STDIN>;
my @lines = <STDIN>;

# Readline with filehandles
open my $fh, '<', 'input.txt' or die "Cannot open input.txt: $!";
my $file_line = <$fh>;
my @file_lines = <$fh>;
close $fh;

# Complex readline processing with error handling
sub process_file_lines {
    my ($filename) = @_;
    
    open my $input_fh, '<', $filename or do {
        warn "Cannot open $filename: $!";
        return;
    };
    
    my @processed_lines;
    my $line_number = 0;
    
    while (my $line = <$input_fh>) {
        $line_number++;
        
        # Skip empty lines and comments
        next if $line =~ /^\s*$/;
        next if $line =~ /^\s*#/;
        
        # Process the line
        chomp $line;
        $line =~ s/^\s+|\s+$//g;
        
        # Handle different line types
        if ($line =~ /^(\w+)\s*=\s*(.+)$/) {
            my ($key, $value) = ($1, $2);
            push @processed_lines, "$key=$value";
        } elsif ($line =~ /^\[(.+)\]$/) {
            my $section = $1;
            push @processed_lines, "SECTION:$section";
        } elsif ($line =~ /^include\s+(.+)$/) {
            my $include_file = $1;
            my @included_lines = process_file_lines($include_file);
            push @processed_lines, @included_lines;
        } else {
            warn "Invalid line format at line $line_number: $line";
        }
    }
    
    close $input_fh;
    return @processed_lines;
}

# Readline with complex data structures
sub read_config_file {
    my ($filename) = @_;
    
    open my $config_fh, '<', $filename or return {};
    
    my %config;
    my $current_section;
    
    while (my $line = <$config_fh>) {
        chomp $line;
        $line =~ s/^\s+|\s+$//g;
        
        # Skip comments and empty lines
        next if $line =~ /^\s*#/;
        next if $line =~ /^\s*$/;
        
        # Section header
        if ($line =~ /^\[([^\]]+)\]\s*$/) {
            $current_section = $1;
            $config{$current_section} = {};
        }
        # Key-value pair
        elsif ($line =~ /^([^=]+)\s*=\s*(.+)$/) {
            my ($key, $value) = ($1, $2);
            $value =~ s/^["']|["']$//g; # Remove quotes
            
            if ($current_section) {
                $config{$current_section}{$key} = $value;
            } else {
                $config{$key} = $value;
            }
        }
    }
    
    close $config_fh;
    return \%config;
}

# Readline with validation and transformation
sub read_and_validate_data {
    my ($filename, $validator) = @_;
    
    open my $data_fh, '<', $filename or die "Cannot open $filename: $!";
    
    my @valid_records;
    my $line_num = 0;
    
    while (my $line = <$data_fh>) {
        $line_num++;
        chomp $line;
        
        # Parse CSV-like format
        my @fields = split /\s*,\s*/, $line;
        
        # Validate fields
        if ($validator->(\@fields, $line_num)) {
            # Transform data
            my $record = {
                id => $fields[0] // '',
                name => $fields[1] // '',
                value => $fields[2] // 0,
                line_number => $line_num,
                timestamp => time()
            };
            
            push @valid_records, $record;
        }
    }
    
    close $data_fh;
    return \@valid_records;
}

# Readline with buffering and performance optimization
sub read_large_file_efficiently {
    my ($filename, $buffer_size) = @_;
    $buffer_size ||= 8192;
    
    open my $large_fh, '<', $filename or die "Cannot open $filename: $!";
    
    my @chunks;
    my $buffer = '';
    
    while (my $bytes_read = read($large_fh, $buffer, $buffer_size)) {
        if ($bytes_read == 0) {
            last; # EOF
        }
        
        # Process buffer in chunks
        push @chunks, $buffer;
        $buffer = '';
    }
    
    close $large_fh;
    return \@chunks;
}

# Test the functions
my @processed = process_file_lines('config.txt');
my $config = read_config_file('settings.ini');
my $validator = sub {
    my ($fields, $line_num) = @_;
    return @$fields >= 3 && $fields[0] =~ /^\d+$/;
};
my $records = read_and_validate_data('data.csv', $validator);
my $chunks = read_large_file_efficiently('largefile.dat', 16384);
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify readline operations
    let readline_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Readline { .. }));
    assert!(!readline_nodes.is_empty(), "Should have readline operations");
    
    // Verify file operations (open, close)
    let function_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::FunctionCall { .. }));
    assert!(!function_calls.is_empty(), "Should have function calls for file operations");
    
    // Verify error handling (die, warn)
    let unary_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Unary { .. }));
    assert!(!unary_ops.is_empty(), "Should have unary operations for die/warn");
    
    // Verify conditional statements
    let if_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::If { .. }));
    assert!(!if_nodes.is_empty(), "Should have conditional statements");
    
    // Verify regex operations
    let match_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Match { .. }));
    assert!(!match_ops.is_empty(), "Should have match operations");
    
    // Verify substitution operations
    let substitution_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Substitution { .. }));
    assert!(!substitution_ops.is_empty(), "Should have substitution operations");
}

/// Test diamond operator in various contexts
#[test]
fn test_diamond_operator_various_contexts() {
    let code = r#"
# Simple diamond operator
my @all_args = <>;
my $next_arg = <>;

# Diamond operator in loops
while (my $filename = <>) {
    process_file($filename);
}

# Diamond operator with conditional processing
while (my $line = <>) {
    if ($line =~ /^#!/) {
        # Skip shebang lines
        next;
    }
    
    if ($line =~ /^\s*#/) {
        # Skip comments
        next;
    }
    
    if ($line =~ /^\s*$/) {
        # Skip empty lines
        next;
    }
    
    # Process the line
    chomp $line;
    print "Processing: $line\n";
}

# Diamond operator in complex data processing
sub process_input_streams {
    my ($processor) = @_;
    
    my @results;
    
    while (my $item = <>) {
        my $processed;
        
        # Handle different input types
        if (-f $item) {
            # File input
            $processed = $processor->process_file($item);
        } elsif ($item =~ /^\d+$/) {
            # Numeric input
            $processed = $processor->process_number($item);
        } elsif ($item =~ /^[a-zA-Z]+$/) {
            # String input
            $processed = $processor->process_string($item);
        } else {
            # Complex input
            $processed = $processor->process_complex($item);
        }
        
        push @results, $processed;
    }
    
    return \@results;
}

# Diamond operator with error handling and logging
sub safe_diamond_processing {
    my ($options) = @_;
    
    my $error_count = 0;
    my $success_count = 0;
    
    while (my $input = <>) {
        eval {
            # Validate input
            if (!defined $input) {
                die "Undefined input";
            }
            
            if (length $input > $options->{max_length}) {
                die "Input too long: " . length($input);
            }
            
            # Process input
            my $result = process_input($input, $options);
            $success_count++;
            
            # Log success
            if ($options->{verbose}) {
                print STDERR "Processed: $input -> $result\n";
            }
            
        };
        
        if ($@) {
            $error_count++;
            warn "Error processing '$input': $@";
            
            # Continue processing other inputs
            next;
        }
    }
    
    return {
        success_count => $success_count,
        error_count => $error_count,
        total_processed => $success_count + $error_count
    };
}

# Diamond operator with filtering and transformation
sub filter_and_transform_input {
    my ($filters, $transformers) = @_;
    
    my @filtered_results;
    
    INPUT_ITEM:
    while (my $item = <>) {
        # Apply filters
        for my $filter (@$filters) {
            if (!$filter->($item)) {
                next INPUT_ITEM;
            }
        }
        
        # Apply transformers
        my $transformed = $item;
        for my $transformer (@$transformers) {
            $transformed = $transformer->($transformed);
        }
        
        push @filtered_results, $transformed;
    }
    
    return \@filtered_results;
}

# Diamond operator with parallel processing simulation
sub parallel_diamond_processing {
    my ($worker_count, $process_func) = @_;
    
    my @workers;
    my @results;
    
    # Create worker processes (simulated)
    for my $i (1..$worker_count) {
        push @workers, sub {
            my @local_results;
            
            while (my $item = <>) {
                my $result = $process_func->($item, $i);
                push @local_results, $result;
            }
            
            return \@local_results;
        };
    }
    
    # Collect results from all workers
    for my $worker (@workers) {
        my $worker_results = $worker->();
        push @results, @$worker_results;
    }
    
    return \@results;
}

# Test diamond operator usage
my @simple_args = <>;
my $processor = InputProcessor->new();
my $results = process_input_streams($processor);
my $stats = safe_diamond_processing({max_length => 1000, verbose => 1});

my $filters = [
    sub { $_[0] =~ /^\d+$/ },
    sub { length $_[0] > 5 }
];

my $transformers = [
    sub { uc $_[0] },
    sub { "PROCESSED: $_[0]" }
];

my $filtered = filter_and_transform_input($filters, $transformers);

package InputProcessor;
sub new {
    return bless {}, shift;
}

sub process_file {
    my ($self, $filename) = @_;
    return "FILE:$filename";
}

sub process_number {
    my ($self, $number) = @_;
    return "NUM:$number";
}

sub process_string {
    my ($self, $string) = @_;
    return "STR:$string";
}

sub process_complex {
    my ($self, $complex) = @_;
    return "COMPLEX:$complex";
}

sub process_input {
    my ($input, $options) = @_;
    return "PROCESSED:$input";
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify diamond operator usage
    let diamond_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Diamond));
    assert!(!diamond_nodes.is_empty(), "Should have diamond operator usage");
    
    // Verify while loops with diamond operator
    let while_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::While { .. }));
    assert!(!while_nodes.is_empty(), "Should have while loops");
    
    // Verify subroutine declarations
    let sub_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Subroutine { .. }));
    assert!(!sub_nodes.is_empty(), "Should have subroutine declarations");
    
    // Verify method calls
    let method_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MethodCall { .. }));
    assert!(!method_calls.is_empty(), "Should have method calls");
    
    // Verify array literals
    let array_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::ArrayLiteral { .. }));
    assert!(!array_literals.is_empty(), "Should have array literals");
}

/// Test filehandle operations with different modes and error conditions
#[test]
fn test_filehandle_operations_modes_errors() {
    let code = r#"
# Basic filehandle operations
open my $read_fh, '<', 'input.txt' or die "Cannot open input.txt: $!";
open my $write_fh, '>', 'output.txt' or die "Cannot open output.txt: $!";
open my $append_fh, '>>', 'log.txt' or die "Cannot open log.txt: $!";

# Filehandle with complex modes
open my $read_write_fh, '+<', 'data.txt' or die "Cannot open data.txt for read/write: $!";
open my $update_fh, '+>', 'update.txt' or die "Cannot open update.txt for update: $!";
open my $append_read_fh, '+>>', 'append.txt' or die "Cannot open append.txt: $!";

# Filehandle with piping
open my $pipe_fh, '|-', 'gzip -c > output.gz' or die "Cannot open pipe: $!";

# Filehandle with binary mode
open my $binary_fh, '<:raw', 'binary.dat' or die "Cannot open binary file: $!";
open my $binary_write_fh, '>:raw', 'binary_out.dat' or die "Cannot open binary output: $!";

# Filehandle with encoding
open my $utf8_fh, '<:encoding(UTF-8)', 'utf8.txt' or die "Cannot open UTF-8 file: $!";
open my $latin1_fh, '<:encoding(latin1)', 'latin1.txt' or die "Cannot open Latin-1 file: $!";

# Complex filehandle operations with error handling
sub safe_file_operation {
    my ($filename, $mode, $operation) = @_;
    
    my $fh;
    my $retry_count = 0;
    my $max_retries = 3;
    
    while ($retry_count < $max_retries) {
        eval {
            open $fh, $mode, $filename or die "Cannot open $filename: $!";
            $retry_count = $max_retries; # Success, exit loop
        };
        
        if ($@) {
            $retry_count++;
            warn "Attempt $retry_count failed: $@";
            sleep(1) if $retry_count < $max_retries;
        }
    }
    
    if (!$fh) {
        die "Failed to open $filename after $max_retries attempts";
    }
    
    # Perform the operation
    my $result;
    eval {
        $result = $operation->($fh);
    };
    
    my $error = $@;
    
    # Close filehandle
    eval { close $fh; };
    
    if ($error) {
        die "Operation failed: $error";
    }
    
    return $result;
}

# Filehandle with buffered operations
sub buffered_file_write {
    my ($filename, $data_ref) = @_;
    
    open my $buffered_fh, '>', $filename or die "Cannot open $filename: $!";
    
    my $buffer_size = 8192;
    my $buffer = '';
    
    for my $item (@$data_ref) {
        $buffer .= $item . "\n";
        
        # Flush buffer when full
        if (length $buffer >= $buffer_size) {
            print $buffered_fh $buffer;
            $buffer = '';
        }
    }
    
    # Flush remaining buffer
    if ($buffer) {
        print $buffered_fh $buffer;
    }
    
    close $buffered_fh;
    return 1;
}

# Filehandle with seek and tell operations
sub random_access_file {
    my ($filename) = @_;
    
    open my $random_fh, '+<', $filename or die "Cannot open $filename: $!";
    
    # Get file size
    seek $random_fh, 0, 2; # SEEK_END
    my $file_size = tell $random_fh;
    
    # Read random positions
    my @random_data;
    for my $i (1..10) {
        my $pos = int(rand($file_size));
        seek $random_fh, $pos, 0; # SEEK_SET
        
        my $line = <$random_fh>;
        chomp $line if defined $line;
        push @random_data, {position => $pos, data => $line};
    }
    
    close $random_fh;
    return \@random_data;
}

# Filehandle with locking
sub exclusive_file_update {
    my ($filename, $update_data) = @_;
    
    open my $lock_fh, '+<', $filename or die "Cannot open $filename: $!";
    
    # Try to get exclusive lock
    if (!flock($lock_fh, 2)) { # LOCK_EX
        warn "Cannot get exclusive lock on $filename";
        return 0;
    }
    
    # Read current content
    seek $lock_fh, 0, 0; # SEEK_SET
    my $current_content = do { local $/; <$lock_fh> };
    
    # Apply update
    my $new_content = $update_data->($current_content);
    
    # Write new content
    seek $lock_fh, 0, 0; # SEEK_SET
    truncate $lock_fh, 0;
    print $lock_fh $new_content;
    
    # Release lock
    flock($lock_fh, 8); # LOCK_UN
    close $lock_fh;
    
    return 1;
}

# Filehandle with temporary files
sub temp_file_operations {
    my ($operations) = @_;
    
    my @temp_files;
    
    for my $op (@$operations) {
        # Create temporary file
        my $temp_fh;
        my $temp_filename = 'temp_' . time() . '_' . rand() . '.tmp';
        
        open $temp_fh, '>', $temp_filename or die "Cannot create temp file: $!";
        push @temp_files, {fh => $temp_fh, name => $temp_filename};
        
        # Perform operation
        $op->($temp_fh);
        
        close $temp_fh;
    }
    
    # Cleanup temporary files
    for my $temp_file (@temp_files) {
        unlink $temp_file->{name} or warn "Cannot delete temp file $temp_file->{name}: $!";
    }
    
    return scalar @temp_files;
}

# Test the filehandle operations
my $read_result = safe_file_operation('input.txt', '<', sub {
    my ($fh) = @_;
    my @lines = <$fh>;
    return scalar @lines;
});

buffered_file_write('output.txt', ['line1', 'line2', 'line3']);
my $random_data = random_access_file('data.txt');

exclusive_file_update('config.txt', sub {
    my ($content) = @_;
    return $content . "\n# Updated at " . scalar(localtime) . "\n";
});

temp_file_operations([
    sub { my ($fh) = @_; print $fh "Temp data 1\n"; },
    sub { my ($fh) = @_; print $fh "Temp data 2\n"; }
]);
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify function calls for file operations
    let function_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::FunctionCall { .. }));
    assert!(!function_calls.is_empty(), "Should have function calls for file operations");
    
    // Verify error handling (die, warn)
    let unary_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Unary { .. }));
    assert!(!unary_ops.is_empty(), "Should have unary operations for error handling");
    
    // Verify conditional statements
    let if_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::If { .. }));
    assert!(!if_nodes.is_empty(), "Should have conditional statements");
    
    // Verify eval blocks for error handling
    let eval_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Eval { .. }));
    assert!(!eval_nodes.is_empty(), "Should have eval blocks");
    
    // Verify subroutine declarations
    let sub_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Subroutine { .. }));
    assert!(!sub_nodes.is_empty(), "Should have subroutine declarations");
    
    // Verify method calls
    let method_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MethodCall { .. }));
    assert!(!method_calls.is_empty(), "Should have method calls");
}

/// Test complex I/O combinations with multiple streams
#[test]
fn test_complex_io_multiple_streams() {
    let code = r#"
# Multiple filehandle operations
open my $input_fh, '<', 'input.txt' or die "Cannot open input: $!";
open my $output_fh, '>', 'output.txt' or die "Cannot open output: $!";
open my $error_fh, '>', 'error.log' or die "Cannot open error log: $!";

# STDIN/STDOUT/STDERR operations
my $stdin_line = <STDIN>;
print STDOUT "Output to STDOUT\n";
print STDERR "Error to STDERR\n";

# Complex stream processing
sub process_multiple_streams {
    my ($input_file, $output_file, $error_file) = @_;
    
    open my $in, '<', $input_file or die "Cannot open input: $!";
    open my $out, '>', $output_file or die "Cannot open output: $!";
    open my $err, '>', $error_file or die "Cannot open error: $!";
    
    my $line_num = 0;
    my $error_count = 0;
    my $success_count = 0;
    
    while (my $line = <$in>) {
        $line_num++;
        
        eval {
            # Process line
            my $processed = process_line($line, $line_num);
            
            # Write to appropriate output
            if ($processed->{error}) {
                print $err "Line $line_num: $processed->{message}\n";
                $error_count++;
            } else {
                print $out $processed->{data};
                $success_count++;
            }
        };
        
        if ($@) {
            print $err "Fatal error on line $line_num: $@\n";
            $error_count++;
        }
    }
    
    close $in;
    close $out;
    close $err;
    
    return {
        lines_processed => $line_num,
        success_count => $success_count,
        error_count => $error_count
    };
}

# Pipe operations
sub pipe_operations {
    my ($command) = @_;
    
    open my $pipe_read, '-|', $command or die "Cannot open pipe: $!";
    open my $pipe_write, '|-', 'cat > pipe_output.txt' or die "Cannot open write pipe: $!";
    
    my @pipe_results;
    
    while (my $line = <$pipe_read>) {
        chomp $line;
        
        # Process pipe output
        my $result = process_pipe_line($line);
        print $pipe_write "$result\n";
        
        push @pipe_results, $result;
    }
    
    close $pipe_read;
    close $pipe_write;
    
    return \@pipe_results;
}

# Socket operations (simulated)
sub socket_like_operations {
    my ($host, $port) = @_;
    
    # Simulate socket connection with filehandle
    open my $socket_fh, '-|', "nc $host $port" or die "Cannot connect to $host:$port: $!";
    
    my @responses;
    
    # Send data and read responses
    for my $request (qw(HELLO STATUS QUIT)) {
        print $socket_fh "$request\n";
        
        # Read response with timeout
        eval {
            local $SIG{ALRM} = sub { die "Timeout\n" };
            alarm(5); # 5 second timeout
            
            my $response = <$socket_fh>;
            chomp $response if defined $response;
            
            alarm(0); # Cancel timeout
            
            push @responses, {request => $request, response => $response};
        };
        
        if ($@) {
            warn "Request '$request' failed: $@";
            last;
        }
    }
    
    close $socket_fh;
    return \@responses;
}

# Bidirectional communication
sub bidirectional_communication {
    my ($input_file, $output_file) = @_;
    
    # Open both files
    open my $read_fh, '<', $input_file or die "Cannot open $input_file: $!";
    open my $write_fh, '>', $output_file or die "Cannot open $output_file: $!";
    
    # Process with feedback loop
    my @conversation;
    
    while (my $input = <$read_fh>) {
        chomp $input;
        
        # Process input
        my $response = generate_response($input);
        
        # Write response
        print $write_fh "$response\n";
        
        push @conversation, {input => $input, output => $response};
        
        # Check for termination
        last if $input =~ /^(quit|exit|bye)$/i;
    }
    
    close $read_fh;
    close $write_fh;
    
    return \@conversation;
}

# Complex I/O with buffering and compression
sub buffered_compressed_io {
    my ($input_file, $output_file) = @_;
    
    # Open input file
    open my $in_fh, '<', $input_file or die "Cannot open input: $!";
    
    # Open compressed output
    open my $out_fh, '|-', "gzip -c > $output_file" or die "Cannot open compressed output: $!";
    
    my $buffer = '';
    my $buffer_size = 32768; # 32KB buffer
    
    while (my $bytes_read = read($in_fh, $buffer, $buffer_size)) {
        if ($bytes_read == 0) {
            last; # EOF
        }
        
        # Process buffer
        my $processed = process_buffer($buffer, $bytes_read);
        print $out_fh $processed;
        
        $buffer = '';
    }
    
    close $in_fh;
    close $out_fh;
    
    return 1;
}

# Test the complex I/O operations
my $stream_stats = process_multiple_streams('input.txt', 'output.txt', 'error.log');
my $pipe_results = pipe_operations('ls -la');
my $socket_results = socket_like_operations('example.com', 80);
my $conversation = bidirectional_communication('commands.txt', 'responses.txt');
buffered_compressed_io('large_input.txt', 'compressed_output.gz');

sub process_line {
    my ($line, $line_num) = @_;
    
    if ($line =~ /^\s*$/) {
        return {error => 0, data => ''};
    }
    
    if ($line =~ /error/i) {
        return {error => 1, message => "Error in line: $line"};
    }
    
    return {error => 0, data => uc($line)};
}

sub process_pipe_line {
    my ($line) = @_;
    return "PROCESSED: $line";
}

sub generate_response {
    my ($input) = @_;
    return "RE: $input";
}

sub process_buffer {
    my ($buffer, $bytes_read) = @_;
    return uc($buffer);
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify file operations
    let function_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::FunctionCall { .. }));
    assert!(!function_calls.is_empty(), "Should have function calls");
    
    // Verify readline operations
    let readline_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Readline { .. }));
    assert!(!readline_nodes.is_empty(), "Should have readline operations");
    
    // Verify typeglob operations for STDIN/STDOUT/STDERR
    let typeglob_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Typeglob { .. }));
    assert!(!typeglob_nodes.is_empty(), "Should have typeglob operations");
    
    // Verify subroutine declarations
    let sub_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Subroutine { .. }));
    assert!(!sub_nodes.is_empty(), "Should have subroutine declarations");
    
    // Verify eval blocks for error handling
    let eval_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Eval { .. }));
    assert!(!eval_nodes.is_empty(), "Should have eval blocks");
    
    // Verify method calls
    let method_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MethodCall { .. }));
    assert!(!method_calls.is_empty(), "Should have method calls");
}

/// Helper function to find nodes of specific kinds
fn find_nodes_of_kind<F>(node: &Node, predicate: F) -> Vec<&Node>
where
    F: Fn(&NodeKind) -> bool,
{
    let mut results = Vec::new();
    find_nodes_recursive(node, &predicate, &mut results);
    results
}

/// Recursive helper to find nodes matching predicate
fn find_nodes_recursive<'a, F>(node: &'a Node, predicate: &F, results: &mut Vec<&'a Node>)
where
    F: Fn(&NodeKind) -> bool,
{
    if predicate(&node.kind) {
        results.push(node);
    }
    
    // Recurse into child nodes based on node type
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                find_nodes_recursive(stmt, predicate, results);
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                find_nodes_recursive(stmt, predicate, results);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            find_nodes_recursive(expression, predicate, results);
        }
        NodeKind::VariableDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                find_nodes_recursive(init, predicate, results);
            }
        }
        NodeKind::VariableListDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                find_nodes_recursive(init, predicate, results);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            find_nodes_recursive(lhs, predicate, results);
            find_nodes_recursive(rhs, predicate, results);
        }
        NodeKind::Binary { left, right, .. } => {
            find_nodes_recursive(left, predicate, results);
            find_nodes_recursive(right, predicate, results);
        }
        NodeKind::Unary { operand, .. } => {
            find_nodes_recursive(operand, predicate, results);
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(then_expr, predicate, results);
            find_nodes_recursive(else_expr, predicate, results);
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(then_branch, predicate, results);
            for (_, branch) in elsif_branches {
                find_nodes_recursive(branch, predicate, results);
            }
            if let Some(else_branch) = else_branch {
                find_nodes_recursive(else_branch, predicate, results);
            }
        }
        NodeKind::While { condition, body, continue_block } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::For { init, condition, update, body, continue_block } => {
            if let Some(init) = init {
                find_nodes_recursive(init, predicate, results);
            }
            if let Some(cond) = condition {
                find_nodes_recursive(cond, predicate, results);
            }
            if let Some(upd) = update {
                find_nodes_recursive(upd, predicate, results);
            }
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::Foreach { variable, list, body, continue_block } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(list, predicate, results);
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::Try { body, catch_blocks, finally_block } => {
            find_nodes_recursive(body, predicate, results);
            for (_, catch_body) in catch_blocks {
                find_nodes_recursive(catch_body, predicate, results);
            }
            if let Some(final_body) = finally_block {
                find_nodes_recursive(final_body, predicate, results);
            }
        }
        NodeKind::Given { expr, body } => {
            find_nodes_recursive(expr, predicate, results);
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::When { condition, body } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Default { body } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Subroutine { body, .. } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Method { body, .. } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Class { body, name: _ } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::FunctionCall { args, name: _ } => {
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::MethodCall { object, args, .. } => {
            find_nodes_recursive(object, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::ArrayLiteral { elements } => {
            for element in elements {
                find_nodes_recursive(element, predicate, results);
            }
        }
        NodeKind::HashLiteral { pairs } => {
            for (key, value) in pairs {
                find_nodes_recursive(key, predicate, results);
                find_nodes_recursive(value, predicate, results);
            }
        }
        NodeKind::StatementModifier { statement, condition, .. } => {
            find_nodes_recursive(statement, predicate, results);
            find_nodes_recursive(condition, predicate, results);
        }
        NodeKind::LabeledStatement { statement, .. } => {
            find_nodes_recursive(statement, predicate, results);
        }
        NodeKind::Eval { block } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::Do { block } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::Return { value } => {
            if let Some(val) = value {
                find_nodes_recursive(val, predicate, results);
            }
        }
        NodeKind::LoopControl { .. } => {} // No children
        NodeKind::Tie { variable, package, args } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(package, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::Untie { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::Readline { .. } => {} // No complex children
        NodeKind::Diamond => {} // No children
        NodeKind::Glob { .. } => {} // No children
        NodeKind::Typeglob { .. } => {} // No children
        NodeKind::Number { .. } => {} // No children
        NodeKind::String { .. } => {} // No children
        NodeKind::Heredoc { .. } => {} // No children
        NodeKind::Undef => {} // No children
        NodeKind::Ellipsis => {} // No children
        NodeKind::Regex { .. } => {} // No children
        NodeKind::Match { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Substitution { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Transliteration { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Package { block, .. } => {
            if let Some(b) = block {
                find_nodes_recursive(b, predicate, results);
            }
        }
        NodeKind::Use { .. } => {} // No complex children
        NodeKind::No { .. } => {} // No complex children
        NodeKind::PhaseBlock { block, .. } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::DataSection { .. } => {} // No children
        NodeKind::Format { .. } => {} // No children
        NodeKind::Identifier { .. } => {} // No children
        NodeKind::Variable { .. } => {} // No children
        NodeKind::VariableWithAttributes { variable, .. } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::Prototype { .. } => {} // No children
        NodeKind::Signature { parameters } => {
            for param in parameters {
                find_nodes_recursive(param, predicate, results);
            }
        }
        NodeKind::MandatoryParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::OptionalParameter { variable, default_value } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(default_value, predicate, results);
        }
        NodeKind::SlurpyParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::NamedParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::IndirectCall { object, args, .. } => {
            find_nodes_recursive(object, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::Error { partial, .. } => {
            if let Some(p) = partial {
                find_nodes_recursive(p, predicate, results);
            }
        }
        NodeKind::MissingExpression | NodeKind::MissingStatement | 
        NodeKind::MissingIdentifier | NodeKind::MissingBlock => {} // No children
        NodeKind::UnknownRest => {} // No children
    }
}