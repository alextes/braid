---
brd: 1
id: brd-o815
title: fix acceptance criteria parsing error with colons
priority: P1
status: todo
deps: []
created_at: 2025-12-26T16:44:10.732419Z
updated_at: 2025-12-26T16:44:10.732419Z
acceptance:
- acceptance criteria with colons parse correctly
- no warning when loading issues with colons in acceptance
---

acceptance criteria containing colons (e.g. `- foo: bar`) get parsed as YAML maps instead of strings, causing parse errors:

```
warning: failed to load .braid/issues/brd-77te.md: parse error in issue frontmatter:
acceptance[0]: invalid type: map, expected a string at line 10 column 3
```

options:
1. quote acceptance strings when writing (safest)
2. accept both strings and maps when parsing (lenient)
3. document to avoid colons (workaround)

the immediate issue was fixed by rephrasing, but the parser should handle this gracefully.
