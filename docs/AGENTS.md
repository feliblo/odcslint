# Docs

The site is built with [Zensical](https://zensical.org/) from `zensical.toml` and the Markdown files here.

- Preview: `just docs` (runs `uvx zensical serve`); build: `just docs-build`.
- New pages must be added to `nav` in `zensical.toml`.
- Rule pages (`rules/<rule>.md`) follow the same structure: description, options table, YAML and TOML examples in `=== "YAML"` / `=== "TOML"` tabs, example findings, and a Fix section for fixable rules.
- Example findings must use the exact message format of the current binary; keep them in sync when messages change.
- Performance numbers must be measured (see `performance.md`), with date and machine.
- Document runtime behaviour only; do not describe planned features.
