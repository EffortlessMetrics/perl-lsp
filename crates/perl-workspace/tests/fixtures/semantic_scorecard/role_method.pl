package ExampleRole;

sub role_method {
    return 1;
}

package RoleConsumer;

use Role::Tiny::With;
with 'ExampleRole';

role_method();
