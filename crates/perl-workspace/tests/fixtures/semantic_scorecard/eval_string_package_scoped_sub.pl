package EvalStringPackageScopedSub;

# Subs declared inside `eval "package NAME; sub SUBNAME { }"` are indexed
# under their fully-qualified name (e.g. Dynamic::generated) so that
# workspace symbol lookup and PL109 suppression can find them.

# Case 1: single package context
eval "package Dynamic; sub generated { 1 }";
Dynamic::generated();  # should resolve to the eval-indexed sub

# Case 2: package switch mid-eval (both subs indexed under their own package)
eval "package A; sub make_a { 1 } package B; sub make_b { 2 }";

# Case 3: sub declared before any package keyword — remains unscoped
eval "sub bare_helper { 42 }";
