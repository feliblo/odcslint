use std::fs;
use std::path::Path;

use odcslint::rules::RuleType;
use strum::IntoEnumIterator;

const RULES_DIRECTORY: &str = "docs/rules";
const RULES_INDEX: &str = "docs/rules/index.md";

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
