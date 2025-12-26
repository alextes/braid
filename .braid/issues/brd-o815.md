---
brd: 1
id: brd-o815
title: improve error message for acceptance criteria with colons
priority: P1
status: done
deps: []
created_at: 2025-12-26T16:44:10.732419Z
updated_at: 2025-12-26T16:44:10.732419Z
acceptance:
- error message includes actionable hint about quoting
- brd ls output format is not disrupted by parse errors
---

acceptance criteria containing colons (e.g. `- foo: bar`) get parsed as YAML maps instead of strings. rather than lenient parsing, we provide a clear error message with a hint about proper quoting.

the fix adds a hint after the parse error:
```
warning: failed to load .braid/issues/foo.md: parse error in issue frontmatter:
acceptance[0]: invalid type: map, expected a string at line 10 column 3
  hint: strings containing colons must be quoted, e.g. '- "foo: bar"'
```

parse errors go to stderr, so they don't disrupt the formatted output of `brd ls`.
