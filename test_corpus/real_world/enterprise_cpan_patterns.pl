#!/usr/bin/env perl
# Enterprise/CPAN Patterns - Exporter, Pragmas, Signals, Dependencies
# Production-quality test file representing actual enterprise Perl practices

use strict;
use warnings;
use v5.20;
use feature 'signatures';

# Exporter Advanced Configurations
{
    package EnterpriseExample::AdvancedExporter;
    
    use strict;
    use warnings;
    use parent 'Exporter';
    use Carp qw(croak carp);
    use Scalar::Util qw(blessed reftype);
    use List::Util qw(first);
    
    our @ISA = qw(Exporter);
    
    # Export groups
    our %EXPORT_TAGS = (
        core => [qw(validate_data format_error log_message)],
        database => [qw(connect_db execute_query transaction)],
        utils => [qw(is_hashref is_arrayref is_coderef)],
        all => [qw(
            validate_data format_error log_message
            connect_db execute_query transaction
            is_hashref is_arrayref is_coderef
            create_timestamp generate_uuid
        )],
    );
    
    # Default exports
    our @EXPORT = qw(validate_data format_error log_message);
    
    # Optional exports
    our @EXPORT_OK = qw(
        connect_db execute_query transaction
        is_hashref is_arrayref is_coderef
        create_timestamp generate_uuid
        parse_config_file
    );
    
    # Version information
    our $VERSION = '2.1.0';
    
    # Configuration variables
    our %CONFIG = (
        log_level => 'info',
        db_timeout => 30,
        max_retries => 3,
    );
    
    # Core validation function
    sub validate_data {
        my ($data, $schema) = @_;
        
        croak "Data is required" unless defined $data;
        croak "Schema is required" unless defined $schema;
        
        # Type validation
        if (exists $schema->{type}) {
            my $expected_type = $schema->{type};
            
            if ($expected_type eq 'hash') {
                return 0 unless is_hashref($data);
            } elsif ($expected_type eq 'array') {
                return 0 unless is_arrayref($data);
            } elsif ($expected_type eq 'string') {
                return 0 unless !ref($data) && defined $data;
            } elsif ($expected_type eq 'number') {
                return 0 unless !ref($data) && defined $data && $data =~ /^-?\d+(?:\.\d+)?$/;
            } elsif ($expected_type eq 'boolean') {
                return 0 unless !ref($data) && defined($data) && ($data == 0 || $data == 1);
            }
        }
        
        # Required fields validation
        if (exists $schema->{required} && is_hashref($data)) {
            for my $field (@{$schema->{required}}) {
                return 0 unless exists $data->{$field};
            }
        }
        
        # Custom validation
        if (exists $schema->{validate} && is_coderef($schema->{validate})) {
            return 0 unless $schema->{validate}->($data);
        }
        
        return 1;
    }
    
    # Error formatting
    sub format_error {
        my ($message, $code, $context) = @_;
        
        my $error = {
            message => $message,
            code => $code || 'UNKNOWN_ERROR',
            timestamp => create_timestamp(),
        };
        
        $error->{context} = $context if $context;
        $error->{stack_trace} = Carp::longmess() if $CONFIG{include_stack_trace};
        
        return $error;
    }
    
    # Logging with levels
    sub log_message {
        my ($level, $message, $context) = @_;
        
        my %levels = (
            debug => 0,
            info => 1,
            warn => 2,
            error => 3,
            fatal => 4,
        );
        
        my $current_level = $levels{$CONFIG{log_level}} || 1;
        my $message_level = $levels{$level} || 1;
        
        return if $message_level < $current_level;
        
        my $timestamp = create_timestamp();
        my $pid = $$;
        my $log_line = "[$timestamp] [$level] [PID:$pid] $message";
        
        if ($context) {
            $log_line .= " " . (is_hashref($context) ? 
                join(', ', map { "$_=" . (defined $context->{$_} ? $context->{$_} : 'undef') } keys %$context) :
                $context);
        }
        
        warn "$log_line\n";
        
        # Send to external logging service if configured
        if ($CONFIG{external_logger}) {
            eval {
                $CONFIG{external_logger}->log($level, $message, $context);
            };
            carp "Failed to log to external service: $@" if $@;
        }
    }
    
    # Database functions
    sub connect_db {
        my ($dsn, $username, $password, $options) = @_;
        
        require DBI;
        
        my $dbh = DBI->connect($dsn, $username, $password, {
            RaiseError => 1,
            AutoCommit => 1,
            PrintError => 0,
            ShowErrorStatement => 1,
            AutoInactiveDestroy => 1,
            %{$options || {}},
        }) or croak "Cannot connect to database: $DBI::errstr";
        
        log_message('info', 'Database connection established', { dsn => $dsn });
        
        return $dbh;
    }
    
    sub execute_query {
        my ($dbh, $query, @params) = @_;
        
        croak "Database handle required" unless $dbh;
        croak "Query required" unless $query;
        
        my $sth = $dbh->prepare($query);
        $sth->execute(@params) or croak "Query execution failed: " . $dbh->errstr;
        
        return $sth;
    }
    
    sub transaction {
        my ($dbh, $code) = @_;
        
        croak "Database handle required" unless $dbh;
        croak "Code reference required" unless is_coderef($code);
        
        $dbh->begin_work;
        
        eval {
            my $result = $code->($dbh);
            $dbh->commit;
            return $result;
        };
        
        if ($@) {
            $dbh->rollback;
            croak "Transaction failed: $@";
        }
    }
    
    # Utility functions
    sub is_hashref {
        my ($ref) = @_;
        return reftype($ref) eq 'HASH';
    }
    
    sub is_arrayref {
        my ($ref) = @_;
        return reftype($ref) eq 'ARRAY';
    }
    
    sub is_coderef {
        my ($ref) = @_;
        return reftype($ref) eq 'CODE';
    }
    
    sub create_timestamp {
        my ($format) = @_;
        
        require POSIX;
        $format ||= '%Y-%m-%d %H:%M:%S';
        
        return POSIX::strftime($format, localtime);
    }
    
    sub generate_uuid {
        require Data::UUID;
        my $ug = Data::UUID->new();
        return $ug->create_str();
    }
    
    sub parse_config_file {
        my ($filename) = @_;
        
        croak "Config file required" unless $filename;
        
        my $config = {};
        
        if ($filename =~ /\.ya?ml$/i) {
            require YAML::XS;
            $config = YAML::XS::LoadFile($filename);
        } elsif ($filename =~ /\.json$/i) {
            require JSON;
            open my $fh, '<', $filename or croak "Cannot open config file: $!";
            my $content = do { local $/; <$fh> };
            close $fh;
            $config = JSON::decode_json($content);
        } else {
            # Simple key=value format
            open my $fh, '<', $filename or croak "Cannot open config file: $!";
            while (my $line = <$fh>) {
                chomp $line;
                next if $line =~ /^\s*#/ || $line =~ /^\s*$/;
                
                if ($line =~ /^\s*(\w+)\s*=\s*(.+?)\s*$/) {
                    my ($key, $value) = ($1, $2);
                    $value =~ s/^["']|["']$//g;  # Remove quotes
                    $config->{$key} = $value;
                }
            }
            close $fh;
        }
        
        return $config;
    }
}

# Advanced Pragma Usage Patterns
{
    package EnterpriseExample::PragmaUsage;
    
    use strict;
    use warnings;
    use v5.20;
    use feature 'signatures';
    no warnings 'experimental::signatures';
    
    # Enable warnings for specific categories
    use warnings qw(uninitialized void once);
    
    # Control integer operations
    use integer;
    
    # Enable floating point comparison warnings
    use warnings 'numeric';
    
    # Control recursion depth
    no warnings 'recursion';
    
    # Enable UTF-8 handling
    use utf8;
    use open ':std', ':encoding(UTF-8)';
    
    # Control constant folding
    use constant {
        DEBUG => 0,
        MAX_RETRIES => 3,
        TIMEOUT => 30,
        DEFAULT_PAGE_SIZE => 50,
    };
    
    # Control autovivification
    use autouse 'autovivification' => qw(autovivification);
    
    # Control array/has behavior
    use autouse 'autobox' => qw(autobox);
    
    # Control subroutine prototypes
    use subs qw(debug info warn error fatal);
    
    # Control namespace cleanup
    use namespace::clean;
    
    # Control method signatures
    use Method::Signatures;
    
    # Control class accessors
    use Class::Accessor::Fast qw(moose);
    
    # Control exception handling
    use Try::Tiny;
    
    # Control lazy loading
    use autouse 'Scalar::Util' => qw(blessed reftype looks_like_number);
    
    # Control file operations
    use File::Spec::Functions qw(catfile catdir);
    use File::Path qw(make_path);
    use File::Basename qw(basename dirname);
    
    # Control time operations
    use Time::HiRes qw(time sleep);
    use Time::Piece;
    
    # Control string operations
    use String::Util qw(trim crunch hascontent);
    use Text::Wrap qw(wrap fill);
    
    # Control data structures
    use List::Util qw(first min max reduce sum shuffle);
    use List::MoreUtils qw(any all none uniq apply);
    use Hash::Util qw(lock_keys unlock_keys);
    
    # Advanced numeric operations
    sub calculate_statistics {
        my (@numbers) = @_;
        
        # Use integer pragma for count operations
        my $count = scalar @numbers;
        
        # Disable integer for floating point calculations
        {
            no integer;
            my $sum = sum(@numbers);
            my $mean = $sum / $count;
            
            my $variance = sum(map { ($_ - $mean) ** 2 } @numbers) / $count;
            my $stddev = sqrt($variance);
            
            return {
                count => $count,
                sum => $sum,
                mean => $mean,
                min => min(@numbers),
                max => max(@numbers),
                variance => $variance,
                stddev => $stddev,
            };
        }
    }
    
    # Safe string operations with UTF-8
    sub process_text {
        my ($text, $options) = @_;
        
        # Ensure proper UTF-8 handling
        utf8::upgrade($text);
        
        # Trim and clean
        $text = trim($text);
        $text = crunch($text);
        
        # Apply transformations
        if ($options->{lowercase}) {
            $text = lc($text);
        }
        
        if ($options->{uppercase}) {
            $text = uc($text);
        }
        
        if ($options->{titlecase}) {
            $text = join(' ', map { ucfirst(lc($_)) } split(/\s+/, $text));
        }
        
        return $text;
    }
    
    # Safe file operations
    sub safe_file_operation {
        my ($operation, $filename, $content) = @_;
        
        # Validate filename
        croak "Invalid filename" unless $filename =~ /^[a-zA-Z0-9_.-]+$/;
        
        # Create directory if needed
        my $dir = dirname($filename);
        make_path($dir) unless -d $dir;
        
        if ($operation eq 'read') {
            open my $fh, '<:encoding(UTF-8)', $filename or croak "Cannot read $filename: $!";
            my $content = do { local $/; <$fh> };
            close $fh;
            return $content;
        } elsif ($operation eq 'write') {
            open my $fh, '>:encoding(UTF-8)', $filename or croak "Cannot write $filename: $!";
            print $fh $content;
            close $fh;
            return 1;
        } else {
            croak "Invalid operation: $operation";
        }
    }
}

# Signal Handling Patterns
{
    package EnterpriseExample::SignalHandler;
    
    use strict;
    use warnings;
    use POSIX qw(:signal_h :sys_wait_h);
    use Time::HiRes qw(sleep);
    use File::Temp qw(tempfile);
    use Proc::ProcessTable;
    
    our $VERSION = '1.0.0';
    
    my %signal_handlers;
    my $pid_file;
    my $running = 1;
    my $restart_requested = 0;
    
    sub setup_signal_handlers {
        my ($config) = @_;
        
        # Set up PID file
        if ($config->{pid_file}) {
            $pid_file = $config->{pid_file};
            write_pid_file($pid_file);
        }
        
        # SIGTERM - Graceful shutdown
        $SIG{TERM} = sub {
            log_message('info', 'SIGTERM received, initiating graceful shutdown');
            $running = 0;
            cleanup_and_exit(0);
        };
        
        # SIGINT - Interrupt (Ctrl+C)
        $SIG{INT} = sub {
            log_message('info', 'SIGINT received, initiating shutdown');
            $running = 0;
            cleanup_and_exit(0);
        };
        
        # SIGHUP - Reload configuration
        $SIG{HUP} = sub {
            log_message('info', 'SIGHUP received, reloading configuration');
            reload_configuration();
        };
        
        # SIGUSR1 - Graceful restart
        $SIG{USR1} = sub {
            log_message('info', 'SIGUSR1 received, initiating graceful restart');
            $restart_requested = 1;
            $running = 0;
        };
        
        # SIGUSR2 - Status report
        $SIG{USR2} = sub {
            log_message('info', 'SIGUSR2 received, generating status report');
            generate_status_report();
        };
        
        # SIGCHLD - Child process cleanup
        $SIG{CHLD} = sub {
            my $pid;
            while (($pid = waitpid(-1, WNOHANG)) > 0) {
                log_message('debug', "Child process $pid terminated");
            }
            $SIG{CHLD} = 'DEFAULT';  # Reset to avoid SysV issues
        };
        
        # SIGPIPE - Broken pipe handling
        $SIG{PIPE} = 'IGNORE';
        
        # SIGALRM - Timeout handling
        $SIG{ALRM} = sub {
            log_message('warn', 'SIGALRM received, operation timed out');
            handle_timeout();
        };
        
        # Store handlers for potential restoration
        %signal_handlers = %SIG;
    }
    
    sub write_pid_file {
        my ($filename) = @_;
        
        open my $fh, '>', $filename or die "Cannot write PID file: $!";
        print $$fh $$;
        close $fh;
    }
    
    sub remove_pid_file {
        return unless $pid_file && -f $pid_file;
        unlink $pid_file or warn "Cannot remove PID file: $!";
    }
    
    sub cleanup_and_exit {
        my ($exit_code) = @_;
        
        log_message('info', 'Performing cleanup before exit');
        
        # Clean up child processes
        cleanup_children();
        
        # Remove PID file
        remove_pid_file();
        
        # Close database connections
        cleanup_connections();
        
        # Flush logs
        flush_logs();
        
        exit($exit_code || 0);
    }
    
    sub cleanup_children {
        my $pt = Proc::ProcessTable->new();
        
        for my $process (@{$pt->table}) {
            if ($process->ppid == $$) {
                log_message('debug', "Terminating child process " . $process->pid);
                kill('TERM', $process->pid);
            }
        }
        
        # Wait for children to terminate
        sleep(1);
        
        # Force kill any remaining children
        for my $process (@{$pt->table}) {
            if ($process->ppid == $$) {
                log_message('warn', "Force killing child process " . $process->pid);
                kill('KILL', $process->pid);
            }
        }
    }
    
    sub reload_configuration {
        # Implementation depends on application
        log_message('info', 'Configuration reloaded');
    }
    
    sub generate_status_report {
        my $status = {
            pid => $$,
            uptime => time() - $^T,
            memory_usage => get_memory_usage(),
            process_count => get_process_count(),
            timestamp => time(),
        };
        
        log_message('info', 'Status report', $status);
    }
    
    sub handle_timeout {
        # Handle timeout based on current operation
        log_message('warn', 'Operation timed out, taking corrective action');
    }
    
    sub get_memory_usage {
        # Platform-specific memory usage
        if ($^O eq 'linux') {
            open my $fh, '<', "/proc/$$/status" or return 0;
            while (my $line = <$fh>) {
                if ($line =~ /^VmRSS:\s+(\d+)\s+kB/) {
                    close $fh;
                    return $1 * 1024;  # Convert to bytes
                }
            }
            close $fh;
        }
        
        return 0;
    }
    
    sub get_process_count {
        my $count = 0;
        my $pt = Proc::ProcessTable->new();
        
        for my $process (@{$pt->table}) {
            $count++ if $process->ppid == $$;
        }
        
        return $count;
    }
    
    sub cleanup_connections {
        # Close database connections, file handles, etc.
    }
    
    sub flush_logs {
        # Flush any buffered log messages
    }
    
    sub run_event_loop {
        my ($config) = @_;
        
        setup_signal_handlers($config);
        
        log_message('info', 'Starting event loop');
        
        while ($running) {
            # Main application logic here
            
            # Check for restart request
            if ($restart_requested) {
                log_message('info', 'Restarting application');
                exec($0, @ARGV) or die "Cannot restart: $!";
            }
            
            # Sleep to prevent busy waiting
            sleep(1);
        }
        
        log_message('info', 'Event loop terminated');
    }
}

# Version Requirements and Dependencies
{
    package EnterpriseExample::DependencyManager;
    
    use strict;
    use warnings;
    use v5.20;
    
    use version;
    use CPAN::Meta::Requirements;
    use Module::Load::Conditional qw(can_load);
    use Scalar::Util qw(blessed);
    
    our $REQUIRED_PERL_VERSION = '5.020';
    our $APPLICATION_VERSION = '3.2.1';
    
    # Define required modules and versions
    my %REQUIRED_MODULES = (
        'DBI' => '1.631',
        'DBD::Pg' => '3.14.2',
        'Moose' => '2.2014',
        'Moo' => '2.004004',
        'JSON' => '4.02',
        'YAML::XS' => '0.82',
        'DateTime' => '1.54',
        'Try::Tiny' => '0.30',
        'namespace::clean' => '0.27',
        'Log::Log4perl' => '1.54',
        'Digest::MD5' => '2.58',
        'Crypt::CBC' => '2.33',
        'LWP::UserAgent' => '6.52',
        'HTTP::Tiny' => '0.076',
        'File::Spec' => '3.75',
        'Path::Tiny' => '0.108',
        'Text::CSV' => '2.01',
        'Excel::Writer::XLSX' => '1.07',
        'Template' => '3.100',
        'Mojolicious' => '9.19',
        'Dancer2' => '0.301004',
    );
    
    # Optional modules with enhanced features
    my %OPTIONAL_MODULES = (
        'Redis' => '1.998',
        'Memcached::libmemcached' => '0.7002',
        'Net::AMQP::RabbitMQ' => '2.40008',
        'XML::LibXML' => '2.0207',
        'Image::Magick' => '6.89',
        'PDF::API2' => '2.043',
        'Email::Sender' => '2.500',
        'SMS::Send' => '1.06',
        'Net::SSH2' => '0.72',
        'Net::OpenSSH' => '0.78',
        'Parallel::ForkManager' => '2.02',
        'Proc::Daemon' => '0.23',
        'Sys::Statistics::Linux' => '0.66',
        'Monitoring::Plugin' => '0.39',
    );
    
    sub check_dependencies {
        my ($check_optional) = @_;
        
        my $results = {
            perl_version_ok => 0,
            required_modules => {},
            optional_modules => {},
            missing_required => [],
            missing_optional => [],
            version_conflicts => [],
        };
        
        # Check Perl version
        my $current_perl = $];
        my $required_perl = version->parse($REQUIRED_PERL_VERSION)->numify;
        
        $results->{perl_version_ok} = $current_perl >= $required_perl;
        $results->{current_perl_version} = $current_perl;
        $results->{required_perl_version} = $required_perl;
        
        # Check required modules
        for my $module (sort keys %REQUIRED_MODULES) {
            my $required_version = $REQUIRED_MODULES{$module};
            
            if (can_load(modules => { $module => undef })) {
                my $installed_version = get_module_version($module);
                
                if ($installed_version) {
                    $results->{required_modules}{$module} = {
                        installed => $installed_version,
                        required => $required_version,
                        ok => version_check($installed_version, $required_version),
                    };
                    
                    unless ($results->{required_modules}{$module}{ok}) {
                        push @{$results->{version_conflicts}}, {
                            module => $module,
                            installed => $installed_version,
                            required => $required_version,
                        };
                    }
                } else {
                    push @{$results->{missing_required}}, $module;
                    $results->{required_modules}{$module} = {
                        installed => undef,
                        required => $required_version,
                        ok => 0,
                    };
                }
            } else {
                push @{$results->{missing_required}}, $module;
                $results->{required_modules}{$module} = {
                    installed => undef,
                    required => $required_version,
                    ok => 0,
                };
            }
        }
        
        # Check optional modules if requested
        if ($check_optional) {
            for my $module (sort keys %OPTIONAL_MODULES) {
                my $required_version = $OPTIONAL_MODULES{$module};
                
                if (can_load(modules => { $module => undef })) {
                    my $installed_version = get_module_version($module);
                    
                    $results->{optional_modules}{$module} = {
                        installed => $installed_version,
                        required => $required_version,
                        ok => version_check($installed_version, $required_version),
                    };
                } else {
                    push @{$results->{missing_optional}}, $module;
                    $results->{optional_modules}{$module} = {
                        installed => undef,
                        required => $required_version,
                        ok => 0,
                    };
                }
            }
        }
        
        return $results;
    }
    
    sub get_module_version {
        my ($module) = @_;
        
        no strict 'refs';
        
        my $version_var = "${module}::VERSION";
        my $version = ${$version_var};
        
        return $version if defined $version;
        
        # Try to load and check
        eval "require $module";
        return undef if $@;
        
        $version = ${$version_var};
        return $version;
    }
    
    sub version_check {
        my ($installed, $required) = @_;
        
        return 0 unless defined $installed && defined $required;
        
        eval {
            my $installed_ver = version->parse($installed);
            my $required_ver = version->parse($required);
            return $installed_ver >= $required_ver;
        };
        
        return 0 if $@;
    }
    
    sub generate_dependency_report {
        my ($results) = @_;
        
        my $report = "Dependency Report for Application v$APPLICATION_VERSION\n";
        $report .= "=" x 60 . "\n\n";
        
        # Perl version
        $report .= "Perl Version:\n";
        $report .= sprintf "  Required: %s\n", $results->{required_perl_version};
        $report .= sprintf "  Current:  %s\n", $results->{current_perl_version};
        $report .= sprintf "  Status:   %s\n\n", $results->{perl_version_ok} ? "OK" : "FAILED";
        
        # Required modules
        $report .= "Required Modules:\n";
        for my $module (sort keys %{$results->{required_modules}}) {
            my $info = $results->{required_modules}{$module};
            my $status = $info->{ok} ? "OK" : "FAILED";
            my $installed = $info->{installed} || "NOT INSTALLED";
            
            $report .= sprintf "  %-30s %s (required: %s)\n", $module, $installed, $info->{required};
        }
        
        if (@{$results->{missing_required}}) {
            $report .= "\nMissing Required Modules:\n";
            $report .= "  " . join("\n  ", @{$results->{missing_required}}) . "\n";
        }
        
        if (@{$results->{version_conflicts}}) {
            $report .= "\nVersion Conflicts:\n";
            for my $conflict (@{$results->{version_conflicts}}) {
                $report .= sprintf "  %s: installed %s, required %s\n",
                    $conflict->{module}, $conflict->{installed}, $conflict->{required};
            }
        }
        
        # Optional modules
        if (%{$results->{optional_modules}}) {
            $report .= "\nOptional Modules:\n";
            for my $module (sort keys %{$results->{optional_modules}}) {
                my $info = $results->{optional_modules}{$module};
                my $status = $info->{ok} ? "OK" : "OUTDATED";
                my $installed = $info->{installed} || "NOT INSTALLED";
                
                $report .= sprintf "  %-30s %s (required: %s)\n", $module, $installed, $info->{required};
            }
        }
        
        return $report;
    }
    
    sub install_missing_modules {
        my ($results) = @_;
        
        return unless @{$results->{missing_required}};
        
        print "The following required modules are missing:\n";
        print "  " . join("\n  ", @{$results->{missing_required}}) . "\n\n";
        
        print "Install them now? (y/N): ";
        my $answer = <STDIN>;
        chomp $answer;
        
        return unless $answer =~ /^y/i;
        
        for my $module (@{$results->{missing_required}}) {
            print "Installing $module...\n";
            system "cpan", $module;
        }
    }
}

# Usage examples
package main;

print "=== Enterprise/CPAN Patterns Test ===\n";

# Simulate enterprise operations
print "Exporter: Advanced export groups, configuration, validation\n";
print "Pragmas: UTF-8 handling, strict/warnings, namespace management\n";
print "Signals: SIGTERM, SIGHUP, SIGUSR1/2, graceful shutdown/restart\n";
print "Dependencies: Version checking, optional modules, conflict resolution\n";

print "\n=== Enterprise/CPAN Patterns Test Complete ===\n";