#!/usr/bin/env perl
use v5.24;
use strict;
use warnings;

# Real-world CPAN ecosystem patterns collected from common production styles.
# Exercises DSL-like APIs, method chaining, callback-heavy code, and pragma blocks.

package MyApp::Model::User {
    use Moo;
    use Types::Standard qw(Str Int Maybe HashRef ArrayRef);
    use namespace::clean;

    has id => (
        is       => 'ro',
        isa      => Int,
        required => 1,
    );

    has email => (
        is       => 'rw',
        isa      => Str,
        required => 1,
    );

    has profile => (
        is      => 'rw',
        isa     => HashRef,
        default => sub { +{} },
    );

    has roles => (
        is      => 'rw',
        isa     => ArrayRef[Str],
        default => sub { ['user'] },
    );

    sub add_role ($self, $role) {
        push @{ $self->roles }, $role if defined $role && length $role;
        return $self;
    }

    sub as_hashref ($self) {
        return {
            id      => $self->id,
            email   => $self->email,
            profile => { %{ $self->profile } },
            roles   => [ @{ $self->roles } ],
        };
    }
}

package MyApp::Service::UserRepo {
    use DBI;
    use Try::Tiny;
    use Scalar::Util qw(blessed);

    sub new ($class, %args) {
        my $dsn  = $args{dsn}  // 'dbi:SQLite:dbname=:memory:';
        my $user = $args{user} // q{};
        my $pass = $args{pass} // q{};

        my $dbh = DBI->connect(
            $dsn,
            $user,
            $pass,
            {
                RaiseError => 1,
                AutoCommit => 1,
                PrintError => 0,
            },
        );

        return bless { dbh => $dbh }, $class;
    }

    sub find_user_by_email ($self, $email) {
        my $sql = q{
            SELECT id, email
            FROM users
            WHERE email = ?
            LIMIT 1
        };

        my $row = $self->{dbh}->selectrow_hashref($sql, undef, $email);
        return unless $row;

        return MyApp::Model::User->new(
            id      => $row->{id},
            email   => $row->{email},
            profile => {},
        );
    }

    sub with_transaction ($self, $callback) {
        my $dbh = $self->{dbh};
        $dbh->begin_work;

        my $result;
        try {
            $result = $callback->($dbh);
            $dbh->commit;
        }
        catch {
            my $err = $_;
            try { $dbh->rollback }
            catch { warn "rollback failed: $_" };
            die $err;
        };

        return $result;
    }

    sub describe_value ($self, $value) {
        return 'undef' unless defined $value;
        return 'object(' . blessed($value) . ')' if blessed $value;
        return 'scalar(' . $value . ')';
    }
}

package MyApp::Web {
    use Mojolicious::Lite -signatures;

    helper json_ok => sub ($c, $payload) {
        $c->res->headers->content_type('application/json');
        return $c->render(json => { ok => 1, %{$payload // {}} });
    };

    any [qw(GET POST)] => '/api/users/:id' => sub ($c) {
        my $id = $c->param('id');
        state $cache = {};

        if (my $cached = $cache->{$id}) {
            return $c->json_ok({ source => 'cache', user => $cached });
        }

        my $user = {
            id    => $id + 0,
            email => sprintf('user%s@example.test', $id),
            tags  => [grep { defined } qw(active beta)],
        };

        $cache->{$id} = $user;
        return $c->json_ok({ source => 'computed', user => $user });
    };

    app->hook(before_dispatch => sub ($c) {
        $c->stash(started_at => time);
    });

    app->hook(after_dispatch => sub ($c) {
        my $elapsed = time - ($c->stash('started_at') // time);
        $c->res->headers->header('X-Elapsed' => $elapsed);
    });
}

package main;

sub build_pipeline ($input) {
    my $step = sub ($v, $name) { +{ name => $name, value => $v } };

    return $step->(
        [
            map { $_->{value} * 2 }
            grep { $_->{value} % 2 == 0 }
            map { $step->($_, 'stage_' . $_) }
            @{ $input // [] }
        ],
        'done'
    );
}

my $pipeline = build_pipeline([ 1 .. 10 ]);
print $pipeline->{name}, "\n" if $pipeline->{name} eq 'done';

1;
