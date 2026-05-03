package InheritedParent;

sub inherited {
    return 1;
}

package InheritedChild;

use parent 'InheritedParent';

inherited();
