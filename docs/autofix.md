# Autofix

```bash
odcslint check --fix
```

Findings marked `[*]` in text output (or `"fixable": true` in structured output) can be fixed automatically. With `--fix`, odcslint rewrites those files in place and then reports whatever is left. Right now the only fixable rule is [`property_order`](rules/property_order.md).

## Formatting is preserved

Fixes never re-serialize the YAML. They move pieces of the original text around, which is why your formatting survives. A comment directly above an item, or inside it, moves together with that item. Block scalars (`|`, `>`), quoting and indentation are left untouched. Blank lines between items stay where they were, and so do comments indented less than the items, such as a comment that closes the list. Line endings (LF or CRLF) are kept, and so is a missing newline at the end of the file.

```diff
     properties:
-      # Load metadata, added by the ingestion framework.
-      - name: _loaded_at
-        logicalType: timestamp
       - name: order_id
         primaryKey: true
         primaryKeyPosition: 1
       - name: amount
+      # Load metadata, added by the ingestion framework.
+      - name: _loaded_at
+        logicalType: timestamp
```

## Safety

odcslint checks every fix pass before keeping it. It parses the fixed file again, and the content has to match the original exactly, apart from the lists it meant to reorder. If it doesn't, that pass is discarded and odcslint reports a `fix_failed` warning; fixes from earlier passes are kept. Files are written to a temporary file first and then renamed, so an interrupted run can't leave a half-written contract behind.

Nested lists, such as the `properties` of an object column, are fixed in later passes. Each file gets at most 10 passes.

## When a finding cannot be fixed

Some lists can't be rearranged safely as text, so their `property_order` findings come without `[*]`. That happens with flow-style lists (`properties: [{name: a}, {name: b}]`) and with items that don't start on their own line.
