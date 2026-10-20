# `property_order`

Checks that the properties (columns) of every schema object come in this order, and can fix them with `--fix`:

1. Primary keys, by `primaryKeyPosition`. Primary keys without a position come after the ones that have one.
2. Regular columns, in their current order.
3. Metadata columns, in their current order.

Metadata columns are the ones your platform adds rather than the source: load timestamps, batch ids, row hashes, validity ranges and so on. By default, any column whose name starts with `_` counts as one. A primary key that also looks like a metadata column, such as `_id`, is treated as a primary key.

## Options

| Option | Default | Description |
|---|---|---|
| `primary_keys_first` | `true` | Put primary keys first. |
| `metadata_columns.prefixes` | `["_"]` | Names starting with one of these are metadata columns. |
| `metadata_columns.suffixes` | `[]` | Names ending with one of these are metadata columns. |
| `metadata_columns.names` | `[]` | Exact metadata column names. |
| `metadata_columns.pattern` | | A regex; names it matches are metadata columns. |
| `recursive` | `true` | Also check nested `properties` of object columns and `items.properties` of array columns. |

A column is a metadata column if any of the four matchers applies. Any key you leave out of `metadata_columns` keeps its default, so setting only `suffixes` still keeps `_` as a prefix. To stop treating `_` columns as metadata, set `prefixes: []`.

## Examples

The simplest setup uses the default: columns starting with `_` go last.

```yaml
rules:
  - type: property_order
```

Most warehouses use more than one naming scheme for metadata, though. This config combines all four matchers:

=== "YAML"

    ```yaml
    rules:
      - type: property_order
        metadata_columns:
          prefixes: ["_", "dw_"]           # _loaded_at, dw_source_system
          suffixes: ["_hash"]              # row_hash
          names: [etl_batch_id, ingested_at]
          pattern: '^valid_(from|to)$'     # valid_from, valid_to
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "property_order"

    [rules.metadata_columns]
    prefixes = ["_", "dw_"]
    suffixes = ["_hash"]
    names = ["etl_batch_id", "ingested_at"]
    pattern = '^valid_(from|to)$'
    ```

With that config, this list of properties:

```yaml
properties:
  - name: dw_source_system
  - name: order_id
    primaryKey: true
    primaryKeyPosition: 1
  - name: row_hash
  - name: customer_id
  - name: valid_from
  - name: amount
  - name: etl_batch_id
  - name: line_number
    primaryKey: true
    primaryKeyPosition: 2
  - name: _loaded_at
  - name: valid_to
  - name: status
```

is fixed to:

```yaml
properties:
  - name: order_id
    primaryKey: true
    primaryKeyPosition: 1
  - name: line_number
    primaryKey: true
    primaryKeyPosition: 2
  - name: customer_id
  - name: amount
  - name: status
  - name: dw_source_system
  - name: row_hash
  - name: valid_from
  - name: etl_batch_id
  - name: _loaded_at
  - name: valid_to
```

The primary keys move to the top in `primaryKeyPosition` order. `customer_id`, `amount` and `status` match none of the matchers, so they stay regular columns. The six metadata columns move to the end and keep their original order relative to each other; the rule doesn't sort them by matcher.

## Findings

You get one finding per list, at the first property that is out of place:

```text
orders.odcs.yaml
  13:9  error    property_order  expected `order_id` at position 1 of `orders`, found `_loaded_at` [*]
```

## Fix

`odcslint check --fix` moves the property blocks into the expected order and keeps comments, block scalars and formatting intact. [Autofix](../autofix.md) explains how.

```diff
     properties:
-      - name: _loaded_at
-        logicalType: timestamp
       - name: order_id
         primaryKey: true
         primaryKeyPosition: 1
       - name: amount
+      - name: _loaded_at
+        logicalType: timestamp
```

Flow-style lists (`properties: [{...}, {...}]`) are reported, but not fixed.
