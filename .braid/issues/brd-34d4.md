---
schema_version: 6
id: brd-34d4
title: separate priority emphasis from meta styling in brd ls/ready
priority: P1
status: done
deps: []
tags:
- visual
owner: null
created_at: 2025-12-28T13:12:35.033894Z
updated_at: 2025-12-28T15:59:38.256385Z
acceptance:
- priority emphasis does not use the same bold styling as meta issues
- brd ls and brd ready both reflect the updated styling
---

high priority items are bold in brd ls/ready. meta issues are also bold, so priority and meta are hard to distinguish. adjust styling so meta stands out without masking priority. consider styling only the priority column or using a different attribute than bold.
