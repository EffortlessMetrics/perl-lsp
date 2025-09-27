# Minimal reproduction case for transliteration parser logic error
# Issue: extract_transliteration_parts("tr/abc/xyz/") returns ("abc", "xyz", "")
# Expected: ("abc", "", "xyz")
# Root cause: Non-paired delimiter logic incorrectly treats third segment as replacement instead of modifiers

tr/abc/xyz/
y/search/replace/dgs
tr{pattern}{replacement}cd