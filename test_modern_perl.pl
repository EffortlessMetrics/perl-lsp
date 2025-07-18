use feature 'try';
try {
    dangerous_code();
} catch ($e) {
    warn "Error: $e";
} finally {
    cleanup();
}

use feature 'defer';
{
    defer { print "cleanup" }
    do_work();
}

use feature 'class';
class Point {
    field $x :param = 0;
    field $y :param = 0;
    
    method move ($dx, $dy) {
        $x += $dx;
        $y += $dy;
    }
}