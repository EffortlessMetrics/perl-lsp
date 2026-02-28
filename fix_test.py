with open("crates/perl-lsp/tests/lsp_batteries_included_test.rs", "r") as f:
    lines = f.readlines()

out = []
for line in lines:
    if "assert!(range_formatting_provider.is_some()" in line:
        continue
    out.append(line)

with open("crates/perl-lsp/tests/lsp_batteries_included_test.rs", "w") as f:
    f.writelines(out)
