#!/usr/bin/env bash
# Generate the xlarge (10 000 file) fixture workspace for perl-workspace-index benchmarks.
#
# Output: test_corpus/workspaces/xlarge/ relative to the repo root.
# The files are intentionally NOT committed; this script regenerates them on demand.
#
# Usage:
#   bash scripts/gen-xlarge-workspace.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_DIR="${REPO_ROOT}/test_corpus/workspaces/xlarge"

echo "Generating xlarge workspace at: ${TARGET_DIR}"

mkdir -p "${TARGET_DIR}/bin"

# 9 buckets * 1100 files = 9900 lib files
for bucket in A B C D E F G H I; do
    mkdir -p "${TARGET_DIR}/lib/${bucket}"
    for i in $(seq 1 1100); do
        cat > "${TARGET_DIR}/lib/${bucket}/Module${bucket}${i}.pm" << EOF
package Module${bucket}${i};
use strict;
use warnings;

our \$VERSION = '1.00';

sub new {
    my (\$class) = @_;
    return bless {}, \$class;
}

sub operate_${i} {
    my (\$self, \$n) = @_;
    return \$n + ${i};
}

1;
EOF
    done
    echo "  bucket ${bucket} done (1100 files)"
done

# 100 bin files
for i in $(seq 1 100); do
    cat > "${TARGET_DIR}/bin/main${i}.pl" << EOF
#!/usr/bin/env perl
use strict;
use warnings;
print "entry point ${i}\n";
EOF
done

TOTAL=$(find "${TARGET_DIR}" -name '*.p[lm]' | wc -l)
echo "Done. xlarge workspace has ${TOTAL} Perl files."
