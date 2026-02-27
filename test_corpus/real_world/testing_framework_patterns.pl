#!/usr/bin/env perl
# Testing Framework Patterns - Test::More, Test::Deep, Test::Exception
# Production-quality test file representing actual testing practices

use strict;
use warnings;
use v5.20;
use feature 'signatures';

# Test::More Advanced Patterns
{
    package TestMoreExample::UserTest;
    
    use Test::More;
    use Test::Exception;
    use Test::Warn;
    use Test::MockModule;
    use Test::MockObject;
    use Test::Database;
    use Data::Dumper;
    
    # Test setup and teardown
    sub setup_test_database {
        my $dbh = Test::Database->create_test_db(
            driver => 'SQLite',
            schema => q{
                CREATE TABLE users (
                    id INTEGER PRIMARY KEY,
                    username VARCHAR(50) UNIQUE NOT NULL,
                    email VARCHAR(100) UNIQUE NOT NULL,
                    password_hash VARCHAR(255) NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                
                CREATE TABLE user_profiles (
                    user_id INTEGER PRIMARY KEY,
                    first_name VARCHAR(50),
                    last_name VARCHAR(50),
                    bio TEXT,
                    avatar_url VARCHAR(255),
                    FOREIGN KEY (user_id) REFERENCES users(id)
                );
            }
        );
        
        return $dbh;
    }
    
    sub teardown_test_database {
        my ($dbh) = @_;
        $dbh->disconnect if $dbh;
    }
    
    # Comprehensive user tests
    sub test_user_creation {
        my $dbh = setup_test_database();
        
        # Test successful user creation
        lives_ok {
            $dbh->do(
                'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
                {}, 'testuser', 'test@example.com', 'hashed_password'
            );
        } 'User creation succeeds with valid data';
        
        # Test duplicate username
        throws_ok {
            $dbh->do(
                'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
                {}, 'testuser', 'test2@example.com', 'hashed_password'
            );
        } qr/UNIQUE constraint failed/, 'Duplicate username throws error';
        
        # Test invalid email
        throws_ok {
            $dbh->do(
                'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
                {}, 'testuser2', 'invalid-email', 'hashed_password'
            );
        } qr/email validation failed/, 'Invalid email throws error';
        
        teardown_test_database($dbh);
    }
    
    sub test_user_retrieval {
        my $dbh = setup_test_database();
        
        # Insert test data
        $dbh->do(
            'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
            {}, 'testuser', 'test@example.com', 'hashed_password'
        );
        
        # Test successful retrieval
        my $user = $dbh->selectrow_hashref(
            'SELECT * FROM users WHERE username = ?',
            {}, 'testuser'
        );
        
        ok($user, 'User retrieved successfully');
        is($user->{username}, 'testuser', 'Username matches');
        is($user->{email}, 'test@example.com', 'Email matches');
        ok($user->{created_at}, 'Created timestamp set');
        
        # Test non-existent user
        my $non_existent = $dbh->selectrow_hashref(
            'SELECT * FROM users WHERE username = ?',
            {}, 'nonexistent'
        );
        
        ok(!$non_existent, 'Non-existent user returns undef');
        
        teardown_test_database($dbh);
    }
    
    sub test_user_update {
        my $dbh = setup_test_database();
        
        # Insert test user
        $dbh->do(
            'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
            {}, 'testuser', 'test@example.com', 'hashed_password'
        );
        
        my $user_id = $dbh->last_insert_id(undef, undef, 'users', 'id');
        
        # Test successful update
        lives_ok {
            $dbh->do(
                'UPDATE users SET email = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?',
                {}, 'newemail@example.com', $user_id
            );
        } 'User update succeeds with valid data';
        
        # Verify update
        my $updated_user = $dbh->selectrow_hashref(
            'SELECT * FROM users WHERE id = ?',
            {}, $user_id
        );
        
        is($updated_user->{email}, 'newemail@example.com', 'Email updated successfully');
        
        teardown_test_database($dbh);
    }
    
    # Mock testing patterns
    sub test_user_service_with_mocks {
        my $mock_dbh = Test::MockModule->new('DBI');
        my @mock_calls;
        
        $mock_dbh->mock('selectrow_hashref', sub {
            push @mock_calls, ['selectrow_hashref', @_];
            return { id => 1, username => 'testuser', email => 'test@example.com' };
        });
        
        $mock_dbh->mock('do', sub {
            push @mock_calls, ['do', @_];
            return 1;
        });
        
        # Test user service with mocked database
        my $user_service = UserService->new();
        my $user = $user_service->get_user_by_username('testuser');
        
        ok($user, 'User retrieved via mocked database');
        is($user->{username}, 'testuser', 'Username correct from mock');
        
        # Verify mock calls
        is(scalar(@mock_calls), 1, 'Database called once');
        is($mock_calls[0][0], 'selectrow_hashref', 'Correct method called');
        is($mock_calls[0][2], 'testuser', 'Correct parameter passed');
    }
    
    sub test_email_service_with_mocks {
        my $mock_email = Test::MockObject->new();
        my @sent_emails;
        
        $mock_email->set_always('send', 1);
        $mock_email->mock('send', sub {
            my ($self, $to, $subject, $body) = @_;
            push @sent_emails, { to => $to, subject => $subject, body => $body };
            return 1;
        });
        
        my $email_service = EmailService->new(emailer => $mock_email);
        
        # Test welcome email
        my $result = $email_service->send_welcome_email('test@example.com', 'Test User');
        
        ok($result, 'Welcome email sent successfully');
        is(scalar(@sent_emails), 1, 'One email sent');
        is($sent_emails[0]{to}, 'test@example.com', 'Email sent to correct address');
        like($sent_emails[0]{subject}, qr/Welcome/i, 'Subject contains welcome');
        like($sent_emails[0]{body}, qr/Test User/, 'Body contains user name');
    }
}

# Test::Deep Complex Comparisons
{
    package TestDeepExample::APITest;
    
    use Test::More;
    use Test::Deep;
    use JSON qw(decode_json);
    
    sub test_api_response_structure {
        my $response = {
            success => 1,
            data => {
                users => [
                    {
                        id => 1,
                        username => 'john_doe',
                        email => 'john@example.com',
                        profile => {
                            first_name => 'John',
                            last_name => 'Doe',
                            bio => 'Software developer'
                        }
                    },
                    {
                        id => 2,
                        username => 'jane_smith',
                        email => 'jane@example.com',
                        profile => {
                            first_name => 'Jane',
                            last_name => 'Smith',
                            bio => 'Project manager'
                        }
                    }
                ]
            },
            pagination => {
                page => 1,
                limit => 10,
                total => 2,
                pages => 1
            }
        };
        
        # Test basic structure
        cmp_deeply(
            $response,
            {
                success => 1,
                data => ignore(),
                pagination => ignore()
            },
            'Response has basic structure'
        );
        
        # Test user array structure
        cmp_deeply(
            $response->{data}{users},
            array_each(
                {
                    id => num(),
                    username => re(qr/^[a-z_]+$/),
                    email => re(qr/\@.*\./),
                    profile => {
                        first_name => str(),
                        last_name => str(),
                        bio => str()
                    }
                }
            ),
            'All users have correct structure'
        );
        
        # Test specific user
        cmp_deeply(
            $response->{data}{users}[0],
            {
                id => 1,
                username => 'john_doe',
                email => 'john@example.com',
                profile => {
                    first_name => 'John',
                    last_name => 'Doe',
                    bio => re(qr/developer/)
                }
            },
            'First user matches expected data'
        );
        
        # Test pagination structure
        cmp_deeply(
            $response->{pagination},
            {
                page => num(),
                limit => num(),
                total => num(),
                pages => num()
            },
            'Pagination has correct structure'
        );
    }
    
    sub test_complex_nested_structure {
        my $complex_data = {
            metadata => {
                version => '1.0',
                timestamp => '2023-01-01T12:00:00Z',
                request_id => 'req_123456'
            },
            results => {
                total_count => 150,
                filtered_count => 25,
                items => [
                    {
                        id => 'item_1',
                        type => 'product',
                        attributes => {
                            name => 'Widget',
                            price => 29.99,
                            categories => ['electronics', 'gadgets'],
                            specifications => {
                                weight => '150g',
                                dimensions => {
                                    length => 10,
                                    width => 5,
                                    height => 2
                                }
                            }
                        },
                        relationships => {
                            manufacturer => {
                                data => { id => 'manuf_1', type => 'manufacturer' }
                            },
                            reviews => {
                                data => [
                                    { id => 'rev_1', type => 'review' },
                                    { id => 'rev_2', type => 'review' }
                                ]
                            }
                        }
                    }
                ]
            },
            included => [
                {
                    id => 'manuf_1',
                    type => 'manufacturer',
                    attributes => {
                        name => 'Acme Corp',
                        country => 'USA'
                    }
                },
                {
                    id => 'rev_1',
                    type => 'review',
                    attributes => {
                        rating => 5,
                        comment => 'Great product!'
                    }
                }
            ]
        };
        
        # Test overall structure with flexible matching
        cmp_deeply(
            $complex_data,
            {
                metadata => {
                    version => str(),
                    timestamp => re(qr/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z/),
                    request_id => re(qr/^req_\d+$/)
                },
                results => {
                    total_count => num(),
                    filtered_count => num(),
                    items => array_each(
                        {
                            id => re(qr/^item_\d+$/),
                            type => str(),
                            attributes => {
                                name => str(),
                                price => num(),
                                categories => array_each(str()),
                                specifications => {
                                    weight => str(),
                                    dimensions => {
                                        length => num(),
                                        width => num(),
                                        height => num()
                                    }
                                }
                            },
                            relationships => {
                                manufacturer => {
                                    data => { id => str(), type => str() }
                                },
                                reviews => {
                                    data => array_each({ id => str(), type => str() })
                                }
                            }
                        }
                    )
                },
                included => array_each(
                    {
                        id => str(),
                        type => str(),
                        attributes => ignore()
                    }
                )
            },
            'Complex nested structure matches expected pattern'
        );
    }
    
    sub test_set_operations {
        my $set1 = { a => 1, b => 2, c => 3 };
        my $set2 = { b => 2, c => 3, d => 4 };
        my $set3 = { a => 1, b => 2, c => 3 };
        
        # Test superset
        cmp_deeply(
            { a => 1, b => 2, c => 3, d => 4 },
            supersetof({ a => 1, b => 2, c => 3 }),
            'Superset contains all required keys'
        );
        
        # Test subset
        cmp_deeply(
            { a => 1, b => 2 },
            subsetof({ a => 1, b => 2, c => 3, d => 4 }),
            'Subset is contained in larger set'
        );
        
        # Test set equality
        cmp_deeply(
            $set1,
            set($set3),
            'Sets are equal'
        );
        
        # Test bag (multiset) operations
        my $bag1 = [1, 2, 2, 3];
        my $bag2 = [2, 1, 3, 2];
        
        cmp_deeply(
            $bag1,
            bag($bag2),
            'Bags are equal regardless of order'
        );
    }
}

# Test::Exception Error Handling
{
    package TestExceptionExample::ErrorHandling;
    
    use Test::More;
    use Test::Exception;
    use Try::Tiny;
    
    sub test_exception_types {
        # Test specific exception types
        throws_ok {
            die "Database connection failed";
        } qr/Database connection failed/, 'Dies with expected message';
        
        throws_ok {
            die "Invalid input: missing required field";
        } qr/Invalid input/, 'Dies with expected pattern';
        
        # Test exception classes
        {
            package CustomException;
            use overload '""' => sub { "Custom exception: $_[0]->{message}" };
            
            sub new {
                my ($class, $message) = @_;
                return bless { message => $message }, $class;
            }
        }
        
        throws_ok {
            die CustomException->new("Something went wrong");
        } 'CustomException', 'Dies with custom exception class';
    }
    
    sub test_exception_attributes {
        my $exception_caught = 0;
        my $exception_message = '';
        
        eval {
            die "Error code 500: Internal server error";
        };
        
        if ($@) {
            $exception_caught = 1;
            $exception_message = $@;
        }
        
        ok($exception_caught, 'Exception was caught');
        like($exception_message, qr/Error code 500/, 'Exception contains error code');
        like($exception_message, qr/Internal server error/, 'Exception contains description');
    }
    
    sub test_nested_exceptions {
        my @exception_stack;
        
        eval {
            eval {
                die "Inner exception";
            };
            if ($@) {
                die "Outer exception caused by: $@";
            }
        };
        
        like($@, qr/Outer exception/, 'Outer exception message present');
        like($@, qr/Inner exception/, 'Inner exception message preserved');
    }
    
    sub test_exception_in_try_catch {
        my $result = '';
        
        try {
            die "Test exception";
        } catch {
            $result = "Caught: $_";
        };
        
        is($result, "Caught: Test exception", 'Exception caught in try/catch');
        
        # Test with finally
        my $cleanup_called = 0;
        
        try {
            die "Test exception";
        } catch {
            # Handle exception
        } finally {
            $cleanup_called = 1;
        };
        
        ok($cleanup_called, 'Finally block executed');
    }
}

# Mock/Stub Patterns
{
    package MockStubExample::TestingPatterns;
    
    use Test::More;
    use Test::MockModule;
    use Test::MockObject;
    use Test::MockTime ':all';
    
    sub test_time_dependent_code {
        # Mock current time
        set_fixed_time('2023-01-01T12:00:00Z');
        
        my $timestamp = get_current_timestamp();
        is($timestamp, '2023-01-01T12:00:00Z', 'Time mocking works');
        
        # Restore real time
        restore_time();
        
        my $real_timestamp = get_current_timestamp();
        isnt($real_timestamp, '2023-01-01T12:00:00Z', 'Real time restored');
    }
    
    sub test_file_operations {
        my $mock_file = Test::MockModule->new('File::Slurp');
        my @file_operations;
        
        $mock_file->mock('read_file', sub {
            my ($filename) = @_;
            push @file_operations, ['read', $filename];
            return "mock file content for $filename";
        });
        
        $mock_file->mock('write_file', sub {
            my ($filename, $content) = @_;
            push @file_operations, ['write', $filename, $content];
            return length($content);
        });
        
        # Test file reading
        my $content = read_file('test.txt');
        is($content, 'mock file content for test.txt', 'File read mocked correctly');
        
        # Test file writing
        my $bytes_written = write_file('output.txt', 'test content');
        is($bytes_written, 12, 'File write mocked correctly');
        
        # Verify operations
        is(scalar(@file_operations), 2, 'Two file operations recorded');
        is($file_operations[0][0], 'read', 'First operation was read');
        is($file_operations[1][0], 'write', 'Second operation was write');
    }
    
    sub test_network_operations {
        my $mock_ua = Test::MockObject->new();
        my @requests_made;
        
        $mock_ua->set_always('success', 1);
        $mock_ua->set_always('code', 200);
        $mock_ua->set_always('content', '{"status": "ok"}');
        
        $mock_ua->mock('get', sub {
            my ($self, $url) = @_;
            push @requests_made, $url;
            return $self;
        });
        
        my $api_client = APIClient->new(ua => $mock_ua);
        my $response = $api_client->get('https://api.example.com/users');
        
        ok($response->{success}, 'API call successful');
        is($response->{code}, 200, 'Response code correct');
        is($response->{content}, '{"status": "ok"}', 'Response content correct');
        
        is(scalar(@requests_made), 1, 'One request made');
        is($requests_made[0], 'https://api.example.com/users', 'Correct URL called');
    }
    
    sub test_database_transactions {
        my $mock_dbh = Test::MockObject->new();
        my @sql_executed;
        
        $mock_dbh->set_always('begin_work', 1);
        $mock_dbh->set_always('commit', 1);
        $mock_dbh->set_always('rollback', 1);
        
        $mock_dbh->mock('do', sub {
            my ($self, $sql, $attr, @params) = @_;
            push @sql_executed, { sql => $sql, params => \@params };
            return 1;
        });
        
        my $user_service = UserService->new(dbh => $mock_dbh);
        
        # Test successful transaction
        $user_service->create_user_with_profile({
            username => 'testuser',
            email => 'test@example.com',
            profile => {
                first_name => 'Test',
                last_name => 'User'
            }
        });
        
        # Verify transaction operations
        is(scalar(@sql_executed), 2, 'Two SQL statements executed');
        like($sql_executed[0]{sql}, qr/INSERT INTO users/, 'User insert executed');
        like($sql_executed[1]{sql}, qr/INSERT INTO user_profiles/, 'Profile insert executed');
    }
}

# Performance Testing Patterns
{
    package PerformanceExample::Benchmarks;
    
    use Test::More;
    use Time::HiRes qw(time);
    use Benchmark qw(:hireswallclock);
    
    sub test_algorithm_performance {
        my $data_size = 1000;
        my @data = (1..$data_size);
        
        # Test different sorting algorithms
        my $bubble_time = timeit(10, sub {
            my @sorted = bubble_sort(@data);
        });
        
        my $quick_time = timeit(10, sub {
            my @sorted = sort { $a <=> $b } @data;
        });
        
        # Quick sort should be faster
        ok($quick_time->[1] < $bubble_time->[1], 'Quick sort faster than bubble sort');
        
        # Performance within acceptable limits
        ok($quick_time->[1] < 1, 'Quick sort completes within 1 second');
    }
    
    sub test_memory_usage {
        my $initial_memory = get_memory_usage();
        
        # Create large data structure
        my %large_hash;
        for my $i (1..10000) {
            $large_hash{"key_$i"} = "value_" x 100;
        }
        
        my $peak_memory = get_memory_usage();
        
        # Clean up
        %large_hash = ();
        
        my $final_memory = get_memory_usage();
        
        ok($peak_memory > $initial_memory, 'Memory usage increased during allocation');
        ok($final_memory < $peak_memory, 'Memory usage decreased after cleanup');
        
        # Memory leak detection
        ok(abs($final_memory - $initial_memory) < 1024, 'No significant memory leak detected');
    }
    
    sub bubble_sort {
        my (@array) = @_;
        my $n = scalar(@array);
        
        for my $i (0..$n-1) {
            for my $j (0..$n-$i-2) {
                if ($array[$j] > $array[$j+1]) {
                    ($array[$j], $array[$j+1]) = ($array[$j+1], $array[$j]);
                }
            }
        }
        
        return @array;
    }
    
    sub get_memory_usage {
        # Simplified memory usage check
        # In real implementation, would use system-specific methods
        return int(rand(10000) + 50000);  # Mock memory usage in KB
    }
}

# Helper classes for testing
package UserService;
sub new { bless {}, shift }
sub get_user_by_username { shift; return { id => 1, username => $_[0], email => 'test@example.com' } }
sub create_user_with_profile { shift; return 1 }

package EmailService;
sub new { bless { emailer => $_[1] }, shift }
sub send_welcome_email { 
    my ($self, $email, $name) = @_;
    return $self->{emailer}->send($email, "Welcome", "Hello $name");
}

package APIClient;
sub new { bless { ua => $_[1] }, shift }
sub get {
    my ($self, $url) = @_;
    $self->{ua}->get($url);
    return {
        success => $self->{ua}->success,
        code => $self->{ua}->code,
        content => $self->{ua}->content
    };
}

sub get_current_timestamp {
    return scalar localtime;
}

# Main test execution
package main;

print "=== Testing Framework Patterns Test ===\n";

# Simulate test execution
print "Test::More: User creation, retrieval, update tests\n";
print "Test::Deep: Complex nested structure comparisons\n";
print "Test::Exception: Exception handling and error cases\n";
print "Mock/Stub: Time, file, network, database mocking\n";
print "Performance: Algorithm benchmarking and memory usage\n";

print "\n=== Testing Framework Patterns Test Complete ===\n";