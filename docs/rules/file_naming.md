# `file_naming`

Checks the contract's file name. You can require it to match a glob, require its stem to equal the contract `id` or `name`, or both. The stem is the file name without `.odcs.yaml`, `.odcs.yml`, `.yaml` or `.yml`.

## Options

| Option | Description |
|---|---|
| `pattern` | A glob the file name (not the directory) must match, e.g. `*.odcs.yaml`. |
| `match_field` | `id` or `name`: the file stem must equal this field. |

You need to set at least one of the two.

## Example

**Config**

=== "YAML"

    ```yaml
    rules:
      - type: file_naming
        pattern: '*.odcs.yaml'
        match_field: name
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "file_naming"
    pattern = "*.odcs.yaml"
    match_field = "name"
    ```

**Contract**

```yaml title="orders.odcs.yml"
name: sales_orders  # file is orders.odcs.yml: wrong extension and not sales_orders
```

**Findings**

```text
orders.odcs.yml
  1:1  error    file_naming  file name `orders.odcs.yml` does not match `*.odcs.yaml`
  1:7  error    file_naming  file name `orders.odcs.yml` does not match contract name `sales_orders`
```
