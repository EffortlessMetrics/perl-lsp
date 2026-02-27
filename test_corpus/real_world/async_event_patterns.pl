#!/usr/bin/env perl
# Async/Event Patterns - Mojo::IOLoop, Coro, Event-driven Programming
# Production-quality test file representing actual async Perl applications

use strict;
use warnings;
use v5.20;
use feature 'signatures';

# Mojo::IOLoop Async Patterns
{
    package MojoIOLoopExample::AsyncServer;
    
    use Mojo::Base 'Mojo::EventEmitter';
    use Mojo::IOLoop;
    use Mojo::IOLoop::Server;
    use Mojo::IOLoop::Client;
    use Mojo::IOLoop::Delay;
    use Mojo::JSON qw(decode_json encode_json);
    use Mojo::UserAgent;
    use Mojo::URL;
    use Scalar::Util qw(weaken);
    
    sub new {
        my ($class, $config) = @_;
        
        my $self = $class->SUPER::new();
        $self->{config} = $config;
        $self->{clients} = {};
        $self->{server} = Mojo::IOLoop::Server->new();
        $self->{ua} = Mojo::UserAgent->new();
        $self->{loop} = Mojo::IOLoop->singleton();
        
        weaken $self;
        
        return $self;
    }
    
    sub start {
        my $self = shift;
        
        # Set up server
        $self->{server}->on(accept => sub {
            my ($server, $handle) = @_;
            $self->_handle_new_client($handle);
        });
        
        # Listen on configured port
        my $port = $self->{config}{port} || 3000;
        $self->{server}->listen(port => $port);
        
        # Set up periodic tasks
        $self->setup_periodic_tasks();
        
        # Set up signal handlers
        $self->setup_signal_handlers();
        
        # Start the event loop
        $self->emit(started => { port => $port });
        $self->{loop}->start unless $self->{loop}->is_running;
        
        return $self;
    }
    
    sub _handle_new_client {
        my ($self, $handle) = @_;
        
        my $client_id = sprintf "%x", int(rand(2**32));
        my $client = {
            id => $client_id,
            handle => $handle,
            buffer => '',
            last_activity => time(),
        };
        
        $self->{clients}{$client_id} = $client;
        
        # Set up client handlers
        weaken $self;
        $handle->on(read => sub {
            my ($handle, $bytes) = @_;
            $self->_handle_client_data($client_id, $bytes);
        });
        
        $handle->on(close => sub {
            $self->_handle_client_disconnect($client_id);
        });
        
        $handle->on(error => sub {
            my ($handle, $error) = @_;
            warn "Client error: $error";
            $self->_handle_client_disconnect($client_id);
        });
        
        $self->emit(client_connected => { client_id => $client_id });
    }
    
    sub _handle_client_data {
        my ($self, $client_id, $bytes) = @_;
        
        my $client = $self->{clients}{$client_id} or return;
        
        $client->{buffer} .= $bytes;
        $client->{last_activity} = time();
        
        # Try to parse complete messages
        while ($client->{buffer} =~ s/^(.*?)\r?\n//) {
            my $message = $1;
            $self->_process_client_message($client_id, $message);
        }
    }
    
    sub _process_client_message {
        my ($self, $client_id, $message) = @_;
        
        my $client = $self->{clients}{$client_id} or return;
        
        try {
            my $data = decode_json($message);
            
            if ($data->{type} eq 'echo') {
                $self->_send_to_client($client_id, {
                    type => 'echo_response',
                    data => $data->{data},
                    timestamp => time()
                });
            } elsif ($data->{type} eq 'broadcast') {
                $self->_broadcast_message($client_id, $data->{data});
            } elsif ($data->{type} eq 'http_request') {
                $self->_handle_http_request($client_id, $data);
            } elsif ($data->{type} eq 'delayed_response') {
                $self->_handle_delayed_response($client_id, $data);
            }
        } catch {
            $self->_send_to_client($client_id, {
                type => 'error',
                message => 'Invalid JSON format'
            });
        };
    }
    
    sub _send_to_client {
        my ($self, $client_id, $data) = @_;
        
        my $client = $self->{clients}{$client_id} or return;
        
        my $json = encode_json($data);
        $client->{handle}->write("$json\n");
    }
    
    sub _broadcast_message {
        my ($self, $sender_id, $data) = @_;
        
        my $message = {
            type => 'broadcast',
            sender_id => $sender_id,
            data => $data,
            timestamp => time()
        };
        
        for my $client_id (keys %{$self->{clients}}) {
            next if $client_id eq $sender_id;
            $self->_send_to_client($client_id, $message);
        }
    }
    
    sub _handle_http_request {
        my ($self, $client_id, $data) = @_;
        
        my $url = $data->{url};
        my $method = $data->{method} || 'GET';
        
        # Make async HTTP request
        weaken $self;
        $self->{ua}->get($url => sub {
            my ($ua, $tx) = @_;
            
            my $response = {
                type => 'http_response',
                url => $url,
                status => $tx->res->code,
                body => $tx->res->body,
                headers => $tx->res->headers->to_hash,
            };
            
            $self->_send_to_client($client_id, $response);
        });
    }
    
    sub _handle_delayed_response {
        my ($self, $client_id, $data) = @_;
        
        my $delay = $data->{delay} || 1;
        
        # Schedule delayed response
        weaken $self;
        $self->{loop}->timer($delay => sub {
            $self->_send_to_client($client_id, {
                type => 'delayed_response',
                original_delay => $delay,
                actual_delay => $delay,
                timestamp => time()
            });
        });
    }
    
    sub _handle_client_disconnect {
        my ($self, $client_id) = @_;
        
        delete $self->{clients}{$client_id};
        $self->emit(client_disconnected => { client_id => $client_id });
    }
    
    sub setup_periodic_tasks {
        my $self = shift;
        
        # Cleanup inactive clients every 30 seconds
        weaken $self;
        $self->{loop}->recurring(30 => sub {
            my $now = time();
            my @inactive_clients;
            
            for my $client_id (keys %{$self->{clients}}) {
                my $client = $self->{clients}{$client_id};
                if ($now - $client->{last_activity} > 300) {  # 5 minutes
                    push @inactive_clients, $client_id;
                }
            }
            
            for my $client_id (@inactive_clients) {
                $self->{clients}{$client_id}{handle}->close;
                delete $self->{clients}{$client_id};
            }
            
            if (@inactive_clients) {
                $self->emit(cleanup => { removed_clients => \@inactive_clients });
            }
        });
        
        # Status report every 60 seconds
        $self->{loop}->recurring(60 => sub {
            my $stats = {
                connected_clients => scalar(keys %{$self->{clients}}),
                timestamp => time(),
            };
            
            $self->emit(status_report => $stats);
        });
    }
    
    sub setup_signal_handlers {
        my $self = shift;
        
        # Graceful shutdown on SIGTERM/SIGINT
        for my $signal (qw(TERM INT)) {
            $SIG{$signal} = sub {
                warn "Received $signal, shutting down gracefully\n";
                $self->shutdown;
            };
        }
        
        # Status report on SIGUSR1
        $SIG{USR1} = sub {
            my $stats = {
                connected_clients => scalar(keys %{$self->{clients}}),
                timestamp => time(),
            };
            
            warn "Status: $stats->{connected_clients} clients connected\n";
        };
    }
    
    sub shutdown {
        my $self = shift;
        
        # Close all client connections
        for my $client_id (keys %{$self->{clients}}) {
            $self->{clients}{$client_id}{handle}->close;
        }
        
        # Stop the server
        $self->{server}->stop();
        
        # Stop the event loop
        $self->{loop}->stop();
        
        $self->emit(shutdown_complete => {});
    }
    
    sub async_parallel_requests {
        my ($self, $urls, $callback) = @_;
        
        my $delay = Mojo::IOLoop::Delay->new();
        
        # Add all requests to the delay
        for my $url (@$urls) {
            $delay->begin();
            
            $self->{ua}->get($url => sub {
                my ($ua, $tx) = @_;
                $delay->end($tx);
            });
        }
        
        # Execute callback when all requests complete
        weaken $self;
        $delay->on(finish => sub {
            my ($delay, @results) = @_;
            $callback->(@results);
        });
        
        # Start the requests
        $delay->wait();
    }
    
    sub async_waterfall {
        my ($self, $operations, $callback) = @_;
        
        my $delay = Mojo::IOLoop::Delay->new();
        
        # Chain operations
        for my $operation (@$operations) {
            $delay->steps(sub {
                my ($delay, @args) = @_;
                
                # Execute operation
                my $result = $operation->(@args);
                
                # Pass result to next step
                $delay->pass($result);
            });
        }
        
        # Final callback
        weaken $self;
        $delay->on(finish => sub {
            my ($delay, @results) = @_;
            $callback->(@results);
        });
        
        # Start the waterfall
        $delay->wait();
    }
}

# Coro Coroutine Patterns
{
    package CoroExample::TaskManager;
    
    use Coro;
    use Coro::AnyEvent;
    use Coro::Channel;
    use Coro::Semaphore;
    use Coro::Timer;
    use AnyEvent::HTTP;
    use JSON qw(decode_json encode_json);
    
    sub new {
        my ($class, $config) = @_;
        
        my $self = bless {
            config => $config,
            task_queue => Coro::Channel->new($config->{queue_size} || 100),
            worker_pool => [],
            semaphore => Coro::Semaphore->new($config->{max_workers} || 10),
            stats => {
                tasks_completed => 0,
                tasks_failed => 0,
                workers_active => 0,
            },
        }, $class;
        
        $self->start_workers();
        return $self;
    }
    
    sub start_workers {
        my $self = shift;
        
        my $worker_count = $self->{config}{max_workers} || 10;
        
        for my $i (1..$worker_count) {
            my $worker = async {
                $self->_worker_loop($i);
            };
            
            push @{$self->{worker_pool}}, $worker;
        }
    }
    
    sub _worker_loop {
        my ($self, $worker_id) = @_;
        
        while (1) {
            # Acquire semaphore slot
            $self->{semaphore}->down();
            
            # Get task from queue (blocking)
            my $task = $self->{task_queue}->get();
            
            # Update stats
            $self->{stats}{workers_active}++;
            
            # Process task
            eval {
                $self->_process_task($task, $worker_id);
                $self->{stats}{tasks_completed}++;
            };
            
            if ($@) {
                warn "Worker $worker_id task failed: $@";
                $self->{stats}{tasks_failed}++;
            }
            
            # Update stats
            $self->{stats}{workers_active}--;
            
            # Release semaphore slot
            $self->{semaphore}->up();
            
            # Small delay to prevent busy waiting
            Coro::Timer->sleep(0.01);
        }
    }
    
    sub _process_task {
        my ($self, $task, $worker_id) = @_;
        
        if ($task->{type} eq 'http_request') {
            $self->_handle_http_task($task, $worker_id);
        } elsif ($task->{type} eq 'compute') {
            $self->_handle_compute_task($task, $worker_id);
        } elsif ($task->{type} eq 'sleep') {
            $self->_handle_sleep_task($task, $worker_id);
        } elsif ($task->{type} eq 'parallel') {
            $self->_handle_parallel_task($task, $worker_id);
        }
    }
    
    sub _handle_http_task {
        my ($self, $task, $worker_id) = @_;
        
        my $cv = AnyEvent->condvar;
        my $result;
        
        http_get $task->{url}, sub {
            my ($data, $headers) = @_;
            $result = {
                status => $headers->{Status},
                data => $data,
                headers => $headers,
            };
            $cv->send;
        };
        
        $cv->recv;
        
        if ($task->{callback}) {
            $task->{callback}->($result);
        }
    }
    
    sub _handle_compute_task {
        my ($self, $task, $worker_id) = @_;
        
        my $result;
        
        if ($task->{operation} eq 'fibonacci') {
            $result = $self->_fibonacci($task->{n});
        } elsif ($task->{operation} eq 'prime_check') {
            $result = $self->_is_prime($task->{n});
        } elsif ($task->{operation} eq 'factorial') {
            $result = $self->_factorial($task->{n});
        }
        
        if ($task->{callback}) {
            $task->{callback}->($result);
        }
    }
    
    sub _handle_sleep_task {
        my ($self, $task, $worker_id) = @_;
        
        Coro::Timer->sleep($task->{duration});
        
        if ($task->{callback}) {
            $task->{callback}->({ slept => $task->{duration} });
        }
    }
    
    sub _handle_parallel_task {
        my ($self, $task, $worker_id) = @_;
        
        my @coros;
        my @results;
        
        for my $subtask (@{$task->{subtasks}}) {
            push @coros, async {
                my $cv = AnyEvent->condvar;
                
                # Create subtask
                my $subtask_result;
                http_get $subtask->{url}, sub {
                    my ($data, $headers) = @_;
                    $subtask_result = {
                        url => $subtask->{url},
                        status => $headers->{Status},
                        data => $data,
                    };
                    $cv->send;
                };
                
                $cv->recv;
                return $subtask_result;
            };
        }
        
        # Wait for all coroutines to complete
        for my $coro (@coros) {
            push @results, $coro->join();
        }
        
        if ($task->{callback}) {
            $task->{callback}->(\@results);
        }
    }
    
    sub _fibonacci {
        my ($self, $n) = @_;
        
        return 0 if $n == 0;
        return 1 if $n == 1;
        
        my ($a, $b) = (0, 1);
        for my $i (2..$n) {
            ($a, $b) = ($b, $a + $b);
        }
        
        return $b;
    }
    
    sub _is_prime {
        my ($self, $n) = @_;
        
        return 0 if $n < 2;
        return 1 if $n == 2;
        return 0 if $n % 2 == 0;
        
        for my $i (3..int(sqrt($n)), 2) {
            return 0 if $n % $i == 0;
        }
        
        return 1;
    }
    
    sub _factorial {
        my ($self, $n) = @_;
        
        return 1 if $n <= 1;
        
        my $result = 1;
        for my $i (2..$n) {
            $result *= $i;
        }
        
        return $result;
    }
    
    sub add_task {
        my ($self, $task) = @_;
        
        $self->{task_queue}->put($task);
    }
    
    sub get_stats {
        my $self = shift;
        
        return {
            %{$self->{stats}},
            queue_size => $self->{task_queue}->size(),
            workers_total => scalar(@{$self->{worker_pool}}),
        };
    }
    
    sub shutdown {
        my $self = shift;
        
        # Cancel all workers
        for my $worker (@{$self->{worker_pool}}) {
            $worker->cancel();
        }
        
        # Clear task queue
        while ($self->{task_queue}->size() > 0) {
            $self->{task_queue}->get();
        }
    }
}

# Event-driven Programming Patterns
{
    package EventDrivenExample::EventBus;
    
    use strict;
    use warnings;
    use Scalar::Util qw(weaken);
    use List::Util qw(first);
    use Time::HiRes qw(time);
    
    sub new {
        my ($class, $config) = @_;
        
        my $self = bless {
            config => $config || {},
            listeners => {},
            event_history => [],
            middleware => [],
            event_filters => {},
            stats => {
                events_published => 0,
                events_processed => 0,
                events_failed => 0,
            },
        }, $class;
        
        return $self;
    }
    
    sub subscribe {
        my ($self, $event_type, $listener, $options) = @_;
        
        $options ||= {};
        
        my $listener_info = {
            callback => $listener,
            id => $options->{id} || sprintf "listener_%x", int(rand(2**32)),
            priority => $options->{priority} || 0,
            once => $options->{once} || 0,
            filter => $options->{filter},
            created_at => time(),
        };
        
        push @{$self->{listeners}{$event_type}}, $listener_info;
        
        # Sort by priority (higher priority first)
        @{$self->{listeners}{$event_type}} = sort {
            $b->{priority} <=> $a->{priority}
        } @{$self->{listeners}{$event_type}};
        
        return $listener_info->{id};
    }
    
    sub unsubscribe {
        my ($self, $event_type, $listener_id) = @_;
        
        return unless exists $self->{listeners}{$event_type};
        
        @{$self->{listeners}{$event_type}} = grep {
            $_->{id} ne $listener_id
        } @{$self->{listeners}{$event_type}};
    }
    
    sub publish {
        my ($self, $event_type, $event_data, $options) = @_;
        
        $options ||= {};
        
        my $event = {
            type => $event_type,
            data => $event_data,
            timestamp => time(),
            id => sprintf "event_%x_%d", int(time()), int(rand(2**32)),
            source => $options->{source} || 'unknown',
        };
        
        # Apply middleware
        for my $middleware (@{$self->{middleware}}) {
            my $result = $middleware->($event);
            last unless $result;  # Stop if middleware returns false
        }
        
        # Store in history
        push @{$self->{event_history}}, $event;
        
        # Limit history size
        if (@{$self->{event_history}} > 1000) {
            shift @{$self->{event_history}};
        }
        
        # Apply event filters
        if (exists $self->{event_filters}{$event_type}) {
            for my $filter (@{$self->{event_filters}{$event_type}}) {
                next unless $filter->($event);
                return;  # Skip event if filter returns true
            }
        }
        
        # Notify listeners
        if (exists $self->{listeners}{$event_type}) {
            my @listeners_to_remove;
            
            for my $listener_info (@{$self->{listeners}{$event_type}}) {
                # Apply listener-specific filter
                if ($listener_info->{filter}) {
                    next unless $listener_info->{filter}->($event);
                }
                
                eval {
                    $listener_info->{callback}->($event);
                    $self->{stats}{events_processed}++;
                };
                
                if ($@) {
                    warn "Event listener error: $@";
                    $self->{stats}{events_failed}++;
                }
                
                # Remove 'once' listeners
                if ($listener_info->{once}) {
                    push @listeners_to_remove, $listener_info->{id};
                }
            }
            
            # Remove 'once' listeners
            for my $listener_id (@listeners_to_remove) {
                $self->unsubscribe($event_type, $listener_id);
            }
        }
        
        $self->{stats}{events_published}++;
        
        return $event->{id};
    }
    
    sub add_middleware {
        my ($self, $middleware) = @_;
        
        push @{$self->{middleware}}, $middleware;
    }
    
    sub add_event_filter {
        my ($self, $event_type, $filter) = @_;
        
        push @{$self->{event_filters}{$event_type}}, $filter;
    }
    
    sub get_event_history {
        my ($self, $event_type, $limit) = @_;
        
        my @events;
        
        for my $event (@{$self->{event_history}}) {
            if (!$event_type || $event->{type} eq $event_type) {
                push @events, $event;
            }
        }
        
        if ($limit) {
            @events = splice(@events, 0, $limit);
        }
        
        return \@events;
    }
    
    sub get_stats {
        my $self = shift;
        
        return {
            %{$self->{stats}},
            total_listeners => scalar(map { scalar(@$_) } values %{$self->{listeners}}),
            event_types => scalar(keys %{$self->{listeners}}),
            history_size => scalar(@{$self->{event_history}}),
        };
    }
    
    # Event-driven state machine
    sub create_state_machine {
        my ($self, $config) = @_;
        
        my $state_machine = {
            current_state => $config->{initial_state},
            states => $config->{states},
            transitions => $config->{transitions},
            context => $config->{context} || {},
        };
        
        # Subscribe to state transition events
        $self->subscribe('state_transition', sub {
            my ($event) = @_;
            
            my $transition = $event->{data};
            my $from_state = $transition->{from};
            my $to_state = $transition->{to};
            my $context = $transition->{context};
            
            # Execute state exit actions
            if ($state_machine->{states}{$from_state}{on_exit}) {
                $state_machine->{states}{$from_state}{on_exit}->($context);
            }
            
            # Update state
            $state_machine->{current_state} = $to_state;
            
            # Execute state entry actions
            if ($state_machine->{states}{$to_state}{on_entry}) {
                $state_machine->{states}{$to_state}{on_entry}->($context);
            }
        });
        
        return $state_machine;
    }
    
    sub transition_state {
        my ($self, $state_machine, $event, $context) = @_;
        
        my $current_state = $state_machine->{current_state};
        my $transitions = $state_machine->{transitions}{$current_state};
        
        return unless $transitions;
        
        my $transition = first { $_->{event} eq $event } @$transitions;
        return unless $transition;
        
        # Publish state transition event
        $self->publish('state_transition', {
            from => $current_state,
            to => $transition->{to},
            event => $event,
            context => $context,
        });
        
        return $transition->{to};
    }
}

# Non-blocking I/O Patterns
{
    package NonBlockingExample::IOManager;
    
    use strict;
    use warnings;
    use IO::Select;
    use IO::Socket::INET;
    use IO::File;
    use Time::HiRes qw(time sleep);
    use Scalar::Util qw(weaken);
    
    sub new {
        my ($class, $config) = @_;
        
        my $self = bless {
            config => $config || {},
            read_select => IO::Select->new(),
            write_select => IO::Select->new(),
            read_handlers => {},
            write_handlers => {},
            connections => {},
            buffers => {},
            running => 0,
        }, $class;
        
        return $self;
    }
    
    sub add_read_handler {
        my ($self, $fh, $callback) = @_;
        
        $self->{read_select}->add($fh);
        $self->{read_handlers}{fileno($fh)} = $callback;
        
        weaken $self;
    }
    
    sub add_write_handler {
        my ($self, $fh, $callback) = @_;
        
        $self->{write_select}->add($fh);
        $self->{write_handlers}{fileno($fh)} = $callback;
        
        weaken $self;
    }
    
    sub remove_read_handler {
        my ($self, $fh) = @_;
        
        $self->{read_select}->remove($fh);
        delete $self->{read_handlers}{fileno($fh)};
    }
    
    sub remove_write_handler {
        my ($self, $fh) = @_;
        
        $self->{write_select}->remove($fh);
        delete $self->{write_handlers}{fileno($fh)};
    }
    
    sub start_tcp_server {
        my ($self, $host, $port, $callback) = @_;
        
        my $server = IO::Socket::INET->new(
            LocalHost => $host,
            LocalPort => $port,
            Listen => 10,
            Reuse => 1,
            Blocking => 0,
        ) or die "Cannot create server: $!";
        
        $self->add_read_handler($server, sub {
            my $fh = shift;
            
            # Accept new connection
            my $client = $fh->accept();
            return unless $client;
            
            $client->blocking(0);
            
            my $client_id = fileno($client);
            $self->{connections}{$client_id} = {
                handle => $client,
                buffer => '',
                created_at => time(),
            };
            
            # Add read handler for client
            $self->add_read_handler($client, sub {
                my $client_fh = shift;
                $self->_handle_client_read($client_id);
            });
            
            # Call connection callback
            $callback->($client_id, $client) if $callback;
        });
        
        return $server;
    }
    
    sub _handle_client_read {
        my ($self, $client_id) = @_;
        
        my $connection = $self->{connections}{$client_id};
        return unless $connection;
        
        my $client = $connection->{handle};
        my $buffer = '';
        
        my $bytes_read = sysread($client, $buffer, 4096);
        
        if (!defined $bytes_read) {
            # Error
            warn "Read error: $!";
            $self->_close_connection($client_id);
            return;
        } elsif ($bytes_read == 0) {
            # Connection closed
            $self->_close_connection($client_id);
            return;
        }
        
        $connection->{buffer} .= $buffer;
        $connection->{last_activity} = time();
        
        # Process complete lines
        while ($connection->{buffer} =~ s/^(.*?)\r?\n//) {
            my $line = $1;
            $self->_process_client_line($client_id, $line);
        }
    }
    
    sub _process_client_line {
        my ($self, $client_id, $line) = @_;
        
        my $connection = $self->{connections}{$client_id};
        return unless $connection;
        
        # Simple echo server
        $self->write_to_client($client_id, "ECHO: $line\n");
    }
    
    sub write_to_client {
        my ($self, $client_id, $data) = @_;
        
        my $connection = $self->{connections}{$client_id};
        return unless $connection;
        
        my $client = $connection->{handle};
        
        # Add to write buffer
        $self->{buffers}{$client_id} .= $data;
        
        # Add write handler if not already present
        unless ($self->{write_select}->exists($client)) {
            $self->add_write_handler($client, sub {
                my $fh = shift;
                $self->_handle_client_write($client_id);
            });
        }
    }
    
    sub _handle_client_write {
        my ($self, $client_id) = @_;
        
        my $connection = $self->{connections}{$client_id};
        return unless $connection;
        
        my $client = $connection->{handle};
        my $buffer = $self->{buffers}{$client_id} || '';
        
        return unless length $buffer;
        
        my $bytes_written = syswrite($client, $buffer);
        
        if (!defined $bytes_written) {
            # Error
            warn "Write error: $!";
            $self->_close_connection($client_id);
            return;
        } elsif ($bytes_written == 0) {
            # Shouldn't happen with non-blocking
            return;
        }
        
        # Remove written data from buffer
        if ($bytes_written < length $buffer) {
            $self->{buffers}{$client_id} = substr($buffer, $bytes_written);
        } else {
            delete $self->{buffers}{$client_id};
            $self->remove_write_handler($client);
        }
    }
    
    sub _close_connection {
        my ($self, $client_id) = @_;
        
        my $connection = $self->{connections}{$client_id};
        return unless $connection;
        
        my $client = $connection->{handle};
        
        # Remove handlers
        $self->remove_read_handler($client);
        $self->remove_write_handler($client);
        
        # Close connection
        close $client;
        
        # Clean up
        delete $self->{connections}{$client_id};
        delete $self->{buffers}{$client_id};
    }
    
    sub event_loop {
        my $self = shift;
        
        $self->{running} = 1;
        
        while ($self->{running}) {
            my $timeout = 1.0;  # 1 second timeout
            
            # Check for ready handles
            my ($read_ready, $write_ready, $error_ready) = 
                IO::Select->select($self->{read_select}, $self->{write_select}, undef, $timeout);
            
            # Handle read-ready handles
            for my $fh (@$read_ready) {
                my $callback = $self->{read_handlers}{fileno($fh)};
                $callback->($fh) if $callback;
            }
            
            # Handle write-ready handles
            for my $fh (@$write_ready) {
                my $callback = $self->{write_handlers}{fileno($fh)};
                $callback->($fh) if $callback;
            }
            
            # Periodic cleanup
            $self->_cleanup_inactive_connections();
        }
    }
    
    sub _cleanup_inactive_connections {
        my $self = shift;
        
        my $now = time();
        my @inactive_clients;
        
        for my $client_id (keys %{$self->{connections}}) {
            my $connection = $self->{connections}{$client_id};
            
            if ($now - ($connection->{last_activity} || $connection->{created_at}) > 300) {
                push @inactive_clients, $client_id;
            }
        }
        
        for my $client_id (@inactive_clients) {
            $self->_close_connection($client_id);
        }
    }
    
    sub stop {
        my $self = shift;
        $self->{running} = 0;
    }
}

# Usage examples
package main;

print "=== Async/Event Patterns Test ===\n";

# Simulate async operations
print "Mojo::IOLoop: Event-driven server, parallel requests, waterfall\n";
print "Coro: Coroutine task manager, worker pool, parallel execution\n";
print "Event-driven: Event bus, state machine, middleware\n";
print "Non-blocking I/O: Select-based server, buffered I/O\n";

print "\n=== Async/Event Patterns Test Complete ===\n";