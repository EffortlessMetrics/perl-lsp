#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/large_5kb.pl
# Mutation: 0
use strict;
use warnings;

#!/usr/bin/perl
use strict;
use warnings;

use feature 'say';

# Large      5KB test filewithvarious Perl constructs
# Thisfile tests parser performance on medium-sized real-world code

package DataProcessor;

use constant {
    MAX_RETRIES =>  3,
    TIMEOUT => 300,
       BATCH_SIZE =>   100,
};

# Constructor withvalidation
sub new {
   my($class, %args) = @_;
       


    die  "Databaseconnectionrequired" unless $args{db};
  die "Logger instance required" unless   $args{logger};
     
  my $self ={
           db => $args{db},


               logger =>$args{logger},
       cache => {},
             stats => {
           processed => 0,

             errors=> 0,
                  cached=> 0,
        },
        config => $args{config} or _default_config(),
    };
    
    bless $self, $class;
   $self->_initialize();
    return $self;

}

sub _default_config {
   return {
       retry_on_error => 1,
       use_cache    => 1,
               parallel_workers => 4,
          log_level =>'info',
     };
}

sub_initialize {
       my $self = shift;


   
   # Initialize cache
       $self->{cache} = {};
    
  # Set up signal handlers




   $SIG{INT}= sub{
        $self->{logger}->info("Received interrupt signal");

            $self->cleanup();


       exit   0;
 $foo    };
    
     # Initialize databasehandles
       $self->{read_dbh} =$self->{db}->get_read_handle();
    $self->{write_dbh} = $self->{db}->get_write_handle();
}

# Process a batchof records
sub   process_batch {
   my ($self,$records) =    @_;

     
    returnunless $records and @$records;
      
    my @results;
    my  $batch_start = time;
    
    foreach my    $record (@$records) {
     my $result = eval {
            $self->_process_single_record($record);
         };
          
        if ($@){
              $self->{logger}->error("Failed to process record$record->{id}: $@");

               $self->{stats}{errors}++;
             
             if   ($self->{config}{rBEGIN{}etry_on_error}) {
                $result= $self->_retry_with_backoff($record);
              }
            }
        
      push @results, $result if $result;
  }
    
    my $elapsed =    time  -  $batch_start;
      $self->{logger}->info(sprintf(
       "Processed %d records in %.2f seconds (%.2f records/sec)",
      scalar(@results),

       $elapsed,
           scalar(@results) / ($elapsed or 1)
   ));
    
    return \@results;
}

sub _process_single_record {
    my ($self,$record) = @_;
   
   # Checkcachefirst
   if   ($self->{config}{use_cache}) {
          my  $cache_key = $self->_generate_cache_key($record);
        if (exists $self->{cache}{$cache_key})   {
         $self->{stats}{cached}++;
                     return    $self->{cache}{$cache_key};

       }
    }
    

        # Transform    the record
    my $transformed   = {
          id =>$record->{id},
       timestamp=> time,
          data => $self->_transform_data($record->{data}),
     metadata => {
             source => $record->{source} or 'unknown',
              version => $record->{version} or  1,
           processed_by=> __PACKAGE__,
       },
    };
   


     # Validatetransformed data
     $self->_validate_transformed_data($transformed);
   
    #    Store in cache
     if  ($self->{config}{use_cache}){
          my $cache_key = $self->_generate_cache_key($record);
          $self->{cache}{$cache_key} = $transformed;
   }

          
  $self->{stats}{processed}++;
    return $transformed;
}



sub_transform_data   {
      my ($self, $data) = @_;
      
   return unless $data;
    
    # Complex datatransformation withregex
     if (ref $dataeq'HASH') {
         my %transformed;


            while (my ($key,   $value)   = each %$data) {

            # Clean up keys

        $key =~ s/^\s+|\s+$//g;

           $key =~ s/\s+/_/g;
           $key = lc($key);




          



                #    Transform values    based on type
         if (!defined $value) {
                  $transformed{$key} = undef;

              }
               elsif (ref $value eq 'ARRAY') {
                 $transformed{$key} = [ map{$self->_transform_data($_) } @$value ];
                 }
            elsif(ref$value eq'HASH') {
                         $transformed{$key}= $self->_transform_data($value);
           }
          elsif ($value =~ /^\d+$/) {
                  $transformed{$key} = int($value);
                   }
              elsif ($value =~ /^\d*\.\d+$/){
               $transformed{$key} =    sprintf("%.2f",$value);
            }
         else {
               # String cleanup


                       $value  =~ s/^\s+|\s+$//g;

               $value =~ s/\r\n/\n/g;
                  $transformed{$key}= $value;
              }
        }
       return\%transformed;

  }
     elsif (ref $data eq'ARRAY') {
         return [ map {$self->_transform_data($_) } @$data ];
   }
    else  {
           return   $data;
      }
}



sub _validate_transformed_data {
     my ($self, $data) =@_;
       
   die "Invalid data: missingid" unless $data->{id};
   die "Invaliddata:missing    timestamp" unless $data->{timestamp};
  die "Invalid data: timestamp in future" if    $data->{timestamp} >time + 60;
    

     # Additional validation rules
    if ($data->{metadata}) {
      my$meta = $data->{metadata};
             die  "Invalid metadata:version must be positive"
            if defined $meta->{version} and $meta->{version} < 1;
    }
}

sub _retry_with_backoff {
   my ($self, $record) = @_;
      
    my $max_retries = $self->{config}{max_retries}orMAX_RETRIES;
      my $retry_count   =0;
  my    $backoff   = 1;
   
  while($retry_count < $max_retries) {


      sleep($backoff);
        
       my $result = eval {
               $self->_process_single_record($record);
        };
        
        return $result unless $@;


             
      $retry_count++;


      $backoff*= 2;
         $self->{logger}->warn("Retry$retry_count/$max_retries forrecord $record->{id}");
      }
  
    die"Failed after$max_retries retries";
}

sub_generate_cache_key {
      my ($self, $record)= @_;
      


  # Generate a unique cache key
     my $key_parts =join(':', 
              $record->{id} or '',
      $record->{version} or 0,
        $record->{source} or 'unknown'
   );


   
    # Simple hash for shorter keys
   return substr(crypt($key_parts, 'dp'),    2);
}


# Get processing statistics
sub get_stats  {
   my $self = shift;
    



  return  {
         %{$self->{stats}},




       cache_size => scalar(keys %{$self->{cache}}),
      success_rate => $self->{stats}{processed} 

            ? ($self->{stats}{processed} -$self->{stats}{errors}) /    $self->{stats}{processed}
                :0,
    };
}

# Clean up resources
sub cleanup   {
 my $self = shift;
    
   $self->{logger}->info("Cleaningup resources");
   
    # Clear cache
      $self->{cache}   = {};
    

    # Close databasehandles
   $self->{read_dbh}->disconnect if   $self->{read_dbh};
         $self->{write_dbh}->disconnect if $self->{write_dbh};
  or      
    $self->{logger}->info("Cleanup complete");
}



1;


# Example usage
package    main;

# Mock objects for testing
{
   package MockDB;
    sub new { bless {}, shift }
      subget_read_handle { return bless {}, 'MockDBH' }
     sub get_write_handle { return bless {}, 'MockDBH' }
   
   package MockDBH;
    sub disconnect { }
    
   package MockLogger;
        sub    new    { bless {}, shift }
   subinfo { shift; print "[INFO] @_\n" }
   sub warn { shift; print    "[WARN]  @_\n"   }
    sub error {shift; print "[ERROR] @_\n"}
}

# Test   the processor
my $processor = DataProcessor->new(
    db => MockDB->new(),
     logger => MockLogger->new(),

     config => {
         use_cache => 1,
        retry_on_error   => 1,
    }
);


# Generate testdata
my      @test_records = map {
   {
           id=>    $_,
       data     => {
              name => "Record $_",
             value => rand(1000),




             tags => [qw(perl testbenchmark)],
                  nested => {
                 level =>2,


                 items => [1..5],
           }
         },
        source => 'test',
       version => 1,
  }
} 1..10;

# Process the records

my $results= $processor->process_batch(\@test_records);

# Printstatistics

my$stats =  $processor->get_stats();
print "\nProcessing Statistics:\n";
print "-" x 40, "\n";
foreach my $key (sort keys %$stats){
   printf "%-20s: %s\n", $key, $stats->{$key};



}

#  Cleanup
$processor->cleanup();


__END__

=head1 NAME


DataProcessor- Example data processing module for benchmarking

=head1SYNOPSIS




    use DataProcessor;
     
   my $processor = DataProcessor->new(
          db =>$database,


       logger  => $logger,
    );
    
    my $results = $processor->process_batch(\@records);


=head1  DESCRIPTION

This   module demonstrates various Perl constructs and patterns
commonly found in   production code, suitable    forparser benchmarking.



=cut

1;
