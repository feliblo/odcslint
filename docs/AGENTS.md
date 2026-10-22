# Docs

The site is built with [Zensical](https://zensical.org/) from `zensical.toml` and the Markdown files here.

- Preview: `just docs` (runs `uvx zensical serve`); build: `just docs-build`.
- New pages must be added to `nav` in `zensical.toml`.
- Rule pages (`rules/<rule>.md`) follow the same structure: description, options table, an Example section, and a Fix section for fixable rules.
- The Example section has three labelled parts: **Config** (YAML and TOML in `=== "YAML"` / `=== "TOML"` tabs), **Contract** (a ` ```yaml title="orders.odcs.yaml" ` snippet with only the lines the rule looks at, each problem line commented with what is wrong) and **Findings** (a ` ```text ` block with what odcslint prints for it, using the YAML config with schema validation off). `tests/integration/docs.rs` runs the binary and fails when they differ.
- Performance numbers must be measured (see `performance.md`), with date and machine.
- Document runtime behaviour only; do not describe planned features.
