#!/usr/bin/env perl
# Database Integration Patterns - DBI, DBIx::Class, Connection Pooling
# Production-quality test file representing actual database operations

use strict;
use warnings;
use v5.20;
use feature 'signatures';

# DBI Complex Query Patterns
{
    package DBIExample::DatabaseManager;
    
    use DBI;
    use DBD::Pg;
    use DBD::MySQL;
    use DBD::SQLite;
    use SQL::Abstract;
    use SQL::Abstract::Limit;
    use Try::Tiny;
    
    sub new {
        my ($class, $config) = @_;
        
        my $self = bless {
            config => $config,
            dsn => $config->{dsn},
            username => $config->{username},
            password => $config->{password},
            options => {
                RaiseError => 1,
                AutoCommit => 1,
                PrintError => 0,
                ShowErrorStatement => 1,
                AutoInactiveDestroy => 1,
                mysql_enable_utf8 => 1,
                pg_enable_utf8 => 1,
                sqlite_unicode => 1,
                %{ $config->{options} || {} }
            }
        }, $class;
        
        $self->connect;
        return $self;
    }
    
    sub connect {
        my $self = shift;
        
        $self->{dbh} = DBI->connect(
            $self->{dsn},
            $self->{username},
            $self->{password},
            $self->{options}
        ) or die "Cannot connect to database: $DBI::errstr";
        
        # Set up session parameters based on database type
        if ($self->{dsn} =~ /Pg:/) {
            $self->{dbh}->do("SET client_encoding TO 'UTF8'");
            $self->{dbh}->do("SET timezone TO 'UTC'");
        } elsif ($self->{dsn} =~ /mysql:/) {
            $self->{dbh}->do("SET NAMES utf8");
            $self->{dbh}->do("SET time_zone = '+00:00'");
        }
        
        return $self->{dbh};
    }
    
    sub disconnect {
        my $self = shift;
        if ($self->{dbh}) {
            $self->{dbh}->disconnect;
            $self->{dbh} = undef;
        }
    }
    
    sub begin_transaction {
        my $self = shift;
        $self->{dbh}->begin_work;
        $self->{in_transaction} = 1;
    }
    
    sub commit_transaction {
        my $self = shift;
        if ($self->{in_transaction}) {
            $self->{dbh}->commit;
            $self->{in_transaction} = 0;
        }
    }
    
    sub rollback_transaction {
        my $self = shift;
        if ($self->{in_transaction}) {
            $self->{dbh}->rollback;
            $self->{in_transaction} = 0;
        }
    }
    
    sub execute_query {
        my ($self, $sql, @params) = @_;
        
        try {
            my $sth = $self->{dbh}->prepare($sql);
            $sth->execute(@params);
            return $sth;
        } catch {
            die "Query execution failed: $_\nSQL: $sql\nParams: " . join(', ', @params);
        };
    }
    
    sub fetch_all {
        my ($self, $sql, @params) = @_;
        
        my $sth = $self->execute_query($sql, @params);
        return $sth->fetchall_arrayref({});
    }
    
    sub fetch_one {
        my ($self, $sql, @params) = @_;
        
        my $sth = $self->execute_query($sql, @params);
        return $sth->fetchrow_hashref;
    }
    
    sub fetch_value {
        my ($self, $sql, @params) = @_;
        
        my $sth = $self->execute_query($sql, @params);
        my ($value) = $sth->fetchrow_array;
        return $value;
    }
    
    sub insert {
        my ($self, $table, $data) = @_;
        
        my @columns = keys %$data;
        my @placeholders = map { '?' } @columns;
        my @values = values %$data;
        
        my $sql = sprintf(
            'INSERT INTO %s (%s) VALUES (%s)',
            $table,
            join(', ', @columns),
            join(', ', @placeholders)
        );
        
        my $sth = $self->execute_query($sql, @values);
        return $self->{dbh}->last_insert_id(undef, undef, $table, undef);
    }
    
    sub update {
        my ($self, $table, $data, $where, @where_params) = @_;
        
        my @set_clauses = map { "$_ = ?" } keys %$data;
        my @values = values %$data;
        
        my $sql = sprintf(
            'UPDATE %s SET %s WHERE %s',
            $table,
            join(', ', @set_clauses),
            $where
        );
        
        push @values, @where_params;
        my $sth = $self->execute_query($sql, @values);
        return $sth->rows;
    }
    
    sub delete {
        my ($self, $table, $where, @params) = @_;
        
        my $sql = "DELETE FROM $table WHERE $where";
        my $sth = $self->execute_query($sql, @params);
        return $sth->rows;
    }
    
    sub complex_join_query {
        my ($self, $params) = @_;
        
        my $sql = q{
            SELECT 
                u.id as user_id,
                u.username,
                u.email,
                p.first_name,
                p.last_name,
                p.bio,
                COUNT(o.id) as order_count,
                SUM(o.total_amount) as total_spent,
                MAX(o.created_at) as last_order_date
            FROM users u
            LEFT JOIN user_profiles p ON u.id = p.user_id
            LEFT JOIN orders o ON u.id = o.user_id
            WHERE 1=1
        };
        
        my @bind_params;
        
        if ($params->{username}) {
            $sql .= " AND u.username ILIKE ?";
            push @bind_params, '%' . $params->{username} . '%';
        }
        
        if ($params->{email}) {
            $sql .= " AND u.email ILIKE ?";
            push @bind_params, '%' . $params->{email} . '%';
        }
        
        if ($params->{min_orders}) {
            $sql .= " AND COUNT(o.id) >= ?";
            push @bind_params, $params->{min_orders};
        }
        
        if ($params->{date_from}) {
            $sql .= " AND u.created_at >= ?";
            push @bind_params, $params->{date_from};
        }
        
        if ($params->{date_to}) {
            $sql .= " AND u.created_at <= ?";
            push @bind_params, $params->{date_to};
        }
        
        $sql .= q{
            GROUP BY u.id, u.username, u.email, p.first_name, p.last_name, p.bio
            ORDER BY total_spent DESC
        };
        
        if ($params->{limit}) {
            $sql .= " LIMIT ?";
            push @bind_params, $params->{limit};
        }
        
        if ($params->{offset}) {
            $sql .= " OFFSET ?";
            push @bind_params, $params->{offset};
        }
        
        return $self->fetch_all($sql, @bind_params);
    }
    
    sub subquery_example {
        my ($self, $category_id) = @_;
        
        my $sql = q{
            SELECT p.id, p.name, p.price, p.category_id,
                   (SELECT AVG(rating) FROM product_reviews pr WHERE pr.product_id = p.id) as avg_rating,
                   (SELECT COUNT(*) FROM product_reviews pr WHERE pr.product_id = p.id) as review_count,
                   (SELECT name FROM categories c WHERE c.id = p.category_id) as category_name
            FROM products p
            WHERE p.id IN (
                SELECT product_id FROM product_categories 
                WHERE category_id = ?
            )
            AND p.active = 1
            ORDER BY avg_rating DESC, review_count DESC
        };
        
        return $self->fetch_all($sql, $category_id);
    }
    
    sub window_function_query {
        my ($self) = @_;
        
        my $sql = q{
            SELECT 
                id,
                username,
                email,
                created_at,
                ROW_NUMBER() OVER (ORDER BY created_at DESC) as user_rank,
                COUNT(*) OVER () as total_users,
                AVG(CASE WHEN created_at > CURRENT_DATE - INTERVAL '30 days' THEN 1 ELSE 0 END) OVER () as recent_signup_rate
            FROM users
            WHERE active = 1
        };
        
        return $self->fetch_all($sql);
    }
    
    sub cte_query {
        my ($self, $user_id) = @_;
        
        my $sql = q{
            WITH RECURSIVE user_hierarchy AS (
                SELECT id, username, manager_id, 0 as level
                FROM users
                WHERE id = ?
                
                UNION ALL
                
                SELECT u.id, u.username, u.manager_id, uh.level + 1
                FROM users u
                INNER JOIN user_hierarchy uh ON u.manager_id = uh.id
                WHERE uh.level < 5
            )
            SELECT * FROM user_hierarchy
            ORDER BY level, username
        };
        
        return $self->fetch_all($sql, $user_id);
    }
}

# DBIx::Class ORM Patterns
{
    package DBIxClassExample::Schema;
    
    use base 'DBIx::Class::Schema';
    
    __PACKAGE__->load_namespaces;
    
    1;
}

{
    package DBIxClassExample::Schema::Result::User;
    
    use base 'DBIx::Class::Core';
    
    __PACKAGE__->table('users');
    __PACKAGE__->add_columns(
        id => {
            data_type => 'integer',
            is_auto_increment => 1,
        },
        username => {
            data_type => 'varchar',
            size => 50,
            is_nullable => 0,
        },
        email => {
            data_type => 'varchar',
            size => 100,
            is_nullable => 0,
        },
        password_hash => {
            data_type => 'varchar',
            size => 255,
            is_nullable => 0,
        },
        created_at => {
            data_type => 'timestamp',
            is_nullable => 0,
            default_value => \'CURRENT_TIMESTAMP',
        },
        updated_at => {
            data_type => 'timestamp',
            is_nullable => 0,
            default_value => \'CURRENT_TIMESTAMP',
        },
        active => {
            data_type => 'boolean',
            default_value => 1,
        },
    );
    
    __PACKAGE__->set_primary_key('id');
    __PACKAGE__->add_unique_constraint(['username']);
    __PACKAGE__->add_unique_constraint(['email']);
    
    __PACKAGE__->has_many(
        'orders',
        'DBIxClassExample::Schema::Result::Order',
        'user_id'
    );
    
    __PACKAGE__->has_one(
        'profile',
        'DBIxClassExample::Schema::Result::UserProfile',
        'user_id'
    );
    
    __PACKAGE__->many_to_many(
        'roles',
        'user_roles',
        'role'
    );
    
    sub sqlt_deploy_hook {
        my ($self, $sqlt_table) = @_;
        
        $sqlt_table->add_index(name => 'idx_users_username', fields => ['username']);
        $sqlt_table->add_index(name => 'idx_users_email', fields => ['email']);
        $sqlt_table->add_index(name => 'idx_users_created_at', fields => ['created_at']);
    }
    
    sub insert {
        my ($self, @args) = @_;
        
        # Hash password before insertion
        if ($self->password_hash && $self->password_hash !~ /^[a-f0-9]{32}$/) {
            $self->password_hash(Digest::MD5::md5_hex($self->password_hash));
        }
        
        return $self->next::method(@args);
    }
    
    sub update {
        my ($self, @args) = @_;
        
        # Hash password if it's being updated
        if ($self->is_column_changed('password_hash')) {
            my $password = $self->password_hash;
            if ($password && $password !~ /^[a-f0-9]{32}$/) {
                $self->password_hash(Digest::MD5::md5_hex($password));
            }
        }
        
        $self->updated_at(\'CURRENT_TIMESTAMP');
        return $self->next::method(@args);
    }
    
    sub full_name {
        my $self = shift;
        return $self->profile ? $self->profile->full_name : $self->username;
    }
    
    sub has_role {
        my ($self, $role_name) = @_;
        return $self->roles->search({ name => $role_name })->count > 0;
    }
    
    sub total_orders {
        my $self = shift;
        return $self->orders->count;
    }
    
    sub total_spent {
        my $self = shift;
        my $sum = $self->orders->get_column('total_amount')->sum;
        return $sum || 0;
    }
}

{
    package DBIxClassExample::Schema::Result::Order;
    
    use base 'DBIx::Class::Core';
    
    __PACKAGE__->table('orders');
    __PACKAGE__->add_columns(
        id => {
            data_type => 'integer',
            is_auto_increment => 1,
        },
        user_id => {
            data_type => 'integer',
            is_foreign_key => 1,
            is_nullable => 0,
        },
        order_number => {
            data_type => 'varchar',
            size => 50,
            is_nullable => 0,
        },
        status => {
            data_type => 'varchar',
            size => 20,
            default_value => 'pending',
        },
        total_amount => {
            data_type => 'numeric',
            size => [10, 2],
            default_value => 0,
        },
        created_at => {
            data_type => 'timestamp',
            default_value => \'CURRENT_TIMESTAMP',
        },
        shipped_at => {
            data_type => 'timestamp',
            is_nullable => 1,
        },
    );
    
    __PACKAGE__->set_primary_key('id');
    __PACKAGE__->add_unique_constraint(['order_number']);
    
    __PACKAGE__->belongs_to(
        'user',
        'DBIxClassExample::Schema::Result::User',
        'user_id'
    );
    
    __PACKAGE__->has_many(
        'items',
        'DBIxClassExample::Schema::Result::OrderItem',
        'order_id'
    );
    
    sub is_shipped {
        my $self = shift;
        return defined $self->shipped_at;
    }
    
    sub mark_as_shipped {
        my $self = shift;
        $self->status('shipped');
        $self->shipped_at(\'CURRENT_TIMESTAMP');
        $self->update;
    }
    
    sub calculate_total {
        my $self = shift;
        
        my $total = $self->items->get_column('quantity * price')->sum;
        $self->total_amount($total || 0);
        $self->update;
        
        return $self->total_amount;
    }
}

# Connection Pooling Patterns
{
    package ConnectionPoolExample::PoolManager;
    
    use DBI;
    use POSIX qw(:sys_wait_h);
    use Time::HiRes qw(sleep);
    use Scalar::Util qw(weaken);
    
    sub new {
        my ($class, $config) = @_;
        
        my $self = bless {
            config => $config,
            min_connections => $config->{min_connections} || 2,
            max_connections => $config->{max_connections} || 10,
            connection_timeout => $config->{connection_timeout} || 30,
            idle_timeout => $config->{idle_timeout} || 300,
            check_interval => $config->{check_interval} || 60,
            
            pool => [],
            available => [],
            in_use => {},
            last_check => time(),
        }, $class;
        
        $self->initialize_pool;
        $self->start_maintenance_thread;
        
        return $self;
    }
    
    sub initialize_pool {
        my $self = shift;
        
        for my $i (1..$self->{min_connections}) {
            my $dbh = $self->create_connection;
            push @{$self->{pool}}, $dbh;
            push @{$self->{available}}, $dbh;
        }
    }
    
    sub create_connection {
        my $self = shift;
        
        my $dbh = DBI->connect(
            $self->{config}{dsn},
            $self->{config}{username},
            $self->{config}{password},
            {
                RaiseError => 1,
                AutoCommit => 1,
                PrintError => 0,
                mysql_auto_reconnect => 1,
                pg_auto_reconnect => 1,
            }
        ) or die "Cannot create database connection: $DBI::errstr";
        
        # Store metadata
        $dbh->{created_at} = time();
        $dbh->{last_used} = time();
        $dbh->{in_use} = 0;
        
        return $dbh;
    }
    
    sub get_connection {
        my $self = shift;
        
        # Try to get an available connection
        while (my $dbh = shift @{$self->{available}}) {
            if ($self->is_connection_valid($dbh)) {
                $dbh->{in_use} = 1;
                $dbh->{last_used} = time();
                $self->{in_use}{"$dbh"} = $dbh;
                weaken($self->{in_use}{"$dbh"});
                return $dbh;
            } else {
                # Remove invalid connection from pool
                @{$self->{pool}} = grep { $_ != $dbh } @{$self->{pool}};
            }
        }
        
        # No available connections, try to create a new one
        if (scalar(@{$self->{pool}}) < $self->{max_connections}) {
            my $dbh = $self->create_connection;
            push @{$self->{pool}}, $dbh;
            $dbh->{in_use} = 1;
            $dbh->{last_used} = time();
            $self->{in_use}{"$dbh"} = $dbh;
            weaken($self->{in_use}{"$dbh"});
            return $dbh;
        }
        
        # Pool is full, wait for a connection to become available
        my $timeout = time() + $self->{connection_timeout};
        while (time() < $timeout) {
            sleep(0.1);
            if (my $dbh = shift @{$self->{available}}) {
                if ($self->is_connection_valid($dbh)) {
                    $dbh->{in_use} = 1;
                    $dbh->{last_used} = time();
                    $self->{in_use}{"$dbh"} = $dbh;
                    weaken($self->{in_use}{"$dbh"});
                    return $dbh;
                } else {
                    @{$self->{pool}} = grep { $_ != $dbh } @{$self->{pool}};
                }
            }
        }
        
        die "Connection timeout: unable to get database connection";
    }
    
    sub release_connection {
        my ($self, $dbh) = @_;
        
        return unless $dbh;
        
        my $dbh_key = "$dbh";
        if (exists $self->{in_use}{$dbh_key}) {
            delete $self->{in_use}{$dbh_key};
            $dbh->{in_use} = 0;
            $dbh->{last_used} = time();
            push @{$self->{available}}, $dbh;
        }
    }
    
    sub is_connection_valid {
        my ($self, $dbh) = @_;
        
        return 0 unless $dbh;
        
        # Check if connection is still alive
        eval {
            $dbh->ping or die "Connection not alive";
            $dbh->do('SELECT 1') or die "Cannot execute test query";
        };
        
        if ($@) {
            warn "Invalid connection detected: $@";
            return 0;
        }
        
        return 1;
    }
    
    sub start_maintenance_thread {
        my $self = shift;
        
        my $pid = fork;
        return if $pid;  # Parent process
        
        # Child process - maintenance thread
        while (1) {
            sleep($self->{check_interval});
            $self->maintenance_check;
        }
    }
    
    sub maintenance_check {
        my $self = shift;
        
        my $now = time();
        
        # Check for idle connections to close
        for my $dbh (@{$self->{pool}}) {
            next if $dbh->{in_use};
            
            if ($now - $dbh->{last_used} > $self->{idle_timeout}) {
                # Close idle connection
                eval { $dbh->disconnect };
                @{$self->{pool}} = grep { $_ != $dbh } @{$self->{pool}};
                @{$self->{available}} = grep { $_ != $dbh } @{$self->{available}};
            }
        }
        
        # Ensure minimum connections
        my $active_connections = scalar(@{$self->{pool}});
        if ($active_connections < $self->{min_connections}) {
            for my $i (1..($self->{min_connections} - $active_connections)) {
                my $dbh = $self->create_connection;
                push @{$self->{pool}}, $dbh;
                push @{$self->{available}}, $dbh;
            }
        }
        
        $self->{last_check} = $now;
    }
    
    sub get_stats {
        my $self = shift;
        
        return {
            total_connections => scalar(@{$self->{pool}}),
            available_connections => scalar(@{$self->{available}}),
            in_use_connections => scalar(keys %{$self->{in_use}}),
            last_check => $self->{last_check},
        };
    }
}

# Transaction Handling Patterns
{
    package TransactionExample::TransactionManager;
    
    use Try::Tiny;
    
    sub new {
        my ($class, $db_manager) = @_;
        return bless { db_manager => $db_manager }, $class;
    }
    
    sub transaction {
        my ($self, $code) = @_;
        
        my $db = $self->{db_manager};
        $db->begin_transaction;
        
        try {
            my $result = $code->($db);
            $db->commit_transaction;
            return $result;
        } catch {
            $db->rollback_transaction;
            die "Transaction failed: $_";
        };
    }
    
    sub nested_transaction {
        my ($self, $code) = @_;
        
        # PostgreSQL supports nested transactions via savepoints
        my $db = $self->{db_manager};
        
        if ($db->{in_transaction}) {
            # Create savepoint
            my $savepoint = 'sp_' . time();
            $db->{dbh}->do("SAVEPOINT $savepoint");
            
            try {
                my $result = $code->($db);
                $db->{dbh}->do("RELEASE SAVEPOINT $savepoint");
                return $result;
            } catch {
                $db->{dbh}->do("ROLLBACK TO SAVEPOINT $savepoint");
                die "Nested transaction failed: $_";
            };
        } else {
            return $self->transaction($code);
        }
    }
    
    sub distributed_transaction {
        my ($self, $databases, $code) = @_;
        
        # Begin transactions on all databases
        for my $db (@$databases) {
            $db->begin_transaction;
        }
        
        try {
            my $result = $code->(@$databases);
            
            # Commit all transactions
            for my $db (@$databases) {
                $db->commit_transaction;
            }
            
            return $result;
        } catch {
            # Rollback all transactions
            for my $db (@$databases) {
                eval { $db->rollback_transaction };
            }
            
            die "Distributed transaction failed: $_";
        };
    }
}

# Usage examples
package main;

print "=== Database Integration Patterns Test ===\n";

# Simulate database operations
print "DBI: Complex JOIN queries, subqueries, window functions, CTEs\n";
print "DBIx::Class: ORM relationships, result sets, custom methods\n";
print "Connection Pooling: Min/max connections, timeout handling, maintenance\n";
print "Transactions: Simple, nested, distributed transaction patterns\n";

print "\n=== Database Integration Patterns Test Complete ===\n";