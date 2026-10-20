# Contributing

## Development

```bash
just fmt     # cargo fmt
just lint    # cargo clippy (pedantic, nursery, cargo; warnings are errors)
just test    # cargo test
just docs    # serve these docs with Zensical
```

Before you open a pull request, make sure these pass:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features
```

## Architecture

| Module | Responsibility |
|---|---|
| `discovery` | Turn files, directories and globs into a sorted list of contract paths (parallel walk with `ignore`). |
| `document` | Parse YAML with source positions (`saphyr`), convert to JSON, resolve JSON pointers to positions, and provide typed views (`Contract`, `SchemaObject`, `PropertyList`, `Property`). |
| `schema` | Pick the embedded ODCS schema by `apiVersion`, compile it once, and turn validation errors into findings. |
| `config` | Discover and load `odcslint.yml` / `odcslint.toml` / `pyproject.toml`; compile globs and regexes at load time. |
| `rules` | One module per rule type: options, `check` function and unit tests. |
| `fix` | Text edits, overlap filtering, and reordering block sequences by moving source text. |
| `engine` | The per-file pipeline (fix loop, lint) run in parallel with `rayon`. |
| `report` | Output formats. |
| `cli` | Argument parsing and commands. |

## Adding a rule

1. Create `src/rules/<rule>.rs` with an options struct (`#[serde(deny_unknown_fields)]`), a `check(options, context) -> Vec<Violation>` function, and unit tests for the passing and failing cases.
2. Add a variant to `RuleKind` in `src/rules/mod.rs`. Keep the variants sorted; `#[remain::sorted]` won't compile otherwise.
3. Add the variant to `RuleType::summary`, to `check`, and to `RuleKind::validate` if its options need validating. These matches are exhaustive, so the compiler tells you if you missed one.
4. If the rule is fixable, return a `Fix` from `check` and add it to `RuleType::fixable`.
5. Add `docs/rules/<rule>.md` and a row in `docs/rules/index.md`. The integration tests fail until both exist.
