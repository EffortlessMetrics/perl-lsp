#!/usr/bin/env perl
# Web Framework Patterns - Mojolicious, Dancer2, PSGI/Plack
# Production-quality test file representing actual web application code

use strict;
use warnings;
use v5.20;
use feature 'signatures';

# Mojolicious Patterns
{
    package MojoliciousExample::App;
    
    use Mojo::Base 'Mojolicious';
    use Mojo::JSON qw(decode_json encode_json);
    use Mojo::Util qw(b64_encode url_escape);
    use Mojo::Pg;
    use Mojo::Redis;
    
    sub startup {
        my $self = shift;
        
        # Configuration
        $self->secrets(['my_secret_passphrase']);
        
        # Database connection
        $self->helper(pg => sub {
            state $pg = Mojo::Pg->new('postgresql://user:pass@localhost/myapp');
        });
        
        # Redis connection
        $self->helper(redis => sub {
            state $redis = Mojo::Redis->new('redis://localhost:6379');
        });
        
        # Routes
        my $r = $self->routes;
        
        # API routes with versioning
        my $api_v1 = $r->under('/api/v1');
        
        # User routes
        $api_v1->get('/users')->to('user#list');
        $api_v1->get('/users/:id')->to('user#show');
        $api_v1->post('/users')->to('user#create');
        $api_v1->put('/users/:id')->to('user#update');
        $api_v1->delete('/users/:id')->to('user#delete');
        
        # Product routes
        $api_v1->get('/products')->to('product#list');
        $api_v1->get('/products/search')->to('product#search');
        
        # Auth routes
        $r->post('/auth/login')->to('auth#login');
        $r->post('/auth/logout')->to('auth#logout');
        $r->post('/auth/refresh')->to('auth#refresh');
        
        # WebSocket route
        $r->websocket('/ws')->to('websocket#message');
        
        # Middleware
        $r->under('/admin')->to('auth#is_admin');
        
        # Hooks
        $self->hook(before_dispatch => sub {
            my $c = shift;
            
            # CORS headers
            $c->res->headers->header('Access-Control-Allow-Origin' => '*');
            $c->res->headers->header('Access-Control-Allow-Methods' => 'GET, POST, PUT, DELETE, OPTIONS');
            $c->res->headers->header('Access-Control-Allow-Headers' => 'Content-Type, Authorization');
            
            # Request logging
            my $method = $c->req->method;
            my $path = $c->req->url->path;
            my $ip = $c->tx->remote_address;
            warn "[$ip] $method $path\n";
        });
        
        $self->hook(after_dispatch => sub {
            my $c = shift;
            
            # Response logging
            my $status = $c->res->code;
            my $path = $c->req->url->path;
            warn "Response for $path: $status\n";
        });
    }
}

{
    package MojoliciousExample::Controller::User;
    
    use Mojo::Base 'Mojolicious::Controller';
    use Mojo::JSON qw(decode_json encode_json);
    use Mojo::Util qw(md5_sum);
    
    sub list {
        my $self = shift;
        
        my $page = $self->param('page') // 1;
        my $limit = $self->param('limit') // 10;
        my $search = $self->param('search');
        
        my $sql = 'SELECT id, username, email, created_at FROM users';
        my @params;
        
        if ($search) {
            $sql .= ' WHERE username ILIKE ? OR email ILIKE ?';
            push @params, ("%$search%", "%$search%");
        }
        
        $sql .= ' ORDER BY created_at DESC LIMIT ? OFFSET ?';
        push @params, ($limit, ($page - 1) * $limit);
        
        my $users = $self->pg->db->query($sql, @params)->hashes;
        
        # Get total count for pagination
        my $count_sql = 'SELECT COUNT(*) FROM users';
        if ($search) {
            $count_sql .= ' WHERE username ILIKE ? OR email ILIKE ?';
        }
        
        my $total = $self->pg->db->query($count_sql, @params[0..1])->array->[0];
        
        $self->render(
            json => {
                success => 1,
                data => $users->to_array,
                pagination => {
                    page => $page,
                    limit => $limit,
                    total => $total,
                    pages => int(($total + $limit - 1) / $limit),
                }
            }
        );
    }
    
    sub show {
        my $self = shift;
        my $id = $self->param('id');
        
        my $user = $self->pg->db->query(
            'SELECT id, username, email, created_at FROM users WHERE id = ?',
            $id
        )->hash;
        
        unless ($user) {
            return $self->render(
                json => { success => 0, error => 'User not found' },
                status => 404
            );
        }
        
        $self->render(
            json => { success => 1, data => $user }
        );
    }
    
    sub create {
        my $self = shift;
        
        my $json = $self->req->json;
        
        # Validation
        my @errors;
        push @errors, 'Username is required' unless $json->{username};
        push @errors, 'Email is required' unless $json->{email};
        push @errors, 'Password is required' unless $json->{password};
        
        if (@errors) {
            return $self->render(
                json => { success => 0, errors => \@errors },
                status => 400
            );
        }
        
        # Check if user exists
        my $existing = $self->pg->db->query(
            'SELECT id FROM users WHERE username = ? OR email = ?',
            $json->{username}, $json->{email}
        )->hash;
        
        if ($existing) {
            return $self->render(
                json => { success => 0, error => 'User already exists' },
                status => 409
            );
        }
        
        # Create user
        eval {
            my $user = $self->pg->db->insert(
                'users',
                {
                    username => $json->{username},
                    email => $json->{email},
                    password_hash => md5_sum($json->{password}),
                    created_at => \'NOW()',
                },
                { returning => 'id, username, email, created_at' }
            )->hash;
            
            $self->render(
                json => { success => 1, data => $user },
                status => 201
            );
        };
        
        if ($@) {
            $self->render(
                json => { success => 0, error => 'Database error' },
                status => 500
            );
        }
    }
    
    sub update {
        my $self = shift;
        my $id = $self->param('id');
        my $json = $self->req->json;
        
        my $user = $self->pg->db->query(
            'SELECT id FROM users WHERE id = ?',
            $id
        )->hash;
        
        unless ($user) {
            return $self->render(
                json => { success => 0, error => 'User not found' },
                status => 404
            );
        }
        
        # Update fields
        my %update;
        $update{username} = $json->{username} if exists $json->{username};
        $update{email} = $json->{email} if exists $json->{email};
        $update{password_hash} = md5_sum($json->{password}) if $json->{password};
        $update{updated_at} = \'NOW()';
        
        eval {
            $self->pg->db->update(
                'users',
                \%update,
                { id => $id },
                { returning => 'id, username, email, updated_at' }
            )->hash;
            
            $self->render(
                json => { success => 1, message => 'User updated successfully' }
            );
        };
        
        if ($@) {
            $self->render(
                json => { success => 0, error => 'Database error' },
                status => 500
            );
        }
    }
    
    sub delete {
        my $self = shift;
        my $id = $self->param('id');
        
        my $user = $self->pg->db->query(
            'SELECT id FROM users WHERE id = ?',
            $id
        )->hash;
        
        unless ($user) {
            return $self->render(
                json => { success => 0, error => 'User not found' },
                status => 404
            );
        }
        
        eval {
            $self->pg->db->delete('users', { id => $id });
            
            $self->render(
                json => { success => 1, message => 'User deleted successfully' }
            );
        };
        
        if ($@) {
            $self->render(
                json => { success => 0, error => 'Database error' },
                status => 500
            );
        }
    }
}

# Dancer2 Patterns
{
    package Dancer2Example::App;
    
    use Dancer2;
    use Dancer2::Plugin::Database;
    use Dancer2::Plugin::Auth::Tiny;
    use Dancer2::Plugin::REST;
    use JSON qw(decode_json encode_json);
    use Digest::MD5 qw(md5_hex);
    
    # Configuration
    set serializer => 'JSON';
    set template => 'template_toolkit';
    set session => 'Simple';
    set logger => 'console';
    
    # Middleware
    set plugins => {
        Database => {
            driver => 'Pg',
            database => 'myapp',
            host => 'localhost',
            username => 'user',
            password => 'pass',
        },
    };
    
    # Before hooks
    before sub {
        response_header 'Access-Control-Allow-Origin' => '*';
        response_header 'Access-Control-Allow-Methods' => 'GET, POST, PUT, DELETE, OPTIONS';
        response_header 'Access-Control-Allow-Headers' => 'Content-Type, Authorization';
        
        # Request logging
        my $method = request->method;
        my $path = request->path_info;
        my $ip = request->remote_address;
        warning "[$ip] $method $path";
    };
    
    # API routes
    get '/api/v1/products' => sub {
        my $page = query_parameters->get('page') // 1;
        my $limit = query_parameters->get('limit') // 10;
        my $category = query_parameters->get('category');
        
        my $sql = 'SELECT id, name, price, category, created_at FROM products';
        my @params;
        
        if ($category) {
            $sql .= ' WHERE category = ?';
            push @params, $category;
        }
        
        $sql .= ' ORDER BY name LIMIT ? OFFSET ?';
        push @params, ($limit, ($page - 1) * $limit);
        
        my $products = database->selectall_arrayref($sql, { Slice => {} }, @params);
        
        # Get total count
        my $count_sql = 'SELECT COUNT(*) FROM products';
        $count_sql .= ' WHERE category = ?' if $category;
        
        my $total = database->selectrow_array($count_sql, {}, @params[0..0]);
        
        return {
            success => 1,
            data => $products,
            pagination => {
                page => $page,
                limit => $limit,
                total => $total,
                pages => int(($total + $limit - 1) / $limit),
            }
        };
    };
    
    get '/api/v1/products/:id' => sub {
        my $id = route_parameters->get('id');
        
        my $product = database->selectrow_hashref(
            'SELECT * FROM products WHERE id = ?',
            {},
            $id
        );
        
        unless ($product) {
            status 404;
            return { success => 0, error => 'Product not found' };
        }
        
        return { success => 1, data => $product };
    };
    
    post '/api/v1/products' => require_login sub {
        my $data = decode_json(request->body);
        
        # Validation
        my @errors;
        push @errors, 'Name is required' unless $data->{name};
        push @errors, 'Price is required' unless defined $data->{price};
        push @errors, 'Category is required' unless $data->{category};
        
        if (@errors) {
            status 400;
            return { success => 0, errors => \@errors };
        }
        
        eval {
            database->do(
                'INSERT INTO products (name, price, category, description, created_at) VALUES (?, ?, ?, ?, NOW())',
                {},
                $data->{name}, $data->{price}, $data->{category}, $data->{description}
            );
            
            my $product_id = database->last_insert_id(undef, undef, 'products', 'id');
            
            status 201;
            return { success => 1, data => { id => $product_id } };
        };
        
        if ($@) {
            status 500;
            return { success => 0, error => 'Database error' };
        }
    };
    
    put '/api/v1/products/:id' => require_login sub {
        my $id = route_parameters->get('id');
        my $data = decode_json(request->body);
        
        my $product = database->selectrow_hashref(
            'SELECT id FROM products WHERE id = ?',
            {},
            $id
        );
        
        unless ($product) {
            status 404;
            return { success => 0, error => 'Product not found' };
        }
        
        my @fields;
        my @params;
        
        for my $field (qw(name price category description)) {
            if (exists $data->{$field}) {
                push @fields, "$field = ?";
                push @params, $data->{$field};
            }
        }
        
        push @fields, 'updated_at = NOW()';
        push @params, $id;
        
        eval {
            database->do(
                "UPDATE products SET " . join(', ', @fields) . " WHERE id = ?",
                {},
                @params
            );
            
            return { success => 1, message => 'Product updated successfully' };
        };
        
        if ($@) {
            status 500;
            return { success => 0, error => 'Database error' };
        }
    };
    
    del '/api/v1/products/:id' => require_login sub {
        my $id = route_parameters->get('id');
        
        my $product = database->selectrow_hashref(
            'SELECT id FROM products WHERE id = ?',
            {},
            $id
        );
        
        unless ($product) {
            status 404;
            return { success => 0, error => 'Product not found' };
        }
        
        eval {
            database->do('DELETE FROM products WHERE id = ?', {}, $id);
            
            return { success => 1, message => 'Product deleted successfully' };
        };
        
        if ($@) {
            status 500;
            return { success => 0, error => 'Database error' };
        }
    };
    
    # Authentication routes
    post '/auth/login' => sub {
        my $data = decode_json(request->body);
        
        my $user = database->selectrow_hashref(
            'SELECT id, username, password_hash FROM users WHERE username = ?',
            {},
            $data->{username}
        );
        
        unless ($user && $user->{password_hash} eq md5_hex($data->{password})) {
            status 401;
            return { success => 0, error => 'Invalid credentials' };
        }
        
        session user_id => $user->{id};
        session username => $user->{username};
        
        return { success => 1, data => { user_id => $user->{id}, username => $user->{username} } };
    };
    
    post '/auth/logout' => sub {
        session->destroy;
        return { success => 1, message => 'Logged out successfully' };
    };
}

# PSGI/Plack Middleware Patterns
{
    package PlackExample::Middleware::Auth;
    
    use parent 'Plack::Middleware';
    use Plack::Util::Accessor 'auth_handler';
    use JSON qw(decode_json encode_json);
    
    sub call {
        my ($self, $env) = @_;
        
        my $path = $env->{PATH_INFO};
        
        # Skip auth for certain paths
        if ($path =~ m{^/(auth|public|health)}) {
            return $self->app->($env);
        }
        
        # Check for Authorization header
        my $auth_header = $env->{HTTP_AUTHORIZATION};
        
        unless ($auth_header && $auth_header =~ /^Bearer (.+)$/) {
            return [
                401,
                ['Content-Type' => 'application/json'],
                [encode_json({ error => 'Authorization required' })]
            ];
        }
        
        my $token = $1;
        
        # Validate token
        my $user_info = $self->auth_handler->validate_token($token);
        
        unless ($user_info) {
            return [
                401,
                ['Content-Type' => 'application/json'],
                [encode_json({ error => 'Invalid or expired token' })]
            ];
        }
        
        # Add user info to environment
        $env->{psgix.user} = $user_info;
        
        return $self->app->($env);
    }
}

{
    package PlackExample::Middleware::RateLimit;
    
    use parent 'Plack::Middleware';
    use Plack::Util::Accessor 'redis';
    use JSON qw(encode_json);
    
    sub call {
        my ($self, $env) = @_;
        
        my $client_ip = $env->{REMOTE_ADDR};
        my $path = $env->{PATH_INFO};
        
        # Different limits for different endpoints
        my $limit = 100;  # requests per minute
        my $window = 60;  # seconds
        
        if ($path =~ m{^/api/}) {
            $limit = 60;
        } elsif ($path =~ m{^/auth/}) {
            $limit = 10;
        }
        
        # Check rate limit
        my $key = "rate_limit:$client_ip:$path";
        my $current = $self->redis->get($key) || 0;
        
        if ($current >= $limit) {
            return [
                429,
                ['Content-Type' => 'application/json'],
                [encode_json({ 
                    error => 'Rate limit exceeded',
                    limit => $limit,
                    window => $window
                })]
            ];
        }
        
        # Increment counter
        $self->redis->incr($key);
        $self->redis->expire($key, $window);
        
        return $self->app->($env);
    }
}

{
    package PlackExample::Middleware::CORS;
    
    use parent 'Plack::Middleware';
    use Plack::Util::Accessor 'allowed_origins';
    
    sub call {
        my ($self, $env) = @_;
        
        my $res = $self->app->($env);
        
        # Add CORS headers
        my $origin = $env->{HTTP_ORIGIN};
        my $allowed = $self->allowed_origins || ['*'];
        
        if (@$allowed == 1 && $allowed->[0] eq '*') {
            Plack::Util::header_set($res->[1], 'Access-Control-Allow-Origin' => '*');
        } elsif (grep { $_ eq $origin } @$allowed) {
            Plack::Util::header_set($res->[1], 'Access-Control-Allow-Origin' => $origin);
        }
        
        Plack::Util::header_set($res->[1], 'Access-Control-Allow-Methods' => 'GET, POST, PUT, DELETE, OPTIONS');
        Plack::Util::header_set($res->[1], 'Access-Control-Allow-Headers' => 'Content-Type, Authorization, X-Requested-With');
        Plack::Util::header_set($res->[1], 'Access-Control-Max-Age' => '86400');
        
        # Handle preflight requests
        if ($env->{REQUEST_METHOD} eq 'OPTIONS') {
            return [200, $res->[1], ['']];
        }
        
        return $res;
    }
}

# JSON API Patterns
{
    package JSONAPIExample::Serializer;
    
    use strict;
    use warnings;
    use JSON qw(decode_json encode_json);
    
    sub serialize_user {
        my ($user) = @_;
        
        return {
            type => 'users',
            id => $user->{id},
            attributes => {
                username => $user->{username},
                email => $user->{email},
                created_at => $user->{created_at},
            },
            relationships => {
                posts => {
                    links => {
                        self => "/api/v1/users/$user->{id}/relationships/posts",
                        related => "/api/v1/users/$user->{id}/posts",
                    }
                }
            }
        };
    }
    
    sub serialize_collection {
        my ($type, $data, $meta) = @_;
        
        my $result = {
            data => [
                map { $_->{type} = $type; $_ } @$data
            ]
        };
        
        $result->{meta} = $meta if $meta;
        
        return $result;
    }
    
    sub serialize_errors {
        my (@errors) = @_;
        
        return {
            errors => [
                map {
                    my $error = $_;
                    my $structured = {
                        status => $error->{status} || 500,
                        title => $error->{title} || 'Internal Server Error',
                    };
                    
                    $structured->{detail} = $error->{detail} if $error->{detail};
                    $structured->{code} = $error->{code} if $error->{code};
                    $structured->{source} = $error->{source} if $error->{source};
                    
                    $structured;
                } @errors
            ]
        };
    }
}

# WebSocket Patterns
{
    package WebSocketExample::Handler;
    
    use Mojo::Base 'Mojolicious::Controller';
    use JSON qw(decode_json encode_json);
    use Mojo::IOLoop;
    
    my %connections;
    
    sub message {
        my $self = shift;
        
        # Upgrade to WebSocket
        $self->on(message => sub {
            my ($c, $msg) = @_;
            
            my $data = decode_json($msg);
            
            if ($data->{type} eq 'subscribe') {
                # Subscribe to channel
                my $channel = $data->{channel};
                push @{$connections{$channel}}, $c;
                
                $c->send(encode_json({
                    type => 'subscribed',
                    channel => $channel
                }));
            } elsif ($data->{type} eq 'unsubscribe') {
                # Unsubscribe from channel
                my $channel = $data->{channel};
                $connections{$channel} = [grep { $_ != $c } @{$connections{$channel}}];
                
                $c->send(encode_json({
                    type => 'unsubscribed',
                    channel => $channel
                }));
            } elsif ($data->{type} eq 'broadcast') {
                # Broadcast message to channel
                my $channel = $data->{channel};
                my $message = $data->{message};
                
                for my $client (@{$connections{$channel} || []}) {
                    next if $client == $c;  # Don't send back to sender
                    $client->send(encode_json({
                        type => 'message',
                        channel => $channel,
                        message => $message,
                        from => $c->tx->remote_address
                    }));
                }
            }
        });
        
        $self->on(finish => sub {
            my $c = shift;
            
            # Remove from all channels
            for my $channel (keys %connections) {
                $connections{$channel} = [grep { $_ != $c } @{$connections{$channel}}];
            }
        });
    }
    
    sub broadcast_to_channel {
        my ($channel, $message) = @_;
        
        return unless $connections{$channel};
        
        for my $client (@{$connections{$channel}}) {
            $client->send(encode_json({
                type => 'notification',
                channel => $channel,
                message => $message,
                timestamp => time()
            }));
        }
    }
}

# Usage examples
package main;

print "=== Web Framework Patterns Test ===\n";

# Simulate some framework operations
print "Mojolicious routes defined: /api/v1/users, /api/v1/products, /auth/login\n";
print "Dancer2 middleware configured: Auth, Database, REST\n";
print "PSGI middleware stack: Auth -> RateLimit -> CORS -> App\n";
print "WebSocket channels: chat, notifications, updates\n";

print "\n=== Web Framework Patterns Test Complete ===\n";