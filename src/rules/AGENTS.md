# Rules

One module per rule type. Each module holds the options struct, the `check` function and the unit tests.

## Adding a rule

1. Create `src/rules/<rule>.rs`:
   - an options struct with `#[derive(Debug, Deserialize)]` and `#[serde(deny_unknown_fields)]`; defaults via `#[serde(default)]` / `#[serde(default = "...")]`
   - regexes and globs as `config::pattern::{Pattern, GlobList, GlobPattern}` so they compile once at load time
   - `pub fn check(options: &Options, context: &Context<'_>) -> Vec<Violation>`: pure, use `context.contract` views and `context.location(node)`
   - optional `pub(super) fn validate(&self) -> Result<(), String>` for option combinations
2. Register it in `src/rules/mod.rs`: a `RuleKind` variant (sorted), `RuleType::summary`, `check`, and `RuleKind::validate`. All matches are exhaustive; never add a `_ =>` arm.
3. Fixable rules return `Violation { fix: Some(Fix { edit, permutation }) }`. Every fix must declare its intended content change so the engine can verify it; add the type to `RuleType::fixable`.
4. Tests at the bottom of the module, using `rules::test_support::{rule, messages, violations_at}` with inline YAML. Cover the passing case, each failing case (exact message), and option variants. At least one test asserts the full `Violation` including location.
5. Docs: `docs/rules/<rule>.md` (description, options table, YAML + TOML examples, example findings) and a row in `docs/rules/index.md`. `tests/integration/docs.rs` enforces both.

## Conventions

- Messages are lowercase, name the object (`property \`id\``), and say what is expected.
- Keys of the ODCS model come from `constants::keys`.
- Missing optional fields are not findings unless the rule has a `required` option; schema-level problems are left to schema validation.
