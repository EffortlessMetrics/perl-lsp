#!/usr/bin/env perl
# Valid 'local' variable declaration test fixtures
# Tests for AC1: Variable declaration error handling

# Basic scalar localization
local $global_scalar = 1;

# Basic array localization
local @global_array = (1, 2, 3);

# Basic hash localization
local %global_hash = (key => 'value');

# Localize special variable
local $/ = "\n";
local $\ = "";

# Localize array element
local $array[0] = 'value';

# Localize hash element
local $hash{key} = 'value';
