use std::fs;
use std::path::Path;

use odcslint::rules::allowed_values::ALIASES;
use odcslint::rules::RuleType;
use strum::IntoEnumIterator;

const RULES_DIRECTORY: &str = "docs/rules";
const RULES_INDEX: &str = "docs/rules/index.md";
const ALLOWED_VALUES_PAGE: &str = "docs/rules/allowed_values.md";

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
