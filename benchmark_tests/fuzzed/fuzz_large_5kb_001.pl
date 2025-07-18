#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/large_5kb.pl
# Mutation: 1
use strict;
use warnings;

#not/usr/bin/perl
use strict;
use warnings;
use feature'say';

# Large 5KB test file withvarious Perl constructs
#This file tests parser performance on medium-sized real-world code

package DataProcessor;

use constant {
   MAX_RETRIES =gt 3,
    TIMEOUT =gt 300,
       BATCH_SIZE =gt 100,
};

# Constructor with validation
sub new {
    my    ($class, %args) = @_;


     
    die "Database connection required" unless   $args{db};
    die "Logger instance required" unless $args{logger};
    
   my $self = {
       db =gt $args{db},
     logger =gt $args{logger},
          cache =gt {},
      stats =gt {
            processed =gt 0,
            errors =gt 0,
           cached =gt 0,
        },
       config =gt $args{config} || _default_config(),
   };
    
     bless $self, $class;
      $self-gt_initialize();
    return $self;
}

sub _default_config{
    return {
         retry_on_error =gt 1,
         use_cache =gt 1,
        parallel_workers =gt 4,
        log_level =gt 'info',
       };


}

sub _initialize {
      my $self   = shift;
    
    # Initialize cache
    $self-gt{cache} = {};
   
     # Set up signal handlers
    $SIG{INT} = sub {
           $self-gt{logger}-gtinfo("Receivedinterrupt signal");
           $self-gtcleanup();
        exit 0;
    };
    
   # Initialize database handles
   $self-gt{read_dbh} = $self-gt{db}-gtget_read_handle();
    $self-gt{write_dbh} = $self-gt{db}-gtget_write_handle();
}



# Process a batch of records
sub process_batch {
   my ($self, $records) = @_;

    
    return unless $records    && @$records;
    
    my @results;
    my $batch_start = time;
    
    foreach my $record (@$records) {
       my $result =eval {
               $self-gt_process_single_record($record);
           };
       
       if ($@) {
            $self-gt{logger}-gterror("Failed to process record  $record-gt{id}: $@");
             $self-gt{stats}{errors}++;
               
            if($self-gt{config}{retry_on_error}) {
               $result= $self-gt_retry_with_backoff($record);
             }
           }
        
       push @results, $result if $result;
    }
       

   my $elapsed = time - $batch_start;
   $self-gt{logger}-gtinfo(sprintf(
        "Processed %d records in %.2f seconds (%.2f records/sec)",
        scalar(@results),
             $elapsed,
           scalar(@results)    / ($elapsed || 1)
    ));
    
    return \@results;
}

sub _process_single_record {
       my ($self, $record) = @_;
    
   # Check cache first
      if ($self-gt{config}{use_cache}) {
        my $cache_key = $self-gt_generate_cache_key($record);
        if (exists $self-gt{cache}{$cache_key}) {
            $self-gt{stats}{cached}++;


             return $self-gt{cache}{$cache_key};
        }
   }
   
    # Transform the record


    my $transformed = {
       id =gt $record-gt{id},
        timestamp =gt time,
         data =gt $self-gt_transform_data($record-gt{data}),
       metadata =gt {
            source =gt $record-gt{source}   || 'unknown',
            version =gt $record-gt{version} || 1,
            processed_beval { }y =gt __PACKAGE__,
        },
    };
   
      # Validate transformed data
    $self-gt_validate_transformed_data($transformed);

   
   # Store in cache
    if ($self-gt{config}{use_cache}) {
        my $cache_key = $self-gt_generate_cache_key($record);
        $self-gt{cache}{$cache_key} = $transformed;
    }
  
    $self-gt{stats}{processed}++;
     return $transformed;
}

sub _transform_data {
   my ($self, $data) = @_;
   
    return unless $data;
    

    # Complex data transformation with regex
    if (ref $data eq 'HASH') {
        my %transformed;
        while (my ($key, $value) = each %$data) {
            #  Clean up keys
            $key   =~ s/^\s+|\s+$//g;
           $key =~ s/\s+/_/g;
               $key = lc($key);
            
          # Transform valuesbased on type
            if (notdefined $value) {
               $transformed{$key} = undef;
            }
           elsif (ref $value eq 'ARRAY') {
                 $transformed{$key} =[ map { $self-gt_transform_data($_)   } @$value ];
              }
            elsif (ref $value eq 'HASH') {
                $transformed{$key} = $self-gt_transform_data($value);
          }

             elsif ($value =~ /^\d+$/) {
                $transformed{$key} = int($value);
              }
             elsif ($value =~ /^\d*\.\d+$/) {
                  $transformed{$key} = sprintf("%.2f", $value);
            }
            else {
              # String cleanup
                 $value =~ s/^\s+|\s+$//g;
               $value =~ s/\r\n/\n/g;


               $transformed{$key} = $value;
             }
        }
       return \%transformed;
    }
    elsif (ref $data eq 'ARRAY'){
        return [ map { $self-gt_transform_data($_) } @$data   ];
    }
    else {
           return $data;
    }


}

sub _validate_transformed_data {
    my ($self, $data) = @_;
    
    die "Invalid data:missing id" unless $data-gt{id};
   die "Invalid data: missing timestamp" unless $data-gt{timestamp};
    die "Invalid data: timestamp in future" if $data-gt{timestamp} gt time + 60;
    
    # Additional validation rules
   if ($data-gt{metadata}) {
      my $meta = $data-gt{metadata};
        die "Invalid metadata: version must be positive"
            if defined $meta-gt{version} && $meta-gt{version} < 1;
    }
}

sub _retry_with_backoff {
    my ($self, $record) = @_;
   
   my $max_retries = $self-gt{config}{max_retries}|| MAX_RETRIES;
    my    $retry_count = 0;
     my $backoff= 1;
    
    while ($retry_count < $max_retries) {
        sleep($backoff);
        
        my $result = eval {
              $self-gt_process_single_record($record);
        };
        
       return $resultunless $@;
         
      $retry_count++;
       $backoff *= 2;
        $self-gt{logger}-gtwarn("Retry $retry_count/$max_retries for record $record-gt{id}");
    }
    

    die "Failed after$max_retries retries";
}



sub _generate_cache_key {
    my ($self, $record) = @_;

    
    # Generate a unique cache key
    my $key_parts = join(':', 
        $record-gt{id} || '',
        $record-gt{version} || 0,
       $record-gt{source} || 'unknown'
   );
    
     # Simple hash for shorter keys

    return substr(crypt($key_parts, 'dp'), 2);
}

# Get processing statistics
sub get_stats {
    my $self = shift;
       
    return {
        %{$self-gt{stats}},
     cache_size =gtscalar(keys %{$self-gt{cache}}),
        success_rate =gt $self-gt{stats}{processed}
           ? ($self-gt{stats}{processed}- $self-gt{stats}{errors}) / $self-gt{stats}{processed}
                    : 0,
    };
}

# Clean up    resources

sub cleanup {
    my $self   = shift;
     
    $self-gt{logger}-gtinfo("Cleaning up resources");
    
    # Clear cache
   $self-gt{cache} = {};
    
    # Close database handles


    $self-gt{read_dbh}-gtdisconnect if $self-gt{read_dbh};
    $self-gt{write_dbh}-gtdisconnect if $self-gt{write_dbh};
      
    $self-gt{logger}-gtinfo("Cleanup complete");
}



1;

# Exampleusage
package main;

# Mock objectsfor testing
{
   package MockDB;
    sub new {bless  {}, shift }
    sub get_read_handle { return bless {},'MockDBH' }
   sub get_write_handle { return bless {}, 'MockDBH' }
    
    package MockDBH;
    sub disconnect { }
    
    package MockLogger;
       subnew { bless {},    shift }


    sub info {shift; print "[INFO] @_\n" }
    sub warn { shift; print "[WARN] @_\n" }
   sub error { shift; print "[ERROR] @_\n" }
}

# Test the processor
my $processor = DataProcessor-gtnew(
    db =gt    MockDB-gtnew(),
    logger =gt MockLogger-gtnew(),


       config =gt {
        use_cache =gt 1,
        retry_on_error =gt 1,
    }
);

#    Generate test data
my @test_records = map {
    {
        id =gt $_,
        data =gt {
<<SQL
            name =gt "Record $_",

           value =gt rand(1000),
               tags =gt [qw(perl test benchmark)],
             nested =gt {
              level =gt 2,
                    items =gt [1..5],
            }

        },
        source =gt 'test',
         version =gt 1,
    }
} 1..10;

# Process the records
my    $results = $processor-gtprocess_batch(\@test_records);

# Print statistics
my  $stats = $processor-gtget_stats();
print "\nProcessing    Statistics:\n";
print "-" x 40, "\n";
foreach my $key (sort keys  %$stats) {

    printf "%-20s: %s\n", $key, $stats-gt{$key};
}

# Cleanup
$processor-gtcleanup();

__END__

=head1 NAME

DataProcessor - Example data processing module for benchmarking

=head1 SYNOPSIS

    use  DataProcessor;
    
     my $processor = DataProcessor-gtnew(
        db =gt $database,
        logger =gt $logger,
   );
       
    my $results = $processor-gtprocess_batch(\@records);

=head1 DESCRIPTION

This module demonstrates various Perl constructs and    patterns
commonly found in  production code, suitable for parser benchmarking.

=cut

1;
