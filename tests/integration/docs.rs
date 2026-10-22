use std::fs;
use std::path::Path;

use assert_cmd::Command;
use odcslint::rules::allowed_values::ALIASES;
use odcslint::rules::RuleType;
use strum::IntoEnumIterator;

const RULES_DIRECTORY: &str = "docs/rules";
const RULES_INDEX: &str = "docs/rules/index.md";
const ALLOWED_VALUES_PAGE: &str = "docs/rules/allowed_values.md";
const DOCS_CONFIG: &str = "zensical.toml";
const EXAMPLE_HEADING: &str = "## Example";
const YAML_TAB: &str = "=== \"YAML\"";
const TAB_INDENT: &str = "    ";
const FENCE: &str = "```";
const YAML_FENCE: &str = "```yaml";
const TEXT_FENCE: &str = "```text";
const SNIPPET_FENCE_PREFIX: &str = "```yaml title=\"";
const CONFIG_FILE_NAME: &str = "odcslint.yml";
/// Snippets are only the lines a rule looks at, so schema validation would drown out its findings.
const SNIPPET_SETTINGS: &str = "settings:\n  schema_validation: off\n";
const SUMMARY_SEPARATOR: &str = "\n\nFound";

#[test]
fn every_rule_has_a_page() {
    let missing: Vec<&str> = RuleType::iter()
        .map(RuleType::name)
        .filter(|name| !Path::new(&format!("{RULES_DIRECTORY}/{name}.md")).is_file())
        .collect();
    assert!(missing.is_empty(), "rules without a docs page: {missing:?}");
}

#[test]
fn every_rule_is_listed_in_the_index() {
    let index = fs::read_to_string(RULES_INDEX).unwrap();
    let missing: Vec<&str> = RuleType::iter()
        .map(RuleType::name)
        .filter(|name| !index.contains(&format!("[`{name}`]({name}.md)")))
        .collect();
    assert!(
        missing.is_empty(),
        "rules missing from {RULES_INDEX}: {missing:?}"
    );
}

#[test]
fn navigation_counts_the_aliases() {
    let config = fs::read_to_string(DOCS_CONFIG).unwrap();
    let label = format!("\"allowed_values ({} aliases)\"", ALIASES.len());
    assert!(
        config.contains(&label),
        "{DOCS_CONFIG} should label the page {label}"
    );
}

/// The alias table drives the clickable examples, so each row must name the field (and role) the
/// alias really sets.
#[test]
fn every_alias_row_names_its_field() {
    let page = fs::read_to_string(ALLOWED_VALUES_PAGE).unwrap();
    let wrong: Vec<&str> = ALIASES
        .iter()
        .filter(|alias| {
            let row_start = format!("| `{}` | ", alias.name);
            let field = alias.field.path().map(|path| format!("`{path}`"));
            let role = alias.role.map(|role| format!("`role: {role}`"));
            !page.lines().map(str::trim).any(|line| {
                line.strip_prefix(&row_start).is_some_and(|rest| {
                    field.as_ref().is_some_and(|field| rest.starts_with(field))
                        && role.as_ref().is_none_or(|role| rest.contains(role))
                })
            })
        })
        .map(|alias| alias.name)
        .collect();
    assert!(
        wrong.is_empty(),
        "aliases without a matching row in {ALLOWED_VALUES_PAGE}: {wrong:?}"
    );
}

/// The lines between `start` (a fence line) and the closing fence.
fn fenced_block<'a>(lines: &[&'a str], start: usize) -> Vec<&'a str> {
    lines[start + 1..]
        .iter()
        .take_while(|line| line.trim() != FENCE)
        .copied()
        .collect()
}

fn example_config(page: &str) -> String {
    let examples = &page[page.find(EXAMPLE_HEADING).unwrap()..];
    let yaml_tab = &examples[examples.find(YAML_TAB).unwrap()..];
    let lines: Vec<&str> = yaml_tab.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim() == YAML_FENCE)
        .unwrap();
    let config: Vec<&str> = fenced_block(&lines, start)
        .iter()
        .map(|line| line.strip_prefix(TAB_INDENT).unwrap_or(line))
        .collect();
    config.join("\n") + "\n"
}

/// `(file name, contract snippet)` and the expected findings of the Example section.
fn findings_section(page: &str) -> ((String, String), String) {
    let section = &page[page.find(EXAMPLE_HEADING).unwrap()..];
    let lines: Vec<&str> = section.lines().collect();
    let snippet_start = lines
        .iter()
        .position(|line| line.starts_with(SNIPPET_FENCE_PREFIX))
        .unwrap();
    let file_name = lines[snippet_start]
        .trim_start_matches(SNIPPET_FENCE_PREFIX)
        .trim_end_matches('"')
        .to_string();
    let snippet = fenced_block(&lines, snippet_start).join("\n") + "\n";
    let text_start = lines
        .iter()
        .position(|line| line.trim() == TEXT_FENCE)
        .unwrap();
    let expected = fenced_block(&lines, text_start).join("\n");
    ((file_name, snippet), expected)
}

/// Every rule page's Example shows a config, a contract snippet and the findings they produce; the
/// findings must be what the binary prints.
#[test]
fn every_rule_page_shows_the_findings_of_its_snippet() {
    let wrong: Vec<String> = RuleType::iter()
        .map(RuleType::name)
        .filter_map(|name| {
            let page = fs::read_to_string(format!("{RULES_DIRECTORY}/{name}.md")).unwrap();
            let ((file_name, snippet), expected) = findings_section(&page);
            let directory = tempfile::tempdir().unwrap();
            fs::write(
                directory.path().join(CONFIG_FILE_NAME),
                format!("{SNIPPET_SETTINGS}{}", example_config(&page)),
            )
            .unwrap();
            fs::write(directory.path().join(&file_name), snippet).unwrap();
            let output = Command::cargo_bin("odcslint")
                .unwrap()
                .current_dir(directory.path())
                .env("NO_COLOR", "1")
                .args(["check", &file_name])
                .output()
                .unwrap();
            let stdout = String::from_utf8(output.stdout).unwrap();
            let actual = stdout.split(SUMMARY_SEPARATOR).next().unwrap_or_default();
            (actual.trim_end() != expected).then(|| {
                format!(
                    "{name}:\n--- documented\n{expected}\n--- actual\n{}",
                    actual.trim_end()
                )
            })
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
}
