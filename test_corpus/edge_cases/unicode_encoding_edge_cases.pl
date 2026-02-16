#!/usr/bin/env perl
# Test: Unicode and encoding edge cases
# Impact: Test parser's handling of Unicode and various encodings

use strict;
use warnings;
use utf8;
use open ':std', ':encoding(UTF-8)';

# Test 1: Unicode identifiers with complex scripts
# Japanese identifiers
my $変数 = 1;
my $配列 = [1, 2, 3];
my %ハッシュ = (キー => "値");

# Arabic identifiers
my $متغير = 2;
my $مصفوفة = [4, 5, 6];
my %هاش = (مفتاح => "قيمة");

# Chinese identifiers
my $变量 = 3;
my $数组 = [7, 8, 9];
my %哈希 = (键 => "值");

# Cyrillic identifiers
my $переменная = 4;
my $массив = [10, 11, 12];
my %хеш = (ключ => "значение");

# Hebrew identifiers
my $משתנה = 5;
my $מערך = [13, 14, 15];
my %האש = (מפתח => "ערך");

# Mixed Unicode identifiers
sub 日本語の関数 { return "日本語" }
sub العربية_الدالة { return "العربية" }
sub 中文函数 { return "中文" }

my $result1 = 日本語の関数();
my $result2 = العربية_الدالة();
my $result3 = 中文函数();

# Test 2: Unicode in string literals and regex patterns
my $unicode_string1 = "Hello 世界";
my $unicode_string2 = "مرحبا بالعالم";
my $unicode_string3 = "Привет мир";
my $unicode_string4 = "שלום עולם";

# Unicode in regex patterns
my $unicode_regex1 = qr/世界/;
my $unicode_regex2 = qr/العالم/;
my $unicode_regex3 = qr/мир/;
my $unicode_regex4 = qr/עולם/;

# Unicode character classes
my $unicode_class1 = qr/\p{Han}+/;  # Chinese characters
my $unicode_class2 = qr/\p{Arabic}+/;  # Arabic characters
my $unicode_class3 = qr/\p{Cyrillic}+/;  # Cyrillic characters
my $unicode_class4 = qr/\p{Hebrew}+/;  # Hebrew characters

# Test 3: Bidirectional text and combining characters
# Right-to-left text
my $rtl_text = "שלום Hello مرحبا";
my $mixed_text = "English العربية English";

# Combining characters
my $combining1 = "e\u{0301}";  # e + acute accent
my $combining2 = "o\u{0308}";  # o + diaeresis
my $combining3 = "u\u{0308}\u{0301}";  # u + diaeresis + acute

# Zero-width characters
my $zero_width1 = "text\u{200B}more";  # Zero-width space
my $zero_width2 = "text\u{200C}more";  # Zero-width non-joiner
my $zero_width3 = "text\u{200D}more";  # Zero-width joiner

# Test 4: Unicode normalization forms
# NFD (Normalization Form D)
my $nfd_text = "e\u{0301}ca\u{0308}";  # Decomposed

# NFC (Normalization Form C)
my $nfc_text = "\u{00E9}ca\u{0308}";  # Composed first character

# NFKD (Normalization Form KD)
my $nfkd_text = "e\u{0301}ca\u{0308}";  # Compatibility decomposition

# NFKC (Normalization Form KC)
my $nfkc_text = "\u{00E9}ca\u{0308}";  # Compatibility composition

# Test 5: UTF-8 encoding issues and BOM handling
# BOM (Byte Order Mark)
my $bom_string = "\u{FEFF}BOM at start";

# Invalid UTF-8 sequences (these should be handled gracefully)
# Note: These are represented as Unicode replacement characters
my $invalid1 = "Valid\x{FFFD}Invalid";
my $invalid2 = "Start\x{FFFD}\x{FFFD}End";

# Overlong encodings and surrogate pairs
my $surrogate1 = "\x{D800}\x{DC00}";  # Basic Multilingual Plane
my $surrogate2 = "\x{D83D}\x{DE00}";  # Emoji (grinning face)

# Test 6: Unicode in various Perl constructs
# Unicode in package names
package 包名测试;
sub 新函数 { return "测试" }
package main;

# Unicode in hash keys
my %unicode_hash = (
    "中文键" => "中文值",
    "العربية" => "قيمة",
    "ключ" => "значение",
    "מפתח" => "ערך"
);

# Unicode in array elements
my @unicode_array = (
    "元素一",
    "العنصر الثاني",
    "третий элемент",
    "אלמנט רביעי"
);

# Test 7: Unicode with quote-like operators
my $unicode_q = q{Unicode in q{}};
my $unicode_qq = qq{Unicode in qq{} with $変数};
my $unicode_qw = qw{Unicode 词 列表};
my $unicode_qr = qr/Unicode 正则表达式/;

# With different delimiters
my $unicode_q1 = q[Unicode in brackets];
my $unicode_q2 = q(Unicode in parentheses);
my $unicode_q3 = q<Unicode in angles>;

# Test 8: Unicode in heredocs
my $unicode_heredoc = <<'UNICODE';
This heredoc contains Unicode:
日本語
العربية
Русский
עברית
中文
UNICODE

my $unicode_heredoc_interp = <<"UNICODE_INTERP";
This heredoc interpolates Unicode: $変数
Also contains: $unicode_string1
UNICODE_INTERP

# Test 9: Unicode with special variables
# Unicode in $1, $2, etc. from regex captures
"世界世界" =~ /(.)\1/;
my $capture1 = $1;  # Should contain "世"

# Unicode in @ARGV simulation
my @unicode_argv = ("脚本.pl", "参数一", "parameter2", "بارامتر");

# Unicode in %ENV simulation
my %unicode_env = (
    "PATH" => "/usr/bin",
    "中文环境变量" => "中文值",
    "HOME" => "/home/user"
);

# Test 10: Unicode with file handles
# Simulating Unicode file operations
my $unicode_filename = "unicode_测试_ملف.txt";
my $unicode_content = "Content with Unicode: 文字, حروف, буквы";

# Test 11: Unicode in format strings
my $format1 = "%-10s %10s\n";
my $format2 = "%-10s %10s\n";

printf $format1, "English", "Value";
printf $format2, "中文", "值";
printf $format1, "العربية", "قيمة";
printf $format2, "Русский", "значение";

# Test 12: Unicode with pack/unpack
my $packed = pack("U*", 0x4E2D, 0x6587);  # "中文" in Unicode code points
my @unpacked = unpack("U*", $packed);

# Test 13: Unicode with tr///
my $tr_text = "café";
$tr_text =~ tr/é/e/;  # Remove accent

# Test 14: Unicode with sprintf/printf formats
my $formatted = sprintf("Unicode: %s, Number: %d", "测试", 42);
my $printf_result = printf("Unicode: %s, Hex: %x\n", "العربية", 255);

# Test 15: Unicode with sort and comparison
my @unicode_list = ("zebra", "ábc", "def", "中文", "العربية");
my @sorted_unicode = sort @unicode_list;

# Unicode comparison
my $cmp1 = ("café" cmp "cafe");
my $cmp2 = ("中文" cmp "English");

print "Unicode and encoding edge cases test completed\n";
print "Variables: $変数, $متغير, $变量, $переменная, $משתנה\n";
print "Results: $result1, $result2, $result3\n";