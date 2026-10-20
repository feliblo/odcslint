use serde::Deserialize;

use super::{describe, Context, Violation};
use crate::constants::keys;
use crate::document::node::{as_str, get, get_path};
use crate::document::view::Scope;
use crate::document::Node;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HasDescription {
    #[serde(default = "default_scope")]
    pub scope: Scope,
    #[serde(default = "default_min_length")]
    pub min_length: usize,
}

const fn default_scope() -> Scope {
    Scope::Property
}

const fn default_min_length() -> usize {
    1
}

fn description<'a>(scope: Scope, node: &'a Node<'a>) -> Option<&'a str> {
    match scope {
        Scope::Contract => get_path(node, &format!("{}.{}", keys::DESCRIPTION, keys::PURPOSE)),
        Scope::Schema | Scope::Property => get(node, keys::DESCRIPTION),
    }
    .and_then(as_str)
}

pub fn check(options: &HasDescription, context: &Context<'_>) -> Vec<Violation> {
    context
        .contract
        .nodes_in(options.scope)
        .into_iter()
        .filter_map(|node| {
            let subject = match options.scope {
                Scope::Contract => "contract".to_string(),
                Scope::Schema => describe("schema", node),
                Scope::Property => describe("property", node),
            };
            let length =
                description(options.scope, node).map_or(0, |text| text.trim().chars().count());
            let message = if length == 0 {
                Some(format!("{subject} has no description"))
            } else if length < options.min_length {
                Some(format!(
                    "description of {subject} is shorter than {} characters",
                    options.min_length
                ))
            } else {
                None
            };
            message.map(|message| Violation::new(message, context.location(node)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{messages, rule};

    const CONTRACT: &str = "\
description:
  purpose: Sales orders
schema:
  - name: orders
    description: All orders
    properties:
      - name: id
        description: Order id
      - name: amount
      - name: note
        description: x
";

    #[test]
    fn reports_properties_without_description() {
        let kind = rule("type: has_description\n");
        assert_eq!(
            messages(&kind, CONTRACT),
            vec!["property `amount` has no description"]
        );
    }

    #[test]
    fn enforces_minimum_length() {
        let kind = rule("type: has_description\nmin_length: 3\n");
        assert_eq!(
            messages(&kind, CONTRACT),
            vec![
                "property `amount` has no description",
                "description of property `note` is shorter than 3 characters"
            ]
        );
    }

    #[test]
    fn contract_scope_uses_purpose() {
        let kind = rule("type: has_description\nscope: contract\n");
        assert!(messages(&kind, CONTRACT).is_empty());
        assert_eq!(
            messages(&kind, "name: x\n"),
            vec!["contract has no description"]
        );
    }

    #[test]
    fn schema_scope() {
        let kind = rule("type: has_description\nscope: schema\n");
        assert!(messages(&kind, CONTRACT).is_empty());
    }
}
