$x=$obj->build()->name();
return $obj->find($id)->wrap(foo(1),{ok=>1});
