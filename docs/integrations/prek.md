# prek

odcslint comes with two [prek](https://github.com/j178/prek) hooks. prek reads the same config format as [pre-commit](https://pre-commit.com/), so the hooks work with either tool.

| Hook | Runs |
|---|---|
| `odcslint` | `odcslint check --force-exclude --allow-empty` on changed contracts |
| `odcslint-fix` | `odcslint check --fix --force-exclude --allow-empty` on changed contracts |

```yaml title=".pre-commit-config.yaml"
repos:
  - repo: https://github.com/feliblo/odcslint
    rev: v0.2.0
    hooks:
      - id: odcslint-fix   # or `odcslint` to only check
```

You don't need to install anything else. The hooks use `language: rust`: on the first run, prek downloads a Rust toolchain if you don't have one and builds odcslint from this repository. That takes about half a minute and happens once per version; after that the hook starts instantly.

The hooks only run on files matching `\.odcs\.ya?ml$`. Because prek passes those files to odcslint explicitly, the hooks add `--force-exclude`; otherwise `settings.exclude` from your config would be ignored. They also add `--allow-empty`, so a commit that only touches excluded contracts doesn't fail the hook.

If odcslint is already on your `PATH` (see [Installation](../installation.md)), you can skip the build by overriding the language:

```yaml
      - id: odcslint
        language: system
```

To check contracts with other file names, override `files`:

```yaml
      - id: odcslint
        files: ^contracts/.*\.ya?ml$
```
